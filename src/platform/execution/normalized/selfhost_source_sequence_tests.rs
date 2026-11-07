//! Publicly authored flat Sequences cross coupled owner, unit and artifact limits.
//! Compiler inputs are complete raw carriers; independent C0 outputs are never
//! supplied to the source compiler. Large controls are explicit acceptance work.

use super::*;
use crate::platform::execution::normalized::NormalizedRunObservation;
use crate::platform::execution::normalized::byte_buffer::StorageObservation;
use crate::platform::execution::normalized::reference::NormalizedReferenceObservation;
use crate::platform::kernel::ExpressionOperation;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::io::Read;
use std::sync::Mutex;

#[path = "selfhost_source_sequence_preparation_tests.rs"]
mod preparation;

fn proposal(count: usize, expected: i64, locals: bool) -> Check<String> {
    ensure(count > 0, "Sequence fixture must have at least one item")?;
    let mut text = String::new();
    text.try_reserve(count.checked_mul(32).ok_or("Sequence text extent")?)
        .map_err(|_| "Sequence text allocation")?;
    text.push_str("declarations.begin\n(units (module create sequence-wide\n");
    text.push_str("(function create entry (visibility public) (effect pure) (returns I64) (body ");
    if locals {
        text.push_str("(let (binding value (i64 9223372036854775807)) (in ");
    }
    text.push_str("(sequence");
    for _ in 1..count {
        text.push_str(if locals {
            " (local value)"
        } else {
            " (i64 9223372036854775807)"
        });
    }
    write!(text, " (i64 {expected}))").map_err(|_| "Sequence result text")?;
    if locals {
        text.push_str("))");
    }
    text.push_str("))\n(component create app (visibility public) (port create run (type (function () I64)) (function entry))))\n");
    text.push_str("(target create sequence-wide (component sequence-wide::app) (runner command) (port sequence-wide::app::run)))\ndeclarations.end\n");
    Ok(text)
}

fn emit(value: Value) {
    eprintln!("SEQUENCE_CAPACITY {}", value);
}

fn public_case(text: &str) -> Check<InputCase> {
    let started = Instant::now();
    let source = byte_buffer_tests::author_only(text)?;
    emit(
        json!({"event": "public_authoring", "proposal_bytes": text.len(),
        "proposal_sha256": sha256(text.as_bytes()), "owners": source.owners.len(),
        "elapsed_ms": started.elapsed().as_millis().to_string()}),
    );
    let started = Instant::now();
    let case = case_from_snapshot(&source)?;
    emit(
        json!({"event": "complete_public_export", "package_revision": case.revision_digest.to_string(),
        "carrier_sha256": sha256(&case.carrier), "carrier_bytes": case.carrier.len(),
        "supplied_types": case.source.types.len(), "owners": case.source.owners.len(),
        "elapsed_ms": started.elapsed().as_millis().to_string()}),
    );
    Ok(case)
}

fn public_disk_case(text: &str, path: &Path) -> Check<InputCase> {
    let started = Instant::now();
    let empty = empty_normalized_snapshot(b"wide-sequence");
    let repository = GraphRepository::create(path, &empty, None)
        .map_err(|error| format!("Sequence public repository creation: {error:?}"))?
        .repository;
    let base = repository
        .view_current()
        .map_err(|error| format!("Sequence initial public read: {error:?}"))?
        .revision();
    let input = format!("request base={base}\n{text}");
    let decoded =
        crate::platform::control::decode_compact_change("wide-sequence", input.as_bytes())
            .map_err(|error| format!("Sequence public change: {error:?}"))?;
    let plan = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .map_err(|error| format!("Sequence public plan: {error:?}"))?;
    repository
        .publish(&plan.publication)
        .map_err(|error| format!("Sequence public apply: {error:?}"))?;
    drop(plan);
    drop(decoded.semantic);
    emit(
        json!({"event": "public_disk_authoring", "path": path, "proposal_bytes": text.len(),
        "proposal_sha256": sha256(text.as_bytes()), "elapsed_ms": started.elapsed().as_millis().to_string()}),
    );
    InputCase::from_repository(repository)
}

fn reclaimed(observation: &NormalizedRunObservation, label: &str) -> Check<()> {
    for live in [
        observation.live_call_frames_after,
        observation.live_locals_after,
        observation.live_type_bindings_after,
        observation.live_operands_after,
        observation.live_transactions_after,
        observation.live_handles_after,
    ] {
        equal(live, 0, label)?;
    }
    ensure(
        !observation.instructions_saturated && !observation.allocated_bytes_saturated,
        "finite Sequence observation unexpectedly saturated",
    )
}

fn reference_reclaimed(observation: &NormalizedReferenceObservation, label: &str) -> Check<()> {
    for live in [
        observation.live_call_frames_after,
        observation.live_control_frames_after,
        observation.live_local_scopes_after,
        observation.live_type_scopes_after,
        observation.live_effect_scopes_after,
        observation.live_allowances_after,
        observation.live_transactions_after,
        observation.live_handles_after,
    ] {
        equal(live, 0, label)?;
    }
    ensure(
        !observation.expressions_saturated && !observation.allocated_bytes_saturated,
        "finite Sequence reference observation unexpectedly saturated",
    )
}

fn code_inventory(case: &InputCase, host: &HostOracle, count: usize, locals: bool) -> Check<Value> {
    let entry = function_named(&case.source, "sequence-wide", "entry")?;
    let index = usize::try_from(host.program.function(entry).ok_or("Sequence entry")?.0)
        .map_err(|_| "Sequence function index")?;
    let code = match &host
        .program
        .functions
        .get(index)
        .ok_or("Sequence function")?
        .body
    {
        NormalizedFunctionBody::Code(code) => code,
        _ => return Err("Sequence compiled as an external".to_owned()),
    };
    equal(
        code.instructions
            .iter()
            .filter(|op| matches!(op, NormalizedInstruction::Drop))
            .count(),
        count - 1,
        "one Drop for every nonfinal Sequence item",
    )?;
    if !locals {
        equal(
            code.instructions.len(),
            count.checked_mul(2).ok_or("Sequence instruction extent")?,
            "literal Sequence retains every value, Drop and Return",
        )?;
    }
    let mut sequence_key = None;
    for (key, owner) in &case.source.owners {
        if let OwnerRecord::Expression(expression) = owner
            && let ExpressionOperation::Sequence { items } = &expression.operation
        {
            equal(items.len(), count, "public Sequence child count")?;
            ensure(
                sequence_key.replace(*key).is_none(),
                "multiple Sequence fixture parents",
            )?;
        }
    }
    let encoded = EncodedOwnerKey::new(sequence_key.ok_or("Sequence owner absent")?).bytes();
    let exported = ExportedSource::from_container(&case.container)
        .map_err(|error| format!("Sequence raw owner extent: {error:?}"))?;
    let raw = exported.as_raw();
    let parent = raw
        .owners
        .iter()
        .find(|row| row.key == encoded)
        .ok_or("Sequence raw owner absent")?;
    let maximum_unit = host
        .loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(_, bytes)| bytes.len())
        .max()
        .ok_or("Sequence unit absent")?;
    let reference_owners = host
        .loaded
        .manifest
        .packages
        .iter()
        .try_fold(0_u64, |total, package| {
            total.checked_add(package.reference_owners.entries())
        })
        .ok_or("Sequence reference inventory extent")?;
    if count > 1_000_000 && !locals {
        ensure(
            parent.bytes.len() > 4 * 1024 * 1024,
            "owner did not cross old 4 MiB ceiling",
        )?;
        ensure(
            maximum_unit > 8 * 1024 * 1024,
            "unit did not cross old 8 MiB ceiling",
        )?;
        ensure(
            code.instructions.len() > 1_000_000,
            "instruction count did not cross old ceiling",
        )?;
        ensure(
            reference_owners > 1_000_000,
            "reference owners did not cross old ceiling",
        )?;
    }
    Ok(
        json!({"items": count, "locals": locals, "owner_bytes": parent.bytes.len(),
        "owner_sha256": sha256(parent.bytes), "unit_bytes": maximum_unit,
        "instructions": code.instructions.len(), "reference_owners": reference_owners,
        "artifact_bytes": host.bytes.len(), "artifact_sha256": sha256(&host.bytes)}),
    )
}

fn verify_case(
    stages: &[(&str, &[u8])],
    case: &InputCase,
    count: usize,
    expected: i64,
    locals: bool,
    phases: bool,
) -> Check<Value> {
    let mut outputs = Vec::new();
    for (name, image) in stages {
        outputs.push(raw_outputs(
            &format!("{name}/sequence-{count}/locals-{locals}"),
            image,
            case,
            "sequence-wide",
            WHOLE_SOURCE_DEADLINE,
        )?);
    }
    let started = Instant::now();
    let host = host_oracle(case)?;
    let host_elapsed = started.elapsed().as_millis().to_string();
    let inventory = code_inventory(case, &host, count, locals)?;
    let entry = function_named(&case.source, "sequence-wide", "entry")?;
    let mut program = None;
    for ((name, _), output) in stages.iter().zip(&outputs) {
        let loaded = strict_artifact(name, &output.artifact, case, &host)?;
        if program.is_none() {
            program = Some(
                NormalizedProgram::prepare(loaded)
                    .map_err(|error| format!("Sequence preparation: {error:?}"))?,
            );
        }
    }
    let program = program.as_ref().ok_or("Sequence compiler stage missing")?;
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let started = Instant::now();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(entry, vec![], None, &control)
        .map_err(|error| format!("Sequence VM: {error:?}"))?;
    let vm_elapsed = started.elapsed().as_millis().to_string();
    let started = Instant::now();
    let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
        .invoke(entry, vec![], None, &control)
        .map_err(|error| format!("Sequence reference: {error:?}"))?;
    let reference_elapsed = started.elapsed().as_millis().to_string();
    equal_value(&vm.0, &NormalizedValue::I64(expected), "Sequence VM result")?;
    equal_value(
        &reference.0,
        &NormalizedValue::I64(expected),
        "Sequence reference result",
    )?;
    reclaimed(&vm.1, "Sequence VM cleanup")?;
    reference_reclaimed(&reference.1, "Sequence reference cleanup")?;
    let native = native_image(&host.program, entry)?;
    run_scalar("Sequence/C0", &native, expected, FOCUSED_DEADLINE)?;
    for ((name, image), output) in stages.iter().zip(&outputs) {
        run_scalar(
            &format!("Sequence/{name}"),
            &output.elf,
            expected,
            FOCUSED_DEADLINE,
        )?;
        if phases {
            phase::verify_compiler_with_address_space(
                &format!("Sequence/{name}/retained-output"),
                image,
                &case.request("sequence-wide", SourceOutputMode::CanonicalArtifact)?,
                &host.bytes,
                &case.request("sequence-wide", SourceOutputMode::Elf)?,
                &output.elf,
                None,
            )?;
        }
    }
    for pair in outputs.windows(2) {
        equal_bytes(&pair[0].elf, &pair[1].elf, "Sequence generations ELF")?;
        equal_bytes(
            &pair[0].artifact,
            &pair[1].artifact,
            "Sequence generations artifact",
        )?;
    }
    let report = json!({"event": "crossing_passed", "inventory": inventory,
        "expected": expected, "stages": stages.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "source_carrier_sha256": sha256(&case.carrier), "carrier_bytes": case.carrier.len(),
        "host_build_link_load_ms": host_elapsed, "vm_ms": vm_elapsed,
        "reference_ms": reference_elapsed, "vm": vm.1, "reference": reference.1,
        "native_same_process_recovery": phases});
    emit(report.clone());
    Ok(report)
}

const TRAP: &str = r#"declarations.begin
(units (module create sequence-wide
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
  (external create freeze (visibility private) (implementation core.buffer.freeze)
    (parameter create data (type ByteBuffer) (use consume)) (returns Bytes))
  (function create entry (visibility public) (effect pure) (returns I64)
    (body (let (binding buffer (type ByteBuffer) (call empty))
      (in (sequence (call divide (i64 1) (i64 0)) (call freeze (local buffer)) (i64 42))))))
  (component create app (visibility public)
    (port create run (type (function () I64)) (function entry))))
  (target create sequence-wide (component sequence-wide::app)
    (runner command) (port sequence-wide::app::run)))
declarations.end
"#;

fn verify_trap(stages: &[(&str, &[u8])]) -> Check<()> {
    let case = public_case(TRAP)?;
    let mut outputs = Vec::new();
    for (name, image) in stages {
        outputs.push(raw_outputs(
            &format!("{name}/sequence-early-trap"),
            image,
            &case,
            "sequence-wide",
            FOCUSED_DEADLINE,
        )?);
    }
    let host = host_oracle(&case)?;
    let entry = function_named(&case.source, "sequence-wide", "entry")?;
    for ((name, _), output) in stages.iter().zip(&outputs) {
        strict_artifact(name, &output.artifact, &case, &host)?;
    }
    for reference in [false, true] {
        let storage = StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        let policy = NormalizedRunPolicy::foreground();
        let result = if reference {
            let observer = Mutex::new(None);
            let result = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
                .observing_checked(&observer)
                .invoke(entry, vec![], None, &control);
            let observation = observer
                .into_inner()
                .map_err(|_| "Sequence observer lock")?
                .ok_or("Sequence reference trap observation")?;
            reference_reclaimed(&observation, "Sequence reference trap cleanup")?;
            result.map(|(value, _)| value)
        } else {
            let observer = Mutex::new(None);
            let result = NormalizedVm::for_test(&host.program, policy)
                .observing_checked(&observer)
                .invoke(entry, vec![], None, &control);
            let observation = observer
                .into_inner()
                .map_err(|_| "Sequence observer lock")?
                .ok_or("Sequence VM trap observation")?;
            reclaimed(&observation, "Sequence VM trap cleanup")?;
            result.map(|(value, _)| value)
        };
        let error = result.err().ok_or("Sequence skipped its early trap")?;
        equal(
            error.code.as_ref(),
            if reference {
                "reference_integer_division"
            } else {
                "normalized_integer_division"
            },
            "Sequence first failure",
        )?;
        equal(
            storage.created(),
            1,
            "Sequence created the owned buffer before trapping",
        )?;
        equal(
            storage.live(),
            (0, 0),
            "Sequence trap reclaimed owned storage",
        )?;
    }
    let native = native_image(&host.program, entry)?;
    for (name, image) in std::iter::once(("C0", native.as_slice())).chain(
        stages
            .iter()
            .zip(&outputs)
            .map(|((name, _), output)| (*name, output.elf.as_slice())),
    ) {
        let output = artifact_support::run_native(name, image, &[], FOCUSED_DEADLINE)?;
        equal(
            output.status.code(),
            Some(70),
            "Sequence native early trap status",
        )?;
        equal(
            output.stdout.as_slice(),
            &[][..],
            "Sequence trap has no result",
        )?;
        equal(
            output.stderr.as_slice(),
            b"normalized_integer_division\n".as_slice(),
            "Sequence native first failure",
        )?;
    }
    Ok(())
}

fn verify_rejections(stages: &[(&str, &[u8])]) -> Check<()> {
    let case = public_case(&proposal(3, 42, false)?)?;
    let (key, record, first) = case
        .source
        .owners
        .iter()
        .find_map(|(key, owner)| {
            if let OwnerRecord::Expression(record) = owner
                && let ExpressionOperation::Sequence { items } = &record.operation
            {
                Some((*key, owner, items[0]))
            } else {
                None
            }
        })
        .ok_or("Sequence negative parent absent")?;
    let original = kernel::encode_owner(record)
        .map_err(|error| format!("Sequence negative envelope: {error:?}"))?
        .1;
    let pattern = bincode::encode_to_vec(first, bincode::config::standard())
        .map_err(|error| error.to_string())?;
    equal(pattern.len(), 17, "typed child-reference width")?;
    let starts = original
        .windows(pattern.len())
        .enumerate()
        .filter_map(|(at, bytes)| (bytes == pattern).then_some(at))
        .collect::<Vec<_>>();
    ensure(
        starts.len() == 1,
        "Sequence first child reference is not unique",
    )?;
    let start = starts[0];
    let end = original
        .len()
        .checked_sub(32)
        .ok_or("Sequence checksum extent")?;
    equal(
        start + 3 * 17,
        end,
        "Sequence references fill owner payload",
    )?;
    equal(original[start - 1], 3, "Sequence canonical small count")?;
    let mut cases: Vec<(&str, Vec<u8>, i64)> = Vec::new();
    let mut empty = original[18..start].to_vec();
    *empty.last_mut().ok_or("Sequence empty count")? = 0;
    cases.push(("empty", empty, 12));
    let mut excessive = original[18..end].to_vec();
    excessive[start - 19] = 4;
    cases.push(("count-beyond-extent", excessive, 12));
    cases.push(("truncated-reference", original[18..end - 1].to_vec(), 12));
    let mut trailing = original[18..end].to_vec();
    trailing.push(0);
    cases.push(("trailing-reference-byte", trailing, 12));
    let mut namespace = original[18..end].to_vec();
    namespace[start - 18] = 1;
    cases.push(("wrong-reference-namespace", namespace, 12));
    let mut zero = original[18..end].to_vec();
    zero[start - 17..start - 1].fill(0);
    cases.push(("zero-reference-identity", zero, 12));
    let mut duplicate = original[18..end].to_vec();
    duplicate[start - 18 + 17..start - 18 + 34].copy_from_slice(&pattern);
    cases.push(("duplicate-child", duplicate, 15));
    let absent = crate::platform::semantic_id::ExpressionId::from_bytes([0xa5; 16])
        .ok_or("nonzero Sequence fixture identity")?;
    ensure(
        !case
            .source
            .owners
            .contains_key(&OwnerKey::Expression(absent)),
        "negative absent child is present",
    )?;
    let absent = bincode::encode_to_vec(absent, bincode::config::standard())
        .map_err(|error| error.to_string())?;
    let mut missing = original[18..end].to_vec();
    missing[start - 18..start - 18 + 17].copy_from_slice(&absent);
    cases.push(("absent-child", missing, 14));
    let target = target_named(&case.source, "sequence-wide")?;
    for (name, payload, diagnostic) in cases {
        let bytes = object_envelope(
            &kernel::contract::OWNER_MAGIC,
            kernel::contract::OWNER_ENVELOPE_DOMAIN,
            &payload,
        );
        let decoded = kernel::decode_owner(
            &bytes,
            key,
            record.kind(),
            kernel::OwnerObjectDigest::of(&bytes),
        );
        if diagnostic == 12 {
            ensure(decoded.is_err(), "host decoder accepted malformed Sequence")?;
        } else {
            let mut source = case.source.clone();
            source.owners.insert(
                key,
                decoded.map_err(|error| format!("valid local negative owner: {error:?}"))?,
            );
            ensure(
                kernel::validate_full(&source).is_err(),
                "host admitted invalid containment/link",
            )?;
        }
        // Test-only synthetic history binds the new object, checksum and invalid
        // carrier. The production exporter is never repaired or filtered.
        let carrier = source_fixture::raw_object_packet(&case.source, Some((key, &bytes)), None);
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request = raw_source::request(&carrier, target, mode)
                .map_err(|error| format!("Sequence negative request: {error:?}"))?;
            for (stage, image) in stages {
                run_filter(
                    &format!("{stage}/sequence-negative/{name}/{mode:?}"),
                    image,
                    &request,
                    diagnostic,
                    FOCUSED_DEADLINE,
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn verify_stages(
    left_name: &str,
    left: &[u8],
    right_name: &str,
    right: &[u8],
) -> Check<()> {
    let stages = [(left_name, left), (right_name, right)];
    for locals in [false, true] {
        let case = public_case(&proposal(100_001, 42, locals)?)?;
        verify_case(&stages, &case, 100_001, 42, locals, false)?;
    }
    verify_trap(&stages)?;
    verify_rejections(&stages)
}

#[test]
fn selfhost_source_sequence_public_order_local_context_and_cleanup() -> Check<()> {
    let stages = [("C1", bootstrap_compiler()?)];
    for (count, expected, locals) in [(1, 42, false), (33, 42, false), (33, 43, true)] {
        let case = public_case(&proposal(count, expected, locals)?)?;
        verify_case(&stages, &case, count, expected, locals, false)?;
    }
    verify_trap(&stages)?;
    verify_rejections(&stages)
}

// Explicit capacity preparation only. Narrow controls keep their existing C1
// fixture and never initialize a generated compiler chain. Every call here runs
// both native generations on the same complete public S; no generated image is
// loaded from disk or substituted from a OnceLock.
fn maintained_capacity_compiler() -> Check<Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let copy = RepositoryCopy::create(&root)?;
    let first_read = (|| {
        InputCase::from_repository(
            GraphRepository::open(&root)
                .map_err(|error| format!("Sequence compiler source: {error:?}"))?,
        )
    })();
    let original = match (first_read, copy.verify_original(&root)) {
        (Ok(original), Ok(())) => original,
        (read, integrity) => {
            return Err(format!(
                "Sequence compiler initial read={} integrity={integrity:?}",
                match read {
                    Ok(_) => "passed".to_owned(),
                    Err(error) => error,
                }
            ));
        }
    };
    let prepared = (|| {
        let case = InputCase::from_repository(
            GraphRepository::open(&copy.path)
                .map_err(|error| format!("Sequence compiler isolated source: {error:?}"))?,
        )?;
        same_source(
            &case,
            &original,
            "Sequence compiler original/copy before C0",
        )?;
        let request = case.request("compile-source-elf", SourceOutputMode::Elf)?;
        let host = host_oracle(&case)?;
        let c1 = native_image(
            &host.program,
            function_named(&case.source, "source-elf", "compile")?,
        )?;
        let c2 = run_filter(
            "Sequence-preparation/C2=C1(S,E)",
            &c1,
            &request,
            0,
            WHOLE_SOURCE_DEADLINE,
        )?;
        let c3 = run_filter(
            "Sequence-preparation/C3=C2(S,E)",
            &c2,
            &request,
            0,
            WHOLE_SOURCE_DEADLINE,
        )?;
        equal_bytes(&c2, &c3, "Sequence preparation actual C2=C3")?;
        let binding = json!({
            "event": "capacity_compiler_prepared",
            "stage": "C2",
            "scope": "capacity image preparation; full fixed-point acceptance is separate",
            "source_carrier_sha256": sha256(&case.carrier),
            "source_carrier_bytes": case.carrier.len(),
            "core_sha256": sha256(&case.canonical_core),
            "package_revision": case.revision_digest.to_string(),
            "owners": case.source.owners.len(),
            "supplied_types": case.source.types.len(),
            "retirements": case.source.retirements.len(),
            "elf_request_sha256": sha256(&request),
            "elf_request_bytes": request.len(),
            "C1_sha256": sha256(&c1),
            "C2_sha256": sha256(&c2),
            "C3_sha256": sha256(&c3),
            "C2_bytes": c2.len(),
            "C3_bytes": c3.len(),
            "native_equations": "C2=C1(S,E); C3=C2(S,E); bytes(C2)=bytes(C3)",
            "source_profile": 3,
            "request_profile": 2,
            "environment": "empty",
            "caller_deadline_seconds": WHOLE_SOURCE_DEADLINE.as_secs(),
            "original_repository_unchanged": true
        });
        Ok((c2, binding))
    })();
    let unchanged_source = (|| {
        let after = InputCase::from_repository(
            GraphRepository::open(&root)
                .map_err(|error| format!("Sequence compiler final source: {error:?}"))?,
        )?;
        same_source(
            &after,
            &original,
            "Sequence compiler original after preparation",
        )
    })();
    // Last, so the final public read cannot repair files after this check.
    let unchanged_files = copy.verify_original(&root);
    match (prepared, unchanged_files, unchanged_source) {
        (Ok((image, binding)), Ok(()), Ok(())) => {
            emit(binding);
            Ok(image)
        }
        (prepared, files, source) => Err(format!(
            "Sequence compiler preparation={} original_files={files:?} original_source={source:?}",
            match prepared {
                Ok(_) => "passed".to_owned(),
                Err(error) => error,
            }
        )),
    }
}

#[test]
#[ignore = "explicit public capacity crossing; defaults to 1,000,001 items and both output modes"]
fn selfhost_source_sequence_crosses_owner_unit_and_reference_ceilings() -> Check<()> {
    let count = match std::env::var("LKJ_SEQUENCE_ITEMS") {
        Ok(value) => value
            .parse::<usize>()
            .map_err(|error| format!("Sequence items: {error}"))?,
        Err(std::env::VarError::NotPresent) => 1_000_001,
        Err(error) => return Err(format!("Sequence items: {error}")),
    };
    let locals = std::env::var("LKJ_SEQUENCE_LOCALS").is_ok_and(|value| value == "1");
    let phases = std::env::var("LKJ_SEQUENCE_PHASES").is_ok_and(|value| value == "1");
    let generation = match std::env::var("LKJ_SEQUENCE_COMPILER") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => "C1".to_owned(),
        Err(error) => return Err(format!("Sequence compiler selection: {error}")),
    };
    // Prepare before the large public fixture so the generation source/oracle
    // inventories do not overlap its peak memory. C1 remains the default.
    let generated = match generation.as_str() {
        "C1" => None,
        "C2" => Some(maintained_capacity_compiler()?),
        _ => return Err("Sequence compiler must be C1 or C2".to_owned()),
    };
    let text = proposal(count, 42, locals)?;
    let case = match std::env::var_os("LKJ_SEQUENCE_REPOSITORY") {
        Some(path) => public_disk_case(&text, Path::new(&path))?,
        None => public_case(&text)?,
    };
    let stage = match generated.as_ref() {
        Some(image) => ("C2", image.as_slice()),
        None => ("C1", bootstrap_compiler()?),
    };
    verify_case(&[stage], &case, count, 42, locals, phases)?;
    Ok(())
}

fn file_digest(path: &Path) -> Check<String> {
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("Sequence binding: {error}"))?;
    let mut hash = Sha256::new();
    let mut chunk = [0_u8; 65_536];
    loop {
        let count = file
            .read(&mut chunk)
            .map_err(|error| format!("Sequence binding: {error}"))?;
        if count == 0 {
            return Ok(crate::platform::semantic_id::encode_hex(&hash.finalize()));
        }
        hash.update(&chunk[..count]);
    }
}

#[test]
#[ignore = "explicit same-binary before/after measurement of complete source compilation"]
fn selfhost_source_sequence_causal_measurement() -> Check<()> {
    let stage = std::env::var("LKJ_SEQUENCE_STAGE").map_err(|_| "Sequence stage required")?;
    ensure(
        matches!(stage.as_str(), "baseline" | "cursor"),
        "Sequence stage must be baseline or cursor",
    )?;
    let expected_binary = std::env::var("LKJ_SEQUENCE_BINARY_SHA256")
        .map_err(|_| "pinned Sequence test binary required")?;
    let expected_head = std::env::var("LKJ_SEQUENCE_HEAD_SHA256")
        .map_err(|_| "pinned Sequence public HEAD required")?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    equal(
        file_digest(&executable)?,
        expected_binary.clone(),
        "same Cargo-selected test image",
    )?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    equal(
        file_digest(&root.join("HEAD"))?,
        expected_head.clone(),
        "measurement public HEAD",
    )?;
    let compiler_case = InputCase::from_repository(
        GraphRepository::open(&root)
            .map_err(|error| format!("Sequence compiler public export: {error:?}"))?,
    )?;
    let image = bootstrap_compiler()?;
    emit(
        json!({"event": "measurement_binding", "stage": stage, "test_binary_sha256": expected_binary,
        "public_head_sha256": expected_head, "compiler_carrier_sha256": sha256(&compiler_case.carrier),
        "compiler_carrier_bytes": compiler_case.carrier.len(), "compiler_image_sha256": sha256(image),
        "compiler_supplied_types": compiler_case.source.types.len(),
        "compiler_owners": compiler_case.source.owners.len(), "compiler_retirements": compiler_case.source.retirements.len()}),
    );
    let fixture_directory = std::env::var_os("LKJ_SEQUENCE_FIXTURE_DIR")
        .ok_or("persistent public measurement fixtures required")?;
    let fixture_directory = Path::new(&fixture_directory);
    for count in [512, 2048, 4096] {
        for locals in [false, true] {
            let fixture_name = format!("items-{count}-locals-{locals}");
            let fixture_path = fixture_directory.join(&fixture_name);
            let carrier_path = fixture_directory.join(format!("{fixture_name}.lkjsg"));
            let case = if stage == "baseline" {
                let case = public_disk_case(&proposal(count, 42, locals)?, &fixture_path)?;
                crate::platform::owned_output::publish_create_new(
                    &carrier_path,
                    &case.carrier,
                    None,
                    "pinned complete measurement carrier",
                )
                .map_err(|error| format!("Sequence carrier capture: {error:?}"))?;
                case
            } else {
                let case = InputCase::from_repository(
                    GraphRepository::open(&fixture_path)
                        .map_err(|error| format!("Sequence pinned public fixture: {error:?}"))?,
                )?;
                equal(
                    file_digest(&carrier_path)?,
                    sha256(&case.carrier),
                    "before/after complete input carrier including unused types",
                )?;
                case
            };
            let started = Instant::now();
            let report = verify_case(&[("C1", image)], &case, count, 42, locals, false)?;
            emit(
                json!({"event": "measurement_case", "stage": stage, "items": count, "locals": locals,
                "total_elapsed_ms": started.elapsed().as_millis().to_string(), "result": report}),
            );
        }
    }
    equal(
        file_digest(&root.join("HEAD"))?,
        expected_head,
        "measurement graph unchanged",
    )?;
    Ok(())
}
