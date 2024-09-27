use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use synx_database::{DatabaseError, Db};
use synx_domain::{
    embedding::Embedding,
    message::{CreateMessage, Message, ThreadMessagesResponse, UpdateMessage},
    thread::{Thread, UpdateThread},
};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct SynxInMemory {
    threads: Arc<Mutex<HashMap<Uuid, Thread>>>,
    messages: Arc<Mutex<HashMap<Uuid, Message>>>,
    thread_messages: Arc<Mutex<HashMap<Uuid, HashSet<Uuid>>>>,
}

#[allow(unused)]
impl SynxInMemory {
    pub fn new() -> Self {
        Self {
            threads: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(HashMap::new())),
            thread_messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl Db for SynxInMemory {
    async fn debug_state(&self) -> Result<serde_json::Value, DatabaseError> {
        let threads = self.threads.lock().await;
        let messages = self.messages.lock().await;
        let thread_messages = self.thread_messages.lock().await;

        Ok(serde_json::json!({
            "threads": threads.clone(),
            "messages": messages.clone(),
            "thread_messages": thread_messages.clone(),
        }))
    }

    async fn get_threads_with_embeddings(
        &self,
        thread_ids: &[Uuid],
    ) -> Result<Vec<Thread>, DatabaseError> {
        let threads = self.threads.lock().await;
        thread_ids
            .iter()
            .filter_map(|id| threads.get(id).cloned())
            .collect::<Vec<Thread>>()
            .into_iter()
            .map(|mut thread| {
                if thread.embedding.is_none() {
                    thread.embedding = Some(Embedding::from(vec![0.0; 1536]));
                }
                Ok(thread)
            })
            .collect()
    }

    async fn update_thread_summary_and_embedding(
        &self,
        thread_id: Uuid,
        summary: String,
        embedding: Embedding,
    ) -> Result<(), DatabaseError> {
        let mut threads = self.threads.lock().await;
        if let Some(thread) = threads.get_mut(&thread_id) {
            thread.set_summary(summary);
            thread.set_embedding(embedding);
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    async fn create_thread(&self) -> Result<Thread, DatabaseError> {
        let thread = Thread::new();
        let mut threads = self.threads.lock().await;
        threads.insert(thread.id(), thread.clone());
        self.thread_messages
            .lock()
            .await
            .insert(thread.id(), HashSet::new());
        Ok(thread)
    }

    async fn delete_thread(&self, thread_id: Uuid) -> Result<(), DatabaseError> {
        let mut threads = self.threads.lock().await;
        if threads.remove(&thread_id).is_none() {
            return Err(DatabaseError::NotFound);
        }

        let mut messages = self.messages.lock().await;
        let mut thread_messages = self.thread_messages.lock().await;

        if let Some(message_ids) = thread_messages.remove(&thread_id) {
            for message_id in message_ids {
                messages.remove(&message_id);
            }
        }

        Ok(())
    }

    async fn create_message(
        &self,
        thread_id: Uuid,
        input: CreateMessage,
    ) -> Result<Message, DatabaseError> {
        self.threads
            .lock()
            .await
            .get(&thread_id)
            .ok_or(DatabaseError::NotFound)?;

        let message = input.into_message(thread_id);
        let message_id = message.id();
        let mut messages = self.messages.lock().await;
        messages.insert(message_id, message.clone());

        let mut thread_messages = self.thread_messages.lock().await;
        thread_messages
            .entry(thread_id)
            .or_insert_with(HashSet::new)
            .insert(message_id);

        Ok(message)
    }

    async fn update_message(
        &self,
        thread_id: Uuid,
        message_id: Uuid,
        content: UpdateMessage,
    ) -> Result<Message, DatabaseError> {
        self.threads
            .lock()
            .await
            .get(&thread_id)
            .ok_or(DatabaseError::NotFound)?;

        let mut messages = self.messages.lock().await;
        let message = messages
            .get_mut(&message_id)
            .ok_or(DatabaseError::NotFound)?;

        message.update_content(content);
        Ok(message.clone())
    }

    async fn list_threads(&self) -> Result<Vec<Thread>, DatabaseError> {
        let threads = self.threads.lock().await;
        Ok(threads.values().cloned().collect())
    }

    async fn get_thread(&self, thread_id: Uuid) -> Result<Thread, DatabaseError> {
        let threads = self.threads.lock().await;
        threads
            .get(&thread_id)
            .cloned()
            .ok_or(DatabaseError::NotFound)
    }

    async fn get_thread_messages(
        &self,
        thread_id: Uuid,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ThreadMessagesResponse, DatabaseError> {
        let threads = self.threads.lock().await;
        if !threads.contains_key(&thread_id) {
            return Err(DatabaseError::NotFound);
        }

        let thread_messages = self.thread_messages.lock().await;
        let messages = self.messages.lock().await;

        let message_ids = thread_messages.get(&thread_id).cloned().unwrap_or_default();
        let mut thread_messages: Vec<Message> = message_ids
            .iter()
            .filter_map(|id| messages.get(id).cloned())
            .collect();

        thread_messages.sort_by(|a, b| a.created_at().cmp(&b.created_at()));

        let total = thread_messages.len();
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(total);

        let paginated_messages = thread_messages
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(ThreadMessagesResponse {
            messages: paginated_messages,
            total,
            offset,
            limit,
        })
    }

    async fn delete_message(&self, thread_id: Uuid, message_id: Uuid) -> Result<(), DatabaseError> {
        self.threads
            .lock()
            .await
            .get(&thread_id)
            .ok_or(DatabaseError::NotFound)?;

        let mut messages = self.messages.lock().await;
        messages
            .remove(&message_id)
            .ok_or(DatabaseError::NotFound)?;

        Ok(())
    }

    async fn update_thread(
        &self,
        thread_id: Uuid,
        update: UpdateThread,
    ) -> Result<Thread, DatabaseError> {
        let mut threads = self.threads.lock().await;
        if let Some(thread) = threads.get_mut(&thread_id) {
            thread.set_title(update.title);
            Ok(thread.clone())
        } else {
            Err(DatabaseError::NotFound)
        }
    }
}
