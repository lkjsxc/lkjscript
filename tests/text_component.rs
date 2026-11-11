//! The production canonical-name helper executes with the owned UTF-8 library.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, names};
use std::io::Write;
use std::process::{Command, Stdio};

const NAMES: &str = include_str!("../src/names.rs");

fn bundle(module: &str, cases: &[(String, String)]) -> SourceBundle {
    let mut root = "mod names; use names::qualified as join;\n".to_string();
    for (index, (parent, name)) in cases.iter().enumerate() {
        let expected = names::qualified(parent, name);
        root.push_str(&format!(
            "pub fn compare_{index}()->usize {{\n\
             let value=join({parent:?},{name:?});\n\
             let actual=value.as_bytes();\n\
             let expected={expected:?}.as_bytes();\n\
             if actual.len()!=expected.len(){{return 1;}}\n\
             let mut index:usize=0;\n\
             while index<actual.len(){{\n\
                 if actual[index]!=expected[index]{{return index+2;}}\n\
                 index=index+1;\n\
             }}\n\
             0\n\
             }}\n"
        ));
    }
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "text_component.rs".to_string(),
        text: root,
    });
    files.push(SourceInput {
        path: "names.rs".to_string(),
        text: module.to_string(),
    });
    SourceBundle {
        root: "text_component.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn run(bytes: &[u8], expected: &[usize]) {
    let mut script = String::from(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{try{const {instance}=await WebAssembly.instantiate(Buffer.concat(chunks),{lkjscript:{trap(c,l){throw Error('trap '+c+':'+l)}}});",
    );
    for (index, expected) in expected.iter().enumerate() {
        script.push_str(&format!(
            "{{const actual=instance.exports['fn:compare_{index}']();if(actual!=={expected}n)throw Error('case {index}: '+actual);}}"
        ));
    }
    script.push_str("}catch(e){console.error(e);process.exitCode=1}})");
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn production_qualified_names_match_rust_over_every_output_byte() {
    let ascii: String = (0u8..=127).map(char::from).collect();
    let cases = vec![
        (String::new(), String::new()),
        (String::new(), "root".to_string()),
        ("outer::inner".to_string(), "leaf".to_string()),
        ("α::水".to_string(), "😀\0leaf\u{10ffff}".to_string()),
        (ascii.clone(), ascii),
        ("module_segment::".repeat(256), "水😀".repeat(256)),
    ];
    let artifact = compiler::compile(&bundle(NAMES, &cases)).unwrap();
    run(&artifact, &vec![0; cases.len()]);
}

#[test]
fn changing_the_production_join_changes_generated_behavior() {
    let cases = vec![
        (String::new(), "leaf".to_string()),
        ("module".to_string(), "leaf".to_string()),
    ];
    let original = compiler::compile(&bundle(NAMES, &cases)).unwrap();
    let changed = NAMES.replace("push_str(\"::\")", "push_str(\":\")");
    assert_ne!(NAMES, changed);
    let mutated = compiler::compile(&bundle(&changed, &cases)).unwrap();
    assert_ne!(original, mutated);
    run(&original, &[0, 0]);
    run(&mutated, &[0, 1]);
}

#[test]
fn invalid_production_join_mutation_is_checked() {
    let changed = NAMES.replace("result.push_str(name)", "result.push_str(1i64)");
    assert_ne!(NAMES, changed);
    let cases = vec![("module".to_string(), "leaf".to_string())];
    let error = compiler::compile(&bundle(&changed, &cases)).unwrap_err();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "names.rs");
}
