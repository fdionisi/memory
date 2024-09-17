use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Serialize, Deserialize)]
pub struct Thread {
    id: Uuid,
}

impl Thread {
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
