use std::sync::Mutex;

use crate::domain::repository::{
    ClaudeBinaryRepository, ClaudeProcessRepository, ClipboardRepository, FileSystemRepository,
    GitRepository, SessionRepository, SettingsRepository, ShellRepository, TerminalRepository, WatchGuard,
};
use crate::domain::usecase::git::GitDeps;
use crate::domain::usecase::terminal::TerminalDeps;
use crate::domain::usecase::project::ProjectDeps;
use crate::domain::usecase::session::SendDeps;
use crate::domain::usecase::tree::TreeDeps;

/// di 가 조립해 tauri 에 넣는다. viewmodel 은 trait 객체만 안다.
pub struct AppState {
    pub binaries: Box<dyn ClaudeBinaryRepository>,
    pub sessions: Box<dyn SessionRepository>,
    pub process: Box<dyn ClaudeProcessRepository>,
    pub fs: Box<dyn FileSystemRepository>,
    pub clipboard: Box<dyn ClipboardRepository>,
    pub shell: Box<dyn ShellRepository>,
    pub settings: Box<dyn SettingsRepository>,
    pub git: Box<dyn GitRepository>,
    pub terminals: Box<dyn TerminalRepository>,
    /// 지금 트리에 보이는 프로젝트의 감시. 다른 프로젝트를 고르면 바뀐다.
    pub tree_watch: Mutex<Option<WatchGuard>>,
    pub session_watch: Mutex<Option<WatchGuard>>,
}

impl AppState {
    pub fn project_deps(&self) -> ProjectDeps<'_> {
        ProjectDeps { settings: &*self.settings, sessions: &*self.sessions, process: &*self.process }
    }

    pub fn tree_deps(&self) -> TreeDeps<'_> {
        TreeDeps { fs: &*self.fs, settings: &*self.settings }
    }

    pub fn terminal_deps(&self) -> TerminalDeps<'_> {
        TerminalDeps { terminals: &*self.terminals, fs: &*self.fs, settings: &*self.settings }
    }

    pub fn git_deps(&self) -> GitDeps<'_> {
        GitDeps { git: &*self.git, fs: &*self.fs, settings: &*self.settings }
    }

    pub fn send_deps(&self) -> SendDeps<'_> {
        SendDeps {
            settings: &*self.settings,
            sessions: &*self.sessions,
            process: &*self.process,
            fs: &*self.fs,
        }
    }
}
