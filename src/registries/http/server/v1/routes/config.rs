use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub name: String,
}

pub async fn handle() -> Json<Config> {
    Json(Config {
        name: "Nye's Registry".into(),
    })
}
