use crate::{
    archive,
    constants::*,
    dto::{
        errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
        sqs::SqsMessage,
    },
    dynamodb, s3,
};
use aws_sdk_s3::Client as S3Client;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::info;

const LOCK_KEY_ATTRIBUTE: &str = "repo";

pub async fn prepare_repo(message: &SqsMessage) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Preparing repo {}", message.repo);
    let client = dynamodb::dynamodb_client().await;
    let table = lock_table();
    let lock = dynamodb::acquire_lock(&client, &table, LOCK_KEY_ATTRIBUTE, &message.repo).await?;
    let sync_result = sync_repo(message).await;
    let release_result = dynamodb::release_lock(&client, &table, &lock).await;
    sync_result?;
    release_result?;
    Ok(())
}

async fn sync_repo(message: &SqsMessage) -> Result<(), StockTrekCompileAlgorithmError> {
    let dir = repo_dir(&message.repo);
    let archive = repo_archive_path(&message.repo);
    let client = s3::s3_client().await;
    download_repo(&client, &message.repo, &archive, &dir).await?;
    update_repo(message, &dir)?;
    add_ref(message, &dir)?;
    upload_repo(&client, &message.repo, &dir, &archive).await?;
    Ok(())
}

async fn download_repo(
    s3_client: &S3Client,
    repo: &str,
    archive_path: &Path,
    dir: &Path,
) -> Result<bool, StockTrekCompileAlgorithmError> {
    fs::create_dir_all(REPO_ARCHIVE_DIR)
        .map_err(|e| internal_server_e("Failed to create archive directory {}", e))?;
    let downloaded =
        s3::download_file(s3_client, S3_BUCKET_UPLOADS, &repo_key(repo), archive_path).await?;
    if !downloaded {
        return Ok(false);
    }
    if dir.exists() {
        fs::remove_dir_all(dir)
            .map_err(|e| internal_server_e("Failed to clear repo directory {}", e))?;
    }
    fs::create_dir_all(dir)
        .map_err(|e| internal_server_e("Failed to create repo directory {}", e))?;
    archive::extract_archive(archive_path, dir)?;
    Ok(true)
}

async fn upload_repo(
    s3_client: &S3Client,
    repo: &str,
    dir: &Path,
    archive_path: &Path,
) -> Result<(), StockTrekCompileAlgorithmError> {
    archive::create_archive(dir, archive_path)?;
    s3::upload_file(s3_client, S3_BUCKET_UPLOADS, &repo_key(repo), archive_path).await
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

fn lock_table() -> String {
    std::env::var("REPO_LOCK_TABLE").unwrap_or_else(|_| DYNAMOD_DB_LOCK_TABLE.to_string())
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
