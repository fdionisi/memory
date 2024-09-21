pub mod error;

pub use async_trait::async_trait;
pub use error::DatabaseError;

use synx_domain::{
    embedding::Embedding,
    message::{CreateMessage, Message, ThreadMessagesResponse, UpdateMessage},
    thread::Thread,
};
use uuid::Uuid;

#[async_trait]
pub trait Db: Send + Sync {
    async fn debug_state(&self) -> Result<serde_json::Value, DatabaseError>;

    async fn get_threads_with_embeddings(
        &self,
        thread_ids: &[Uuid],
    ) -> Result<Vec<Thread>, DatabaseError>;

    async fn update_thread_summary_and_embedding(
        &self,
        thread_id: Uuid,
        summary: String,
        embedding: Embedding,
    ) -> Result<(), DatabaseError>;

    async fn create_thread(&self) -> Result<Thread, DatabaseError>;

    async fn delete_thread(&self, thread_id: Uuid) -> Result<(), DatabaseError>;

    async fn create_message(
        &self,
        thread_id: Uuid,
        input: CreateMessage,
    ) -> Result<Message, DatabaseError>;

    async fn update_message(
        &self,
        thread_id: Uuid,
        message_id: Uuid,
        content: UpdateMessage,
    ) -> Result<Message, DatabaseError>;

    async fn list_threads(&self) -> Result<Vec<Thread>, DatabaseError>;

    async fn get_thread(&self, thread_id: Uuid) -> Result<Thread, DatabaseError>;

    async fn get_thread_messages(
        &self,
        thread_id: Uuid,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ThreadMessagesResponse, DatabaseError>;

    async fn delete_message(&self, thread_id: Uuid, message_id: Uuid) -> Result<(), DatabaseError>;
}
