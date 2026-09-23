use crate::{
    constants::{LOCK_RETRIES, LOCK_RETRY_DELAY_MS},
    dto::errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
};
use aws_sdk_dynamodb::{Client as DynamoDbClient, types::AttributeValue};
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

const LOCK_ID_ATTRIBUTE: &str = "lock_id";

pub struct Lock {
    key_name: String,
    key_value: String,
    token: String,
}

pub async fn dynamodb_client() -> DynamoDbClient {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    DynamoDbClient::new(&config)
}

pub async fn acquire_lock(
    client: &DynamoDbClient,
    table: &str,
    key_name: &str,
    key_value: &str,
) -> Result<Lock, StockTrekCompileAlgorithmError> {
    let token = Uuid::new_v4().to_string();
    for _ in 0..LOCK_RETRIES {
        let result = client
            .put_item()
            .table_name(table)
            .item(key_name, AttributeValue::S(key_value.to_string()))
            .item(LOCK_ID_ATTRIBUTE, AttributeValue::S(token.clone()))
            .condition_expression(format!("attribute_not_exists({})", key_name))
            .send()
            .await;
        match result {
            Ok(_) => {
                return Ok(Lock {
                    key_name: key_name.to_string(),
                    key_value: key_value.to_string(),
                    token,
                });
            }
            Err(error) => {
                if error
                    .as_service_error()
                    .map(|e| e.is_conditional_check_failed_exception())
                    .unwrap_or(false)
                {
                    warn!("Lock for {} is held, retrying", key_value);
                    tokio::time::sleep(Duration::from_millis(LOCK_RETRY_DELAY_MS)).await;
                } else {
                    return Err(internal_server_e("Failed to acquire lock {}", error));
                }
            }
        }
    }
    Err(internal_server(&format!(
        "Timed out acquiring lock for {}",
        key_value
    )))
}

pub async fn release_lock(
    client: &DynamoDbClient,
    table: &str,
    lock: &Lock,
) -> Result<(), StockTrekCompileAlgorithmError> {
    client
        .delete_item()
        .table_name(table)
        .key(&lock.key_name, AttributeValue::S(lock.key_value.clone()))
        .condition_expression(format!("{} = :lock_id", LOCK_ID_ATTRIBUTE))
        .expression_attribute_values(":lock_id", AttributeValue::S(lock.token.clone()))
        .send()
        .await
        .map_err(|e| internal_server_e("Failed to release lock {}", e))?;
    Ok(())
}
