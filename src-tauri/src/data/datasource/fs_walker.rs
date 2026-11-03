use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use crate::domain::entity::{FsKind, FsNode};

/// 아주 큰 폴더를 열어도 UI 가 멈추지 않게 노드 수를 제한한다.
const MAX_ENTRIES: usize = 50_000;

/// 폴더 우선, 이름순으로 정렬한 트리. .gitignore 와 상관없이 모두 보여주고 `.git` 만 뺀다.
/// 얕은 항목부터 읽어서, 상한에 걸려도 node_modules 같은 큰 폴더의 깊은 곳만 잘리고 다른 폴더는 남는다.
/// 심볼릭 링크·정션은 따라가지 않는다(순환 방지).
pub fn walk(root: &Path) -> FsNode {
    let mut entries: Vec<(PathBuf, bool)> = Vec::new();
    let mut queue = VecDeque::from([root.to_path_buf()]);
    'walk: while let Some(dir) = queue.pop_front() {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for entry in read.flatten() {
            if entries.len() >= MAX_ENTRIES {
                break 'walk;
            }
            if entry.file_name() == ".git" {
                continue;
            }
            let is_dir = entry.file_type().map_or(false, |t| t.is_dir());
            if is_dir {
                queue.push_back(entry.path());
            }
            entries.push((entry.path(), is_dir));
        }
    }

    // 깊은 것부터 부모의 자식 목록에 넣는다.
    entries.sort_by_key(|(path, _)| std::cmp::Reverse(path.components().count()));
    let mut children: HashMap<PathBuf, Vec<FsNode>> = HashMap::new();
    for (path, is_dir) in entries {
        let node = FsNode {
            path: path.to_string_lossy().into_owned(),
            name: file_name(&path),
            kind: if is_dir { FsKind::Dir } else { FsKind::File },
            children: is_dir.then(|| sorted(children.remove(&path).unwrap_or_default())),
        };
        if let Some(parent) = path.parent() {
            children.entry(parent.to_path_buf()).or_default().push(node);
        }
    }

    FsNode {
        path: root.to_string_lossy().into_owned(),
        name: file_name(root),
        kind: FsKind::Dir,
        children: Some(sorted(children.remove(root).unwrap_or_default())),
    }
}

fn sorted(mut nodes: Vec<FsNode>) -> Vec<FsNode> {
    nodes.sort_by(|a, b| {
        (a.kind != FsKind::Dir)
            .cmp(&(b.kind != FsKind::Dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    nodes
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| path.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(node: &FsNode) -> Vec<String> {
        node.children.as_ref().unwrap().iter().map(|c| c.name.clone()).collect()
    }

    #[test]
    fn shows_everything_except_dot_git_regardless_of_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for d in ["src/inner", "node_modules/pkg", ".git/objects", "target", "Docs", ".vs"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        for f in ["src/inner/a.rs", "src/b.rs", "README.md", "target/out.bin", "z.txt", ".git/HEAD"] {
            std::fs::write(root.join(f), "").unwrap();
        }
        std::fs::write(root.join(".gitignore"), "target/\n.vs/\nz.txt\n").unwrap();

        let tree = walk(root);
        assert_eq!(names(&tree), vec![".vs", "Docs", "node_modules", "src", "target", ".gitignore", "README.md", "z.txt"]);
        let src = &tree.children.as_ref().unwrap()[3];
        assert_eq!(names(src), vec!["inner", "b.rs"]);
        assert_eq!(names(&src.children.as_ref().unwrap()[0]), vec!["a.rs"]);
        assert_eq!(src.children.as_ref().unwrap()[1].children, None);
        let target = &tree.children.as_ref().unwrap()[4];
        assert_eq!(names(target), vec!["out.bin"]);
    }

    #[test]
    fn shallow_entries_survive_when_a_deep_folder_hits_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // 'a_deep' 가 이름순으로 먼저 오지만, 너비 우선이라 최상위 'z_last' 는 항상 남는다.
        let mut deep = root.join("a_deep");
        for i in 0..30 {
            deep = deep.join(format!("d{i}"));
        }
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(root.join("z_last.txt"), "").unwrap();
        assert!(names(&walk(root)).contains(&"z_last.txt".to_string()));
    }
}
