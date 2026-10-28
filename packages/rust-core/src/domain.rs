use serde::{Deserialize, Serialize};

pub type ChefResult<T> = Result<T, ChefError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    LimitExceeded,
    OutOfScope,
    Cancelled,
    WorkerFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChefError {
    pub code: ErrorCode,
    pub message: &'static str,
}

impl ChefError {
    pub const fn new(code: ErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceInput {
    pub address: String,
    pub protocol: String,
    pub port: u16,
    pub observed_at: String,
    pub locator: String,
}
