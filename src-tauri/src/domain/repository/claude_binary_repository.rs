use crate::domain::entity::{AppError, ClaudeSource};

#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeCandidate {
    pub path: String,
    pub source: ClaudeSource,
}

pub trait ClaudeBinaryRepository: Send + Sync {
    /// 자동 찾기 우선순위대로 정렬된 후보. 실제 실행 가능한지는 probe_version 으로 검증한다.
    fn locate_candidates(&self) -> Vec<ClaudeCandidate>;
    fn probe_version(&self, path: &str) -> Result<String, AppError>;
}
