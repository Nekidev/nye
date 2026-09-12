use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::SharedCredentialsProvider;
use axum::Router;
use url::Url;

use crate::args::{
    RegistrySubcommandRunSubcommandArgsStorageLocal, RegistrySubcommandRunSubcommandArgsStorageS3,
};
use crate::registries::http::server::storage::backends::local::LocalStorageBackend;
use crate::registries::http::server::storage::backends::s3::S3StorageBackend;

pub mod backends;

pub trait Storage {
    /// Returns a router to be nested under `/storage/`.
    ///
    /// This is used, for example, by the local storage backend to serve files.
    fn router(&self) -> Router {
        Router::new()
    }

    /// Stores a file object in the backend.
    ///
    /// Arguments:
    /// * `key` - AKA filename.
    /// * `filename` - The path to the file in the local system.
    async fn set_object(&self, key: impl Into<String>, filename: impl Into<String>) -> anyhow::Result<()>;

    /// Returns a URL that can be used by clients to download the file.
    ///
    /// Arguments:
    /// * `key` - AKA filename.
    async fn get_object_url(&self, key: impl Into<String>) -> anyhow::Result<Url>;
}

#[derive(Debug)]
pub enum StorageBackend {
    S3(S3StorageBackend),
    Local(LocalStorageBackend),
}

impl Storage for StorageBackend {
    async fn set_object(&self, key: impl Into<String>, filename: impl Into<String>) -> anyhow::Result<()> {
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

impl From<RegistrySubcommandRunSubcommandArgsStorageS3> for StorageBackend {
    fn from(value: RegistrySubcommandRunSubcommandArgsStorageS3) -> Self {
        let config = aws_config::SdkConfig::builder()
            .endpoint_url(value.endpoint_url)
            .region(Some(Region::new(value.region)))
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                value.access_key,
                value.secret_key,
                None,
                None,
                "manual",
            )))
            .build();

        Self::S3(S3StorageBackend {
            client: Client::new(&config),
            bucket: value.bucket_name,
        })
    }
}

impl From<RegistrySubcommandRunSubcommandArgsStorageLocal> for StorageBackend {
    fn from(value: RegistrySubcommandRunSubcommandArgsStorageLocal) -> Self {
        Self::Local(LocalStorageBackend {
            location: value.location,
        })
    }
}
