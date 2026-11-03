use std::path::Path;

use super::project::ensure_within_project;
use super::viewer::resolve_viewer_mode;
use crate::domain::entity::{AppError, GitChange, GitCommit, GitDiff, GitOverview, ViewerMode};
use crate::domain::repository::{FileSystemRepository, GitRepository, SettingsRepository};

pub struct GitDeps<'a> {
    pub git: &'a dyn GitRepository,
    pub fs: &'a dyn FileSystemRepository,
    pub settings: &'a dyn SettingsRepository,
}

pub fn overview(deps: &GitDeps, root: &str) -> Result<GitOverview, AppError> {
    ensure_within_project(deps.fs, deps.settings, root)?;
    if !deps.git.is_repo(root) {
        return Ok(GitOverview { is_repo: false, branches: Vec::new() });
    }
    Ok(GitOverview { is_repo: true, branches: deps.git.branches(root)? })
}

/// branch 가 None 이거나 현재 브랜치면 작업 트리 변경, 아니면 그 브랜치가 갈라진 뒤 바꾼 파일.
pub fn changes(deps: &GitDeps, root: &str, branch: Option<&str>) -> Result<Vec<GitChange>, AppError> {
    ensure_within_project(deps.fs, deps.settings, root)?;
    match other_branch(deps, root, branch)? {
        Some(branch) => deps.git.branch_changes(root, &branch),
        None => deps.git.working_changes(root),
    }
}

const COMMIT_LIMIT: usize = 200;

/// 선택한 브랜치(None 이면 현재 브랜치)의 최근 커밋. 브랜치 이름은 실제 목록에 있는 것만 받는다.
pub fn commits(deps: &GitDeps, root: &str, branch: Option<&str>) -> Result<Vec<GitCommit>, AppError> {
    ensure_within_project(deps.fs, deps.settings, root)?;
    if let Some(branch) = branch {
        if !deps.git.branches(root)?.iter().any(|b| b.name == branch) {
            return Err(AppError::NotFound(format!("브랜치가 없습니다: {branch}")));
        }
    }
    deps.git.commits(root, branch, COMMIT_LIMIT)
}

/// 커밋 전체 로그. 해시는 16진수만 받아 git 옵션으로 해석될 여지를 없앤다.
pub fn commit_detail(deps: &GitDeps, root: &str, hash: &str) -> Result<String, AppError> {
    ensure_within_project(deps.fs, deps.settings, root)?;
    let valid = (4..=64).contains(&hash.len()) && hash.chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(AppError::InvalidPath(format!("커밋 해시가 아닙니다: {hash}")));
    }
    deps.git.commit_detail(root, hash)
}

/// 텍스트로 볼 수 있는 파일(소스·txt·md·html)만 diff 를 만든다.
pub fn diff(
    deps: &GitDeps,
    root: &str,
    branch: Option<&str>,
    path: &str,
    untracked: bool,
) -> Result<GitDiff, AppError> {
    ensure_within_project(deps.fs, deps.settings, &Path::new(root).join(path).to_string_lossy())?;
    if resolve_viewer_mode(path) != ViewerMode::Source {
        return Ok(GitDiff { text: String::new(), unsupported: true });
    }
    let text = match other_branch(deps, root, branch)? {
        Some(branch) => deps.git.diff_branch(root, &branch, path)?,
        None => deps.git.diff_working(root, path, untracked)?,
    };
    Ok(match text {
        Some(text) => GitDiff { text, unsupported: false },
        None => GitDiff { text: String::new(), unsupported: true },
    })
}

fn other_branch(deps: &GitDeps, root: &str, branch: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(branch) = branch else { return Ok(None) };
    let is_current = deps.git.branches(root)?.iter().any(|b| b.current && b.name == branch);
    Ok((!is_current).then(|| branch.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::{GitBranch, GitChangeKind};
    use crate::domain::usecase::fakes::{FakeFs, FakeGit, FakeSettings};

    fn branch(name: &str, current: bool) -> GitBranch {
        GitBranch { name: name.into(), current, subject: String::new(), committed_at: 0.0 }
    }

    #[test]
    fn non_repo_reports_is_repo_false() {
        let (fs, settings) = (FakeFs::with_dirs(&["/p"]), FakeSettings::with_project("/p", &[]));
        let git = FakeGit::default();
        let overview = overview(&GitDeps { git: &git, fs: &fs, settings: &settings }, "/p").unwrap();
        assert!(!overview.is_repo && overview.branches.is_empty());
    }

    #[test]
    fn current_branch_shows_working_changes_and_others_show_branch_changes() {
        let (fs, settings) = (FakeFs::with_dirs(&["/p"]), FakeSettings::with_project("/p", &[]));
        let git = FakeGit {
            repo: true,
            branches: vec![branch("main", true), branch("feature", false)],
            working: vec![GitChange { path: "a.rs".into(), kind: GitChangeKind::Modified, untracked: false }],
            diff: Some("+x".into()),
        };
        let deps = GitDeps { git: &git, fs: &fs, settings: &settings };
        assert_eq!(changes(&deps, "/p", None).unwrap().len(), 1);
        assert_eq!(changes(&deps, "/p", Some("main")).unwrap().len(), 1);
        assert!(changes(&deps, "/p", Some("feature")).unwrap().is_empty());
    }

    #[test]
    fn commits_only_for_known_branches_and_details_only_for_hex_hashes() {
        let (fs, settings) = (FakeFs::with_dirs(&["/p"]), FakeSettings::with_project("/p", &[]));
        let git = FakeGit { repo: true, branches: vec![branch("main", true)], ..FakeGit::default() };
        let deps = GitDeps { git: &git, fs: &fs, settings: &settings };
        assert!(commits(&deps, "/p", None).is_ok());
        assert!(commits(&deps, "/p", Some("main")).is_ok());
        assert!(matches!(commits(&deps, "/p", Some("--output=x")), Err(AppError::NotFound(_))));
        assert_eq!(commit_detail(&deps, "/p", "abc123").unwrap(), "detail abc123");
        for bad in ["--stat", "abc", "xyz123", "abc123; rm"] {
            assert!(commit_detail(&deps, "/p", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn diff_is_only_for_text_like_files_inside_the_project() {
        let (fs, settings) = (FakeFs::with_dirs(&["/p"]), FakeSettings::with_project("/p", &[]));
        let git = FakeGit { repo: true, diff: Some("+x".into()), ..FakeGit::default() };
        let deps = GitDeps { git: &git, fs: &fs, settings: &settings };
        assert_eq!(diff(&deps, "/p", None, "src/a.rs", false).unwrap().text, "+x");
        assert!(diff(&deps, "/p", None, "docs/a.md", false).unwrap().text == "+x");
        assert!(diff(&deps, "/p", None, "img.png", false).unwrap().unsupported);
        assert!(matches!(diff(&deps, "/p", None, "../secret", false), Err(AppError::OutsideProject(_))));
    }
}
