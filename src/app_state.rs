use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{post, put},
    Json, Router,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{
    database::Database,
    message::{CreateMessage, UpdateMessage},
};

async fn update_message(
    State(db): State<Database>,
    Path((thread_id, message_id)): Path<(Uuid, Uuid)>,
    Json(update_message): Json<UpdateMessage>,
) -> Response {
    match db
        .update_message(thread_id, message_id, update_message)
        .await
    {
        Ok(message) => (StatusCode::OK, Json(message)).into_response(),
        Err(e) => match e.to_string().as_str() {
            "thread not found" | "message not found" => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response(),
        },
    }
}

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
            .route(
                "/threads/:thread_id/messages/:message_id",
                put(update_message),
            )
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

async fn create_message(
    State(db): State<Database>,
    Path(thread_id): Path<Uuid>,
    Json(create_message): Json<CreateMessage>,
) -> Response {
    match db.create_message(thread_id, create_message).await {
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
