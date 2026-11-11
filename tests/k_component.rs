//! The entire production K representation module, including its Clone/equality
//! consumers, executes through the ordinary source/library pipeline.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const K: &str = include_str!("../src/k.rs");
const PROBES: &str = include_str!("fixtures/k_component.rs");
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lkjscript-k-component-{}-{}",
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
    wasm: String,
}
fn call(name: &str, rust_arguments: &str, wasm_arguments: &str) -> Observation {
    Observation {
        rust: format!("{name}({rust_arguments})"),
        wasm: format!("e['fn:{name}']({wasm_arguments})"),
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
    // Every production Op variant: clone followed by payload reads, equality,
    // and inequality after a payload/tag change. These Op values are data;
    // this component probe does not execute them as a checked K program.
    for index in 0..26 {
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
    for which in 0..3 {
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
            path: "src/k_component.rs".to_string(),
            text: PROBES.to_string(),
        },
        SourceInput {
            path: "src/k.rs".to_string(),
            text: module.to_string(),
        },
    ]);
    SourceBundle {
        root: "src/k_component.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}
fn rust_results(module: &str, cases: &[Observation]) -> Vec<i64> {
    let directory = Temporary::new();
    std::fs::write(directory.0.join("k.rs"), module).unwrap();
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
fn run(artifact: &[u8], cases: &[Observation], expected: &[i64]) {
    assert_eq!(cases.len(), expected.len());
    let mut script = String::from(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{try{const {instance}=await WebAssembly.instantiate(Buffer.concat(chunks),{lkjscript:{trap(c,l){throw Error('trap '+c+':'+l)}}});const e=instance.exports;",
    );
    for (index, (case, expected)) in cases.iter().zip(expected).enumerate() {
        script.push_str(&format!(
            "{{const actual={};if(actual!=={expected}n)throw Error('observation {index}: '+actual+' != {expected}');}}",
            case.wasm
        ));
    }
    script.push_str("}catch(error){console.error(error);process.exitCode=1}})");
    let mut child = Command::new("node")
        .args(["-e", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(artifact).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn entire_production_k_module_executes_cloning_equality_and_representation_access() {
    let cases = observations();
    let expected = rust_results(K, &cases);
    let artifact = compiler::compile(&bundle(K)).unwrap();
    run(&artifact, &cases, &expected);
}

#[test]
fn production_k_body_mutations_change_artifacts_and_results() {
    let cases = vec![
        call("primitive_snapshot", "2, 0", "2n,0n"),
        call("production_place_local", "", ""),
    ];
    let changed = K.replacen("bits: 32,", "bits: 31,", 1).replacen(
        "Place {\n            local,",
        "Place {\n            local: local + 1,",
        1,
    );
    assert_ne!(changed, K);
    let original_values = rust_results(K, &cases);
    let changed_values = rust_results(&changed, &cases);
    assert_eq!(original_values, vec![65, 42]);
    assert_eq!(changed_values, vec![63, 43]);
    let original = compiler::compile(&bundle(K)).unwrap();
    let mutated = compiler::compile(&bundle(&changed)).unwrap();
    assert_ne!(original, mutated);
    run(&original, &cases, &original_values);
    run(&mutated, &cases, &changed_values);
}

#[test]
fn invalid_production_k_body_mutation_reports_its_original_file() {
    let changed = K.replacen("bits: 32,", "bits: true,", 1);
    assert_ne!(changed, K);
    let error = compiler::compile(&bundle(&changed)).unwrap_err();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "src/k.rs", "{error:?}");
}
