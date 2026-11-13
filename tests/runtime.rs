use lkjscript::{compiler, machine, runtime, source, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

fn bundle(text: String) -> source::SourceBundle {
    source::SourceBundle {
        root: "runtime.rs".to_string(),
        library: None,
        files: vec![source::SourceInput {
            path: "runtime.rs".to_string(),
            text,
        }],
    }
}
fn source_text() -> String {
    format!(
        "{}\n{}\n{}",
        include_str!("../library/runtime.rs"),
        include_str!("runtime/allocator.rs"),
        include_str!("runtime/drop_walk.rs")
    )
}
fn value(
    runtime: &runtime::Runtime,
    name: &str,
    args: &[u64],
    memory: &mut runtime::Memory,
) -> machine::RunResult {
    machine::run_runtime(
        runtime,
        runtime.function(name).unwrap(),
        args,
        memory,
        &mut Vec::new(),
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message))
}
fn bits(result: machine::RunResult) -> u64 {
    match result.value {
        machine::Scalar::Int { value, .. } => value,
        _ => panic!("expected a runtime word"),
    }
}
fn node(bytes: &[u8], script: &str) {
    let mut child=Command::new("node").arg("-e").arg(format!(
        "let parts=[];process.stdin.on('data',x=>parts.push(x));process.stdin.on('end',async()=>{{try{{const {{instance}}=await WebAssembly.instantiate(Buffer.concat(parts));const e=instance.exports;{script}}}catch(error){{console.error(error);process.exitCode=1}}}});"))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn source_runtime_reuse_growth_copy_utf8_and_cleanup_agree_with_its_k_model() {
    let runtime = runtime::prepare(&bundle(source_text())).unwrap();
    let step = runtime.function("model_drop_step").unwrap();
    let mut memory = runtime::Memory::new();
    memory.set_drop_steps(vec![step]);
    assert_eq!(
        bits(value(&runtime, "probe_allocator", &[], &mut memory)),
        42
    );
    let cleanup = value(&runtime, "probe_drop", &[], &mut memory);
    assert!(
        cleanup.peak_frames < 32,
        "cleanup recursed instead of using its owned work stack"
    );
    assert_eq!(bits(cleanup), 42);
    let cases = vec![
        vec![],
        vec![0, 127],
        vec![194, 128],
        vec![223, 191],
        vec![224, 160, 128],
        vec![237, 159, 191],
        vec![240, 144, 128, 128],
        vec![244, 143, 191, 191],
        vec![128],
        vec![192, 128],
        vec![224, 159, 191],
        vec![237, 160, 128],
        vec![244, 144, 128, 128],
        vec![240, 144, 128],
    ];
    let mut expected = Vec::new();
    for bytes in &cases {
        memory
            .write(4096, bytes)
            .unwrap_or_else(|error| panic!("{}", error.message()));
        let answer = u64::from(std::str::from_utf8(bytes).is_ok());
        assert_eq!(
            bits(value(
                &runtime,
                "valid_utf8_bytes",
                &[4096, bytes.len() as u64],
                &mut memory
            )),
            answer
        );
        expected.push(answer);
    }
    let overflow = machine::run_runtime(
        &runtime,
        runtime.function("probe_overflow").unwrap(),
        &[],
        &mut memory,
        &mut Vec::new(),
    )
    .err()
    .unwrap();
    assert_eq!(overflow.code(), "R0006");
    let bytes = wasm::encode(&runtime.model_program(&[step]).unwrap()).unwrap();
    node(
        &bytes,
        &format!(
            r#"
        if(e['fn:probe_allocator']()!==42n||e['fn:probe_drop']()!==42n)throw Error('runtime model probe');
        const cases={cases:?},expected={expected:?};
        for(let i=0;i<cases.length;i++){{new Uint8Array(e.memory.buffer).set(cases[i],4096);
            if(e['fn:valid_utf8_bytes'](4096n,BigInt(cases[i].length))!==BigInt(expected[i]))throw Error('UTF8 '+i);}}
        let trapped=false;try{{e['fn:probe_overflow']()}}catch(error){{trapped=error instanceof WebAssembly.RuntimeError}}
        if(!trapped||e.__lkj_trap_code.value!==6n)throw Error('allocation overflow');
    "#
        ),
    );
}

#[test]
fn raw_names_in_ordinary_source_do_not_grant_runtime_authority() {
    let input = bundle(source_text());
    let checked = compiler::check_bundle(&input).unwrap();
    let entry = checked
        .program()
        .functions
        .iter()
        .position(|function| function.name == "probe_allocator")
        .unwrap();
    let result = machine::run(checked.program(), entry, &[], &mut Vec::new());
    assert!(matches!(result.err().unwrap().kind,machine::ErrorKind::Trap(code) if code>=16));
    node(
        &wasm::emit(&checked).unwrap(),
        r#"
        let trapped=false;try{e['fn:probe_allocator']()}catch(error){trapped=error instanceof WebAssembly.RuntimeError}
        if(!trapped||e.__lkj_trap_code.value<16n)throw Error('ordinary raw name gained authority');
    "#,
    );
    let replaced = input.files[0]
        .text
        .replacen("panic!(\"unsealed runtime primitive\")", "0", 1);
    assert!(runtime::prepare(&bundle(replaced)).is_err());
}
