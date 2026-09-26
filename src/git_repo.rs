use crate::{
    dto::sqs_event::SqsRefType, error::ACResult, files::Files, program::Program, timeouts::Timeouts,
};
use std::{path::Path, time::SystemTime};

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub clone_url: String,
    pub commit: Option<GitCommit>,
    timeouts: Timeouts,
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub ref_name: String,
    pub commit_hash: String,
}

impl GitRepo {
    pub fn new(clone_url: &str, commit: Option<(&str, &str)>, timeouts: Timeouts) -> Self {
        Self {
            clone_url: clone_url.into(),
            commit: commit.map(|(branch_name, commit_hash)| GitCommit {
                ref_name: format!("refs/stock-trek/{branch_name}-{commit_hash}"),
                commit_hash: commit_hash.to_string(),
            }),
            timeouts,
        }
    }

    pub async fn clone_bare(&self, path: &Path, deadline: SystemTime) -> ACResult<String> {
        let destination = Files::path_str(path)?;
        self.exec_git(
            path,
            &["clone", "--bare", &self.clone_url, destination],
            deadline,
        )
        .await
    }

    pub async fn fetch(&self, path: &Path, deadline: SystemTime) -> ACResult<String> {
        self.exec_git(path, &["fetch", "--prune", "--tags", "origin"], deadline)
            .await
    }

    pub async fn set_remote(&self, path: &Path, deadline: SystemTime) -> ACResult<String> {
        self.exec_git(
            path,
            &["remote", "set-url", "origin", &self.clone_url],
            deadline,
        )
        .await
    }

    pub async fn create_ref(&self, path: &Path, deadline: SystemTime) -> ACResult<String> {
        match &self.commit {
            Some(commit) => {
                self.exec_git(
                    path,
                    &["update-ref", "--", &commit.ref_name, &commit.commit_hash],
                    deadline,
                )
                .await
            }
            None => Ok(String::new()),
        }
    }

    pub async fn add_ref(
        &self,
        path: &Path,
        ref_name: &str,
        ref_type: SqsRefType,
        deadline: SystemTime,
    ) -> ACResult<String> {
        let full_name = Self::full_ref_name(ref_name, ref_type);
        let refspec = format!("+{full_name}:{full_name}");
        self.exec_git(path, &["fetch", "origin", &refspec], deadline)
            .await
    }

    pub async fn delete_ref(
        &self,
        path: &Path,
        ref_name: &str,
        ref_type: SqsRefType,
        deadline: SystemTime,
    ) -> ACResult<String> {
        let full_name = Self::full_ref_name(ref_name, ref_type);
        self.exec_git(path, &["update-ref", "-d", "--", &full_name], deadline)
            .await
    }

    fn full_ref_name(ref_name: &str, ref_type: SqsRefType) -> String {
        let prefix = match ref_type {
            SqsRefType::Branch => "refs/heads",
            SqsRefType::Tag => "refs/tags",
        };
        format!("{prefix}/{ref_name}")
    }

    async fn exec_git(&self, path: &Path, args: &[&str], deadline: SystemTime) -> ACResult<String> {
        let timeout = self.timeouts.command_for(deadline)?;
        Program::run_with_timeout("git", args, path, timeout).await
    }
}
