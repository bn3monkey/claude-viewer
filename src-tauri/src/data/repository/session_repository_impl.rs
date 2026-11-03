use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::data::datasource::jsonl_session_log::{
    encode_project_dir, read_events, read_meta, session_dirs, session_files,
};
use crate::domain::entity::{AppError, ChatEvent};
use crate::domain::repository::{SessionLogMeta, SessionRepository, WatchGuard};
use crate::infrastructure::watcher;

pub struct SessionRepositoryImpl {
    /// `~/.claude/projects`
    pub projects_dir: PathBuf,
}

impl SessionRepositoryImpl {
    fn log_path(&self, root: &str, session_id: &str) -> Option<PathBuf> {
        session_dirs(&self.projects_dir, root)
            .into_iter()
            .map(|dir| dir.join(format!("{session_id}.jsonl")))
            .find(|path| path.is_file())
    }
}

impl SessionRepository for SessionRepositoryImpl {
    fn scan(&self, root: &str) -> Result<Vec<SessionLogMeta>, AppError> {
        let mut metas: Vec<SessionLogMeta> = session_dirs(&self.projects_dir, root)
            .iter()
            .flat_map(|dir| session_files(dir))
            .filter_map(|path| read_meta(&path))
            // 대화 없이 끝난 로그(설정 확인 등)는 다이얼로그에 보여줄 이유가 없다.
            .filter(|meta| meta.message_count > 0)
            .collect();
        metas.sort_by(|a, b| b.updated_at.total_cmp(&a.updated_at));
        Ok(metas)
    }

    fn meta(&self, root: &str, session_id: &str) -> Option<SessionLogMeta> {
        read_meta(&self.log_path(root, session_id)?)
    }

    fn read_log(&self, root: &str, session_id: &str) -> Result<Vec<ChatEvent>, AppError> {
        match self.log_path(root, session_id) {
            Some(path) => read_events(&path).map_err(|e| AppError::Io(e.to_string())),
            None => Ok(Vec::new()),
        }
    }

    fn owns_session_dir(&self, root: &str, dir_name: &str) -> bool {
        encode_project_dir(root).eq_ignore_ascii_case(dir_name)
    }

    fn watch(&self, on_change: Box<dyn Fn(Vec<String>) + Send + Sync>) -> Result<WatchGuard, AppError> {
        let projects_dir = self.projects_dir.clone();
        let handle = watcher::watch(&self.projects_dir, Duration::from_millis(800), move |paths| {
            let mut dirs: Vec<String> = paths
                .iter()
                .filter(|path| path.extension().map_or(false, |ext| ext == "jsonl"))
                .filter_map(|path| session_dir_name(&projects_dir, path))
                .collect();
            dirs.sort();
            dirs.dedup();
            if !dirs.is_empty() {
                on_change(dirs);
            }
        })
        .map_err(AppError::Io)?;
        Ok(handle)
    }
}

/// `<projects>/<dir>/<id>.jsonl` 의 `<dir>`. 서브에이전트 로그처럼 더 깊은 파일은 무시한다.
fn session_dir_name(projects_dir: &Path, path: &Path) -> Option<String> {
    let dir = path.parent()?;
    if dir.parent()? != projects_dir {
        return None;
    }
    Some(dir.file_name()?.to_string_lossy().into_owned())
}
