use std::path::Path;

use super::project::ensure_within_project;
use crate::domain::entity::{AppError, TerminalKind, TerminalPaths, TerminalSession};
use crate::domain::repository::{FileSystemRepository, SettingsRepository, TerminalRepository, TerminalSpec};

pub struct TerminalDeps<'a> {
    pub terminals: &'a dyn TerminalRepository,
    pub fs: &'a dyn FileSystemRepository,
    pub settings: &'a dyn SettingsRepository,
}

/// 터미널로 실행할 수 있는 확장자.
const RUNNABLE: [&str; 6] = ["exe", "bat", "cmd", "ps1", "sh", "bash"];

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn is_runnable(path: &str) -> bool {
    RUNNABLE.contains(&extension(path).as_str())
}

/// 더블클릭했을 때 쓸 터미널. `.exe` 는 터미널 없이 바로 실행하므로 None.
pub fn default_kind(path: &str, is_windows: bool) -> Option<TerminalKind> {
    match extension(path).as_str() {
        "bat" | "cmd" => Some(TerminalKind::Cmd),
        "ps1" => Some(TerminalKind::PowerShell),
        "sh" | "bash" if is_windows => Some(TerminalKind::GitBash),
        "sh" | "bash" => Some(TerminalKind::MacTerminal),
        _ => None,
    }
}

/// 설정한 경로가 있으면 그것을, 없으면 자동 감지한 경로를 쓴다.
pub fn effective_paths(terminals: &dyn TerminalRepository, settings: &dyn SettingsRepository) -> TerminalPaths {
    let saved = settings.load().terminals;
    let pick = |kind| saved.get(kind).map(str::to_string).or_else(|| terminals.default_path(kind));
    TerminalPaths {
        cmd: pick(TerminalKind::Cmd),
        power_shell: pick(TerminalKind::PowerShell),
        git_bash: pick(TerminalKind::GitBash),
        mac_terminal: pick(TerminalKind::MacTerminal),
    }
}

pub fn detected_paths(terminals: &dyn TerminalRepository) -> TerminalPaths {
    TerminalPaths {
        cmd: terminals.default_path(TerminalKind::Cmd),
        power_shell: terminals.default_path(TerminalKind::PowerShell),
        git_bash: terminals.default_path(TerminalKind::GitBash),
        mac_terminal: terminals.default_path(TerminalKind::MacTerminal),
    }
}

/// 빈 문자열은 "자동 감지" 로 되돌린다.
pub fn save_paths(settings: &dyn SettingsRepository, paths: TerminalPaths) -> Result<(), AppError> {
    let clean = |p: Option<String>| p.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let paths = TerminalPaths {
        cmd: clean(paths.cmd),
        power_shell: clean(paths.power_shell),
        git_bash: clean(paths.git_bash),
        mac_terminal: clean(paths.mac_terminal),
    };
    settings.update(&mut |s| s.terminals = paths.clone())?;
    Ok(())
}

/// 파일을 그 폴더에서 실행하고, 끝난 뒤에도 셸을 남겨 출력을 보고 이어서 입력할 수 있게 한다.
pub struct LaunchRequest<'a> {
    /// 프론트가 정한 id. 영숫자와 `-` 만.
    pub id: &'a str,
    pub path: &'a str,
    pub kind: TerminalKind,
    pub cols: u16,
    pub rows: u16,
}

pub fn launch(deps: &TerminalDeps, request: &LaunchRequest) -> Result<TerminalSession, AppError> {
    let LaunchRequest { id, path, kind, cols, rows } = *request;
    let valid_id = (8..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !valid_id {
        return Err(AppError::InvalidPath(format!("터미널 id 형식이 아닙니다: {id}")));
    }
    ensure_within_project(deps.fs, deps.settings, path)?;
    if !is_runnable(path) || deps.fs.is_dir(path) {
        return Err(AppError::InvalidPath(format!("터미널로 실행할 수 없는 파일입니다: {path}")));
    }
    let program = effective_paths(deps.terminals, deps.settings)
        .get(kind)
        .map(str::to_string)
        .ok_or_else(|| AppError::NotFound(format!("{} 경로를 찾지 못했습니다. 설정에서 지정해 주세요.", label(kind))))?;
    let file = Path::new(path);
    let cwd = file.parent().unwrap_or(file).to_string_lossy().into_owned();
    let args = command_args(kind, &program, path);
    deps.terminals.spawn(TerminalSpec { id: id.to_string(), program, args, cwd, cols, rows })?;
    Ok(TerminalSession { id: id.to_string(), kind, path: path.to_string() })
}

pub fn label(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Cmd => "cmd",
        TerminalKind::PowerShell => "PowerShell",
        TerminalKind::GitBash => "git bash",
        TerminalKind::MacTerminal => "macOS Terminal",
    }
}

/// 터미널별 실행 인자.
pub fn command_args(kind: TerminalKind, program: &str, file: &str) -> Vec<String> {
    let is_script = matches!(extension(file).as_str(), "sh" | "bash");
    match kind {
        TerminalKind::Cmd => vec!["/K".into(), file.into()],
        TerminalKind::PowerShell => vec![
            "-NoExit".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            format!("& {}", ps_quote(file)),
        ],
        TerminalKind::GitBash | TerminalKind::MacTerminal => {
            let target = sh_quote(&to_posix(file));
            let run = if is_script { format!("bash {target}") } else { target };
            vec!["-l".into(), "-i".into(), "-c".into(), format!("{run}; exec {} -l -i", sh_quote(&to_posix(program)))]
        }
    }
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// `C:\a\b` → `/c/a/b` (git bash 형식). 이미 POSIX 경로면 그대로.
fn to_posix(path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        format!("/{}{}", (bytes[0] as char).to_ascii_lowercase(), path[2..].replace('\\', "/"))
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_click_kind_follows_extension_and_os() {
        assert_eq!(default_kind("a.exe", true), None);
        assert_eq!(default_kind("a.BAT", true), Some(TerminalKind::Cmd));
        assert_eq!(default_kind("a.ps1", true), Some(TerminalKind::PowerShell));
        assert_eq!(default_kind("a.sh", true), Some(TerminalKind::GitBash));
        assert_eq!(default_kind("a.bash", false), Some(TerminalKind::MacTerminal));
        assert!(is_runnable("x.exe") && is_runnable("x.cmd") && !is_runnable("x.txt"));
    }

    #[test]
    fn command_args_keep_the_shell_open_and_quote_paths() {
        assert_eq!(command_args(TerminalKind::Cmd, "cmd.exe", r"C:\a b\x.bat"), vec!["/K", r"C:\a b\x.bat"]);
        assert_eq!(
            command_args(TerminalKind::PowerShell, "powershell.exe", r"C:\it's\x.ps1").last().unwrap(),
            r"& 'C:\it''s\x.ps1'"
        );
        assert_eq!(
            command_args(TerminalKind::GitBash, r"D:\Git\bin\bash.exe", r"C:\a b\run.sh"),
            vec!["-l", "-i", "-c", "bash '/c/a b/run.sh'; exec '/d/Git/bin/bash.exe' -l -i"]
        );
        assert_eq!(
            command_args(TerminalKind::MacTerminal, "/bin/zsh", "/Users/me/tool.exe").last().unwrap(),
            "'/Users/me/tool.exe'; exec '/bin/zsh' -l -i"
        );
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
    }
}
