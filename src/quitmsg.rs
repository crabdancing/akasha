use tokio::sync::RwLock;
use std::time::Duration;

impl QuitMsg {
    pub fn new() -> Self {
        Self {
            flag: RwLock::new(true)
        }
    }

    pub async fn poll(&self) -> bool {
        !self.flag.read().await.clone()
    }

    pub async fn wait(&self) {
        while *self.flag.read().await {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub async fn send_quit(&self) {
        *self.flag.write().await = false;
    }
}

pub struct QuitMsg {
    flag: RwLock<bool>
}
