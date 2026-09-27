use crate::{
    dto::sqs_event::{GitSource, SqsRefType},
    error::{ACError, ACResult},
    program::Program,
    timeouts::Timeouts,
};
use jsonwebtoken::EncodingKey;
use octocrab::{Octocrab, models::InstallationId};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::SystemTime};

const GITHUB_APP_ID_ENV: &str = "GITHUB_APP_ID";
const GITHUB_APP_PRIVATE_KEY_ENV: &str = "GITHUB_APP_PRIVATE_KEY";
const GITHUB_PER_PAGE: u32 = 100;
const STOCK_TREK_REF_PREFIX: &str = "refs/stock-trek";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub clone_url: String,
    pub owner: GitHubOwner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubOwner {
    pub login: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub owner: String,
    pub clone_url: String,
}

impl From<GitHubRepo> for Repo {
    fn from(repo: GitHubRepo) -> Self {
        Self {
            id: repo.id,
            name: repo.name,
            owner: repo.owner.login,
            clone_url: repo.clone_url,
        }
    }
}

pub struct GitHub {
    client: Octocrab,
}

impl GitHub {
    pub fn new() -> ACResult<Self> {
        let app_id = required(GITHUB_APP_ID_ENV)?;
        let app_id: u64 = app_id
            .parse()
            .map_err(|error| ACError::Config(format!("{GITHUB_APP_ID_ENV} is invalid: {error}")))?;
        let private_key = required(GITHUB_APP_PRIVATE_KEY_ENV)?.replace("\\n", "\n");
        let key = EncodingKey::from_rsa_pem(private_key.as_bytes()).map_err(|error| {
            ACError::Config(format!("{GITHUB_APP_PRIVATE_KEY_ENV} is invalid: {error}"))
        })?;
        let client = Octocrab::builder()
            .app(app_id.into(), key)
            .build()
            .map_err(|error| ACError::GitHub(format!("failed to build GitHub client: {error}")))?;
        Ok(Self { client })
    }

    async fn repos(&self, installation_id: u64) -> ACResult<Vec<GitHubRepo>> {
        let client = self.installation(installation_id)?;
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
                    ACError::GitHub(format!(
                        "failed to list repositories for installation {installation_id}: {error}"
                    ))
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

    async fn repo(&self, installation_id: u64, repo_id: u64) -> ACResult<GitHubRepo> {
        let client = self.installation(installation_id)?;
        client
            .get::<GitHubRepo, _, _>(format!("/repositories/{repo_id}"), None::<&()>)
            .await
            .map_err(|error| {
                ACError::GitHub(format!(
                    "failed to get repository {repo_id} for installation {installation_id}: {error}"
                ))
            })
    }

    fn installation(&self, installation_id: u64) -> ACResult<Octocrab> {
        self.client
            .installation(InstallationId::from(installation_id))
            .map_err(|error| {
                ACError::GitHub(format!(
                    "failed to create client for installation {installation_id}: {error}"
                ))
            })
    }
}

impl GitSource {
    pub fn account_id(&self) -> String {
        match self {
            Self::GitHub {
                installation_id, ..
            } => installation_id.to_string(),
        }
    }

    pub async fn account_repo_ids(&self) -> ACResult<Vec<String>> {
        match self {
            Self::GitHub {
                installation_id, ..
            } => Ok(GitHub {
                client: Octocrab::default(),
            }
            .repos(*installation_id)
            .await?
            .into_iter()
            .map(|repo| repo.id.to_string())
            .collect()),
        }
    }

    pub async fn repo(&self, repo_id: &str) -> ACResult<Repo> {
        let repo_id = parse_repo_id(repo_id)?;
        match self {
            Self::GitHub {
                installation_id, ..
            } => Ok(GitHub {
                client: Octocrab::default(),
            }
            .repo(*installation_id, repo_id)
            .await?
            .into()),
        }
    }

    pub async fn clone_bare_repo(
        &self,
        timeouts: Timeouts,
        repo_id: &str,
        repo_dir: &str,
        deadline: SystemTime,
    ) -> ACResult<()> {
        match self {
            GitSource::GitHub { .. } => {
                let repo = self.repo(repo_id).await?;
                Self::exec_git(
                    timeouts,
                    Path::new(repo_dir),
                    &["clone", "--bare", &repo.clone_url, repo_dir],
                    deadline,
                )
                .await?;
                Ok(())
            }
        }
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

    pub fn stock_trek_ref_name(ref_name: &str, commit_hash: &str) -> String {
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

    async fn exec_git(
        timeouts: Timeouts,
        path: &Path,
        args: &[&str],
        deadline: SystemTime,
    ) -> ACResult<String> {
        let timeout = timeouts.command_for(deadline)?;
        Program::run_with_timeout("git", args, path, timeout).await
    }
}

fn parse_repo_id(repo_id: &str) -> ACResult<u64> {
    repo_id
        .parse()
        .map_err(|_| ACError::InvalidMessage(format!("Invalid repo id: {repo_id:?}")))
}

fn required(key: &str) -> ACResult<String> {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(ACError::Config(format!("{key} must not be empty"))),
        Err(std::env::VarError::NotPresent) => Err(ACError::Config(format!("{key} must be set"))),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(ACError::Config(format!("{key} is not valid UTF-8")))
        }
    }
}
