#[path = "../src/frontend_bundle.rs"]
mod frontend_bundle;
#[path = "../src/frontend_input.rs"]
mod frontend_input;
pub use lkjscript::source;
use lkjscript::{frontend, k_text};
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};

fn arguments(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn source_options(root: &str) -> frontend_input::SourceOptions {
    frontend_input::SourceOptions {
        root: root.to_string(),
        directory: ".".to_string(),
        library: None,
        output: frontend_input::Output::K,
    }
}

fn load<F: FnMut(&str) -> Result<Option<String>, String>>(
    options: &frontend_input::SourceOptions,
    read: &mut F,
) -> frontend_input::Loaded {
    let mut cache = frontend_input::Cache::new();
    let standard = options.library.as_ref().map(|_| "$std/lib.rs");
    let report = source::load_report(&options.root, standard, &mut |path| cache.read(path, read));
    frontend_input::Loaded {
        report,
        bundle: cache.into_bundle(options),
    }
}

fn load_files(
    options: &frontend_input::SourceOptions,
    files: &[(&str, &str)],
) -> frontend_input::Loaded {
    load(options, &mut |path| {
        Ok(files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, text)| text.to_string()))
    })
}

#[test]
fn cli_keeps_bundle_check_and_requires_a_library_choice_for_source() {
    for flags in [
        arguments(&["input.lkjsb"]),
        arguments(&["--check", "input.lkjsb"]),
        arguments(&["input.lkjsb", "--check"]),
    ] {
        let checking = flags.len() == 2;
        match frontend_input::parse(flags).ok().unwrap() {
            frontend_input::Request::Bundle { path, check } => {
                assert_eq!(path, "input.lkjsb");
                assert_eq!(check, checking);
            }
            _ => panic!("expected bundle request"),
        }
    }
    assert!(frontend_input::parse(arguments(&["--source", "main.rs"])).is_err());
    assert!(matches!(
        frontend_input::parse(arguments(&["--help"])),
        Ok(frontend_input::Request::Help)
    ));
    assert!(frontend_input::usage().contains("--library-dir"));
}

#[test]
fn cli_preserves_logical_roots_and_explicit_directory_inputs() {
    match frontend_input::parse(arguments(&[
        "--source",
        "./app/main.rs",
        "--source-dir",
        "/project",
        "--library-dir",
        "/toolchain/library",
        "--pack",
    ]))
    .ok()
    .unwrap()
    {
        frontend_input::Request::Source(options) => {
            assert_eq!(options.root, "./app/main.rs");
            assert_eq!(options.directory, "/project");
            assert_eq!(options.library.as_deref(), Some("/toolchain/library"));
            assert!(options.output == frontend_input::Output::Pack);
        }
        _ => panic!("expected source request"),
    }
    match frontend_input::parse(arguments(&[
        "--source",
        "main.rs",
        "--no-library",
        "--check",
    ]))
    .ok()
    .unwrap()
    {
        frontend_input::Request::Source(options) => {
            assert_eq!(options.directory, ".");
            assert!(options.library.is_none());
            assert!(options.output == frontend_input::Output::Check);
        }
        _ => panic!("expected source request"),
    }
}

#[test]
fn cli_rejects_conflicting_missing_and_duplicate_options() {
    for flags in [
        vec![],
        vec!["--source"],
        vec!["--source", "", "--no-library"],
        vec!["--source", "main.rs", "--no-library", "input.lkjsb"],
        vec!["--source", "a.rs", "--source", "b.rs", "--no-library"],
        vec![
            "--source",
            "main.rs",
            "--no-library",
            "--library-dir",
            "lib",
        ],
        vec!["--source", "main.rs", "--no-library", "--check", "--pack"],
        vec![
            "--source",
            "main.rs",
            "--no-library",
            "--source-dir",
            ".",
            "--source-dir",
            "x",
        ],
        vec!["--source", "main.rs", "--no-library", "--unknown"],
        vec!["--library-dir", "lib", "input.lkjsb"],
        vec!["--pack", "input.lkjsb"],
    ] {
        assert!(
            frontend_input::parse(arguments(&flags)).is_err(),
            "accepted conflicting arguments: {flags:?}"
        );
    }
}

#[test]
fn the_production_loader_selects_nested_inline_and_cfg_modules() {
    let files = [
        (
            "app/main.rs",
            "// exact π bytes\r\nmod direct; mod nested; mod inline { mod leaf; } #[cfg(test)] mod excluded;",
        ),
        ("app/direct.rs", "pub fn value()->i64{1}\r\n"),
        ("app/nested/mod.rs", "mod inner;"),
        ("app/nested/inner.rs", "pub fn value()->i64{2}"),
        ("app/inline/leaf.rs", "pub fn value()->i64{3}"),
        ("app/excluded.rs", "this file must not be requested"),
    ];
    let mut calls = BTreeMap::<String, usize>::new();
    let loaded = load(&source_options("app/main.rs"), &mut |path| {
        *calls.entry(path.to_string()).or_default() += 1;
        Ok(files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, text)| text.to_string()))
    });
    assert!(loaded.report.diagnostics.is_empty());
    assert_eq!(loaded.bundle.root, "app/main.rs");
    assert!(loaded.bundle.library.is_none());
    assert_eq!(loaded.bundle.files.len(), 5);
    assert!(calls.values().all(|count| *count == 1));
    assert!(!calls.keys().any(|path| path.contains("excluded")));
    for input in &loaded.bundle.files {
        let expected = files.iter().find(|(path, _)| *path == input.path).unwrap();
        assert_eq!(input.text.as_bytes(), expected.1.as_bytes());
    }
}

#[test]
fn repeated_mounts_snapshot_successful_reads_and_absence_once() {
    let mut calls = BTreeMap::<String, usize>::new();
    let loaded = load(&source_options("main.rs"), &mut |path| {
        let count = calls.entry(path.to_string()).or_default();
        *count += 1;
        Ok(match path {
            "main.rs" => Some("mod shared; mod shared;".to_string()),
            "shared.rs" => Some(format!("pub fn value()->i64{{{count}}}")),
            _ => None,
        })
    });
    assert!(loaded.report.diagnostics.is_empty());
    assert_eq!(calls.get("shared.rs"), Some(&1));
    assert_eq!(calls.get("shared/mod.rs"), Some(&1));
    assert_eq!(loaded.bundle.files.len(), 2);
    assert_eq!(loaded.bundle.files[1].text, "pub fn value()->i64{1}");
}

#[test]
fn library_inputs_use_declared_logical_paths() {
    let mut options = source_options("main.rs");
    options.library = Some("/physical/library".to_string());
    let files = [
        ("main.rs", "fn probe()->i64{1}"),
        ("$std/lib.rs", "mod helper;"),
        ("$std/helper.rs", "pub fn helper()->i64{2}"),
    ];
    let loaded = load_files(&options, &files);
    assert!(loaded.report.diagnostics.is_empty());
    assert_eq!(loaded.bundle.library.as_deref(), Some("$std/lib.rs"));
    assert_eq!(loaded.bundle.files.len(), 3);
    assert!(
        loaded
            .bundle
            .files
            .iter()
            .all(|file| !file.path.contains("/physical/library"))
    );
}

#[test]
fn syntax_missing_and_ambiguous_module_errors_survive_a_bundle_snapshot() {
    let cases = [
        (vec![("main.rs", "fn broken( {")], "E_PARSE"),
        (vec![("main.rs", "mod missing;")], "E_MODULE_MISSING"),
        (
            vec![
                ("main.rs", "mod chosen;"),
                ("chosen.rs", "pub fn direct()->i64{1}"),
                ("chosen/mod.rs", "pub fn nested()->i64{2}"),
            ],
            "E_MODULE_AMBIGUOUS",
        ),
    ];
    for (files, code) in cases {
        let loaded = load_files(&source_options("main.rs"), &files);
        assert_eq!(loaded.report.diagnostics[0].code, code);
        let bytes = frontend_bundle::encode(&loaded.bundle);
        let bundle = frontend_bundle::decode(&bytes).unwrap();
        let again = source::parse_bundle_report(&bundle);
        assert_eq!(again.diagnostics[0], loaded.report.diagnostics[0]);
    }
}

#[test]
fn io_errors_remain_distinct_from_an_absent_source() {
    let loaded = load(&source_options("main.rs"), &mut |_| {
        Err("source is not valid UTF-8".to_string())
    });
    assert_eq!(loaded.report.diagnostics[0].code, "E_IO");
    assert_eq!(loaded.report.diagnostics[0].path, "main.rs");
    assert!(loaded.bundle.files.is_empty());
    let absent = source::parse_bundle_report(&loaded.bundle);
    assert_eq!(absent.diagnostics[0].code, "E_MODULE_MISSING");
}

#[test]
fn the_loaded_program_and_its_snapshot_emit_identical_k() {
    let loaded = load_files(
        &source_options("app/main.rs"),
        &[
            (
                "app/main.rs",
                "mod helper;fn probe()->i64{helper::value()+3}",
            ),
            ("app/helper.rs", "pub fn value()->i64{7}"),
        ],
    );
    assert!(loaded.report.diagnostics.is_empty());
    let checked = frontend::check_source(&loaded.report.program).unwrap();
    let direct = k_text::write(checked.program());
    let packed = frontend::source_to_k(&loaded.bundle).unwrap();
    assert_eq!(direct.as_bytes(), packed.as_bytes());
}

#[test]
fn the_source_encoder_matches_the_independent_node_encoder_exactly() {
    let text = "\u{feff}// π\r\nfn probe()->i64{7}\n// nul:\0";
    let bundle = source::SourceBundle {
        root: "app/π.rs".to_string(),
        library: None,
        files: vec![source::SourceInput {
            path: "app/π.rs".to_string(),
            text: text.to_string(),
        }],
    };
    let script = r#"
import fs from 'node:fs';
import {encodeBundle} from './tools/source-bundle.mjs';
const text=fs.readFileSync(0,'utf8');
process.stdout.write(encodeBundle({
    root:'app/π.rs',library:null,files:[{path:'app/π.rs',text}]
}));
"#;
    let mut child = Command::new("node")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--input-type=module", "-e", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    let observed = child.wait_with_output().unwrap();
    assert!(observed.status.success(), "{:?}", observed.stderr);
    let bytes = frontend_bundle::encode(&bundle);
    assert_eq!(bytes, observed.stdout);
    let decoded = frontend_bundle::decode(&bytes).unwrap();
    assert_eq!(decoded.files[0].text.as_bytes(), text.as_bytes());
}
