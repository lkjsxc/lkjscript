//! The production batch-protocol JSON encoder executes from its actual source.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, json};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const JSON: &str = include_str!("../src/json.rs");

fn bundle(module: &str, cases: &[String]) -> SourceBundle {
    let mut root = "mod json;\npub fn release(bytes:Vec<u8>) { drop(bytes); }\n".to_string();
    for (index, input) in cases.iter().enumerate() {
        root.push_str(&format!(
            "pub fn encode_{index}()->Vec<u8> {{\n\
             let mut bytes:Vec<u8>=Vec::new();\n\
             bytes.push(b'[');\n\
             json::write_string(&mut bytes,{input:?});\n\
             bytes.push(b',');\n\
             json::write_string(&mut bytes,\"tail\");\n\
             bytes.push(b']');\n\
             bytes\n\
             }}\n"
        ));
    }
    root.push_str("pub fn main(mut world:World) { let mut out=world.stdout();\n");
    for index in 0..cases.len() {
        root.push_str(&format!("let value=encode_{index}();out.write(&value);\n"));
    }
    root.push_str("}\n");
    let mut files = bootstrap_library::standard_library();
    files.extend([
        SourceInput {
            path: "json_component.rs".to_string(),
            text: root,
        },
        SourceInput {
            path: "json.rs".to_string(),
            text: module.to_string(),
        },
    ]);
    SourceBundle {
        root: "json_component.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn run(artifact: &[u8]) -> Vec<u8> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/json-component");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!(
        "{}-{}.elf",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, artifact).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new("prlimit")
        .arg("--as=268435456")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn rust_output(input: &str) -> Vec<u8> {
    let mut bytes = vec![b'['];
    json::write_string(&mut bytes, input);
    bytes.push(b',');
    json::write_string(&mut bytes, "tail");
    bytes.push(b']');
    bytes
}

fn compare_outputs(artifact: &[u8], cases: &[String], outputs: &[Vec<u8>], valid: &[bool]) {
    assert_eq!(cases.len(), outputs.len());
    assert_eq!(cases.len(), valid.len());
    let expected: Vec<u8> = outputs.iter().flatten().copied().collect();
    assert_eq!(run(artifact), expected);
    // The reference JSON parser verifies the bytes, not a second execution engine.
    let mut script = String::new();
    for (index, ((input, output), valid)) in cases.iter().zip(outputs).zip(valid).enumerate() {
        script.push_str(&format!("{{const text=Buffer.from('{}','hex').toString('utf8');let value;let parsed=true;try{{value=JSON.parse(text)}}catch{{parsed=false}}if(parsed!=={valid})throw Error('validity {index}');if(parsed&&(!Array.isArray(value)||value.length!==2||value[1]!=='tail'||Buffer.from(value[0],'utf8').toString('hex')!=='{}'))throw Error('round trip {index}');}}",hex(output),hex(input.as_bytes())));
    }
    let result = Command::new("node").args(["-e", &script]).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn cases() -> Vec<String> {
    vec![
        String::new(),
        "plain ASCII".to_string(),
        "\"quoted\" \\ slash /".to_string(),
        (0u8..=31).map(char::from).collect(),
        (32u8..=255).map(char::from).collect(),
        "é 水 😀 \u{2028}\u{2029}\u{10ffff}".to_string(),
        "é\"\\\n水".repeat(128),
    ]
}

#[test]
fn rust_json_encoder_preserves_existing_escape_bytes() {
    for (input, expected) in [
        ("", "\"\""),
        ("plain /", "\"plain /\""),
        ("\"\\", "\"\\\"\\\\\""),
        ("\0\t\n\r\u{1f}", "\"\\u0000\\u0009\\u000a\\u000d\\u001f\""),
        ("é\u{85}\u{9f}水", "\"é\u{85}\u{9f}水\""),
    ] {
        let mut actual = Vec::new();
        json::write_string(&mut actual, input);
        assert_eq!(actual, expected.as_bytes());
    }
    let controls: String = (0u8..=31).map(char::from).collect();
    let mut expected = String::from("\"");
    for byte in 0u8..=31 {
        expected.push_str(&format!("\\u{byte:04x}"));
    }
    expected.push('"');
    let mut actual = Vec::new();
    json::write_string(&mut actual, &controls);
    assert_eq!(actual, expected.as_bytes());
}

#[test]
fn production_json_encoder_executes_and_round_trips_every_output_byte() {
    let cases = cases();
    let expected: Vec<Vec<u8>> = cases.iter().map(|input| rust_output(input)).collect();
    let artifact = compiler::compile(&bundle(JSON, &cases)).unwrap();
    compare_outputs(&artifact, &cases, &expected, &vec![true; cases.len()]);
}

#[test]
fn production_range_mutation_changes_artifact_and_json_behavior() {
    let cases = vec!["\u{1f}".to_string(), "ordinary".to_string()];
    let original = compiler::compile(&bundle(JSON, &cases)).unwrap();
    let changed = JSON.replace("0..=31", "0..=30");
    assert_ne!(JSON, changed);
    let mutated = compiler::compile(&bundle(&changed, &cases)).unwrap();
    assert_ne!(original, mutated);
    let expected: Vec<Vec<u8>> = cases.iter().map(|input| rust_output(input)).collect();
    compare_outputs(&original, &cases, &expected, &[true, true]);
    let changed_output = vec![
        vec![
            b'[', b'"', 31, b'"', b',', b'"', b't', b'a', b'i', b'l', b'"', b']',
        ],
        expected[1].clone(),
    ];
    compare_outputs(&mutated, &cases, &changed_output, &[false, true]);
}

#[test]
fn invalid_production_range_mutation_reports_the_original_file() {
    let changed = JSON.replace("0..=31", "32..=31");
    assert_ne!(JSON, changed);
    let error = compiler::compile(&bundle(&changed, &["text".to_string()])).unwrap_err();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "json.rs", "{error:?}");
    assert!(error.message.contains("range"), "{error:?}");
}
