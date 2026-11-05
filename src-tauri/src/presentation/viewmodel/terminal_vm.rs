use tauri::{AppHandle, Emitter, Manager, State};

use crate::domain::entity::{AppError, TerminalKind, TerminalPaths, TerminalSession};
use crate::domain::repository::TerminalOutput;
use crate::domain::usecase::terminal;
use crate::presentation::state::AppState;

/// 더블클릭할 때 쓸 터미널. `.exe` 처럼 바로 실행할 파일이면 None.
#[tauri::command(async)]
pub fn terminal_default_kind(state: State<'_, AppState>, path: String) -> Option<TerminalKind> {
    terminal::default_kind(&path, state.terminals.is_windows())
}

/// id 는 프론트가 정한다. 출력 구독을 먼저 마쳐야 시작 직후 출력(ConPTY 의 커서 위치 질의 등)을 놓치지 않는다.
#[tauri::command(async)]
pub fn terminal_launch(
    state: State<'_, AppState>,
    id: String,
    path: String,
    kind: TerminalKind,
    cols: u16,
    rows: u16,
) -> Result<TerminalSession, AppError> {
    terminal::launch(&state.terminal_deps(), &terminal::LaunchRequest { id: &id, path: &path, kind, cols, rows })
}

/// 저장소에서 "터미널 열기" 를 누를 때 쓸 셸 (git bash 우선, 없으면 OS 기본)
#[tauri::command(async)]
pub fn terminal_repo_shell(state: State<'_, AppState>) -> TerminalKind {
    terminal::repo_shell_kind(&state.terminal_deps())
}

/// 프로젝트 안의 폴더에서 셸을 연다.
#[tauri::command(async)]
pub fn terminal_open_shell(
    state: State<'_, AppState>,
    id: String,
    path: String,
    kind: TerminalKind,
    cols: u16,
    rows: u16,
) -> Result<TerminalSession, AppError> {
    terminal::open_shell(&state.terminal_deps(), &terminal::LaunchRequest { id: &id, path: &path, kind, cols, rows })
}

#[tauri::command(async)]
pub fn terminal_write(state: State<'_, AppState>, id: String, data: String) -> Result<(), AppError> {
    state.terminals.write(&id, &data)
}

#[tauri::command(async)]
pub fn terminal_resize(state: State<'_, AppState>, id: String, cols: u16, rows: u16) -> Result<(), AppError> {
    state.terminals.resize(&id, cols, rows)
}

#[tauri::command(async)]
pub fn terminal_close(state: State<'_, AppState>, id: String) {
    state.terminals.kill(&id);
}

/// 설정 창: 저장된 경로와 자동 감지 경로
#[tauri::command(async)]
pub fn terminal_paths(state: State<'_, AppState>) -> (TerminalPaths, TerminalPaths) {
    (state.settings.load().terminals, terminal::detected_paths(&*state.terminals))
}

#[tauri::command(async)]
pub fn terminal_save_paths(state: State<'_, AppState>, paths: TerminalPaths) -> Result<(), AppError> {
    terminal::save_paths(&*state.settings, paths)
}

/// PTY 출력을 `terminal://data/{id}`, 종료를 `terminal://exit/{id}` 로 보낸다.
pub fn start_terminal_pump(app: &AppHandle) {
    let receiver = app.state::<AppState>().terminals.subscribe();
    let app = app.clone();
    std::thread::spawn(move || {
        for output in receiver {
            match output {
                TerminalOutput::Data { id, data } => {
                    let _ = app.emit(&format!("terminal://data/{id}"), data);
                }
                TerminalOutput::Exit { id, code } => {
                    let _ = app.emit(&format!("terminal://exit/{id}"), code);
                }
            }
        }
    });
}
