use tauri::{AppHandle, Url};
use tauri_plugin_opener::OpenerExt;

use crate::domain::entity::AppError;
use crate::domain::repository::ShellRepository;
use crate::infrastructure::process::launch_detached;

pub struct ShellRepositoryImpl {
    pub app: AppHandle,
}

impl ShellRepository for ShellRepositoryImpl {
    fn reveal_in_explorer(&self, path: &str) -> Result<(), AppError> {
        self.app.opener().reveal_item_in_dir(path).map_err(|e| AppError::Io(e.to_string()))
    }

    /// 앱 안에서 임의 HTML 을 실행하지 않고 기본 브라우저로 넘긴다.
    fn open_in_browser(&self, path: &str) -> Result<(), AppError> {
        let url = Url::from_file_path(path)
            .map_err(|_| AppError::InvalidPath(format!("URL 로 바꿀 수 없습니다: {path}")))?;
        self.app.opener().open_url(url.as_str(), None::<&str>).map_err(|e| AppError::Io(e.to_string()))
    }

    fn open_url(&self, url: &str) -> Result<(), AppError> {
        self.app.opener().open_url(url, None::<&str>).map_err(|e| AppError::Io(e.to_string()))
    }

    fn run_executable(&self, path: &str) -> Result<(), AppError> {
        let program = std::path::Path::new(path);
        let cwd = program.parent().unwrap_or(program);
        launch_detached(program, cwd).map_err(|e| AppError::Process(format!("실행하지 못했습니다 ({path}): {e}")))
    }
}
