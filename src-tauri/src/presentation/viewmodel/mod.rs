//! 프론트엔드와의 유일한 접점. 입력을 받아 usecase 를 부르고 결과나 이벤트를 돌려줄 뿐 로직은 없다.

pub mod connection_vm;
pub mod git_vm;
pub mod project_vm;
pub mod session_vm;
pub mod terminal_vm;
pub mod tree_vm;
pub mod viewer_vm;

pub const EVENT_PROJECT_CHANGED: &str = "project://changed";
pub const EVENT_TREE_CHANGED: &str = "tree://changed";
pub const EVENT_VIEWER_OPEN: &str = "viewer://open";

pub fn session_event_name(session_id: &str) -> String {
    format!("session://event/{session_id}")
}
