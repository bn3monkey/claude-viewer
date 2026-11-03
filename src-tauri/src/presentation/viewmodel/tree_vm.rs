use tauri::{AppHandle, Emitter, Manager, State};

use super::EVENT_TREE_CHANGED;
use crate::domain::entity::{AppError, FsNode};
use crate::domain::usecase::tree;
use crate::presentation::state::AppState;

/// 트리를 읽고, 이 프로젝트를 감시해 바뀌면 `tree://changed` 로 새 트리를 보낸다.
#[tauri::command(async)]
pub fn tree_read(app: AppHandle, state: State<'_, AppState>, root: String) -> Result<FsNode, AppError> {
    let node = tree::read_tree(&state.tree_deps(), &root)?;
    let handle = app.clone();
    let watched_root = root.clone();
    let guard = state.fs.watch(
        &root,
        Box::new(move || {
            let state = handle.state::<AppState>();
            if let Ok(node) = tree::read_tree(&state.tree_deps(), &watched_root) {
                let _ = handle.emit(EVENT_TREE_CHANGED, node);
            }
        }),
    );
    *state.tree_watch.lock().unwrap() = guard.ok();
    Ok(node)
}

#[tauri::command(async)]
pub fn tree_rename(state: State<'_, AppState>, path: String, new_name: String) -> Result<(), AppError> {
    tree::rename(&state.tree_deps(), &path, &new_name)
}

#[tauri::command(async)]
pub fn tree_move(state: State<'_, AppState>, path: String, dest_dir: String) -> Result<(), AppError> {
    tree::move_(&state.tree_deps(), &path, &dest_dir)
}

#[tauri::command(async)]
pub fn tree_delete(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    tree::delete(&state.tree_deps(), &path)
}

#[tauri::command(async)]
pub fn tree_create_file(state: State<'_, AppState>, dir: String, name: String) -> Result<String, AppError> {
    tree::create_file(&state.tree_deps(), &dir, &name)
}

#[tauri::command(async)]
pub fn tree_create_dir(state: State<'_, AppState>, dir: String, name: String) -> Result<String, AppError> {
    tree::create_dir(&state.tree_deps(), &dir, &name)
}

#[tauri::command(async)]
pub fn tree_copy_path(state: State<'_, AppState>, path: String, relative: bool) -> Result<(), AppError> {
    tree::copy_path(&state.tree_deps(), &*state.clipboard, &path, relative)
}

#[tauri::command(async)]
pub fn tree_run(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    tree::run_executable(&state.tree_deps(), &*state.shell, &path)
}

#[tauri::command(async)]
pub fn tree_reveal(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    tree::reveal(&state.tree_deps(), &*state.shell, &path)
}
