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

/// 뷰어 분할 하나. active 는 tabs 의 인덱스.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SavedPane {
    pub tabs: Vec<SavedTab>,
    pub active: Option<usize>,
}

/// 프로젝트 하나의 뷰어 레이아웃: 분할(1~2개), 포커스된 분할, 왼쪽 분할 비율(%), 세션 대화창 숨김.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SavedLayout {
    pub panes: Vec<SavedPane>,
    pub focused: usize,
    pub ratio: f64,
    pub session_hidden: bool,
}

impl Default for SavedLayout {
    fn default() -> Self {
        Self { panes: vec![SavedPane::default()], focused: 0, ratio: 50.0, session_hidden: false }
    }
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
    /// 1.1.0 까지 마지막으로 쓴 모델을 전역으로 저장하던 값. 1.1.1 부터는 읽지 않는다 (기본은 언제나 Opus).
    pub selected_model: String,
    /// 세션 id → 그 세션에서 마지막으로 쓴 모델. (1.1.1)
    pub session_models: BTreeMap<String, String>,
    /// 프로젝트 루트 → 그 프로젝트에서 마지막으로 쓴 모델. 기록 없는 새 세션의 기본값. (1.1.1)
    pub project_models: BTreeMap<String, String>,
    /// `--permission-mode` 값. None 이면 CLI 기본값을 쓴다.
    pub permission_mode: Option<String>,
    /// 설정 창에서 지정한 터미널 경로.
    pub terminals: TerminalPaths,
    /// 세션별로 아직 보내지 않은 입력 (세션 id → 본문).
    pub drafts: BTreeMap<String, String>,
    /// 프로젝트 루트("" 는 프로젝트 없음) → 뷰어 레이아웃. (1.1.0)
    pub viewer_layouts: BTreeMap<String, SavedLayout>,
    /// 1.0.x 의 단일 탭 목록. 읽기만 하고 첫 복원 때 viewer_layouts 로 옮긴 뒤 비운다.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub viewer_tabs: Vec<SavedTab>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
            session_models: BTreeMap::new(),
            project_models: BTreeMap::new(),
            permission_mode: None,
            terminals: TerminalPaths::default(),
            drafts: BTreeMap::new(),
            viewer_layouts: BTreeMap::new(),
            viewer_tabs: Vec::new(),
            active_tab: None,
        }
    }
}
