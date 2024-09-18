use std::sync::Arc;

use axum::extract::FromRef;
use ferrochain::{completion::Completion, embedding::Embedder};
use ferrochain_anthropic_completion::AnthropicCompletion;
use ferrochain_voyageai_embedder::{EmbeddingInputType, EmbeddingModel, VoyageAiEmbedder};

use crate::database::{Db, InMemory};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<dyn Db + Send + Sync>,
    pub completion: Arc<dyn Completion>,
    pub document_embedder: Arc<dyn Embedder>,
    pub query_embedder: Arc<dyn Embedder>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Arc::new(InMemory::new()),
            completion: Arc::new(
                AnthropicCompletion::builder()
                    .build()
                    .expect("Failed to create AnthropicCompletion"),
            ),
            document_embedder: Arc::new(
                VoyageAiEmbedder::builder()
                    .model(EmbeddingModel::Voyage3)
                    .input_type(EmbeddingInputType::Document)
                    .build()
                    .expect("Failed to create VoyageAiEmbedder"),
            ),
            query_embedder: Arc::new(
                VoyageAiEmbedder::builder()
                    .model(EmbeddingModel::Voyage3)
                    .input_type(EmbeddingInputType::Query)
                    .build()
                    .expect("Failed to create VoyageAiEmbedder"),
            ),
        }
    }
}

impl FromRef<AppState> for Arc<dyn Db> {
    fn from_ref(app_state: &AppState) -> Arc<dyn Db> {
        app_state.db.clone()
    }
}
