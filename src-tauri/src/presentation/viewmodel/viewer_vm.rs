use tauri::http;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};

use crate::domain::entity::{AppError, DefinitionHit, LinkTarget, SavedTab, TextContent, ViewerMode};
use crate::domain::usecase::viewer;
use crate::presentation::state::AppState;

#[tauri::command(async)]
pub fn viewer_resolve_mode(path: String) -> ViewerMode {
    viewer::resolve_viewer_mode(&path)
}

/// 대화·마크다운 속 링크. `base_dir` 은 상대 경로의 기준 폴더 (세션이면 프로젝트 루트).
#[tauri::command(async)]
pub fn viewer_resolve_link(state: State<'_, AppState>, base_dir: String, href: String) -> Result<LinkTarget, AppError> {
    viewer::resolve_link(&*state.fs, &*state.settings, &base_dir, &href)
}

#[tauri::command(async)]
pub fn viewer_save_tabs(state: State<'_, AppState>, tabs: Vec<SavedTab>, active: Option<usize>) -> Result<(), AppError> {
    viewer::save_tabs(&*state.settings, &tabs, active)
}

/// 앱 시작 때 되살릴 탭과 활성 탭 위치.
#[tauri::command(async)]
pub fn viewer_restore_tabs(state: State<'_, AppState>) -> (Vec<SavedTab>, Option<usize>) {
    viewer::restore_tabs(&*state.fs, &*state.settings)
}

#[tauri::command(async)]
pub fn viewer_read_text(state: State<'_, AppState>, path: String) -> Result<TextContent, AppError> {
    viewer::read_text(&*state.fs, &*state.settings, &path)
}

/// PDF / DOCX 원본 바이트. JSON 배열이 아닌 ArrayBuffer 로 보낸다.
#[tauri::command(async)]
pub fn viewer_read_bytes(state: State<'_, AppState>, path: String) -> Result<Response, AppError> {
    viewer::read_bytes(&*state.fs, &*state.settings, &path).map(Response::new)
}

/// 뷰어에서 고친 텍스트를 저장한다.
#[tauri::command(async)]
pub fn viewer_write_text(state: State<'_, AppState>, path: String, text: String) -> Result<(), AppError> {
    viewer::write_text(&*state.fs, &*state.settings, &path, &text)
}

/// F12: `from` 이 속한 프로젝트에서 `name` 의 선언·정의 후보.
#[tauri::command(async)]
pub fn viewer_find_definitions(
    state: State<'_, AppState>,
    from: String,
    name: String,
) -> Result<Vec<DefinitionHit>, AppError> {
    viewer::find_definitions(&*state.fs, &*state.settings, &from, &name)
}

#[tauri::command(async)]
pub fn viewer_open_external(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    viewer::open_external(&*state.fs, &*state.settings, &*state.shell, &path)
}

/// 대화·마크다운 속 http(s) 링크를 기본 브라우저로 연다.
#[tauri::command(async)]
pub fn viewer_open_url(state: State<'_, AppState>, url: String) -> Result<(), AppError> {
    viewer::open_web_link(&*state.shell, &url)
}

/// `htmlview` 스킴. 프로젝트 안의 HTML 과 그 파일이 참조하는 css·이미지를 그대로 서빙한다.
/// URL 경로가 파일 경로 구조를 유지하므로 iframe 안의 상대 경로가 같은 폴더로 풀린다.
/// 앱 CSP 를 물려받지 않도록 srcdoc 대신 이 스킴을 iframe src 로 쓴다.
pub fn serve_html(app: &AppHandle, request: http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
    let path = file_path_of(request.uri().path());
    let state = app.state::<AppState>();
    match viewer::read_bytes(&*state.fs, &*state.settings, &path) {
        Ok(bytes) => http::Response::builder()
            .status(200)
            .header("Content-Type", mime_of(&path))
            .header("Cache-Control", "no-store")
            .body(bytes)
            .unwrap(),
        Err(e) => http::Response::builder()
            .status(if matches!(e, AppError::OutsideProject(_)) { 403 } else { 404 })
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(e.to_string().into_bytes())
            .unwrap(),
    }
}

/// `/C:/a/b.html` → `C:/a/b.html`, `/Users/a/b.html` 은 그대로.
fn file_path_of(url_path: &str) -> String {
    let decoded = viewer::percent_decode(url_path);
    let is_windows_drive = decoded.as_bytes().get(2) == Some(&b':') && decoded.starts_with('/');
    if is_windows_drive { decoded[1..].to_string() } else { decoded }
}

fn mime_of(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "mp4" => "video/mp4",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_paths_map_to_file_paths() {
        assert_eq!(file_path_of("/C:/Users/me/docs/a%20b.html"), "C:/Users/me/docs/a b.html");
        assert_eq!(file_path_of("/Users/me/%ED%95%9C%EA%B8%80.html"), "/Users/me/한글.html");
        assert_eq!(mime_of("x/style.CSS"), "text/css; charset=utf-8");
    }
}
