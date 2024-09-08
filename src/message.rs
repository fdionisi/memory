use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    id: Uuid,
    thread_id: Uuid,
}

impl Message {
    pub fn new(thread_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            thread_id,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
