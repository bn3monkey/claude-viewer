use crate::domain::entity::{AppError, Settings};

pub trait SettingsRepository: Send + Sync {
    fn load(&self) -> Settings;
    /// 읽기-수정-저장을 한 번에 한다. 명령이 스레드 풀에서 동시에 돌아도 유실되지 않는다.
    fn update(&self, change: &mut dyn FnMut(&mut Settings)) -> Result<Settings, AppError>;
}
