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
}

impl ServerPool {
    pub fn new(urls: &[&str], cooldown: Duration) -> Self {
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
        }
    }

    pub async fn find_one_ready(&self) -> Option<Arc<str>> {
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
    }
}
