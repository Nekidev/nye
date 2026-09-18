use axum::Json;
use serde::{Deserialize, Serialize};

use crate::state::State;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub is_signin_enabled: bool,
    pub is_signup_enabled: bool,
    pub duckity_signin_policy_id: Option<String>,
    pub duckity_signup_policy_id: Option<String>,
}

pub async fn handle(state: State) -> Json<Config> {
    Json(Config {
        name: state.registry.name.clone(),
        is_signin_enabled: state.registry.is_signin_enabled,
        is_signup_enabled: state.registry.is_signup_enabled,
        duckity_signin_policy_id: (*state.duckity).clone().map(|i| i.signin_policy_id),
        duckity_signup_policy_id: (*state.duckity).clone().map(|i| i.signup_policy_id),
    })
}
