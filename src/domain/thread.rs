use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::embedding::Embedding;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip)]
    pub embedding: Option<Embedding>,
}

impl Thread {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            summary: None,
            embedding: None,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn set_summary(&mut self, summary: String) {
        self.summary = Some(summary);
    }

    pub fn set_embedding(&mut self, embedding: Embedding) {
        self.embedding = Some(dbg!(embedding));
    }
}
