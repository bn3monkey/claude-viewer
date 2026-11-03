use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::domain::entity::AppError;
use crate::domain::repository::ClipboardRepository;

pub struct ClipboardRepositoryImpl {
    pub app: AppHandle,
}

impl ClipboardRepository for ClipboardRepositoryImpl {
    fn write_text(&self, text: &str) -> Result<(), AppError> {
        self.app.clipboard().write_text(text).map_err(|e| AppError::Io(e.to_string()))
    }
}
