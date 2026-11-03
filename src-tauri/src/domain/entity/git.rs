use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
    /// 마지막 커밋 제목
    pub subject: String,
    /// 마지막 커밋 시각 (epoch 초)
    pub committed_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitOverview {
    pub is_repo: bool,
    pub branches: Vec<GitBranch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    /// 커밋 시각 (epoch 초)
    pub committed_at: f64,
    /// 제목 한 줄
    pub subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GitChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitChange {
    /// 저장소 기준 상대 경로 (`/` 구분)
    pub path: String,
    pub kind: GitChangeKind,
    /// 아직 커밋된 적 없는 파일 (현재 브랜치 작업 트리에서만)
    pub untracked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GitDiff {
    /// unified diff 텍스트. 지원하지 않는 파일이면 빈 문자열.
    pub text: String,
    /// 텍스트·소스·html·md 가 아니거나 바이너리라 diff 를 보여줄 수 없음
    pub unsupported: bool,
}
