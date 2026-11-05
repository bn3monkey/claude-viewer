//! `git` 명령 실행과 출력 파싱. PATH 의 git 을 쓴다.

use std::path::Path;

use crate::domain::entity::{GitBranch, GitChange, GitChangeKind, GitCommit};
use crate::infrastructure::process::std_command;

pub fn run(root: &Path, args: &[&str]) -> Result<String, String> {
    // 사용자 설정(i18n.logOutputEncoding=cp949 등)과 무관하게 UTF-8 로 받는다. git 이 커밋 인코딩에서 변환해 준다.
    // 자격 증명을 물으면 창을 띄우지 않고 실패하게 한다 (pull 실패 대화상자로 이유를 보여준다).
    let output = std_command("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["-c", "core.quotepath=false", "-c", "i18n.logOutputEncoding=UTF-8"])
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("git 을 실행할 수 없습니다: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `git branch --format` 출력 한 줄: `<current>\t<name>\t<epoch>\t<subject>`
pub const BRANCH_FORMAT: &str = "%(if)%(HEAD)%(then)*%(else)-%(end)\t%(refname:short)\t%(committerdate:unix)\t%(subject)";

pub fn parse_branches(text: &str) -> Vec<GitBranch> {
    let mut branches: Vec<GitBranch> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            let current = parts.next()? == "*";
            let name = parts.next()?.to_string();
            let committed_at = parts.next()?.parse().unwrap_or(0.0);
            let subject = parts.next().unwrap_or_default().to_string();
            Some(GitBranch { name, current, subject, committed_at })
        })
        .collect();
    branches.sort_by(|a, b| b.current.cmp(&a.current).then(b.committed_at.total_cmp(&a.committed_at)));
    branches
}

/// `git log --format` 필드 구분자는 0x1f (unit separator)
pub const LOG_FORMAT: &str = "--format=%H%x1f%an%x1f%ct%x1f%s";

pub fn parse_log(text: &str) -> Vec<GitCommit> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\u{1f}');
            Some(GitCommit {
                hash: parts.next()?.to_string(),
                author: parts.next()?.to_string(),
                committed_at: parts.next()?.parse().unwrap_or(0.0),
                subject: parts.next().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// `git status --porcelain` (v1). 두 글자 상태 + 공백 + 경로. 이름 변경은 `old -> new`.
pub fn parse_status(text: &str) -> Vec<GitChange> {
    text.lines()
        .filter(|line| line.len() > 3)
        .filter_map(|line| {
            let (code, rest) = line.split_at(2);
            let path = rest.trim_start();
            let path = path.split(" -> ").last().unwrap_or(path);
            let path = unquote(path);
            let kind = match (code.as_bytes()[0], code.as_bytes()[1]) {
                (b'?', _) => return Some(GitChange { path, kind: GitChangeKind::Added, untracked: true }),
                (b'!', _) => return None,
                (b'D', _) | (_, b'D') => GitChangeKind::Deleted,
                (b'A', _) => GitChangeKind::Added,
                (b'R', _) | (_, b'R') => GitChangeKind::Renamed,
                _ => GitChangeKind::Modified,
            };
            Some(GitChange { path, kind, untracked: false })
        })
        .collect()
}

/// `git diff --name-status`: `<code>\t<path>` 또는 `R100\t<old>\t<new>`
pub fn parse_name_status(text: &str) -> Vec<GitChange> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let code = parts.next()?;
            let path = parts.last()?.to_string();
            let kind = match code.chars().next()? {
                'A' => GitChangeKind::Added,
                'D' => GitChangeKind::Deleted,
                'R' | 'C' => GitChangeKind::Renamed,
                _ => GitChangeKind::Modified,
            };
            Some(GitChange { path, kind, untracked: false })
        })
        .collect()
}

/// 아직 추적되지 않는 파일은 git diff 가 없으므로 전체를 추가로 만든다.
pub fn synthesize_added_diff(path: &str, content: &str) -> String {
    let mut out = format!("--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n", content.lines().count());
    for line in content.lines() {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn is_binary_diff(diff: &str) -> bool {
    diff.lines().any(|l| l.starts_with("Binary files ") || l.starts_with("GIT binary patch"))
}

/// git 이 공백 등이 든 경로를 따옴표로 감싼 경우
fn unquote(path: &str) -> String {
    if path.len() >= 2 && path.starts_with('"') && path.ends_with('"') {
        path[1..path.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branches_put_current_first_then_newest() {
        let text = "-\told\t100\tfirst\n*\tmain\t200\tsecond\n-\tnew\t300\tthird\n";
        let names: Vec<_> = parse_branches(text).into_iter().map(|b| (b.name, b.current)).collect();
        assert_eq!(names, vec![("main".into(), true), ("new".into(), false), ("old".into(), false)]);
    }

    #[test]
    fn log_lines_split_on_unit_separator() {
        let commits = parse_log("abc123def\u{1f}Hans\u{1f}1700000000\u{1f}Fix: a | b\n");
        assert_eq!(
            commits,
            vec![GitCommit { hash: "abc123def".into(), author: "Hans".into(), committed_at: 1.7e9, subject: "Fix: a | b".into() }]
        );
    }

    #[test]
    fn status_maps_codes_and_untracked() {
        let text = " M src/a.rs\nA  new.rs\n D gone.rs\nR  old.rs -> renamed.rs\n?? \"my file.txt\"\n!! ignored\n";
        let changes = parse_status(text);
        let summary: Vec<_> = changes.iter().map(|c| (c.path.as_str(), c.kind, c.untracked)).collect();
        assert_eq!(
            summary,
            vec![
                ("src/a.rs", GitChangeKind::Modified, false),
                ("new.rs", GitChangeKind::Added, false),
                ("gone.rs", GitChangeKind::Deleted, false),
                ("renamed.rs", GitChangeKind::Renamed, false),
                ("my file.txt", GitChangeKind::Added, true),
            ]
        );
    }

    #[test]
    fn name_status_uses_the_new_path_for_renames() {
        let changes = parse_name_status("M\ta.rs\nR090\told.rs\tnew.rs\nD\tz.rs\n");
        let paths: Vec<_> = changes.iter().map(|c| (c.path.as_str(), c.kind)).collect();
        assert_eq!(paths, vec![("a.rs", GitChangeKind::Modified), ("new.rs", GitChangeKind::Renamed), ("z.rs", GitChangeKind::Deleted)]);
    }

    #[test]
    fn synthesized_diff_marks_every_line_added() {
        assert_eq!(synthesize_added_diff("n.txt", "a\nb"), "--- /dev/null\n+++ b/n.txt\n@@ -0,0 +1,2 @@\n+a\n+b\n");
        assert!(is_binary_diff("diff --git a/x b/x\nBinary files a/x and b/x differ\n"));
    }
}
