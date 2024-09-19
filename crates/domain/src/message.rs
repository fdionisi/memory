use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::content::{Content, ContentKind};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub role: String,
    pub content: Content,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn update_content(&mut self, new_content: UpdateMessage) {
        self.content = new_content.content;
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

impl ToString for Message {
    fn to_string(&self) -> String {
        match &self.content {
            Content::Single(ContentKind::Text { text }) => text.clone(),
            Content::Multiple(content) => content
                .iter()
                .map(|content| match content {
                    ContentKind::Text { text } => text.clone(),
                    ContentKind::Image { url } => url.clone(),
                })
                .collect::<Vec<String>>()
                .join("\n"),
            Content::Single(ContentKind::Image { url }) => url.clone(),
        }
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
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMessage {
    pub content: Content,
}

#[derive(Serialize)]
pub struct ThreadMessagesResponse {
    pub messages: Vec<Message>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}
