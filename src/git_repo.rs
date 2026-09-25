use crate::{error::ACResult, files::Files, program::Program};
use std::path::PathBuf;
use url::Url;

#[derive(Debug)]
pub struct GitRepo {
    pub clone_url: Url,
    pub repo_id: u64,
    pub full_name: String,
    pub name: String,
    pub ref_name: String,
    pub branch_name: String,
    pub commit_hash: String,
}

impl GitRepo {
    pub fn clone_bare(&self, path: &PathBuf) -> ACResult<String> {
        let dest_dir = Files::path_str(path)?;
        self.exec_git(
            path,
            &["clone", "--bare", self.clone_url.as_str(), dest_dir],
        )
    }
    pub fn fetch(&self, path: &PathBuf) -> ACResult<String> {
        self.exec_git(path, &["fetch", "--prune", "--tags", "origin"])
    }
    pub fn create_ref(&self, path: &PathBuf) -> ACResult<String> {
        self.exec_git(path, &["update-ref", &self.ref_name, &self.commit_hash])
    }
    fn exec_git(&self, path: &PathBuf, args: &[&str]) -> ACResult<String> {
        Program::run("git", args, path)
    }
}
