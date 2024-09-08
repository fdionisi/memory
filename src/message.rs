use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    id: Uuid,
    thread_id: Uuid,
    content: Content,
}

impl Message {
    pub fn id(&self) -> Uuid {
        self.id
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
        }
    }
}
