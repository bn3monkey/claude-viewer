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
pub fn git_changes(state: State<'_, AppState>, root: String, branch: Option<String>) -> Result<Vec<GitChange>, AppError> {
    git::changes(&state.git_deps(), &root, branch.as_deref())
}

#[tauri::command(async)]
pub fn git_diff(
    state: State<'_, AppState>,
    root: String,
    branch: Option<String>,
    path: String,
    untracked: bool,
) -> Result<GitDiff, AppError> {
    git::diff(&state.git_deps(), &root, branch.as_deref(), &path, untracked)
}
