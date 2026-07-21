//! Quiet verification gates for the lkj workspace.

mod tree;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = PathBuf::from(".");
    let code = match args.first().map(String::as_str) {
        Some("check-lines") => check_lines(&root),
        Some("check-docs") => check_docs(&root),
        Some("check-tree") => tree::check_tree(&root, 8),
        Some("quiet") => match args.get(1).map(String::as_str) {
            Some("test") => quiet_test(&root),
            Some("verify") => quiet_verify(&root),
            _ => {
                eprintln!("usage: lkjscript2026-xtask quiet [test|verify]");
                2
            }
        },
        _ => {
            eprintln!("usage: lkjscript2026-xtask [check-lines|check-docs|check-tree|quiet …]");
            2
        }
    };
    ExitCode::from(code as u8)
}

fn check_lines(root: &Path) -> i32 {
    let mut bad = 0;
    for dir in ["crates", "docs"] {
        let p = root.join(dir);
        if !p.exists() {
            continue;
        }
        walk(&p, &mut |path| {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "rs" | "md") {
                return;
            }
            if let Ok(text) = fs::read_to_string(path) {
                let lines = text.lines().count();
                if lines > 200 {
                    eprintln!("{} has {lines} lines (max 200)", path.display());
                    bad += 1;
                }
            }
        });
    }
    if bad > 0 {
        1
    } else {
        0
    }
}

fn check_docs(root: &Path) -> i32 {
    let required = [
        "docs/README.md",
        "docs/current-state.md",
        "docs/vision/README.md",
        "docs/language/README.md",
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
