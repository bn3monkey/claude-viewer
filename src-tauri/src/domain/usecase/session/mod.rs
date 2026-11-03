use std::sync::mpsc::Receiver;

use super::project::{ensure_within_project, to_session};
use super::viewer::VIEWER_SYSTEM_PROMPT;
use crate::domain::entity::{AppError, ChatEvent, Session};
use crate::domain::repository::{
    ClaudeProcessRepository, FileSystemRepository, SessionRepository, SessionStart, SettingsRepository,
    SpawnSpec,
};

/// 새 활성화 세션 다이얼로그에 보여줄 프로젝트의 기존 세션 전체.
pub fn list_sessions(
    sessions: &dyn SessionRepository,
    process: &dyn ClaudeProcessRepository,
    root: &str,
) -> Result<Vec<Session>, AppError> {
    Ok(sessions
        .scan(root)?
        .into_iter()
        .map(|meta| to_session(process, root, &meta.id.clone(), Some(meta)))
        .collect())
}

/// 프로세스는 첫 메시지 때 띄운다. 그 전까지 모델을 바꿔도 다시 띄울 필요가 없다.
pub fn start_session(settings: &dyn SettingsRepository, root: &str) -> Result<String, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    activate(settings, root, &id)?;
    Ok(id)
}

pub fn resume_session(settings: &dyn SettingsRepository, root: &str, id: &str) -> Result<(), AppError> {
    activate(settings, root, id)
}

fn activate(settings: &dyn SettingsRepository, root: &str, id: &str) -> Result<(), AppError> {
    settings.update(&mut |s| {
        if let Some(project) = s.projects.iter_mut().find(|p| p.root == root) {
            if !project.active_sessions.iter().any(|active| active == id) {
                project.active_sessions.push(id.to_string());
            }
        }
    })?;
    Ok(())
}

pub struct SendMessage<'a> {
    pub root: &'a str,
    pub session_id: &'a str,
    pub text: &'a str,
    pub model: &'a str,
    /// `--permission-mode` 값. None 이면 CLI 기본값.
    pub permission_mode: Option<&'a str>,
    pub attached_file: Option<&'a str>,
}

pub struct SendDeps<'a> {
    pub settings: &'a dyn SettingsRepository,
    pub sessions: &'a dyn SessionRepository,
    pub process: &'a dyn ClaudeProcessRepository,
    pub fs: &'a dyn FileSystemRepository,
}

/// 필요하면 프로세스를 띄운 뒤 stdin 에 메시지 1줄을 쓴다. 실제로 보낸 본문을 돌려준다.
pub fn send_message(deps: &SendDeps, request: &SendMessage) -> Result<String, AppError> {
    let settings = deps.settings.load();
    let binary = settings
        .claude_path
        .clone()
        .ok_or_else(|| AppError::NotConnected("먼저 Claude 경로를 연결해 주세요.".into()))?;

    match deps.process.status(request.session_id) {
        // 응답 중이면 진행 중인 턴에 덧붙인다. 모델·권한 변경은 다시 띄워야 하므로 다음 메시지로 미룬다.
        Some(status) if status.busy => {}
        // 모델과 권한 모드는 실행 인자라서, 바뀌면 프로세스를 다시 띄워 이어간다.
        Some(status) if status.model != request.model || status.permission_mode.as_deref() != request.permission_mode => {
            deps.process.kill(request.session_id)?
        }
        _ => {}
    }

    let body = compose_body(deps, request)?;

    if deps.process.status(request.session_id).is_none() {
        let has_log = deps.sessions.meta(request.root, request.session_id).is_some();
        deps.process.spawn(SpawnSpec {
            binary,
            cwd: request.root.to_string(),
            session_id: request.session_id.to_string(),
            start: if has_log { SessionStart::Resume } else { SessionStart::New },
            model: request.model.to_string(),
            permission_mode: request.permission_mode.map(str::to_string),
            append_system_prompt: VIEWER_SYSTEM_PROMPT.to_string(),
        })?;
    }

    deps.process.write_stdin(request.session_id, &body)?;
    deps.settings.update(&mut |s| {
        s.selected_model = request.model.to_string();
        s.permission_mode = request.permission_mode.map(str::to_string);
    })?;
    Ok(body)
}

/// 권한 요청 응답. `always` 면 CLI 가 제안한 규칙을 함께 보내 세션 동안 다시 묻지 않게 한다.
pub fn respond_permission(
    process: &dyn ClaudeProcessRepository,
    session_id: &str,
    request_id: &str,
    allow: bool,
    always_suggestions: Option<&str>,
) -> Result<(), AppError> {
    process.respond_permission(session_id, request_id, allow, always_suggestions.filter(|_| allow))
}

/// AskUserQuestion 답. 답이 빈 질문은 빼고 보낸다 (CLI 는 답 없는 질문을 건너뛴 것으로 본다).
pub fn answer_question(
    process: &dyn ClaudeProcessRepository,
    session_id: &str,
    request_id: &str,
    input: &str,
    answers: &[(String, String)],
) -> Result<(), AppError> {
    let answers: Vec<(String, String)> = answers
        .iter()
        .filter(|(_, answer)| !answer.trim().is_empty())
        .map(|(question, answer)| (question.clone(), answer.trim().to_string()))
        .collect();
    process.answer_question(session_id, request_id, input, &answers)
}

/// `/compact` 는 CLI 가 사용자 메시지로 받아 처리한다.
pub fn compact(deps: &SendDeps, root: &str, session_id: &str, model: &str, permission_mode: Option<&str>) -> Result<(), AppError> {
    send_message(
        deps,
        &SendMessage { root, session_id, text: "/compact", model, permission_mode, attached_file: None },
    )?;
    Ok(())
}

/// 선택한 파일은 본문 앞에 `@상대경로` 로 붙인다.
fn compose_body(deps: &SendDeps, request: &SendMessage) -> Result<String, AppError> {
    let Some(file) = request.attached_file else {
        return Ok(request.text.to_string());
    };
    let relative = ensure_within_project(deps.fs, deps.settings, file)?.relative();
    let mention = if relative.contains(char::is_whitespace) {
        format!("@\"{relative}\"")
    } else {
        format!("@{relative}")
    };
    Ok(format!("{mention}\n\n{}", request.text))
}

/// v0 의 Abort 는 프로세스 kill 이다. 다음 메시지 때 `--resume` 으로 다시 띄운다.
pub fn abort_session(process: &dyn ClaudeProcessRepository, id: &str) -> Result<(), AppError> {
    process.kill(id)
}

pub fn close_session(
    settings: &dyn SettingsRepository,
    process: &dyn ClaudeProcessRepository,
    root: &str,
    id: &str,
) -> Result<(), AppError> {
    process.kill(id)?;
    settings.update(&mut |s| {
        if let Some(project) = s.projects.iter_mut().find(|p| p.root == root) {
            project.active_sessions.retain(|active| active != id);
        }
    })?;
    Ok(())
}

pub fn read_log(
    sessions: &dyn SessionRepository,
    root: &str,
    id: &str,
) -> Result<Vec<ChatEvent>, AppError> {
    sessions.read_log(root, id)
}

pub fn subscribe(process: &dyn ClaudeProcessRepository) -> Receiver<(String, ChatEvent)> {
    process.subscribe()
}

/// 세션의 프로젝트 루트. 로그 폴더가 아니라 설정의 활성 세션 목록으로 찾는다.
pub fn project_root_of(settings: &dyn SettingsRepository, id: &str) -> Option<String> {
    settings
        .load()
        .projects
        .into_iter()
        .find(|p| p.active_sessions.iter().any(|active| active == id))
        .map(|p| p.root)
}

/// 세션 로그 폴더 이름으로 영향받는 프로젝트 루트를 찾는다.
pub fn projects_of_session_dirs(
    settings: &dyn SettingsRepository,
    sessions: &dyn SessionRepository,
    dir_names: &[String],
) -> Vec<String> {
    settings
        .load()
        .projects
        .into_iter()
        .filter(|p| dir_names.iter().any(|dir| sessions.owns_session_dir(&p.root, dir)))
        .map(|p| p.root)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usecase::fakes::{meta, FakeFs, FakeProcess, FakeSessions, FakeSettings};

    struct World {
        settings: FakeSettings,
        sessions: FakeSessions,
        process: FakeProcess,
        fs: FakeFs,
    }

    impl World {
        fn new() -> Self {
            let settings = FakeSettings::with_project("/p", &["s"]);
            settings.update(&mut |s| s.claude_path = Some("claude".into())).unwrap();
            Self {
                settings,
                sessions: FakeSessions::default(),
                process: FakeProcess::default(),
                fs: FakeFs::with_dirs(&["/p"]),
            }
        }

        fn deps(&self) -> SendDeps<'_> {
            SendDeps {
                settings: &self.settings,
                sessions: &self.sessions,
                process: &self.process,
                fs: &self.fs,
            }
        }
    }

    fn message<'a>(model: &'a str, attached_file: Option<&'a str>) -> SendMessage<'a> {
        SendMessage { root: "/p", session_id: "s", text: "안녕", model, permission_mode: None, attached_file }
    }

    #[test]
    fn changing_permission_mode_restarts_the_process_and_persists_it() {
        let world = World::new();
        world.process.set_status_with_model("s", false, "opus");
        world.sessions.add(meta("s", "t", false));

        let request = SendMessage { permission_mode: Some("acceptEdits"), ..message("opus", None) };
        send_message(&world.deps(), &request).unwrap();

        assert_eq!(world.process.killed(), vec!["s".to_string()]);
        assert_eq!(world.process.spawned()[0].permission_mode.as_deref(), Some("acceptEdits"));
        assert_eq!(world.settings.load().permission_mode.as_deref(), Some("acceptEdits"));
    }

    #[test]
    fn permission_suggestions_are_only_sent_when_allowing() {
        let world = World::new();
        respond_permission(&world.process, "s", "r1", true, Some("[rule]")).unwrap();
        respond_permission(&world.process, "s", "r2", false, Some("[rule]")).unwrap();
        assert_eq!(
            world.process.permission_responses(),
            vec![("r1".to_string(), true, Some("[rule]".to_string())), ("r2".to_string(), false, None)]
        );
    }

    #[test]
    fn first_message_of_a_session_without_log_spawns_new() {
        let world = World::new();
        send_message(&world.deps(), &message("opus", None)).unwrap();

        let spawned = world.process.spawned();
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0].start, SessionStart::New);
        assert_eq!(spawned[0].model, "opus");
        assert_eq!(spawned[0].cwd, "/p");
        assert_eq!(world.process.written(), vec![("s".to_string(), "안녕".to_string())]);
        assert_eq!(world.settings.load().selected_model, "opus");
    }

    #[test]
    fn session_with_log_resumes() {
        let world = World::new();
        world.sessions.add(meta("s", "t", false));

        send_message(&world.deps(), &message("opus", None)).unwrap();

        let spawned = world.process.spawned();
        assert_eq!((spawned[0].start.clone(), spawned[0].cwd.as_str()), (SessionStart::Resume, "/p"));
    }

    #[test]
    fn alive_process_with_same_model_is_reused() {
        let world = World::new();
        world.process.set_status_with_model("s", false, "opus");

        send_message(&world.deps(), &message("opus", None)).unwrap();

        assert!(world.process.spawned().is_empty());
        assert!(world.process.killed().is_empty());
        assert_eq!(world.process.written().len(), 1);
    }

    #[test]
    fn changing_model_restarts_the_process_with_resume() {
        let world = World::new();
        world.process.set_status_with_model("s", false, "opus");
        world.sessions.add(meta("s", "t", false));

        send_message(&world.deps(), &message("haiku", None)).unwrap();

        assert_eq!(world.process.killed(), vec!["s".to_string()]);
        let spawned = world.process.spawned();
        assert_eq!((spawned[0].start.clone(), spawned[0].model.as_str()), (SessionStart::Resume, "haiku"));
    }

    #[test]
    fn busy_session_appends_to_the_running_turn_without_restarting() {
        let world = World::new();
        world.process.set_status_with_model("s", true, "opus");
        let request = SendMessage { permission_mode: Some("plan"), ..message("haiku", None) };
        send_message(&world.deps(), &request).unwrap();
        assert!(world.process.killed().is_empty());
        assert!(world.process.spawned().is_empty());
        assert_eq!(world.process.written(), vec![("s".to_string(), "안녕".to_string())]);
    }

    #[test]
    fn blank_question_answers_are_dropped() {
        let world = World::new();
        let answers = [("색?".to_string(), " 파랑 ".to_string()), ("크기?".to_string(), "  ".to_string())];
        answer_question(&world.process, "s", "r1", "{}", &answers).unwrap();
        assert_eq!(
            world.process.question_answers(),
            vec![("r1".to_string(), vec![("색?".to_string(), "파랑".to_string())])]
        );
    }

    #[test]
    fn requires_a_connected_binary() {
        let world = World::new();
        world.settings.update(&mut |s| s.claude_path = None).unwrap();
        let result = send_message(&world.deps(), &message("opus", None));
        assert!(matches!(result, Err(AppError::NotConnected(_))));
    }

    #[test]
    fn attached_file_is_mentioned_relative_to_the_project() {
        let world = World::new();
        let body = send_message(&world.deps(), &message("opus", Some("/p/src/a.rs"))).unwrap();
        assert_eq!(body, "@src/a.rs\n\n안녕");

        let world = World::new();
        let body = send_message(&world.deps(), &message("opus", Some("/p/my docs/a.md"))).unwrap();
        assert_eq!(body, "@\"my docs/a.md\"\n\n안녕");
    }

    #[test]
    fn attached_file_outside_project_is_rejected_before_spawning() {
        let world = World::new();
        let result = send_message(&world.deps(), &message("opus", Some("/secret/a.txt")));
        assert!(matches!(result, Err(AppError::OutsideProject(_))));
        assert!(world.process.spawned().is_empty());
    }

    #[test]
    fn closing_a_session_kills_it_and_removes_it_from_the_active_list() {
        let world = World::new();
        close_session(&world.settings, &world.process, "/p", "s").unwrap();
        assert_eq!(world.process.killed(), vec!["s".to_string()]);
        assert!(world.settings.load().projects[0].active_sessions.is_empty());
    }

    #[test]
    fn activating_twice_does_not_duplicate() {
        let settings = FakeSettings::with_project("/p", &[]);
        resume_session(&settings, "/p", "x").unwrap();
        resume_session(&settings, "/p", "x").unwrap();
        assert_eq!(settings.load().projects[0].active_sessions, vec!["x".to_string()]);
    }
}
