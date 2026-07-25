use std::fs;
use std::path::Path;

use crate::util::walk;

pub fn check(root: &Path) -> i32 {
    let required = [
        "AGENTS.md",
        "docs/README.md",
        "docs/current-state.md",
        "docs/vision/README.md",
        "docs/vision/experiments.md",
        "docs/vision/process-supervisor.md",
        "docs/language/README.md",
        "docs/language/source-format.md",
        "docs/runtime/README.md",
        "docs/operations/README.md",
        "docs/operations/architecture.md",
        "docs/operations/verification.md",
        "docs/product/README.md",
        "docs/decisions/README.md",
        "docs/decisions/platform/ai-native-platform.md",
        "docs/decisions/platform/bounded-repository-topology.md",
        "docs/decisions/platform/repository-intelligence-graph.md",
        "docs/decisions/platform/agent-work-state.md",
        "docs/decisions/platform/semantic-source-and-agent-protocol.md",
        "docs/decisions/platform/resource-budget-profiles.md",
        "docs/decisions/execution/compiler-pipeline.md",
        "docs/decisions/execution/execution-portfolio.md",
        "docs/decisions/execution/linux-x86-64-native-backend.md",
        "docs/decisions/semantics/equality-families.md",
        "docs/decisions/semantics/immutable-nominal-products.md",
        "docs/decisions/semantics/numeric-semantics.md",
        "docs/decisions/semantics/semantic-core.md",
        "docs/decisions/jit/callable-baseline-jit.md",
        "docs/decisions/jit/runtime-jit-instead-of-offline-pgo.md",
        "docs/decisions/capabilities/sha256.md",
        "docs/decisions/capabilities/sqlite-capabilities.md",
        "docs/vision/performance-scorecard.md",
    ];
    let mut failures = required
        .iter()
        .filter(|path| !root.join(path).is_file())
        .count();
    for path in required {
        if !root.join(path).is_file() {
            eprintln!("missing {path}");
        }
    }
    if root.join("docs/language/lkjml.md").exists() {
        eprintln!("superseded active documentation path: docs/language/lkjml.md");
        failures += 1;
    }
    failures += markdown(root);
    failures += inert_markers(root);
    i32::from(failures > 0)
}

fn markdown(root: &Path) -> usize {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        files.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
            path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("md")
        }));
    }
    let mut docs = Vec::new();
    if let Err(error) = walk(&root.join("docs"), &mut docs) {
        eprintln!("{error}");
        return 1;
    }
    files.extend(
        docs.into_iter()
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md")),
    );
    files.sort();
    let mut failures = 0;
    for path in files {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("read Markdown {}: {error}", path.display());
                failures += 1;
                continue;
            }
        };
        if path.starts_with(root.join("docs")) && !content.contains("\n## Status\n") {
            eprintln!("documentation status missing: {}", path.display());
            failures += 1;
        }
        for target in targets(&content) {
            if target.starts_with('#') || target.contains("://") || target.starts_with("mailto:") {
                continue;
            }
            let local = target.split('#').next().unwrap_or("");
            if !local.is_empty() && !path.parent().unwrap_or(root).join(local).exists() {
                eprintln!("broken local Markdown link in {}: {target}", path.display());
                failures += 1;
            }
        }
    }
    failures
}

fn targets(content: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut remaining = content;
    while let Some(start) = remaining.find("](") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find(')') else {
            break;
        };
        let target = remaining[..end].trim().trim_matches(['<', '>']);
        if !target.is_empty() {
            result.push(target);
        }
        remaining = &remaining[end + 1..];
    }
    result
}

fn inert_markers(root: &Path) -> usize {
    const WORD: &str = concat!("place", "holder");
    let mut files = Vec::new();
    for directory in ["crates", "src"] {
        if let Err(error) = walk(&root.join(directory), &mut files) {
            eprintln!("{error}");
            return 1;
        }
    }
    let mut failures = 0;
    for path in files {
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("rs" | "lkjscript")
        ) {
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("read inert-marker input {}: {error}", path.display());
                failures += 1;
                continue;
            }
        };
        for (index, line) in content.lines().enumerate() {
            if line.to_ascii_lowercase().contains(WORD) && !line.contains("PLACEHOLDER") {
                eprintln!("unlabeled inert marker at {}:{}", path.display(), index + 1);
                failures += 1;
            }
        }
    }
    failures
}
