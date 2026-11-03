use std::path::{Path, PathBuf};

use crate::domain::entity::session::NEW_SESSION_TITLE;
use crate::domain::entity::{AppError, ProjectEntry, Project, Session, SessionState};
use crate::domain::repository::{
    ClaudeProcessRepository, FileSystemRepository, SessionLogMeta, SessionRepository, SettingsRepository,
};

pub struct ProjectDeps<'a> {
    pub settings: &'a dyn SettingsRepository,
    pub sessions: &'a dyn SessionRepository,
    pub process: &'a dyn ClaudeProcessRepository,
}

pub fn list_projects(deps: &ProjectDeps) -> Vec<Project> {
    deps.settings.load().projects.iter().map(|entry| build_project(deps, entry)).collect()
}

pub fn get_project(deps: &ProjectDeps, root: &str) -> Option<Project> {
    let settings = deps.settings.load();
    let entry = settings.projects.iter().find(|p| p.root == root)?;
    Some(build_project(deps, entry))
}

/// 이미 열려 있는 폴더를 다시 고르면 새로 추가하지 않고 기존 프로젝트를 돌려준다.
/// 처음 여는 폴더는 활성 세션 없이 시작한다. 기존 세션은 "새 활성화 세션" 다이얼로그에서 고른다.
pub fn open_project(
    deps: &ProjectDeps,
    fs: &dyn FileSystemRepository,
    path: &str,
) -> Result<Project, AppError> {
    if !fs.is_dir(path) {
        return Err(AppError::InvalidPath(format!("폴더가 아닙니다: {path}")));
    }
    let canonical = fs.canonicalize(path)?;
    let existing = deps.settings.load().projects.into_iter().find(|p| {
        fs.canonicalize(&p.root).map_or(false, |root| root == canonical)
    });
    let root = match existing {
        Some(entry) => entry.root,
        None => {
            deps.settings.update(&mut |s| {
                s.projects.push(ProjectEntry { root: path.to_string(), active_sessions: Vec::new() });
            })?;
            path.to_string()
        }
    };
    get_project(deps, &root).ok_or_else(|| AppError::NotFound(root))
}

/// 프로젝트를 닫으면 이 앱이 띄운 세션 프로세스도 함께 끝낸다. 로그는 남는다.
pub fn close_project(deps: &ProjectDeps, root: &str) -> Result<(), AppError> {
    let settings = deps.settings.load();
    if let Some(entry) = settings.projects.iter().find(|p| p.root == root) {
        for id in &entry.active_sessions {
            deps.process.kill(id)?;
        }
    }
    deps.settings.update(&mut |s| s.projects.retain(|p| p.root != root))?;
    Ok(())
}

fn build_project(deps: &ProjectDeps, entry: &ProjectEntry) -> Project {
    let sessions: Vec<Session> = entry
        .active_sessions
        .iter()
        .map(|id| {
            let meta = deps.sessions.meta(&entry.root, id);
            to_session(deps.process, &entry.root, id, meta)
        })
        .collect();
    let done = sessions.iter().filter(|s| s.state == SessionState::Done).count() as u32;
    Project {
        root: entry.root.clone(),
        name: folder_name(&entry.root),
        done,
        total: sessions.len() as u32,
        sessions,
    }
}

/// 이 앱이 멈춘 뒤 이만큼 지나서도 로그가 쓰였으면 다른 곳에서 이어간 것으로 본다.
const STOPPED_HERE_GRACE_MS: f64 = 2000.0;

/// 이 앱이 띄운 프로세스의 상태를 먼저 믿고, 없으면 로그로 추정한다.
/// 첫 메시지 전의 새 세션은 진행 중으로 표시한다.
pub fn to_session(
    process: &dyn ClaudeProcessRepository,
    root: &str,
    id: &str,
    meta: Option<SessionLogMeta>,
) -> Session {
    let status = process.status(id);
    let busy = status.as_ref().map_or(false, |s| s.busy);
    // Abort 로 끊은 턴은 로그에 열린 채 남으므로, 멈춘 뒤의 쓰기만 다른 곳의 진행으로 본다.
    let written_after_stop = |m: &SessionLogMeta| {
        process.stopped_at(id).map_or(true, |stopped| m.updated_at > stopped + STOPPED_HERE_GRACE_MS)
    };
    let state = match (&status, &meta) {
        (Some(s), _) if s.busy => SessionState::Running,
        (Some(_), _) => SessionState::Done,
        (None, Some(m)) if m.looks_running && written_after_stop(m) => SessionState::Running,
        (None, Some(_)) => SessionState::Done,
        (None, None) => SessionState::Running,
    };
    Session {
        id: id.to_string(),
        project: root.to_string(),
        title: meta
            .as_ref()
            .and_then(|m| m.title.clone())
            .unwrap_or_else(|| NEW_SESSION_TITLE.to_string()),
        state,
        busy,
        message_count: meta.as_ref().map_or(0, |m| m.message_count),
        updated_at: meta.as_ref().map(|m| m.updated_at),
    }
}

fn folder_name(root: &str) -> String {
    Path::new(root)
        .file_name()
        .map_or_else(|| root.to_string(), |name| name.to_string_lossy().into_owned())
}

/// 프로젝트 범위 검사를 통과한 경로.
#[derive(Debug, PartialEq)]
pub struct ScopedPath {
    pub canonical: PathBuf,
    pub canonical_root: PathBuf,
}

impl ScopedPath {
    pub fn is_root(&self) -> bool {
        self.canonical == self.canonical_root
    }

    /// `/` 로 구분한 프로젝트 기준 상대 경로.
    pub fn relative(&self) -> String {
        self.canonical
            .strip_prefix(&self.canonical_root)
            .map(|rel| {
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default()
    }
}

/// 사용자가 연 프로젝트 안의 경로만 통과시킨다. 심볼릭 링크는 풀어서 검사한다.
pub fn ensure_within_project(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    path: &str,
) -> Result<ScopedPath, AppError> {
    let canonical = fs.canonicalize(path)?;
    settings
        .load()
        .projects
        .iter()
        .filter_map(|p| fs.canonicalize(&p.root).ok())
        .find(|root| canonical.starts_with(root))
        .map(|canonical_root| ScopedPath { canonical, canonical_root })
        .ok_or_else(|| AppError::OutsideProject(format!("열린 프로젝트 밖의 경로입니다: {path}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::session::NEW_SESSION_TITLE;
    use crate::domain::usecase::fakes::{meta, FakeFs, FakeProcess, FakeSessions, FakeSettings};

    fn deps<'a>(
        settings: &'a FakeSettings,
        sessions: &'a FakeSessions,
        process: &'a FakeProcess,
    ) -> ProjectDeps<'a> {
        ProjectDeps { settings, sessions, process }
    }

    #[test]
    fn counts_done_sessions_using_process_then_log_state() {
        let settings = FakeSettings::with_project("/p", &["busy", "idle", "logged", "external", "new"]);
        let process = FakeProcess::default();
        process.set_status("busy", true);
        process.set_status("idle", false);
        let sessions = FakeSessions::default();
        sessions.add(meta("busy", "처리 중", true));
        sessions.add(meta("idle", "끝남", false));
        sessions.add(meta("logged", "지난 세션", false));
        sessions.add(meta("external", "VSCode 세션", true));

        let project = get_project(&deps(&settings, &sessions, &process), "/p").unwrap();

        let states: Vec<_> = project.sessions.iter().map(|s| s.state).collect();
        use SessionState::*;
        assert_eq!(states, vec![Running, Done, Done, Running, Running]);
        assert_eq!((project.done, project.total), (2, 5));
        assert_eq!(project.sessions[4].title, NEW_SESSION_TITLE);
        assert!(project.sessions[0].busy && !project.sessions[3].busy);
    }

    #[test]
    fn session_aborted_here_is_done_until_someone_writes_to_it_again() {
        let settings = FakeSettings::with_project("/p", &["s"]);
        let (sessions, process) = (FakeSessions::default(), FakeProcess::default());
        let mut open_turn = meta("s", "t", true);
        open_turn.updated_at = 10_000.0;
        sessions.add(open_turn.clone());
        process.set_stopped_at("s", 10_500.0);

        let state = |sessions: &FakeSessions| get_project(&deps(&settings, sessions, &process), "/p").unwrap().sessions[0].state;
        assert_eq!(state(&sessions), SessionState::Done);

        open_turn.updated_at = 20_000.0;
        sessions.add(open_turn);
        assert_eq!(state(&sessions), SessionState::Running, "멈춘 뒤 다른 곳에서 이어 쓰면 진행 중");
    }

    #[test]
    fn opening_the_same_folder_twice_does_not_duplicate() {
        let settings = FakeSettings::default();
        let (sessions, process, fs) = (FakeSessions::default(), FakeProcess::default(), FakeFs::with_dirs(&["/p"]));
        let d = deps(&settings, &sessions, &process);

        open_project(&d, &fs, "/p").unwrap();
        let again = open_project(&d, &fs, "/p").unwrap();

        assert_eq!(again.name, "p");
        assert_eq!(settings.load().projects.len(), 1);
    }

    #[test]
    fn newly_opened_project_starts_without_active_sessions() {
        let settings = FakeSettings::default();
        let (sessions, process, fs) = (FakeSessions::default(), FakeProcess::default(), FakeFs::with_dirs(&["/p"]));
        sessions.add(meta("old1", "a", false));

        let project = open_project(&deps(&settings, &sessions, &process), &fs, "/p").unwrap();

        assert!(project.sessions.is_empty());
        assert_eq!((project.done, project.total), (0, 0));
    }

    #[test]
    fn opening_a_missing_folder_fails() {
        let settings = FakeSettings::default();
        let (sessions, process, fs) = (FakeSessions::default(), FakeProcess::default(), FakeFs::default());
        let result = open_project(&deps(&settings, &sessions, &process), &fs, "/none");
        assert!(matches!(result, Err(AppError::InvalidPath(_))));
    }

    #[test]
    fn closing_a_project_kills_its_sessions() {
        let settings = FakeSettings::with_project("/p", &["a", "b"]);
        let (sessions, process) = (FakeSessions::default(), FakeProcess::default());
        process.set_status("a", false);

        close_project(&deps(&settings, &sessions, &process), "/p").unwrap();

        assert_eq!(process.killed(), vec!["a".to_string(), "b".to_string()]);
        assert!(settings.load().projects.is_empty());
    }

    #[test]
    fn scope_check_rejects_paths_outside_open_projects() {
        let settings = FakeSettings::with_project("/p", &[]);
        let fs = FakeFs::with_dirs(&["/p"]);

        let inside = ensure_within_project(&fs, &settings, "/p/src/a.rs").unwrap();
        assert_eq!(inside.relative(), "src/a.rs");
        assert!(!inside.is_root());
        assert!(ensure_within_project(&fs, &settings, "/p").unwrap().is_root());

        assert!(matches!(
            ensure_within_project(&fs, &settings, "/p2/a.rs"),
            Err(AppError::OutsideProject(_))
        ));
    }
}
