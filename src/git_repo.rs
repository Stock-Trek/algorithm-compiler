use crate::{error::ACResult, files::Files, program::Program};
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
    pub fn new(account: &str, repo: &str, commit: Option<(&str, &str)>) -> Self {
        Self {
            clone_url: format!("https://github.com/{account}/{repo}.git"),
            commit: commit.map(|(branch_name, commit_hash)| GitCommit {
                ref_name: format!("refs/stock-trek/{branch_name}-{commit_hash}"),
                commit_hash: commit_hash.to_string(),
            }),
        }
    }

    pub fn clone_bare(&self, path: &Path) -> ACResult<String> {
        let destination = Files::path_str(path)?;
        self.exec_git(path, &["clone", "--bare", &self.clone_url, destination])
    }

    pub fn fetch(&self, path: &Path) -> ACResult<String> {
        self.exec_git(path, &["fetch", "--prune", "--tags", "origin"])
    }

    pub fn set_remote(&self, path: &Path) -> ACResult<String> {
        self.exec_git(path, &["remote", "set-url", "origin", &self.clone_url])
    }

    pub fn create_ref(&self, path: &Path) -> ACResult<String> {
        match &self.commit {
            Some(commit) => {
                self.exec_git(path, &["update-ref", &commit.ref_name, &commit.commit_hash])
            }
            None => Ok(String::new()),
        }
    }

    fn exec_git(&self, path: &Path, args: &[&str]) -> ACResult<String> {
        Program::run("git", args, path)
    }
}
