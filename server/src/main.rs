mod cache;

use cache::Cache;
use reqwest::Client;
use std::{
    collections::HashMap, sync::Arc, time::{Duration, Instant}
};
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply, http::Response};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0";
const API_SERVERS: &[&str] = &[
    "https://lz4.overpass-api.de/api/interpreter",
    "https://z.overpass-api.de/api/interpreter",
    "https://overpass-api.de/api/interpreter",
];
const MAX_BODY_SIZE_LEN: u64 = 1024;

const ADDR: std::net::SocketAddr = std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
    std::net::Ipv4Addr::new(127, 0, 0, 1),
    0xa44d,
));

pub struct APIServer {
    url: String,
    last_request: Instant,
    cooldown: Duration,
}

impl APIServer {
    fn ready(&self) -> bool {
        self.last_request.elapsed() > self.cooldown
    }
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let cache: Arc<RwLock<Box<dyn Cache>>> =
        Arc::new(RwLock::new(Box::new(cache::DiskCache{ root_path: "./cache".into() })));

    let api_servers = Arc::new(RwLock::new(
        API_SERVERS
            .iter()
            .map(|url| APIServer {
                url: (*url).to_owned(),
                last_request: Instant::now(),
                cooldown: Duration::from_secs(1),
            })
            .collect::<Vec<_>>(),
    ));

    let rate_limiter_config = warp_rate_limit::RateLimitConfig::default();

    let proxy = warp::path("overpass_api")
        .and(warp::get())
        .and(warp::query::query::<HashMap<String, String>>())
        .and(warp_rate_limit::with_rate_limit(rate_limiter_config))
        .and(warp::any().map(move || client.clone()))
        .and(warp::any().map(move || cache.clone()))
        .and(warp::any().map(move || api_servers.clone()))
        .and_then(handle_proxy);

    println!("Listening on http://{}:{}", ADDR.ip(), ADDR.port());
    warp::serve(proxy).run(ADDR).await;
}

async fn handle_proxy(
    user_query: HashMap<String, String>,
    _rli: warp_rate_limit::RateLimitInfo,
    client: Client,
    cache: Arc<RwLock<Box<dyn Cache>>>,
    api_servers: Arc<RwLock<Vec<APIServer>>>,
) -> Result<impl Reply, Rejection> {
    println!("Received a query for {user_query:?}");
    
    match cache.read().await.get(&format!("{user_query:?}")) {
        Ok(Some(saved)) => {
            println!("Cache hit");
            return Ok(Response::builder()
                .status(200)
                .body(saved.to_owned())
                .unwrap());
        }
        Ok(None) => (),
        Err(e) => {
            println!("[ERROR]: An error occured while fetching cache: {e}");
            return Ok(Response::builder()
                .status(500)
                .body(String::from("Internal server error"))
                .unwrap());
        }
    }

    let mut servers = api_servers.write().await;

    #[allow(clippy::manual_inspect)]
    let Some(api_server)=  // Find first server that is available
        servers.iter_mut()
            .find(|server| 
                server.ready()
            )
            .map(|server| {
                server.last_request = Instant::now();
                server
            })else{
        
        return Ok(Response::builder()
            .status(503)
            .body(String::from("No api server is ready atm"))
            .unwrap());
    };

    let request= match client.get(&api_server.url).query(&user_query).build(){
        Ok(request) => request,
        Err(e) => {
            println!("[ERROR]: An error occured while building the request with user data: {e}");
            return Ok(Response::builder()
                .status(400)
                .body(String::from("Bad request"))
                .unwrap());
            
        },
    };

    println!("Querying url ({}) with: {:?}", api_server.url, request.url());

    let response= match client.execute(request).await{
        Ok(res) => res,
        Err(e) => {
            
            println!("[ERROR]: An error occured while querying api server ({}): {e}", api_server.url);
            return Ok(Response::builder()
                .status(500)
                .body(String::from("Internal server error"))
                .unwrap());
        }
    };
    if !response.status().is_success(){
        println!("[ERROR]: got a non-success code when querying api server ({}): {}", api_server.url, response.status());
        
        return Ok(Response::builder()
            .status(500)
            .body(String::from("Internal server error"))
            .unwrap());
    }
    
    let res_body = match response.text().await{
        Ok(res_body) => res_body,
        Err(e) => {
            
            println!("[ERROR]: An error occured while unpacking the reponse of the api server ({}): {e}", api_server.url);
            return Ok(Response::builder()
                .status(500)
                .body(String::from("Internal server error"))
                .unwrap());
        },
    };

    {
        if let Err(e) = cache.write().await.insert(&format!("{user_query:?}"), &res_body) {
            println!("[ERROR]: An error occured while writing response to cache: {e}");
            return Ok(Response::builder()
                .status(500)
                .body(String::from("Internal server error"))
                .unwrap());
        }
    }

    Ok(Response::builder()
        .status(200)
        .body(res_body)
        .unwrap())
    
}
