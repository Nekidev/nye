use url::Url;

use crate::registries::http::client::HttpRegistryClient;
use crate::registries::local::client::LocalRegistryClient;

pub mod config;
pub mod credentials;
pub mod http;
pub mod local;

/// The registry's exposed configuration.
///
/// Not the same as [`config::RegistryConfig`], which is the **local** configuration of a registry.
pub struct RegistryConfig {
    pub name: String,
    pub is_signin_enabled: bool,
    pub is_signup_enabled: bool,
    pub duckity_signin_policy_id: Option<String>,
    pub duckity_signup_policy_id: Option<String>,
}

pub struct Credentials {
    pub name: String,
    pub access_token: String,
    pub access_token_expires_in: u64,
    pub refresh_token: String,
    pub refresh_token_expires_in: u64,
}

pub trait Registry {
    /// Fetches and returns the registry's configuration.
    async fn get_config(&self) -> anyhow::Result<RegistryConfig>;

    /// Creates an account at the registry.
    async fn signup(
        &self,
        email: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<()>;

    /// Signs into the registry.
    async fn signin(
        &self,
        username: impl Into<String>,
        password: impl Into<String>,
        duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<Credentials>;
}

pub enum RegistryClient {
    Http(HttpRegistryClient),
    Local(LocalRegistryClient),
}

impl RegistryClient {
    pub fn from_url(url: Url) -> anyhow::Result<Self> {
        match url.scheme() {
            "local" => Ok(Self::Local(LocalRegistryClient::from_url(url)?)),
            "https" | "http" => Ok(Self::Http(HttpRegistryClient::from_url(url)?)),
            _ => anyhow::bail!(
                "Only `local`, `http`, and `https` URL schemes can be used for registry URLs."
            ),
        }
    }

    pub fn from_config(config: &config::RegistryConfig) -> Self {
        match config.kind() {
            RegistryKind::Http => Self::Http(HttpRegistryClient::new(config)),
            RegistryKind::Local => Self::Local(LocalRegistryClient::new(config)),
        }
    }

    pub fn kind(&self) -> RegistryKind {
        match &self {
            Self::Http(_) => RegistryKind::Http,
            Self::Local(_) => RegistryKind::Local,
        }
    }
}

impl Registry for RegistryClient {
    async fn get_config(&self) -> anyhow::Result<RegistryConfig> {
        match &self {
            RegistryClient::Http(client) => client.get_config().await,
            RegistryClient::Local(client) => client.get_config().await,
        }
    }

    async fn signup(
        &self,
        email: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<()> {
        match &self {
            RegistryClient::Http(client) => client.signup(email, username, password, duckity).await,
            RegistryClient::Local(client) => {
                client.signup(email, username, password, duckity).await
            }
        }
    }

    async fn signin(
        &self,
        username: impl Into<String>,
        password: impl Into<String>,
        duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<Credentials> {
        match &self {
            RegistryClient::Http(client) => client.signin(username, password, duckity).await,
            RegistryClient::Local(client) => client.signin(username, password, duckity).await,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryKind {
    Http,
    Local,
}
