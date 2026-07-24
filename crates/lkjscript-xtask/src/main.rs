//! Verification gates for the lkjscript workspace.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use lkjscript_compiler::{compile_path_with_sources, validate_source, validate_source_tree};
use lkjscript_core::Limits;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = PathBuf::from(".");
    let code = match args.first().map(String::as_str) {
        Some("check-docs") => check_docs(&root),
        Some("check-tree") => check_tree(&root),
        Some("check-sources") => check_sources(&root),
        Some("quiet") => match args.get(1).map(String::as_str) {
            Some("test") => quiet_test(&root),
            Some("verify") => quiet_verify(&root),
            _ => {
                eprintln!("usage: lkjscript-xtask quiet [test|verify]");
                2
            }
        },
        _ => {
            eprintln!("usage: lkjscript-xtask [check-docs|check-tree|check-sources|quiet ...]");
            2
        }
    };
    ExitCode::from(code as u8)
}

fn check_docs(root: &Path) -> i32 {
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
        "docs/decisions/callable-baseline-jit.md",
        "docs/decisions/compiler-pipeline.md",
        "docs/decisions/equality-families.md",
        "docs/decisions/immutable-nominal-products.md",
        "docs/decisions/linux-x86-64-native-backend.md",
        "docs/decisions/runtime-jit-instead-of-offline-pgo.md",
        "docs/decisions/numeric-semantics.md",
        "docs/decisions/semantic-core.md",
        "docs/vision/performance-scorecard.md",
    ];
    let mut failures = 0;
    for relative in required {
        if !root.join(relative).is_file() {
            eprintln!("missing {relative}");
            failures += 1;
        }
    }
    if root.join("docs/language/lkjml.md").exists() {
        eprintln!("superseded active documentation path: docs/language/lkjml.md");
        failures += 1;
    }
    failures += check_markdown(root);
    failures += check_explicit_inert_markers(root);
    i32::from(failures > 0)
}

fn check_markdown(root: &Path) -> usize {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("md")
            {
                files.push(path);
            }
        }
    }
    let mut docs = Vec::new();
    if let Err(error) = walk(&root.join("docs"), &mut docs) {
        eprintln!("{error}");
        return 1;
    }
    docs.retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"));
    files.extend(docs.iter().cloned());
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
        for target in markdown_targets(&content) {
            if target.starts_with('#') || target.contains("://") || target.starts_with("mailto:") {
                continue;
            }
            let local = target.split('#').next().unwrap_or("");
            if local.is_empty() {
                continue;
            }
            let destination = path.parent().unwrap_or(root).join(local);
            if !destination.exists() {
                eprintln!("broken local Markdown link in {}: {target}", path.display());
                failures += 1;
            }
        }
    }
    failures
}

fn markdown_targets(content: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remaining = content;
    while let Some(start) = remaining.find("](") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find(')') else {
            break;
        };
        let target = remaining[..end].trim().trim_matches(['<', '>']);
        if !target.is_empty() {
            targets.push(target);
        }
        remaining = &remaining[end + 1..];
    }
    targets
}

fn check_explicit_inert_markers(root: &Path) -> usize {
    const PLACEHOLDER_WORD: &str = concat!("place", "holder");

    let mut files = Vec::new();
    for directory in ["crates", "src"] {
        if let Err(error) = walk(&root.join(directory), &mut files) {
            eprintln!("{error}");
            return 1;
        }
    }
    let mut failures = 0;
    for path in files {
        let extension = path.extension().and_then(|extension| extension.to_str());
        if !matches!(extension, Some("rs" | "lkjscript")) {
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
            if line.to_ascii_lowercase().contains(PLACEHOLDER_WORD) && !line.contains("PLACEHOLDER")
            {
                eprintln!("unlabeled inert marker at {}:{}", path.display(), index + 1);
                failures += 1;
            }
        }
    }
    failures
}

fn check_tree(root: &Path) -> i32 {
    let limits = Limits::default();
    match validate_source_tree(&root.join("src"), &limits) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn check_sources(root: &Path) -> i32 {
    let mut files = Vec::new();
    if let Err(error) = walk(&root.join("src"), &mut files) {
        eprintln!("{error}");
        return 1;
    }
    files.sort();

    let mut failures = 0;
    if root.join("examples").exists() {
        eprintln!("obsolete examples/ directory; use src/examples/");
        failures += 1;
    }
    let limits = Limits::default();
    for path in &files {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("lkjml") => {
                eprintln!("superseded source extension: {}", path.display());
                failures += 1;
            }
            Some("lkjscript") => match fs::read_to_string(path) {
                Ok(source) => {
                    let label = path.display().to_string();
                    if let Err(error) = validate_source(&source, &label, &limits) {
                        eprintln!("{error}");
                        failures += 1;
                    }
                }
                Err(error) => {
                    eprintln!("read {}: {error}", path.display());
                    failures += 1;
                }
            },
            _ => {}
        }
    }

    let entries = [
        "src/examples/bench/main.lkjscript",
        "src/examples/brainfuck/main.lkjscript",
        "src/examples/hello/main.lkjscript",
        "src/examples/http/hello.lkjscript",
        "src/examples/jit-scalar/main.lkjscript",
        "src/examples/mandel/main.lkjscript",
        "src/examples/lkjedit/buffer-demo.lkjscript",
        "src/examples/lkjedit/edit-mem.lkjscript",
        "src/examples/lkjedit/hello.lkjscript",
        "src/examples/lkjedit/main.lkjscript",
    ];
    let mut covered = HashSet::new();
    for entry in entries {
        match compile_path_with_sources(&root.join(entry), &limits) {
            Ok((_, sources)) => covered.extend(sources),
            Err(error) => {
                eprintln!("{entry}: {error}");
                failures += 1;
            }
        }
    }

    let mut expected = HashSet::new();
    for path in files.iter().filter(|path| {
        path.extension().and_then(|extension| extension.to_str()) == Some("lkjscript")
    }) {
        match path.canonicalize() {
            Ok(path) => {
                expected.insert(path);
            }
            Err(error) => {
                eprintln!("canonicalize source {}: {error}", path.display());
                failures += 1;
            }
        }
    }
    let mut missing: Vec<_> = expected.difference(&covered).collect();
    missing.sort();
    for path in missing {
        eprintln!(
            "source is outside compiled entry closures: {}",
            path.display()
        );
        failures += 1;
    }
    let mut unexpected: Vec<_> = covered.difference(&expected).collect();
    unexpected.sort();
    for path in unexpected {
        eprintln!(
            "compiled source is outside canonical corpus: {}",
            path.display()
        );
        failures += 1;
    }

    i32::from(failures > 0)
}

fn quiet_test(root: &Path) -> i32 {
    run_cargo(
        root,
        &["test", "--workspace", "--quiet", "--locked"],
        "cargo test",
    )
}

fn quiet_format(root: &Path) -> i32 {
    run_cargo(root, &["fmt", "--all", "--", "--check"], "cargo fmt")
}

fn quiet_clippy(root: &Path) -> i32 {
    run_cargo(
        root,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        "cargo clippy",
    )
}

fn run_cargo(root: &Path, arguments: &[&str], label: &str) -> i32 {
    match Command::new("cargo")
        .args(arguments)
        .current_dir(root)
        .status()
    {
        Ok(status) if status.success() => 0,
        Ok(status) => {
            eprintln!("{label} exited with {status}");
            1
        }
        Err(error) => {
            eprintln!("run {label}: {error}");
            1
        }
    }
}

fn quiet_verify(root: &Path) -> i32 {
    if check_docs(root) != 0
        || check_tree(root) != 0
        || check_sources(root) != 0
        || quiet_format(root) != 0
        || quiet_clippy(root) != 0
        || quiet_test(root) != 0
    {
        return 1;
    }
    0
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|error| format!("read source directory {}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read entry in {}: {error}", dir.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if kind.is_dir() {
            walk(&path, files)?;
        } else if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
}
