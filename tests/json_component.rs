//! The production trap-metadata JSON encoder executes from its actual source.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{check, compiler, json, k, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

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

fn run(artifact: &[u8], observations: &str) {
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{\
         const {{module,instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});\
         const e=instance.exports;{observations}\
         }}catch(error){{console.error(error);process.exitCode=1}}}});"
    );
    let mut child = Command::new("node")
        .args(["-e", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node is required for generated component execution");
    child.stdin.take().unwrap().write_all(artifact).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
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
    let mut script = r#"
        function readOutput(header){
            const view=new DataView(e.memory.buffer);
            const data=Number(view.getBigUint64(Number(header),true));
            const length=Number(view.getBigUint64(Number(header)+8,true));
            const bytes=Buffer.alloc(length);
            for(let i=0;i<length;i++){
                const byte=view.getBigUint64(data+8*i,true);
                if(byte>255n)throw Error('nonbyte output');
                bytes[i]=Number(byte);
            }
            return bytes;
        }
        function verify(index,expected,input,valid){
            const owner=e['fn:encode_'+index]();
            const actual=readOutput(owner);
            e['fn:release'](owner);
            if(!actual.equals(Buffer.from(expected,'hex')))throw Error('output bytes differ: '+index);
            if(valid){
                const decoded=JSON.parse(actual.toString('utf8'));
                if(!Array.isArray(decoded)||decoded.length!==2||decoded[1]!=='tail')throw Error('append structure: '+index);
                if(!Buffer.from(decoded[0],'utf8').equals(Buffer.from(input,'hex')))throw Error('JSON round trip: '+index);
            }else{
                let rejected=false;
                try{JSON.parse(actual.toString('utf8'))}catch(error){rejected=error instanceof SyntaxError}
                if(!rejected)throw Error('mutated control byte was not rejected by JSON parser');
            }
        }
    "#.to_string();
    for (index, ((input, expected), valid)) in cases.iter().zip(outputs).zip(valid).enumerate() {
        script.push_str(&format!(
            "verify({index},'{}','{}',{valid});",
            hex(expected),
            hex(input.as_bytes())
        ));
    }
    run(artifact, &script);
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
fn production_emitter_uses_the_shared_json_encoder_for_trap_metadata() {
    let cases = cases();
    let functions = cases
        .iter()
        .enumerate()
        .map(|(index, message)| k::Function {
            file: 0,
            name: format!("metadata_{index}"),
            params: 0,
            result: k::UNIT,
            return_source: None,
            locals: Vec::new(),
            body: vec![k::Instruction {
                op: k::Op::Trap {
                    message: message.clone(),
                },
                line: 1,
            }],
            test: false,
        })
        .collect();
    let checked = check::check(k::Program {
        types: k::primitives(),
        functions,
        destructors: Vec::new(),
    })
    .unwrap();
    let artifact = wasm::emit(&checked).unwrap();
    let mut expected = vec![b'['];
    for (index, input) in cases.iter().enumerate() {
        if index != 0 {
            expected.push(b',');
        }
        json::write_string(&mut expected, input);
    }
    expected.push(b']');
    let mut script = format!(
        "const sections=WebAssembly.Module.customSections(module,'lkjscript.traps');\
         if(sections.length!==1)throw Error('trap metadata missing');\
         const actual=Buffer.from(sections[0]);\
         if(!actual.equals(Buffer.from('{}','hex')))throw Error('trap metadata bytes differ');\
         const messages=JSON.parse(actual.toString('utf8'));\
         if(messages.length!=={})throw Error('trap message count');",
        hex(&expected),
        cases.len()
    );
    for (index, input) in cases.iter().enumerate() {
        script.push_str(&format!(
            "if(!Buffer.from(messages[{index}],'utf8').equals(Buffer.from('{}','hex')))throw Error('trap message {index}');",
            hex(input.as_bytes())
        ));
    }
    run(&artifact, &script);
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
