use tauri::{AppHandle, Emitter, Manager, State};

use super::{EVENT_PROJECT_CHANGED, EVENT_SUBAGENT_CHANGED};
use crate::domain::entity::{AppError, Project};
use crate::domain::usecase::{project, session};
use crate::presentation::state::AppState;

#[tauri::command(async)]
pub fn project_list(state: State<'_, AppState>) -> Vec<Project> {
    project::list_projects(&state.project_deps())
}

#[tauri::command(async)]
pub fn project_open(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<Project, AppError> {
    let opened = project::open_project(&state.project_deps(), &*state.fs, &path)?;
    allow_assets(&app, &opened.root);
    Ok(opened)
}

#[tauri::command(async)]
pub fn project_close(state: State<'_, AppState>, root: String) -> Result<(), AppError> {
    project::close_project(&state.project_deps(), &root)
}

/// 이미지·동영상은 `asset:` 프로토콜로 읽으므로 연 프로젝트 폴더만 허용한다.
pub fn allow_assets(app: &AppHandle, root: &str) {
    let _ = app.asset_protocol_scope().allow_directory(root, true);
}

/// 세션 로그 폴더가 바뀌면 해당 프로젝트를 다시 계산해 보낸다.
pub fn start_session_watch(app: &AppHandle) {
    let state = app.state::<AppState>();
    let handle = app.clone();
    let watch = state.sessions.watch(Box::new(move |change| {
        let state = handle.state::<AppState>();
        for root in session::projects_of_session_dirs(&*state.settings, &*state.sessions, &change.dirs) {
            emit_project(&handle, &root);
        }
        for session_id in change.subagent_sessions {
            let _ = handle.emit(EVENT_SUBAGENT_CHANGED, session_id);
        }
    }));
    match watch {
        Ok(guard) => *state.session_watch.lock().unwrap() = Some(guard),
        Err(e) => eprintln!("세션 폴더를 감시하지 못했습니다: {e}"),
    }
}

pub fn emit_project(app: &AppHandle, root: &str) {
    let state = app.state::<AppState>();
    if let Some(project) = project::get_project(&state.project_deps(), root) {
        let _ = app.emit(EVENT_PROJECT_CHANGED, project);
    }
}
