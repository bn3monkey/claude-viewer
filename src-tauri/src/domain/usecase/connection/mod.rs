use crate::domain::entity::{AppError, ClaudeBinary, ClaudeSource};
use crate::domain::repository::{ClaudeBinaryRepository, SettingsRepository};

/// 후보를 우선순위대로 실행해 보고 처음으로 동작하는 바이너리를 저장한다.
pub fn find_claude_auto(
    binaries: &dyn ClaudeBinaryRepository,
    settings: &dyn SettingsRepository,
) -> Result<ClaudeBinary, AppError> {
    let found = binaries.locate_candidates().into_iter().find_map(|candidate| {
        let version = binaries.probe_version(&candidate.path).ok()?;
        Some(ClaudeBinary { path: candidate.path, version, source: candidate.source })
    });
    let binary = found.ok_or_else(|| {
        AppError::ClaudeNotFound("Claude Code 를 찾지 못했습니다. 수동 찾기로 지정해 주세요.".into())
    })?;
    save_path(settings, &binary.path)?;
    Ok(binary)
}

pub fn set_claude_manual(
    binaries: &dyn ClaudeBinaryRepository,
    settings: &dyn SettingsRepository,
    path: &str,
) -> Result<ClaudeBinary, AppError> {
    let version = binaries.probe_version(path)?;
    save_path(settings, path)?;
    Ok(ClaudeBinary { path: path.to_string(), version, source: ClaudeSource::Manual })
}

/// 저장된 경로가 아직 유효한지 확인한다. 저장된 경로가 없거나 깨졌으면 None.
pub fn verify_claude(
    binaries: &dyn ClaudeBinaryRepository,
    settings: &dyn SettingsRepository,
) -> Option<ClaudeBinary> {
    let path = settings.load().claude_path?;
    let version = binaries.probe_version(&path).ok()?;
    let source = binaries
        .locate_candidates()
        .into_iter()
        .find(|candidate| candidate.path == path)
        .map_or(ClaudeSource::Manual, |candidate| candidate.source);
    Some(ClaudeBinary { path, version, source })
}

fn save_path(settings: &dyn SettingsRepository, path: &str) -> Result<(), AppError> {
    settings.update(&mut |s| s.claude_path = Some(path.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::ClaudeCandidate;
    use crate::domain::usecase::fakes::{FakeBinaries, FakeSettings};

    fn candidate(path: &str, source: ClaudeSource) -> ClaudeCandidate {
        ClaudeCandidate { path: path.into(), source }
    }

    #[test]
    fn auto_find_skips_broken_candidates_and_saves_first_working_one() {
        let binaries = FakeBinaries {
            candidates: vec![
                candidate("broken", ClaudeSource::VsCodeExtension),
                candidate("native", ClaudeSource::NativeInstall),
                candidate("npm", ClaudeSource::PackageManager),
            ],
            working: vec!["native".into(), "npm".into()],
        };
        let settings = FakeSettings::default();

        let found = find_claude_auto(&binaries, &settings).unwrap();

        assert_eq!(found.path, "native");
        assert_eq!(found.source, ClaudeSource::NativeInstall);
        assert_eq!(settings.load().claude_path.as_deref(), Some("native"));
    }

    #[test]
    fn auto_find_reports_when_nothing_works() {
        let binaries = FakeBinaries {
            candidates: vec![candidate("broken", ClaudeSource::Path)],
            working: vec![],
        };
        let result = find_claude_auto(&binaries, &FakeSettings::default());
        assert!(matches!(result, Err(AppError::ClaudeNotFound(_))));
    }

    #[test]
    fn manual_path_is_not_saved_when_it_does_not_run() {
        let binaries = FakeBinaries { candidates: vec![], working: vec![] };
        let settings = FakeSettings::default();
        assert!(set_claude_manual(&binaries, &settings, "nope").is_err());
        assert_eq!(settings.load().claude_path, None);
    }
}
