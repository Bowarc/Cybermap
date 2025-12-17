mod cache;

use cache::Cache;
use reqwest::Client;
use std::{
    hash::{DefaultHasher, Hasher},
    sync::Arc,
};
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply, http::Response, hyper::body::Bytes};

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0";
const API_SERVERS: &[&str] = &["https://overpass-api.de/api/interpreter"];
const MAX_BODY_SIZE_LEN: u64 = 1024;

const ADDR: std::net::SocketAddr = std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
    std::net::Ipv4Addr::new(127, 0, 0, 1),
    0xa44d,
));

#[tokio::main]
async fn main() {
    let client = Client::new();
    let cache: Arc<RwLock<Box<dyn Cache>>> =
        Arc::new(RwLock::new(Box::new(cache::CacheMap::new())));

    let proxy = warp::path("overpass_api")
        .and(warp::get())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(MAX_BODY_SIZE_LEN))
        .and(warp::body::bytes())
        .and(warp::any().map(move || client.clone()))
        .and(warp::any().map(move || cache.clone()))
        .and_then(handle_proxy);

    println!("Listening on http://{}:{}", ADDR.ip(), ADDR.port());
    warp::serve(proxy).run(ADDR).await;
}

async fn handle_proxy(
    body: Bytes,
    client: Client,
    cache: Arc<RwLock<Box<dyn Cache>>>,
) -> Result<impl Reply, Rejection> {

    // TODO: Remove that debug log
    {
        println!("{:#?}", cache.read().await);
    }

    let Ok(body) = str::from_utf8(&body) else {
        eprintln!("Failed to parse request body to str, aborting");
        return Ok(Response::builder()
            .status(400)
            .body("Invalid body content".to_owned())
            .unwrap());
    };

    match cache.read().await.get(body) {
        Ok(Some(saved)) => {
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

    // TODO: Redo this to actually send a request to one of the api servers (random pick)
    // Don't forget about the 1req/s limit
    // Could a tokio task with a sleep and a std::sync::atomic::AtomicBool do the trick ?
    // We know for a fact that it will create at MAX one task at a time
    //
    // Wayy simpler: we could save the last request time and check agaist that before sending another one
    let out = {
        let mut hasher = DefaultHasher::new();
        hasher.write(body.as_bytes());
        hasher.finish()
    };

    {
        if let Err(e) = cache.write().await.insert(body, &format!("{out}")) {
            println!("[ERROR]: An error occured while writing response to cache: {e}");
            return Ok(Response::builder()
                .status(500)
                .body(String::from("Internal server error"))
                .unwrap());
        }
    }

    Ok(Response::builder()
        .status(200)
        .body(format!("New: {out}"))
        .unwrap())
}
