pub mod in_memory;

use anyhow::Result;
use ferrochain::embedding::Embedding;
use uuid::Uuid;

use crate::domain::{
    message::{CreateMessage, Message, ThreadMessagesResponse, UpdateMessage},
    thread::Thread,
};

pub use in_memory::InMemory;

#[ferrochain::async_trait]
pub trait Db: Send + Sync {
    async fn get_threads_with_embeddings(&self, thread_ids: &[Uuid]) -> Result<Vec<Thread>>;
    async fn update_thread_summary_and_embedding(
        &self,
        thread_id: Uuid,
        summary: String,
        embedding: Embedding,
    ) -> Result<()>;
    async fn create_thread(&self) -> Thread;
    async fn delete_thread(&self, thread_id: Uuid) -> Result<()>;
    async fn create_message(&self, thread_id: Uuid, input: CreateMessage) -> Result<Message>;
    async fn update_message(
        &self,
        thread_id: Uuid,
        message_id: Uuid,
        content: UpdateMessage,
    ) -> Result<Message>;
    async fn list_threads(&self) -> Vec<Thread>;
    async fn get_thread(&self, thread_id: Uuid) -> Result<Thread>;
    async fn get_thread_messages(
        &self,
        thread_id: Uuid,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ThreadMessagesResponse>;
    async fn delete_message(&self, thread_id: Uuid, message_id: Uuid) -> Result<()>;
}
