use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

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
            .route("/threads/:id/messages", post(create_message))
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

async fn create_message(State(db): State<Database>, Path(thread_id): Path<Uuid>) -> Response {
    match db.create_message(thread_id).await {
        Ok(message) => (StatusCode::CREATED, Json(message)).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "thread not found" })),
        )
            .into_response(),
    }
}

impl FromRef<AppState> for Database {
    fn from_ref(app_state: &AppState) -> Database {
        app_state.db.clone()
    }
}
