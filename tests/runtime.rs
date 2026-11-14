use lkjscript::{compiler, interpreter, runtime, runtime_memory, source};

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
        "{}\n{}",
        include_str!("../library/runtime.rs"),
        include_str!("runtime/allocator.rs")
    )
}
fn prepared() -> runtime::Runtime {
    runtime::seal(compiler::check_bundle(&bundle(source_text())).unwrap()).unwrap()
}
fn value(
    runtime: &runtime::Runtime,
    name: &str,
    arguments: &[u64],
    memory: &mut runtime_memory::Memory,
) -> u64 {
    let result = interpreter::run_runtime(
        runtime.checked_program(),
        runtime.function(name).unwrap(),
        arguments,
        memory,
        &mut Vec::new(),
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message));
    match result.value {
        interpreter::Scalar::Int { value, .. } => value,
        _ => panic!("expected a runtime word"),
    }
}

#[test]
fn source_allocator_reuses_regions_and_releases_dedicated_mappings() {
    let runtime = prepared();
    let mut memory = runtime_memory::Memory::new();
    assert_eq!(value(&runtime, "probe_allocator", &[], &mut memory), 42);
    let retained = memory.mapped_bytes();
    let regions = memory.mapping_count();
    assert_eq!(value(&runtime, "probe_allocator", &[], &mut memory), 42);
    assert_eq!(memory.mapped_bytes(), retained);
    assert_eq!(memory.mapping_count(), regions);
    let medium = value(&runtime, "allocate", &[65536], &mut memory);
    assert_eq!(memory.mapped_bytes(), retained);
    value(&runtime, "release", &[medium], &mut memory);
    assert_eq!(value(&runtime, "allocate", &[65536], &mut memory), medium);
    value(&runtime, "release", &[medium], &mut memory);
    let large = value(&runtime, "allocate", &[2097152], &mut memory);
    assert!(memory.mapped_bytes() > retained);
    value(&runtime, "release", &[large], &mut memory);
    assert_eq!(memory.mapped_bytes(), retained);
    assert_eq!(
        memory.read(large, 1),
        Err(runtime_memory::MemoryFault::Bounds)
    );
}

#[test]
fn region_growth_preserves_live_blocks_and_reuses_medium_classes() {
    let runtime = prepared();
    let mut memory = runtime_memory::Memory::new();
    assert_eq!(
        value(&runtime, "probe_medium_regions", &[], &mut memory),
        42
    );
    let retained = memory.mapped_bytes();
    let regions = memory.mapping_count();
    assert_eq!(
        value(&runtime, "probe_medium_regions", &[], &mut memory),
        42
    );
    assert_eq!(memory.mapped_bytes(), retained);
    assert_eq!(memory.mapping_count(), regions);
}

#[test]
fn refused_geometric_region_retries_a_smaller_request() {
    let runtime = prepared();
    let mut memory = runtime_memory::Memory::new();
    memory.refuse_next_mapping();
    let pointer = value(&runtime, "allocate", &[8], &mut memory);
    memory.write(pointer, &[42]).unwrap();
    assert_eq!(memory.read(pointer, 1).unwrap(), &[42]);
    assert_eq!(
        memory.mapped_bytes(),
        runtime::ALLOCATOR_STATE_BYTES as usize + 32768
    );
    value(&runtime, "release", &[pointer], &mut memory);
}

#[test]
fn source_utf8_validation_matches_independent_decoding() {
    let runtime = prepared();
    let mut memory = runtime_memory::Memory::new();
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
    for bytes in cases {
        let address = value(&runtime, "allocate", &[bytes.len() as u64], &mut memory);
        memory.write(address, &bytes).unwrap();
        assert_eq!(
            value(
                &runtime,
                "utf8_validate",
                &[address, bytes.len() as u64],
                &mut memory
            ),
            u64::from(std::str::from_utf8(&bytes).is_ok())
        );
        value(&runtime, "release", &[address], &mut memory);
    }
}

#[test]
fn kernel_refusal_and_unrepresentable_allocation_report_heap_failure() {
    let runtime = prepared();
    let mut memory = runtime_memory::Memory::new();
    memory.refuse_mappings();
    let refused = interpreter::run_runtime(
        runtime.checked_program(),
        runtime.function("allocate").unwrap(),
        &[8],
        &mut memory,
        &mut Vec::new(),
    )
    .err()
    .unwrap();
    assert_eq!(refused.code(), "R0006");
    let overflow = interpreter::run_runtime(
        runtime.checked_program(),
        runtime.function("probe_overflow").unwrap(),
        &[],
        &mut memory,
        &mut Vec::new(),
    )
    .err()
    .unwrap();
    assert_eq!(overflow.code(), "R0006");
}

#[test]
fn raw_names_require_an_explicit_checked_runtime_seal() {
    let input = bundle(source_text());
    let checked = compiler::check_bundle(&input).unwrap();
    let entry = checked
        .program()
        .functions
        .iter()
        .position(|function| function.name == "probe_allocator")
        .unwrap();
    let result = interpreter::run(checked.program(), entry, &[], &mut Vec::new());
    assert!(matches!(result.err().unwrap().kind, interpreter::ErrorKind::Trap(code) if code >= 16));
    let replaced = input.files[0]
        .text
        .replacen("panic!(\"unsealed runtime primitive\")", "0", 1);
    assert!(runtime::seal(compiler::check_bundle(&bundle(replaced)).unwrap()).is_err());
    let wrong_entry = input.files[0].text.replace(
        "pub fn utf8_validate(data: u64, length: u64) -> u64 {\n    valid_utf8_bytes(data, length)\n}",
        "pub fn utf8_validate(data: u64) -> u64 { valid_utf8_bytes(data, 0) }",
    );
    let error = runtime::seal(compiler::check_bundle(&bundle(wrong_entry)).unwrap())
        .err()
        .unwrap();
    assert_eq!(error.code, "E_RUNTIME");
    assert!(error.message.contains("utf8_validate"));
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn native_result(name: &str, bytes: Vec<u8>) -> Result<u64, String> {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/native-runtime");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{}-{name}.elf", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = std::process::Command::new(&path).output();
    std::fs::remove_file(path).unwrap();
    let output = output.unwrap();
    assert!(output.stdout.is_empty());
    if !output.status.success() {
        return Err(String::from_utf8(output.stderr).unwrap());
    }
    assert_eq!(output.stderr.len(), 8, "native scalar result: {output:?}");
    Ok(u64::from_le_bytes(output.stderr.try_into().unwrap()))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn native_allocator_matches_its_independent_raw_model() {
    let runtime = prepared();
    for name in ["probe_allocator", "probe_medium_regions"] {
        let mut memory = runtime_memory::Memory::new();
        let expected = value(&runtime, name, &[], &mut memory);
        let image = lkjscript::native::encode_runtime(
            &runtime,
            lkjscript::native::Entry {
                function: runtime.function(name).unwrap(),
                arguments: vec![],
                result_fd: Some(2),
            },
        )
        .unwrap_or_else(|error| panic!("{}: {}", error.code, error.message));
        assert_eq!(native_result(name, image.bytes), Ok(expected));
    }

    let ordinary = compiler::check_bundle(&bundle(source_text())).unwrap();
    let index = ordinary
        .program()
        .functions
        .iter()
        .position(|function| function.name == "probe_allocator")
        .unwrap();
    let image = lkjscript::native::encode(
        &ordinary,
        &runtime,
        lkjscript::native::Entry {
            function: index,
            arguments: vec![],
            result_fd: Some(2),
        },
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.code, error.message));
    let error = native_result("unsealed", image.bytes).unwrap_err();
    assert!(error.starts_with("R0016 at line "), "{error}");
}
