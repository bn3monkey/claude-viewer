use std::path::PathBuf;

/// 자동 찾기 후보의 종류. infrastructure 는 domain 을 모르므로 자체 타입을 쓴다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CandidateKind {
    VsCodeExtension,
    NativeInstall,
    PackageManager,
}

/// 후보 경로. `glob` 이 true 면 패턴이다.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidatePath {
    pub kind: CandidateKind,
    pub path: PathBuf,
    pub glob: bool,
}

pub trait Platform: Send + Sync {
    fn home_dir(&self) -> Option<PathBuf>;
    fn app_data_dir(&self) -> Option<PathBuf>;
    /// 우선순위 순서. PATH 탐색은 포함하지 않는다.
    fn candidate_paths(&self) -> Vec<CandidatePath>;
    /// `where claude` / `which claude` 결과.
    fn find_in_path(&self) -> Vec<PathBuf>;
    /// WebView2 등 실행에 필요한 런타임이 없으면 사용자에게 알리고 false.
    fn ensure_webview_runtime(&self) -> bool;
    fn is_windows(&self) -> bool;
    /// 이 OS 에서 찾은 터미널 실행 파일.
    fn terminal_defaults(&self) -> TerminalDefaults;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerminalDefaults {
    pub cmd: Option<PathBuf>,
    pub power_shell: Option<PathBuf>,
    pub git_bash: Option<PathBuf>,
    pub mac_terminal: Option<PathBuf>,
}

pub fn current() -> Box<dyn Platform> {
    #[cfg(windows)]
    return Box::new(super::windows::WindowsPlatform);
    #[cfg(target_os = "macos")]
    return Box::new(super::macos::MacPlatform);
    #[cfg(not(any(windows, target_os = "macos")))]
    compile_error!("Windows 와 macOS 만 지원한다");
}

/// VSCode 계열 편집기의 확장 폴더 아래 Claude Code 동봉 바이너리 패턴.
pub(super) fn editor_extension_patterns(home: &std::path::Path, binary_name: &str) -> Vec<CandidatePath> {
    [".vscode", ".vscode-insiders", ".cursor"]
        .iter()
        .map(|editor| CandidatePath {
            kind: CandidateKind::VsCodeExtension,
            path: home
                .join(editor)
                .join("extensions")
                .join("anthropic.claude-code-*")
                .join("resources")
                .join("native-binary")
                .join(binary_name),
            glob: true,
        })
        .collect()
}

/// `where` / `which` 를 실행해 줄 단위 경로를 얻는다.
pub(super) fn lookup_command(program: &str, name: &str) -> Vec<PathBuf> {
    super::process::std_command(program)
        .arg(name)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}
