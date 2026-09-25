use crate::{dynamodb::DynamoDb, s3::S3};
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;

pub struct Aws {
    pub dynamodb: DynamoDb,
    pub s3: S3,
}

impl Aws {
    pub async fn new() -> Self {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        Self {
            dynamodb: DynamoDb {
                client: DynamoDbClient::new(&sdk_config),
            },
            s3: S3 {
                client: S3Client::new(&sdk_config),
            },
        }
    }
}
