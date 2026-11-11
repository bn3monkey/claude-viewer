//! `~/.claude/projects/<인코딩된 프로젝트 경로>/<session-id>.jsonl` 파서.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::stream_json::{decode_message, strip_injected_context, summarize_tool_input};
use crate::domain::entity::{ChatEvent, SubagentRun};
use crate::domain::repository::SessionLogMeta;

/// 마지막 쓰기 후 이만큼 조용하면 다른 클라이언트의 세션도 끝난 것으로 본다.
const RUNNING_QUIET_LIMIT: Duration = Duration::from_secs(60);
const TITLE_MAX_CHARS: usize = 60;

/// 영숫자가 아닌 모든 문자를 `-` 로 바꾼다. `d:\repo\claude_viewer` → `d--repo-claude-viewer`.
/// 실제 CLI 는 `/ \ :` 뿐 아니라 `_`, `.`, 한글도 바꾼다.
pub fn encode_project_dir(root: &str) -> String {
    root.trim_end_matches(['/', '\\'])
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// 폴더 선택기는 `D:\`, CLI 는 `d:\` 처럼 드라이브 문자 대소문자가 다를 수 있어 대소문자를 무시하고 찾는다.
/// 대소문자를 구분하는 파일 시스템이면 둘 다 있을 수 있으므로 모두 돌려준다.
pub fn session_dirs(projects_dir: &Path, root: &str) -> Vec<PathBuf> {
    let encoded = encode_project_dir(root);
    let Ok(entries) = std::fs::read_dir(projects_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().eq_ignore_ascii_case(&encoded))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

pub fn session_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "jsonl"))
        .collect()
}

/// 큰 로그도 빠르게 훑도록 필요한 type 이 있는 줄만 JSON 으로 파싱한다.
pub fn read_meta(path: &Path) -> Option<SessionLogMeta> {
    let id = path.file_stem()?.to_string_lossy().into_owned();
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let reader = BufReader::new(File::open(path).ok()?);

    let mut ai_title = None;
    let mut summary = None;
    let mut first_user_text: Option<String> = None;
    let mut message_count = 0u32;
    let mut turn_open = false;

    for line in reader.lines().map_while(Result::ok) {
        let interesting = ["\"type\":\"user\"", "\"type\":\"assistant\"", "\"type\":\"ai-title\"", "\"type\":\"summary\""]
            .iter()
            .any(|needle| line.contains(needle));
        if !interesting {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        match value["type"].as_str() {
            Some("ai-title") => ai_title = value["aiTitle"].as_str().map(str::to_string),
            Some("summary") => summary = value["summary"].as_str().map(str::to_string),
            Some(kind @ ("user" | "assistant")) => {
                if value["isSidechain"] == true {
                    continue;
                }
                if kind == "user" {
                    if value["isMeta"] != true {
                        if let Some(text) = user_text(&value) {
                            message_count += 1;
                            first_user_text.get_or_insert(text);
                        }
                    }
                    turn_open = true;
                } else {
                    // 도구 호출 중이면 tool_use, 응답이 끝나면 end_turn 등이 기록된다.
                    turn_open = matches!(value["message"]["stop_reason"].as_str(), None | Some("tool_use"));
                }
            }
            _ => {}
        }
    }

    let recently_written = SystemTime::now()
        .duration_since(modified)
        .map_or(true, |elapsed| elapsed < RUNNING_QUIET_LIMIT);
    let title = ai_title.or(summary).or(first_user_text).map(|t| truncate_title(&t));
    Some(SessionLogMeta {
        id,
        title,
        message_count,
        updated_at: modified.duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64),
        looks_running: turn_open && recently_written,
    })
}

/// 대화 기록. 서브에이전트(sidechain)와 메타 메시지는 뺀다.
pub fn read_events(path: &Path) -> std::io::Result<Vec<ChatEvent>> {
    read_conversation(path, false)
}

/// 서브에이전트 로그의 대화. 줄마다 `isSidechain: true` 가 붙어 있다.
pub fn read_subagent_events(path: &Path) -> std::io::Result<Vec<ChatEvent>> {
    read_conversation(path, true)
}

fn read_conversation(path: &Path, sidechain: bool) -> std::io::Result<Vec<ChatEvent>> {
    let reader = BufReader::new(File::open(path)?);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !(line.contains("\"type\":\"user\"") || line.contains("\"type\":\"assistant\"")) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        let is_conversation = matches!(value["type"].as_str(), Some("user" | "assistant"));
        if is_conversation && (value["isSidechain"] == true) == sidechain && value["isMeta"] != true {
            events.extend(decode_message(&value));
        }
    }
    Ok(events)
}

/// `<세션 폴더>/<session-id>/subagents/agent-<id>.jsonl`
pub fn subagent_file(session_dir: &Path, session_id: &str, agent_id: &str) -> PathBuf {
    session_dir.join(session_id).join("subagents").join(format!("agent-{agent_id}.jsonl"))
}

pub fn subagent_files(session_dir: &Path, session_id: &str) -> Vec<PathBuf> {
    session_files(&session_dir.join(session_id).join("subagents"))
        .into_iter()
        .filter(|path| path.file_stem().map_or(false, |stem| stem.to_string_lossy().starts_with("agent-")))
        .collect()
}

/// 서브에이전트 로그와 옆의 `.meta.json` 으로 진행 상황을 만든다.
pub fn read_subagent(path: &Path) -> Option<SubagentRun> {
    let agent_id = path.file_stem()?.to_string_lossy().strip_prefix("agent-")?.to_string();
    let file_meta = std::fs::metadata(path).ok()?;
    let modified = file_meta.modified().ok()?;
    let started = file_meta.created().unwrap_or(modified);
    let meta: Value = std::fs::read_to_string(path.with_extension("meta.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);

    let mut tool_uses = 0u32;
    // 한 응답이 블록마다 한 줄씩 같은 message.id 와 usage 로 기록되므로 id 별로 한 번만 센다.
    let mut usage_by_message: Vec<(String, f64)> = Vec::new();
    let mut last_activity = None;
    let mut finished = false;
    let reader = BufReader::new(File::open(path).ok()?);
    for line in reader.lines().map_while(Result::ok) {
        if !(line.contains("\"type\":\"user\"") || line.contains("\"type\":\"assistant\"")) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        let message = &value["message"];
        match value["type"].as_str() {
            Some("assistant") => {
                let blocks = message["content"].as_array().map(Vec::as_slice).unwrap_or_default();
                for block in blocks.iter().filter(|b| b["type"] == "tool_use") {
                    tool_uses += 1;
                    let name = block["name"].as_str().unwrap_or_default();
                    let summary = summarize_tool_input(&block["input"]);
                    last_activity = Some(if summary.is_empty() { name.to_string() } else { format!("{name} · {summary}") });
                }
                // 도구 없이 글로 끝난 응답이 최종 보고다.
                finished = blocks.iter().any(|b| b["type"] == "text")
                    && !blocks.iter().any(|b| b["type"] == "tool_use")
                    && message["stop_reason"].as_str() != Some("tool_use");
                if let Some(output) = message["usage"]["output_tokens"].as_f64() {
                    let id = message["id"].as_str().unwrap_or_default().to_string();
                    match usage_by_message.iter_mut().find(|(seen, _)| !id.is_empty() && *seen == id) {
                        Some(entry) => entry.1 = output,
                        None => usage_by_message.push((id, output)),
                    }
                }
            }
            Some("user") if value["isMeta"] != true => {
                // 도구 결과가 오면 다시 일하는 중이다.
                if message["content"].as_array().map_or(false, |c| c.iter().any(|b| b["type"] == "tool_result")) {
                    finished = false;
                }
            }
            _ => {}
        }
    }

    let recently_written = SystemTime::now()
        .duration_since(modified)
        .map_or(true, |elapsed| elapsed < RUNNING_QUIET_LIMIT);
    let millis = |t: SystemTime| t.duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64);
    let text = |key: &str| meta[key].as_str().unwrap_or_default().to_string();
    Some(SubagentRun {
        agent_id,
        tool_use_id: text("toolUseId"),
        agent_type: meta["agentType"].as_str().unwrap_or("agent").to_string(),
        description: text("description"),
        model: meta["model"].as_str().map(str::to_string),
        background: meta["requestShape"] == "background",
        tool_uses,
        output_tokens: usage_by_message.iter().map(|(_, n)| n).sum(),
        last_activity,
        started_at: millis(started),
        updated_at: millis(modified),
        finished,
        running: !finished && recently_written,
    })
}

fn user_text(value: &Value) -> Option<String> {
    let content = &value["message"]["content"];
    let raw = match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };
    let text = strip_injected_context(&raw);
    (!text.is_empty()).then_some(text)
}

/// 첫 줄이 이 앱이 붙인 `@파일` 첨부뿐이면 건너뛰고 실제 요청 문장을 제목으로 쓴다.
fn truncate_title(title: &str) -> String {
    let is_mention_only = |line: &str| {
        (line.starts_with("@\"") && line.ends_with('"') && line.len() > 3)
            || (line.starts_with('@') && !line.contains(char::is_whitespace))
    };
    let line = title
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !is_mention_only(l))
        .or_else(|| title.lines().map(str::trim).find(|l| !l.is_empty()))
        .unwrap_or_default();
    match line.char_indices().nth(TITLE_MAX_CHARS) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn encodes_paths_like_the_cli() {
        assert_eq!(encode_project_dir(r"d:\repo\claude_viewer"), "d--repo-claude-viewer");
        assert_eq!(encode_project_dir(r"C:\Users\hansono_bn3monkey\"), "C--Users-hansono-bn3monkey");
        assert_eq!(encode_project_dir(r"\\192.168.0.200\a"), "--192-168-0-200-a");
        assert_eq!(encode_project_dir("/Users/me/프로젝트"), "-Users-me-----");
    }

    #[test]
    fn finds_session_dir_regardless_of_drive_letter_case() {
        let projects = tempfile::tempdir().unwrap();
        for name in ["d--repo-x", "d--repo-y"] {
            std::fs::create_dir(projects.path().join(name)).unwrap();
        }
        let found: Vec<_> = session_dirs(projects.path(), r"D:\repo\x")
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(found, vec!["d--repo-x"]);
    }

    fn write_log(lines: &[&str]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.jsonl");
        let mut file = File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        (dir, path)
    }

    const USER: &str = r#"{"type":"user","cwd":"d:\\repo\\x","message":{"role":"user","content":[{"type":"text","text":"<ide_opened_file>a</ide_opened_file>"},{"type":"text","text":"요구사항 정리해줘\n자세히"}]}}"#;
    const TOOL_USE: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t","name":"Read","input":{}}],"stop_reason":"tool_use"}}"#;
    const TOOL_RESULT: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t","content":"x"}]}}"#;
    const END: &str = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"끝"}],"stop_reason":"end_turn"}}"#;

    #[test]
    fn title_prefers_ai_title_then_first_user_message() {
        let (_dir, path) = write_log(&[USER, END]);
        let meta = read_meta(&path).unwrap();
        assert_eq!(meta.id, "abc");
        assert_eq!(meta.title.as_deref(), Some("요구사항 정리해줘"));
        assert_eq!(meta.message_count, 1, "tool_result 는 사용자 메시지로 세지 않는다");

        let (_dir, path) = write_log(&[
            USER,
            r#"{"type":"ai-title","aiTitle":"옛 제목","sessionId":"abc"}"#,
            r#"{"type":"ai-title","aiTitle":"요구사항 문서 정리","sessionId":"abc"}"#,
        ]);
        assert_eq!(read_meta(&path).unwrap().title.as_deref(), Some("요구사항 문서 정리"));
    }

    #[test]
    fn title_skips_the_attached_file_mention_line() {
        assert_eq!(truncate_title("@notes.txt\n\n이 파일 요약해줘"), "이 파일 요약해줘");
        assert_eq!(truncate_title("@\"my docs/a.md\"\n\n요약"), "요약");
        assert_eq!(truncate_title("@a.md"), "@a.md");
    }

    #[test]
    fn recently_written_open_turn_looks_running() {
        let (_dir, path) = write_log(&[USER, TOOL_USE, TOOL_RESULT]);
        assert!(read_meta(&path).unwrap().looks_running);

        let (_dir, path) = write_log(&[USER, TOOL_USE, TOOL_RESULT, END]);
        assert!(!read_meta(&path).unwrap().looks_running);
    }

    #[test]
    fn subagent_progress_comes_from_its_log_and_meta() {
        let dir = tempfile::tempdir().unwrap();
        let sub_dir = dir.path().join("s1").join("subagents");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(
            sub_dir.join("agent-a1.meta.json"),
            r#"{"agentType":"Explore","description":"로그 형식 확인","toolUseId":"toolu_1","requestShape":"background","model":"haiku"}"#,
        )
        .unwrap();
        let prompt = r#"{"type":"user","isSidechain":true,"message":{"role":"user","content":"파일을 봐"}}"#;
        let think = r#"{"type":"assistant","isSidechain":true,"message":{"id":"m1","content":[{"type":"thinking","thinking":"음"}],"usage":{"output_tokens":30}}}"#;
        let call = r#"{"type":"assistant","isSidechain":true,"message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"sleep 20"}}],"stop_reason":"tool_use","usage":{"output_tokens":30}}}"#;
        let result = r#"{"type":"user","isSidechain":true,"message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]}}"#;
        let report = r#"{"type":"assistant","isSidechain":true,"message":{"id":"m2","content":[{"type":"text","text":"보고"}],"stop_reason":"end_turn","usage":{"output_tokens":5}}}"#;
        let path = sub_dir.join("agent-a1.jsonl");
        let write = |lines: &[&str]| std::fs::write(&path, lines.join("\n")).unwrap();

        write(&[prompt, think, call, result]);
        assert_eq!(subagent_files(dir.path(), "s1"), vec![path.clone()]);
        let run = read_subagent(&path).unwrap();
        assert_eq!((run.agent_id.as_str(), run.tool_use_id.as_str(), run.agent_type.as_str()), ("a1", "toolu_1", "Explore"));
        assert!(run.background && run.running && !run.finished);
        assert_eq!((run.tool_uses, run.output_tokens), (1, 30.0), "같은 응답의 usage 는 한 번만 센다");
        assert_eq!(run.last_activity.as_deref(), Some("Bash · sleep 20"));

        write(&[prompt, think, call, result, report]);
        let run = read_subagent(&path).unwrap();
        assert!(run.finished && !run.running);
        assert_eq!(run.output_tokens, 35.0);

        let events = read_subagent_events(&path).unwrap();
        assert_eq!(events.first(), Some(&ChatEvent::User { text: "파일을 봐".into() }));
        assert!(events.contains(&ChatEvent::AssistantText { text: "보고".into() }));
    }

    #[test]
    fn events_skip_sidechains_and_injected_context() {
        let sidechain = r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"서브"}],"stop_reason":"end_turn"}}"#;
        let (_dir, path) = write_log(&[USER, sidechain, r#"{"type":"queue-operation"}"#, END]);
        assert_eq!(
            read_events(&path).unwrap(),
            vec![
                ChatEvent::User { text: "요구사항 정리해줘\n자세히".into() },
                ChatEvent::AssistantText { text: "끝".into() },
            ]
        );
    }
}
