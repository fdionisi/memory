use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Thread {
    id: Uuid,
}

impl Thread {
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app()).await.unwrap();
}

/// Having a function that produces our app makes it easy to call it from tests
/// without having to create an HTTP server.

#[derive(Clone)]
struct AppState {
    threads: Arc<Mutex<Vec<Thread>>>,
}

async fn create_thread(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let thread = Thread::new();
    let thread_id = thread.id();

    let mut threads = state.threads.lock().await;
    threads.push(thread);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": thread_id })),
    )
}

fn app() -> Router {
    let state = AppState {
        threads: Arc::new(Mutex::new(Vec::new())),
    };

    Router::new()
        .route("/threads", post(create_thread))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt; // for `collect`
    use serde_json::Value;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn create_thread() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/threads")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert!(
            body.get("id").is_some(),
            "Response should contain an 'id' field"
        );
        assert!(body["id"].is_string(), "The 'id' field should be a string");

        // Validate that the ID is a valid UUID
        let id_str = body["id"].as_str().unwrap();
        assert!(
            Uuid::parse_str(id_str).is_ok(),
            "The 'id' should be a valid UUID"
        );
    }
}
