//! tauri-plugin-store 래퍼. 설정은 `%APPDATA%\claude_viewer\settings.json` 하나에 저장한다.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

use crate::domain::entity::{AppError, Settings};

const KEY: &str = "settings";

pub struct SettingsStore {
    store: Arc<Store<Wry>>,
}

impl SettingsStore {
    pub fn open(app: &AppHandle, path: PathBuf) -> Result<Self, AppError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| AppError::Settings(e.to_string()))?;
        }
        let store = app.store(path).map_err(|e| AppError::Settings(e.to_string()))?;
        Ok(Self { store })
    }

    /// 손으로 고친 파일이 깨졌으면 기본값으로 시작한다.
    pub fn read(&self) -> Settings {
        self.store
            .get(KEY)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    }

    pub fn write(&self, settings: &Settings) -> Result<(), AppError> {
        let value = serde_json::to_value(settings).map_err(|e| AppError::Settings(e.to_string()))?;
        self.store.set(KEY, value);
        self.store.save().map_err(|e| AppError::Settings(e.to_string()))
    }
}
