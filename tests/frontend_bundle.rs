//! Source bundle framing is independent from production module interpretation.
pub use lkjscript::source;
#[path = "../src/frontend_bundle.rs"]
mod frontend_bundle;

use lkjscript::frontend;
use source::{SourceBundle, SourceInput};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn append_text(bytes: &mut Vec<u8>, text: &[u8]) {
    bytes.extend_from_slice(&(text.len() as u64).to_le_bytes());
    bytes.extend_from_slice(text);
}

fn wire(root: &[u8], library: Option<&[u8]>, files: &[(&[u8], &[u8])]) -> Vec<u8> {
    let mut bytes = b"LKJSB1\n".to_vec();
    append_text(&mut bytes, root);
    if let Some(library) = library {
        bytes.push(1);
        append_text(&mut bytes, library);
    } else {
        bytes.push(0);
    }
    bytes.extend_from_slice(&(files.len() as u64).to_le_bytes());
    for (path, text) in files {
        append_text(&mut bytes, path);
        append_text(&mut bytes, text);
    }
    bytes
}

fn problem(bytes: &[u8]) -> String {
    match frontend_bundle::decode(bytes) {
        Ok(_) => panic!("malformed bundle was accepted"),
        Err(problem) => problem,
    }
}

#[test]
fn exact_utf8_source_and_logical_paths_round_trip() {
    let root = "src/main.rs";
    let text = "\u{feff}// 水 😀\r\npub fn probe()->i64 { 7 }\0";
    let bytes = wire(
        root.as_bytes(),
        Some(b"library/lib.rs"),
        &[(root.as_bytes(), text.as_bytes()), (b"library/lib.rs", b"")],
    );
    let bundle = frontend_bundle::decode(&bytes).unwrap();
    assert_eq!(bundle.root, root);
    assert_eq!(bundle.library.as_deref(), Some("library/lib.rs"));
    assert_eq!(bundle.files.len(), 2);
    assert_eq!(bundle.files[0].path, root);
    assert_eq!(bundle.files[0].text, text);
    assert_eq!(bundle.files[1].path, "library/lib.rs");
    assert_eq!(bundle.files[1].text, "");
}

#[test]
fn every_truncated_prefix_is_rejected() {
    let valid = wire(
        b"src/main.rs",
        Some(b"library/lib.rs"),
        &[(b"src/main.rs", b"fn probe(){}")],
    );
    for length in 0..valid.len() {
        assert!(
            frontend_bundle::decode(&valid[..length]).is_err(),
            "prefix {length}"
        );
    }
    assert!(frontend_bundle::decode(&valid).is_ok());
}

#[test]
fn invalid_utf8_is_rejected_in_each_text_field() {
    let invalid = &[0xc0, 0x80];
    for bytes in [
        wire(invalid, None, &[]),
        wire(b"main.rs", Some(invalid), &[]),
        wire(b"main.rs", None, &[(invalid, b"")]),
        wire(b"main.rs", None, &[(b"main.rs", invalid)]),
        wire(b"main.rs", None, &[(b"main.rs", &[0xe2, 0x82])]),
    ] {
        assert!(problem(&bytes).contains("UTF-8"));
    }
}

#[test]
fn malformed_header_tag_lengths_count_and_trailing_data_are_rejected() {
    let valid = wire(b"main.rs", None, &[]);
    let mut header = valid.clone();
    header[0] = b'X';
    assert!(problem(&header).contains("header"));
    let mut tag = valid.clone();
    tag[7 + 8 + b"main.rs".len()] = 2;
    assert!(problem(&tag).contains("library tag"));
    let mut length = valid.clone();
    length[7..15].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(frontend_bundle::decode(&length).is_err());
    let mut count = valid.clone();
    let offset = count.len() - 8;
    count[offset..].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(frontend_bundle::decode(&count).is_err());
    let mut trailing = valid;
    trailing.extend_from_slice(&[0xff, 0xfe]);
    assert!(problem(&trailing).contains("trailing"));
}

#[test]
fn decoder_leaves_duplicate_and_missing_module_rules_to_production_loader() {
    let duplicates = wire(
        b"main.rs",
        None,
        &[(b"main.rs", b"fn probe(){}"), (b"main.rs", b"fn probe(){}")],
    );
    let bundle = frontend_bundle::decode(&duplicates).unwrap();
    let error = match source::parse_bundle(&bundle) {
        Ok(_) => panic!("duplicate source accepted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "E_SOURCE_DUPLICATE");
    let missing = frontend_bundle::decode(&wire(b"main.rs", None, &[])).unwrap();
    let error = match source::parse_bundle(&missing) {
        Ok(_) => panic!("missing root accepted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "E_MODULE_MISSING");
}

#[test]
fn bundle_boundary_preserves_k_bytes_and_child_diagnostics() {
    let root = b"mod child; pub fn probe()->i64 { child::answer() }";
    let child = b"pub fn answer()->i64 { 7 }";
    let decoded = frontend_bundle::decode(&wire(
        b"src/main.rs",
        None,
        &[(b"src/main.rs", root), (b"src/child.rs", child)],
    ))
    .unwrap();
    let direct = SourceBundle {
        root: "src/main.rs".to_string(),
        library: None,
        files: vec![
            SourceInput {
                path: "src/main.rs".to_string(),
                text: String::from_utf8(root.to_vec()).unwrap(),
            },
            SourceInput {
                path: "src/child.rs".to_string(),
                text: String::from_utf8(child.to_vec()).unwrap(),
            },
        ],
    };
    assert_eq!(
        frontend::source_to_k(&decoded).unwrap(),
        frontend::source_to_k(&direct).unwrap()
    );
    let invalid = frontend_bundle::decode(&wire(
        b"src/main.rs",
        None,
        &[
            (b"src/main.rs", root),
            (b"src/child.rs", b"pub fn answer()->i64 { true }"),
        ],
    ))
    .unwrap();
    let error = frontend::source_to_k(&invalid).unwrap_err();
    assert_eq!(error.code, "E_TYPE");
    assert_eq!(error.path, "src/child.rs");
    assert_ne!(error.line, 0);
}

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Workspace(std::path::PathBuf);
impl Workspace {
    fn new() -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/frontend-bundle")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn node_packer_interoperates_and_records_exact_inputs() {
    let workspace = Workspace::new();
    let input = workspace.0.join("input.rs");
    let output = workspace.0.join("source.bundle");
    let spec = workspace.0.join("inputs.json");
    let text = b"\xef\xbb\xbf// exact\r\npub fn probe()->i64 { 9 }\n";
    std::fs::write(&input, text).unwrap();
    let mut specification = b"{\"root\":\"src/main.rs\",\"library\":null,\"files\":[{\"path\":\"src/main.rs\",\"file\":".to_vec();
    lkjscript::json::write_string(&mut specification, input.to_str().unwrap());
    specification.extend_from_slice(b"}]}");
    std::fs::write(&spec, specification).unwrap();
    let result = Command::new("node")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/source-bundle.mjs"))
        .arg("--inputs")
        .arg(&spec)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&output).unwrap();
    let bundle = frontend_bundle::decode(&bytes).unwrap();
    assert_eq!(bundle.root, "src/main.rs");
    assert!(bundle.library.is_none());
    assert_eq!(bundle.files[0].path, "src/main.rs");
    assert_eq!(bundle.files[0].text.as_bytes(), text);
    let manifest = std::fs::read_to_string(format!("{}.json", output.display())).unwrap();
    assert!(manifest.contains("d12f191aefe8340ed0d68de3374966719d9f27fb85ad58617a24ce55cc6f0f6b"));
    assert!(manifest.contains("\"path\": \"src/main.rs\""));
    // Exercise the packer validation before any output file is written.
    std::fs::write(&input, [0xff]).unwrap();
    let rejected = workspace.0.join("invalid.bundle");
    let result = Command::new("node")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/source-bundle.mjs"))
        .arg("--inputs")
        .arg(&spec)
        .arg("--output")
        .arg(&rejected)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("UTF-8"));
    assert!(!rejected.exists());
}

fn pack_to(
    input: &std::path::Path,
    output: &std::path::Path,
    manifest: &std::path::Path,
) -> std::process::Output {
    Command::new("node")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/source-bundle.mjs"))
        .arg("--root")
        .arg(input)
        .arg("--file")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--manifest")
        .arg(manifest)
        .output()
        .unwrap()
}

#[test]
fn node_packer_preserves_existing_artifacts_and_removes_incomplete_pair() {
    for existing_manifest in [false, true] {
        let workspace = Workspace::new();
        let input = workspace.0.join("input.rs");
        let output = workspace.0.join("source.bundle");
        let manifest = workspace.0.join("record.json");
        let source = b"pub fn probe()->i64 { 9 }\n";
        let sentinel = b"keep this existing file\n";
        std::fs::write(&input, source).unwrap();
        let (existing, absent) = if existing_manifest {
            (&manifest, &output)
        } else {
            (&output, &manifest)
        };
        std::fs::write(existing, sentinel).unwrap();
        let result = pack_to(&input, &output, &manifest);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("EEXIST"));
        assert_eq!(std::fs::read(&input).unwrap(), source);
        assert_eq!(std::fs::read(existing).unwrap(), sentinel);
        assert!(
            !absent.exists(),
            "failed invocation left {}",
            absent.display()
        );
    }
}

#[cfg(unix)]
#[test]
fn node_packer_preserves_inputs_aliased_by_either_artifact() {
    for hardlink in [false, true] {
        for manifest_alias in [false, true] {
            let workspace = Workspace::new();
            let input = workspace.0.join("input.rs");
            let output = workspace.0.join("source.bundle");
            let manifest = workspace.0.join("record.json");
            let source = b"pub fn probe()->i64 { 9 }\n";
            std::fs::write(&input, source).unwrap();
            let (alias, absent) = if manifest_alias {
                (&manifest, &output)
            } else {
                (&output, &manifest)
            };
            if hardlink {
                std::fs::hard_link(&input, alias).unwrap();
            } else {
                std::os::unix::fs::symlink(&input, alias).unwrap();
            }
            let result = pack_to(&input, &output, &manifest);
            assert!(!result.status.success());
            assert!(String::from_utf8_lossy(&result.stderr).contains("EEXIST"));
            assert_eq!(std::fs::read(&input).unwrap(), source);
            assert_eq!(std::fs::read(alias).unwrap(), source);
            assert!(
                !absent.exists(),
                "failed invocation left {}",
                absent.display()
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn node_packer_runs_through_symlink_entry() {
    let workspace = Workspace::new();
    let direct = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/source-bundle.mjs");
    let alias = workspace.0.join("packer.mjs");
    std::os::unix::fs::symlink(&direct, &alias).unwrap();
    let invoke = |script: &std::path::Path| {
        Command::new("node")
            .arg(script)
            .arg("--help")
            .output()
            .unwrap()
    };
    let expected = invoke(&direct);
    let actual = invoke(&alias);
    assert!(expected.status.success());
    assert!(!expected.stdout.is_empty());
    assert!(actual.status.success());
    assert_eq!(actual.stdout, expected.stdout);
    assert_eq!(actual.stderr, expected.stderr);
}
