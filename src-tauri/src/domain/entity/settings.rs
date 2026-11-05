use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::terminal::TerminalPaths;
use super::viewer::ViewerMode;

/// diff 탭이 기준으로 삼는 브랜치 또는 커밋.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedDiff {
    pub root: String,
    pub branch: Option<String>,
    pub untracked: bool,
    /// 커밋 하나의 변경을 볼 때 그 해시
    #[serde(default)]
    pub commit: Option<String>,
}

/// 앱을 다시 켰을 때 되살릴 뷰어 탭. 터미널 탭은 프로세스라 저장하지 않는다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedTab {
    pub path: String,
    pub mode: ViewerMode,
    pub diff: Option<SavedDiff>,
}

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
    /// 세션별로 아직 보내지 않은 입력 (세션 id → 본문).
    pub drafts: BTreeMap<String, String>,
    /// 마지막으로 열려 있던 뷰어 탭과 그중 활성 탭의 위치.
    pub viewer_tabs: Vec<SavedTab>,
    pub active_tab: Option<usize>,
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
            drafts: BTreeMap::new(),
            viewer_tabs: Vec::new(),
            active_tab: None,
        }
    }
}
