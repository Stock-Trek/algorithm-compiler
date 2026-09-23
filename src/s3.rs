use crate::dto::errors::{StockTrekCompileAlgorithmError, internal_server_e};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_smithy_types::byte_stream::ByteStream;
use std::path::Path;

pub async fn s3_client() -> S3Client {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    S3Client::new(&config)
}

pub async fn upload_file(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    file_path: &Path,
) -> Result<(), StockTrekCompileAlgorithmError> {
    let byte_stream = ByteStream::from_path(file_path)
        .await
        .map_err(|e| internal_server_e("Failed to read byte stream from file", e))?;
    s3_client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(byte_stream)
        .send()
        .await
        .map_err(|e| internal_server_e("Failed to save file", e))?;
    Ok(())
}

pub async fn download_file(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    file_path: &Path,
) -> Result<bool, StockTrekCompileAlgorithmError> {
    let result = s3_client.get_object().bucket(bucket).key(key).send().await;
    match result {
        Ok(output) => {
            let bytes = output
                .body
                .collect()
                .await
                .map_err(|e| internal_server_e("Failed to read downloaded object", e))?
                .into_bytes();
            tokio::fs::write(file_path, &bytes)
                .await
                .map_err(|e| internal_server_e("Failed to write downloaded object", e))?;
            Ok(true)
        }
        Err(error) => {
            if error
                .as_service_error()
                .map(|e| e.is_no_such_key())
                .unwrap_or(false)
            {
                Ok(false)
            } else {
                Err(internal_server_e("Failed to download object", error))
            }
        }
    }
}
