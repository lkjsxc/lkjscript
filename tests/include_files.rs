#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "support/native.rs"]
mod native;
use lkjscript::{ast::Diagnostic, frontend, interpreter, ir, source};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

fn compile(root: &str, files: &[(&str, &[u8])]) -> Result<ir::CheckedProgram, Diagnostic> {
    let mut inputs: BTreeMap<String, Vec<u8>> = bootstrap_library::standard_library()
        .into_iter()
        .map(|input| (input.path, input.text.into_bytes()))
        .collect();
    for (path, bytes) in files {
        inputs.insert(path.to_string(), bytes.to_vec());
    }
    let program = source::load(root, Some("$std/lib.rs"), &mut |path| {
        Ok(inputs.get(path).cloned())
    })?;
    frontend::check_source(&program)
}

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn embedded_text_and_arbitrary_bytes_match_rust_and_both_routes() {
    let text = br#"mod child;
fn data() -> &'static [u8; 4] { include_bytes!("../assets/data.bin") }
pub fn probe() -> i64 {
    let bytes: [u8; 4] = *data();
    bytes[0] as i64 + bytes[1] as i64 + bytes[2] as i64 + bytes[3] as i64
        + include_str!("../assets/message.txt").len() as i64 + child::length()
}"#;
    let files: &[(&str, &[u8])] = &[
        ("src/main.rs", text),
        (
            "src/child.rs",
            br#"pub fn length() -> i64 { include_str!("../assets/child.txt",).len() as i64 }"#,
        ),
        ("assets/data.bin", &[255, 0, 10, 27]),
        ("assets/message.txt", "π\n".as_bytes()),
        ("assets/child.txt", b"four"),
    ];
    let checked = compile("src/main.rs", files).unwrap();
    native::assert_values(&checked, &[("probe", 299)]);
    let entry = checked
        .program()
        .functions
        .iter()
        .position(|function| function.name == "probe")
        .unwrap();
    let result = interpreter::run(checked.program(), entry, &[], &mut Vec::new())
        .unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message));
    assert!(matches!(
        result.value,
        interpreter::Scalar::Int { value: 299, .. }
    ));

    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/include-files-tests")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&directory).unwrap();
    let fixture = Fixture(directory);
    for (path, bytes) in files {
        let path = fixture.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let mut root = text.to_vec();
    root.extend_from_slice(b"\nfn main() { println!(\"{}\", probe()); }\n");
    std::fs::write(fixture.0.join("src/main.rs"), root).unwrap();
    let rust = fixture.0.join("rust-probe");
    let compiled = Command::new("rustc")
        .arg("--edition=2024")
        .arg(fixture.0.join("src/main.rs"))
        .arg("-o")
        .arg(&rust)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let observed = Command::new(rust).output().unwrap();
    assert!(observed.status.success());
    assert_eq!(observed.stdout, b"299\n");
}

#[test]
fn includes_keep_the_call_site_in_diagnostics() {
    for (macro_name, bytes, code) in [
        ("include_str", Some(&[255u8][..]), "E_INCLUDE_UTF8"),
        ("include_str", None, "E_INCLUDE_MISSING"),
        ("include_bytes", None, "E_INCLUDE_MISSING"),
    ] {
        let text = format!("fn probe() {{\n  {macro_name}!(\"asset\");\n}}");
        let mut files = vec![("main.rs", text.as_bytes())];
        if let Some(bytes) = bytes {
            files.push(("asset", bytes));
        }
        let error = match compile("main.rs", &files) {
            Err(error) => error,
            Ok(_) => panic!("expected rejection"),
        };
        assert_eq!(error.code, code);
        assert_eq!(error.path, "main.rs");
        assert_eq!(error.line, 2);
        assert_eq!(error.column, 3);
    }
}

#[test]
fn include_paths_cannot_escape_the_granted_logical_directory() {
    for (root, requested) in [
        ("src/main.rs", "../../secret"),
        ("main.rs", "/etc/passwd"),
        ("$std/root.rs", "../secret"),
    ] {
        let text = format!("fn probe() {{ include_bytes!(\"{requested}\"); }}");
        let mut requested_paths = Vec::new();
        let report = source::load_report(root, None, &mut |path| {
            requested_paths.push(path.to_string());
            if path == root {
                Ok(Some(text.as_bytes().to_vec()))
            } else {
                panic!("escaped read: {path}")
            }
        });
        assert_eq!(report.diagnostics[0].code, "E_INCLUDE_PATH");
        assert_eq!(requested_paths, vec![root]);
    }
}

#[test]
fn includes_require_one_literal_path_and_do_not_read_disabled_items() {
    for expression in [
        "include_str!()",
        "include_bytes!(path)",
        "include_bytes!(\"a\", \"b\")",
        "include_str!(\"a\"; 1)",
    ] {
        let text = format!("fn probe() {{ let path = \"a\"; {expression}; }}");
        let report = source::load_report("main.rs", None, &mut |path| {
            assert_eq!(path, "main.rs");
            Ok(Some(text.as_bytes().to_vec()))
        });
        assert_eq!(report.diagnostics[0].code, "E_INCLUDE_PATH", "{expression}");
    }
    let text = b"#[cfg(test)] fn ignored() { include_bytes!(\"absent\"); } fn probe() -> i64 { 1 }";
    let report = source::load_report("main.rs", None, &mut |path| {
        assert_eq!(path, "main.rs");
        Ok(Some(text.to_vec()))
    });
    assert!(report.diagnostics.is_empty());
}

#[test]
fn include_io_errors_stay_distinct_from_missing_files() {
    let report = source::load_report("main.rs", None, &mut |path| {
        if path == "main.rs" {
            Ok(Some(
                b"fn probe() { include_bytes!(\"blocked\"); }".to_vec(),
            ))
        } else {
            Err("permission denied".to_string())
        }
    });
    assert_eq!(report.diagnostics[0].code, "E_IO");
    assert!(report.diagnostics[0].message.contains("permission denied"));
}

#[test]
fn absolute_roots_keep_module_and_embedded_file_paths() {
    for (root, child, asset, requested) in [
        ("/main.rs", "/child.rs", "/asset", "asset"),
        ("/nested/main.rs", "/nested/child.rs", "/asset", "../asset"),
    ] {
        let text = format!(
            "mod child; fn probe() -> i64 {{ child::value() + include_bytes!(\"{requested}\")[0] as i64 }}"
        );
        let mut reads = Vec::new();
        let program = source::load(root, None, &mut |path| {
            reads.push(path.to_string());
            Ok(if path == root {
                Some(text.as_bytes().to_vec())
            } else if path == child {
                Some(b"pub fn value() -> i64 { 1 }".to_vec())
            } else if path == asset {
                Some(vec![41u8])
            } else {
                None
            })
        })
        .unwrap();
        assert!(program.files.iter().any(|file| file.path == child));
        assert!(reads.iter().any(|path| path == asset));
        assert!(
            reads
                .iter()
                .all(|path| path.starts_with('/') && !path.starts_with("//"))
        );
        frontend::check_source(&program).unwrap();
    }
}
