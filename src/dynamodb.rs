use crate::error::{ACError, ACResult};
use aws_sdk_dynamodb::{Client as DynamoDbClient, types::AttributeValue};
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

const LOCK_RETRIES: u32 = 60;
const LOCK_RETRY_DELAY_MS: u64 = 1000;
const LOCK_ID_ATTRIBUTE: &str = "lock_id";

#[derive(Debug)]
pub struct DynamoDbDatumRef {
    pub table: String,
    pub key_name: String,
    pub key_value: String,
}

#[derive(Debug)]
pub struct DynamoDbLock {
    token: String,
}

pub struct DynamoDb {
    pub client: DynamoDbClient,
}

impl DynamoDb {
    pub async fn acquire_lock(&self, datum_ref: &DynamoDbDatumRef) -> ACResult<DynamoDbLock> {
        let token = Uuid::new_v4().to_string();
        for _ in 0..LOCK_RETRIES {
            let result = self
                .client
                .put_item()
                .table_name(datum_ref.table.clone())
                .item(
                    datum_ref.key_name.clone(),
                    AttributeValue::S(datum_ref.key_value.to_string()),
                )
                .item(LOCK_ID_ATTRIBUTE, AttributeValue::S(token.clone()))
                .condition_expression(format!("attribute_not_exists({})", datum_ref.key_name))
                .send()
                .await;
            match result {
                Ok(_) => {
                    return Ok(DynamoDbLock { token });
                }
                Err(error) => {
                    if error
                        .as_service_error()
                        .map(|e| e.is_conditional_check_failed_exception())
                        .unwrap_or(false)
                    {
                        warn!("Lock for {:?} is held, retrying", datum_ref);
                        tokio::time::sleep(Duration::from_millis(LOCK_RETRY_DELAY_MS)).await;
                    } else {
                        return Err(ACError::DynamoDbPutItem(Box::new(
                            error.into_service_error(),
                        )));
                    }
                }
            }
        }
        Err(ACError::InternalServer(format!(
            "Timed out acquiring lock for {:?}",
            datum_ref
        )))
    }

    pub async fn release_lock(
        &self,
        datum_ref: &DynamoDbDatumRef,
        lock: &DynamoDbLock,
    ) -> ACResult<()> {
        self.client
            .delete_item()
            .table_name(&datum_ref.table)
            .key(
                &datum_ref.key_name,
                AttributeValue::S(datum_ref.key_value.clone()),
            )
            .condition_expression(format!("{} = :lock_id", LOCK_ID_ATTRIBUTE))
            .expression_attribute_values(":lock_id", AttributeValue::S(lock.token.clone()))
            .send()
            .await
            .map_err(|e| ACError::DynamoDbDeleteItem(Box::new(e.into_service_error())))?;
        Ok(())
    }
}
