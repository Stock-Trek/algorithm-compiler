use crate::{
    constants::*,
    dto::{
        errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
        sqs::SqsMessage,
    },
    s3,
};
use aws_sdk_dynamodb::{Client as DynamoDbClient, types::AttributeValue};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tracing::{info, warn};
use uuid::Uuid;

struct RepoLock {
    client: DynamoDbClient,
    table: String,
    repo: String,
    token: String,
}

pub async fn prepare_repo(message: &SqsMessage) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Preparing repo {}", message.repo);
    let lock = acquire_lock(&message.repo).await?;
    let sync_result = sync_repo(message).await;
    let release_result = release_lock(&lock).await;
    sync_result?;
    release_result?;
    Ok(())
}

async fn sync_repo(message: &SqsMessage) -> Result<(), StockTrekCompileAlgorithmError> {
    let dir = repo_dir(&message.repo);
    let client = s3::s3_client().await;
    download_repo(&client, &message.repo, &dir).await?;
    update_repo(message, &dir)?;
    add_ref(message, &dir)?;
    upload_repo(&client, &message.repo, &dir).await?;
    Ok(())
}

async fn acquire_lock(repo: &str) -> Result<RepoLock, StockTrekCompileAlgorithmError> {
    let client = dynamodb_client().await;
    let table = lock_table();
    let token = Uuid::new_v4().to_string();
    for _ in 0..LOCK_RETRIES {
        let result = client
            .put_item()
            .table_name(&table)
            .item("repo", AttributeValue::S(repo.to_string()))
            .item("lock_id", AttributeValue::S(token.clone()))
            .condition_expression("attribute_not_exists(repo)")
            .send()
            .await;
        match result {
            Ok(_) => {
                return Ok(RepoLock {
                    client,
                    table,
                    repo: repo.to_string(),
                    token,
                });
            }
            Err(error) => {
                if error
                    .as_service_error()
                    .map(|e| e.is_conditional_check_failed_exception())
                    .unwrap_or(false)
                {
                    warn!("Lock for repo {} is held, retrying", repo);
                    tokio::time::sleep(Duration::from_millis(LOCK_RETRY_DELAY_MS)).await;
                } else {
                    return Err(internal_server_e("Failed to acquire repo lock {}", error));
                }
            }
        }
    }
    Err(internal_server(&format!(
        "Timed out acquiring lock for repo {}",
        repo
    )))
}

async fn release_lock(lock: &RepoLock) -> Result<(), StockTrekCompileAlgorithmError> {
    lock.client
        .delete_item()
        .table_name(&lock.table)
        .key("repo", AttributeValue::S(lock.repo.clone()))
        .condition_expression("lock_id = :lock_id")
        .expression_attribute_values(":lock_id", AttributeValue::S(lock.token.clone()))
        .send()
        .await
        .map_err(|e| internal_server_e("Failed to release repo lock {}", e))?;
    Ok(())
}

async fn dynamodb_client() -> DynamoDbClient {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    DynamoDbClient::new(&config)
}

fn lock_table() -> String {
    std::env::var("REPO_LOCK_TABLE").unwrap_or_else(|_| DEFAULT_LOCK_TABLE.to_string())
}

async fn download_repo(
    client: &aws_sdk_s3::Client,
    repo: &str,
    dir: &Path,
) -> Result<(), StockTrekCompileAlgorithmError> {
    fs::create_dir_all(REPO_ARCHIVE_DIR)
        .map_err(|e| internal_server_e("Failed to create archive directory {}", e))?;
    let archive = repo_archive_path(repo);
    let downloaded =
        s3::download_file(client, S3_BUCKET_UPLOADS, &repo_key(repo), &archive).await?;
    if downloaded {
        if dir.exists() {
            fs::remove_dir_all(dir)
                .map_err(|e| internal_server_e("Failed to clear repo directory {}", e))?;
        }
        fs::create_dir_all(dir)
            .map_err(|e| internal_server_e("Failed to create repo directory {}", e))?;
        extract_repo(&archive, dir)?;
    }
    Ok(())
}

fn update_repo(message: &SqsMessage, dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    if !dir.join(".git").exists() {
        fs::create_dir_all(dir)
            .map_err(|e| internal_server_e("Failed to create repo directory {}", e))?;
        git(&["init"], dir)?;
    }
    let remote = remote_url(message)?;
    if has_remote(dir) {
        git(&["remote", "set-url", "origin", &remote], dir)?;
    } else {
        git(&["remote", "add", "origin", &remote], dir)?;
    }
    git(&["fetch", "--prune", "--tags", "origin"], dir)?;
    Ok(())
}

fn add_ref(message: &SqsMessage, dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    let branch = branch_name(message);
    let ref_name = format!("refs/heads/{}", branch);
    git(&["update-ref", &ref_name, &message.after], dir)?;
    git(&["checkout", "-f", &branch], dir)?;
    Ok(())
}

async fn upload_repo(
    client: &aws_sdk_s3::Client,
    repo: &str,
    dir: &Path,
) -> Result<(), StockTrekCompileAlgorithmError> {
    let archive = repo_archive_path(repo);
    create_archive(dir, &archive)?;
    s3::upload_file(client, S3_BUCKET_UPLOADS, &repo_key(repo), &archive).await
}

fn remote_url(message: &SqsMessage) -> Result<String, StockTrekCompileAlgorithmError> {
    if let Some((prefix, _)) = message.url.split_once("/commit/")
        && !prefix.is_empty()
    {
        return Ok(format!("{}.git", prefix.trim_end_matches('/')));
    }
    if message.repo.contains('/') {
        return Ok(format!("https://github.com/{}.git", message.repo));
    }
    Err(internal_server(&format!(
        "Could not determine remote url for repo {}",
        message.repo
    )))
}

fn branch_name(message: &SqsMessage) -> String {
    message
        .r#ref
        .strip_prefix("refs/heads/")
        .unwrap_or(&message.r#ref)
        .to_string()
}

pub fn repo_dir(repo: &str) -> PathBuf {
    Path::new(REPO_CACHE_DIR).join(sanitize(repo))
}

fn repo_key(repo: &str) -> String {
    format!("repos/{}.tar.gz", repo)
}

fn repo_archive_path(repo: &str) -> PathBuf {
    Path::new(REPO_ARCHIVE_DIR).join(format!("{}.tar.gz", sanitize(repo)))
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            other => other,
        })
        .collect()
}

fn git(args: &[&str], dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| internal_server_e("Failed to run git {}", e))?;
    if !output.status.success() {
        return Err(internal_server(&format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn has_remote(dir: &Path) -> bool {
    Command::new("git")
        .args(["remote"])
        .current_dir(dir)
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == "origin")
        })
        .unwrap_or(false)
}

fn extract_repo(archive: &Path, dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
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
