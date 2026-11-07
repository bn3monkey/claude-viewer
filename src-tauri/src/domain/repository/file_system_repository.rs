use std::path::{Path, PathBuf};

use super::WatchGuard;
use crate::domain::entity::{AppError, FsNode, TextContent};

pub trait FileSystemRepository: Send + Sync {
    /// .gitignore 와 상관없이 전부. `.git` 만 뺀다.
    fn list(&self, root: &str) -> Result<FsNode, AppError>;
    /// 심볼릭 링크까지 풀어 실제 경로를 얻는다. 프로젝트 범위 검사에 쓴다.
    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError>;
    fn is_dir(&self, path: &str) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), AppError>;
    fn trash(&self, path: &str) -> Result<(), AppError>;
    fn create_file(&self, path: &Path) -> Result<(), AppError>;
    fn create_dir(&self, path: &Path) -> Result<(), AppError>;
    fn read_text(&self, path: &str, max_bytes: u64) -> Result<TextContent, AppError>;
    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, AppError>;
    /// 파일 전체를 바꿔 쓴다. 없는 파일은 만든다.
    fn write_text(&self, path: &Path, text: &str) -> Result<(), AppError>;
    /// .git 안의 변경은 알리지 않는다.
    fn watch(&self, root: &str, on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchGuard, AppError>;
}
