use crate::domain::entity::AppError;

pub trait ClipboardRepository: Send + Sync {
    fn write_text(&self, text: &str) -> Result<(), AppError>;
}
