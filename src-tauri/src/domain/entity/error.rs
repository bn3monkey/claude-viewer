use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// viewmodel 이 그대로 직렬화해 보내고, 프론트는 message 를 토스트로 표시한다.
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
#[ts(export)]
pub enum AppError {
    #[error("{0}")]
    ClaudeNotFound(String),
    #[error("{0}")]
    NotConnected(String),
    #[error("{0}")]
    InvalidPath(String),
    #[error("{0}")]
    OutsideProject(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Process(String),
    #[error("{0}")]
    SessionBusy(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Settings(String),
}
