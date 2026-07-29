use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::scanner::contains_unsafe_token;
use super::validate;

#[test]
fn scanner_finds_exact_code_tokens() {
    assert!(contains_unsafe_token("unsafe { call(); }"));
    assert!(contains_unsafe_token("unsafe fn call() {}"));
    assert!(contains_unsafe_token("unsafe impl Send for Item {}"));
    assert!(!contains_unsafe_token("safe_unsafe_name();"));
    assert!(!contains_unsafe_token("r#unsafe();"));
}

#[test]
fn scanner_ignores_comments_and_literals() {
    let source = r####"
// unsafe { ignored(); }
/* unsafe fn ignored() { /* unsafe */ } */
let ordinary = "unsafe with an escaped quote: \"";
let raw = r#"unsafe"#;
let bytes = b"unsafe";
let raw_bytes = br##"unsafe"##;
let character = 'u';
let lifetime: &'unsafe_name str = ordinary;
"####;
    assert!(!contains_unsafe_token(source));
}

#[test]
fn registry_is_bidirectionally_exact() {
    let root = fixture_root("exact");
    write_source(&root, "crates/demo/src/lib.rs", "unsafe { call(); }\n");
    write_registry(&root, &["crates/demo/src/lib.rs"]);
    assert_eq!(validate(&root), Vec::<String>::new());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn registry_rejects_unregistered_and_stale_files() {
    let root = fixture_root("mismatch");
    write_source(&root, "crates/demo/src/lib.rs", "fn safe() {}\n");
    write_source(&root, "crates/demo/src/other.rs", "unsafe { call(); }\n");
    write_registry(&root, &["crates/demo/src/lib.rs"]);
    let errors = validate(&root).join("\n");
    assert!(errors.contains("registered file has no unsafe code token"));
    assert!(errors.contains("unsafe code token is not registered: crates/demo/src/other.rs"));
    let _ = fs::remove_dir_all(root);
}

fn fixture_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "lkjscript-unsafe-check-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn write_source(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("source parent")).expect("create source parent");
    fs::write(path, source).expect("write source");
}

fn write_registry(root: &Path, files: &[&str]) {
    let path = root.join("meta/unsafe/registry.json");
    fs::create_dir_all(path.parent().expect("registry parent")).expect("create registry parent");
    let files = files
        .iter()
        .map(|file| format!("\"{file}\""))
        .collect::<Vec<_>>()
        .join(",");
    let registry = format!(
        concat!(
            "{{\"schema\":\"lkjscript.unsafe-boundary-registry\",",
            "\"boundaries\":[{{\"id\":\"test-boundary\",",
            "\"responsibility\":\"test\",\"safe_caller_contract\":\"test\",",
            "\"files\":[{files}]}}]}}"
        ),
        files = files
    );
    fs::write(path, registry).expect("write registry");
}
