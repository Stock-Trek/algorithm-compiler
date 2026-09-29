use crate::{dto::sqs_event::SqsRefType, error::ACResult, program::Program, timeouts::Timeouts};
use std::{path::Path, time::SystemTime};

const STOCK_TREK_REF_PREFIX: &str = "refs/stock-trek";
const SHA1_HEX_LENGTH: usize = 40;
const SHA256_HEX_LENGTH: usize = 64;

pub struct GitLocal;

impl GitLocal {
    pub fn stock_trek_ref_name(ref_name: &str, commit_hash: &str) -> String {
        format!("{STOCK_TREK_REF_PREFIX}/{ref_name}-{commit_hash}")
    }

    pub async fn fetch(
        &self,
        timeouts: Timeouts,
        path: &Path,
        deadline: SystemTime,
    ) -> ACResult<String> {
        Self::exec_git(
            timeouts,
            path,
            &["fetch", "--prune", "--tags", "origin"],
            deadline,
        )
        .await
    }

    pub async fn set_remote(
        &self,
        timeouts: Timeouts,
        path: &Path,
        remote_url: &str,
        deadline: SystemTime,
    ) -> ACResult<String> {
        Self::exec_git(
            timeouts,
            path,
            &["remote", "set-url", "origin", remote_url],
            deadline,
        )
        .await
    }

    pub async fn create_ref(
        &self,
        timeouts: Timeouts,
        ref_name: &str,
        commit_hash: &str,
        path: &Path,
        deadline: SystemTime,
    ) -> ACResult<String> {
        Self::exec_git(
            timeouts,
            path,
            &["update-ref", "--", ref_name, commit_hash],
            deadline,
        )
        .await
    }

    pub async fn add_ref(
        &self,
        timeouts: Timeouts,
        path: &Path,
        ref_name: &str,
        ref_type: SqsRefType,
        deadline: SystemTime,
    ) -> ACResult<String> {
        let full_name = Self::full_ref_name(ref_name, ref_type);
        let refspec = format!("+{full_name}:{full_name}");
        Self::exec_git(timeouts, path, &["fetch", "origin", &refspec], deadline).await?;
        let commit_hash = Self::exec_git(
            timeouts,
            path,
            &["rev-parse", &format!("{full_name}^{{commit}}")],
            deadline,
        )
        .await?;
        let commit_hash = commit_hash.trim();
        let stock_trek_ref = Self::stock_trek_ref_name(ref_name, commit_hash);
        Self::exec_git(
            timeouts,
            path,
            &["update-ref", "--", &stock_trek_ref, commit_hash],
            deadline,
        )
        .await
    }

    pub async fn delete_ref(
        &self,
        timeouts: Timeouts,
        path: &Path,
        ref_name: &str,
        ref_type: SqsRefType,
        deadline: SystemTime,
    ) -> ACResult<String> {
        let prefix = format!("{STOCK_TREK_REF_PREFIX}/{ref_name}-");
        let stock_trek_refs = Self::exec_git(
            timeouts,
            path,
            &["for-each-ref", "--format=%(refname)", STOCK_TREK_REF_PREFIX],
            deadline,
        )
        .await?;
        for stock_trek_ref in stock_trek_refs
            .lines()
            .filter(|stock_trek_ref| Self::is_stock_trek_ref(stock_trek_ref, &prefix))
        {
            Self::exec_git(
                timeouts,
                path,
                &["update-ref", "-d", "--", stock_trek_ref],
                deadline,
            )
            .await?;
        }
        let full_name = Self::full_ref_name(ref_name, ref_type);
        Self::exec_git(
            timeouts,
            path,
            &["update-ref", "-d", "--", &full_name],
            deadline,
        )
        .await
    }

    pub async fn exec_git(
        timeouts: Timeouts,
        path: &Path,
        args: &[&str],
        deadline: SystemTime,
    ) -> ACResult<String> {
        let timeout = timeouts.command_for(deadline)?;
        Program::run_with_timeout("git", args, path, timeout).await
    }
}

// helpers
impl GitLocal {
    fn is_stock_trek_ref(stock_trek_ref: &str, prefix: &str) -> bool {
        stock_trek_ref.strip_prefix(prefix).is_some_and(|hash| {
            matches!(hash.len(), SHA1_HEX_LENGTH | SHA256_HEX_LENGTH)
                && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    }

    fn full_ref_name(ref_name: &str, ref_type: SqsRefType) -> String {
        let prefix = match ref_type {
            SqsRefType::Branch => "refs/heads",
            SqsRefType::Tag => "refs/tags",
        };
        format!("{prefix}/{ref_name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{process::Command as StdCommand, time::Duration};

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[tokio::test]
    async fn sets_origin_remote_url() {
        let base =
            std::env::temp_dir().join(format!("algorithm-compiler-test-{}", uuid::Uuid::new_v4()));
        let repo = base.join("repo.git");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "--bare", "-q"]);
        git(
            &repo,
            &["remote", "add", "origin", "https://example.com/old.git"],
        );

        let timeouts = Timeouts {
            command: Duration::from_secs(60),
            aws_connect: Duration::from_secs(10),
            aws_operation: Duration::from_secs(10),
        };
        let deadline = SystemTime::now() + Duration::from_secs(600);
        let remote_url = "https://example.com/new.git";

        GitLocal
            .set_remote(timeouts, &repo, remote_url, deadline)
            .await
            .unwrap();

        assert_eq!(git(&repo, &["remote", "get-url", "origin"]), remote_url);
        let _ = std::fs::remove_dir_all(&base);
    }
}
