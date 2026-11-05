//! 조립. 모든 Repository 구현체를 만들어 AppState 에 넣고 tauri 를 실행한다.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, RunEvent};

use crate::data::datasource::settings_store::SettingsStore;
use crate::data::repository::{
    claude_binary_repository_impl::ClaudeBinaryRepositoryImpl,
    claude_process_repository_impl::ClaudeProcessRepositoryImpl,
    clipboard_repository_impl::ClipboardRepositoryImpl, file_system_repository_impl::FileSystemRepositoryImpl,
    git_repository_impl::GitRepositoryImpl, session_repository_impl::SessionRepositoryImpl,
    settings_repository_impl::SettingsRepositoryImpl, shell_repository_impl::ShellRepositoryImpl,
    terminal_repository_impl::TerminalRepositoryImpl,
};
use crate::infrastructure::platform::{self, Platform};
use crate::presentation::state::AppState;
use crate::presentation::viewmodel::{
    connection_vm, git_vm, project_vm, session_vm, terminal_vm, tree_vm, viewer_vm,
};

pub fn run() {
    let platform = platform::current();
    if !platform.ensure_webview_runtime() {
        return;
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .register_uri_scheme_protocol("htmlview", |ctx, request| viewer_vm::serve_html(ctx.app_handle(), request))
        .setup(move |app| {
            let state = build(app.handle(), platform)?;
            for project in state.settings.load().projects {
                project_vm::allow_assets(app.handle(), &project.root);
            }
            app.manage(state);
            session_vm::start_event_pump(app.handle());
            terminal_vm::start_terminal_pump(app.handle());
            project_vm::start_session_watch(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connection_vm::connection_get,
            connection_vm::connection_find_auto,
            connection_vm::connection_set_manual,
            connection_vm::settings_get,
            project_vm::project_list,
            project_vm::project_open,
            project_vm::project_close,
            session_vm::session_list,
            session_vm::session_start,
            session_vm::session_resume,
            session_vm::session_read_log,
            session_vm::session_send_message,
            session_vm::session_compact,
            session_vm::session_respond_permission,
            session_vm::session_answer_question,
            session_vm::session_save_draft,
            session_vm::session_abort,
            session_vm::session_close,
            tree_vm::tree_read,
            tree_vm::tree_rename,
            tree_vm::tree_move,
            tree_vm::tree_delete,
            tree_vm::tree_create_file,
            tree_vm::tree_create_dir,
            tree_vm::tree_copy_path,
            tree_vm::tree_reveal,
            tree_vm::tree_run,
            viewer_vm::viewer_resolve_mode,
            viewer_vm::viewer_resolve_link,
            viewer_vm::viewer_save_tabs,
            viewer_vm::viewer_restore_tabs,
            viewer_vm::viewer_read_text,
            viewer_vm::viewer_read_bytes,
            viewer_vm::viewer_open_external,
            git_vm::git_overview,
            git_vm::git_commits,
            git_vm::git_commit_detail,
            git_vm::git_changes,
            git_vm::git_diff,
            git_vm::git_commit_changes,
            git_vm::git_pull,
            git_vm::git_reset_hard,
            terminal_vm::terminal_default_kind,
            terminal_vm::terminal_launch,
            terminal_vm::terminal_open_shell,
            terminal_vm::terminal_repo_shell,
            terminal_vm::terminal_write,
            terminal_vm::terminal_resize,
            terminal_vm::terminal_close,
            terminal_vm::terminal_paths,
            terminal_vm::terminal_save_paths,
        ])
        .build(tauri::generate_context!())
        .expect("tauri 앱을 만들지 못했습니다");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.process.kill_all();
                state.terminals.kill_all();
            }
        }
    });
}

fn build(app: &AppHandle, platform: Box<dyn Platform>) -> Result<AppState, Box<dyn std::error::Error>> {
    let home = platform.home_dir().ok_or("홈 폴더를 찾을 수 없습니다")?;
    let app_data = platform.app_data_dir().ok_or("설정 폴더를 찾을 수 없습니다")?;
    let store = SettingsStore::open(app, app_data.join("settings.json"))?;
    let runtime = tauri::async_runtime::handle().inner().clone();
    let terminals = TerminalRepositoryImpl::new(platform.terminal_defaults(), platform.is_windows());

    Ok(AppState {
        binaries: Box::new(ClaudeBinaryRepositoryImpl { platform }),
        sessions: Box::new(SessionRepositoryImpl { projects_dir: home.join(".claude").join("projects") }),
        process: Box::new(ClaudeProcessRepositoryImpl::new(runtime)),
        fs: Box::new(FileSystemRepositoryImpl),
        clipboard: Box::new(ClipboardRepositoryImpl { app: app.clone() }),
        shell: Box::new(ShellRepositoryImpl { app: app.clone() }),
        settings: Box::new(SettingsRepositoryImpl::new(store)),
        git: Box::new(GitRepositoryImpl),
        terminals: Box::new(terminals),
        tree_watch: Mutex::new(None),
        session_watch: Mutex::new(None),
    })
}
