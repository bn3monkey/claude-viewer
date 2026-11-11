use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 메인 세션이 띄운 서브에이전트 하나. `<session>/subagents/agent-<id>.jsonl` 과 `.meta.json` 에서 읽는다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SubagentRun {
    pub agent_id: String,
    /// 메인 대화에서 이 에이전트를 띄운 `Agent` 도구 호출 id. 모르면 빈 문자열.
    pub tool_use_id: String,
    pub agent_type: String,
    pub description: String,
    pub model: Option<String>,
    pub background: bool,
    /// 지금까지 부른 도구 수
    pub tool_uses: u32,
    pub output_tokens: f64,
    /// 마지막 도구 호출. `Bash · cargo test`
    pub last_activity: Option<String>,
    /// 로그 파일 생성·마지막 수정 시각 (epoch ms)
    pub started_at: f64,
    pub updated_at: f64,
    /// 최종 답을 썼으면 true
    pub finished: bool,
    /// 끝나지 않았고 최근에 기록이 늘었으면 true
    pub running: bool,
}
