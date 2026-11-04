use tauri::{AppHandle, Emitter, Manager, State};

use super::project_vm::emit_project;
use super::{session_event_name, EVENT_VIEWER_OPEN};
use crate::domain::entity::{AppError, ChatEvent, Session};
use crate::domain::usecase::session::{self, SendMessage};
use crate::domain::usecase::viewer;
use crate::presentation::state::AppState;

#[tauri::command(async)]
pub fn session_list(state: State<'_, AppState>, root: String) -> Result<Vec<Session>, AppError> {
    session::list_sessions(&*state.sessions, &*state.process, &root)
}

#[tauri::command(async)]
pub fn session_start(app: AppHandle, state: State<'_, AppState>, root: String) -> Result<String, AppError> {
    let id = session::start_session(&*state.settings, &root)?;
    emit_project(&app, &root);
    Ok(id)
}

#[tauri::command(async)]
pub fn session_resume(app: AppHandle, state: State<'_, AppState>, root: String, id: String) -> Result<(), AppError> {
    session::resume_session(&*state.settings, &root, &id)?;
    emit_project(&app, &root);
    Ok(())
}

#[tauri::command(async)]
pub fn session_read_log(state: State<'_, AppState>, root: String, id: String) -> Result<Vec<ChatEvent>, AppError> {
    session::read_log(&*state.sessions, &root, &id)
}

#[tauri::command(async)]
pub fn session_send_message(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    session_id: String,
    text: String,
    model: String,
    permission_mode: Option<String>,
    attached_file: Option<String>,
) -> Result<(), AppError> {
    let request = SendMessage {
        root: &root,
        session_id: &session_id,
        text: &text,
        model: &model,
        permission_mode: permission_mode.as_deref(),
        attached_file: attached_file.as_deref(),
    };
    let body = session::send_message(&state.send_deps(), &request)?;
    let _ = app.emit(&session_event_name(&session_id), ChatEvent::User { text: body });
    emit_project(&app, &root);
    Ok(())
}

#[tauri::command(async)]
pub fn session_compact(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    session_id: String,
    model: String,
    permission_mode: Option<String>,
) -> Result<(), AppError> {
    session::compact(&state.send_deps(), &root, &session_id, &model, permission_mode.as_deref())?;
    emit_project(&app, &root);
    Ok(())
}

#[tauri::command(async)]
pub fn session_respond_permission(
    state: State<'_, AppState>,
    session_id: String,
    request_id: String,
    allow: bool,
    always_suggestions: Option<String>,
) -> Result<(), AppError> {
    session::respond_permission(&*state.process, &session_id, &request_id, allow, always_suggestions.as_deref())
}

/// `answers` 는 [질문, 답] 쌍의 배열.
#[tauri::command(async)]
pub fn session_answer_question(
    state: State<'_, AppState>,
    session_id: String,
    request_id: String,
    input: String,
    answers: Vec<(String, String)>,
) -> Result<(), AppError> {
    session::answer_question(&*state.process, &session_id, &request_id, &input, &answers)
}

/// 입력창 내용. 프론트가 타이핑을 잠시 멈췄을 때 보낸다.
#[tauri::command(async)]
pub fn session_save_draft(state: State<'_, AppState>, session_id: String, text: String) -> Result<(), AppError> {
    session::save_draft(&*state.settings, &session_id, &text)
}

#[tauri::command(async)]
pub fn session_abort(state: State<'_, AppState>, session_id: String) -> Result<(), AppError> {
    session::abort_session(&*state.process, &session_id)
}

#[tauri::command(async)]
pub fn session_close(app: AppHandle, state: State<'_, AppState>, root: String, id: String) -> Result<(), AppError> {
    session::close_session(&*state.settings, &*state.process, &root, &id)?;
    emit_project(&app, &root);
    Ok(())
}

/// 프로세스 출력 채널을 받아 프론트로 emit 한다. 응답의 뷰어 지시문은 떼어내 뷰어 열기 이벤트로 보낸다.
pub fn start_event_pump(app: &AppHandle) {
    let receiver = session::subscribe(&*app.state::<AppState>().process);
    let app = app.clone();
    std::thread::spawn(move || {
        for (session_id, event) in receiver {
            let state = app.state::<AppState>();
            let root = session::project_root_of(&*state.settings, &session_id);
            let event = match event {
                ChatEvent::AssistantText { text } => {
                    let (text, directives) = viewer::parse_viewer_directives(&text);
                    for directive in directives {
                        let request = root
                            .as_deref()
                            .ok_or_else(|| AppError::NotFound(session_id.clone()))
                            .and_then(|root| viewer::resolve_directive(&*state.fs, &*state.settings, root, &directive));
                        match request {
                            Ok(request) => {
                                let _ = app.emit(EVENT_VIEWER_OPEN, request);
                            }
                            Err(e) => {
                                let _ = app.emit(&session_event_name(&session_id), ChatEvent::Error { message: e.to_string() });
                            }
                        }
                    }
                    ChatEvent::AssistantText { text }
                }
                other => other,
            };
            let finished = matches!(event, ChatEvent::Result { .. } | ChatEvent::Exited { .. });
            let _ = app.emit(&session_event_name(&session_id), event);
            if let (true, Some(root)) = (finished, root) {
                emit_project(&app, &root);
            }
        }
    });
}
