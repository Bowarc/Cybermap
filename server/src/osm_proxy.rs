use reqwest::{Client, StatusCode};
use std::{sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::Instant};
use warp_rate_limit::serde;

use warp::{Filter, Rejection, Reply};

mod api_server;
mod cache;
use api_server::ServerPool;
use cache::Cache;

use crate::rejection;

const USER_AGENT: &str = "Cybermap/0.1.0 (linux; x86_64)";

const API_SERVERS: &[&str] = &[
    "https://lz4.overpass-api.de/api/interpreter",
    "https://z.overpass-api.de/api/interpreter",
    "https://overpass-api.de/api/interpreter",
];

// Fallback, we won't use it unless we have no other choice
const FALLBACK_API_SERVER: &str = "https://overpass.private.coffee/api/interpreter";

#[derive(serde::Deserialize)]
struct OSMQuery {
    data: String,
}

pub fn build_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let client = reqwest::Client::new();
    let cache: Arc<RwLock<Box<dyn Cache>>> = Arc::new(RwLock::new(Box::new(cache::DiskCache {
        root_path: "./cache".into(),
    })));
    let api_pool = ServerPool::new(API_SERVERS, Duration::from_secs(1), FALLBACK_API_SERVER);

    let rate_limiter_config = warp_rate_limit::RateLimitConfig {
        max_requests: 4,
        window: Duration::from_secs(10),
        retry_after_format: warp_rate_limit::RetryAfterFormat::HttpDate,
        ip_header: "X-Forwarded-For".to_owned(),
    };

    warp::get()
        .and(warp::path("overpass_api"))
        .and(warp::path::end())
        // Naïve 'security' to make sure bots won't trigger an api call by spamming random sht
        .and(warp::filters::header::exact(
            "cybermap",
            "8b3d00bf-b0cc-4a7d-b389-9c0e9d0688f8",
        ))
        .and(warp::query::query::<OSMQuery>().map(|query: OSMQuery| query.data))
        .and(warp_rate_limit::with_rate_limit(rate_limiter_config))
        .and(warp::any().map(move || client.clone()))
        .and(warp::any().map(move || cache.clone()))
        .and(warp::any().map(move || api_pool.clone()))
        .and_then(handle_request)
        .recover(rejection::method_not_allowed)
        .recover(rejection::missing_header)
        .recover(rejection::invalid_header)
        .recover(rejection::invalid_query)
        .recover(rejection::rate_limit)
        .recover(rejection::proxy)
}

#[derive(Debug)]
pub enum OSMProxyRejection {
    CacheFailure,
    InvalidUserData,
    APIServerFailure,
    APIResponseUnpackingFailed,
}

impl warp::reject::Reject for OSMProxyRejection {}

async fn handle_request(
    data_query: String,
    _rli: warp_rate_limit::RateLimitInfo,
    client: Client,
    cache: Arc<RwLock<Box<dyn Cache>>>,
    api_server_pool: api_server::ServerPool,
) -> Result<impl Reply, Rejection> {
    trace!("Received a query for {data_query:?}");

    let t1 = Instant::now();

    match cache.read().await.get(&format!("{data_query:?}")) {
        Ok(None) => (),
        Ok(Some(saved)) => {
            debug!("Cache hit");
            debug!("Response in: {}", time::format(&t1.elapsed(), 2));
            return Ok(warp::reply::with_status(saved, StatusCode::OK));
        }
        Err(e) => {
            error!("An error occured while fetching cache: {e}");
            return Err(OSMProxyRejection::CacheFailure.into());
        }
    }

    let api_server_url = api_server_pool.find_one_ready().await;

    let request = client
        .get(api_server_url.to_string())
        .query(&[("data", &data_query)])
        .header("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| {
            error!("An error occured while building the request with user data: {e}");
            OSMProxyRejection::InvalidUserData
        })?;

    debug!("Querying url ({api_server_url})");

    let response = client.execute(request).await.map_err(|e| {
        error!(
            "An error occured while querying api server ({}) with data ({}): {e}",
            api_server_url, data_query
        );
        OSMProxyRejection::InvalidUserData
    })?;

    if !response.status().is_success() {
        let status_code = response.status();

        error!(
            "Got a non-success code when querying api server ({}): {}",
            api_server_url,
            response.status()
        );

        if status_code.is_client_error() {
            return Err(OSMProxyRejection::InvalidUserData.into());
        }

        return Err(OSMProxyRejection::APIServerFailure.into());
    }

    let res_body = response.text().await.map_err(|e| {
        error!(
            "An error occured while unpacking the reponse of the api server ({}): {e}",
            api_server_url
        );
        OSMProxyRejection::APIResponseUnpackingFailed
    })?;

    if let Err(e) = cache
        .write()
        .await
        .insert(&format!("{data_query:?}"), &res_body)
    {
        error!("An error occured while writing response to cache: {e}");
    }

    debug!("Response in: {}", time::format(&t1.elapsed(), 2));

    Ok(warp::reply::with_status(res_body, StatusCode::OK))
}
