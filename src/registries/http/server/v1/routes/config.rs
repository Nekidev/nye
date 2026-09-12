use axum::Json;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::state::State;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub is_signin_enabled: bool,
    pub is_signup_enabled: bool,
    pub duckity_signin_protection_profile_id: Option<String>,
    pub duckity_signup_protection_profile_id: Option<String>,
}

pub async fn handle(state: State) -> Json<Config> {
    Json(Config {
        name: state.registry.name.clone(),
        is_signin_enabled: state.registry.is_signin_enabled,
        is_signup_enabled: state.registry.is_signup_enabled,
        duckity_signin_protection_profile_id: (*state.duckity)
            .clone()
            .map(|i| i.signin_protection_profile_id),
        duckity_signup_protection_profile_id: (*state.duckity)
            .clone()
            .map(|i| i.signup_protection_profile_id),
    })
}
