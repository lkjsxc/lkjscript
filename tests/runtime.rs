use lkjscript::{compiler, machine, runtime, source};

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
        machine::Resources::unlimited(),
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message))
}
fn bits(result: machine::RunResult) -> u64 {
    match result.value {
        machine::Scalar::Int { value, .. } => value,
        _ => panic!("expected a runtime word"),
    }
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
    for bytes in &cases {
        memory
            .write(4096, bytes)
            .unwrap_or_else(|error| panic!("{}", error.message()));
        let answer = u64::from(std::str::from_utf8(bytes).is_ok());
        assert_eq!(
            bits(value(
                &runtime,
                "utf8_validate",
                &[4096, bytes.len() as u64],
                &mut memory
            )),
            answer
        );
    }
    let overflow = machine::run_runtime(
        &runtime,
        runtime.function("probe_overflow").unwrap(),
        &[],
        &mut memory,
        &mut Vec::new(),
        machine::Resources::unlimited(),
    )
    .err()
    .unwrap();
    assert_eq!(overflow.code(), "R0006");
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
    let result = machine::run(
        checked.program(),
        entry,
        &[],
        &mut Vec::new(),
        machine::Resources::unlimited(),
    );
    assert!(matches!(result.err().unwrap().kind,machine::ErrorKind::Trap(code) if code>=16));
    let replaced = input.files[0]
        .text
        .replacen("panic!(\"unsealed runtime primitive\")", "0", 1);
    assert!(runtime::prepare(&bundle(replaced)).is_err());
    let wrong_entry = input.files[0].text.replace(
        "pub fn utf8_validate(data: u64, length: u64) -> u64 {\n    valid_utf8_bytes(data, length)\n}",
        "pub fn utf8_validate(data: u64) -> u64 { valid_utf8_bytes(data, 0) }",
    );
    let error = runtime::prepare(&bundle(wrong_entry)).err().unwrap();
    assert_eq!(error.code, "E_RUNTIME");
    assert!(error.message.contains("utf8_validate"));
}
