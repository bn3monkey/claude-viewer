use super::WatchGuard;
use crate::domain::entity::{AppError, ChatEvent};

#[derive(Debug, Clone, PartialEq)]
pub struct SessionLogMeta {
    pub id: String,
    /// ai-title → summary → 첫 사용자 메시지 순. 전부 없으면 None.
    pub title: Option<String>,
    pub message_count: u32,
    pub updated_at: f64,
    /// 다른 클라이언트에서 돌고 있는 것으로 보이면 true (휴리스틱).
    pub looks_running: bool,
}

pub trait SessionRepository: Send + Sync {
    /// 프로젝트의 모든 세션 메타. 최근 수정순.
    fn scan(&self, root: &str) -> Result<Vec<SessionLogMeta>, AppError>;
    fn meta(&self, root: &str, session_id: &str) -> Option<SessionLogMeta>;
    fn read_log(&self, root: &str, session_id: &str) -> Result<Vec<ChatEvent>, AppError>;
    /// 이 세션 폴더 이름이 해당 프로젝트 것인지.
    fn owns_session_dir(&self, root: &str, dir_name: &str) -> bool;
    /// `~/.claude/projects` 아래 세션 로그 변경을 폴더 이름과 함께 알린다.
    fn watch(&self, on_change: Box<dyn Fn(Vec<String>) + Send + Sync>) -> Result<WatchGuard, AppError>;
}
