use std::{
    iter::Iterator,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;

pub struct APIServer {
    url: Arc<str>,
    last_request: SystemTime,
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
                last_request: SystemTime::UNIX_EPOCH,
            })
            .collect::<Box<[APIServer]>>();

        Self {
            servers: Arc::new(RwLock::new(servers)),
            cooldown,
        }
    }

    pub async fn find_one_ready(&self) -> Option<Arc<str>> {
        let now = SystemTime::now();

        self.servers
            .write()
            .await
            .iter_mut()
            .find(|server| now.duration_since(server.last_request).unwrap() > self.cooldown)
            .map(|server| {
                server.last_request = now;
                server.url.clone()
            })
    }
}
