use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TerminalKind {
    Cmd,
    PowerShell,
    GitBash,
    /// macOS 의 Terminal 셸 (기본 /bin/zsh)
    MacTerminal,
}

/// 사용자가 설정한 터미널 실행 파일 경로. None 이면 자동 감지한 경로를 쓴다.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TerminalPaths {
    pub cmd: Option<String>,
    pub power_shell: Option<String>,
    pub git_bash: Option<String>,
    pub mac_terminal: Option<String>,
}

impl TerminalPaths {
    pub fn get(&self, kind: TerminalKind) -> Option<&str> {
        match kind {
            TerminalKind::Cmd => self.cmd.as_deref(),
            TerminalKind::PowerShell => self.power_shell.as_deref(),
            TerminalKind::GitBash => self.git_bash.as_deref(),
            TerminalKind::MacTerminal => self.mac_terminal.as_deref(),
        }
    }
}

/// 뷰어에 띄운 터미널 하나.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TerminalSession {
    pub id: String,
    pub kind: TerminalKind,
    /// 실행한 파일
    pub path: String,
}
