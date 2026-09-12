use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use aws_sdk_s3::Client;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use url::Url;

use crate::registries::http::server::storage::Storage;

#[derive(Debug)]
pub struct S3StorageBackend {
    pub client: Client,
    pub bucket: String,
}

impl Storage for S3StorageBackend {
    async fn set_object(
        &self,
        key: impl Into<String>,
        filename: impl Into<String>,
    ) -> anyhow::Result<()> {
        let key = key.into();

        self.client
            .put_object()
            .bucket(&self.bucket)
            .body(
                ByteStream::from_path(filename.into())
                    .await
                    .context("Could not read local file from filename to upload to S3.")?,
            )
            .key(key.clone())
            .send()
            .await
            .context(format!("Could not store file `{key}` in S3 storage."));

        Ok(())
    }

    async fn get_object_url(&self, key: impl Into<String>) -> anyhow::Result<Url> {
        let key = key.into();

        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key.clone())
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_mins(10))
                    .build()
                    .expect("Could not create URL presigning config."),
            )
            .await
            .context(format!("Could not get presigned URL of object `{key}`."))?;

        let url = Url::from_str(request.uri())
            .expect("The URL returned by the S3 client was not a valid URL.");

        Ok(url)
    }
}
