use crate::dto::{
    errors::{StockTrekCompileAlgorithmError, invalid_request},
    metadata::Metadata,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

pub fn payload_to_request(
    payload: &Value,
) -> Result<CompileRequest, StockTrekCompileAlgorithmError> {
    info!("parse request");
    let body = payload
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid_request(&["Missing path 'body'"]))?;
    info!("body {}", body);
    let deserializer = &mut serde_json::Deserializer::from_str(&body);
    let deserialized_result: Result<HttpRequest, serde_path_to_error::Error<serde_json::Error>> =
        serde_path_to_error::deserialize(deserializer);
    match deserialized_result {
        Err(e) => {
            let path = e.path();
            let message = format!("Deserializing error in path '{}'", path);
            tracing::error!(message);
            Err(invalid_request(&[message.as_str()]))
        }
        Ok(http_request) => {
            info!("Successfully deserialized http request");
            let metadata = match http_request.body.metadata {
                None => None,
                Some(metadata) => Some(MetadataRequest {
                    generator_id: new_generator_id(),
                    metadata,
                }),
            };
            Ok(CompileRequest {
                code: http_request.body.code,
                user_id: http_request.body.user_id,
                metadata,
            })
        }
    }
}

fn new_generator_id() -> String {
    format!("gen_{}", Uuid::new_v4())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    pub body: HttpBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpBody {
    // TODO use jwt
    pub user_id: String,
    pub code: String,
    pub metadata: Option<Metadata>,
}

#[derive(Debug)]
pub struct CompileRequest {
    pub user_id: String,
    pub code: String,
    pub metadata: Option<MetadataRequest>,
}

#[derive(Debug)]
pub struct MetadataRequest {
    pub generator_id: String,
    pub metadata: Metadata,
}
