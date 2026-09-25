use crate::{error::ACResult, files::Files, program::Program};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub clone_url: String,
    pub ref_name: Option<String>,
    pub commit_hash: Option<String>,
}

impl GitRepo {
    pub fn new(
        account: &str,
        repo: &str,
        branch_name: Option<&str>,
        commit_hash: Option<&str>,
    ) -> Self {
        Self {
            clone_url: format!("https://github.com/{account}/{repo}.git"),
            ref_name: branch_name.map(|branch| format!("refs/stock-trek/{branch}")),
            commit_hash: commit_hash.map(str::to_string),
        }
    }

    pub fn clone_bare(&self, path: &Path) -> ACResult<String> {
        let destination = Files::path_str(path)?;
        self.exec_git(path, &["clone", "--bare", &self.clone_url, destination])
    }

    pub fn fetch(&self, path: &Path) -> ACResult<String> {
        self.exec_git(path, &["fetch", "--prune", "--tags", "origin"])
    }

    pub fn create_ref(&self, path: &Path) -> ACResult<String> {
        match (&self.ref_name, &self.commit_hash) {
            (Some(ref_name), Some(commit_hash)) => {
                self.exec_git(path, &["update-ref", ref_name, commit_hash])
            }
            _ => Ok(String::new()),
        }
    }

    fn exec_git(&self, path: &Path, args: &[&str]) -> ACResult<String> {
        Program::run("git", args, path)
    }
}
