use std::path::PathBuf;

use super::platform::{
    editor_extension_patterns, lookup_command, CandidateKind, CandidatePath, Platform, TerminalDefaults,
};

pub struct MacPlatform;

impl Platform for MacPlatform {
    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn app_data_dir(&self) -> Option<PathBuf> {
        dirs::data_dir().map(|dir| dir.join("claude_viewer"))
    }

    fn candidate_paths(&self) -> Vec<CandidatePath> {
        let mut candidates = Vec::new();
        if let Some(home) = self.home_dir() {
            candidates.extend(editor_extension_patterns(&home, "claude"));
            candidates.push(CandidatePath {
                kind: CandidateKind::NativeInstall,
                path: home.join(".local").join("bin").join("claude"),
                glob: false,
            });
        }
        for path in ["/opt/homebrew/bin/claude", "/usr/local/bin/claude"] {
            candidates.push(CandidatePath {
                kind: CandidateKind::PackageManager,
                path: PathBuf::from(path),
                glob: false,
            });
        }
        candidates
    }

    fn find_in_path(&self) -> Vec<PathBuf> {
        lookup_command("which", "claude")
    }

    /// WKWebView 는 OS 에 내장되어 있다.
    fn ensure_webview_runtime(&self) -> bool {
        true
    }

    fn is_windows(&self) -> bool {
        false
    }

    fn terminal_defaults(&self) -> TerminalDefaults {
        let existing = |p: &str| Some(PathBuf::from(p)).filter(|p| p.is_file());
        TerminalDefaults {
            cmd: None,
            power_shell: existing("/usr/local/bin/pwsh").or_else(|| existing("/opt/homebrew/bin/pwsh")),
            git_bash: existing("/bin/bash"),
            mac_terminal: existing("/bin/zsh"),
        }
    }
}
