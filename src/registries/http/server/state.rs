use std::sync::Arc;

use axum::extract::State as AxumState;
use toasty::Db;

use crate::registries::http::server::storage::StorageBackend;

#[derive(Debug, Clone)]
pub struct RegistryState {
    pub db: Db,
    pub registry: Arc<RegistryConfig>,
    pub duckity: Arc<Option<DuckityState>>,
    pub storage: Arc<StorageBackend>,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub name: String,
    pub is_signup_enabled: bool,
    pub is_signin_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct DuckityState {
    pub application_secret: String,
    pub signin_protection_profile_id: String,
    pub signup_protection_profile_id: String,
}

pub type State = AxumState<RegistryState>;
