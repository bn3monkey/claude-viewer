use tauri::State;

use crate::domain::entity::{AppError, ClaudeBinary, Settings};
use crate::domain::usecase::connection;
use crate::presentation::state::AppState;

#[tauri::command(async)]
pub fn connection_get(state: State<'_, AppState>) -> Option<ClaudeBinary> {
    connection::verify_claude(&*state.binaries, &*state.settings)
}

#[tauri::command(async)]
pub fn connection_find_auto(state: State<'_, AppState>) -> Result<ClaudeBinary, AppError> {
    connection::find_claude_auto(&*state.binaries, &*state.settings)
}

#[tauri::command(async)]
pub fn connection_set_manual(state: State<'_, AppState>, path: String) -> Result<ClaudeBinary, AppError> {
    connection::set_claude_manual(&*state.binaries, &*state.settings, &path)
}

/// 모델 목록, 선택한 모델 등 프론트가 그리는 데 필요한 설정.
#[tauri::command(async)]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.settings.load()
}
