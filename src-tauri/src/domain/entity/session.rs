use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const NEW_SESSION_TITLE: &str = "New Session...";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SessionState {
    Running,
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub title: String,
    pub state: SessionState,
    /// 이 앱이 띄운 프로세스가 지금 응답을 만들고 있으면 true. Abort 가능 여부에 쓴다.
    pub busy: bool,
    pub message_count: u32,
    /// 로그 파일 마지막 수정 시각 (epoch ms). 로그가 아직 없으면 None.
    pub updated_at: Option<f64>,
}
