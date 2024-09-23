use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::content::Content;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub role: String,
    pub content: Content,
    pub created_at: u64,
}

impl Message {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn update_content(&mut self, new_content: UpdateMessage) {
        self.content = new_content.content;
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.created_at as i64).unwrap()
    }
}

impl ToString for Message {
    fn to_string(&self) -> String {
        self.content.to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateMessage {
    pub role: String,
    pub content: Content,
}

impl CreateMessage {
    pub fn into_message(self, thread_id: Uuid) -> Message {
        Message {
            id: Uuid::new_v4(),
            thread_id,
            role: self.role,
            content: self.content,
            created_at: Utc::now().timestamp_millis() as u64,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMessage {
    pub content: Content,
}

#[derive(Serialize, Deserialize)]
pub struct ThreadMessagesResponse {
    pub messages: Vec<Message>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}
