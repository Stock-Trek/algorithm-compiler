use crate::{
    config::Config,
    dynamodb::DynamoDb,
    error::{ACError, ACResult},
    s3::S3,
    timeouts::Timeouts,
};
use aws_config::{BehaviorVersion, timeout::TimeoutConfig};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sqs::Client as SqsClient;

pub struct Dlq {
    pub client: SqsClient,
    pub queue_url: String,
}

impl Dlq {
    pub async fn push(&self, event: &str) -> ACResult<()> {
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(event)
            .send()
            .await
            .map_err(|e| ACError::SqsSendMessage(Box::new(e.into_service_error())))?;
        Ok(())
    }
}

pub struct Aws {
    pub config: Config,
    pub dynamodb: DynamoDb,
    pub s3: S3,
    pub dlq: Dlq,
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
            dlq: Dlq {
                client: SqsClient::new(&sdk_config),
                queue_url: config.sqs_dead_letter_queue_url.clone(),
            },
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
