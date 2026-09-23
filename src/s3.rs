use crate::{
    constants::REPO_ARCHIVE_DIR,
    dto::errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_smithy_types::byte_stream::ByteStream;
use std::{
    fs,
    path::Path,
    process::Command,
};

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
        .map_err(|e| internal_server_e("Failed to read byte stream from file {}", e))?;
    s3_client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(byte_stream)
        .send()
        .await
        .map_err(|e| internal_server_e("Failed to save file {}", e))?;
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
                .map_err(|e| internal_server_e("Failed to read downloaded object {}", e))?
                .into_bytes();
            tokio::fs::write(file_path, &bytes)
                .await
                .map_err(|e| internal_server_e("Failed to write downloaded object {}", e))?;
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
                Err(internal_server_e("Failed to download object {}", error))
            }
        }
    }
}

pub async fn download_repo(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    archive_path: &Path,
    dir: &Path,
) -> Result<bool, StockTrekCompileAlgorithmError> {
    tokio::fs::create_dir_all(REPO_ARCHIVE_DIR)
        .await
        .map_err(|e| internal_server_e("Failed to create archive directory {}", e))?;
    let downloaded = download_file(s3_client, bucket, key, archive_path).await?;
    if !downloaded {
        return Ok(false);
    }
    if dir.exists() {
        fs::remove_dir_all(dir)
            .map_err(|e| internal_server_e("Failed to clear repo directory {}", e))?;
    }
    fs::create_dir_all(dir)
        .map_err(|e| internal_server_e("Failed to create repo directory {}", e))?;
    extract_archive(archive_path, dir)?;
    Ok(true)
}

pub async fn upload_repo(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    dir: &Path,
    archive_path: &Path,
) -> Result<(), StockTrekCompileAlgorithmError> {
    create_archive(dir, archive_path)?;
    upload_file(s3_client, bucket, key, archive_path).await
}

fn extract_archive(archive: &Path, dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    run_tar(&["xzf", path_str(archive)?, "-C", path_str(dir)?])
}

fn create_archive(dir: &Path, archive: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    fs::create_dir_all(REPO_ARCHIVE_DIR)
        .map_err(|e| internal_server_e("Failed to create archive directory {}", e))?;
    run_tar(&["czf", path_str(archive)?, "-C", path_str(dir)?, "."])
}

fn run_tar(args: &[&str]) -> Result<(), StockTrekCompileAlgorithmError> {
    let output = Command::new("tar")
        .args(args)
        .output()
        .map_err(|e| internal_server_e("Failed to run tar {}", e))?;
    if !output.status.success() {
        return Err(internal_server(&format!(
            "tar {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn path_str(path: &Path) -> Result<&str, StockTrekCompileAlgorithmError> {
    path.to_str()
        .ok_or_else(|| internal_server("Path is not valid utf-8"))
}
