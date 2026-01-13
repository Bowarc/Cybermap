use std::{
    iter::Iterator,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub struct APIServer {
    url: Arc<str>,
    last_request: Instant,
}

#[derive(Clone)]
pub struct ServerPool {
    servers: Arc<RwLock<Box<[APIServer]>>>,
    cooldown: Duration,
    fallback_url: Arc<str>,
}

impl ServerPool {
    pub fn new(urls: &[&str], cooldown: Duration, fallback: &str) -> Self {
        let servers = urls
            .iter()
            .map(|url| APIServer {
                url: Arc::from(*url),
                last_request: Instant::now(),
            })
            .collect::<Box<[APIServer]>>();

        Self {
            servers: Arc::new(RwLock::new(servers)),
            cooldown,
            fallback_url: Arc::from(fallback),
        }
    }

    pub async fn find_one_ready(&self) -> Arc<str> {
        let now = Instant::now();

        self.servers
            .write()
            .await
            .iter_mut()
            .find(|server| now - server.last_request > self.cooldown)
            .map(|server| {
                server.last_request = now;
                server.url.clone()
            })
            .unwrap_or_else(|| self.fallback_url.clone())
    }
}
