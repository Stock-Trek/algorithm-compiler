use crate::error::{ACError, ACResult};
use std::{env, path::PathBuf};

const DEFAULT_WORK_DIR: &str = "/tmp/algorithm-compiler";
const DEFAULT_SOURCE_DIR: &str = "./algorithm-runner";
const DEFAULT_S3_BUCKET_UPLOADS: &str = "stock-trek-uploads";
const DEFAULT_DYNAMODB_LOCK_TABLE: &str = "stock-trek-locks";

const WORK_DIR_ENV: &str = "WORK_DIR";
const SOURCE_DIR_ENV: &str = "SOURCE_DIR";
const S3_BUCKET_UPLOADS_ENV: &str = "S3_BUCKET_UPLOADS";
const DYNAMODB_LOCK_TABLE_ENV: &str = "DYNAMODB_LOCK_TABLE";

#[derive(Debug, Clone)]
pub struct Config {
    pub work_dir: PathBuf,
    pub source_dir: PathBuf,
    pub s3_bucket_uploads: String,
    pub dynamodb_lock_table: String,
}

impl Config {
    pub fn from_env() -> ACResult<Self> {
        Ok(Self {
            work_dir: PathBuf::from(value(WORK_DIR_ENV, DEFAULT_WORK_DIR)?),
            source_dir: PathBuf::from(value(SOURCE_DIR_ENV, DEFAULT_SOURCE_DIR)?),
            s3_bucket_uploads: value(S3_BUCKET_UPLOADS_ENV, DEFAULT_S3_BUCKET_UPLOADS)?,
            dynamodb_lock_table: value(DYNAMODB_LOCK_TABLE_ENV, DEFAULT_DYNAMODB_LOCK_TABLE)?,
        })
    }
}

fn value(key: &str, default: &str) -> ACResult<String> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(ACError::Config(format!("{key} must not be empty"))),
        Err(env::VarError::NotPresent) => Ok(default.to_string()),
        Err(env::VarError::NotUnicode(_)) => {
            Err(ACError::Config(format!("{key} is not valid UTF-8")))
        }
    }
}
