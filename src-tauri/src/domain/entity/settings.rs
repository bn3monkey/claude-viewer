use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::terminal::TerminalPaths;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ProjectEntry {
    pub root: String,
    pub active_sessions: Vec<String>,
}

impl Default for ProjectEntry {
    fn default() -> Self {
        Self { root: String::new(), active_sessions: Vec::new() }
    }
}

/// 사용자가 settings.json 을 직접 편집할 수 있으므로 빠진 필드는 기본값으로 채운다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    pub claude_path: Option<String>,
    pub projects: Vec<ProjectEntry>,
    pub models: Vec<String>,
    pub selected_model: String,
    /// `--permission-mode` 값. None 이면 CLI 기본값을 쓴다.
    pub permission_mode: Option<String>,
    /// 설정 창에서 지정한 터미널 경로.
    pub terminals: TerminalPaths,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            claude_path: None,
            projects: Vec::new(),
            models: ["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"]
                .map(String::from)
                .to_vec(),
            selected_model: "claude-opus-5-5".into(),
            permission_mode: None,
            terminals: TerminalPaths::default(),
        }
    }
}
