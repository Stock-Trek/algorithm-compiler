use crate::{config::Config, dynamodb::DynamoDb, s3::S3, timeouts::Timeouts};
use aws_config::{BehaviorVersion, timeout::TimeoutConfig};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;

pub struct Aws {
    pub config: Config,
    pub dynamodb: DynamoDb,
    pub s3: S3,
}

impl Aws {
    pub async fn new(config: Config) -> Self {
        let timeout_config = TimeoutConfig::builder()
            .connect_timeout(Timeouts::aws_connect())
            .operation_timeout(Timeouts::aws_operation())
            .build();
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .timeout_config(timeout_config)
            .load()
            .await;
        Self {
            config,
            dynamodb: DynamoDb {
                client: DynamoDbClient::new(&sdk_config),
            },
            s3: S3 {
                client: S3Client::new(&sdk_config),
            },
        }
    }
}
