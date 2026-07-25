use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::model::{Audit, Edge};

use super::{edge, read_text};
use crate::structure::graph::Budget;

pub fn add(root: &Path, audit: &Audit, edges: &mut Vec<Edge>, budget: &mut Budget) {
    let tracked: BTreeSet<_> = audit.files.iter().map(|file| file.path.as_str()).collect();
    for file in audit.files.iter().filter(|file| file.path.ends_with(".md")) {
        let Some(text) = read_text(root, &file.path, file.bytes, budget) else {
            break;
        };
        links(&file.path, &text, &tracked, edges);
    }
}

fn links(path: &str, text: &str, tracked: &BTreeSet<&str>, edges: &mut Vec<Edge>) {
    for (line_index, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            let Some(end) = rest.find(')') else { break };
            let raw = rest[..end].trim().trim_matches(['<', '>']);
            rest = &rest[end + 1..];
            let target = raw.split('#').next().unwrap_or("");
            if target.is_empty() || target.starts_with('#') || target.contains("://") {
                continue;
            }
            let Some(target) = normalize(path, target) else {
                continue;
            };
            if tracked.contains(target.as_str()) {
                let kind = if line.to_ascii_lowercase().contains("supersed") {
                    "supersedes"
                } else {
                    "documents"
                };
                edge(
                    edges,
                    &format!("file:{path}"),
                    &format!("file:{target}"),
                    kind,
                    &format!("{path}:{}", line_index + 1),
                    "declared",
                );
            }
        }
    }
}

fn normalize(source: &str, target: &str) -> Option<String> {
    let parent = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    let mut result = PathBuf::new();
    for component in parent.join(target).components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    result.to_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    #[test]
    fn target_is_lexically_normalized() {
        assert_eq!(
            super::normalize("docs/a/readme.md", "../b.md").as_deref(),
            Some("docs/b.md")
        );
    }
}
