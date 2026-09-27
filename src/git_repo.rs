use crate::{
    dto::sqs_event::SqsRefType, error::ACResult, files::Files, program::Program, timeouts::Timeouts,
};
use std::{path::Path, time::SystemTime};

const STOCK_TREK_REF_PREFIX: &str = "refs/stock-trek";

#[derive(Debug, Clone)]
pub struct GitRepo {
    timeouts: Timeouts,
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub ref_name: String,
    pub commit_hash: String,
}

impl GitRepo {
    pub fn new(timeouts: Timeouts) -> Self {
        Self { timeouts }
    }

    pub async fn clone_bare(
        &self,
        clone_url: &str,
        path: &Path,
        deadline: SystemTime,
    ) -> ACResult<String> {
        let destination = Files::path_str(path)?;
        self.exec_git(path, &["clone", "--bare", clone_url, destination], deadline)
            .await
    }

    pub async fn fetch(&self, path: &Path, deadline: SystemTime) -> ACResult<String> {
        self.exec_git(path, &["fetch", "--prune", "--tags", "origin"], deadline)
            .await
    }

    pub async fn create_ref(
        &self,
        ref_name: &str,
        commit_hash: &str,
        path: &Path,
        deadline: SystemTime,
    ) -> ACResult<String> {
        self.exec_git(path, &["update-ref", "--", ref_name, commit_hash], deadline)
            .await
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
            .await?;
        let commit_hash = self
            .exec_git(
                path,
                &["rev-parse", &format!("{full_name}^{{commit}}")],
                deadline,
            )
            .await?;
        let commit_hash = commit_hash.trim();
        let stock_trek_ref = Self::stock_trek_ref_name(ref_name, commit_hash);
        self.exec_git(
            path,
            &["update-ref", "--", &stock_trek_ref, commit_hash],
            deadline,
        )
        .await
    }

    pub async fn delete_ref(
        &self,
        path: &Path,
        ref_name: &str,
        ref_type: SqsRefType,
        deadline: SystemTime,
    ) -> ACResult<String> {
        let prefix = format!("{STOCK_TREK_REF_PREFIX}/{ref_name}-");
        let stock_trek_refs = self
            .exec_git(
                path,
                &["for-each-ref", "--format=%(refname)", STOCK_TREK_REF_PREFIX],
                deadline,
            )
            .await?;
        for stock_trek_ref in stock_trek_refs
            .lines()
            .filter(|stock_trek_ref| Self::is_stock_trek_ref(stock_trek_ref, &prefix))
        {
            self.exec_git(path, &["update-ref", "-d", "--", stock_trek_ref], deadline)
                .await?;
        }
        let full_name = Self::full_ref_name(ref_name, ref_type);
        self.exec_git(path, &["update-ref", "-d", "--", &full_name], deadline)
            .await
    }

    fn stock_trek_ref_name(ref_name: &str, commit_hash: &str) -> String {
        format!("{STOCK_TREK_REF_PREFIX}/{ref_name}-{commit_hash}")
    }

    fn is_stock_trek_ref(stock_trek_ref: &str, prefix: &str) -> bool {
        stock_trek_ref.strip_prefix(prefix).is_some_and(|hash| {
            hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
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
