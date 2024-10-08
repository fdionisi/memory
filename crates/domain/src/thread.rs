use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::embedding::Embedding;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    pub title: Option<String>,
    pub summary: Option<String>,
    #[serde(skip)]
    pub embedding: Option<Embedding>,
}

impl Thread {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: None,
            summary: None,
            embedding: None,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    pub fn set_summary(&mut self, summary: String) {
        self.summary = Some(summary);
    }

    pub fn set_embedding(&mut self, embedding: Embedding) {
        self.embedding = Some(embedding);
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateThread {
    pub title: Option<String>,
}
