use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use tower_http::trace::TraceLayer;

use crate::database::Database;

#[derive(Clone)]
pub struct AppState {
    db: Database,
}

impl AppState {
    pub fn router() -> Router {
        let state = Self {
            db: Database::new(),
        };

        Router::new()
            .route("/threads", post(create_thread))
            .with_state(state)
            .layer(TraceLayer::new_for_http())
    }
}

async fn create_thread(State(db): State<Database>) -> (StatusCode, Json<serde_json::Value>) {
    let thread = db.create_thread().await;
    let thread_id = thread.id();

    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": thread_id })),
    )
}

impl FromRef<AppState> for Database {
    fn from_ref(app_state: &AppState) -> Database {
        app_state.db.clone()
    }
}
