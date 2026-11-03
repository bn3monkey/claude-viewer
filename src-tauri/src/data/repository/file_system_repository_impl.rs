use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use crate::data::datasource::fs_walker;
use crate::domain::entity::{AppError, FsNode, TextContent};
use crate::domain::repository::{FileSystemRepository, WatchGuard};
use crate::infrastructure::{trash, watcher};

pub struct FileSystemRepositoryImpl;

fn io(e: std::io::Error) -> AppError {
    AppError::Io(e.to_string())
}

impl FileSystemRepository for FileSystemRepositoryImpl {
    fn list(&self, root: &str) -> Result<FsNode, AppError> {
        Ok(fs_walker::walk(Path::new(root)))
    }

    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        // 아직 없는 경로(새 파일)는 존재하는 부모까지 풀고 나머지를 붙인다.
        let path = Path::new(path);
        let mut existing = path;
        let mut rest = Vec::new();
        while !existing.exists() {
            rest.push(existing.file_name().ok_or_else(|| AppError::InvalidPath(path.display().to_string()))?);
            existing = existing.parent().ok_or_else(|| AppError::InvalidPath(path.display().to_string()))?;
        }
        let mut canonical = fs::canonicalize(existing).map_err(io)?;
        for name in rest.into_iter().rev() {
            canonical.push(name);
        }
        if canonical.components().any(|c| c == Component::ParentDir) {
            return Err(AppError::InvalidPath(path.display().to_string()));
        }
        Ok(canonical)
    }

    fn is_dir(&self, path: &str) -> bool {
        Path::new(path).is_dir()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), AppError> {
        fs::rename(from, to).map_err(io)
    }

    fn trash(&self, path: &str) -> Result<(), AppError> {
        trash::move_to_trash(Path::new(path)).map_err(AppError::Io)
    }

    fn create_file(&self, path: &Path) -> Result<(), AppError> {
        File::create_new(path).map(|_| ()).map_err(io)
    }

    fn create_dir(&self, path: &Path) -> Result<(), AppError> {
        fs::create_dir(path).map_err(io)
    }

    fn read_text(&self, path: &str, max_bytes: u64) -> Result<TextContent, AppError> {
        let file = File::open(path).map_err(io)?;
        let size = file.metadata().map_err(io)?.len();
        let mut bytes = Vec::new();
        file.take(max_bytes).read_to_end(&mut bytes).map_err(io)?;
        let binary = bytes.iter().take(8192).any(|b| *b == 0);
        let text = if binary { String::new() } else { String::from_utf8_lossy(&bytes).into_owned() };
        Ok(TextContent { text, truncated: size > max_bytes, binary })
    }

    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, AppError> {
        fs::read(path).map_err(io)
    }

    fn watch(&self, root: &str, on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchGuard, AppError> {
        let handle = watcher::watch(Path::new(root), Duration::from_millis(500), move |paths| {
            let relevant = paths.iter().any(|path| {
                !path.components().any(|c| c.as_os_str() == ".git")
            });
            if relevant {
                on_change();
            }
        })
        .map_err(AppError::Io)?;
        Ok(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_handles_paths_that_do_not_exist_yet() {
        let dir = tempfile::tempdir().unwrap();
        let repo = FileSystemRepositoryImpl;
        let root = repo.canonicalize(&dir.path().to_string_lossy()).unwrap();
        let new_file = repo.canonicalize(&dir.path().join("new").join("a.txt").to_string_lossy()).unwrap();
        assert_eq!(new_file, root.join("new").join("a.txt"));
    }

    #[test]
    fn canonicalize_resolves_parent_components_of_existing_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("a")).unwrap();
        let repo = FileSystemRepositoryImpl;
        let root = repo.canonicalize(&dir.path().to_string_lossy()).unwrap();
        let escaped = repo.canonicalize(&dir.path().join("a").join("..").join("..").to_string_lossy()).unwrap();
        assert!(!escaped.starts_with(&root));
    }

    #[test]
    fn read_text_flags_binary_and_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let repo = FileSystemRepositoryImpl;
        let text = dir.path().join("a.txt");
        fs::write(&text, "가나다").unwrap();
        let bin = dir.path().join("a.bin");
        fs::write(&bin, [1u8, 0, 2]).unwrap();

        let full = repo.read_text(&text.to_string_lossy(), 1024).unwrap();
        assert_eq!((full.text.as_str(), full.truncated, full.binary), ("가나다", false, false));
        assert!(repo.read_text(&text.to_string_lossy(), 3).unwrap().truncated);
        assert!(repo.read_text(&bin.to_string_lossy(), 1024).unwrap().binary);
    }
}
