//! Opt-in measurement keeps the input carrier identical across compiler changes.
use super::*;

#[test]
#[ignore = "explicit same-carrier VM/reference measurement"]
fn selfhost_source_measure_nominal_on_pinned_carrier() {
    let bytes = if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_CARRIER") {
        std::fs::read(path).unwrap()
    } else {
        graph::packet(&compiler().0)
    };
    if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_CAPTURE") {
        use std::io::Write;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        output.write_all(&bytes).unwrap();
        output.sync_all().unwrap();
    }
    assert!(bytes.len() >= 128);
    assert_eq!(&bytes[..8], b"LKJSG002");
    eprintln!(
        "selfhost pinned carrier bytes={} sha256={} state={}",
        bytes.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
        crate::platform::semantic_id::encode_hex(&bytes[48..80])
    );
    let module = "source-nominal-probe";
    let (_, program) = probes(module);
    let entry = program
        .function(function(module, "read-nominal"))
        .unwrap()
        .0 as usize;
    let payload = input::export(program, entry).unwrap();
    let word = |at| u64::from_le_bytes(payload[at..at + 8].try_into().unwrap());
    eprintln!(
        "selfhost validator normalized capacity functions={} nodes={} type_rows={} input_bytes={}",
        word(96),
        word(120),
        word(80),
        payload.len()
    );
    assert_eq!(result("read-nominal", &bytes), (0, bytes[48..80].to_vec()));
}

#[test]
#[ignore = "explicit maintained-closure native measurement"]
fn selfhost_source_measure_maintained_native_closure() {
    use serde_json::json;
    use std::io::Write;
    use std::time::Instant;

    let total_started = Instant::now();
    let started = Instant::now();
    let (source, program) = compiler();
    let initialize_ms = started.elapsed().as_millis();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let repository = GraphRepository::open(&root).unwrap();
    let revision = repository.view_current().unwrap().revision().to_string();
    let entry_declaration = declaration_named(source, "compile-elf");
    let entry = program.function(entry_declaration).unwrap().0 as usize;

    let started = Instant::now();
    let carrier = graph::packet(source);
    let carrier_export_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let validation = kernel::validate_full(source).unwrap();
    let local_validation_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let mode0 = input::export(program, entry).unwrap();
    let mode0_export_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let mode1 =
        crate::platform::compiler::selfhost_artifact_input::bundle(program.artifact()).unwrap();
    let mode1_export_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let c1 = crate::platform::compiler::elf_x86_64::compile_function(program, entry)
        .unwrap()
        .0;
    let bootstrap_ms = started.elapsed().as_millis();

    let run = |image: &[u8], bytes: &[u8]| {
        let started = Instant::now();
        let output = native_filter_tests::execute_filter(image, bytes, &[], 4093);
        let milliseconds = started.elapsed().as_millis();
        assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
        assert!(output.stderr.is_empty());
        (output.stdout, milliseconds)
    };
    let (c2, c1_mode0_ms) = run(&c1, &mode0);
    let (c3, c2_mode0_ms) = run(&c2, &mode0);
    assert!(c2 == c3);
    let (artifact_c1, c1_mode1_ms) = run(&c1, &mode1);
    let (artifact_c2, c2_mode1_ms) = run(&c2, &mode1);
    let (artifact_c3, c3_mode1_ms) = run(&c3, &mode1);
    assert!(artifact_c1 == artifact_c2);
    assert!(artifact_c2 == artifact_c3);
    let started = Instant::now();
    let loaded = crate::platform::compiler::load_artifact(&artifact_c1).unwrap();
    let artifact_load_ms = started.elapsed().as_millis();
    let units = loaded
        .objects
        .keys()
        .filter(|key| key.domain == ObjectDomain::CompilerUnit)
        .count();
    let word = |bytes: &[u8], at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let policy_tag = word(&mode1, 12);
    let allowance = wide_at(&mode1, 32);
    assert_eq!(word(&mode0, 8), 6);
    assert_eq!(word(&mode1, 8), 7);
    assert_eq!(policy_tag, 0);
    assert_eq!(allowance, 0);
    assert_eq!(u64::try_from(units).unwrap(), wide_at(&mode1, 80));
    let identity = |bytes: &[u8]| {
        json!({
            "bytes": bytes.len(),
            "sha256": crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes)),
        })
    };
    // Reproduce the combined source compiler through the already checked
    // Tier 1 backend stages. Its own scalar frontend cannot compile this
    // maintained compiler closure, so this is not source self-compilation.
    let source_entry_declaration = super::function("source-elf", "compile");
    let source_entry = program.function(source_entry_declaration).unwrap().0 as usize;
    let started = Instant::now();
    let source_input = input::export(program, source_entry).unwrap();
    let source_export_ms = started.elapsed().as_millis();
    let source_export = json!({
        "status": "ok",
        "input": identity(&source_input),
        "profile": word(&source_input, 8),
        "functions": wide_at(&source_input, 96),
        "nodes": wide_at(&source_input, 120),
        "types": wide_at(&source_input, 80),
        "exceeds_historical_node_limit": wide_at(&source_input, 120) > 32_768,
    });
    let started = Instant::now();
    let (source_bootstrap_image, source_functions, source_rewrites) =
        crate::platform::compiler::elf_x86_64::compile_function(program, source_entry).unwrap();
    let source_bootstrap_ms = started.elapsed().as_millis();
    let source_bootstrap = json!({
        "status": "ok",
        "image": identity(&source_bootstrap_image),
        "functions": source_functions,
        "rewrites": source_rewrites,
    });
    let (source_from_c1, source_c1_emit_ms) = run(&c1, &source_input);
    let (source_from_c2, source_c2_emit_ms) = run(&c2, &source_input);
    assert!(source_from_c1 == source_from_c2);

    // Rust authors a valid independent seed and transports complete object bytes.
    // It neither normalizes this target nor supplies source typing/lowering facts.
    let mut scalar_source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-scalar.lkjc"
    ))
    .unwrap();
    let selected_target = super::scalar::target_id(&scalar_source, "scalar-main");
    let scalar_carrier = graph::packet(&scalar_source);
    let scalar_request = super::scalar::source_request(&scalar_carrier, selected_target);
    let unused_bytes = kernel::TypeObject::new(kernel::TypeForm::Bytes).unwrap();
    let unused_digest = kernel::encode_type_object(&unused_bytes).unwrap().0;
    assert!(
        scalar_source
            .types
            .insert(unused_digest, unused_bytes)
            .is_none()
    );
    let rejected_carrier = graph::packet(&scalar_source);
    let rejected_request = super::scalar::source_request(&rejected_carrier, selected_target);
    assert_eq!(&scalar_carrier[48..80], &rejected_carrier[48..80]);
    assert_ne!(scalar_carrier, rejected_carrier);
    assert_ne!(
        Sha256::digest(&scalar_carrier),
        Sha256::digest(&rejected_carrier)
    );
    assert_ne!(
        Sha256::digest(&scalar_request),
        Sha256::digest(&rejected_request)
    );
    let mut expected_scalar_image: Option<Vec<u8>> = None;
    let mut scalar_controls = Vec::new();
    for (stage, image) in [
        ("Rust bootstrap", &source_bootstrap_image),
        ("backend C1 emission", &source_from_c1),
        ("backend C2 emission", &source_from_c2),
    ] {
        let (scalar_image, compile_ms) = run(image, &scalar_request);
        if let Some(expected) = &expected_scalar_image {
            assert_eq!(scalar_image.as_slice(), expected.as_slice(), "{stage}");
        } else {
            expected_scalar_image = Some(scalar_image.clone());
        }
        let output = execute(&scalar_image, &[]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{stage}: {:?}",
            output.stderr
        );
        assert_eq!(output.stdout, b"42", "{stage}");
        assert!(output.stderr.is_empty(), "{stage}");
        let started = Instant::now();
        let rejected = native_filter_tests::execute_filter(image, &rejected_request, &[], 4093);
        let reject_ms = started.elapsed().as_millis();
        assert_eq!(
            rejected.status.code(),
            Some(65),
            "{stage}: {:?}",
            rejected.stderr
        );
        assert!(rejected.stdout.is_empty(), "{stage}");
        assert_eq!(rejected.stderr, b"compiler_rejected:13\n", "{stage}");
        scalar_controls.push(json!({
            "stage": stage,
            "compiler": identity(image),
            "accepted_carrier": identity(&scalar_carrier),
            "accepted_request": identity(&scalar_request),
            "request_format": "LKJSRQ01",
            "request_profile": 1,
            "selected_target": selected_target,
            "selector_key_hex": crate::platform::semantic_id::encode_hex(
                &kernel::EncodedOwnerKey::new(OwnerKey::Target(selected_target)).bytes()
            ),
            "emitted_program": identity(&scalar_image),
            "program_exit": output.status.code(),
            "program_stdout": "42",
            "rejected_carrier": identity(&rejected_carrier),
            "rejected_request": identity(&rejected_request),
            "rejection": "hash-consistent unused Bytes type",
            "rejection_diagnostic": 13,
            "compile_wall_time_ms": compile_ms,
            "reject_wall_time_ms": reject_ms,
        }));
    }
    let report = json!({
        "schema": "lkjscript.selfhost.maintained-native-measurement.v1",
        "scope": "Tier 1 maintained closure; Rust normalized inputs and C1 bootstrap",
        "revision": revision,
        "semantic_state": kernel::semantic_state_digest(source).unwrap().to_string(),
        "complete_source_carrier": identity(&carrier),
        "owners": source.owners.len(),
        "retirements": source.retirements.len(),
        "types": source.types.len(),
        "entry": entry_declaration,
        "source_compiler_capacity": {
            "scope": "Tier 1 backend reproduction and explicitly selected scalar source execution; no maintained source self-compilation",
            "entry": source_entry_declaration,
            "target": "linux-x86_64",
            "bootstrap": "Rust elf_x86_64::compile_function",
            "export_input_consumed": true,
            "bootstrap_image_executed": true,
            "normalized_export": source_export,
            "native_bootstrap": source_bootstrap,
            "normalized_reproduction": {
                "backend_C1": identity(&c1),
                "backend_C2": identity(&c2),
                "compiler_from_backend_C1": identity(&source_from_c1),
                "compiler_from_backend_C2": identity(&source_from_c2),
                "identical": true,
            },
            "source_request_controls": scalar_controls,
        },
        "mode0": {
            "input": identity(&mode0),
            "profile": word(&mode0, 8),
            "functions": wide_at(&mode0, 96),
            "nodes": wide_at(&mode0, 120),
            "types": wide_at(&mode0, 80),
        },
        "mode1": {
            "input": identity(&mode1),
            "profile": word(&mode1, 8),
            "units": wide_at(&mode1, 80),
            "source_objects": wide_at(&mode1, 96),
            "policy_tag": policy_tag,
            "allowance": allowance,
        },
        "compiler_C1": identity(&c1),
        "compiler_C2": identity(&c2),
        "compiler_C3": identity(&c3),
        "artifact_C1": identity(&artifact_c1),
        "artifact_C2": identity(&artifact_c2),
        "artifact_C3": identity(&artifact_c3),
        "emitted_canonical_units": units,
        "direct_local_full_kernel_work": validation.work_consumed,
        "wall_time_ms": {
            "compiler_initialize": initialize_ms,
            "carrier_export": carrier_export_ms,
            "local_full_validation": local_validation_ms,
            "mode0_export": mode0_export_ms,
            "mode1_export": mode1_export_ms,
            "C1_bootstrap": bootstrap_ms,
            "source_compiler_export": source_export_ms,
            "source_compiler_bootstrap": source_bootstrap_ms,
            "source_compiler_from_backend_C1": source_c1_emit_ms,
            "source_compiler_from_backend_C2": source_c2_emit_ms,
            "native_filter_harness": {
                "C1_mode0": c1_mode0_ms,
                "C2_mode0": c2_mode0_ms,
                "C1_mode1": c1_mode1_ms,
                "C2_mode1": c2_mode1_ms,
                "C3_mode1": c3_mode1_ms,
            },
            "strict_artifact_load": artifact_load_ms,
            "total_before_report_encoding": total_started.elapsed().as_millis(),
        },
        "execution_context": {
            "os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "pointer_bits": usize::BITS,
            "native_environment": "empty",
            "input_chunk_bytes": 4093,
            "child_deadline_seconds": 10,
            "harness_output_limit_bytes": 32 * 1024 * 1024,
            "timing_scope": "publication, process creation, transport, execution, wait and cleanup",
        },
        "not_measured": [
            "reachable historical storage bytes",
            "native requested allocation bytes",
            "native live retained storage",
            "native mapped arena capacity",
            "native resident memory",
            "maintained-compiler source-driven fixed points",
        ],
    });
    let mut encoded = serde_json::to_vec_pretty(&report).unwrap();
    encoded.push(b'\n');
    if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_NATIVE_MEASUREMENT_OUTPUT") {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        output.write_all(&encoded).unwrap();
        output.sync_all().unwrap();
    }
    println!("{}", String::from_utf8(encoded).unwrap());
}
