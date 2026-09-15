use std::path::PathBuf;

use url::Url;

use crate::registries::{Registry, RegistryConfig, config};

pub struct LocalRegistryClient {
    base_path: PathBuf,
}

impl LocalRegistryClient {
    pub fn new(config: &config::RegistryConfig) -> Self {
        Self {
            base_path: PathBuf::from(config.source.path()),
        }
    }

    pub fn from_url(url: Url) -> anyhow::Result<Self> {
        match url.to_file_path() {
            Ok(path) => Ok(LocalRegistryClient { base_path: path }),
            Err(_) => anyhow::bail!("The URL provided could not be converted into a file path."),
        }
    }
}

impl Registry for LocalRegistryClient {
    async fn get_config(&self) -> anyhow::Result<RegistryConfig> {
        Ok(RegistryConfig {
            name: String::from("Registry in Local Directory"),
            is_signin_enabled: false,
            is_signup_enabled: false,
            duckity_signin_policy_id: None,
            duckity_signup_policy_id: None,
        })
    }

    async fn signup(
        &self,
        _email: impl Into<String>,
        _username: impl Into<String>,
        _password: impl Into<String>,
        _duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("Sign ups are not supported in local registries.");
    }

    async fn signin(
        &self,
        _username: impl Into<String>,
        _password: impl Into<String>,
        _duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<crate::registries::Credentials> {
        anyhow::bail!("Signing in is not supported in local registries.");
    }
}
