use serde::{Deserialize, Serialize};

/// The JSON-serialized part of an error response.
///
/// Do not return this struct directly from handlers. Use [`Error`](super::errors::Error) instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSchema {
    pub title: String,
    pub message: String,
}
