use crate::error::{ACError, ACResult};
use jsonwebtoken::EncodingKey;
use octocrab::{Octocrab, models::InstallationId};
use serde::{Deserialize, Serialize};

const GITHUB_APP_ID_ENV: &str = "GITHUB_APP_ID";
const GITHUB_APP_PRIVATE_KEY_ENV: &str = "GITHUB_APP_PRIVATE_KEY";
const GITHUB_PER_PAGE: u32 = 100;

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub clone_url: String,
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

    pub async fn repos(&self, installation_id: &str) -> ACResult<Vec<GitHubRepo>> {
        let installation_id = installation_id.parse::<u64>().map_err(|error| {
            ACError::GitHub(format!(
                "invalid installation id {installation_id}: {error}"
            ))
        })?;
        let client = self
            .client
            .installation(InstallationId::from(installation_id))
            .map_err(|error| {
                ACError::GitHub(format!(
                    "failed to create client for installation {installation_id}: {error}"
                ))
            })?;
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
