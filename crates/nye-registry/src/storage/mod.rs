use anyhow::Context;
use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::SharedCredentialsProvider;
use axum::Router;
use nye_utils::permissions;
use url::Url;

use crate::args::{RunSubcommandArgsStorageLocal, RunSubcommandArgsStorageS3};
use crate::storage::backends::local::LocalStorageBackend;
use crate::storage::backends::s3::S3StorageBackend;

pub mod backends;

pub trait Storage {
    /// Returns a router to be nested under `/storage/`.
    ///
    /// This is used, for example, by the local storage backend to serve files.
    fn router<T>(&self) -> Router<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        Router::new()
    }

    /// Stores a file object in the backend.
    ///
    /// Arguments:
    /// * `key` - AKA filename.
    /// * `filename` - The path to the file in the local system.
    fn set_object(
        &self,
        key: impl Into<String>,
        filename: impl Into<String>,
    ) -> impl Future<Output = anyhow::Result<()>>;

    /// Returns a URL that can be used by clients to download the file.
    ///
    /// Arguments:
    /// * `key` - AKA filename.
    fn get_object_url(&self, key: impl Into<String>) -> impl Future<Output = anyhow::Result<Url>>;
}

#[derive(Debug)]
pub enum StorageBackend {
    S3(S3StorageBackend),
    Local(LocalStorageBackend),
}

impl StorageBackend {
    pub async fn from_local_args(args: RunSubcommandArgsStorageLocal) -> anyhow::Result<Self> {
        let permissions = permissions::get_current_user_permissions(&args.location)
            .context("Could not get permissions for local storage location dir.")?;

        if !permissions.read {
            anyhow::bail!(
                "You don't have permission to read files in the local storage directory."
            );
        }
        if !permissions.write {
            anyhow::bail!(
                "You don't have permission to write files in the local storage directory."
            );
        }

        Ok(Self::Local(LocalStorageBackend {
            location: args.location,
        }))
    }

    pub async fn from_s3_args(args: RunSubcommandArgsStorageS3) -> anyhow::Result<Self> {
        let config = aws_config::SdkConfig::builder()
            .endpoint_url(args.endpoint_url)
            .region(Some(Region::new(args.region)))
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                args.access_key,
                args.secret_key,
                None,
                None,
                "manual",
            )))
            .build();

        Ok(Self::S3(S3StorageBackend {
            client: Client::new(&config),
            bucket: args.bucket_name,
        }))
    }
}

impl Storage for StorageBackend {
    async fn set_object(
        &self,
        key: impl Into<String>,
        filename: impl Into<String>,
    ) -> anyhow::Result<()> {
        match &self {
            Self::S3(backend) => backend.set_object(key, filename).await,
            Self::Local(backend) => backend.set_object(key, filename).await,
        }
    }

    async fn get_object_url(&self, key: impl Into<String>) -> anyhow::Result<Url> {
        match &self {
            Self::S3(backend) => backend.get_object_url(key).await,
            Self::Local(backend) => backend.get_object_url(key).await,
        }
    }
}
