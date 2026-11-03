use std::sync::mpsc::Receiver;

use crate::domain::entity::{AppError, ChatEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStart {
    /// 로그가 없는 세션. `--session-id` 로 앱이 정한 id 를 넘긴다.
    New,
    /// 로그가 있는 세션. `--resume` 으로 잇는다.
    Resume,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnSpec {
    pub binary: String,
    pub cwd: String,
    pub session_id: String,
    pub start: SessionStart,
    pub model: String,
    pub permission_mode: Option<String>,
    pub append_system_prompt: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessStatus {
    pub busy: bool,
    pub model: String,
    pub permission_mode: Option<String>,
}

pub trait ClaudeProcessRepository: Send + Sync {
    fn spawn(&self, spec: SpawnSpec) -> Result<(), AppError>;
    /// stdin 에 사용자 메시지 1줄을 쓰고 busy 로 표시한다.
    fn write_stdin(&self, session_id: &str, text: &str) -> Result<(), AppError>;
    /// 권한 요청에 답한다. 턴이 이어지므로 busy 는 그대로.
    fn respond_permission(
        &self,
        session_id: &str,
        request_id: &str,
        allow: bool,
        updated_permissions: Option<&str>,
    ) -> Result<(), AppError>;
    fn kill(&self, session_id: &str) -> Result<(), AppError>;
    fn kill_all(&self);
    /// 프로세스가 살아 있을 때만 Some.
    fn status(&self, session_id: &str) -> Option<ProcessStatus>;
    /// 이 앱이 띄운 프로세스가 마지막으로 끝난 시각 (epoch ms).
    fn stopped_at(&self, session_id: &str) -> Option<f64>;
    fn subscribe(&self) -> Receiver<(String, ChatEvent)>;
}
