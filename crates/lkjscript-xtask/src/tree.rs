//! Directory fan-out gate: at most N visible children per directory.

use std::fs;
use std::path::Path;

pub fn check_tree(root: &Path, max_dir_children: u32) -> i32 {
    let mut bad = 0;
    check_dir(root, max_dir_children, &mut bad);
    if bad > 0 {
        1
    } else {
        0
    }
}

fn check_dir(dir: &Path, max: u32, bad: &mut i32) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut kids: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    kids.sort();
    let visible: Vec<_> = kids
        .iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.starts_with('.') && name != "target" && name != "LICENSE"
        })
        .collect();
    if visible.len() as u32 > max {
        eprintln!(
            "{} has {} children (max {max}); split into subdirectories",
            dir.display(),
            visible.len()
        );
        *bad += 1;
    }
    for p in visible {
        if p.is_dir() {
            check_dir(p, max, bad);
        }
    }
}
