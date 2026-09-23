use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqsMessage {
    pub after: String,
    pub before: String,
    pub forced: bool,
    pub id: String,
    pub r#ref: String,
    pub repo: String,
    pub tree_id: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqsEvent {
    #[serde(rename = "Records")]
    pub records: Vec<SqsRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqsRecord {
    pub body: String,
}
