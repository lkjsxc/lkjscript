//! The entire production IR representation module, including its Clone/equality
//! consumers, executes through the ordinary source/library pipeline.
#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "support/native.rs"]
mod native_test;

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const IR: &str = include_str!("../src/ir.rs");
const PROBES: &str = include_str!("fixtures/ir_component.rs");
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lkjscript-ir-component-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Observation {
    rust: String,
    name: String,
    arguments: Vec<i64>,
}
fn call(name: &str, rust_arguments: &str, _abi_arguments: &str) -> Observation {
    Observation {
        rust: format!("{name}({rust_arguments})"),
        name: name.to_string(),
        arguments: rust_arguments
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .map(|value| match value.trim() {
                "true" => 1,
                "false" => 0,
                value => value.parse().unwrap(),
            })
            .collect(),
    }
}
fn observations() -> Vec<Observation> {
    let mut cases = Vec::new();
    // Every byte and terminator of every production primitive name, plus kinds.
    for index in 0..7 {
        for field in 0..12 {
            cases.push(call(
                "primitive_snapshot",
                &format!("{index}, {field}"),
                &format!("{index}n,{field}n"),
            ));
        }
    }
    for which in 0..3 {
        cases.push(call(
            "capability_snapshot",
            &which.to_string(),
            &format!("{which}n"),
        ));
    }
    // Every production Op variant: clone followed by payload reads, equality,
    // and inequality after a payload/tag change. These Op values are data;
    // this component probe does not execute them as a checked IR program.
    for index in 0..30 {
        cases.push(call(
            "operation_snapshot",
            &index.to_string(),
            &format!("{index}n"),
        ));
        for changed in [false, true] {
            cases.push(call(
                "operation_equality",
                &format!("{index}, {changed}"),
                &format!("{index}n,{}n", u8::from(changed)),
            ));
        }
    }
    for field in 0..6 {
        cases.push(call(
            "program_snapshot",
            &field.to_string(),
            &format!("{field}n"),
        ));
    }
    for which in 0..6 {
        cases.push(call(
            "return_source_snapshot",
            &which.to_string(),
            &format!("{which}n"),
        ));
    }
    cases.push(call("production_place_local", "", ""));
    cases
}
fn bundle(module: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.extend([
        SourceInput {
            path: "src/ir_component.rs".to_string(),
            text: PROBES.to_string(),
        },
        SourceInput {
            path: "src/ir.rs".to_string(),
            text: module.to_string(),
        },
    ]);
    SourceBundle {
        root: "src/ir_component.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}
fn rust_results(module: &str, cases: &[Observation]) -> Vec<i64> {
    let directory = Temporary::new();
    std::fs::write(directory.0.join("ir.rs"), module).unwrap();
    let mut root = PROBES.to_string();
    root.push_str("\nfn main() {\n");
    for case in cases {
        root.push_str(&format!("println!(\"{{}}\", {});\n", case.rust));
    }
    root.push_str("}\n");
    std::fs::write(directory.0.join("component.rs"), root).unwrap();
    let executable = directory.0.join("component");
    let compilation = Command::new("rustc")
        .args(["--edition=2024", "-Awarnings", "-Coverflow-checks=yes"])
        .arg(directory.0.join("component.rs"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let values: Vec<i64> = String::from_utf8(execution.stdout)
        .unwrap()
        .lines()
        .map(|line| line.parse().unwrap())
        .collect();
    assert_eq!(values.len(), cases.len());
    values
}
fn run(checked: &lkjscript::ir::CheckedProgram, cases: &[Observation], expected: &[i64]) {
    assert_eq!(cases.len(), expected.len());
    let calls: Vec<_> = cases
        .iter()
        .zip(expected)
        .map(|(case, value)| (case.name.as_str(), case.arguments.as_slice(), *value))
        .collect();
    native_test::assert_calls(checked, &calls);
}

#[test]
fn entire_production_ir_module_executes_cloning_equality_and_representation_access() {
    let cases = observations();
    let expected = rust_results(IR, &cases);
    let artifact = compiler::check_bundle(&bundle(IR)).unwrap();
    run(&artifact, &cases, &expected);
}

#[test]
fn production_ir_body_mutations_change_artifacts_and_results() {
    let cases = vec![
        call("primitive_snapshot", "2, 0", "2n,0n"),
        call("production_place_local", "", ""),
    ];
    let changed = IR.replacen("bits: 32,", "bits: 31,", 1).replacen(
        "Place {\n            local,",
        "Place {\n            local: local + 1,",
        1,
    );
    assert_ne!(changed, IR);
    let original_values = rust_results(IR, &cases);
    let changed_values = rust_results(&changed, &cases);
    assert_eq!(original_values, vec![65, 42]);
    assert_eq!(changed_values, vec![63, 43]);
    let original = compiler::check_bundle(&bundle(IR)).unwrap();
    let mutated = compiler::check_bundle(&bundle(&changed)).unwrap();
    assert_ne!(
        lkjscript::ir_text::write(original.program()),
        lkjscript::ir_text::write(mutated.program())
    );
    run(&original, &cases, &original_values);
    run(&mutated, &cases, &changed_values);
}

#[test]
fn invalid_production_ir_body_mutation_reports_its_original_file() {
    let changed = IR.replacen("bits: 32,", "bits: true,", 1);
    assert_ne!(changed, IR);
    let error = compiler::check_bundle(&bundle(&changed)).err().unwrap();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "src/ir.rs", "{error:?}");
}
