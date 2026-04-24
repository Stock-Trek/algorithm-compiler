use crate::dto::{
    errors::{StockTrekCompileAlgorithmError, invalid_request},
    metadata::Metadata,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub fn body_to_request(body: &Value) -> Result<CompileRequest, StockTrekCompileAlgorithmError> {
    let body_str = body.to_string();
    let deserializer = &mut serde_json::Deserializer::from_str(&body_str);
    let http_request: HttpRequest = serde_path_to_error::deserialize(deserializer)
        .map_err(|e| invalid_request(&[e.path().to_string().as_str()]))?;
    let metadata = match http_request.metadata {
        None => None,
        Some(metadata) => Some(MetadataRequest {
            generator_id: Uuid::new_v4().to_string(),
            metadata,
        }),
    };
    Ok(CompileRequest {
        code: http_request.code,
        user_id: http_request.user_id,
        metadata,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequest {
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
