use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub generator: Generator,
    pub provenance: Provenance,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Generator {
    pub creator: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub description: String,
    pub methodology: String,
    pub references: Vec<String>,
}
