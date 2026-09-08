use axum::Json;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::v1::errors::Error;

#[derive(Serialize, Deserialize)]
pub struct SigninRequestPayload {
    pub login: String,
    pub password: String,

    #[serde(default)]
    pub duckity: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SigninResponsePayload {
    pub access_token: String,
    pub access_token_expires_in: u64,
    pub refresh_token: String,
    pub refresh_token_expires_in: u64,
}

pub async fn handle(
    Json(_payload): Json<SigninRequestPayload>,
) -> Result<Json<SigninResponsePayload>, Error> {
    todo!()
}
