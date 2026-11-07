use std::path::{Component, Path, PathBuf};

use super::project::ensure_within_project;
use crate::domain::entity::{AppError, DefinitionHit, LinkTarget, SavedTab, TextContent, ViewerMode, ViewerRequest};
use crate::domain::repository::{FileSystemRepository, SettingsRepository, ShellRepository};

/// `--append-system-prompt` 로 전달해 AI 가 File Viewer 지시문을 알게 한다.
pub const VIEWER_SYSTEM_PROMPT: &str = "This session runs inside Claude Viewer, which has a File Viewer panel. \
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

/// 마크다운 링크나 코드 속 경로를 뷰어 탭으로 푼다. 상대 경로는 `base_dir` 기준.
/// `file://`, 퍼센트 인코딩, `#L42` / `#L42-L50`, 끝의 `:42` / `:42:7` 을 받는다.
pub fn resolve_link(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    base_dir: &str,
    href: &str,
) -> Result<LinkTarget, AppError> {
    let decoded = percent_decode(href.trim());
    let without_scheme = decoded.strip_prefix("file://").unwrap_or(&decoded);
    // file:///C:/a → /C:/a → C:/a
    let without_scheme = match without_scheme.as_bytes() {
        [b'/', drive, b':', ..] if drive.is_ascii_alphabetic() => &without_scheme[1..],
        _ => without_scheme,
    };
    let (target, fragment) = without_scheme.split_once('#').unwrap_or((without_scheme, ""));
    let (target, suffix_line) = split_line_suffix(target);
    let line = fragment_line(fragment).or(suffix_line);
    if target.is_empty() {
        return Err(AppError::InvalidPath(format!("파일 경로가 아닙니다: {href}")));
    }

    let path = find_link_target(fs, settings, base_dir, target)
        .ok_or_else(|| AppError::NotFound(format!("파일을 찾을 수 없습니다: {target}")))?;
    let path = path.to_string_lossy().into_owned();
    ensure_within_project(fs, settings, &path)?;
    if fs.is_dir(&path) {
        return Err(AppError::InvalidPath(format!("폴더는 뷰어로 열 수 없습니다: {target}")));
    }
    let mode = resolve_viewer_mode(&path);
    Ok(LinkTarget { request: ViewerRequest { path, mode }, line: line.filter(|_| mode == ViewerMode::Source) })
}

/// 링크 경로를 다음 순서로 찾는다.
/// 1. git bash 식 `/d/repo/a.md` 는 `D:/repo/a.md` 로 (Windows)
/// 2. base_dir 과 그 위 폴더를 프로젝트 루트까지. `.md` 미리 보기 속 루트 기준 경로도 찾는다.
/// 3. 프로젝트 안에서 경로 끝이 같은 파일 (가장 얕은 것). AI 가 하위 폴더로 cd 한 뒤 쓴 상대 경로도 찾는다.
fn find_link_target(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    base_dir: &str,
    target: &str,
) -> Option<PathBuf> {
    if cfg!(windows) {
        if let Some(path) = msys_drive_path(target).map(|p| normalize(Path::new(&p))) {
            if fs.exists(&path) {
                return Some(path);
            }
        }
    }
    let target = normalize(Path::new(target));
    let root = project_root_of(fs, settings, base_dir);
    let canonical_root = root.as_deref().and_then(|r| fs.canonicalize(r).ok());
    for dir in Path::new(base_dir).ancestors() {
        let candidate = normalize(&dir.join(&target));
        if fs.exists(&candidate) {
            return Some(candidate);
        }
        // 프로젝트 루트보다 위로는 올라가지 않는다.
        let at_root = canonical_root.is_some() && fs.canonicalize(&dir.to_string_lossy()).ok() == canonical_root;
        if at_root || canonical_root.is_none() {
            break;
        }
    }
    let searchable = target.is_relative() && !target.components().any(|c| matches!(c, Component::ParentDir));
    if !searchable {
        return None;
    }
    let tree = fs.list(root.as_deref()?).ok()?;
    let mut files = Vec::new();
    collect_files(&tree, &mut files);
    files
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.ends_with(&target))
        .min_by_key(|p| p.components().count())
}

/// base_dir 을 품은 등록 프로젝트 루트. 겹치면 가장 깊은 것.
fn project_root_of(fs: &dyn FileSystemRepository, settings: &dyn SettingsRepository, base_dir: &str) -> Option<String> {
    let base = fs.canonicalize(base_dir).ok()?;
    settings
        .load()
        .projects
        .into_iter()
        .filter_map(|p| fs.canonicalize(&p.root).ok().map(|c| (p.root, c)))
        .filter(|(_, canonical)| base.starts_with(canonical))
        .max_by_key(|(_, canonical)| canonical.components().count())
        .map(|(root, _)| root)
}

fn collect_files(node: &crate::domain::entity::FsNode, out: &mut Vec<String>) {
    match &node.children {
        Some(children) => children.iter().for_each(|child| collect_files(child, out)),
        None => out.push(node.path.clone()),
    }
}

/// 구분자를 OS 기본으로 맞추고 `.` 조각을 뗀다. 같은 파일이 다른 탭으로 열리지 않게 한다.
fn normalize(path: &Path) -> PathBuf {
    path.components().filter(|c| !matches!(c, Component::CurDir)).collect()
}

/// `/d/repo/a` → `D:/repo/a`. git bash 가 쓰는 드라이브 표기.
fn msys_drive_path(target: &str) -> Option<String> {
    let bytes = target.as_bytes();
    let drive = *bytes.get(1)?;
    let is_drive = bytes[0] == b'/' && drive.is_ascii_alphabetic() && matches!(bytes.get(2), None | Some(b'/'));
    is_drive.then(|| format!("{}:{}", (drive as char).to_ascii_uppercase(), if bytes.len() > 2 { &target[2..] } else { "/" }))
}

/// 대화·마크다운 속 웹 주소를 기본 브라우저로 연다. http(s) 만 받는다.
pub fn open_web_link(shell: &dyn ShellRepository, url: &str) -> Result<(), AppError> {
    let url = url.trim();
    if !is_web_url(url) {
        return Err(AppError::InvalidPath(format!("http(s) 주소가 아닙니다: {url}")));
    }
    shell.open_url(url)
}

pub fn is_web_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"));
    rest.is_some_and(|r| !r.is_empty()) && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// `L42`, `L42-L50`, `L42C3` → 42
fn fragment_line(fragment: &str) -> Option<u32> {
    let digits: String = fragment.strip_prefix('L')?.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok().filter(|n| *n > 0)
}

/// 마지막 경로 조각 끝의 `:42`, `:42:7`, `:42-50` 을 뗀다. 드라이브 문자 `C:` 는 건드리지 않는다.
fn split_line_suffix(target: &str) -> (&str, Option<u32>) {
    let name_start = target.rfind(['/', '\\']).map_or(0, |i| i + 1);
    let Some(colon) = target[name_start..].find(':').map(|i| name_start + i) else {
        return (target, None);
    };
    let suffix = &target[colon + 1..];
    let is_line = !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit() || c == ':' || c == '-');
    let line = suffix.split([':', '-']).next().and_then(|n| n.parse().ok()).filter(|n| *n > 0);
    match (is_line, line) {
        (true, Some(line)) => (&target[..colon], Some(line)),
        _ => (target, None),
    }
}

pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let decoded = (bytes[i] == b'%' && i + 3 <= bytes.len())
            .then(|| std::str::from_utf8(&bytes[i + 1..i + 3]).ok())
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match decoded {
            Some(byte) => {
                out.push(byte);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 열린 탭을 저장한다. 터미널 탭은 되살릴 수 없으므로 빼고, 활성 위치도 그에 맞춘다.
pub fn save_tabs(settings: &dyn SettingsRepository, tabs: &[SavedTab], active: Option<usize>) -> Result<(), AppError> {
    let (kept, active) = keep_tabs(tabs, active, |tab| tab.mode != ViewerMode::Terminal);
    settings.update(&mut |s| {
        s.viewer_tabs = kept.clone();
        s.active_tab = active;
    })?;
    Ok(())
}

/// 저장된 탭 중 지금도 열 수 있는 것. 사라진 파일이나 닫힌 프로젝트의 탭은 건너뛴다.
/// diff 탭은 지워진 파일도 보여 줄 수 있으므로 프로젝트가 열려 있는지만 본다.
pub fn restore_tabs(fs: &dyn FileSystemRepository, settings: &dyn SettingsRepository) -> (Vec<SavedTab>, Option<usize>) {
    let saved = settings.load();
    keep_tabs(&saved.viewer_tabs, saved.active_tab, |tab| match &tab.diff {
        Some(diff) => saved.projects.iter().any(|p| p.root == diff.root),
        None => tab.mode != ViewerMode::Terminal && ensure_within_project(fs, settings, &tab.path).is_ok(),
    })
}

/// 조건에 맞는 탭만 남긴다. 활성 탭이 빠지면 그 앞의 탭을 활성으로 한다.
fn keep_tabs(tabs: &[SavedTab], active: Option<usize>, keep: impl Fn(&SavedTab) -> bool) -> (Vec<SavedTab>, Option<usize>) {
    let mut kept = Vec::new();
    let mut new_active = None;
    for (i, tab) in tabs.iter().enumerate() {
        if keep(tab) {
            kept.push(tab.clone());
        }
        if Some(i) == active {
            new_active = kept.len().checked_sub(1);
        }
    }
    let new_active = new_active.or_else(|| (!kept.is_empty()).then_some(0));
    (kept, new_active)
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

/// 뷰어에서 고친 텍스트를 저장한다. 프로젝트 밖은 거부.
pub fn write_text(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    path: &str,
    text: &str,
) -> Result<(), AppError> {
    ensure_within_project(fs, settings, path)?;
    fs.write_text(Path::new(path), text)
}

/// F12 로 찾는 파일 확장자. 빌드 산출물·의존성 폴더는 건너뛴다.
const CODE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue", "rs", "py", "go", "java", "kt", "kts", "swift", "c",
    "h", "cpp", "hpp", "cc", "cxx", "hh", "cs", "sh", "bash", "ps1", "lua", "rb", "php", "scala", "dart", "zig", "m",
    "mm",
];
const SKIPPED_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", ".venv", "venv", "__pycache__", ".next", ".idea", ".vs", "obj",
    ".gradle", "generated",
];
const MAX_DEFINITION_FILE_BYTES: u64 = 1024 * 1024;
const MAX_DEFINITION_HITS: usize = 50;

/// `from` 이 속한 프로젝트 전체에서 `name` 을 선언·정의하는 줄을 찾는다.
/// 언어 서버 없이 줄 단위 패턴으로 찾으므로 후보가 여럿일 수 있다. 같은 파일, 같은 폴더 순으로 앞에 둔다.
pub fn find_definitions(
    fs: &dyn FileSystemRepository,
    settings: &dyn SettingsRepository,
    from: &str,
    name: &str,
) -> Result<Vec<DefinitionHit>, AppError> {
    if !is_identifier(name) {
        return Ok(Vec::new());
    }
    let scoped = ensure_within_project(fs, settings, from)?;
    let root = scoped.canonical_root.to_string_lossy().into_owned();
    let mut files = Vec::new();
    collect_code_files(&fs.list(&root)?, Path::new(&root), &mut files);

    let from_dir = Path::new(from).parent().map(normalize);
    let from_canonical = fs.canonicalize(from).ok();
    let rank = |path: &str| {
        let same_file = fs.canonicalize(path).ok() == from_canonical;
        let same_dir = Path::new(path).parent().map(normalize) == from_dir;
        (!same_file, !same_dir, path.to_lowercase())
    };
    files.sort_by_cached_key(|path| rank(path));

    let mut hits = Vec::new();
    for path in files {
        let Ok(content) = fs.read_text(&path, MAX_DEFINITION_FILE_BYTES) else { continue };
        if content.binary || content.truncated {
            continue;
        }
        for (index, line) in content.text.lines().enumerate() {
            if is_definition(line, name) {
                hits.push(DefinitionHit { path: path.clone(), line: index as u32 + 1, text: line.trim().to_string() });
                if hits.len() >= MAX_DEFINITION_HITS {
                    return Ok(hits);
                }
            }
        }
    }
    Ok(hits)
}

/// 루트 아래의 코드 파일. 건너뛸 폴더는 노드 이름이 아니라 루트 기준 경로 조각으로 본다 (목록이 평평해도 된다).
fn collect_code_files(node: &crate::domain::entity::FsNode, root: &Path, out: &mut Vec<String>) {
    match &node.children {
        Some(children) => children.iter().for_each(|child| collect_code_files(child, root, out)),
        None => {
            let path = Path::new(&node.path);
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
            let in_skipped_dir = path
                .strip_prefix(root)
                .unwrap_or(path)
                .components()
                .any(|c| SKIPPED_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()));
            if !in_skipped_dir && ext.is_some_and(|e| CODE_EXTENSIONS.contains(&e.as_str())) {
                out.push(node.path.clone());
            }
        }
    }
}

fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.as_bytes()[0].is_ascii_digit()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}

fn is_identifier_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// `NAME` 바로 앞에 오면 선언으로 보는 낱말.
const DECLARATION_KEYWORDS: &[&str] = &[
    "class", "struct", "enum", "interface", "trait", "type", "typedef", "union", "namespace", "module", "mod", "fn",
    "function", "def", "func", "fun", "proc", "sub", "const", "static", "let", "var", "val", "object", "record",
    "protocol", "extension", "actor", "macro", "macro_rules!", "impl", "define", "typealias", "async", "get", "set",
    "delegate", "event", "local", "global", "my", "our", "alias", "pub", "@interface", "@implementation",
    "@protocol", "@property", "#define", "%define",
];
/// 줄의 첫 낱말이 이것이면 `타입 NAME(` 꼴의 정의로 보지 않는다 (호출·반환 등).
const NOT_DEFINITION_STARTS: &[&str] = &[
    "return", "new", "await", "yield", "throw", "else", "in", "of", "as", "typeof", "instanceof", "delete", "void",
    "case", "is", "not", "and", "or", "import", "from", "extends", "implements", "if", "while", "for", "switch",
    "catch", "with", "elif", "unless", "until", "do", "try", "assert", "print", "echo", "use", "require", "include",
    "goto", "break", "continue", "export", "sizeof", "co_await", "co_return", "co_yield", "emit", "raise",
];

/// 한 줄이 `name` 의 선언·정의로 보이는가. 언어 서버가 없으니 흔한 꼴만 잡는다:
/// `키워드 NAME`, `impl … for NAME`, `타입 NAME(` / `타입 NAME =` / `타입 NAME;`, 줄머리의 `NAME(…) {` 메서드,
/// 들여쓰기 없는 `NAME = …`.
pub fn is_definition(line: &str, name: &str) -> bool {
    let bytes = line.as_bytes();
    let mut search = 0;
    while let Some(found) = line[search..].find(name) {
        let start = search + found;
        let end = start + name.len();
        search = start + 1;
        let whole_word = !(start > 0 && is_identifier_byte(bytes[start - 1]))
            && !(end < bytes.len() && is_identifier_byte(bytes[end]));
        if !whole_word {
            continue;
        }
        let separated = line[..start].ends_with(char::is_whitespace);
        let before = line[..start].trim_end();
        let after = line[end..].trim_start();
        if before.ends_with('.') || before.ends_with("->") {
            continue; // obj.name, ptr->name: 멤버 접근
        }
        let previous_word = before.rsplit(char::is_whitespace).next().unwrap_or("");
        let previous_word = previous_word.split('<').next().unwrap_or(previous_word);
        if DECLARATION_KEYWORDS.contains(&previous_word) || (previous_word == "for" && before.starts_with("impl")) {
            return true;
        }
        if before.is_empty() {
            if after.starts_with('(') && is_method_definition(after) {
                return true;
            }
            if line.starts_with(name) && is_assignment(after) {
                return true;
            }
            continue;
        }
        let declares = after.starts_with('(') || after.starts_with(';') || after.starts_with('{') || is_assignment(after);
        if declares && is_typed_definition(before, separated) {
            return true;
        }
    }
    false
}

/// `= …` 이되 `==`, `=>` 가 아닌 것.
fn is_assignment(after: &str) -> bool {
    after.starts_with('=') && !after.starts_with("==") && !after.starts_with("=>")
}

/// 줄머리의 `foo(a, b) {` / `foo(a): T {`. 괄호가 이 줄에서 닫히고 그 뒤가 본문 `{` 로 이어져야 한다.
/// `foo(bar, () => {` 같은 콜백 호출은 괄호가 닫히지 않아 걸러진다.
fn is_method_definition(after: &str) -> bool {
    let mut depth = 0usize;
    for (index, c) in after.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let tail = after[index + 1..].trim();
                    return tail.ends_with('{') && !tail.contains("=>") && (tail.starts_with('{') || tail.starts_with(':'));
                }
            }
            _ => {}
        }
    }
    false
}

/// `static void Foo::bar(` / `private foo(` / `std::string name =` / Go 의 `func (r *T) Name(`.
/// 앞쪽이 타입·한정자 낱말만으로 되어 있어야 한다 (`=`, `(`, `,` 가 끼면 식의 일부).
fn is_typed_definition(before: &str, separated: bool) -> bool {
    let first_word = before.split(|c: char| !is_identifier_byte(c as u8)).next().unwrap_or("");
    if NOT_DEFINITION_STARTS.contains(&first_word) {
        return false;
    }
    if before.starts_with("func ") || before.starts_with("func(") {
        return true;
    }
    let type_like = before.chars().all(|c| is_identifier_byte(c as u8) || c.is_whitespace() || "<>*&:[]~".contains(c));
    let qualified = before.ends_with("::");
    // `Foo::bar(` 는 호출, `void Foo::bar(` 는 정의.
    let ends_like_type = separated
        || before.ends_with(|c: char| "*&>".contains(c))
        || (qualified && before.split_whitespace().count() >= 2);
    type_like && ends_like_type
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

    fn link_world() -> (FakeFs, FakeSettings) {
        let fs = FakeFs::with_dirs(&["/p", "/p/src", "/secret"]);
        for file in ["/p/src/a.rs", "/p/my docs/b.md", "/p/c.pdf", "/secret/key.pem"] {
            fs.add_file(file);
        }
        (fs, FakeSettings::with_project("/p", &[]))
    }

    fn link(href: &str) -> Result<(String, ViewerMode, Option<u32>), AppError> {
        let (fs, settings) = link_world();
        resolve_link(&fs, &settings, "/p", href).map(|t| {
            (t.request.path.replace('\\', "/"), t.request.mode, t.line)
        })
    }

    #[test]
    fn links_resolve_relative_to_the_base_with_line_marks() {
        assert_eq!(link("src/a.rs").unwrap(), ("/p/src/a.rs".into(), ViewerMode::Source, None));
        assert_eq!(link("src/a.rs#L42").unwrap().2, Some(42));
        assert_eq!(link("src/a.rs#L42-L50").unwrap().2, Some(42));
        assert_eq!(link("src/a.rs:42").unwrap().2, Some(42));
        assert_eq!(link("src/a.rs:42:7").unwrap().2, Some(42));
        assert_eq!(link("./src/a.rs:10-12").unwrap().2, Some(10));
        assert_eq!(link("my%20docs/b.md").unwrap().0, "/p/my docs/b.md");
        assert_eq!(link("/p/src/a.rs").unwrap().0, "/p/src/a.rs");
        assert_eq!(link("file:///p/src/a.rs#L3").unwrap().2, Some(3));
        // 줄 번호는 소스 탭에만 의미가 있다.
        assert_eq!(link("c.pdf:3").unwrap(), ("/p/c.pdf".into(), ViewerMode::Pdf, None));
    }

    #[test]
    fn links_that_are_missing_folders_or_outside_fail() {
        assert!(matches!(link("nope.rs"), Err(AppError::NotFound(_))));
        assert!(matches!(link("src"), Err(AppError::InvalidPath(_))));
        assert!(matches!(link("/secret/key.pem"), Err(AppError::OutsideProject(_))));
        assert!(link("../secret/key.pem").is_err());
        assert!(matches!(link("#L3"), Err(AppError::InvalidPath(_))));
    }

    #[test]
    fn links_fall_back_to_parent_folders_and_then_to_a_project_wide_suffix_match() {
        let fs = FakeFs::with_dirs(&["/p", "/p/docs", "/p/docs/progress", "/p/lib", "/p/lib/core", "/p/lib/core/src"]);
        for file in ["/p/docs/progress/a.md", "/p/lib/core/src/x.rs", "/p/lib/core/src/deep/x.rs"] {
            fs.add_file(file);
        }
        let settings = FakeSettings::with_project("/p", &[]);
        let at = |base: &str, href: &str| resolve_link(&fs, &settings, base, href).map(|t| t.request.path.replace('\\', "/"));
        // .md 미리 보기(기준 폴더 /p/docs/progress)에서 루트 기준 경로
        assert_eq!(at("/p/docs/progress", "docs/progress/a.md").unwrap(), "/p/docs/progress/a.md");
        // 세션(기준 /p)에서 AI 가 하위 폴더 기준으로 쓴 경로. 가장 얕은 파일을 고른다.
        assert_eq!(at("/p", "src/x.rs").unwrap(), "/p/lib/core/src/x.rs");
        assert_eq!(at("/p", "./src/x.rs:3").unwrap(), "/p/lib/core/src/x.rs");
        assert!(matches!(at("/p", "src/nope.rs"), Err(AppError::NotFound(_))));
        assert!(at("/p/docs", "../../secret/key.pem").is_err());
    }

    #[test]
    fn git_bash_drive_paths_and_web_urls() {
        assert_eq!(msys_drive_path("/d/repo/a.md").as_deref(), Some("D:/repo/a.md"));
        assert_eq!(msys_drive_path("/d").as_deref(), Some("D:/"));
        assert_eq!(msys_drive_path("/docs/a.md"), None);
        assert_eq!(msys_drive_path("docs/a.md"), None);
        assert!(is_web_url("https://example.com/a?b=1") && is_web_url("HTTP://x.y"));
        for bad in ["https://", "file:///c:/a", "javascript:alert(1)", "https://a b", "ftp://x"] {
            assert!(!is_web_url(bad), "{bad}");
        }
    }

    #[test]
    fn line_suffix_leaves_drive_letters_alone() {
        assert_eq!(split_line_suffix("C:/a/b.rs"), ("C:/a/b.rs", None));
        assert_eq!(split_line_suffix("C:\\a\\b.rs:9"), ("C:\\a\\b.rs", Some(9)));
        assert_eq!(split_line_suffix("a.rs:x"), ("a.rs:x", None));
        assert_eq!(split_line_suffix("a.rs:0"), ("a.rs:0", None));
        assert_eq!(percent_decode("abc%"), "abc%");
    }

    fn tab(path: &str, mode: ViewerMode) -> SavedTab {
        SavedTab { path: path.into(), mode, diff: None }
    }

    #[test]
    fn saving_tabs_drops_terminals_and_keeps_the_active_one() {
        let settings = FakeSettings::with_project("/p", &[]);
        let tabs = [tab("/p/a.rs", ViewerMode::Source), tab("/p/run.bat", ViewerMode::Terminal), tab("/p/b.md", ViewerMode::Markdown)];
        save_tabs(&settings, &tabs, Some(2)).unwrap();
        let saved = settings.load();
        assert_eq!(saved.viewer_tabs, vec![tabs[0].clone(), tabs[2].clone()]);
        assert_eq!(saved.active_tab, Some(1));

        // 활성 탭이 터미널이면 그 앞 탭을 활성으로.
        save_tabs(&settings, &tabs, Some(1)).unwrap();
        assert_eq!(settings.load().active_tab, Some(0));
        save_tabs(&settings, &[], None).unwrap();
        assert_eq!(settings.load().active_tab, None);
    }

    #[test]
    fn restoring_skips_missing_files_and_closed_projects() {
        let fs = FakeFs::with_dirs(&["/p"]);
        fs.add_file("/p/a.rs");
        let settings = FakeSettings::with_project("/p", &[]);
        let deleted_diff = SavedTab {
            diff: Some(crate::domain::entity::settings::SavedDiff { root: "/p".into(), branch: None, untracked: false, commit: None }),
            ..tab("/p/gone.rs", ViewerMode::Diff)
        };
        let other_diff = SavedTab {
            diff: Some(crate::domain::entity::settings::SavedDiff { root: "/q".into(), branch: None, untracked: false, commit: None }),
            ..tab("/q/x.rs", ViewerMode::Diff)
        };
        let tabs = vec![tab("/p/a.rs", ViewerMode::Source), tab("/secret/k.pem", ViewerMode::Source), deleted_diff.clone(), other_diff];
        settings.update(&mut |s| {
            s.viewer_tabs = tabs.clone();
            s.active_tab = Some(1);
        }).unwrap();

        let (restored, active) = restore_tabs(&fs, &settings);
        assert_eq!(restored, vec![tabs[0].clone(), deleted_diff]);
        assert_eq!(active, Some(0));
    }

    #[test]
    fn reading_outside_the_project_fails() {
        let fs = FakeFs::with_dirs(&["/p"]);
        let settings = FakeSettings::with_project("/p", &[]);
        assert!(read_text(&fs, &settings, "/secret/a.txt").is_err());
        assert!(read_bytes(&fs, &settings, "/secret/a.bin").is_err());
    }

    #[test]
    fn definition_lines_are_recognised_across_languages() {
        let yes = [
            ("export class Foo<T> {", "Foo"),
            ("pub(crate) fn foo(a: u32) -> Bar {", "foo"),
            ("impl<T> Trait for Foo<T> {", "Foo"),
            ("def foo(self):", "foo"),
            ("    private foo(a: string): void {", "foo"),
            ("  foo(a) {", "foo"),
            ("static void Foo::bar(int a)", "bar"),
            ("int main(int argc, char** argv)", "main"),
            ("std::vector<int> make(", "make"),
            ("func (r *T) Name(x int) {", "Name"),
            ("public int Count { get; set; }", "Count"),
            ("const int MAX = 10;", "MAX"),
            ("MAX = 10", "MAX"),
            ("#define MAX 10", "MAX"),
            ("    pub name: String,", "name"),
            ("export const useFoo = create(", "useFoo"),
            ("  async load() {", "load"),
            ("macro_rules! foo {", "foo"),
            ("type Foo = Bar;", "Foo"),
        ];
        for (line, name) in yes {
            assert!(is_definition(line, name), "should match: {line}");
        }
        let no = [
            ("return foo(1);", "foo"),
            ("  foo(1);", "foo"),
            ("  useEffect(() => {", "useEffect"),
            ("let x = foo(1);", "foo"),
            ("obj.foo(1) {", "foo"),
            ("ptr->foo = 1;", "foo"),
            ("Foo::bar();", "bar"),
            ("new Foo(1)", "Foo"),
            ("    result = compute()", "result"),
            ("if foo == 1 {", "foo"),
            ("fn foobar() {", "foo"),
            ("class Foo {", "Fo"),
            ("fn f() -> Foo {", "Foo"),
            ("  foo(a, () => {", "foo"),
        ];
        for (line, name) in no {
            assert!(!is_definition(line, name), "should not match: {line}");
        }
    }

    #[test]
    fn definitions_are_found_project_wide_with_the_current_file_first() {
        let fs = FakeFs::with_dirs(&["/p"]);
        let settings = FakeSettings::with_project("/p", &[]);
        fs.add_text("/p/src/b.rs", "pub struct Foo;\nfn use_it(f: Foo) {}\n");
        fs.add_text("/p/src/a.rs", "fn main() {\n    let foo = Foo::new();\n}\nstruct Foo {}\n");
        fs.add_text("/p/node_modules/x/index.js", "class Foo {}\n");
        fs.add_text("/p/notes.md", "class Foo\n");

        let hits = find_definitions(&fs, &settings, "/p/src/a.rs", "Foo").unwrap();
        let summary: Vec<_> = hits.iter().map(|h| (h.path.as_str(), h.line, h.text.as_str())).collect();
        assert_eq!(summary, vec![("/p/src/a.rs", 4, "struct Foo {}"), ("/p/src/b.rs", 1, "pub struct Foo;")]);

        assert!(find_definitions(&fs, &settings, "/p/src/a.rs", "no such").unwrap().is_empty());
        assert!(find_definitions(&fs, &settings, "/secret/a.rs", "Foo").is_err());
    }

    #[test]
    fn writing_is_limited_to_the_project() {
        let fs = FakeFs::with_dirs(&["/p"]);
        let settings = FakeSettings::with_project("/p", &[]);
        write_text(&fs, &settings, "/p/a.txt", "hello").unwrap();
        assert_eq!(fs.text_of("/p/a.txt").as_deref(), Some("hello"));
        assert!(write_text(&fs, &settings, "/secret/a.txt", "x").is_err());
    }
}
