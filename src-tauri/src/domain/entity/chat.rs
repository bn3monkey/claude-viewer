use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RateWindow {
    /// `five_hour`, `seven_day`, 모델별 창 등 CLI 가 주는 이름 그대로.
    pub name: String,
    /// 0.0 ~ 1.0
    pub utilization: f64,
    /// epoch 초
    pub resets_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum ChatEvent {
    Init {
        session_id: String,
        model: String,
    },
    User {
        text: String,
    },
    AssistantText {
        text: String,
    },
    Thinking {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        summary: String,
    },
    ToolResult {
        tool_use_id: String,
        text: String,
        is_error: bool,
    },
    /// CLI 가 도구 사용 허락을 묻는다. 응답할 때까지 턴이 멈춘다.
    PermissionRequest {
        request_id: String,
        tool_use_id: String,
        tool_name: String,
        description: String,
        /// 도구 입력 JSON (보여주기용)
        input: String,
        /// CLI 가 제안한 영구 허용 규칙 JSON 배열. "항상 허용" 에 그대로 돌려준다.
        suggestions: String,
    },
    /// assistant 메시지마다 붙는 토큰 사용량. 입력 세 값의 합이 현재 컨텍스트 크기다.
    Usage {
        input_tokens: f64,
        cache_read_tokens: f64,
        cache_creation_tokens: f64,
        output_tokens: f64,
    },
    RateLimit {
        windows: Vec<RateWindow>,
    },
    /// `/compact` 로 대화가 요약됐다.
    Compacted {
        summary: String,
    },
    Result {
        is_error: bool,
        text: String,
        duration_ms: f64,
        cost_usd: f64,
        /// 이번 턴에 쓴 모델 중 가장 큰 컨텍스트 창. 모르면 0.
        context_window: f64,
    },
    Exited {
        code: Option<i32>,
        aborted: bool,
    },
    Error {
        message: String,
    },
}
