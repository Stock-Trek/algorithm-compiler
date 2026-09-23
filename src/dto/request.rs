use crate::dto::metadata::Metadata;

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
