use std::fs;
use std::path::{Path, PathBuf};

use crate::util::walk;

pub(super) fn check(root: &Path) -> usize {
    let mut files = vec![root.join("AGENTS.md"), root.join("README.md")];
    for directory in ["crates", "docs", "examples", "meta", "src"] {
        let path = root.join(directory);
        if !path.exists() {
            continue;
        }
        if let Err(error) = walk(&path, &mut files) {
            eprintln!("{error}");
            return 1;
        }
    }
    files.sort();
    files
        .into_iter()
        .filter(|path| current_owned(root, path))
        .map(|path| inspect(&path))
        .sum()
}

fn current_owned(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative.starts_with("docs/history")
        || relative.starts_with("docs/vision/experiments")
        || relative.starts_with("meta/benchmarks/jit/results")
        || relative.starts_with("meta/results")
        || relative == Path::new("meta/rustfmt.toml")
        || relative == Path::new("crates/lkjscript-xtask/src/documentation/generations.rs")
    {
        return false;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("json" | "lkjscript" | "md" | "py" | "rs" | "sh" | "toml")
    )
}

fn inspect(path: &PathBuf) -> usize {
    let Ok(content) = fs::read_to_string(path) else {
        return 0;
    };
    let mut failures = 0;
    for (index, line) in content.lines().enumerate() {
        if generation_name(line) {
            eprintln!(
                "LKJ-DOC-GENERATION numbered Current contract at {}:{}",
                path.display(),
                index + 1
            );
            failures += 1;
        }
    }
    failures
}

fn generation_name(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.contains("sqlite3_")
        || lower.contains("meta/results/")
        || (lower.contains("cargo ") && lower.contains("lockfile v"))
    {
        return false;
    }
    let words: Vec<_> = lower
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .collect();
    words.iter().enumerate().any(|(index, word)| {
        let next = words.get(index + 1).copied().unwrap_or("");
        (word.starts_with('v') && numeric_generation(word))
            || (word.starts_with("edition")
                && ((word.len() > "edition".len()
                    && word
                        .trim_start_matches("edition")
                        .bytes()
                        .all(|byte| byte.is_ascii_digit()))
                    || (*word == "edition" && numeric_generation(next))))
            || (matches!(*word, "abi" | "profile" | "protocol" | "schema")
                && next.starts_with('v')
                && numeric_generation(next))
    }) || numbered_category(&lower)
        || owned_schema_generation(&lower)
        || numbered_version(&lower)
}

fn numbered_category(value: &str) -> bool {
    ["abi", "metrics", "profile", "protocol", "schema"]
        .into_iter()
        .any(|name| {
            [' ', '-', '_'].into_iter().any(|separator| {
                let prefix = format!("{name}{separator}");
                value.match_indices(&prefix).any(|(index, _)| {
                    value
                        .as_bytes()
                        .get(index + prefix.len())
                        .is_some_and(|byte| {
                            byte.is_ascii_digit()
                                || (*byte == b'v'
                                    && value
                                        .as_bytes()
                                        .get(index + prefix.len() + 1)
                                        .is_some_and(u8::is_ascii_digit))
                        })
                })
            })
        })
}

fn numbered_version(value: &str) -> bool {
    if ![
        "abi",
        "envelope",
        "identity",
        "profile",
        "protocol",
        "schema",
        "semantic source",
    ]
    .iter()
    .any(|category| value.contains(category))
    {
        return false;
    }
    let words: Vec<_> = value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words
        .windows(2)
        .any(|pair| pair[0] == "version" && pair[1].bytes().all(|byte| byte.is_ascii_digit()))
}

fn owned_schema_generation(value: &str) -> bool {
    let Some((_, owned)) = value.split_once("lkjscript.") else {
        return false;
    };
    owned.as_bytes().windows(2).any(|pair| {
        (pair[0] == b'/' && pair[1].is_ascii_digit())
            || (pair[0] == b'v' && pair[1].is_ascii_digit() && owned.contains(".v"))
    })
}

fn numeric_generation(value: &str) -> bool {
    let digits = value.strip_prefix('v').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::generation_name;

    #[test]
    fn rejects_numbered_contract_names_only() {
        assert!(generation_name("Edition 2 source"));
        assert!(generation_name("lkjscript.semantic-source.v3"));
        assert!(generation_name("native ABI v4"));
        assert!(generation_name("the old V2 output"));
        assert!(generation_name("semantic source schema version 2"));
        assert!(!generation_name("schema uses a full contract digest"));
        assert!(!generation_name("sqlite3_prepare_v2"));
    }
}
