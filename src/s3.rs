use crate::dto::errors::{StockTrekCompileAlgorithmError, internal_server_e};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_smithy_types::byte_stream::ByteStream;

pub async fn s3_client() -> S3Client {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    S3Client::new(&config)
}

pub async fn upload_file(
    s3_client: &S3Client,
    bucket: &impl AsRef<str>,
    key: &impl AsRef<str>,
    file_path: &impl AsRef<str>,
) -> Result<(), StockTrekCompileAlgorithmError> {
    let byte_stream = ByteStream::from_path(file_path.as_ref())
        .await
        .map_err(|e| internal_server_e("Failed to read byte stream from file {}", e))?;
    s3_client
        .put_object()
        .bucket(bucket.as_ref())
        .key(key.as_ref())
        .body(byte_stream)
        .send()
        .await
        .map_err(|e| internal_server_e("Failed to save file {}", e))?;
    Ok(())
}
