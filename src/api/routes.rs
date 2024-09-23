use axum::{
    routing::{delete, get, post, put},
    Router,
};
use synx::Synx;

use crate::api::handlers;

pub fn router(synx: Synx) -> Router {
    Router::new()
        .route("/threads", post(handlers::create_thread))
        .route("/threads", get(handlers::list_threads))
        .route("/threads/:id", get(handlers::get_thread))
        .route("/threads/:id", delete(handlers::delete_thread))
        .route("/threads/:id/messages", post(handlers::create_message))
        .route("/threads/:id/messages", get(handlers::get_messages))
        .route(
            "/threads/:thread_id/messages/:message_id",
            put(handlers::update_message),
        )
        .route(
            "/threads/:thread_id/messages/:message_id",
            delete(handlers::delete_message),
        )
        .route("/search", post(handlers::search_threads))
        .route("/debug/database", get(handlers::debug_database_state))
        .with_state(synx)
}
