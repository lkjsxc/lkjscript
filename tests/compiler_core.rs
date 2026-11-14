//! The compiler-core boundary is driven by declared source and library bytes.
use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn bundle(value: i64) -> SourceBundle {
    SourceBundle {
        root: "main.rs".to_string(),
        library: Some("library/lib.rs".to_string()),
        files: vec![
            SourceInput {
                path: "main.rs".to_string(),
                text: "mod child; pub fn main(mut world:World) { let mut out=world.stdout(); out.write(&[child::answer() as u8]); }".to_string(),
            },
            SourceInput {
                path: "child.rs".to_string(),
                text: "pub fn answer()->i64 { std::answer() }".to_string(),
            },
            SourceInput {
                path: "library/lib.rs".to_string(),
                text: format!("pub fn answer()->i64 {{ {value} }}"),
            },
        ],
    }
}

fn run(bytes: &[u8], expected: i64) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/compiler-core");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{}-{expected}.elf", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(&path).output().unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, vec![expected as u8]);
}

#[test]
fn declared_library_bytes_determine_the_emitted_behavior() {
    let first = compiler::compile(&bundle(41)).unwrap();
    assert_eq!(first, compiler::compile(&bundle(41)).unwrap());
    let changed = compiler::compile(&bundle(42)).unwrap();
    assert_ne!(first, changed);
    run(&first, 41);
    run(&changed, 42);
}

#[test]
fn diagnostics_name_the_original_child_file() {
    let mut input = bundle(42);
    input.files[1].text = "pub fn answer()->i64 { true }".to_string();
    let problem = compiler::compile(&input).unwrap_err();
    assert_eq!(problem.code, "E_TYPE");
    assert_eq!(problem.path, "child.rs");
}

#[test]
fn missing_library_input_is_not_replaced_with_embedded_source() {
    let mut input = bundle(42);
    input.files.pop();
    let problem = compiler::compile(&input).unwrap_err();
    assert_eq!(problem.code, "E_MODULE_MISSING");
    assert!(problem.message.contains("library/lib.rs"));
}
