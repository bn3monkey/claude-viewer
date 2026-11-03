use crate::domain::entity::AppError;

pub trait ShellRepository: Send + Sync {
    fn reveal_in_explorer(&self, path: &str) -> Result<(), AppError>;
    fn open_in_browser(&self, path: &str) -> Result<(), AppError>;
    /// 실행 파일을 그 파일이 있는 폴더를 작업 폴더로 해서 띄운다.
    fn run_executable(&self, path: &str) -> Result<(), AppError>;
}
