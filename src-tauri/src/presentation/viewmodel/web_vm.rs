//! 인터넷 탭. 메인 창 위에 자식 웹뷰를 겹쳐 띄우고, 프론트가 알려 주는 탭 본문 자리로 옮긴다.
//! 원격 페이지라 capability 의 remote 목록에 없으므로 IPC 는 주입되지 않는다.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tauri::webview::{NewWindowFeatures, NewWindowResponse, PageLoadEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, Webview, WebviewBuilder, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::domain::entity::AppError;
use crate::domain::usecase::viewer;

pub const EVENT_WEB_CHANGED: &str = "web://changed";
const MAIN_WINDOW: &str = "main";

/// 탭 본문의 위치와 크기 (CSS px = 논리 px).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct WebBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 주소나 제목이 바뀌었다. title 은 제목 변경 때만 온다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebChanged {
    id: String,
    url: Option<String>,
    title: Option<String>,
    loading: Option<bool>,
}

fn label_of(id: &str) -> String {
    format!("web-{id}")
}

fn parse_url(url: &str) -> Result<Url, AppError> {
    if !viewer::is_web_url(url) {
        return Err(AppError::InvalidPath(format!("http(s) 주소만 열 수 있습니다: {url}")));
    }
    Url::parse(url).map_err(|e| AppError::InvalidPath(format!("잘못된 주소입니다: {e}")))
}

fn webview(app: &AppHandle, id: &str) -> Result<Webview, AppError> {
    app.get_webview(&label_of(id)).ok_or_else(|| AppError::NotFound(format!("인터넷 탭이 없습니다: {id}")))
}

fn tauri_err(e: tauri::Error) -> AppError {
    AppError::Process(e.to_string())
}

fn emit(app: &AppHandle, change: WebChanged) {
    let _ = app.emit_to(MAIN_WINDOW, EVENT_WEB_CHANGED, change);
}

fn place(view: &Webview, bounds: WebBounds) -> Result<(), AppError> {
    view.set_position(LogicalPosition::new(bounds.x, bounds.y)).map_err(tauri_err)?;
    view.set_size(LogicalSize::new(bounds.width.max(1.0), bounds.height.max(1.0))).map_err(tauri_err)
}

static POPUP_SEQ: AtomicU64 = AtomicU64::new(0);

/// 페이지가 연 새 창을 진짜 창으로 띄운다. 여는 쪽과 같은 웹뷰 환경을 써야(window_features)
/// 쿠키가 공유되고 window.opener 가 이어져 OAuth 팝업이 결과를 돌려줄 수 있다.
/// 팝업이 또 창을 열 수도 있으므로 같은 처리기를 붙인다. 페이지가 window.close() 하면 닫힌다.
fn popup_handler(app: AppHandle) -> impl Fn(Url, NewWindowFeatures) -> NewWindowResponse<tauri::Wry> + Send + 'static {
    move |url, features| {
        if !matches!(url.scheme(), "http" | "https" | "about") {
            return NewWindowResponse::Deny;
        }
        let label = format!("web-popup-{}", POPUP_SEQ.fetch_add(1, Ordering::Relaxed));
        let built = WebviewWindowBuilder::new(&app, label, WebviewUrl::External("about:blank".parse().expect("about:blank")))
            .window_features(features)
            .title(url.as_str())
            .on_navigation(|url| matches!(url.scheme(), "http" | "https" | "about"))
            .on_new_window(popup_handler(app.clone()))
            .on_document_title_changed(|window, title| {
                let _ = window.set_title(&title);
            })
            .build();
        match built {
            Ok(window) => NewWindowResponse::Create { window },
            Err(_) => NewWindowResponse::Deny,
        }
    }
}

/// 없으면 url 로 만들고, 있으면 그 자리로 옮겨 보인다.
#[tauri::command(async)]
pub fn web_show(app: AppHandle, id: String, url: String, bounds: WebBounds) -> Result<(), AppError> {
    if let Ok(view) = webview(&app, &id) {
        place(&view, bounds)?;
        return view.show().map_err(tauri_err);
    }
    let start = parse_url(&url)?;
    let window = app.get_window(MAIN_WINDOW).ok_or_else(|| AppError::NotFound("메인 창이 없습니다".into()))?;
    let label = label_of(&id);

    let (app_nav, id_nav) = (app.clone(), id.clone());
    let (app_title, id_title) = (app.clone(), id.clone());
    let (app_load, id_load) = (app.clone(), id.clone());
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(start))
        // http(s) 밖(file:, javascript: 등)으로는 이동하지 않는다.
        .on_navigation(move |url| {
            let allowed = matches!(url.scheme(), "http" | "https");
            if allowed {
                emit(&app_nav, WebChanged { id: id_nav.clone(), url: Some(url.to_string()), title: None, loading: Some(true) });
            }
            allowed
        })
        // 새 창(target=_blank, window.open)은 별도 팝업 창으로 연다. 로그인 팝업이 window.opener 로
        // 결과를 돌려주므로 같은 탭에서 열면 로그인이 끝나지 않는다.
        .on_new_window(popup_handler(app.clone()))
        .on_document_title_changed(move |_, title| {
            emit(&app_title, WebChanged { id: id_title.clone(), url: None, title: Some(title), loading: None });
        })
        .on_page_load(move |_, payload| {
            let loading = matches!(payload.event(), PageLoadEvent::Started);
            emit(&app_load, WebChanged { id: id_load.clone(), url: Some(payload.url().to_string()), title: None, loading: Some(loading) });
        });
    window
        .add_child(
            builder,
            LogicalPosition::new(bounds.x, bounds.y),
            LogicalSize::new(bounds.width.max(1.0), bounds.height.max(1.0)),
        )
        .map_err(tauri_err)?;
    Ok(())
}

#[tauri::command(async)]
pub fn web_set_bounds(app: AppHandle, id: String, bounds: WebBounds) -> Result<(), AppError> {
    place(&webview(&app, &id)?, bounds)
}

/// 탭이 화면에서 내려가거나 메뉴·다이얼로그가 떠 있는 동안 숨긴다. 페이지는 살아 있다.
#[tauri::command(async)]
pub fn web_hide(app: AppHandle, id: String) {
    if let Ok(view) = webview(&app, &id) {
        let _ = view.hide();
    }
}

#[tauri::command(async)]
pub fn web_close(app: AppHandle, id: String) {
    if let Ok(view) = webview(&app, &id) {
        let _ = view.close();
    }
}

#[tauri::command(async)]
pub fn web_navigate(app: AppHandle, id: String, url: String) -> Result<(), AppError> {
    webview(&app, &id)?.navigate(parse_url(&url)?).map_err(tauri_err)
}

#[tauri::command(async)]
pub fn web_reload(app: AppHandle, id: String) -> Result<(), AppError> {
    webview(&app, &id)?.reload().map_err(tauri_err)
}

/// Tauri 에 뒤로/앞으로 API 가 없어 페이지의 history 를 부른다.
#[tauri::command(async)]
pub fn web_history(app: AppHandle, id: String, forward: bool) -> Result<(), AppError> {
    let js = if forward { "history.forward()" } else { "history.back()" };
    webview(&app, &id)?.eval(js).map_err(tauri_err)
}
