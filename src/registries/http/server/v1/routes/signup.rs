use axum::Json;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::v1::errors::Error;

#[derive(Serialize, Deserialize)]
pub struct SignupRequestPayload {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct SignupResponsePayload {}

pub async fn handle(
    Json(payload): Json<SignupRequestPayload>,
) -> Result<Json<SignupResponsePayload>, Error> {
    todo!()
}
