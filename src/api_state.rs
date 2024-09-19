use std::sync::Arc;

use axum::extract::FromRef;
use database::Db;
use ferrochain::{completion::Completion, embedding::Embedder};

#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<dyn Db>,
    pub completion: Arc<dyn Completion>,
    pub completion_model: String,
    pub document_embedder: Arc<dyn Embedder>,
    pub query_embedder: Arc<dyn Embedder>,
}

impl FromRef<ApiState> for Arc<dyn Db> {
    fn from_ref(app_state: &ApiState) -> Arc<dyn Db> {
        app_state.db.clone()
    }
}
