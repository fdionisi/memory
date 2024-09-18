use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl Thread {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            summary: None,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn set_summary(&mut self, summary: String) {
        self.summary = Some(summary);
    }
}
