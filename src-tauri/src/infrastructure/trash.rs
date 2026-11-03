use std::path::Path;

/// 영구 삭제 대신 휴지통으로 옮긴다.
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| e.to_string())
}
