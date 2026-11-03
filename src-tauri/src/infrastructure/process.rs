//! 자식 프로세스 생성과 종료. GUI 앱에서 콘솔 창이 뜨지 않게 하고, 종료 시 자손까지 끝낸다.

/// 출력을 기다리는 짧은 명령용 (`--version`, `where` 등).
pub fn std_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut command = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// 오래 도는 claude 세션용. macOS 에서는 자손을 한 번에 끝낼 수 있게 새 프로세스 그룹으로 띄운다.
pub fn tokio_command(program: impl AsRef<std::ffi::OsStr>) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(program);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    #[cfg(unix)]
    command.process_group(0);
    command.kill_on_drop(true);
    command
}

/// 사용자가 고른 실행 파일을 앱과 떨어진 프로세스로 띄운다. 콘솔 프로그램이면 자기 콘솔 창을 갖는다.
/// 관리자 권한이 필요한 exe 는 직접 spawn 이 실패하므로 셸(ShellExecute)로 다시 시도한다.
pub fn launch_detached(program: &std::path::Path, cwd: &std::path::Path) -> Result<(), String> {
    let mut command = std::process::Command::new(program);
    command.current_dir(cwd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    match command.spawn() {
        Ok(_) => Ok(()),
        #[cfg(windows)]
        Err(e) if e.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED) => std_command("cmd")
            .args(["/C", "start", "\"\"", "/D"])
            .arg(cwd)
            .arg(program)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// claude 가 실행한 Bash 등 자손 프로세스까지 끝낸다.
pub fn kill_tree(pid: u32) {
    #[cfg(windows)]
    let _ = std_command("taskkill").args(["/T", "/F", "/PID", &pid.to_string()]).output();
    #[cfg(unix)]
    let _ = std_command("kill").args(["-KILL", &format!("-{pid}")]).output();
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
#[cfg(windows)]
const ERROR_ELEVATION_REQUIRED: i32 = 740;
