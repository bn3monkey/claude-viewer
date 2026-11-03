use std::path::Path;

use crate::data::datasource::git_cli::{
    is_binary_diff, parse_branches, parse_log, parse_name_status, parse_status, run, synthesize_added_diff,
    BRANCH_FORMAT, LOG_FORMAT,
};
use crate::domain::entity::{AppError, GitBranch, GitChange, GitCommit};
use crate::domain::repository::GitRepository;

pub struct GitRepositoryImpl;

fn git(root: &str, args: &[&str]) -> Result<String, AppError> {
    run(Path::new(root), args).map_err(AppError::Io)
}

impl GitRepository for GitRepositoryImpl {
    fn is_repo(&self, root: &str) -> bool {
        run(Path::new(root), &["rev-parse", "--is-inside-work-tree"]).map_or(false, |out| out.trim() == "true")
    }

    fn branches(&self, root: &str) -> Result<Vec<GitBranch>, AppError> {
        Ok(parse_branches(&git(root, &["branch", "--format", BRANCH_FORMAT])?))
    }

    fn commits(&self, root: &str, branch: Option<&str>, limit: usize) -> Result<Vec<GitCommit>, AppError> {
        let limit = format!("-n{limit}");
        let mut args = vec!["log", limit.as_str(), LOG_FORMAT];
        args.push(branch.unwrap_or("HEAD"));
        args.push("--");
        Ok(parse_log(&git(root, &args)?))
    }

    fn commit_detail(&self, root: &str, hash: &str) -> Result<String, AppError> {
        git(root, &["show", "-s", "--format=fuller", hash, "--"])
    }

    fn working_changes(&self, root: &str) -> Result<Vec<GitChange>, AppError> {
        Ok(parse_status(&git(root, &["status", "--porcelain", "--untracked-files=all"])?))
    }

    fn branch_changes(&self, root: &str, branch: &str) -> Result<Vec<GitChange>, AppError> {
        // `HEAD...branch` 는 공통 조상부터 branch 까지의 변경
        Ok(parse_name_status(&git(root, &["diff", "--name-status", &format!("HEAD...{branch}")])?))
    }

    fn diff_working(&self, root: &str, path: &str, untracked: bool) -> Result<Option<String>, AppError> {
        if untracked {
            let bytes = std::fs::read(Path::new(root).join(path)).map_err(|e| AppError::Io(e.to_string()))?;
            if bytes.iter().take(8192).any(|b| *b == 0) {
                return Ok(None);
            }
            return Ok(Some(synthesize_added_diff(path, &String::from_utf8_lossy(&bytes))));
        }
        let diff = git(root, &["diff", "HEAD", "--", path])?;
        Ok((!is_binary_diff(&diff)).then_some(diff))
    }

    fn diff_branch(&self, root: &str, branch: &str, path: &str) -> Result<Option<String>, AppError> {
        let diff = git(root, &["diff", &format!("HEAD...{branch}"), "--", path])?;
        Ok((!is_binary_diff(&diff)).then_some(diff))
    }
}
