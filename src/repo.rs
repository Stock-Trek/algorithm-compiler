use crate::{
    constants::*,
    dto::{
        errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
        sqs::SqsMessage,
    },
    dynamodb::{acquire_lock, release_lock},
    s3,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::info;

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
    let archive = repo_archive_path(&message.repo);
    let client = s3::s3_client().await;
    s3::download_repo(
        &client,
        S3_BUCKET_UPLOADS,
        &repo_key(&message.repo),
        &archive,
        &dir,
    )
    .await?;
    update_repo(message, &dir)?;
    add_ref(message, &dir)?;
    s3::upload_repo(
        &client,
        S3_BUCKET_UPLOADS,
        &repo_key(&message.repo),
        &dir,
        &archive,
    )
    .await?;
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
