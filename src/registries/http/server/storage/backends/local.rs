use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use axum::Router;
use tokio::fs;
use tower_http::services::ServeDir;
use url::Url;

use crate::registries::http::server::storage::Storage;

#[derive(Debug)]
pub struct LocalStorageBackend {
    pub location: PathBuf,
}

impl Storage for LocalStorageBackend {
    fn router(&self) -> Router {
        Router::new().nest_service("/", ServeDir::new(&self.location))
    }

    async fn set_object(&self, key: impl Into<String>, filename: impl Into<String>) -> anyhow::Result<()> {
        let key = key.into();
        let path = self.location.join(&key);

        fs::copy(path, filename.into())
            .await
            .context("Could not write file to local directory.")?;

        Ok(())
    }

    async fn get_object_url(&self, key: impl Into<String>) -> anyhow::Result<Url> {
        let key = key.into();
        Url::from_str(&format!("/storage/{key}"))
            .context(format!("Could not get local storage URL for file `{key}`."))
    }
}
