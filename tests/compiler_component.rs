//! The exact production encoding module executes under the generated toolchain.
use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, encoding};
use std::io::Write;
use std::process::{Command, Stdio};

const ENCODING: &str = include_str!("../src/encoding.rs");
const WRAPPER: &str = r#"
mod encoding;
pub fn unsigned(value:u64,index:usize)->i64 {
    let mut bytes:Vec<u8>=Vec::new();
    encoding::uleb(&mut bytes,value);
    if index<bytes.len(){bytes[index] as i64}else{-1}
}
pub fn signed(value:i64,index:usize)->i64 {
    let mut bytes:Vec<u8>=Vec::new();
    encoding::sleb(&mut bytes,value);
    if index<bytes.len(){bytes[index] as i64}else{-1}
}
"#;
fn bundle(module: &str) -> SourceBundle {
    SourceBundle {
        root: "component.rs".to_string(),
        library: None,
        files: vec![
            SourceInput {
                path: "component.rs".to_string(),
                text: WRAPPER.to_string(),
            },
            SourceInput {
                path: "encoding.rs".to_string(),
                text: module.to_string(),
            },
        ],
    }
}
fn run(bytes: &[u8], observations: &str) {
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});{observations}}}catch(e){{console.error(e);process.exitCode=1}}}})"
    );
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
fn expected_call(script: &mut String, name: &str, value: &str, bytes: &[u8]) {
    for (index, byte) in bytes.iter().enumerate() {
        script.push_str(&format!("if(instance.exports['fn:{name}']({value}n,{index}n)!=={byte}n)throw Error('{name} {value} byte {index}');"));
    }
    script.push_str(&format!("if(instance.exports['fn:{name}']({value}n,{}n)!==-1n)throw Error('{name} {value} has extra bytes');",bytes.len()));
}
#[test]
fn production_encoders_execute_through_ordinary_source_compilation() {
    let mut script = String::new();
    let mut unsigned = vec![0u64, 1, 63, 64, 127, 128, 129, 255, 256, u64::MAX];
    let mut bit = 7;
    while bit < 64 {
        let boundary = 1u64 << bit;
        unsigned.extend([boundary - 1, boundary, boundary + 1]);
        bit += 7;
    }
    unsigned.sort_unstable();
    unsigned.dedup();
    for value in unsigned {
        let mut bytes = Vec::new();
        encoding::uleb(&mut bytes, value);
        expected_call(&mut script, "unsigned", &value.to_string(), &bytes);
    }
    let mut signed = vec![
        i64::MIN,
        -129,
        -128,
        -65,
        -64,
        -1,
        0,
        1,
        63,
        64,
        127,
        128,
        i64::MAX,
    ];
    let mut bit = 6;
    while bit < 63 {
        let boundary = 1i64 << bit;
        signed.extend([
            -boundary - 1,
            -boundary,
            -boundary + 1,
            boundary - 1,
            boundary,
            boundary + 1,
        ]);
        bit += 7;
    }
    signed.sort_unstable();
    signed.dedup();
    for value in signed {
        let mut bytes = Vec::new();
        encoding::sleb(&mut bytes, value);
        expected_call(&mut script, "signed", &value.to_string(), &bytes);
    }
    let bytes = compiler::compile(&bundle(ENCODING)).unwrap();
    run(&bytes, &script);
}
#[test]
fn production_encoders_match_known_representation_boundaries() {
    for (value, expected) in [
        (0u64, vec![0]),
        (127, vec![127]),
        (128, vec![128, 1]),
        (
            u64::MAX,
            vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 1],
        ),
    ] {
        let mut bytes = Vec::new();
        encoding::uleb(&mut bytes, value);
        assert_eq!(bytes, expected);
    }
    for (value, expected) in [
        (-1i64, vec![127]),
        (-64, vec![64]),
        (-65, vec![191, 127]),
        (63, vec![63]),
        (64, vec![192, 0]),
        (
            i64::MIN,
            vec![128, 128, 128, 128, 128, 128, 128, 128, 128, 127],
        ),
        (
            i64::MAX,
            vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 0],
        ),
    ] {
        let mut bytes = Vec::new();
        encoding::sleb(&mut bytes, value);
        assert_eq!(bytes, expected);
    }
}
#[test]
fn production_source_mutations_change_generated_component_behavior() {
    let original = compiler::compile(&bundle(ENCODING)).unwrap();
    let changed = ENCODING.replacen("value & 127", "value & 126", 1);
    assert_ne!(changed, ENCODING);
    let mutated = compiler::compile(&bundle(&changed)).unwrap();
    assert_ne!(original, mutated);
    run(
        &original,
        "if(instance.exports['fn:unsigned'](127n,0n)!==127n)throw Error('original source was not executed');",
    );
    run(
        &mutated,
        "if(instance.exports['fn:unsigned'](127n,0n)!==126n)throw Error('changed source was not executed');",
    );
}
#[test]
fn invalid_production_component_mutation_is_rejected_semantically() {
    let changed = ENCODING.replacen("let byte = (value & 127) as u8;", "let byte = true;", 1);
    assert_ne!(changed, ENCODING);
    let error = compiler::compile(&bundle(&changed)).unwrap_err();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "encoding.rs");
}
