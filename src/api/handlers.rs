use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ferrochain::vectorstore::Similarity;
use synx::{SearchRequest, Synx};
use synx_domain::{
    message::{CreateMessage, UpdateMessage},
    thread::{Thread, UpdateThread},
};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct PaginationParams {
    limit: Option<usize>,
    offset: Option<usize>,
}

pub async fn create_thread(State(synx): State<Synx>) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("Attempting to create a new thread");
    match synx.create_thread().await {
        Ok(thread) => {
            tracing::info!("Thread created successfully: {:?}", thread);
            Ok((StatusCode::CREATED, Json(thread)))
        }
        Err(e) => {
            tracing::error!("Failed to create thread: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_threads(State(synx): State<Synx>) -> Result<Json<Vec<Thread>>, StatusCode> {
    tracing::info!("Attempting to list threads");
    match synx.list_threads().await {
        Ok(threads) => {
            tracing::info!("Successfully retrieved {} threads", threads.len());
            Ok(Json(threads))
        }
        Err(e) => {
            tracing::error!("Failed to list threads: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_thread(
    State(synx): State<Synx>,
    Path(thread_id): Path<Uuid>,
) -> Result<Json<Thread>, StatusCode> {
    match synx.get_thread(thread_id).await {
        Ok(thread) => Ok(Json(thread)),
        Err(e) => {
            tracing::error!("Failed to get thread {}: {:?}", thread_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_thread(
    State(synx): State<Synx>,
    Path(thread_id): Path<Uuid>,
    Json(update_thread): Json<UpdateThread>,
) -> Result<Json<Thread>, StatusCode> {
    match synx.update_thread(thread_id, update_thread).await {
        Ok(thread) => Ok(Json(thread)),
        Err(e) => {
            tracing::error!("Failed to update thread {}: {:?}", thread_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_messages(
    State(synx): State<Synx>,
    Path(thread_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, StatusCode> {
    match synx
        .get_messages(thread_id, params.limit, params.offset)
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
        Err(e) => {
            tracing::error!("Failed to get messages for thread {}: {:?}", thread_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn create_message(
    State(synx): State<Synx>,
    Path(thread_id): Path<Uuid>,
    Json(create_message): Json<CreateMessage>,
) -> Response {
    match synx.create_message(thread_id, create_message).await {
        Ok(message) => (StatusCode::CREATED, Json(message)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create message in thread {}: {:?}", thread_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response()
        }
    }
}

pub async fn update_message(
    State(synx): State<Synx>,
    Path((thread_id, message_id)): Path<(Uuid, Uuid)>,
    Json(update_message): Json<UpdateMessage>,
) -> Response {
    match synx
        .update_message(thread_id, message_id, update_message)
        .await
    {
        Ok(message) => (StatusCode::OK, Json(message)).into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to update message {} in thread {}: {:?}",
                message_id,
                thread_id,
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("internal server error: {}", e) })),
            )
                .into_response()
        }
    }
}

pub async fn delete_message(
    State(synx): State<Synx>,
    Path((thread_id, message_id)): Path<(Uuid, Uuid)>,
) -> StatusCode {
    match synx.delete_message(thread_id, message_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(e) => {
            tracing::error!(
                "Failed to delete message {} in thread {}: {:?}",
                message_id,
                thread_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn delete_thread(State(synx): State<Synx>, Path(thread_id): Path<Uuid>) -> StatusCode {
    match synx.delete_thread(thread_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(e) => {
            tracing::error!("Failed to delete thread {}: {:?}", thread_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn debug_database_state(
    State(synx): State<Synx>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Debugging database state");
    match synx.debug_state().await {
        Ok(state) => {
            tracing::info!("Database state retrieved successfully");
            Ok(Json(state))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve database state: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn search_threads(
    State(synx): State<Synx>,
    Json(search_request): Json<SearchRequest>,
) -> Result<Json<Vec<Similarity>>, StatusCode> {
    match synx.search_threads(search_request).await {
        Ok(similarities) => Ok(Json(similarities)),
        Err(e) => {
            tracing::error!("Failed to search threads: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}
