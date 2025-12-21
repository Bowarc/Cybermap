use reqwest::{Client, StatusCode};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use warp_rate_limit::{RateLimitRejection, add_rate_limit_headers_from_rejection, serde};

use warp::{Filter, Rejection, Reply};

mod api_server;
mod cache;
use api_server::ServerPool;
use cache::Cache;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0";

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

pub fn build_route() -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    let client = reqwest::Client::new();
    let cache: Arc<RwLock<Box<dyn Cache>>> = Arc::new(RwLock::new(Box::new(cache::DiskCache {
        root_path: "./cache".into(),
    })));
    let api_pool = ServerPool::new(API_SERVERS, Duration::from_secs(1), FALLBACK_API_SERVER);

    let rate_limiter_config = warp_rate_limit::RateLimitConfig {
        max_requests: 60,
        window: Duration::from_secs(60),
        retry_after_format: warp_rate_limit::RetryAfterFormat::HttpDate,
        ip_header: "X-Forwarded-For".to_owned(),
    };

    warp::path("overpass_api")
        .and(warp::get())
        .and(warp::query::query::<OSMQuery>().map(|query: OSMQuery| query.data))
        .and(warp_rate_limit::with_rate_limit(rate_limiter_config))
        .and(warp::any().map(move || client.clone()))
        .and(warp::any().map(move || cache.clone()))
        .and(warp::any().map(move || api_pool.clone()))
        .and_then(handle_request)
        .recover(handle_rejection)
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

    match cache.read().await.get(&format!("{data_query:?}")) {
        Ok(None) => (),
        Ok(Some(saved)) => {
            debug!("Cache hit");
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
        .build()
        .map_err(|e| {
            error!("An error occured while building the request with user data: {e}");
            OSMProxyRejection::InvalidUserData
        })?;

    debug!(
        "Querying url ({}) with: {:?}",
        api_server_url,
        request.url()
    );

    let response = client.execute(request).await.map_err(|e| {
        error!(
            "An error occured while querying api server ({}): {e}",
            api_server_url
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

    Ok(warp::reply::with_status(res_body, StatusCode::OK))
}

async fn handle_rejection(rejection: Rejection) -> Result<impl Reply, Infallible> {
    if rejection.is_not_found() {
        return Ok(warp::reply::with_status(
            "The content you requested does not exist",
            StatusCode::NOT_FOUND,
        )
        .into_response());
    }

    if let Some(proxy_rejection) = rejection.find::<OSMProxyRejection>() {
        let response = match proxy_rejection {
            OSMProxyRejection::CacheFailure
            | OSMProxyRejection::APIServerFailure
            | OSMProxyRejection::APIResponseUnpackingFailed => {
                warp::reply::with_status("Internal server error", StatusCode::INTERNAL_SERVER_ERROR)
                    .into_response()
            }

            OSMProxyRejection::InvalidUserData => {
                warp::reply::with_status("Bad request", StatusCode::BAD_REQUEST).into_response()
            }
        };
        return Ok(response);
    };

    if let Some(rate_limit_rejection) = rejection.find::<RateLimitRejection>() {
        let message = format!(
            "Rate limit exceeded. Try again after {:?}.",
            rate_limit_rejection.retry_after
        );

        let mut response =
            warp::reply::with_status(message, StatusCode::TOO_MANY_REQUESTS).into_response();

        if let Err(e) =
            add_rate_limit_headers_from_rejection(response.headers_mut(), rate_limit_rejection)
        {
            error!("Failed to set rejection rate limit headers due to: {e}")
        }

        return Ok(response);
    }

    error!("Unable to find rejection in: {rejection:?}");

    Ok(
        warp::reply::with_status("Something went wrong", StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    )
}
