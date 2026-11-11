//! The compiler-core boundary is driven by declared source and library bytes.
use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::io::Write;
use std::process::{Command, Stdio};

fn bundle(value: i64) -> SourceBundle {
    SourceBundle {
        root: "main.rs".to_string(),
        library: Some("library/lib.rs".to_string()),
        files: vec![
            SourceInput {
                path: "main.rs".to_string(),
                text: "mod child; pub fn probe()->i64 { child::answer() }".to_string(),
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
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));\
         process.stdin.on('end',async()=>{{try{{\
         const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),\
         {{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});\
         const result=instance.exports['fn:probe']();\
         if(result!=={expected}n)throw Error('expected {expected}, got '+result);\
         }}catch(e){{console.error(e);process.exitCode=1}}}});"
    );
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
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
