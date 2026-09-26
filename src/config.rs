use crate::error::{ACError, ACResult};
use std::env;

const S3_BUCKET_UPLOADS_ENV: &str = "S3_BUCKET_UPLOADS";
const DYNAMODB_LOCK_TABLE_ENV: &str = "DYNAMODB_LOCK_TABLE";

#[derive(Debug, Clone)]
pub struct Config {
    pub s3_bucket_uploads: String,
    pub dynamodb_lock_table: String,
}

impl Config {
    pub fn from_env() -> ACResult<Self> {
        Ok(Self {
            s3_bucket_uploads: required(S3_BUCKET_UPLOADS_ENV)?,
            dynamodb_lock_table: required(DYNAMODB_LOCK_TABLE_ENV)?,
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
