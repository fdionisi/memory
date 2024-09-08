mod app_state;
mod database;
mod message;
mod thread;

use app_state::AppState;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    axum::serve(listener, AppState::router()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::{Service, ServiceExt};
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn create_thread() {
        let app = AppState::router();

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

        let id_str = body["id"].as_str().unwrap();
        assert!(
            Uuid::parse_str(id_str).is_ok(),
            "The 'id' should be a valid UUID"
        );
    }

    #[tokio::test]
    async fn create_message() {
        let mut app = AppState::router();

        let thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/threads")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(thread_response.status(), StatusCode::CREATED);

        let thread_body = thread_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let thread_body: Value = serde_json::from_slice(&thread_body).unwrap();
        let thread_id = thread_body["id"].as_str().unwrap();

        let message_content = "Test message";
        let message_response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(format!(r#"{{"content": "{message_content}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(message_response.status(), StatusCode::CREATED);

        let message_body = message_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let message_body: Value = serde_json::from_slice(&message_body).unwrap();

        assert_eq!(
            message_body["thread_id"], thread_id,
            "Thread ID in message response should match the created thread ID"
        );
        assert!(
            message_body.get("id").is_some(),
            "Message response should contain an 'id' field"
        );
        assert!(
            message_body["id"].is_string(),
            "The message 'id' field should be a string"
        );
        assert!(
            Uuid::parse_str(message_body["id"].as_str().unwrap()).is_ok(),
            "The message 'id' should be a valid UUID"
        );

        dbg!(&message_body["content"]);

        assert_eq!(
            message_body["content"]["text"], message_content,
            "Message content in response should match the sent content"
        );
    }

    #[tokio::test]
    async fn submit_message_to_nonexistent_thread() {
        let app = AppState::router();

        let non_existent_thread_id = Uuid::new_v4();
        let message_response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{non_existent_thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(r#"{"content": "Test message"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(message_response.status(), StatusCode::NOT_FOUND);

        let message_body = message_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let message_body: Value = serde_json::from_slice(&message_body).unwrap();

        assert_eq!(
            message_body["error"], "thread not found",
            "Response should indicate that the thread was not found"
        );
    }

    #[tokio::test]
    async fn submit_message_with_string_vec_content() {
        let mut app = AppState::router();

        let thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/threads")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(thread_response.status(), StatusCode::CREATED);

        let thread_body = thread_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let thread_body: Value = serde_json::from_slice(&thread_body).unwrap();
        let thread_id = thread_body["id"].as_str().unwrap();

        let message_content = vec!["Hello", "World"];
        let message_response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "content": message_content
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(message_response.status(), StatusCode::CREATED);

        let message_body = message_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let message_body: Value = serde_json::from_slice(&message_body).unwrap();

        assert_eq!(
            message_body["thread_id"], thread_id,
            "Thread ID in message response should match the created thread ID"
        );
        assert!(
            message_body.get("id").is_some(),
            "Message response should contain an 'id' field"
        );
        assert!(
            message_body["id"].is_string(),
            "The message 'id' field should be a string"
        );
        assert!(
            Uuid::parse_str(message_body["id"].as_str().unwrap()).is_ok(),
            "The message 'id' should be a valid UUID"
        );
        assert_eq!(
            message_body["content"][0]["text"], message_content[0],
            "Message content in response should match the sent content"
        );
        assert_eq!(
            message_body["content"][1]["text"], message_content[1],
            "Message content in response should match the sent content"
        );
    }

    #[tokio::test]
    async fn submit_message_with_text_and_image_content() {
        let mut app = AppState::router();

        let thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/threads")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(thread_response.status(), StatusCode::CREATED);

        let thread_body = thread_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let thread_body: Value = serde_json::from_slice(&thread_body).unwrap();
        let thread_id = thread_body["id"].as_str().unwrap();

        let message_content = serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": "This is a text message"
                },
                {
                    "type": "image",
                    "url": "https://example.com/image.jpg"
                }
            ]
        });

        let message_response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_string(&message_content).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(message_response.status(), StatusCode::CREATED);

        let message_body = message_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let message_body: Value = serde_json::from_slice(&message_body).unwrap();

        assert_eq!(
            message_body["thread_id"], thread_id,
            "Thread ID in message response should match the created thread ID"
        );
        assert!(
            message_body.get("id").is_some(),
            "Message response should contain an 'id' field"
        );
        assert!(
            message_body["id"].is_string(),
            "The message 'id' field should be a string"
        );
        assert!(
            Uuid::parse_str(message_body["id"].as_str().unwrap()).is_ok(),
            "The message 'id' should be a valid UUID"
        );
        assert_eq!(
            message_body["content"][0]["type"], "text",
            "First content item should be of type 'text'"
        );
        assert_eq!(
            message_body["content"][0]["text"], "This is a text message",
            "Text content should match the sent content"
        );
        assert_eq!(
            message_body["content"][1]["type"], "image",
            "Second content item should be of type 'image'"
        );
        assert_eq!(
            message_body["content"][1]["url"], "https://example.com/image.jpg",
            "Image URL should match the sent URL"
        );
    }
}
