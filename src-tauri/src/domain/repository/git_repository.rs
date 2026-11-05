use crate::domain::entity::{AppError, GitBranch, GitChange, GitCommit};

pub trait GitRepository: Send + Sync {
    fn is_repo(&self, root: &str) -> bool;
    /// 현재 브랜치가 앞에 오도록 정렬. 최근 커밋순.
    fn branches(&self, root: &str) -> Result<Vec<GitBranch>, AppError>;
    /// 그 브랜치(없으면 HEAD)의 최근 커밋. 최신순.
    fn commits(&self, root: &str, branch: Option<&str>, limit: usize) -> Result<Vec<GitCommit>, AppError>;
    /// 커밋 전체 로그 (작성자, 날짜, 메시지 본문)
    fn commit_detail(&self, root: &str, hash: &str) -> Result<String, AppError>;
    /// 작업 트리의 변경 (마지막 커밋 기준, 미추적 파일 포함)
    fn working_changes(&self, root: &str) -> Result<Vec<GitChange>, AppError>;
    /// 그 브랜치가 현재 브랜치와 갈라진 지점부터 바꾼 파일
    fn branch_changes(&self, root: &str, branch: &str) -> Result<Vec<GitChange>, AppError>;
    /// unified diff. None 이면 바이너리.
    fn diff_working(&self, root: &str, path: &str, untracked: bool) -> Result<Option<String>, AppError>;
    fn diff_branch(&self, root: &str, branch: &str, path: &str) -> Result<Option<String>, AppError>;
    /// 그 커밋이 바꾼 파일 (병합 커밋은 첫 부모 기준)
    fn commit_changes(&self, root: &str, hash: &str) -> Result<Vec<GitChange>, AppError>;
    /// 그 커밋에서 한 파일의 unified diff. None 이면 바이너리.
    fn diff_commit(&self, root: &str, hash: &str, path: &str) -> Result<Option<String>, AppError>;
    /// `git pull origin <branch>`
    fn pull(&self, root: &str, branch: &str) -> Result<(), AppError>;
    /// `git reset --hard HEAD`. 추적 중인 파일의 커밋하지 않은 변경을 모두 버린다.
    fn reset_hard(&self, root: &str) -> Result<(), AppError>;
}
