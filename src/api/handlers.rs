use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ferrochain::{
    document::{Document, StoredDocument},
    vectorstore::Similarity,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    database::Database,
    domain::{
        message::{CreateMessage, UpdateMessage},
        thread::Thread,
    },
    utils::{
        completion::generate_summary, content::extract_text_content,
        embedding::generate_embeddings, similarity::cosine_similarity,
    },
};

#[derive(Deserialize)]
pub struct PaginationParams {
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct SearchRequest {
    pub query: String,
    pub thread_ids: Vec<Uuid>,
}

pub async fn create_thread(State(db): State<Database>) -> (StatusCode, Json<serde_json::Value>) {
    let thread = db.create_thread().await;
    let thread_id = thread.id();

    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": thread_id })),
    )
}

pub async fn list_threads(State(db): State<Database>) -> Json<Vec<Thread>> {
    let threads = db.list_threads().await;
    Json(threads)
}

pub async fn get_thread(
    State(db): State<Database>,
    Path(thread_id): Path<Uuid>,
) -> Result<Json<Thread>, StatusCode> {
    match db.get_thread(thread_id).await {
        Ok(thread) => Ok(Json(thread)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn get_messages(
    State(db): State<Database>,
    Path(thread_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
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

pub async fn create_message(
    State(AppState {
        db,
        document_embedder,
        completion,
        ..
    }): State<AppState>,
    Path(thread_id): Path<Uuid>,
    Json(create_message): Json<CreateMessage>,
) -> Response {
    match db.create_message(thread_id, create_message).await {
        Ok(message) => {
            if let Some(text_content) = extract_text_content(&message.content) {
                let completion_content = text_content.clone();
                let db_content = db.clone();
                let message_role = message.role.clone();
                tokio::spawn(async move {
                    let thread = match db_content.get_thread(thread_id).await {
                        Ok(response) => response,
                        Err(e) => {
                            tracing::error!("Failed to fetch thread messages: {}", e);
                            return;
                        }
                    };
                    let summary = match generate_summary(
                        completion,
                        thread.summary.unwrap_or_default(),
                        message_role,
                        completion_content,
                    )
                    .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Failed to generate summary: {}", e);
                            return;
                        }
                    };

                    let embedding = match generate_embeddings(&document_embedder, &summary).await {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::error!("Failed to create embedding: {}", e);
                            return;
                        }
                    };

                    if let Err(e) = db_content
                        .update_thread_summary_and_embedding(thread_id, summary, embedding)
                        .await
                    {
                        tracing::error!("Failed to update thread summary and embedding: {}", e);
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

pub async fn update_message(
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

pub async fn delete_message(
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

pub async fn delete_thread(State(db): State<Database>, Path(thread_id): Path<Uuid>) -> StatusCode {
    match db.delete_thread(thread_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

pub async fn search_threads(
    State(AppState {
        db, query_embedder, ..
    }): State<AppState>,
    Json(search_request): Json<SearchRequest>,
) -> Result<Json<Vec<Similarity>>, StatusCode> {
    let threads = db
        .get_threads_with_embeddings(&search_request.thread_ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let query_embedding = generate_embeddings(&query_embedder, &search_request.query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut similarities: Vec<Similarity> = threads
        .into_iter()
        .filter_map(|thread| {
            thread.embedding.map(|embedding| {
                let score = cosine_similarity(&query_embedding, &embedding);
                Similarity {
                    stored: StoredDocument {
                        id: thread.id.to_string(),
                        document: Document {
                            content: thread.summary.unwrap_or_default(),
                            metadata: HashMap::new(),
                        },
                    },
                    score,
                }
            })
        })
        .collect();

    similarities.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    Ok(Json(similarities))
}
