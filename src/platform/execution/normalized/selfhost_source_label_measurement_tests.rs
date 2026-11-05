//! Opt-in same-binary comparison of the accepted canonical packing helper.
//!
//! The compiler is read from the current public GraphRepository by compiler().
//! This is an internal no-Match trace measurement, not source admission, native
//! compilation, a fixed point, or a full staged-tree acceptance claim.

use super::*;
use crate::platform::execution::normalized::child_storage::Fields;
use crate::platform::kernel::{DeclarationPayload, Name, semantic_state_digest};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

const MEASUREMENT_SCHEMA: &str = "lkjscript.canonical-label-index-measurement.v1";

fn digest(bytes: &[u8]) -> String {
    crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes))
}

fn file_digest(path: &Path) -> Check<String> {
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("measurement file {}: {error}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let length = file
            .read(&mut buffer)
            .map_err(|error| format!("measurement read {}: {error}", path.display()))?;
        if length == 0 {
            return Ok(crate::platform::semantic_id::encode_hex(&hash.finalize()));
        }
        hash.update(&buffer[..length]);
    }
}

fn environment(name: &str) -> Check<String> {
    std::env::var(name).map_err(|error| format!("measurement requires {name}: {error}"))
}

fn emit(value: &Value) -> Check<()> {
    let line =
        serde_json::to_string(value).map_err(|error| format!("measurement JSON: {error}"))?;
    eprintln!("LABEL_INDEX_MEASUREMENT {line}");
    Ok(())
}

fn owner_binding(
    view: &crate::platform::publication::RepositoryView,
    source: &KernelSnapshot,
    raw: &raw_source::RawSourceGraph<'_>,
    key: OwnerKey,
) -> Check<Value> {
    let public = view
        .owner(key)
        .map_err(|error| format!("measurement public owner: {error:?}"))?;
    equal(
        public.revision,
        view.revision(),
        "public helper read revision",
    )?;
    equal(
        public.value.as_ref(),
        source.owners.get(&key),
        "public helper read matches prepared compiler snapshot",
    )?;
    ensure(
        public.value.is_some(),
        "helper owner absent from public readback",
    )?;
    let encoded = EncodedOwnerKey::new(key).bytes();
    let row = raw
        .owners
        .iter()
        .find(|row| row.key == encoded)
        .ok_or_else(|| "helper owner absent from complete public export".to_owned())?;
    Ok(json!({
        "owner_key": crate::platform::semantic_id::encode_hex(&encoded),
        "canonical_owner_digest": crate::platform::semantic_id::encode_hex(&row.digest),
        "canonical_bytes_sha256": digest(row.bytes),
        "canonical_bytes": row.bytes.len(),
    }))
}

fn public_binding(source: &KernelSnapshot, stage: &str, root: &Path) -> Check<Value> {
    let repository = GraphRepository::open(root)
        .map_err(|error| format!("measurement public repository: {error:?}"))?;
    let view = repository
        .view_current()
        .map_err(|error| format!("measurement public current: {error:?}"))?;
    let exported = view
        .export_package_transport()
        .map_err(|error| format!("measurement public export: {error:?}"))?;
    let container = PackageContainer::decode(&exported.container, exported.transport_digest)
        .map_err(|error| format!("measurement public container: {error:?}"))?;
    let original = ExportedSource::from_container(&container)
        .map_err(|error| format!("measurement complete raw export: {error:?}"))?;
    let raw = original.as_raw();
    equal(
        raw.provenance.claimed_revision,
        view.revision().bytes(),
        "original revision binding",
    )?;
    equal(
        raw.owners.len(),
        source.owners.len(),
        "complete compiler owner inventory",
    )?;
    equal(
        raw.types.len(),
        source.types.len(),
        "complete supplied compiler type inventory",
    )?;
    equal(
        raw.retirements.len(),
        source.retirements.len(),
        "complete compiler retirement inventory",
    )?;
    equal(
        raw.package,
        source.root.package_id.bytes(),
        "compiler package binding",
    )?;
    equal(
        raw.semantic_state,
        semantic_state_digest(source)
            .map_err(|error| format!("measurement semantic state: {error:?}"))?
            .bytes(),
        "public export matches compiler semantic state",
    )?;
    let mut helpers = Vec::new();
    let mut index_present = false;
    let mut packing_present = false;
    for (module_key, module_owner) in &source.owners {
        let (OwnerKey::Module(module_id), OwnerRecord::Module(module)) = (module_key, module_owner)
        else {
            continue;
        };
        if !matches!(
            module.name.as_str(),
            "source-unit-code-pack"
                | "source-unit-code-index"
                | "source-unit-tables"
                | "wire"
                | "ci"
        ) {
            continue;
        }
        if module.name.as_str() == "source-unit-code-index" {
            index_present = true;
        } else if module.name.as_str() == "source-unit-code-pack" {
            packing_present = true;
        }
        for (key, owner) in &source.owners {
            let OwnerRecord::Declaration(declaration) = owner else {
                continue;
            };
            if declaration.module != *module_id {
                continue;
            }
            let body = match &declaration.payload {
                DeclarationPayload::Function(function) => {
                    owner_binding(&view, source, &raw, OwnerKey::Expression(function.body))?
                }
                DeclarationPayload::External(_) => Value::Null,
                _ => {
                    return Err(
                        "measurement helper module contains an unexpected declaration".to_owned(),
                    );
                }
            };
            helpers.push(json!({
                "module": module.name.as_str(),
                "name": declaration.name.as_str(),
                "declaration": owner_binding(&view, source, &raw, *key)?,
                "body_root": body,
            }));
        }
    }
    ensure(
        packing_present,
        "current graph has no canonical packing helper",
    )?;
    equal(
        index_present,
        stage == "indexed",
        "stage-specific index module inventory",
    )?;
    if index_present {
        function_named(source, "source-unit-code-index", "prepare")?;
        function_named(source, "source-unit-code-index", "target")?;
    }
    let carrier = raw_source::carrier(&raw)
        .map_err(|error| format!("measurement complete raw carrier: {error:?}"))?;
    Ok(json!({
        "graph_revision": view.revision().to_string(),
        "semantic_state": crate::platform::semantic_id::encode_hex(&raw.semantic_state),
        "source_profile": raw_source::SOURCE_PROFILE,
        "source_carrier_sha256": digest(&carrier),
        "source_carrier_bytes": carrier.len(),
        "canonical_revision_core_sha256": digest(raw.provenance.canonical_core),
        "package_transport_sha256": digest(&exported.container),
        "package_revision_sha256": digest(&exported.revision_bytes),
        "owners": raw.owners.len(), "types": raw.types.len(), "retirements": raw.retirements.len(),
        "helper_public_readback": helpers,
        "scope": "whole original exported graph; no synthesized revision and no embedded proposal reconstruction",
    }))
}

struct Fixture {
    kind: &'static str,
    blocks: usize,
    count: i64,
    trace: Vec<u8>,
    expected: Vec<u8>,
}

fn append_words(out: &mut Vec<u8>, words: &[u64]) {
    for word in words {
        out.extend_from_slice(&word.to_le_bytes());
    }
}

fn signed_bits(value: i64) -> u64 {
    u64::from_le_bytes(value.to_le_bytes())
}

fn fixture(blocks: usize, branches: bool) -> Check<Fixture> {
    let count = blocks
        .checked_mul(6)
        .and_then(|value| value.checked_add(2))
        .ok_or_else(|| "measurement fixture instruction count overflow".to_owned())?;
    let count =
        i64::try_from(count).map_err(|error| format!("measurement fixture count: {error}"))?;
    let mut trace = Vec::new();
    let mut expected = Vec::new();
    append_words(
        &mut expected,
        &[
            0,
            0,
            1056,
            u64::try_from(count).map_err(|error| error.to_string())?,
        ],
    );
    if branches {
        for index in 0..blocks {
            let owner = u64::try_from(blocks - index).map_err(|error| error.to_string())?;
            let pc = u64::try_from(index * 6).map_err(|error| error.to_string())?;
            let condition = u64::try_from(index % 2).map_err(|error| error.to_string())?;
            let value = i64::try_from(index + 1).map_err(|error| error.to_string())?;
            for row in [
                [3, condition, 0, 0, 0, 0],
                [10, owner, 0, 0, 0, 0],
                [4, 0, 0, signed_bits(value), 0, 0],
                [11, owner, 1, 0, 0, 0],
                [0, owner, 0, pc + 4, 0, 0],
                [4, 0, 0, signed_bits(-value), 0, 0],
                [0, owner, 1, pc + 5, 0, 0],
                [9, 0, 0, 0, 0, 0],
            ] {
                append_words(&mut trace, &row);
            }
            // Independent PC formulas, not label lookup or scanning of trace.
            for row in [
                [3, condition, 0, 0, 0, 0],
                [10, pc + 4, 0, 0, 0, 0],
                [4, 0, 0, signed_bits(value), 0, 0],
                [11, pc + 5, 0, 0, 0, 0],
                [4, 0, 0, signed_bits(-value), 0, 0],
                [9, 0, 0, 0, 0, 0],
            ] {
                append_words(&mut expected, &row);
            }
        }
    } else {
        for index in 0..(blocks * 3) {
            let value = u64::try_from(index + 1).map_err(|error| error.to_string())?;
            for row in [[4, 0, 0, value, 0, 0], [9, 0, 0, 0, 0, 0]] {
                append_words(&mut trace, &row);
                append_words(&mut expected, &row);
            }
        }
    }
    for row in [[4, 0, 0, 42, 0, 0], [27, 0, 0, 0, 0, 0]] {
        append_words(&mut trace, &row);
        append_words(&mut expected, &row);
    }
    Ok(Fixture {
        kind: if branches { "if-labels" } else { "straight" },
        blocks,
        count,
        trace,
        expected,
    })
}

fn arguments(fixture: &Fixture) -> Check<Vec<NormalizedValue>> {
    let fields = [
        ("code", NormalizedValue::I64(0)),
        ("count", NormalizedValue::I64(fixture.count)),
        ("locals", NormalizedValue::I64(0)),
        ("pool", NormalizedValue::bytes(Vec::<u8>::new())),
        ("trace", NormalizedValue::bytes(fixture.trace.as_slice())),
    ]
    .into_iter()
    .map(|(name, value)| {
        Name::new(name.to_owned())
            .map(|name| (name, value))
            .map_err(|error| format!("measurement field name: {error:?}"))
    })
    .collect::<Check<Vec<_>>>()?;
    let fields = Fields::try_new(fields)
        .map_err(|error| format!("measurement structural record: {error:?}"))?;
    Ok(vec![
        NormalizedValue::Record(NormalizedRecord::Structural { fields }),
        NormalizedValue::I64(0),
        NormalizedValue::I64(1024),
    ])
}

fn verify_result(value: &NormalizedValue, fixture: &Fixture) -> Check<String> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("measurement project result is not structural".to_owned());
    };
    let field = |name| {
        fields
            .iter()
            .find_map(|(key, value)| (key.as_str() == name).then_some(value))
    };
    match field("code") {
        Some(NormalizedValue::I64(0)) => {}
        other => return Err(format!("measurement project failed: {other:?}")),
    }
    let Some(NormalizedValue::Bytes(bytes)) = field("bytes") else {
        return Err("measurement project result has no bytes".to_owned());
    };
    equal_bytes(
        bytes.as_ref(),
        &fixture.expected,
        "independent exact packed oracle",
    )?;
    Ok(digest(bytes.as_ref()))
}

fn invoke(
    vm: &NormalizedVm<'_>,
    policy: NormalizedRunPolicy,
    entry: DeclarationReference,
    fixture: &Fixture,
) -> Check<Value> {
    // Input construction and its host allocations are intentionally outside the timer.
    let arguments = arguments(fixture)?;
    let control = ExecutionControl::uncancelled();
    let started = Instant::now();
    let result = vm.invoke(entry, arguments, None, &control);
    let elapsed = started.elapsed();
    let (value, observation) =
        result.map_err(|error| format!("measurement helper invocation: {error:?}"))?;
    // Output checking, hashing, JSON serialization and disposal are outside the timer.
    let output_sha256 = verify_result(&value, fixture)?;
    equal(
        observation.live_call_frames_after,
        0,
        "measurement VM frames reclaimed",
    )?;
    Ok(json!({
        "case": fixture.kind, "blocks": fixture.blocks,
        "instructions_in_packed_output": fixture.count,
        "trace_bytes": fixture.trace.len(), "trace_sha256": digest(&fixture.trace),
        "packed_bytes": fixture.expected.len(), "packed_sha256": output_sha256,
        "helper_wall_elapsed_ns": elapsed.as_nanos().to_string(),
        "policy": format!("{policy:?}"),
        "modeled_observation": observation,
        "observation_scope": "NormalizedVm accounting; allocated_bytes is modeled cumulative allocation, not allocator traffic or RSS",
    }))
}

#[test]
#[ignore = "opt-in causal measurement; requires both graph stages with one unchanged Cargo-selected binary"]
fn selfhost_source_label_index_causal_measurement() -> Check<()> {
    let stage = environment("LKJ_LABEL_MEASURE_STAGE")?;
    ensure(
        matches!(stage.as_str(), "baseline" | "indexed"),
        "stage must be baseline or indexed",
    )?;
    let blocks = environment("LKJ_LABEL_MEASURE_BLOCKS")?
        .parse::<usize>()
        .map_err(|error| format!("measurement blocks: {error}"))?;
    ensure(
        matches!(blocks, 64 | 256 | 1024),
        "select the 64, 256 or 1024 fixture",
    )?;
    let repetitions = environment("LKJ_LABEL_MEASURE_REPEATS")?
        .parse::<usize>()
        .map_err(|error| format!("measurement repetitions: {error}"))?;
    ensure(repetitions > 0, "select at least one measurement pair")?;
    let measured_invocations = repetitions
        .checked_mul(2)
        .ok_or_else(|| "measurement repetition count overflow".to_owned())?;
    let expected_binary = environment("LKJ_LABEL_MEASURE_BINARY_SHA256")?;
    let expected_head = environment("LKJ_LABEL_MEASURE_EXPECTED_HEAD_SHA256")?;
    let executable =
        std::env::current_exe().map_err(|error| format!("measurement executable: {error}"))?;
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/deps")
        .canonicalize()
        .map_err(|error| format!("measurement shared target: {error}"))?;
    ensure(
        executable.parent() == Some(target.as_path()),
        "measurement must run the original shared-target test executable",
    )?;
    equal(
        file_digest(&executable)?,
        expected_binary.clone(),
        "pinned Cargo-selected executable digest",
    )?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let head = root.join("HEAD");
    equal(
        file_digest(&head)?,
        expected_head.clone(),
        "selected public graph HEAD digest",
    )?;
    let prepared = Instant::now();
    // This helper opens GraphRepository::view_current, not proposed_compiler().
    let (source, program) = compiler();
    let preparation_ns = prepared.elapsed().as_nanos().to_string();
    let exported = Instant::now();
    let binding = public_binding(source, &stage, &root)?;
    let export_ns = exported.elapsed().as_nanos().to_string();
    equal(
        file_digest(&head)?,
        expected_head.clone(),
        "graph unchanged through preparation and public binding",
    )?;
    let setup = Instant::now();
    let branch = fixture(blocks, true)?;
    let straight = fixture(blocks, false)?;
    let entry = function_named(source, "source-unit-code-pack", "project")?;
    let setup_ns = setup.elapsed().as_nanos().to_string();
    let vm_setup = Instant::now();
    let policy = NormalizedRunPolicy::foreground();
    let vm = NormalizedVm::for_test(program, policy);
    let vm_setup_ns = vm_setup.elapsed().as_nanos().to_string();
    emit(&json!({
        "schema": MEASUREMENT_SCHEMA, "event": "binding", "stage": stage,
        "engine": "normalized_vm", "test_executable": executable,
        "test_executable_sha256": expected_binary, "public_head_sha256": expected_head,
        "selected_environment": {
            "PATH": std::env::var("PATH").ok(), "LANG": std::env::var("LANG").ok(),
            "LC_ALL": std::env::var("LC_ALL").ok(), "TZ": std::env::var("TZ").ok(),
            "RUST_BACKTRACE": std::env::var("RUST_BACKTRACE").ok(),
        },
        "graph": binding,
        "setup": {
            "graph_read_and_c0_prepare_wall_ns": preparation_ns,
            "public_export_and_helper_readback_wall_ns": export_ns,
            "fixture_and_entry_setup_wall_ns": setup_ns,
            "vm_clone_and_executor_setup_wall_ns": vm_setup_ns,
        },
        "warmups": 2, "measured_invocations": measured_invocations,
        "actual_allocator_traffic": null, "copied_bytes": null,
        "peak_live_bytes": null, "mapped_native_arena_bytes": null, "rss": null,
        "raw_source_or_native_stage_claim": false,
    }))?;
    for branches in [true, false] {
        invoke(&vm, policy, entry, &fixture(4, branches)?)?;
    }
    for repetition in 0..repetitions {
        // Alternation is within each graph stage; phases are not temporally interleaved.
        let order = if repetition % 2 == 0 {
            [&branch, &straight]
        } else {
            [&straight, &branch]
        };
        for (position, fixture) in order.into_iter().enumerate() {
            let mut result = invoke(&vm, policy, entry, fixture)?;
            let object = result
                .as_object_mut()
                .ok_or_else(|| "measurement report is not an object".to_owned())?;
            object.insert("schema".to_owned(), json!(MEASUREMENT_SCHEMA));
            object.insert("event".to_owned(), json!("invocation"));
            object.insert("stage".to_owned(), json!(stage));
            object.insert("repetition".to_owned(), json!(repetition));
            object.insert("position_in_pair".to_owned(), json!(position));
            emit(&result)?;
        }
    }
    equal(
        file_digest(&head)?,
        expected_head,
        "public graph unchanged throughout measurement",
    )?;
    equal(
        file_digest(&executable)?,
        expected_binary,
        "Cargo-selected executable unchanged throughout measurement",
    )?;
    emit(
        &json!({"schema": MEASUREMENT_SCHEMA, "event": "terminal", "stage": stage, "passed": true}),
    )
}
