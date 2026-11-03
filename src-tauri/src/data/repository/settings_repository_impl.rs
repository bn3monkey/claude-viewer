use std::sync::Mutex;

use crate::data::datasource::settings_store::SettingsStore;
use crate::domain::entity::{AppError, Settings};
use crate::domain::repository::SettingsRepository;

pub struct SettingsRepositoryImpl {
    store: Mutex<SettingsStore>,
}

impl SettingsRepositoryImpl {
    pub fn new(store: SettingsStore) -> Self {
        Self { store: Mutex::new(store) }
    }
}

impl SettingsRepository for SettingsRepositoryImpl {
    fn load(&self) -> Settings {
        self.store.lock().unwrap().read()
    }

    fn update(&self, change: &mut dyn FnMut(&mut Settings)) -> Result<Settings, AppError> {
        let store = self.store.lock().unwrap();
        let mut settings = store.read();
        change(&mut settings);
        store.write(&settings)?;
        Ok(settings)
    }
}
