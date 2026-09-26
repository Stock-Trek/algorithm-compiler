use crate::error::{ACError, ACResult};
use aws_sdk_dynamodb::{Client as DynamoDbClient, types::AttributeValue};
use std::{
    future::Future,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::task::JoinHandle;
use tracing::{error, warn};
use uuid::Uuid;

const LOCK_RETRY_DELAY: Duration = Duration::from_secs(1);
const LOCK_DEADLINE_BUFFER: Duration = Duration::from_secs(5);
const LOCK_ID_ATTRIBUTE: &str = "lock_id";
const LOCK_EXPIRES_AT_ATTRIBUTE: &str = "expires_at";
const LOCK_TTL: Duration = Duration::from_secs(45);
const LOCK_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const LOCK_ACQUIRE_CONDITION: &str =
    "attribute_not_exists(#key) OR attribute_not_exists(#expires) OR #expires < :now";

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

struct LockHeartbeat {
    handle: JoinHandle<()>,
}

impl Drop for LockHeartbeat {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub struct DynamoDb {
    pub client: DynamoDbClient,
}

impl DynamoDb {
    pub async fn acquire_lock(
        &self,
        datum_ref: &DynamoDbDatumRef,
        deadline: SystemTime,
    ) -> ACResult<DynamoDbLock> {
        let token = Uuid::new_v4().to_string();
        let wait_until = deadline
            .checked_sub(LOCK_DEADLINE_BUFFER)
            .unwrap_or(deadline);
        loop {
            let result = self
                .client
                .put_item()
                .table_name(datum_ref.table.clone())
                .item(
                    datum_ref.key_name.clone(),
                    AttributeValue::S(datum_ref.key_value.clone()),
                )
                .item(LOCK_ID_ATTRIBUTE, AttributeValue::S(token.clone()))
                .item(
                    LOCK_EXPIRES_AT_ATTRIBUTE,
                    AttributeValue::N(lock_expires_at().to_string()),
                )
                .condition_expression(LOCK_ACQUIRE_CONDITION)
                .expression_attribute_names("#key", datum_ref.key_name.clone())
                .expression_attribute_names("#expires", LOCK_EXPIRES_AT_ATTRIBUTE)
                .expression_attribute_values(":now", AttributeValue::N(epoch_seconds().to_string()))
                .send()
                .await;
            match result {
                Ok(_) => {
                    return Ok(DynamoDbLock { token });
                }
                Err(error) => {
                    let conditional = error
                        .as_service_error()
                        .map(|e| e.is_conditional_check_failed_exception())
                        .unwrap_or(false);
                    if conditional {
                        let remaining = wait_until
                            .duration_since(SystemTime::now())
                            .unwrap_or_default();
                        if remaining.is_zero() {
                            return Err(lock_timeout(datum_ref));
                        }
                        warn!("Lock for {:?} is held, retrying", datum_ref);
                        tokio::time::sleep(remaining.min(LOCK_RETRY_DELAY)).await;
                    } else if error.as_service_error().is_some() {
                        return Err(ACError::DynamoDbPutItem(Box::new(
                            error.into_service_error(),
                        )));
                    } else {
                        return Err(ACError::InternalServer(format!(
                            "Failed to acquire lock for {:?}: {error}",
                            datum_ref
                        )));
                    }
                }
            }
        }
    }

    pub async fn release_lock(
        &self,
        datum_ref: &DynamoDbDatumRef,
        lock: &DynamoDbLock,
    ) -> ACResult<()> {
        let result = self
            .client
            .delete_item()
            .table_name(&datum_ref.table)
            .key(
                &datum_ref.key_name,
                AttributeValue::S(datum_ref.key_value.clone()),
            )
            .condition_expression(format!("{LOCK_ID_ATTRIBUTE} = :lock_id"))
            .expression_attribute_values(":lock_id", AttributeValue::S(lock.token.clone()))
            .send()
            .await;
        match result {
            Ok(_) => Ok(()),
            Err(error) => {
                let conditional = error
                    .as_service_error()
                    .map(|e| e.is_conditional_check_failed_exception())
                    .unwrap_or(false);
                if conditional {
                    warn!(
                        lock = %datum_ref.key_value,
                        "Lock expired or was taken over before it could be released",
                    );
                    Ok(())
                } else if error.as_service_error().is_some() {
                    Err(ACError::DynamoDbDeleteItem(Box::new(
                        error.into_service_error(),
                    )))
                } else {
                    Err(ACError::InternalServer(format!(
                        "Failed to release lock for {}: {error}",
                        datum_ref.key_value
                    )))
                }
            }
        }
    }

    pub async fn locked<F, Fut, T>(
        &self,
        lock_ref: &DynamoDbDatumRef,
        deadline: SystemTime,
        action: F,
    ) -> ACResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ACResult<T>> + Send,
        T: Send,
    {
        let lock = self.acquire_lock(lock_ref, deadline).await?;
        let heartbeat = LockHeartbeat {
            handle: self.spawn_heartbeat(lock_ref, &lock),
        };
        let result = action().await;
        drop(heartbeat);
        let release = self.release_lock(lock_ref, &lock).await;
        match result {
            Ok(value) => {
                if let Err(release_error) = release {
                    error!(
                        lock = %lock_ref.key_value,
                        %release_error,
                        "Failed to release lock after successful action",
                    );
                }
                Ok(value)
            }
            Err(error) => {
                if let Err(release_error) = release {
                    error!(
                        lock = %lock_ref.key_value,
                        %release_error,
                        "Failed to release lock after action failed",
                    );
                }
                Err(error)
            }
        }
    }

    fn spawn_heartbeat(&self, datum_ref: &DynamoDbDatumRef, lock: &DynamoDbLock) -> JoinHandle<()> {
        let client = self.client.clone();
        let table = datum_ref.table.clone();
        let key_name = datum_ref.key_name.clone();
        let key_value = datum_ref.key_value.clone();
        let token = lock.token.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(LOCK_HEARTBEAT_INTERVAL);
            interval.tick().await;
            loop {
                interval.tick().await;
                let result = client
                    .update_item()
                    .table_name(&table)
                    .key(&key_name, AttributeValue::S(key_value.clone()))
                    .update_expression(format!("SET {LOCK_EXPIRES_AT_ATTRIBUTE} = :expires_at"))
                    .condition_expression(format!("{LOCK_ID_ATTRIBUTE} = :lock_id"))
                    .expression_attribute_values(
                        ":expires_at",
                        AttributeValue::N(lock_expires_at().to_string()),
                    )
                    .expression_attribute_values(":lock_id", AttributeValue::S(token.clone()))
                    .send()
                    .await;
                match result {
                    Ok(_) => {}
                    Err(error) => {
                        let conditional = error
                            .as_service_error()
                            .map(|e| e.is_conditional_check_failed_exception())
                            .unwrap_or(false);
                        if conditional {
                            warn!(
                                lock = %key_value,
                                "Lock is no longer held, stopping heartbeat",
                            );
                            return;
                        }
                        warn!(
                            lock = %key_value,
                            %error,
                            "Failed to refresh lock",
                        );
                    }
                }
            }
        })
    }
}

fn lock_timeout(datum_ref: &DynamoDbDatumRef) -> ACError {
    ACError::LockTimeout(format!("Timed out acquiring lock for {:?}", datum_ref))
}

fn epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn lock_expires_at() -> i64 {
    epoch_seconds() + LOCK_TTL.as_secs() as i64
}
