//! Quiet verification gates for the lkj workspace.

mod tree;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use lkjscript2026_compiler::{compile_path, validate_source};
use lkjscript2026_core::Limits;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = PathBuf::from(".");
    let code = match args.first().map(String::as_str) {
        Some("check-docs") => check_docs(&root),
        Some("check-tree") => tree::check_tree(&root, 8),
        Some("check-sources") => check_sources(&root),
        Some("quiet") => match args.get(1).map(String::as_str) {
            Some("test") => quiet_test(&root),
            Some("verify") => quiet_verify(&root),
            _ => {
                eprintln!("usage: lkjscript2026-xtask quiet [test|verify]");
                2
            }
        },
        _ => {
            eprintln!(
                "usage: lkjscript2026-xtask [check-docs|check-tree|check-sources|quiet …]"
            );
            2
        }
    };
    ExitCode::from(code as u8)
}

fn check_docs(root: &Path) -> i32 {
    let required = [
        "docs/README.md",
        "docs/current-state.md",
        "docs/vision/README.md",
        "docs/vision/process-supervisor.md",
        "docs/language/README.md",
        "docs/language/lkjml.md",
        "docs/runtime/README.md",
        "docs/operations/README.md",
        "docs/product/README.md",
        "docs/decisions/README.md",
    ];
    for r in required {
        if !root.join(r).is_file() {
            eprintln!("missing {r}");
            return 1;
        }
    }
    0
}

fn check_sources(root: &Path) -> i32 {
    let mut files = Vec::new();
    walk(&root.join("src"), &mut |path| files.push(path.to_path_buf()));
    files.sort();
    let mut bad = 0;
    if root.join("examples").exists() {
        eprintln!("legacy examples/ directory; move examples under src/examples/");
        bad += 1;
    }
    let limits = Limits::default();
    for path in files {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("lkjscript") => {
                eprintln!("legacy source extension: {}", path.display());
                bad += 1;
            }
            Some("lkjml") => match fs::read_to_string(&path) {
                Ok(source) => {
                    let label = path.display().to_string();
                    if let Err(error) = validate_source(&source, &label, &limits) {
                        eprintln!("{error}");
                        bad += 1;
                    }
                }
                Err(error) => {
                    eprintln!("read {}: {error}", path.display());
                    bad += 1;
                }
            },
            _ => {}
        }
    }
    for entry in [
        "src/examples/bench/main.lkjml",
        "src/examples/hello/main.lkjml",
        "src/examples/http/hello.lkjml",
        "src/examples/mandel/main.lkjml",
        "src/examples/lkjedit/buffer-demo.lkjml",
        "src/examples/lkjedit/edit-mem.lkjml",
        "src/examples/lkjedit/hello.lkjml",
        "src/examples/lkjedit/main.lkjml",
        "src/examples/lkjedit/vimlike.lkjml",
    ] {
        if let Err(error) = compile_path(&root.join(entry), &limits) {
            eprintln!("{entry}: {error}");
            bad += 1;
        }
    }
    if bad > 0 { 1 } else { 0 }
}

fn quiet_test(root: &Path) -> i32 {
    let status = Command::new("cargo")
        .args(["test", "--workspace", "--quiet"])
        .current_dir(root)
        .status();
    match status {
        Ok(s) if s.success() => 0,
        _ => 1,
    }
}

fn quiet_verify(root: &Path) -> i32 {
    if check_docs(root) != 0 {
        return 1;
    }
    if tree::check_tree(root, 8) != 0 {
        return 1;
    }
    if check_sources(root) != 0 {
        return 1;
    }
    if quiet_test(root) != 0 {
        return 1;
    }
    0
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            walk(&p, f);
        } else {
            f(&p);
        }
    }
}
