use crate::{
    dto::sqs_event::GitSource,
    error::{ACError, ACResult},
    git_local::GitLocal,
    timeouts::Timeouts,
};
use jsonwebtoken::EncodingKey;
use octocrab::{Octocrab, models::InstallationId};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::SystemTime};

const GITHUB_APP_ID_ENV: &str = "GITHUB_APP_ID";
const GITHUB_APP_PRIVATE_KEY_ENV: &str = "GITHUB_APP_PRIVATE_KEY";
const GITHUB_PER_PAGE: u32 = 100;

#[derive(Debug, Clone, Deserialize)]
struct GitHubRepo {
    id: u64,
    name: String,
    clone_url: String,
    owner: GitHubOwner,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubOwner {
    id: u64,
    login: String,
}

#[derive(Debug, Deserialize)]
struct InstallationRepositoriesResponse {
    repositories: Vec<GitHubRepo>,
}

#[derive(Debug, Serialize)]
struct RepositoriesParams {
    per_page: u32,
    page: u32,
}

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
        Ok(self.owner().await?.id.to_string())
    }

    pub async fn account_repo_ids(&self) -> ACResult<Vec<String>> {
        Ok(self
            .repos()
            .await?
            .into_iter()
            .map(|repo| repo.id.to_string())
            .collect())
    }

    pub async fn account_repo_name(&self, repo_id: &str) -> ACResult<(String, String)> {
        let repo = self.repo(repo_id).await?;
        Ok((repo.owner.login, repo.name))
    }

    pub async fn clone_bare_repo(
        &self,
        timeouts: Timeouts,
        repo_id: &str,
        repo_dir: &str,
        deadline: SystemTime,
    ) -> ACResult<()> {
        let repo = self.repo(repo_id).await?;
        GitLocal::exec_git(
            timeouts,
            Path::new(repo_dir),
            &["clone", "--bare", &repo.clone_url, repo_dir],
            deadline,
        )
        .await?;
        Ok(())
    }

    async fn owner(&self) -> ACResult<GitHubOwner> {
        let GitRemote::GitHub { client } = self;
        let params = RepositoriesParams {
            per_page: 1,
            page: 1,
        };
        let response = client
            .get::<InstallationRepositoriesResponse, _, _>(
                "/installation/repositories",
                Some(&params),
            )
            .await
            .map_err(|error| {
                ACError::GitHub(format!("failed to fetch installation owner: {error}"))
            })?;
        response
            .repositories
            .into_iter()
            .next()
            .map(|repo| repo.owner)
            .ok_or_else(|| ACError::GitHub("installation has no repositories".into()))
    }

    async fn repos(&self) -> ACResult<Vec<GitHubRepo>> {
        let GitRemote::GitHub { client } = self;
        let mut repos = Vec::new();
        let mut page = 1;
        loop {
            let params = RepositoriesParams {
                per_page: GITHUB_PER_PAGE,
                page,
            };
            let response = client
                .get::<InstallationRepositoriesResponse, _, _>(
                    "/installation/repositories",
                    Some(&params),
                )
                .await
                .map_err(|error| {
                    ACError::GitHub(format!("failed to list installation repositories: {error}"))
                })?;
            let received = response.repositories.len();
            repos.extend(response.repositories);
            if received < GITHUB_PER_PAGE as usize {
                break;
            }
            page += 1;
        }
        Ok(repos)
    }

    async fn repo(&self, repo_id: &str) -> ACResult<GitHubRepo> {
        let repo_id = parse_repo_id(repo_id)?;
        let GitRemote::GitHub { client } = self;
        client
            .get::<GitHubRepo, _, _>(format!("/repositories/{repo_id}"), None::<&()>)
            .await
            .map_err(|error| {
                ACError::GitHub(format!("failed to get repository {repo_id}: {error}"))
            })
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

fn parse_repo_id(repo_id: &str) -> ACResult<u64> {
    repo_id
        .parse()
        .map_err(|_| ACError::InvalidMessage(format!("Invalid repo id: {repo_id:?}")))
}
