use std::path::Path;

use super::project::ensure_within_project;
use crate::domain::entity::{AppError, TextContent, ViewerMode, ViewerRequest};
use crate::domain::repository::{FileSystemRepository, SettingsRepository, ShellRepository};

/// `--append-system-prompt` 로 전달해 AI 가 File Viewer 지시문을 알게 한다.
pub const VIEWER_SYSTEM_PROMPT: &str = "This session runs inside Claude Viewer, which has a read-only File Viewer panel. \
To show the user a project file there, include this tag in your reply: \
<viewer open=\"relative/path\" mode=\"source|markdown|html|pdf|docx|image|video\"/> \
The mode attribute is optional. The tag is removed from the displayed text. \
Use it only when the user asks to see a file.";

/// 파일을 왼쪽 클릭했을 때의 기본 모드. .md / .html 은 소스로 연다.
pub fn resolve_viewer_mode(path: &str) -> ViewerMode {
    let extension = Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "pdf" => ViewerMode::Pdf,
        "docx" => ViewerMode::Docx,
        "png" | "jpg" | "jpeg" | "webp" | "bmp" => ViewerMode::Image,
        "mp4" => ViewerMode::Video,
        _ => ViewerMode::Source,
    }
}

#[derive(Debug, PartialEq)]
pub struct ViewerDirective {
    pub path: String,
    pub mode: Option<ViewerMode>,
}

/// 응답 본문에서 `<viewer open="..." mode="..."/>` 를 찾아 떼어낸다.
pub fn parse_viewer_directives(text: &str) -> (String, Vec<ViewerDirective>) {
    const OPEN_TAG: &str = "<viewer ";
    let mut stripped = String::with_capacity(text.len());
    let mut directives = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN_TAG) {
        let Some(length) = rest[start..].find("/>") else { break };
        let tag = &rest[start..start + length];
        match attribute(tag, "open") {
            Some(path) => {
                stripped.push_str(&rest[..start]);
                directives.push(ViewerDirective {
                    path: path.to_string(),
                    mode: attribute(tag, "mode").and_then(parse_mode),
                });
            }
            None => stripped.push_str(&rest[..start + length + 2]),
        }
        rest = &rest[start + length + 2..];
    }
    stripped.push_str(rest);
    (stripped, directives)
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let length = tag[start..].find('"')?;
    Some(&tag[start..start + length])
}

fn parse_mode(mode: &str) -> Option<ViewerMode> {
    match mode {
        "source" => Some(ViewerMode::Source),
        "markdown" => Some(ViewerMode::Markdown),
        "html" => Some(ViewerMode::Html),
        "pdf" => Some(ViewerMode::Pdf),
        "docx" => Some(ViewerMode::Docx),
        "image" => Some(ViewerMode::Image),
        "video" => Some(ViewerMode::Video),
        _ => None,
    }
}

/// AI 가 요청한 경로를 프로젝트 기준으로 풀어 뷰어 요청으로 바꾼다. 프로젝트 밖은 거부한다.
pub fn resolve_directive(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    root: &str,
    directive: &ViewerDirective,
) -> Result<ViewerRequest, AppError> {
    let path = Path::new(root).join(&directive.path).to_string_lossy().into_owned();
    ensure_within_project(fs, settings, &path)?;
    let mode = directive.mode.unwrap_or_else(|| resolve_viewer_mode(&path));
    Ok(ViewerRequest { path, mode })
}

pub fn read_text(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    path: &str,
) -> Result<TextContent, AppError> {
    const MAX_TEXT_BYTES: u64 = 5 * 1024 * 1024;
    ensure_within_project(fs, settings, path)?;
    fs.read_text(path, MAX_TEXT_BYTES)
}

pub fn read_bytes(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    path: &str,
) -> Result<Vec<u8>, AppError> {
    ensure_within_project(fs, settings, path)?;
    fs.read_bytes(path)
}

pub fn open_external(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    shell: &dyn ShellRepository,
    path: &str,
) -> Result<(), AppError> {
    ensure_within_project(fs, settings, path)?;
    shell.open_in_browser(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usecase::fakes::{FakeFs, FakeSettings};

    #[test]
    fn click_mode_is_source_for_md_html_and_code() {
        for path in ["a.md", "a.html", "a.rs", "a.txt", "Makefile"] {
            assert_eq!(resolve_viewer_mode(path), ViewerMode::Source, "{path}");
        }
    }

    #[test]
    fn click_mode_follows_binary_formats_case_insensitively() {
        assert_eq!(resolve_viewer_mode("a.PDF"), ViewerMode::Pdf);
        assert_eq!(resolve_viewer_mode("a.docx"), ViewerMode::Docx);
        for path in ["a.png", "a.jpeg", "a.jpg", "a.webp", "a.bmp"] {
            assert_eq!(resolve_viewer_mode(path), ViewerMode::Image, "{path}");
        }
        assert_eq!(resolve_viewer_mode("a.mp4"), ViewerMode::Video);
    }

    #[test]
    fn directives_are_extracted_and_removed_from_the_text() {
        let (text, directives) = parse_viewer_directives(
            "여기 있습니다 <viewer open=\"docs/a.md\" mode=\"markdown\"/> 그리고 <viewer open=\"b.png\"/>끝",
        );
        assert_eq!(text, "여기 있습니다  그리고 끝");
        assert_eq!(
            directives,
            vec![
                ViewerDirective { path: "docs/a.md".into(), mode: Some(ViewerMode::Markdown) },
                ViewerDirective { path: "b.png".into(), mode: None },
            ]
        );
    }

    #[test]
    fn text_without_a_complete_directive_is_unchanged() {
        for text in ["그냥 글", "<viewer open=\"a\"", "<viewer mode=\"pdf\"/> 경로 없음", "<viewers/>"] {
            let (stripped, directives) = parse_viewer_directives(text);
            assert_eq!(stripped, text);
            assert!(directives.is_empty());
        }
    }

    #[test]
    fn directive_cannot_escape_the_project() {
        let fs = FakeFs::with_dirs(&["/p"]);
        let settings = FakeSettings::with_project("/p", &[]);
        let escape = ViewerDirective { path: "/secret/key.pem".into(), mode: None };
        assert!(matches!(
            resolve_directive(&fs, &settings, "/p", &escape),
            Err(AppError::OutsideProject(_))
        ));
        let inside = ViewerDirective { path: "a.pdf".into(), mode: None };
        assert_eq!(resolve_directive(&fs, &settings, "/p", &inside).unwrap().mode, ViewerMode::Pdf);
    }

    #[test]
    fn reading_outside_the_project_fails() {
        let fs = FakeFs::with_dirs(&["/p"]);
        let settings = FakeSettings::with_project("/p", &[]);
        assert!(read_text(&fs, &settings, "/secret/a.txt").is_err());
        assert!(read_bytes(&fs, &settings, "/secret/a.bin").is_err());
    }
}
