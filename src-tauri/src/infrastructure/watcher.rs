use std::path::{Path, PathBuf};
use std::time::Duration;

use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

/// drop 되면 감시가 멈춘다.
pub type WatchHandle = Box<dyn Send>;

/// 변경된 경로 묶음을 debounce 해서 넘긴다.
pub fn watch(
    path: &Path,
    debounce: Duration,
    on_change: impl Fn(Vec<PathBuf>) + Send + 'static,
) -> Result<WatchHandle, String> {
    let mut debouncer = new_debouncer(debounce, None, move |result: DebounceEventResult| {
        if let Ok(events) = result {
            let paths: Vec<PathBuf> = events.into_iter().flat_map(|event| event.event.paths).collect();
            if !paths.is_empty() {
                on_change(paths);
            }
        }
    })
    .map_err(|e| e.to_string())?;
    debouncer.watch(path, RecursiveMode::Recursive).map_err(|e| e.to_string())?;
    Ok(Box::new(debouncer))
}
