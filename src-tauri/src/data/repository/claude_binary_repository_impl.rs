use std::path::PathBuf;

use crate::domain::entity::{AppError, ClaudeSource};
use crate::domain::repository::{ClaudeBinaryRepository, ClaudeCandidate};
use crate::infrastructure::platform::{CandidateKind, Platform};
use crate::infrastructure::process::std_command;

pub struct ClaudeBinaryRepositoryImpl {
    pub platform: Box<dyn Platform>,
}

impl ClaudeBinaryRepository for ClaudeBinaryRepositoryImpl {
    fn locate_candidates(&self) -> Vec<ClaudeCandidate> {
        let mut found: Vec<ClaudeCandidate> = Vec::new();
        let mut push = |path: PathBuf, source: ClaudeSource| {
            let path = path.to_string_lossy().into_owned();
            if !found.iter().any(|c| c.path == path) {
                found.push(ClaudeCandidate { path, source });
            }
        };
        for candidate in self.platform.candidate_paths() {
            let source = to_source(candidate.kind);
            if candidate.glob {
                for path in newest_first(&candidate.path.to_string_lossy()) {
                    push(path, source);
                }
            } else if candidate.path.is_file() {
                push(candidate.path, source);
            }
        }
        for path in self.platform.find_in_path() {
            push(path, ClaudeSource::Path);
        }
        found
    }

    fn probe_version(&self, path: &str) -> Result<String, AppError> {
        let output = std_command(path)
            .arg("--version")
            .output()
            .map_err(|e| AppError::ClaudeNotFound(format!("실행할 수 없습니다 ({path}): {e}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        // "2.1.314 (Claude Code)"
        let version = stdout.split_whitespace().next().unwrap_or_default();
        if !output.status.success() || !version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Err(AppError::ClaudeNotFound(format!("Claude Code 가 아닙니다: {path}")));
        }
        Ok(version.to_string())
    }
}

fn to_source(kind: CandidateKind) -> ClaudeSource {
    match kind {
        CandidateKind::VsCodeExtension => ClaudeSource::VsCodeExtension,
        CandidateKind::NativeInstall => ClaudeSource::NativeInstall,
        CandidateKind::PackageManager => ClaudeSource::PackageManager,
    }
}

/// 확장 폴더가 여러 버전이면 가장 높은 버전부터.
fn newest_first(pattern: &str) -> Vec<PathBuf> {
    let Ok(paths) = glob::glob(pattern) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = paths.flatten().filter(|p| p.is_file()).collect();
    paths.sort_by_key(|path| std::cmp::Reverse(extension_version(path)));
    paths
}

/// `anthropic.claude-code-2.1.314-win32-x64` → [2, 1, 314]
fn extension_version(path: &std::path::Path) -> Vec<u32> {
    path.ancestors()
        .filter_map(|p| p.file_name()?.to_str())
        .find_map(|name| name.strip_prefix("anthropic.claude-code-"))
        .and_then(|rest| rest.split('-').next())
        .map(|version| version.split('.').map(|n| n.parse().unwrap_or(0)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_version_is_numeric_not_lexical() {
        let v = |s: &str| extension_version(&PathBuf::from(s).join("resources/native-binary/claude.exe"));
        assert_eq!(v("x/anthropic.claude-code-2.1.314-win32-x64"), vec![2, 1, 314]);
        assert!(v("x/anthropic.claude-code-2.1.314-win32-x64") > v("x/anthropic.claude-code-2.1.94-win32-x64"));
        assert_eq!(v("x/other"), Vec::<u32>::new());
    }
}
