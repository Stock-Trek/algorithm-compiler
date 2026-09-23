use crate::{
    constants::{DYNAMOD_DB_LOCK_TABLE, LOCK_RETRIES, LOCK_RETRY_DELAY_MS},
    dto::errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
};
use aws_sdk_dynamodb::{Client as DynamoDbClient, types::AttributeValue};
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

pub struct RepoLock {
    client: DynamoDbClient,
    table: String,
    repo: String,
    token: String,
}

pub async fn acquire_lock(repo: &str) -> Result<RepoLock, StockTrekCompileAlgorithmError> {
    let client = dynamodb_client().await;
    let table = lock_table();
    let token = Uuid::new_v4().to_string();
    for _ in 0..LOCK_RETRIES {
        let result = client
            .put_item()
            .table_name(&table)
            .item("repo", AttributeValue::S(repo.to_string()))
            .item("lock_id", AttributeValue::S(token.clone()))
            .condition_expression("attribute_not_exists(repo)")
            .send()
            .await;
        match result {
            Ok(_) => {
                return Ok(RepoLock {
                    client,
                    table,
                    repo: repo.to_string(),
                    token,
                });
            }
            Err(error) => {
                if error
                    .as_service_error()
                    .map(|e| e.is_conditional_check_failed_exception())
                    .unwrap_or(false)
                {
                    warn!("Lock for repo {} is held, retrying", repo);
                    tokio::time::sleep(Duration::from_millis(LOCK_RETRY_DELAY_MS)).await;
                } else {
                    return Err(internal_server_e("Failed to acquire repo lock {}", error));
                }
            }
        }
    }
    Err(internal_server(&format!(
        "Timed out acquiring lock for repo {}",
        repo
    )))
}

pub async fn release_lock(lock: &RepoLock) -> Result<(), StockTrekCompileAlgorithmError> {
    lock.client
        .delete_item()
        .table_name(&lock.table)
        .key("repo", AttributeValue::S(lock.repo.clone()))
        .condition_expression("lock_id = :lock_id")
        .expression_attribute_values(":lock_id", AttributeValue::S(lock.token.clone()))
        .send()
        .await
        .map_err(|e| internal_server_e("Failed to release repo lock {}", e))?;
    Ok(())
}

async fn dynamodb_client() -> DynamoDbClient {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    DynamoDbClient::new(&config)
}

fn lock_table() -> String {
    std::env::var("REPO_LOCK_TABLE").unwrap_or_else(|_| DYNAMOD_DB_LOCK_TABLE.to_string())
}
