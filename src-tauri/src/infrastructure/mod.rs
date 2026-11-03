//! OS 의존 기능. `cfg(target_os)` 분기는 이 모듈 안에서만 한다.

pub mod platform;
pub mod process;
pub mod trash;
pub mod watcher;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
