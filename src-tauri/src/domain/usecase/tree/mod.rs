use std::path::{Path, PathBuf};

use super::project::ensure_within_project;
use crate::domain::entity::{AppError, FsNode};
use crate::domain::repository::{
    ClipboardRepository, FileSystemRepository, SettingsRepository, ShellRepository,
};

pub struct TreeDeps<'a> {
    pub fs: &'a dyn FileSystemRepository,
    pub settings: &'a dyn SettingsRepository,
}

pub fn read_tree(deps: &TreeDeps, root: &str) -> Result<FsNode, AppError> {
    ensure_within_project(deps.fs, deps.settings, root)?;
    deps.fs.list(root)
}

pub fn rename(deps: &TreeDeps, path: &str, new_name: &str) -> Result<(), AppError> {
    ensure_movable(deps, path)?;
    validate_name(new_name)?;
    let source = Path::new(path);
    let target = source.parent().unwrap_or(source).join(new_name);
    relocate(deps, source, &target)
}

pub fn move_(deps: &TreeDeps, path: &str, dest_dir: &str) -> Result<(), AppError> {
    let scoped = ensure_movable(deps, path)?;
    let dest = ensure_within_project(deps.fs, deps.settings, dest_dir)?;
    if dest.canonical.starts_with(&scoped.canonical) {
        return Err(AppError::InvalidPath("폴더를 자기 자신 안으로 옮길 수 없습니다.".into()));
    }
    let source = Path::new(path);
    let name = source
        .file_name()
        .ok_or_else(|| AppError::InvalidPath(format!("이름이 없는 경로입니다: {path}")))?;
    relocate(deps, source, &Path::new(dest_dir).join(name))
}

pub fn delete(deps: &TreeDeps, path: &str) -> Result<(), AppError> {
    ensure_movable(deps, path)?;
    deps.fs.trash(path)
}

pub fn create_file(deps: &TreeDeps, dir: &str, name: &str) -> Result<String, AppError> {
    let target = new_child_path(deps, dir, name)?;
    deps.fs.create_file(&target)?;
    Ok(target.to_string_lossy().into_owned())
}

pub fn create_dir(deps: &TreeDeps, dir: &str, name: &str) -> Result<String, AppError> {
    let target = new_child_path(deps, dir, name)?;
    deps.fs.create_dir(&target)?;
    Ok(target.to_string_lossy().into_owned())
}

pub fn copy_path(
    deps: &TreeDeps,
    clipboard: &dyn ClipboardRepository,
    path: &str,
    relative: bool,
) -> Result<(), AppError> {
    let scoped = ensure_within_project(deps.fs, deps.settings, path)?;
    if relative {
        clipboard.write_text(&scoped.relative())
    } else {
        clipboard.write_text(path)
    }
}

pub fn reveal(deps: &TreeDeps, shell: &dyn ShellRepository, path: &str) -> Result<(), AppError> {
    ensure_within_project(deps.fs, deps.settings, path)?;
    shell.reveal_in_explorer(path)
}

/// 트리에서 고른 `.exe` 만 실행한다. 다른 확장자는 실행 파일로 취급하지 않는다.
pub fn run_executable(deps: &TreeDeps, shell: &dyn ShellRepository, path: &str) -> Result<(), AppError> {
    ensure_within_project(deps.fs, deps.settings, path)?;
    let is_exe = Path::new(path)
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("exe"));
    if !is_exe || deps.fs.is_dir(path) {
        return Err(AppError::InvalidPath(format!(".exe 파일만 실행할 수 있습니다: {path}")));
    }
    shell.run_executable(path)
}

/// 프로젝트 루트 자체는 트리에서 지우거나 옮기지 못한다.
fn ensure_movable(deps: &TreeDeps, path: &str) -> Result<super::project::ScopedPath, AppError> {
    let scoped = ensure_within_project(deps.fs, deps.settings, path)?;
    if scoped.is_root() {
        return Err(AppError::InvalidPath("프로젝트 루트는 변경할 수 없습니다.".into()));
    }
    Ok(scoped)
}

fn relocate(deps: &TreeDeps, source: &Path, target: &Path) -> Result<(), AppError> {
    if source == target {
        return Ok(());
    }
    if deps.fs.exists(target) {
        return Err(AppError::InvalidPath(format!("이미 있습니다: {}", target.display())));
    }
    deps.fs.rename(source, target)
}

fn new_child_path(deps: &TreeDeps, dir: &str, name: &str) -> Result<PathBuf, AppError> {
    ensure_within_project(deps.fs, deps.settings, dir)?;
    validate_name(name)?;
    let target = Path::new(dir).join(name);
    if deps.fs.exists(&target) {
        return Err(AppError::InvalidPath(format!("이미 있습니다: {}", target.display())));
    }
    Ok(target)
}

/// 이름에 경로 구분자나 `..` 가 들어오면 프로젝트 밖으로 나갈 수 있으므로 막는다.
fn validate_name(name: &str) -> Result<(), AppError> {
    let invalid = name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']);
    if invalid {
        return Err(AppError::InvalidPath(format!("사용할 수 없는 이름입니다: {name}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usecase::fakes::{FakeClipboard, FakeFs, FakeSettings};

    struct World {
        fs: FakeFs,
        settings: FakeSettings,
    }

    impl World {
        fn new() -> Self {
            Self {
                fs: FakeFs::with_dirs(&["/p", "/p/src", "/p/docs"]),
                settings: FakeSettings::with_project("/p", &[]),
            }
        }
        fn deps(&self) -> TreeDeps<'_> {
            TreeDeps { fs: &self.fs, settings: &self.settings }
        }
    }

    #[test]
    fn rename_stays_in_the_same_folder() {
        let world = World::new();
        rename(&world.deps(), "/p/src/a.rs", "b.rs").unwrap();
        assert_eq!(world.fs.renames(), vec![("/p/src/a.rs".into(), "/p/src/b.rs".into())]);
    }

    #[test]
    fn rename_rejects_path_traversal_in_the_new_name() {
        let world = World::new();
        for name in ["../evil.rs", "..", "a/b", "a\\b", "  "] {
            assert!(
                matches!(rename(&world.deps(), "/p/src/a.rs", name), Err(AppError::InvalidPath(_))),
                "{name} 은 거부되어야 한다"
            );
        }
        assert!(world.fs.renames().is_empty());
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let world = World::new();
        world.fs.add_file("/p/src/b.rs");
        assert!(rename(&world.deps(), "/p/src/a.rs", "b.rs").is_err());
        assert!(world.fs.renames().is_empty());
    }

    #[test]
    fn move_puts_the_item_under_the_destination() {
        let world = World::new();
        move_(&world.deps(), "/p/src/a.rs", "/p/docs").unwrap();
        assert_eq!(world.fs.renames(), vec![("/p/src/a.rs".into(), "/p/docs/a.rs".into())]);
    }

    #[test]
    fn move_into_same_parent_is_a_no_op() {
        let world = World::new();
        move_(&world.deps(), "/p/src/a.rs", "/p/src").unwrap();
        assert!(world.fs.renames().is_empty());
    }

    #[test]
    fn moving_a_folder_into_itself_fails() {
        let world = World::new();
        assert!(move_(&world.deps(), "/p/src", "/p/src/inner").is_err());
        assert!(move_(&world.deps(), "/p/src", "/p/src").is_err());
    }

    #[test]
    fn nothing_can_leave_the_project() {
        let world = World::new();
        assert!(move_(&world.deps(), "/p/src/a.rs", "/outside").is_err());
        assert!(move_(&world.deps(), "/outside/a.rs", "/p/src").is_err());
        assert!(delete(&world.deps(), "/outside/a.rs").is_err());
        assert!(create_file(&world.deps(), "/outside", "a.rs").is_err());
        assert!(world.fs.renames().is_empty() && world.fs.trashed().is_empty());
    }

    #[test]
    fn project_root_cannot_be_deleted_or_renamed() {
        let world = World::new();
        assert!(delete(&world.deps(), "/p").is_err());
        assert!(rename(&world.deps(), "/p", "q").is_err());
        assert!(world.fs.trashed().is_empty());
    }

    #[test]
    fn delete_goes_to_trash() {
        let world = World::new();
        delete(&world.deps(), "/p/src/a.rs").unwrap();
        assert_eq!(world.fs.trashed(), vec!["/p/src/a.rs".to_string()]);
    }

    #[test]
    fn create_refuses_existing_names() {
        let world = World::new();
        world.fs.add_file("/p/src/a.rs");
        assert!(create_file(&world.deps(), "/p/src", "a.rs").is_err());
        assert_eq!(create_file(&world.deps(), "/p/src", "b.rs").unwrap(), Path::new("/p/src").join("b.rs").to_string_lossy());
    }

    #[derive(Default)]
    struct FakeShell(std::sync::Mutex<Vec<String>>);

    impl ShellRepository for FakeShell {
        fn reveal_in_explorer(&self, _path: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn open_in_browser(&self, _path: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn run_executable(&self, path: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().push(path.into());
            Ok(())
        }
    }

    #[test]
    fn only_exe_files_inside_the_project_run() {
        let world = World::new();
        let shell = FakeShell::default();
        run_executable(&world.deps(), &shell, "/p/bin/tool.EXE").unwrap();
        assert!(run_executable(&world.deps(), &shell, "/p/run.bat").is_err());
        assert!(run_executable(&world.deps(), &shell, "/p/src/a.rs").is_err());
        assert!(matches!(
            run_executable(&world.deps(), &shell, "/outside/tool.exe"),
            Err(AppError::OutsideProject(_))
        ));
        assert_eq!(*shell.0.lock().unwrap(), vec!["/p/bin/tool.EXE".to_string()]);
    }

    #[test]
    fn copy_path_writes_absolute_or_relative() {
        let world = World::new();
        let clipboard = FakeClipboard::default();
        copy_path(&world.deps(), &clipboard, "/p/src/a.rs", false).unwrap();
        copy_path(&world.deps(), &clipboard, "/p/src/a.rs", true).unwrap();
        assert_eq!(clipboard.texts(), vec!["/p/src/a.rs".to_string(), "src/a.rs".to_string()]);
    }
}
