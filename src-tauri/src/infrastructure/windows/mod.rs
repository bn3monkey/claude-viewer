use std::path::PathBuf;

use super::platform::{
    editor_extension_patterns, lookup_command, CandidateKind, CandidatePath, Platform, TerminalDefaults,
};

pub struct WindowsPlatform;

const WEBVIEW2_DOWNLOAD: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

impl Platform for WindowsPlatform {
    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn app_data_dir(&self) -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("claude_viewer"))
    }

    fn candidate_paths(&self) -> Vec<CandidatePath> {
        let mut candidates = Vec::new();
        if let Some(home) = self.home_dir() {
            candidates.extend(editor_extension_patterns(&home, "claude.exe"));
            candidates.push(CandidatePath {
                kind: CandidateKind::NativeInstall,
                path: home.join(".local").join("bin").join("claude.exe"),
                glob: false,
            });
        }
        if let Some(roaming) = dirs::data_dir() {
            candidates.push(CandidatePath {
                kind: CandidateKind::PackageManager,
                path: roaming.join("npm").join("claude.cmd"),
                glob: false,
            });
        }
        candidates
    }

    fn find_in_path(&self) -> Vec<PathBuf> {
        lookup_command("where", "claude")
    }

    fn is_windows(&self) -> bool {
        true
    }

    fn terminal_defaults(&self) -> TerminalDefaults {
        let system_root = std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        let cmd = std::env::var_os("ComSpec")
            .map(PathBuf::from)
            .or_else(|| Some(system_root.join("System32").join("cmd.exe")))
            .filter(|p| p.is_file());
        let power_shell = Some(system_root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"))
            .filter(|p| p.is_file())
            .or_else(|| lookup_command("where", "pwsh").into_iter().next());
        // `where git` 이 가리키는 Git 설치 폴더의 bin\bash.exe
        let git_bash = lookup_command("where", "git")
            .into_iter()
            .flat_map(|git| git.ancestors().map(|dir| dir.join("bin").join("bash.exe")).collect::<Vec<_>>())
            .chain([PathBuf::from(r"C:\Program Files\Git\bin\bash.exe")])
            .find(|p| p.is_file());
        TerminalDefaults { cmd, power_shell, git_bash, mac_terminal: None }
    }

    /// 인스톨러 없이 exe 만 배포하므로 `webviewInstallMode` 가 동작하지 않는다.
    /// 런타임이 없으면 창을 만들다 조용히 죽으니, 먼저 안내하고 설치 페이지를 연다.
    fn ensure_webview_runtime(&self) -> bool {
        if tauri::webview_version().is_ok() {
            return true;
        }
        let answer = message_box(
            "Claude Viewer",
            "Microsoft Edge WebView2 런타임이 필요합니다.\n확인을 누르면 설치 페이지를 엽니다. 설치 후 다시 실행해 주세요.",
        );
        if answer {
            let _ = super::process::std_command("explorer").arg(WEBVIEW2_DOWNLOAD).spawn();
        }
        false
    }
}

/// 확인을 누르면 true.
fn message_box(title: &str, text: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDOK, MB_ICONWARNING, MB_OKCANCEL};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (title, text) = (wide(title), wide(text));
    // SAFETY: 두 버퍼 모두 NUL 로 끝나고 호출이 끝날 때까지 살아 있다.
    let result = unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OKCANCEL | MB_ICONWARNING)
    };
    result == IDOK
}
