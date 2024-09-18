use std::sync::Arc;

use axum::extract::FromRef;
use ferrochain::{completion::Completion, embedding::Embedder};
use ferrochain_anthropic_completion::AnthropicCompletion;
use ferrochain_voyageai_embedder::{EmbeddingInputType, EmbeddingModel, VoyageAiEmbedder};

use crate::database::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub completion: Arc<dyn Completion>,
    pub embedder: Arc<dyn Embedder>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Database::new(),
            completion: Arc::new(
                AnthropicCompletion::builder()
                    .build()
                    .expect("Failed to create AnthropicCompletion"),
            ),
            embedder: Arc::new(
                VoyageAiEmbedder::builder()
                    .model(EmbeddingModel::Voyage3)
                    .input_type(EmbeddingInputType::Document)
                    .build()
                    .expect("Failed to create VoyageAiEmbedder"),
            ),
        }
    }
}

impl FromRef<AppState> for Database {
    fn from_ref(app_state: &AppState) -> Database {
        app_state.db.clone()
    }
}
