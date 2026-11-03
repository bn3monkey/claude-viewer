use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::session::Session;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Project {
    pub root: String,
    pub name: String,
    pub done: u32,
    pub total: u32,
    /// 활성화된 세션만 담는다. 나머지 기존 세션은 다이얼로그에서 조회한다.
    pub sessions: Vec<Session>,
}
