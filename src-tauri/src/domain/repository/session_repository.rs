use super::WatchGuard;
use crate::domain::entity::{AppError, ChatEvent, SubagentRun};

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

/// 세션 로그 폴더에서 바뀐 것.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionLogChange {
    /// 메인 로그가 바뀐 프로젝트 폴더 이름
    pub dirs: Vec<String>,
    /// 서브에이전트 로그가 바뀐 세션 id
    pub subagent_sessions: Vec<String>,
}

pub trait SessionRepository: Send + Sync {
    /// 프로젝트의 모든 세션 메타. 최근 수정순.
    fn scan(&self, root: &str) -> Result<Vec<SessionLogMeta>, AppError>;
    fn meta(&self, root: &str, session_id: &str) -> Option<SessionLogMeta>;
    fn read_log(&self, root: &str, session_id: &str) -> Result<Vec<ChatEvent>, AppError>;
    /// 세션이 띄운 서브에이전트들. 시작 순.
    fn subagents(&self, root: &str, session_id: &str) -> Result<Vec<SubagentRun>, AppError>;
    fn read_subagent_log(&self, root: &str, session_id: &str, agent_id: &str) -> Result<Vec<ChatEvent>, AppError>;
    /// 이 세션 폴더 이름이 해당 프로젝트 것인지.
    fn owns_session_dir(&self, root: &str, dir_name: &str) -> bool;
    /// `~/.claude/projects` 아래 세션·서브에이전트 로그 변경을 알린다.
    fn watch(&self, on_change: Box<dyn Fn(SessionLogChange) + Send + Sync>) -> Result<WatchGuard, AppError>;
}
