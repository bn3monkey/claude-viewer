pub mod claude_binary_repository;
pub mod claude_process_repository;
pub mod clipboard_repository;
pub mod file_system_repository;
pub mod git_repository;
pub mod session_repository;
pub mod settings_repository;
pub mod shell_repository;
pub mod terminal_repository;

pub use claude_binary_repository::{ClaudeBinaryRepository, ClaudeCandidate};
pub use claude_process_repository::{ClaudeProcessRepository, ProcessStatus, SessionStart, SpawnSpec};
pub use clipboard_repository::ClipboardRepository;
pub use file_system_repository::FileSystemRepository;
pub use git_repository::GitRepository;
pub use session_repository::{SessionLogMeta, SessionRepository};
pub use settings_repository::SettingsRepository;
pub use shell_repository::ShellRepository;
pub use terminal_repository::{TerminalOutput, TerminalRepository, TerminalSpec};

/// 감시 핸들. drop 되면 감시가 멈춘다.
pub type WatchGuard = Box<dyn Send>;
