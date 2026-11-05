use tauri::State;

use crate::domain::entity::{AppError, GitChange, GitCommit, GitDiff, GitOverview};
use crate::domain::usecase::git;
use crate::presentation::state::AppState;

#[tauri::command(async)]
pub fn git_overview(state: State<'_, AppState>, root: String) -> Result<GitOverview, AppError> {
    git::overview(&state.git_deps(), &root)
}

#[tauri::command(async)]
pub fn git_commits(state: State<'_, AppState>, root: String, branch: Option<String>) -> Result<Vec<GitCommit>, AppError> {
    git::commits(&state.git_deps(), &root, branch.as_deref())
}

#[tauri::command(async)]
pub fn git_commit_detail(state: State<'_, AppState>, root: String, hash: String) -> Result<String, AppError> {
    git::commit_detail(&state.git_deps(), &root, &hash)
}

#[tauri::command(async)]
pub fn git_commit_changes(state: State<'_, AppState>, root: String, hash: String) -> Result<Vec<GitChange>, AppError> {
    git::commit_changes(&state.git_deps(), &root, &hash)
}

/// 현재 브랜치를 origin 에서 받는다.
#[tauri::command(async)]
pub fn git_pull(state: State<'_, AppState>, root: String) -> Result<(), AppError> {
    git::pull(&state.git_deps(), &root)
}

/// 추적 중인 변경을 모두 버린다. 프론트가 확인을 받은 뒤에만 호출한다.
#[tauri::command(async)]
pub fn git_reset_hard(state: State<'_, AppState>, root: String) -> Result<(), AppError> {
    git::reset_hard(&state.git_deps(), &root)
}

#[tauri::command(async)]
pub fn git_changes(state: State<'_, AppState>, root: String, branch: Option<String>) -> Result<Vec<GitChange>, AppError> {
    git::changes(&state.git_deps(), &root, branch.as_deref())
}

#[tauri::command(async)]
pub fn git_diff(
    state: State<'_, AppState>,
    root: String,
    branch: Option<String>,
    commit: Option<String>,
    path: String,
    untracked: bool,
) -> Result<GitDiff, AppError> {
    let request =
        git::DiffRequest { root: &root, branch: branch.as_deref(), commit: commit.as_deref(), path: &path, untracked };
    git::diff(&state.git_deps(), &request)
}
