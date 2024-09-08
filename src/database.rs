use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{message::Message, thread::Thread};

#[derive(Clone)]
pub struct Database {
    threads: Arc<Mutex<HashMap<Uuid, Thread>>>,
    messages: Arc<Mutex<HashMap<Uuid, Message>>>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            threads: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_thread(&self) -> Thread {
        let thread = Thread::new();
        let mut threads = self.threads.lock().await;
        threads.insert(thread.id(), thread.clone());
        thread
    }

    pub async fn create_message(&self, thread_id: Uuid) -> Result<Message> {
        self.threads
            .lock()
            .await
            .get(&thread_id)
            .ok_or_else(|| anyhow!("thread not found"))?;

        let message = Message::new(thread_id);
        let mut messages = self.messages.lock().await;
        messages.insert(message.id(), message.clone());
        Ok(message)
    }
}
