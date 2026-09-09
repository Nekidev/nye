use std::sync::Arc;

use axum::extract::State as AxumState;
use toasty::Db;

#[derive(Debug, Clone)]
pub struct RegistryState {
    pub db: Db,
    pub duckity: Arc<Option<DuckityState>>,
}

#[derive(Debug, Clone)]
pub struct DuckityState {
    pub application_secret: String,
    pub signin_protection_profile_id: String,
    pub signup_protection_profile_id: String,
}

pub type State = AxumState<RegistryState>;
