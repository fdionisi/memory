use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::thread::Thread;

#[derive(Clone)]
pub struct Database {
    threads: Arc<Mutex<HashMap<Uuid, Thread>>>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            threads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_thread(&self) -> Thread {
        let thread = Thread::new();
        let mut threads = self.threads.lock().await;
        threads.insert(thread.id(), thread.clone());
        thread
    }
}
