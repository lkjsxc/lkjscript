use std::fs;
use std::path::Path;

use crate::util::walk;

const MAX_DIAGNOSTICS: usize = 256;

pub fn check(root: &Path) -> usize {
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        paths.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| is_markdown(path)),
        );
    }
    let mut docs = Vec::new();
    if walk(&root.join("docs"), &mut docs).is_err() {
        return 1;
    }
    paths.extend(docs.into_iter().filter(|path| is_markdown(path)));
    paths.sort();
    let mut failures = 0;
    for path in paths {
        let Ok(content) = fs::read_to_string(&path) else {
            failures += 1;
            continue;
        };
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
        let historical = relative.starts_with("docs/history/")
            && !relative.starts_with("docs/history/evidence/");
        let active = !relative.starts_with("docs/history/")
            && !relative.starts_with("docs/vision/experiments/");
        let mut in_fence = false;
        let mut historical_fence = false;
        let mut pending_historical = false;
        for (index, line) in content.lines().enumerate() {
            if line == "<!-- LKJ-EXAMPLE class=historical -->" {
                pending_historical = true;
                continue;
            }
            if line.trim_start().starts_with("```") {
                if in_fence {
                    in_fence = false;
                    historical_fence = false;
                } else {
                    in_fence = true;
                    historical_fence = pending_historical;
                    pending_historical = false;
                }
                continue;
            }
            if line.starts_with("<!-- LKJ-F ") {
                continue;
            }
            for diagnostic in diagnostics(line, active, historical, in_fence, historical_fence) {
                eprintln!(
                    "documentation coherence {relative}:{}: {diagnostic}",
                    index + 1
                );
                failures += 1;
                if failures >= MAX_DIAGNOSTICS {
                    eprintln!("documentation coherence diagnostic limit reached");
                    return failures;
                }
            }
        }
    }
    failures
}

fn is_markdown(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("md")
}

fn diagnostics(
    line: &str,
    active: bool,
    historical: bool,
    in_fence: bool,
    historical_fence: bool,
) -> Vec<&'static str> {
    let mut result = Vec::new();
    let lower = line.to_ascii_lowercase();
    if active && in_fence {
        if lower.contains("memory traced") && !historical_fence {
            result.push("removed command described as available");
        }
        return result;
    }
    if active {
        for phrase in [
            "the canonical source contract",
            "the removed legacy source contract",
            "resource resource",
        ] {
            if lower.contains(phrase) {
                result.push("registered malformed migration phrase");
            }
        }
        if repeated_word(line) {
            result.push("repeated adjacent word");
        }
        if lower.contains("still use tracing") || lower.contains("uses tracing `gcheap`") {
            result.push("retired tracing mechanism described as active");
        }
        if lower.contains("borrowed `str`")
            && lower.contains("current")
            && !lower.contains("not current")
            && !lower.contains("non-current")
        {
            result.push("borrowed str described as Current");
        }
        if lower.contains("collector-free persistent lists")
            && !lower.contains("not current")
            && !lower.contains("accepted target")
        {
            result.push("persistent lists exceed the Current interface");
        }
    }
    if historical
        && (line == "## Current"
            || line.starts_with("## Current ")
            || line.contains("**Current.**")
            || (lower.contains(" is current") && !negative_context(&lower)))
    {
        result.push("unqualified Current claim in Historical documentation");
    }
    result
}

fn negative_context(lower: &str) -> bool {
    [
        "historical",
        "not current",
        "removed",
        "reject",
        "superseded",
        "no `memory traced`",
    ]
    .iter()
    .any(|word| lower.contains(word))
}

fn repeated_word(line: &str) -> bool {
    if line.trim_start().starts_with('|') {
        return false;
    }
    let words: Vec<_> = line.split_whitespace().collect();
    words.windows(2).any(|pair| {
        let first = pair[0];
        let second = pair[1].trim_end_matches(|character: char| !character.is_alphabetic());
        first.len() > 2
            && first.chars().all(char::is_alphabetic)
            && second.chars().all(char::is_alphabetic)
            && first.eq_ignore_ascii_case(second)
    })
}

#[cfg(test)]
mod tests {
    use super::diagnostics;

    #[test]
    fn stale_no_tracing_and_removed_command_claims_fail() {
        let stale = format!(
            "VM and native still use tracing `{}`.",
            ["Gc", "Heap"].concat()
        );
        assert!(!diagnostics(&stale, true, false, false, false).is_empty());
        assert!(!diagnostics("run memory traced --json", true, false, true, false).is_empty());
        assert!(diagnostics("run memory traced --json", true, false, true, true).is_empty());
        assert!(diagnostics(
            "The memory traced command was removed.",
            true,
            false,
            false,
            false
        )
        .is_empty());
    }

    #[test]
    fn overclaims_and_migration_damage_fail() {
        assert!(!diagnostics("borrowed `str` is Current", true, false, false, false).is_empty());
        assert!(!diagnostics("the the canonical value", true, false, false, false).is_empty());
        assert!(!diagnostics("Resource resource profile", true, false, false, false).is_empty());
    }

    #[test]
    fn historical_current_requires_an_envelope() {
        assert!(!diagnostics("## Current", false, true, false, false).is_empty());
        assert!(diagnostics("## Recorded Baseline", false, true, false, false).is_empty());
    }
}
