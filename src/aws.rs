use crate::{
    config::Config,
    dynamodb::{DynamoDb, DynamoDbDatumRef, DynamoDbLock},
    error::ACResult,
    fenced::FencedS3,
    s3::S3,
    source::GitHub,
};
use aws_config::{BehaviorVersion, timeout::TimeoutConfig};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;

pub struct Aws {
    pub config: Config,
    pub dynamodb: DynamoDb,
    pub s3: S3,
    pub github: GitHub,
}

impl Aws {
    pub async fn new(config: Config) -> ACResult<Self> {
        let timeout_config = TimeoutConfig::builder()
            .connect_timeout(config.timeouts.aws_connect)
            .operation_timeout(config.timeouts.aws_operation)
            .build();
        let operation_timeout = config.timeouts.aws_operation;
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .timeout_config(timeout_config)
            .load()
            .await;
        Ok(Self {
            config,
            dynamodb: DynamoDb {
                client: DynamoDbClient::new(&sdk_config),
            },
            s3: S3 {
                client: S3Client::new(&sdk_config),
                operation_timeout,
            },
            github: GitHub::new()?,
        })
    }

    pub fn fenced_s3<'a>(
        &'a self,
        datum_ref: &'a DynamoDbDatumRef,
        lock: &'a DynamoDbLock,
    ) -> FencedS3<'a> {
        FencedS3::new(&self.dynamodb, &self.s3, datum_ref, lock)
    }
}
