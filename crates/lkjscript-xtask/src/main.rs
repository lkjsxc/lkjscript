//! Verification gates for the lkjscript workspace.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use lkjscript_compiler::{compile_path, validate_source, validate_source_tree};
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
            eprintln!(
                "usage: lkjscript-xtask [check-docs|check-tree|check-sources|quiet ...]"
            );
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
    ];
    for relative in required {
        if !root.join(relative).is_file() {
            eprintln!("missing {relative}");
            return 1;
        }
    }
    if root.join("docs/language/lkjml.md").exists() {
        eprintln!("superseded active documentation path: docs/language/lkjml.md");
        return 1;
    }
    0
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

    for entry in [
        "src/examples/bench/main.lkjscript",
        "src/examples/hello/main.lkjscript",
        "src/examples/http/hello.lkjscript",
        "src/examples/mandel/main.lkjscript",
        "src/examples/lkjedit/buffer-demo.lkjscript",
        "src/examples/lkjedit/edit-mem.lkjscript",
        "src/examples/lkjedit/hello.lkjscript",
        "src/examples/lkjedit/main.lkjscript",
        "src/std/io/now-ms.lkjscript",
        "src/std/io/wait.lkjscript",
        "src/std/term/write-str.lkjscript",
    ] {
        if let Err(error) = compile_path(&root.join(entry), &limits) {
            eprintln!("{entry}: {error}");
            failures += 1;
        }
    }

    i32::from(failures > 0)
}

fn quiet_test(root: &Path) -> i32 {
    let status = Command::new("cargo")
        .args(["test", "--workspace", "--quiet", "--locked"])
        .current_dir(root)
        .status();
    match status {
        Ok(status) if status.success() => 0,
        Ok(status) => {
            eprintln!("cargo test exited with {status}");
            1
        }
        Err(error) => {
            eprintln!("run cargo test: {error}");
            1
        }
    }
}

fn quiet_verify(root: &Path) -> i32 {
    if check_docs(root) != 0
        || check_tree(root) != 0
        || check_sources(root) != 0
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
