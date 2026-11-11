use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::data::datasource::jsonl_session_log::{
    encode_project_dir, read_events, read_meta, read_subagent, read_subagent_events, session_dirs, session_files,
    subagent_file, subagent_files,
};
use crate::domain::entity::{AppError, ChatEvent, SubagentRun};
use crate::domain::repository::{SessionLogChange, SessionLogMeta, SessionRepository, WatchGuard};
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

    fn subagents(&self, root: &str, session_id: &str) -> Result<Vec<SubagentRun>, AppError> {
        let mut runs: Vec<SubagentRun> = session_dirs(&self.projects_dir, root)
            .iter()
            .flat_map(|dir| subagent_files(dir, session_id))
            .filter_map(|path| read_subagent(&path))
            .collect();
        runs.sort_by(|a, b| a.started_at.total_cmp(&b.started_at));
        Ok(runs)
    }

    fn read_subagent_log(&self, root: &str, session_id: &str, agent_id: &str) -> Result<Vec<ChatEvent>, AppError> {
        let path = session_dirs(&self.projects_dir, root)
            .iter()
            .map(|dir| subagent_file(dir, session_id, agent_id))
            .find(|path| path.is_file())
            .ok_or_else(|| AppError::NotFound(format!("서브에이전트 기록이 없습니다: {agent_id}")))?;
        read_subagent_events(&path).map_err(|e| AppError::Io(e.to_string()))
    }

    fn owns_session_dir(&self, root: &str, dir_name: &str) -> bool {
        encode_project_dir(root).eq_ignore_ascii_case(dir_name)
    }

    fn watch(&self, on_change: Box<dyn Fn(SessionLogChange) + Send + Sync>) -> Result<WatchGuard, AppError> {
        let projects_dir = self.projects_dir.clone();
        let handle = watcher::watch(&self.projects_dir, Duration::from_millis(800), move |paths| {
            let logs: Vec<&PathBuf> = paths.iter().filter(|path| path.extension().map_or(false, |ext| ext == "jsonl")).collect();
            let mut change = SessionLogChange {
                dirs: logs.iter().filter_map(|path| session_dir_name(&projects_dir, path)).collect(),
                subagent_sessions: logs.iter().filter_map(|path| subagent_session_id(&projects_dir, path)).collect(),
            };
            for list in [&mut change.dirs, &mut change.subagent_sessions] {
                list.sort();
                list.dedup();
            }
            if change != SessionLogChange::default() {
                on_change(change);
            }
        })
        .map_err(AppError::Io)?;
        Ok(handle)
    }
}

/// `<projects>/<dir>/<session-id>/subagents/agent-<id>.jsonl` 의 `<session-id>`.
fn subagent_session_id(projects_dir: &Path, path: &Path) -> Option<String> {
    let subagents = path.parent()?;
    let session = subagents.parent()?;
    let is_subagent_log = subagents.file_name()? == "subagents" && session.parent()?.parent()? == projects_dir;
    is_subagent_log.then(|| session.file_name().map(|n| n.to_string_lossy().into_owned())).flatten()
}

/// `<projects>/<dir>/<id>.jsonl` 의 `<dir>`. 서브에이전트 로그처럼 더 깊은 파일은 무시한다.
fn session_dir_name(projects_dir: &Path, path: &Path) -> Option<String> {
    let dir = path.parent()?;
    if dir.parent()? != projects_dir {
        return None;
    }
    Some(dir.file_name()?.to_string_lossy().into_owned())
}
