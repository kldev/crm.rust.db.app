//! The bytes of desk attachments in a private S3 bucket (ADR-0009). Path-style URLs and checksums only when an
//! operation requires them: the SDK's default CRC32 trailers are not understood by every S3-compatible server (RustFS).
//! The bucket is created by the deployment (rustfs-init locally), never by the application.

use std::time::Duration;

use aws_sdk_s3::{
    Client,
    config::{
        Builder, Credentials, Region, RequestChecksumCalculation, ResponseChecksumValidation,
        timeout::TimeoutConfig,
    },
    primitives::ByteStream,
};

#[derive(Clone, Debug)]
pub struct StorageConfig {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    pub fn new(cfg: &StorageConfig) -> Self {
        let config = Builder::new()
            .behavior_version_latest()
            .endpoint_url(&cfg.endpoint)
            .region(Region::new(cfg.region.clone()))
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .credentials_provider(Credentials::new(
                &cfg.access_key,
                &cfg.secret_key,
                None,
                None,
                "crm",
            ))
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(Duration::from_secs(3))
                    .operation_attempt_timeout(Duration::from_secs(60))
                    .build(),
            )
            .build();
        Storage {
            client: Client::from_conf(config),
            bucket: cfg.bucket.clone(),
        }
    }

    pub async fn put(
        &self,
        key: &str,
        bytes: bytes::Bytes,
        content_type: &str,
    ) -> anyhow::Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .content_length(bytes.len() as i64)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| {
                anyhow::anyhow!("put {key}: {}", aws_sdk_s3::error::DisplayErrorContext(e))
            })?;
        Ok(())
    }

    pub async fn open(&self, key: &str) -> anyhow::Result<ByteStream> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                anyhow::anyhow!("get {key}: {}", aws_sdk_s3::error::DisplayErrorContext(e))
            })?;
        Ok(object.body)
    }

    /// Removes objects the database refused to record. Best effort: an orphan object is harmless, a lost event is
    /// not. One request per object (DeleteObjects always needs a checksum header).
    pub async fn delete(&self, keys: &[String]) {
        for key in keys {
            if let Err(e) = self
                .client
                .delete_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
            {
                tracing::warn!(key, error = %aws_sdk_s3::error::DisplayErrorContext(e),
                    "could not delete a refused attachment object; it stays as an orphan");
            }
        }
    }

    /// Readiness: the bucket answers with these keys.
    pub async fn ping(&self) -> bool {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .is_ok()
    }
}
