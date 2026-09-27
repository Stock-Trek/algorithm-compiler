use crate::{
    dto::sqs_event::GitSource,
    error::{ACError, ACResult},
    git_local::GitLocal,
    timeouts::Timeouts,
};
use jsonwebtoken::EncodingKey;
use octocrab::{Octocrab, models::InstallationId};
use std::{path::Path, time::SystemTime};

const GITHUB_APP_ID_ENV: &str = "GITHUB_APP_ID";
const GITHUB_APP_PRIVATE_KEY_ENV: &str = "GITHUB_APP_PRIVATE_KEY";

#[derive(Clone)]
pub enum GitRemote {
    GitHub { client: Octocrab },
}

impl TryFrom<&GitSource> for GitRemote {
    type Error = ACError;

    fn try_from(value: &GitSource) -> Result<Self, Self::Error> {
        match value {
            GitSource::GitHub {
                installation_id, ..
            } => {
                let app_id = Self::required(GITHUB_APP_ID_ENV)?;
                let app_id: u64 = app_id.parse().map_err(|error| {
                    ACError::Config(format!("{GITHUB_APP_ID_ENV} is invalid: {error}"))
                })?;
                let private_key = Self::required(GITHUB_APP_PRIVATE_KEY_ENV)?.replace("\\n", "\n");
                let key = EncodingKey::from_rsa_pem(private_key.as_bytes()).map_err(|error| {
                    ACError::Config(format!("{GITHUB_APP_PRIVATE_KEY_ENV} is invalid: {error}"))
                })?;
                let client = Octocrab::builder()
                    .app(app_id.into(), key)
                    .build()
                    .map_err(|error| {
                        ACError::GitHub(format!("failed to build GitHub client: {error}"))
                    })?
                    .installation(InstallationId::from(*installation_id))
                    .map_err(|error| {
                        ACError::GitHub(format!(
                            "failed to create client for installation {installation_id}: {error}"
                        ))
                    })?;
                Ok(GitRemote::GitHub { client })
            }
        }
    }
}

impl GitRemote {
    pub async fn account_id(&self) -> ACResult<String> {
        Ok("".into())
    }

    pub async fn account_repo_ids(&self) -> ACResult<Vec<String>> {
        Ok(vec![])
    }

    pub async fn account_repo_name(&self) -> ACResult<(String, String)> {
        Ok(("".into(), "".into()))
    }

    pub async fn clone_bare_repo(
        &self,
        timeouts: Timeouts,
        repo_id: &str,
        repo_dir: &str,
        deadline: SystemTime,
    ) -> ACResult<()> {
        match self {
            GitRemote::GitHub { .. } => {
                let clone_url = format!("TODO: use {repo_id}");
                GitLocal::exec_git(
                    timeouts,
                    Path::new(repo_dir),
                    &["clone", "--bare", &clone_url, repo_dir],
                    deadline,
                )
                .await?;
                Ok(())
            }
        }
    }
}

// helpers
impl GitRemote {
    fn required(key: &str) -> ACResult<String> {
        match std::env::var(key) {
            Ok(value) if !value.is_empty() => Ok(value),
            Ok(_) => Err(ACError::Config(format!("{key} must not be empty"))),
            Err(std::env::VarError::NotPresent) => {
                Err(ACError::Config(format!("{key} must be set")))
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                Err(ACError::Config(format!("{key} is not valid UTF-8")))
            }
        }
    }
}
