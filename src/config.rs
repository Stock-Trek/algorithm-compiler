use crate::error::{ACError, ACResult};
use std::env;

const DYNAMODB_LOCK_TABLE_ENV: &str = "DYNAMODB_LOCK_TABLE";
const S3_BUCKET_COMMIT_ARTIFACTS_ENV: &str = "S3_BUCKET_COMMIT_ARTIFACTS";
const SQS_DLQ_URL_ENV: &str = "SQS_DLQ_URL";

#[derive(Debug, Clone)]
pub struct Config {
    pub dynamodb_lock_table: String,
    pub s3_bucket_commit_artifacts: String,
    pub sqs_dlq_url: String,
}

impl Config {
    pub fn from_env() -> ACResult<Self> {
        Ok(Self {
            dynamodb_lock_table: required(DYNAMODB_LOCK_TABLE_ENV)?,
            s3_bucket_commit_artifacts: required(S3_BUCKET_COMMIT_ARTIFACTS_ENV)?,
            sqs_dlq_url: required(SQS_DLQ_URL_ENV)?,
        })
    }
}

fn required(key: &str) -> ACResult<String> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(ACError::Config(format!("{key} must not be empty"))),
        Err(env::VarError::NotPresent) => Err(ACError::Config(format!("{key} must be set"))),
        Err(env::VarError::NotUnicode(_)) => {
            Err(ACError::Config(format!("{key} is not valid UTF-8")))
        }
    }
}
