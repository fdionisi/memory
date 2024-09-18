use std::sync::Arc;

use axum::{
    extract::{FromRef, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use ferrochain::embedding::Embedder;
use ferrochain_voyageai_embedder::{EmbeddingInputType, EmbeddingModel, VoyageAiEmbedder};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{
    database::Database,
    message::{Content, ContentKind, CreateMessage, UpdateMessage},
    thread::Thread,
};

#[derive(Clone)]
pub struct AppState {
    db: Database,
    embedder: Arc<dyn Embedder>,
}

impl AppState {
    pub fn router() -> Router {
        let state = Self {
            db: Database::new(),
            embedder: Arc::new(
                VoyageAiEmbedder::builder()
                    .model(EmbeddingModel::Voyage3)
                    .input_type(EmbeddingInputType::Document)
                    .build()
                    .expect("Failed to create VoyageAiEmbedder"),
            ),
        };

        Router::new()
            .route("/threads", post(create_thread))
            .route("/threads", get(list_threads))
            .route("/threads/:id", get(get_thread))
            .route("/threads/:id", delete(delete_thread))
            .route("/threads/:id/messages", post(create_message))
            .route("/threads/:id/messages", get(get_messages))
            .route(
                "/threads/:thread_id/messages/:message_id",
                put(update_message),
            )
            .route(
                "/threads/:thread_id/messages/:message_id",
                delete(delete_message),
            )
            .with_state(state)
            .layer(TraceLayer::new_for_http())
    }
}

impl FromRef<AppState> for Database {
    fn from_ref(app_state: &AppState) -> Database {
        app_state.db.clone()
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

async fn list_threads(State(db): State<Database>) -> Json<Vec<Thread>> {
    let threads = db.list_threads().await;
    Json(threads)
}

async fn get_thread(
    State(db): State<Database>,
    Path(thread_id): Path<Uuid>,
) -> Result<Json<Thread>, StatusCode> {
    match db.get_thread(thread_id).await {
        Ok(thread) => Ok(Json(thread)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
struct ThreadMessagesParams {
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn get_messages(
    State(db): State<Database>,
    Path(thread_id): Path<Uuid>,
    Query(params): Query<ThreadMessagesParams>,
) -> Result<impl IntoResponse, StatusCode> {
    match db
        .get_thread_messages(thread_id, params.limit, params.offset)
        .await
    {
        Ok(response) => {
            let headers = [
                ("X-Total-Count", response.total.to_string()),
                ("X-Offset", response.offset.to_string()),
                ("X-Limit", response.limit.to_string()),
            ];
            Ok((headers, Json(response.messages)))
        }
        Err(e) => match e.to_string().as_str() {
            "thread not found" => Err(StatusCode::NOT_FOUND),
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
    }
}
async fn create_message(
    State(AppState { db, embedder }): State<AppState>,
    Path(thread_id): Path<Uuid>,
    Json(create_message): Json<CreateMessage>,
) -> Response {
    match db.create_message(thread_id, create_message).await {
        Ok(message) => {
            if let Some(text_content) = extract_text_content(&message.content) {
                tokio::spawn(async move {
                    let embeddings = match embedder.embed(vec![text_content]).await {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::error!("Failed to create embedding: {}", e);
                            return;
                        }
                    };

                    let Some(embedding) = embeddings.first() else {
                        tracing::warn!("No embedding generated for message");
                        return;
                    };

                    if let Err(e) = db.save_message_embedding(message.id, embedding).await {
                        tracing::error!("Failed to save message embedding: {}", e);
                    }
                });
            }
            (StatusCode::CREATED, Json(message)).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "thread not found" })),
        )
            .into_response(),
    }
}

fn extract_text_content(content: &Content) -> Option<String> {
    match content {
        Content::Single(ContentKind::Text { text }) => Some(text.clone()),
        Content::Multiple(contents) => {
            let text_contents: Vec<String> = contents
                .iter()
                .filter_map(|c| {
                    if let ContentKind::Text { text } = c {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if text_contents.is_empty() {
                None
            } else {
                Some(text_contents.join("\n"))
            }
        }
        _ => None,
    }
}

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

async fn delete_message(
    State(db): State<Database>,
    Path((thread_id, message_id)): Path<(Uuid, Uuid)>,
) -> StatusCode {
    match db.delete_message(thread_id, message_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(e) => match e.to_string().as_str() {
            "thread not found" | "message not found" => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    }
}

async fn delete_thread(State(db): State<Database>, Path(thread_id): Path<Uuid>) -> StatusCode {
    match db.delete_thread(thread_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_FOUND,
    }
}
