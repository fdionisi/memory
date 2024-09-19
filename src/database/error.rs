use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Item not found")]
    NotFound,
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Query error: {0}")]
    QueryError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Database operation failed: {0}")]
    OperationFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}
