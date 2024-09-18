use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Utc};
use ferrochain::embedding::Embedding;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentKind {
    Text { text: String },
    Image { url: String },
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum Content {
    Single(ContentKind),
    Multiple(Vec<ContentKind>),
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Content::Single(ContentKind::Text { text })
    }
}

impl From<Vec<String>> for Content {
    fn from(texts: Vec<String>) -> Self {
        Content::Multiple(
            texts
                .into_iter()
                .map(|text| ContentKind::Text { text })
                .collect(),
        )
    }
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ContentHelper {
            Single(String),
            Multiple(Vec<String>),
            SingleObject(ContentKind),
            MultipleObjects(Vec<ContentKind>),
        }

        let helper = ContentHelper::deserialize(deserializer)?;
        match helper {
            ContentHelper::Single(text) => Ok(Content::Single(ContentKind::Text { text })),
            ContentHelper::Multiple(texts) => Ok(Content::Multiple(
                texts
                    .into_iter()
                    .map(|text| ContentKind::Text { text })
                    .collect(),
            )),
            ContentHelper::SingleObject(content_type) => Ok(Content::Single(content_type)),
            ContentHelper::MultipleObjects(content_types) => Ok(Content::Multiple(content_types)),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub content: Content,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl From<MessageWithEmbedding> for Message {
    fn from(message_with_embedding: MessageWithEmbedding) -> Self {
        message_with_embedding.message
    }
}

#[derive(Clone)]
pub struct MessageWithEmbedding {
    pub message: Message,
    pub embedding: Option<Embedding>,
}

impl MessageWithEmbedding {
    pub fn set_embedding(&mut self, embedding: Embedding) {
        self.embedding = Some(embedding);
    }
}

impl From<Message> for MessageWithEmbedding {
    fn from(message: Message) -> Self {
        MessageWithEmbedding {
            message,
            embedding: None,
        }
    }
}

impl Deref for MessageWithEmbedding {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl DerefMut for MessageWithEmbedding {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.message
    }
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
    content: Content,
}

impl CreateMessage {
    pub fn into_message(self, thread_id: Uuid) -> Message {
        Message {
            id: Uuid::new_v4(),
            thread_id,
            content: self.content,
            created_at: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdateMessage {
    content: Content,
}

#[derive(Serialize)]
pub struct ThreadMessagesResponse {
    pub messages: Vec<Message>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}
