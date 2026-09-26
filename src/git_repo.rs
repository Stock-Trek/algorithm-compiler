use crate::{dto::sqs_event::GitProvider, error::ACResult, files::Files, program::Program};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub clone_url: String,
    pub commit: Option<GitCommit>,
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub ref_name: String,
    pub commit_hash: String,
}

impl GitRepo {
    pub fn new(
        provider: &GitProvider,
        account: &str,
        repo: &str,
        commit: Option<(&str, &str)>,
    ) -> Self {
        Self {
            clone_url: provider.clone_url(account, repo),
            commit: commit.map(|(branch_name, commit_hash)| GitCommit {
                ref_name: format!("refs/stock-trek/{branch_name}-{commit_hash}"),
                commit_hash: commit_hash.to_string(),
            }),
        }
    }

    pub async fn clone_bare(&self, path: &Path) -> ACResult<String> {
        let destination = Files::path_str(path)?;
        self.exec_git(path, &["clone", "--bare", &self.clone_url, destination])
            .await
    }

    pub async fn fetch(&self, path: &Path) -> ACResult<String> {
        self.exec_git(path, &["fetch", "--prune", "--tags", "origin"])
            .await
    }

    pub async fn set_remote(&self, path: &Path) -> ACResult<String> {
        self.exec_git(path, &["remote", "set-url", "origin", &self.clone_url])
            .await
    }

    pub async fn create_ref(&self, path: &Path) -> ACResult<String> {
        match &self.commit {
            Some(commit) => {
                self.exec_git(path, &["update-ref", &commit.ref_name, &commit.commit_hash])
                    .await
            }
            None => Ok(String::new()),
        }
    }

    async fn exec_git(&self, path: &Path, args: &[&str]) -> ACResult<String> {
        Program::run("git", args, path).await
    }
}
