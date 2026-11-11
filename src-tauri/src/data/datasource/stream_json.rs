//! claude CLI `--input-format stream-json` / `--output-format stream-json` 인코더·디코더.
//! 세션 로그(.jsonl)의 user / assistant 줄도 같은 message 형식이라 함께 쓴다.

use serde_json::{json, Value};

use crate::domain::entity::{ChatEvent, Question, QuestionOption, RateWindow};

pub fn encode_user_message(text: &str) -> String {
    json!({
        "type": "user",
        "message": { "role": "user", "content": [{ "type": "text", "text": text }] }
    })
    .to_string()
}

/// `--permission-prompt-tool stdio` 의 control_request 에 대한 응답 한 줄.
pub fn encode_permission_response(
    request_id: &str,
    allow: bool,
    updated_permissions: Option<&str>,
) -> String {
    let mut response = if allow {
        json!({ "behavior": "allow" })
    } else {
        json!({ "behavior": "deny", "message": "사용자가 Claude Viewer 에서 거부했습니다." })
    };
    if let Some(rules) = updated_permissions.and_then(|r| serde_json::from_str::<Value>(r).ok()) {
        if rules.as_array().map_or(false, |a| !a.is_empty()) {
            response["updatedPermissions"] = rules;
        }
    }
    json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": response }
    })
    .to_string()
}

/// AskUserQuestion 응답. 원래 입력에 `answers` (질문 → 답) 를 붙여 허용하면 CLI 가 도구 결과로 넘긴다.
/// 여러 개를 고른 답은 `, ` 로 이어 붙인다.
pub fn encode_question_answer(request_id: &str, input: &str, answers: &[(String, String)]) -> String {
    let mut updated = serde_json::from_str::<Value>(input).unwrap_or_else(|_| json!({}));
    updated["answers"] = answers.iter().map(|(q, a)| (q.clone(), Value::String(a.clone()))).collect();
    json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": { "behavior": "allow", "updatedInput": updated }
        }
    })
    .to_string()
}

/// 앱이 처리하지 못하는 control_request 면 (request_id, subtype). 답하지 않으면 CLI 가 영원히 기다린다.
pub fn unsupported_control_request(line: &str) -> Option<(String, String)> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    if value["type"] != "control_request" || value["request"]["subtype"] == "can_use_tool" {
        return None;
    }
    Some((str_of(&value["request_id"]), str_of(&value["request"]["subtype"])))
}

pub fn encode_control_error(request_id: &str, message: &str) -> String {
    json!({
        "type": "control_response",
        "response": { "subtype": "error", "request_id": request_id, "error": message }
    })
    .to_string()
}

/// stdout 한 줄을 대화 이벤트로 바꾼다. 화면에 필요 없는 줄은 빈 Vec.
pub fn decode_line(line: &str) -> Vec<ChatEvent> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    match value["type"].as_str() {
        Some("system") if value["subtype"] == "init" => vec![ChatEvent::Init {
            session_id: str_of(&value["session_id"]),
            model: str_of(&value["model"]),
        }],
        Some("assistant") | Some("user") => decode_message(&value),
        Some("control_request") if value["request"]["subtype"] == "can_use_tool" => {
            let request = &value["request"];
            if request["tool_name"] == "AskUserQuestion" {
                if let Some(questions) = parse_questions(&request["input"]) {
                    return vec![ChatEvent::Question {
                        request_id: str_of(&value["request_id"]),
                        tool_use_id: str_of(&request["tool_use_id"]),
                        questions,
                        input: request["input"].to_string(),
                    }];
                }
            }
            vec![ChatEvent::PermissionRequest {
                request_id: str_of(&value["request_id"]),
                tool_use_id: str_of(&request["tool_use_id"]),
                tool_name: str_of(&request["tool_name"]),
                description: str_of(&request["description"]),
                input: serde_json::to_string_pretty(&request["input"]).unwrap_or_default(),
                suggestions: request["permission_suggestions"].to_string(),
            }]
        }
        Some("rate_limit_event") => {
            let Some(windows) = value["rate_limit_info"]["unifiedWindows"].as_object() else {
                return Vec::new();
            };
            vec![ChatEvent::RateLimit {
                windows: windows
                    .iter()
                    .map(|(name, w)| RateWindow {
                        name: name.clone(),
                        utilization: w["utilization"].as_f64().unwrap_or(0.0),
                        resets_at: w["resetsAt"].as_f64().unwrap_or(0.0),
                    })
                    .collect(),
            }]
        }
        Some("result") => vec![ChatEvent::Result {
            is_error: value["is_error"].as_bool().unwrap_or(false),
            text: str_of(&value["result"]),
            duration_ms: value["duration_ms"].as_f64().unwrap_or(0.0),
            cost_usd: value["total_cost_usd"].as_f64().unwrap_or(0.0),
            context_window: value["modelUsage"]
                .as_object()
                .map(|models| models.values().filter_map(|m| m["contextWindow"].as_f64()).fold(0.0, f64::max))
                .unwrap_or(0.0),
        }],
        _ => Vec::new(),
    }
}

/// 형식이 다르면 None — 일반 권한 요청으로 보여준다.
fn parse_questions(input: &Value) -> Option<Vec<Question>> {
    input["questions"]
        .as_array()?
        .iter()
        .map(|q| {
            Some(Question {
                question: q["question"].as_str()?.to_string(),
                header: str_of(&q["header"]),
                multi_select: q["multiSelect"].as_bool().unwrap_or(false),
                options: q["options"]
                    .as_array()?
                    .iter()
                    .map(|o| {
                        Some(QuestionOption {
                            label: o["label"].as_str()?.to_string(),
                            description: str_of(&o["description"]),
                        })
                    })
                    .collect::<Option<_>>()?,
            })
        })
        .collect()
}

const COMPACT_PREFIX: &str = "This session is being continued from a previous conversation";

/// `{"type":"user"|"assistant","message":{"content":...}}` 의 블록들을 이벤트로 바꾼다.
pub fn decode_message(value: &Value) -> Vec<ChatEvent> {
    let is_user = value["type"] == "user";
    let content = &value["message"]["content"];
    if let Some(text) = content.as_str() {
        if is_user && text.starts_with(COMPACT_PREFIX) {
            let summary = text.split_once("Summary:").map_or(text, |(_, s)| s).trim().to_string();
            return vec![ChatEvent::Compacted { summary }];
        }
        return text_event(is_user, text).into_iter().collect();
    }
    let Some(blocks) = content.as_array() else {
        return Vec::new();
    };
    let usage = (!is_user)
        .then(|| value["message"]["usage"].as_object())
        .flatten()
        .map(|u| ChatEvent::Usage {
            input_tokens: u.get("input_tokens").and_then(Value::as_f64).unwrap_or(0.0),
            cache_read_tokens: u.get("cache_read_input_tokens").and_then(Value::as_f64).unwrap_or(0.0),
            cache_creation_tokens: u.get("cache_creation_input_tokens").and_then(Value::as_f64).unwrap_or(0.0),
            output_tokens: u.get("output_tokens").and_then(Value::as_f64).unwrap_or(0.0),
        });
    blocks
        .iter()
        .filter_map(|block| match block["type"].as_str()? {
            "text" => text_event(is_user, block["text"].as_str()?),
            "thinking" => {
                let text = block["thinking"].as_str()?.trim();
                (!text.is_empty()).then(|| ChatEvent::Thinking { text: text.to_string() })
            }
            "tool_use" => Some(ChatEvent::ToolUse {
                id: str_of(&block["id"]),
                name: str_of(&block["name"]),
                summary: summarize_tool_input(&block["input"]),
            }),
            "tool_result" => Some(ChatEvent::ToolResult {
                tool_use_id: str_of(&block["tool_use_id"]),
                text: truncate(&tool_result_text(&block["content"]), 4000),
                is_error: block["is_error"].as_bool().unwrap_or(false),
            }),
            _ => None,
        })
        .chain(usage)
        .collect()
}

fn text_event(is_user: bool, text: &str) -> Option<ChatEvent> {
    if is_user {
        let text = strip_injected_context(text);
        (!text.is_empty()).then_some(ChatEvent::User { text })
    } else {
        let text = text.trim();
        (!text.is_empty()).then(|| ChatEvent::AssistantText { text: text.to_string() })
    }
}

/// 클라이언트가 사용자 메시지에 덧붙이는 태그 블록(IDE 컨텍스트, 시스템 알림 등)을 뺀다.
pub fn strip_injected_context(text: &str) -> String {
    const TAGS: [&str; 8] = [
        "ide_opened_file",
        "ide_selection",
        "system-reminder",
        "command-name",
        "command-message",
        "command-args",
        "local-command-stdout",
        "local-command-caveat",
    ];
    let mut out = text.to_string();
    for tag in TAGS {
        let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
        while let Some(start) = out.find(&open) {
            let Some(end) = out[start..].find(&close) else { break };
            out.replace_range(start..start + end + close.len(), "");
        }
    }
    out.trim().to_string()
}

/// 도구 호출을 한 줄로 요약한다. 파일 경로나 명령처럼 사람이 알아볼 값을 우선한다.
pub fn summarize_tool_input(input: &Value) -> String {
    const KEYS: [&str; 8] =
        ["file_path", "command", "pattern", "path", "url", "query", "description", "prompt"];
    let summary = KEYS
        .iter()
        .find_map(|key| input[key].as_str())
        .or_else(|| input["questions"][0]["question"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| if input.is_null() { String::new() } else { input.to_string() });
    truncate(summary.lines().next().unwrap_or_default(), 200)
}

fn tool_result_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}

fn str_of(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_user_message_line() {
        let line = encode_user_message("안녕\n\"따옴표\"");
        assert!(!line.contains('\n'), "stdin 에는 한 줄로 써야 한다");
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["type"], "user");
        assert_eq!(value["message"]["content"][0]["text"], "안녕\n\"따옴표\"");
    }

    // 2.1.314 에서 실제로 받은 줄을 줄인 것
    #[test]
    fn decodes_init_assistant_and_result() {
        let init = r#"{"type":"system","subtype":"init","cwd":"C:\\p","session_id":"598f","tools":[],"model":"claude-haiku-5-5"}"#;
        let assistant = r#"{"type":"assistant","message":{"model":"claude-haiku-5-5","role":"assistant","content":[{"type":"text","text":"pong"}],"stop_reason":null},"session_id":"598f"}"#;
        let result = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":1333,"result":"pong","total_cost_usd":0.0083358,"session_id":"598f","modelUsage":{"claude-haiku-5-5":{"contextWindow":1000000},"claude-opus-5-5":{"contextWindow":200000}}}"#;
        let rate = r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":{"five_hour":{"utilization":0.03,"resetsAt":1793689200},"seven_day":{"utilization":0.18,"resetsAt":1793970000}}}}"#;

        assert_eq!(
            decode_line(init),
            vec![ChatEvent::Init { session_id: "598f".into(), model: "claude-haiku-5-5".into() }]
        );
        assert_eq!(decode_line(assistant), vec![ChatEvent::AssistantText { text: "pong".into() }]);
        assert_eq!(
            decode_line(result),
            vec![ChatEvent::Result {
                is_error: false,
                text: "pong".into(),
                duration_ms: 1333.0,
                cost_usd: 0.0083358,
                context_window: 1_000_000.0
            }]
        );
        assert_eq!(
            decode_line(rate),
            vec![ChatEvent::RateLimit {
                windows: vec![
                    RateWindow { name: "five_hour".into(), utilization: 0.03, resets_at: 1793689200.0 },
                    RateWindow { name: "seven_day".into(), utilization: 0.18, resets_at: 1793970000.0 },
                ]
            }]
        );
        assert!(decode_line(r#"{"type":"rate_limit_event","rate_limit_info":{}}"#).is_empty());
        assert!(decode_line("not json").is_empty());
    }

    #[test]
    fn assistant_usage_becomes_a_usage_event_after_the_blocks() {
        let assistant = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}],"usage":{"input_tokens":2,"cache_creation_input_tokens":41046,"cache_read_input_tokens":10,"output_tokens":4}}}"#;
        assert_eq!(
            decode_line(assistant),
            vec![
                ChatEvent::AssistantText { text: "hi".into() },
                ChatEvent::Usage { input_tokens: 2.0, cache_read_tokens: 10.0, cache_creation_tokens: 41046.0, output_tokens: 4.0 },
            ]
        );
    }

    #[test]
    fn permission_request_round_trip() {
        let request = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Write","input":{"file_path":"a.txt","content":"x"},"description":"a.txt","permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"t1"}}"#;
        let events = decode_line(request);
        let ChatEvent::PermissionRequest { request_id, tool_use_id, tool_name, suggestions, .. } = &events[0] else {
            panic!("{events:?}");
        };
        assert_eq!((request_id.as_str(), tool_use_id.as_str(), tool_name.as_str()), ("r1", "t1", "Write"));

        let allow: Value = serde_json::from_str(&encode_permission_response("r1", true, Some(suggestions))).unwrap();
        assert_eq!(allow["type"], "control_response");
        assert_eq!(allow["response"]["request_id"], "r1");
        assert_eq!(allow["response"]["response"]["behavior"], "allow");
        assert_eq!(allow["response"]["response"]["updatedPermissions"][0]["mode"], "acceptEdits");

        let deny: Value = serde_json::from_str(&encode_permission_response("r1", false, None)).unwrap();
        assert_eq!(deny["response"]["response"]["behavior"], "deny");
        assert!(deny["response"]["response"].get("updatedPermissions").is_none());
    }

    // 2.1.314 에서 실제로 받은 줄
    #[test]
    fn ask_user_question_round_trip() {
        let request = r#"{"type":"control_request","request_id":"9151","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","display_name":"AskUserQuestion","input":{"questions":[{"question":"Which color do you prefer?","header":"Color","options":[{"label":"Red","description":"red"},{"label":"Blue","description":"blue"}],"multiSelect":false}]},"tool_use_id":"t1","requires_user_interaction":true}}"#;
        let events = decode_line(request);
        let ChatEvent::Question { request_id, questions, input, .. } = &events[0] else {
            panic!("{events:?}");
        };
        assert_eq!(request_id, "9151");
        assert_eq!(questions[0].header, "Color");
        assert_eq!(questions[0].options[1].label, "Blue");
        assert!(!questions[0].multi_select);

        let answer = encode_question_answer("9151", input, &[("Which color do you prefer?".into(), "Blue".into())]);
        let answer: Value = serde_json::from_str(&answer).unwrap();
        let response = &answer["response"]["response"];
        assert_eq!(response["behavior"], "allow");
        assert_eq!(response["updatedInput"]["answers"]["Which color do you prefer?"], "Blue");
        assert_eq!(response["updatedInput"]["questions"][0]["header"], "Color");
    }

    #[test]
    fn malformed_questions_fall_back_to_a_permission_request() {
        let request = r#"{"type":"control_request","request_id":"r","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","input":{"questions":"?"}}}"#;
        assert!(matches!(decode_line(request)[0], ChatEvent::PermissionRequest { .. }));
    }

    #[test]
    fn only_non_tool_control_requests_are_unsupported() {
        let tool = r#"{"type":"control_request","request_id":"r","request":{"subtype":"can_use_tool"}}"#;
        let other = r#"{"type":"control_request","request_id":"r2","request":{"subtype":"elicitation"}}"#;
        assert_eq!(unsupported_control_request(tool), None);
        assert_eq!(unsupported_control_request(other), Some(("r2".into(), "elicitation".into())));
        assert!(decode_line(other).is_empty());
        let error: Value = serde_json::from_str(&encode_control_error("r2", "x")).unwrap();
        assert_eq!((error["response"]["subtype"].as_str(), error["response"]["request_id"].as_str()), (Some("error"), Some("r2")));
    }

    #[test]
    fn compact_summary_and_local_command_output_are_recognised() {
        let summary = r#"{"type":"user","message":{"role":"user","content":"This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\nSummary:\n1. Primary Request: echo"}}"#;
        assert_eq!(decode_line(summary), vec![ChatEvent::Compacted { summary: "1. Primary Request: echo".into() }]);
        let compacted = r#"{"type":"user","message":{"role":"user","content":"<local-command-stdout>Compacted </local-command-stdout>"}}"#;
        assert!(decode_line(compacted).is_empty());
    }

    #[test]
    fn decodes_thinking_tool_use_and_tool_result() {
        let assistant = r#"{"type":"assistant","message":{"content":[
            {"type":"thinking","thinking":"고민"},
            {"type":"thinking","thinking":""},
            {"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"C:\\p\\a.md"}}]}}"#;
        let user = r#"{"type":"user","message":{"content":[
            {"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"내용"}],"is_error":true}]}}"#;

        assert_eq!(
            decode_line(assistant),
            vec![
                ChatEvent::Thinking { text: "고민".into() },
                ChatEvent::ToolUse { id: "t1".into(), name: "Read".into(), summary: "C:\\p\\a.md".into() },
            ]
        );
        assert_eq!(
            decode_line(user),
            vec![ChatEvent::ToolResult { tool_use_id: "t1".into(), text: "내용".into(), is_error: true }]
        );
    }

    #[test]
    fn user_text_drops_injected_ide_context() {
        let user = r#"{"type":"user","message":{"content":[
            {"type":"text","text":"<ide_opened_file>The user opened a.md</ide_opened_file>"},
            {"type":"text","text":"이거 보고 구현해줘"}]}}"#;
        assert_eq!(decode_line(user), vec![ChatEvent::User { text: "이거 보고 구현해줘".into() }]);
    }

    #[test]
    fn long_tool_summaries_are_cut_to_one_line() {
        let assistant = r#"{"type":"assistant","message":{"content":[
            {"type":"tool_use","id":"t","name":"Bash","input":{"command":"echo a\necho b"}}]}}"#;
        assert_eq!(
            decode_line(assistant),
            vec![ChatEvent::ToolUse { id: "t".into(), name: "Bash".into(), summary: "echo a".into() }]
        );
        assert_eq!(truncate("가나다라", 2), "가나…");
    }
}
