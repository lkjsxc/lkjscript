use std::collections::HashSet;
use std::fs;
use std::path::Path;

use lkjscript_compiler::{compile_path_with_sources, validate_source, validate_source_tree};
use lkjscript_core::Limits;

use crate::util::walk;

pub fn check_tree(root: &Path) -> i32 {
    match validate_source_tree(&root.join("src"), &Limits::default()) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

pub fn check_sources(root: &Path) -> i32 {
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
        match path.extension().and_then(|value| value.to_str()) {
            Some("lkjml") => {
                eprintln!("superseded source extension: {}", path.display());
                failures += 1;
            }
            Some("lkjscript") => failures += validate(path, root, &limits),
            _ => {}
        }
    }
    let entries = [
        "src/examples/bench/main.lkjscript",
        "src/examples/brainfuck/main.lkjscript",
        "src/examples/bulk-bytes/main.lkjscript",
        "src/examples/durable-files/main.lkjscript",
        "src/examples/sha256/main.lkjscript",
        "src/examples/sqlite/main.lkjscript",
        "src/examples/hello/main.lkjscript",
        "src/examples/http/hello.lkjscript",
        "src/examples/jit-scalar/main.lkjscript",
        "src/examples/jit-optimizing/main.lkjscript",
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
    for path in files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("lkjscript"))
    {
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

fn validate(path: &Path, root: &Path, limits: &Limits) -> usize {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("read {}: {error}", path.display());
            return 1;
        }
    };
    let Some(label) = path.strip_prefix(root).ok().and_then(Path::to_str) else {
        eprintln!("source path is not repository-relative UTF-8: {path:?}");
        return 1;
    };
    if let Err(error) = validate_source(&source, label, limits) {
        eprintln!("{error}");
        1
    } else {
        0
    }
}
