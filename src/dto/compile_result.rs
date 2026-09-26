use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompileStatus {
    Success,
    Failure,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub result: CompileStatus,
    pub errors: Vec<String>,
    pub compile_messages: Vec<CompileMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileMessage {
    pub file: String,
    pub level: String,
    pub start: CodeLocation,
    pub end: CodeLocation,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeLocation {
    pub line: i32,
    pub column: i32,
}

impl CompileResult {
    pub fn failed(&self) -> bool {
        self.result == CompileStatus::Failure
    }
}
