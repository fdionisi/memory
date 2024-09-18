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
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use chrono::DateTime;
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

    #[tokio::test]
    async fn update_message() {
        let mut app = AppState::router();

        // Create a thread
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
        let thread_body: Value = serde_json::from_slice(
            &thread_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let thread_id = thread_body["id"].as_str().unwrap();

        // Create a message
        let create_message_response = app
            .call(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(r#"{"content": "Original message"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_message_response.status(), StatusCode::CREATED);
        let create_message_body: Value = serde_json::from_slice(
            &create_message_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let message_id = create_message_body["id"].as_str().unwrap();

        // Update the message
        let update_message_response = app
            .call(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri(format!("/threads/{thread_id}/messages/{message_id}"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(r#"{"content": "Updated message"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(update_message_response.status(), StatusCode::OK);
        let update_message_body: Value = serde_json::from_slice(
            &update_message_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();

        assert_eq!(update_message_body["id"], message_id);
        assert_eq!(update_message_body["thread_id"], thread_id);
        assert_eq!(update_message_body["content"]["text"], "Updated message");

        // Try to update a non-existent message
        let non_existent_message_id = Uuid::new_v4();
        let non_existent_update_response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri(format!(
                        "/threads/{thread_id}/messages/{non_existent_message_id}"
                    ))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(r#"{"content": "This should fail"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(non_existent_update_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_threads() {
        let mut app = AppState::router();

        // Create multiple threads
        let thread_count = 3;
        let mut thread_ids = Vec::new();

        for _ in 0..thread_count {
            let response = app
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

            assert_eq!(response.status(), StatusCode::CREATED);
            let body: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            thread_ids.push(body["id"].as_str().unwrap().to_string());
        }

        // List threads
        let list_response = app
            .call(
                Request::builder()
                    .method(http::Method::GET)
                    .uri("/threads")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body: Value = serde_json::from_slice(
            &list_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();

        assert!(list_body.is_array());
        let threads = list_body.as_array().unwrap();
        assert_eq!(threads.len(), thread_count);

        // Verify all created thread IDs are in the list
        for thread in threads {
            assert!(thread_ids.contains(&thread["id"].as_str().unwrap().to_string()));
        }
    }

    #[tokio::test]
    async fn delete_thread() {
        let mut app = AppState::router();

        // Create a thread
        let create_thread_response = app
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

        assert_eq!(create_thread_response.status(), StatusCode::CREATED);
        let thread_body: Value = serde_json::from_slice(
            &create_thread_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let thread_id = thread_body["id"].as_str().unwrap();

        // Create messages in the thread
        for _ in 0..3 {
            let create_message_response = app
                .call(
                    Request::builder()
                        .method(http::Method::POST)
                        .uri(format!("/threads/{thread_id}/messages"))
                        .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                        .body(Body::from(r#"{"content": "Test message"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_message_response.status(), StatusCode::CREATED);
        }

        // Delete the thread
        let delete_thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(format!("/threads/{thread_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(delete_thread_response.status(), StatusCode::NO_CONTENT);

        // Attempt to get the deleted thread (should fail)
        let get_deleted_thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/threads/{thread_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_deleted_thread_response.status(), StatusCode::NOT_FOUND);

        // Attempt to create a message in the deleted thread (should fail)
        let create_message_in_deleted_thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(r#"{"content": "This should fail"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            create_message_in_deleted_thread_response.status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn get_thread() {
        let mut app = AppState::router();

        // Create a thread
        let create_thread_response = app
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

        assert_eq!(create_thread_response.status(), StatusCode::CREATED);
        let create_thread_body: Value = serde_json::from_slice(
            &create_thread_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let thread_id = create_thread_body["id"].as_str().unwrap();

        // Get the created thread
        let get_thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/threads/{thread_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_thread_response.status(), StatusCode::OK);
        let get_thread_body: Value = serde_json::from_slice(
            &get_thread_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();

        assert_eq!(get_thread_body["id"], thread_id);

        // Attempt to get a non-existent thread
        let non_existent_thread_id = Uuid::new_v4();
        let get_non_existent_thread_response = app
            .call(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/threads/{non_existent_thread_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            get_non_existent_thread_response.status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn get_thread_messages() {
        let mut app = AppState::router();

        // Create a thread
        let create_thread_response = app
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

        assert_eq!(create_thread_response.status(), StatusCode::CREATED);
        let create_thread_body: Value = serde_json::from_slice(
            &create_thread_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let thread_id = create_thread_body["id"].as_str().unwrap();

        // Create multiple messages in the thread
        let message_contents = vec![
            "First message",
            "Second message",
            "Third message",
            "Fourth message",
            "Fifth message",
        ];
        for content in &message_contents {
            let create_message_response = app
                .call(
                    Request::builder()
                        .method(http::Method::POST)
                        .uri(format!("/threads/{thread_id}/messages"))
                        .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                        .body(Body::from(format!(r#"{{"content": "{}"}}"#, content)))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_message_response.status(), StatusCode::CREATED);

            // Add a small delay to ensure different created_at times
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Get the thread messages with pagination
        let get_messages_response = app
            .call(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/threads/{thread_id}/messages?limit=3&offset=1"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_messages_response.status(), StatusCode::OK);

        // Check headers
        assert_eq!(
            get_messages_response
                .headers()
                .get("X-Total-Count")
                .unwrap(),
            "5"
        );
        assert_eq!(
            get_messages_response.headers().get("X-Offset").unwrap(),
            "1"
        );
        assert_eq!(get_messages_response.headers().get("X-Limit").unwrap(), "3");

        let get_messages_body: Value = serde_json::from_slice(
            &get_messages_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();

        let messages = get_messages_body.as_array().unwrap();
        assert_eq!(messages.len(), 3);

        // Verify messages are in order
        for (i, message) in messages.iter().enumerate() {
            assert_eq!(message["content"]["text"], message_contents[i + 1]);
            if i > 0 {
                let prev_created_at = DateTime::from_timestamp_millis(
                    messages[i - 1]["created_at"].as_i64().unwrap(),
                )
                .unwrap();
                let current_created_at =
                    DateTime::from_timestamp_millis(message["created_at"].as_i64().unwrap())
                        .unwrap();
                assert!(
                    current_created_at > prev_created_at,
                    "Messages are not in correct order"
                );
            }
        }

        // Attempt to get messages for a non-existent thread
        let non_existent_thread_id = Uuid::new_v4();
        let get_non_existent_thread_messages_response = app
            .call(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/threads/{non_existent_thread_id}/messages"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            get_non_existent_thread_messages_response.status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn delete_message() {
        let mut app = AppState::router();

        // Create a thread
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
        let thread_body: Value = serde_json::from_slice(
            &thread_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let thread_id = thread_body["id"].as_str().unwrap();

        // Create a message
        let create_message_response = app
            .call(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/threads/{thread_id}/messages"))
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(r#"{"content": "Test message"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_message_response.status(), StatusCode::CREATED);
        let create_message_body: Value = serde_json::from_slice(
            &create_message_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let message_id = create_message_body["id"].as_str().unwrap();

        // Delete the message
        let delete_message_response = app
            .call(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(format!("/threads/{thread_id}/messages/{message_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(delete_message_response.status(), StatusCode::NO_CONTENT);

        // Try to delete the same message again (should fail)
        let delete_nonexistent_response = app
            .call(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri(format!("/threads/{thread_id}/messages/{message_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(delete_nonexistent_response.status(), StatusCode::NOT_FOUND);
    }
}
