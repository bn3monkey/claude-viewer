//! usecase 테스트에 주입하는 Fake 리포지토리.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;

use crate::domain::entity::{
    AppError, ChatEvent, FsKind, FsNode, GitBranch, GitChange, GitCommit, ProjectEntry, Settings, TextContent,
};
use crate::domain::repository::{
    ClaudeBinaryRepository, ClaudeCandidate, ClaudeProcessRepository, ClipboardRepository,
    FileSystemRepository, GitRepository, ProcessStatus, SessionLogMeta, SessionRepository,
    SettingsRepository, SpawnSpec, WatchGuard,
};

pub fn meta(id: &str, title: &str, looks_running: bool) -> SessionLogMeta {
    SessionLogMeta {
        id: id.into(),
        title: Some(title.into()),
        message_count: 1,
        updated_at: 0.0,
        looks_running,
    }
}

pub struct FakeBinaries {
    pub candidates: Vec<ClaudeCandidate>,
    pub working: Vec<String>,
}

impl ClaudeBinaryRepository for FakeBinaries {
    fn locate_candidates(&self) -> Vec<ClaudeCandidate> {
        self.candidates.clone()
    }
    fn probe_version(&self, path: &str) -> Result<String, AppError> {
        if self.working.iter().any(|p| p == path) {
            Ok("1.0.0".into())
        } else {
            Err(AppError::Process("실행 실패".into()))
        }
    }
}

#[derive(Default)]
pub struct FakeSettings(Mutex<Settings>);

impl FakeSettings {
    pub fn with_project(root: &str, active: &[&str]) -> Self {
        Self(Mutex::new(Settings {
            projects: vec![ProjectEntry {
                root: root.into(),
                active_sessions: active.iter().map(|s| s.to_string()).collect(),
            }],
            ..Settings::default()
        }))
    }
}

impl SettingsRepository for FakeSettings {
    fn load(&self) -> Settings {
        self.0.lock().unwrap().clone()
    }
    fn update(&self, change: &mut dyn FnMut(&mut Settings)) -> Result<Settings, AppError> {
        let mut settings = self.0.lock().unwrap();
        change(&mut settings);
        Ok(settings.clone())
    }
}

#[derive(Default)]
pub struct FakeSessions(Mutex<HashMap<String, SessionLogMeta>>);

impl FakeSessions {
    pub fn add(&self, meta: SessionLogMeta) {
        self.0.lock().unwrap().insert(meta.id.clone(), meta);
    }
}

impl SessionRepository for FakeSessions {
    fn scan(&self, _root: &str) -> Result<Vec<SessionLogMeta>, AppError> {
        Ok(self.0.lock().unwrap().values().cloned().collect())
    }
    fn meta(&self, _root: &str, session_id: &str) -> Option<SessionLogMeta> {
        self.0.lock().unwrap().get(session_id).cloned()
    }
    fn read_log(&self, _root: &str, _session_id: &str) -> Result<Vec<ChatEvent>, AppError> {
        Ok(Vec::new())
    }
    fn owns_session_dir(&self, _root: &str, _dir_name: &str) -> bool {
        false
    }
    fn watch(&self, _on_change: Box<dyn Fn(Vec<String>) + Send + Sync>) -> Result<WatchGuard, AppError> {
        Ok(Box::new(()))
    }
}

#[derive(Default)]
pub struct FakeProcess {
    statuses: Mutex<HashMap<String, ProcessStatus>>,
    spawned: Mutex<Vec<SpawnSpec>>,
    written: Mutex<Vec<(String, String)>>,
    killed: Mutex<Vec<String>>,
    stopped: Mutex<HashMap<String, f64>>,
    permissions: Mutex<Vec<(String, bool, Option<String>)>>,
    answers: Mutex<Vec<(String, Vec<(String, String)>)>>,
}

impl FakeProcess {
    pub fn permission_responses(&self) -> Vec<(String, bool, Option<String>)> {
        self.permissions.lock().unwrap().clone()
    }
    pub fn question_answers(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.answers.lock().unwrap().clone()
    }
    pub fn set_stopped_at(&self, id: &str, at: f64) {
        self.stopped.lock().unwrap().insert(id.into(), at);
    }
    pub fn set_status(&self, id: &str, busy: bool) {
        self.set_status_with_model(id, busy, "opus");
    }
    pub fn set_status_with_model(&self, id: &str, busy: bool, model: &str) {
        self.statuses
            .lock()
            .unwrap()
            .insert(id.into(), ProcessStatus { busy, model: model.into(), permission_mode: None });
    }
    pub fn spawned(&self) -> Vec<SpawnSpec> {
        self.spawned.lock().unwrap().clone()
    }
    pub fn written(&self) -> Vec<(String, String)> {
        self.written.lock().unwrap().clone()
    }
    pub fn killed(&self) -> Vec<String> {
        self.killed.lock().unwrap().clone()
    }
}

impl ClaudeProcessRepository for FakeProcess {
    fn spawn(&self, spec: SpawnSpec) -> Result<(), AppError> {
        self.statuses.lock().unwrap().insert(
            spec.session_id.clone(),
            ProcessStatus { busy: false, model: spec.model.clone(), permission_mode: spec.permission_mode.clone() },
        );
        self.spawned.lock().unwrap().push(spec);
        Ok(())
    }
    fn respond_permission(
        &self,
        _session_id: &str,
        request_id: &str,
        allow: bool,
        updated_permissions: Option<&str>,
    ) -> Result<(), AppError> {
        self.permissions.lock().unwrap().push((request_id.into(), allow, updated_permissions.map(String::from)));
        Ok(())
    }
    fn answer_question(
        &self,
        _session_id: &str,
        request_id: &str,
        _input: &str,
        answers: &[(String, String)],
    ) -> Result<(), AppError> {
        self.answers.lock().unwrap().push((request_id.into(), answers.to_vec()));
        Ok(())
    }
    fn write_stdin(&self, session_id: &str, text: &str) -> Result<(), AppError> {
        self.written.lock().unwrap().push((session_id.into(), text.into()));
        Ok(())
    }
    fn kill(&self, session_id: &str) -> Result<(), AppError> {
        self.statuses.lock().unwrap().remove(session_id);
        self.killed.lock().unwrap().push(session_id.into());
        Ok(())
    }
    fn kill_all(&self) {}
    fn status(&self, session_id: &str) -> Option<ProcessStatus> {
        self.statuses.lock().unwrap().get(session_id).cloned()
    }
    fn stopped_at(&self, session_id: &str) -> Option<f64> {
        self.stopped.lock().unwrap().get(session_id).copied()
    }
    fn subscribe(&self) -> Receiver<(String, ChatEvent)> {
        channel().1
    }
}

/// 실제 디스크 없이 경로만 다룬다. `..` 는 풀어서 canonicalize 를 흉내 낸다.
#[derive(Default)]
pub struct FakeFs {
    dirs: Mutex<HashSet<PathBuf>>,
    files: Mutex<HashSet<PathBuf>>,
    renames: Mutex<Vec<(PathBuf, PathBuf)>>,
    trashed: Mutex<Vec<String>>,
}

impl FakeFs {
    pub fn with_dirs(dirs: &[&str]) -> Self {
        let fs = Self::default();
        fs.dirs.lock().unwrap().extend(dirs.iter().map(PathBuf::from));
        fs
    }
    pub fn add_file(&self, path: &str) {
        self.files.lock().unwrap().insert(path.into());
    }
    pub fn renames(&self) -> Vec<(PathBuf, PathBuf)> {
        self.renames.lock().unwrap().clone()
    }
    pub fn trashed(&self) -> Vec<String> {
        self.trashed.lock().unwrap().clone()
    }
}

impl FileSystemRepository for FakeFs {
    fn list(&self, root: &str) -> Result<FsNode, AppError> {
        Ok(FsNode { path: root.into(), name: root.into(), kind: FsKind::Dir, children: Some(Vec::new()) })
    }
    fn canonicalize(&self, path: &str) -> Result<PathBuf, AppError> {
        let mut out = PathBuf::new();
        for component in Path::new(path).components() {
            match component {
                Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other),
            }
        }
        Ok(out)
    }
    fn is_dir(&self, path: &str) -> bool {
        self.dirs.lock().unwrap().contains(Path::new(path))
    }
    fn exists(&self, path: &Path) -> bool {
        self.dirs.lock().unwrap().contains(path) || self.files.lock().unwrap().contains(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), AppError> {
        self.renames.lock().unwrap().push((from.into(), to.into()));
        Ok(())
    }
    fn trash(&self, path: &str) -> Result<(), AppError> {
        self.trashed.lock().unwrap().push(path.into());
        Ok(())
    }
    fn create_file(&self, path: &Path) -> Result<(), AppError> {
        self.add_file(&path.to_string_lossy());
        Ok(())
    }
    fn create_dir(&self, path: &Path) -> Result<(), AppError> {
        self.dirs.lock().unwrap().insert(path.into());
        Ok(())
    }
    fn read_text(&self, _path: &str, _max_bytes: u64) -> Result<TextContent, AppError> {
        Ok(TextContent { text: String::new(), truncated: false, binary: false })
    }
    fn read_bytes(&self, _path: &str) -> Result<Vec<u8>, AppError> {
        Ok(Vec::new())
    }
    fn watch(&self, _root: &str, _on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchGuard, AppError> {
        Ok(Box::new(()))
    }
}

/// 고정된 브랜치·변경 목록을 돌려준다.
#[derive(Default)]
pub struct FakeGit {
    pub repo: bool,
    pub branches: Vec<GitBranch>,
    pub working: Vec<GitChange>,
    pub diff: Option<String>,
}

impl GitRepository for FakeGit {
    fn is_repo(&self, _root: &str) -> bool {
        self.repo
    }
    fn branches(&self, _root: &str) -> Result<Vec<GitBranch>, AppError> {
        Ok(self.branches.clone())
    }
    fn commits(&self, _root: &str, _branch: Option<&str>, _limit: usize) -> Result<Vec<GitCommit>, AppError> {
        Ok(Vec::new())
    }
    fn commit_detail(&self, _root: &str, hash: &str) -> Result<String, AppError> {
        Ok(format!("detail {hash}"))
    }
    fn working_changes(&self, _root: &str) -> Result<Vec<GitChange>, AppError> {
        Ok(self.working.clone())
    }
    fn branch_changes(&self, _root: &str, _branch: &str) -> Result<Vec<GitChange>, AppError> {
        Ok(Vec::new())
    }
    fn diff_working(&self, _root: &str, _path: &str, _untracked: bool) -> Result<Option<String>, AppError> {
        Ok(self.diff.clone())
    }
    fn diff_branch(&self, _root: &str, _branch: &str, _path: &str) -> Result<Option<String>, AppError> {
        Ok(self.diff.clone())
    }
    fn commit_changes(&self, _root: &str, _hash: &str) -> Result<Vec<GitChange>, AppError> {
        Ok(self.working.clone())
    }
    fn diff_commit(&self, _root: &str, hash: &str, _path: &str) -> Result<Option<String>, AppError> {
        Ok(Some(format!("commit {hash}")))
    }
    fn pull(&self, _root: &str, _branch: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn reset_hard(&self, _root: &str) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeClipboard(Mutex<Vec<String>>);

impl FakeClipboard {
    pub fn texts(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl ClipboardRepository for FakeClipboard {
    fn write_text(&self, text: &str) -> Result<(), AppError> {
        self.0.lock().unwrap().push(text.into());
        Ok(())
    }
}
