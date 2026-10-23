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
    assert_eq!(&bytes[..8], b"LKJSG001");
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
    let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    eprintln!(
        "selfhost validator normalized capacity functions={} nodes={} type_rows={} input_bytes={}",
        word(92),
        word(100),
        word(84),
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
    let allowance = u64::from_le_bytes(mode1[32..40].try_into().unwrap());
    assert_eq!(word(&mode0, 8), 4);
    assert_eq!(word(&mode1, 8), 5);
    assert_eq!(allowance, 0);
    assert_eq!(units, word(&mode1, 76) as usize);
    let identity = |bytes: &[u8]| {
        json!({
            "bytes": bytes.len(),
            "sha256": crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes)),
        })
    };
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
        "mode0": {
            "input": identity(&mode0),
            "profile": word(&mode0, 8),
            "functions": word(&mode0, 92),
            "nodes": word(&mode0, 100),
            "types": word(&mode0, 84),
        },
        "mode1": {
            "input": identity(&mode1),
            "profile": word(&mode1, 8),
            "units": word(&mode1, 76),
            "source_objects": word(&mode1, 84),
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
            "source-driven E2/E3",
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
