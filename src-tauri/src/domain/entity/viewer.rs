use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ViewerMode {
    Source,
    Markdown,
    /// 앱 안의 sandbox iframe 으로 렌더링
    Html,
    Pdf,
    Docx,
    Image,
    Video,
    /// git 변경 내역. 프론트가 브랜치 정보를 탭에 따로 둔다.
    Diff,
    /// 파일을 실행 중인 터미널. 프론트가 터미널 세션을 탭에 따로 둔다.
    Terminal,
}

/// 뷰어 탭은 (path, mode) 쌍으로 식별한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ViewerRequest {
    pub path: String,
    pub mode: ViewerMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextContent {
    pub text: String,
    /// 크기 제한으로 앞부분만 읽었으면 true.
    pub truncated: bool,
    /// NUL 바이트가 있어 텍스트로 보여줄 수 없으면 true.
    pub binary: bool,
}

/// F12 로 찾은 선언·정의 후보. `line` 은 1부터, `text` 는 그 줄의 내용(앞뒤 공백 제거).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DefinitionHit {
    pub path: String,
    pub line: u32,
    pub text: String,
}

/// 대화·문서 속 링크가 가리키는 파일. `line` 은 `#L42`, `:42` 같은 줄 표시 (1부터).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkTarget {
    pub request: ViewerRequest,
    pub line: Option<u32>,
}
