use std::path::Path;
use std::process::Command;

pub fn quiet(root: &Path, args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("test") => cargo(
            root,
            &["test", "--workspace", "--quiet", "--locked"],
            "cargo test",
        ),
        Some("verify") => verify(root),
        _ => {
            eprintln!("usage: lkjscript-xtask quiet [test|verify]");
            2
        }
    }
}

fn verify(root: &Path) -> i32 {
    if crate::documentation::check(root) != 0
        || crate::unsafe_check::run(root) != 0
        || crate::source_checks::check_tree(root) != 0
        || crate::source_checks::check_sources(root) != 0
        || cargo(root, &["fmt", "--all", "--", "--check"], "cargo fmt") != 0
        || cargo(
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
        ) != 0
        || cargo(
            root,
            &["test", "--workspace", "--quiet", "--locked"],
            "cargo test",
        ) != 0
    {
        1
    } else {
        0
    }
}

fn cargo(root: &Path, args: &[&str], label: &str) -> i32 {
    match Command::new("cargo").args(args).current_dir(root).status() {
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
