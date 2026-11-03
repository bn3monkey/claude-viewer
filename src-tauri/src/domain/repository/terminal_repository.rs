use std::sync::mpsc::Receiver;

use crate::domain::entity::{AppError, TerminalKind};

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalSpec {
    /// 프론트가 미리 정해 출력 구독을 마친 id. 시작 직후 출력도 놓치지 않는다.
    pub id: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalOutput {
    /// UTF-8 경계가 맞춰진 출력 조각
    Data { id: String, data: String },
    Exit { id: String, code: Option<u32> },
}

pub trait TerminalRepository: Send + Sync {
    /// 자동 감지한 터미널 경로. 이 OS 에서 쓸 수 없으면 None.
    fn default_path(&self, kind: TerminalKind) -> Option<String>;
    fn is_windows(&self) -> bool;
    /// 의사 터미널(PTY)로 띄운다.
    fn spawn(&self, spec: TerminalSpec) -> Result<(), AppError>;
    fn write(&self, id: &str, data: &str) -> Result<(), AppError>;
    fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), AppError>;
    fn kill(&self, id: &str);
    fn kill_all(&self);
    fn subscribe(&self) -> Receiver<TerminalOutput>;
}
