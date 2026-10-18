//! Artifact emission primitives use semantic inputs and independent strict readers.
use super::*;
use crate::platform::compiler::selfhost_artifact_input;
use crate::platform::persistent_map::{MapWork, MemoryPageStore, PersistentMap};
use crate::platform::storage::pack::{PackBuilder, PackMetadata};

fn probes() -> &'static (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    static PROBES: OnceLock<(crate::platform::kernel::KernelSnapshot, NormalizedProgram)> =
        OnceLock::new();
    PROBES.get_or_init(|| {
        let (source, _) = compiler();
        let (_, changes) = proposals::steps(include_str!(
            "../../../../tests/fixtures/selfhost-artifact-probes.lkjc"
        ))
        .unwrap();
        let source = apply_proposals(source, changes);
        let program = prepare_snapshot(&source);
        (source, program)
    })
}

fn result(name: &str, input: &[u8]) -> (i64, Vec<u8>) {
    let (source, program) = probes();
    let declaration = declaration_named(source, name);
    let arguments = vec![NormalizedValue::bytes(input)];
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap();
    if name == "compile-elf" {
        assert!(
            vm.1.maximum_call_depth <= 128,
            "artifact compiler scan grew to {} call frames",
            vm.1.maximum_call_depth
        );
    }
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(declaration, arguments, None, &control)
        .unwrap();
    if name == "compile-elf" {
        assert!(
            reference.1.maximum_call_depth <= 128,
            "reference artifact compiler scan grew to {} call frames",
            reference.1.maximum_call_depth
        );
    }
    assert!(vm.0 == reference.0, "compiler engines disagree for {name}");
    assert_eq!(vm.1.live_call_frames_after, 0);
    assert_eq!(reference.1.live_call_frames_after, 0);
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = vm.0 else {
        panic!("probe result")
    };
    let NormalizedValue::I64(diagnostic) = &fields
        .iter()
        .find(|(name, _)| name.as_str() == "diagnostic")
        .unwrap()
        .1
    else {
        panic!("probe diagnostic")
    };
    let NormalizedValue::Bytes(bytes) = &fields
        .iter()
        .find(|(name, _)| name.as_str() == "image")
        .unwrap()
        .1
    else {
        panic!("probe bytes")
    };
    let native = crate::platform::compiler::elf_x86_64::compile_function(
        program,
        program.function(declaration).unwrap().0 as usize,
    )
    .unwrap()
    .0;
    static PRODUCERS: OnceLock<std::sync::Mutex<BTreeSet<String>>> = OnceLock::new();
    let native_hash = crate::platform::semantic_id::encode_hex(&Sha256::digest(&native));
    if PRODUCERS
        .get_or_init(|| std::sync::Mutex::new(BTreeSet::new()))
        .lock()
        .unwrap()
        .insert(native_hash.clone())
    {
        eprintln!(
            "selfhost artifact native producer entry={name} bytes={} sha256={native_hash}",
            native.len()
        );
    }
    if name == "compile-elf" {
        eprintln!(
            "selfhost artifact encoder input_bytes={} C1_bytes={} C1_sha256={} vm_steps={} vm_cumulative_allocation={} ref_expressions={} ref_cumulative_allocation={} vm_max_call_depth={} ref_max_call_depth={}",
            input.len(),
            native.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&native)),
            vm.1.instructions,
            vm.1.allocated_bytes,
            reference.1.expressions,
            reference.1.allocated_bytes,
            vm.1.maximum_call_depth,
            reference.1.maximum_call_depth
        );
    }
    let output = native_filter_tests::execute_filter(&native, input, &[], 127);
    assert_eq!(
        output.status.code(),
        Some(if *diagnostic == 0 { 0 } else { 65 }),
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if *diagnostic == 0 {
        assert!(output.stderr.is_empty());
    } else {
        assert_eq!(
            output.stderr,
            format!("compiler_rejected:{diagnostic}\n").as_bytes()
        );
        assert!(bytes.is_empty());
    }
    assert!(
        output.stdout == bytes.as_ref(),
        "native compiler disagrees for {name}"
    );
    (*diagnostic, bytes.to_vec())
}

pub(super) fn image(name: &str, input: &[u8]) -> Vec<u8> {
    let (code, bytes) = result(name, input);
    assert_eq!(code, 0, "{name}");
    bytes
}

pub(super) fn complete_compiler() -> &'static (Vec<u8>, Vec<u8>) {
    static COMPLETE: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    COMPLETE.get_or_init(|| {
        let (_, program) = compiler();
        let input = selfhost_artifact_input::bundle(program.artifact()).unwrap();
        let emitted = image("compile-elf", &input);
        (input, emitted)
    })
}

#[test]
fn selfhost_artifact_input_rejects_malformed_complete_bundles() {
    let source = byte_buffer_tests::author_only("declarations.begin\n(units (module create independent (function create entry (visibility public) (effect pure) (parameter create x (type I64)) (returns I64) (body (i64 42)))))\ndeclarations.end\n").unwrap();
    let program = prepare_snapshot(&source);
    let valid = selfhost_artifact_input::bundle(program.artifact()).unwrap();
    let at = |offset| u32::from_le_bytes(valid[offset..offset + 4].try_into().unwrap()) as usize;
    let rows = at(80);
    let meta = at(88);
    let unit = at(160);
    let first_object = at(rows + 8);
    assert_eq!(at(meta + 304), 1, "one parameter runtime-owner row");
    crate::platform::compiler::load_artifact(&image("compile-elf", &valid)).unwrap();
    for length in [0, 115, 159, valid.len() - 1] {
        assert_eq!(result("compile-elf", &valid[..length]), (3, vec![]));
    }
    for (offset, value, diagnostic) in [
        (0, 0_u32, 1),
        (8, 3, 2),
        (12, 1, 3),
        (20, 1, 3),
        (24, 0, 3),
        (24, 701, 3),
        (28, 1, 3),
        (32, 0, 6),
        (32, u32::MAX, 6),
        (36, 1, 6),
        (72, 0, 10),
        (76, 0, 10),
        (80, u32::MAX, 10),
        (84, u32::MAX, 10),
        (88, u32::MAX, 10),
        (92, u32::MAX, 10),
        (96, 1, 10),
        (112, 2, 2),
        (116, 1, 10),
        (160, 0, 10),
        (164, 335, 10),
        (164, u32::MAX, 10),
        (unit + 12, 1, 10),
        (rows, 24, 11),
        (rows + 4, 1, 11),
        (rows + 8, 0, 11),
        (rows + 12, u32::MAX, 11),
        (meta + 196, 1, 10),
        (meta + 268, 1, 10),
        (meta + 304, u32::MAX, 10),
        (meta + 308, 1, 10),
        (meta + 312, 0, 10),
        (meta + 316, 99, 10),
    ] {
        let mut changed = valid.clone();
        changed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        input::rechecksum(&mut changed);
        assert_eq!(
            result("compile-elf", &changed),
            (diagnostic, vec![]),
            "field {offset}"
        );
    }
    for (offset, diagnostic) in [
        (40, 4),
        (unit + 16, 10),
        (rows + 16, 11),
        (first_object, 11),
        (meta + 32, 10),
        (meta + 128, 10),
        (meta + 160, 10),
        (meta + 232, 10),
        (meta + 312 + 24, 10),
    ] {
        let mut changed = valid.clone();
        changed[offset] ^= 1;
        if offset != 40 {
            input::rechecksum(&mut changed);
        }
        assert_eq!(
            result("compile-elf", &changed),
            (diagnostic, vec![]),
            "byte {offset}"
        );
    }
    let mut zero_owner = valid.clone();
    zero_owner[unit + 32..unit + 48].fill(0);
    input::rechecksum(&mut zero_owner);
    assert_eq!(result("compile-elf", &zero_owner), (10, vec![]));
}

#[test]
fn selfhost_artifact_unit_layout_rejects_malformed_and_unsupported_ir() {
    let source = byte_buffer_tests::author_only("declarations.begin\n(units (module create independent (function create entry (visibility public) (effect pure) (returns I64) (body (i64 42)))))\ndeclarations.end\n").unwrap();
    let program = prepare_snapshot(&source);
    let (key, bytes) = program
        .artifact()
        .objects
        .iter()
        .find(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .unwrap();
    let unit = crate::platform::compiler::CompilationUnit::decode(bytes, *key).unwrap();
    let valid = selfhost_artifact_input::unit(&unit).unwrap();
    assert_eq!(image("probe-unit", &valid), *bytes);
    let at = |offset| u32::from_le_bytes(valid[offset..offset + 4].try_into().unwrap()) as usize;
    let signature = at(244);
    let code = at(252);
    let nodes = at(code + 8);
    for length in [0, 255, 335, valid.len() - 1] {
        assert_eq!(
            result("probe-unit", &valid[..length]),
            (10, vec![]),
            "length {length}"
        );
    }
    for (offset, value) in [
        (0, 0_u32),
        (8, 0),
        (12, 1),
        (240, 0),
        (248, 99),
        (252, 0),
        (308, u32::MAX),
        (signature + 16, u32::MAX),
        (code, 1),
        (nodes, 99),
        (nodes + 4, 1),
        (nodes + 12, 1),
        (nodes + 24, 1),
        (nodes + 32 + 16, 1),
    ] {
        let mut changed = valid.clone();
        changed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert_eq!(
            result("probe-unit", &changed),
            (10, vec![]),
            "field {offset}"
        );
    }
    for (tag, operand) in [
        (3_u32, 2_u32),
        (7, 0),
        (11, 0),
        (12, u32::MAX),
        (15, 0),
        (17, 0),
        (18, u32::MAX),
        (27, 0),
    ] {
        let mut changed = valid.clone();
        changed[nodes..nodes + 32].fill(0);
        changed[nodes..nodes + 4].copy_from_slice(&tag.to_le_bytes());
        changed[nodes + 4..nodes + 8].copy_from_slice(&operand.to_le_bytes());
        assert_eq!(result("probe-unit", &changed), (10, vec![]), "opcode {tag}");
    }
}

#[test]
fn selfhost_artifact_signed_wire_preserves_independent_boundaries() {
    for (value, expected) in [
        (0_i64, vec![0]),
        (1, vec![2]),
        (-1, vec![1]),
        (125, vec![250]),
        (-125, vec![249]),
        (126, vec![251, 252, 0]),
        (-126, vec![251, 251, 0]),
        (32767, vec![251, 254, 255]),
        (-32768, vec![251, 255, 255]),
        (32768, vec![252, 0, 0, 1, 0]),
        (-32769, vec![252, 1, 0, 1, 0]),
        (2147483647, vec![252, 254, 255, 255, 255]),
        (-2147483648, vec![252, 255, 255, 255, 255]),
        (2147483648, vec![253, 0, 0, 0, 0, 1, 0, 0, 0]),
        (i64::MIN, vec![253, 255, 255, 255, 255, 255, 255, 255, 255]),
        (i64::MAX, vec![253, 254, 255, 255, 255, 255, 255, 255, 255]),
    ] {
        assert_eq!(
            image("probe-signed", &value.to_le_bytes()),
            expected,
            "{value}"
        );
    }
}

#[test]
fn selfhost_artifact_units_match_canonical_complete_compiler_ir() {
    let (_, program) = compiler();
    let mut count = 0;
    let mut kinds = BTreeSet::new();
    for (key, bytes) in &program.artifact().objects {
        if key.domain != ObjectDomain::CompilerUnit {
            continue;
        }
        let unit = crate::platform::compiler::CompilationUnit::decode(bytes, *key).unwrap();
        let input = selfhost_artifact_input::unit(&unit).unwrap();
        let emitted = image("probe-unit", &input);
        assert!(
            emitted == *bytes,
            "{:?}: first differing byte {:?}; emitted {} bytes, expected {}",
            unit.source.owner,
            emitted.iter().zip(bytes).position(|(a, b)| a != b),
            emitted.len(),
            bytes.len()
        );
        let actual = crate::platform::compiler::CompilationUnit::decode(&emitted, *key).unwrap();
        assert_eq!(actual, unit);
        kinds.insert(unit.source.kind.tag());
        count += 1;
    }
    assert!(count >= program.functions.len());
    assert_eq!(kinds, BTreeSet::from([5, 6, 9, 20]));
    eprintln!(
        "selfhost canonical unit emission units={count} kinds={kinds:?}; VM/reference/native bytes match current strict unit encoding"
    );
}

#[test]
fn selfhost_artifact_map_pages_match_independent_logical_roots() {
    for count in [0_usize, 1, 2, 65, 257, 700] {
        let entries = (0..count)
            .map(|i| {
                let mut key = vec![7; 17];
                key[15..17].copy_from_slice(&(i as u16).to_be_bytes());
                (key, vec![(i % 251) as u8; 65])
            })
            .collect::<Vec<_>>();
        let data = entries
            .iter()
            .flat_map(|(key, value)| key.iter().chain(value))
            .copied()
            .collect::<Vec<_>>();
        let emitted = image("probe-map", &data);
        let mut pages = MemoryPageStore::default();
        let map = PersistentMap::from_sorted_with_leaf_target(
            &mut pages,
            entries.clone(),
            65536,
            &mut MapWork::default(),
        )
        .unwrap();
        map.verify(&pages, &mut MapWork::default()).unwrap();
        assert_eq!(pages.object_count(), 1);
        assert_eq!(pages.objects().next().unwrap().1, emitted, "{count}");
        let mut canonical_pages = MemoryPageStore::default();
        let canonical =
            PersistentMap::from_sorted(&mut canonical_pages, entries, &mut MapWork::default())
                .unwrap();
        assert_eq!(map.root().content_root(), canonical.root().content_root());
        if count == 700 {
            assert_ne!(map.root().page(), canonical.root().page());
        }
    }
}

#[test]
fn selfhost_artifact_raw_packs_match_current_strict_reader() {
    for objects in [
        vec![vec![]],
        vec![
            vec![0, 255, 0, 128],
            (0..2049).map(|i| (i % 251) as u8).collect(),
        ],
    ] {
        let objects = objects
            .into_iter()
            .map(|bytes| (ObjectKey::for_bytes(ObjectDomain::Blob, &bytes), bytes))
            .collect::<BTreeMap<_, _>>();
        let rows_len = objects.len() * 48;
        let mut data = vec![0; 4 + rows_len];
        data[..4].copy_from_slice(&(rows_len as u32).to_le_bytes());
        let mut builder = PackBuilder::default();
        for (i, (key, bytes)) in objects.iter().enumerate() {
            let at = 4 + i * 48;
            let offset = data.len();
            data[at..at + 4].copy_from_slice(&u32::from(key.domain.tag()).to_le_bytes());
            data[at + 8..at + 12].copy_from_slice(&(offset as u32).to_le_bytes());
            data[at + 12..at + 16].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
            data[at + 16..at + 48].copy_from_slice(&key.digest.bytes());
            data.extend_from_slice(bytes);
            builder.insert(*key, bytes).unwrap();
        }
        let expected = builder.seal().unwrap();
        let emitted = image("probe-pack", &data);
        assert_eq!(emitted, expected.bytes);
        let read = PackMetadata::decode(&emitted, true).unwrap();
        read.verify_all(&emitted).unwrap();
        assert_eq!(read.entries.len(), objects.len());
    }
}

#[test]
fn selfhost_artifact_container_executes_independent_and_mutated_programs() {
    for expected in [42_i64, -91] {
        let proposal =
            include_str!("../../../../tests/fixtures/selfhost-artifact-empty-runtime.lkjc")
                .replace("(i64 42)", &format!("(i64 {expected})"));
        let source = byte_buffer_tests::author_only(&proposal).unwrap();
        let original = prepare_snapshot(&source);
        assert!(
            original.artifact().manifest.packages[0]
                .runtime_owners
                .is_empty()
        );
        let input = selfhost_artifact_input::bundle(original.artifact()).unwrap();
        let emitted = image("compile-elf", &input);
        let loaded = crate::platform::compiler::load_artifact(&emitted).unwrap();
        let program = NormalizedProgram::prepare(loaded).unwrap();
        let entry = declaration_named(&source, "entry");
        let control = ExecutionControl::uncancelled();
        let policy = NormalizedRunPolicy::foreground();
        assert_eq!(
            NormalizedVm::for_test(&program, policy)
                .invoke(entry, vec![], None, &control)
                .unwrap()
                .0,
            NormalizedValue::I64(expected)
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(&source, &program, policy)
                .invoke(entry, vec![], None, &control)
                .unwrap()
                .0,
            NormalizedValue::I64(expected)
        );
        eprintln!(
            "selfhost artifact independent result={expected} input_bytes={} artifact_bytes={} artifact_sha256={}",
            input.len(),
            emitted.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&emitted))
        );
    }
}

#[test]
fn selfhost_artifact_object_scans_keep_bounded_call_frames() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-artifact-object-scan.lkjc"
    ))
    .unwrap();
    let original = prepare_snapshot(&source);
    let input = selfhost_artifact_input::bundle(original.artifact()).unwrap();
    let objects = u32::from_le_bytes(input[84..88].try_into().unwrap());
    assert!(objects > 300);
    let emitted = image("compile-elf", &input);
    let program =
        NormalizedProgram::prepare(crate::platform::compiler::load_artifact(&emitted).unwrap())
            .unwrap();
    let entry = declaration_named(&source, "entry");
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    assert_eq!(
        NormalizedVm::for_test(&program, policy)
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(42)
    );
    assert_eq!(
        NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(42)
    );
}

#[test]
fn selfhost_artifact_container_carries_complete_compiler_and_executes_it() {
    let (source, original) = compiler();
    let (input, emitted) = complete_compiler();
    let loaded = crate::platform::compiler::load_artifact(emitted).unwrap();
    let program = NormalizedProgram::prepare(loaded).unwrap();
    assert_eq!(program.functions.len(), original.functions.len());
    let arithmetic = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    let entry = declaration_named(source, "compile-elf");
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let expected = NormalizedVm::for_test(original, policy)
        .invoke(
            entry,
            vec![NormalizedValue::bytes(arithmetic.as_slice())],
            None,
            &control,
        )
        .unwrap()
        .0;
    assert_eq!(
        NormalizedVm::for_test(&program, policy)
            .invoke(
                entry,
                vec![NormalizedValue::bytes(arithmetic.as_slice())],
                None,
                &control
            )
            .unwrap()
            .0,
        expected
    );
    assert_eq!(
        NormalizedReferenceInterpreter::new(source, &program, policy)
            .invoke(
                entry,
                vec![NormalizedValue::bytes(arithmetic.as_slice())],
                None,
                &control
            )
            .unwrap()
            .0,
        expected
    );
    eprintln!(
        "selfhost artifact compiler functions={} input_bytes={} artifact_bytes={} artifact_sha256={}; strict loader and both artifact evaluators passed",
        program.functions.len(),
        input.len(),
        emitted.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(emitted))
    );
}

#[test]
fn selfhost_artifact_executes_control_flow_and_exact_arithmetic_traps() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-closure.lkjc"
    ))
    .unwrap();
    let original = prepare_snapshot(&source);
    let payload = selfhost_artifact_input::bundle(original.artifact()).unwrap();
    let emitted = image("compile-elf", &payload);
    let loaded = crate::platform::compiler::load_artifact(&emitted).unwrap();
    let program = NormalizedProgram::prepare(loaded).unwrap();
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    for (name, expected) in [
        ("branch-true", Some(42)),
        ("branch-false", Some(17)),
        ("tail-loop", Some(5_000_050_000)),
        ("recursive", Some(3_628_800)),
        ("mutually-recursive", Some(42)),
        ("locals", Some(42)),
        ("bools", Some(42)),
        ("unit-local", Some(42)),
        ("untaken-trap", Some(42)),
        ("called-overflow", None),
    ] {
        let declaration = declaration_named(&source, name);
        let vm =
            NormalizedVm::for_test(&program, policy).invoke(declaration, vec![], None, &control);
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy).invoke(
            declaration,
            vec![],
            None,
            &control,
        );
        if let Some(expected) = expected {
            assert_eq!(vm.unwrap().0, NormalizedValue::I64(expected), "{name}");
            assert_eq!(
                reference.unwrap().0,
                NormalizedValue::I64(expected),
                "{name}"
            );
        } else {
            assert_eq!(vm.unwrap_err().code, "normalized_integer_overflow");
            assert_eq!(reference.unwrap_err().code, "reference_integer_overflow");
        }
    }
    for at in [
        0,
        8,
        12,
        28,
        60,
        emitted.len() / 2,
        emitted.len() - 9,
        emitted.len() - 1,
    ] {
        let mut damaged = emitted.clone();
        damaged[at] ^= 1;
        assert!(
            crate::platform::compiler::load_artifact(&damaged).is_err(),
            "byte {at}"
        );
    }
    for length in [0, 59, emitted.len() - 1] {
        assert!(crate::platform::compiler::load_artifact(&emitted[..length]).is_err());
    }
}
