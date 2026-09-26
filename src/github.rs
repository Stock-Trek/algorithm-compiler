use crate::error::{ACError, ACResult};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_PER_PAGE: usize = 100;
const GITHUB_TOKEN_ENV: &str = "GITHUB_TOKEN";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub clone_url: String,
}

pub struct GitHub {
    client: Client,
    token: Option<String>,
}

impl GitHub {
    pub fn new() -> ACResult<Self> {
        let client = Client::builder()
            .user_agent("stock-trek-algorithm-compiler")
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| ACError::GitHub(format!("failed to build HTTP client: {error}")))?;
        let token = match std::env::var(GITHUB_TOKEN_ENV) {
            Ok(token) if !token.is_empty() => Some(token),
            _ => None,
        };
        Ok(Self { client, token })
    }

    pub async fn repos(&self, account_login: &str) -> ACResult<Vec<GitHubRepo>> {
        let mut repos = Vec::new();
        let mut page = 1;
        loop {
            let url = format!(
                "{GITHUB_API_BASE}/users/{account_login}/repos?per_page={GITHUB_PER_PAGE}&page={page}&type=all"
            );
            let mut request = self
                .client
                .get(url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", GITHUB_API_VERSION);
            if let Some(token) = &self.token {
                request = request.bearer_auth(token);
            }
            let response = request.send().await.map_err(|error| {
                ACError::GitHub(format!(
                    "failed to list repositories for {account_login}: {error}"
                ))
            })?;
            let status = response.status();
            if !status.is_success() {
                return Err(ACError::GitHub(format!(
                    "GitHub returned {status} listing repositories for {account_login}"
                )));
            }
            let batch: Vec<GitHubRepo> = response.json().await.map_err(|error| {
                ACError::GitHub(format!(
                    "failed to parse repositories for {account_login}: {error}"
                ))
            })?;
            let received = batch.len();
            repos.extend(batch);
            if received < GITHUB_PER_PAGE {
                break;
            }
            page += 1;
        }
        Ok(repos)
    }
}
