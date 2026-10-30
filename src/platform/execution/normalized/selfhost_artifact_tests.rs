//! Artifact emission primitives use semantic inputs and independent strict readers.
use super::*;
use crate::platform::compiler::selfhost_artifact_input;
use crate::platform::persistent_map::{
    MapRoot, MapWork, MemoryPageStore, PageDigest, PageStore, PersistentMap,
};
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
        (8, 4, 2),
        (12, 1, 3),
        (20, 1, 3),
        (24, 0, 3),
        (24, 701, 10),
        (28, 1, 3),
        (32, 1, 6),
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
    // Equal maximum counts pass header agreement and must fail layout before
    // the allowance estimate can multiply the unit count.
    let mut huge_count = valid.clone();
    for offset in [24, 76] {
        huge_count[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    }
    input::rechecksum(&mut huge_count);
    assert_eq!(result("compile-elf", &huge_count), (10, vec![]));
    let mut zero_owner = valid.clone();
    zero_owner[unit + 32..unit + 48].fill(0);
    input::rechecksum(&mut zero_owner);
    assert_eq!(result("compile-elf", &zero_owner), (10, vec![]));
}

#[test]
fn selfhost_artifact_input_preserves_unmetered_and_exact_u64_allowances() {
    use std::num::NonZeroU64;

    let source = byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create allowance (function create entry (visibility public) (effect pure) (returns I64) (body (i64 42)))))\ndeclarations.end\n",
    )
    .unwrap();
    let program = prepare_snapshot(&source);
    let valid = selfhost_artifact_input::bundle(program.artifact()).unwrap();
    assert_eq!(u32::from_le_bytes(valid[8..12].try_into().unwrap()), 5);
    assert_eq!(&valid[32..40], &[0; 8]);
    let expected = image("compile-elf", &valid);
    let units = u64::from(u32::from_le_bytes(valid[76..80].try_into().unwrap()));
    let objects = u64::from(u32::from_le_bytes(valid[84..88].try_into().unwrap()));
    // Normative admission estimate, distinct from evaluator execution counters.
    let required = 4096 + 128 * valid.len() as u64 + 64 * (units * units + objects);
    assert!(required < 1_u64 << 31);
    for selected in [
        required - 1,
        required,
        required + 1,
        (1_u64 << 31) - 1,
        1_u64 << 31,
        (1_u64 << 32) - 1,
        1_u64 << 32,
        (1_u64 << 32) + 1,
        (1_u64 << 63) - 1,
        1_u64 << 63,
        (1_u64 << 63) + 1,
        u64::MAX,
    ] {
        let mut input = valid.clone();
        input[32..40].copy_from_slice(&selected.to_le_bytes());
        input::rechecksum(&mut input);
        let projected = selfhost_artifact_input::bundle_with_allowance(
            program.artifact(),
            NonZeroU64::new(selected),
        );
        if selected < required {
            assert!(projected.is_err());
            assert_eq!(result("compile-elf", &input), (6, vec![]));
        } else {
            assert_eq!(projected.unwrap(), input);
            assert_eq!(result("compile-elf", &input), (0, expected.clone()));
        }
    }
    for allowance in [0, required] {
        let mut stale = valid.clone();
        stale[8..12].copy_from_slice(&4_u32.to_le_bytes());
        stale[32..40].copy_from_slice(&allowance.to_le_bytes());
        input::rechecksum(&mut stale);
        assert_eq!(result("compile-elf", &stale), (2, vec![]));
    }
}

#[test]
fn selfhost_artifact_input_extents_reject_overflow_before_multiplication() {
    for (offset, count, stride, expected) in [
        (0_i64, 1_i64, 257_i64, vec![251, 2, 2]),
        (257, 0, 1, vec![251, 2, 2]),
        (1, 128, 2, vec![251, 2, 2]),
        (2, 128, 2, vec![1]),
        (258, 0, 1, vec![1]),
        (0, 0, i64::MAX, vec![0]),
        (0, 1, i64::MAX, vec![1]),
        (0, i64::MAX, i64::MAX, vec![1]),
        (i64::MAX, 0, 1, vec![1]),
        (-1, 1, 1, vec![1]),
        (0, -1, 1, vec![1]),
        (0, 1, -1, vec![1]),
        (0, 1, 0, vec![1]),
        (i64::MIN, 1, 1, vec![1]),
        (0, i64::MIN, 1, vec![1]),
        (0, 1, i64::MIN, vec![1]),
    ] {
        let mut input = vec![0; 257];
        for (at, value) in [(0, offset), (8, count), (16, stride)] {
            input[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(
            image("probe-extent", &input),
            expected,
            "offset={offset} count={count} stride={stride}"
        );
    }
    for (length, expected) in [
        (8 * 1_048_576, vec![252, 0, 0, 0, 1]),
        (8 * 1_048_576 + 1, vec![252, 2, 0, 0, 1]),
    ] {
        let mut input = vec![0; length];
        input[8..16].copy_from_slice(&(length as i64).to_le_bytes());
        input[16..24].copy_from_slice(&1_i64.to_le_bytes());
        assert_eq!(image("probe-extent", &input), expected, "length={length}");
    }
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
    type Entry = (Vec<u8>, Vec<u8>);

    fn entries_for(mut keys: Vec<[u8; 17]>) -> Vec<Entry> {
        keys.sort_unstable();
        keys.into_iter()
            .enumerate()
            .map(|(index, key)| {
                let value = (0..65)
                    .map(|offset| (index * 29 + offset * 17) as u8)
                    .collect();
                (key.to_vec(), value)
            })
            .collect()
    }
    fn prefix_15(count: usize) -> Vec<Entry> {
        entries_for(
            (0..count)
                .map(|index| {
                    let mut key = [7; 17];
                    key[15..17].copy_from_slice(&(index as u16).to_be_bytes());
                    key
                })
                .collect(),
        )
    }
    fn prefix_1(count: usize) -> Vec<Entry> {
        entries_for(
            (0..count)
                .map(|index| {
                    let mut key = [7; 17];
                    key[1] = (index % 256) as u8;
                    key[15..17].copy_from_slice(&((index / 256) as u16).to_be_bytes());
                    key
                })
                .collect(),
        )
    }
    fn decode_pool(pool: &[u8]) -> Vec<&[u8]> {
        let mut cursor = 0;
        let mut pages = Vec::new();
        while cursor < pool.len() {
            assert!(
                pool.len() - cursor >= 4,
                "truncated page length at {cursor}"
            );
            let length = u32::from_le_bytes(pool[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            assert!((1..=65_536).contains(&length), "page length {length}");
            let end = cursor.checked_add(length).unwrap();
            assert!(end <= pool.len(), "truncated page at {cursor}");
            pages.push(&pool[cursor..end]);
            cursor = end;
        }
        assert_eq!(cursor, pool.len());
        assert!(!pages.is_empty());
        pages
    }
    let three_levels = entries_for(
        (0_u8..2)
            .flat_map(|group| {
                (0_u16..897).map(move |index| {
                    let mut key = [7; 17];
                    key[1] = group;
                    key[15..17].copy_from_slice(&index.to_be_bytes());
                    key
                })
            })
            .collect(),
    );
    let cases = [
        ("empty", Vec::new(), Some(1)),
        ("singleton", prefix_15(1), Some(1)),
        ("small", prefix_15(9), Some(1)),
        // 107 + 15 + count * 73: 65,530 bytes, then 65,603 bytes.
        ("prefix15-fits", prefix_15(896), Some(1)),
        ("prefix15-splits", prefix_15(897), None),
        // 107 + 1 + count * 87: 65,532 bytes, then 65,619 bytes.
        ("prefix1-fits", prefix_1(752), Some(1)),
        ("prefix1-splits", prefix_1(753), None),
        // One root, two child branches, and eight leaves.
        ("three-levels", three_levels, Some(11)),
    ];
    for (name, entries, expected_page_count) in cases {
        assert!(entries.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let mut input = Vec::with_capacity(entries.len() * 82);
        for (key, value) in &entries {
            assert_eq!(key.len(), 17);
            assert_eq!(value.len(), 65);
            input.extend_from_slice(key);
            input.extend_from_slice(value);
        }
        let mut expected_pages = MemoryPageStore::default();
        let expected = PersistentMap::from_sorted_with_leaf_target(
            &mut expected_pages,
            entries.clone(),
            65_536,
            &mut MapWork::default(),
        )
        .unwrap();
        expected
            .verify(&expected_pages, &mut MapWork::default())
            .unwrap();
        let mut logical_pages = MemoryPageStore::default();
        let logical = PersistentMap::from_sorted(
            &mut logical_pages,
            entries.clone(),
            &mut MapWork::default(),
        )
        .unwrap();
        logical
            .verify(&logical_pages, &mut MapWork::default())
            .unwrap();
        assert_eq!(
            expected.root().content_root(),
            logical.root().content_root(),
            "{name}"
        );

        let output = image("probe-map-pages", &input);
        let emitted = decode_pool(&output)
            .into_iter()
            .map(|bytes| (PageDigest::of(bytes), bytes))
            .collect::<Vec<_>>();
        let expected_objects = expected_pages.objects().collect::<BTreeMap<_, _>>();
        assert_eq!(emitted.len(), expected_pages.object_count(), "{name}");
        match expected_page_count {
            Some(count) => assert_eq!(emitted.len(), count, "{name}"),
            None => assert!(emitted.len() > 1, "{name}: leaf must split"),
        }
        let mut reconstructed_pages = MemoryPageStore::default();
        let mut digests = BTreeSet::new();
        for (digest, bytes) in &emitted {
            assert!(digests.insert(*digest), "{name}: duplicate page");
            assert_eq!(*bytes, *expected_objects.get(digest).unwrap(), "{name}");
            reconstructed_pages.write_page(*digest, bytes).unwrap();
        }
        let root_digest = emitted.last().unwrap().0;
        assert_eq!(
            root_digest,
            expected.root().page(),
            "{name}: root must be last"
        );
        let root = MapRoot::from_parts(
            root_digest,
            expected.root().entries(),
            expected.root().content(),
        );
        let report = PersistentMap::from_root(root)
            .verify(&reconstructed_pages, &mut MapWork::default())
            .unwrap();
        assert_eq!(
            report.pages as usize,
            emitted.len(),
            "{name}: unreachable emitted page"
        );
        assert_eq!(report.entries as usize, entries.len());

        if name == "three-levels" {
            assert_eq!(image("probe-map-pages", &input), output);
            let mut incomplete_pages = MemoryPageStore::default();
            for (digest, bytes) in emitted.iter().skip(1) {
                incomplete_pages.write_page(*digest, bytes).unwrap();
            }
            assert!(
                expected
                    .verify(&incomplete_pages, &mut MapWork::default())
                    .is_err()
            );

            let original_root = emitted.last().unwrap().1;
            assert_eq!(original_root[10], 1, "branch kind");
            let prefix = u16::from_le_bytes(original_root[20..22].try_into().unwrap()) as usize;
            assert_eq!(
                original_root[71 + prefix],
                0,
                "no terminal for fixed-width keys"
            );
            for (case, code) in [
                ("child-summary", "persistent_map_child_summary"),
                ("content", "persistent_map_page_content"),
            ] {
                let mut changed = original_root.to_vec();
                if case == "child-summary" {
                    // Keep the root's local sum exact, then disagree with the
                    // referenced child's actual logical-byte summary.
                    for at in [30 + prefix, 74 + prefix + 41] {
                        let before = u64::from_le_bytes(changed[at..at + 8].try_into().unwrap());
                        changed[at..at + 8]
                            .copy_from_slice(&before.checked_add(1).unwrap().to_le_bytes());
                    }
                } else {
                    changed[39 + prefix] ^= 1;
                }
                let core = changed.len() - 32;
                let mut checksum =
                    blake3::Hasher::new_derive_key("lkjscript.persistent-map-checksum.v2");
                checksum.update(&(core as u64).to_be_bytes());
                checksum.update(&changed[..core]);
                changed[core..].copy_from_slice(checksum.finalize().as_bytes());
                let rebound = PageDigest::of(&changed);
                let mut corrupt_pages = reconstructed_pages.clone();
                corrupt_pages.write_page(rebound, &changed).unwrap();
                let rejected = PersistentMap::from_root(MapRoot::from_parts(
                    rebound,
                    root.entries(),
                    root.content(),
                ))
                .verify(&corrupt_pages, &mut MapWork::default())
                .unwrap_err();
                assert_eq!(rejected.code, code, "{case}: {rejected}");
            }
        }
        eprintln!(
            "selfhost map case={name} entries={} pages={} pool_bytes={} root={root_digest} content={}",
            entries.len(),
            emitted.len(),
            output.len(),
            root.content()
        );
    }
}

#[test]
fn selfhost_artifact_generated_pools_check_exact_extents() {
    let maximum = i64::from(u32::MAX);
    for (base, size, accepted) in [
        (0, 0, true),
        (0, maximum, true),
        (maximum, 0, true),
        (maximum - 1, 1, true),
        (maximum, 1, false),
        (-1, 0, false),
        (0, -1, false),
        (i64::MAX, 1, false),
        (1, i64::MAX, false),
    ] {
        let input = [base.to_le_bytes(), size.to_le_bytes()].concat();
        assert_eq!(
            result("probe-pool-extent", &input),
            (if accepted { 0 } else { 3 }, vec![])
        );
    }
    let pool = image("probe-map-pages", &[]);
    let framed = |base: i64, bytes: &[u8]| [base.to_le_bytes().as_slice(), bytes].concat();
    let exact_base = maximum - pool.len() as i64;
    let rows = image("probe-map-rows", &framed(exact_base, &pool));
    assert_eq!(rows.len(), 48);
    let word = |at| u32::from_le_bytes(rows[at..at + 4].try_into().unwrap());
    assert_eq!(word(0), 5);
    assert_eq!(word(4), 1);
    assert_eq!(u64::from(word(8)), (exact_base + 4) as u64);
    assert_eq!(word(12) as usize, pool.len() - 4);
    assert_eq!(rows[16..48], PageDigest::of(&pool[4..]).bytes());
    for base in [-1, exact_base + 1, i64::MAX] {
        assert_eq!(result("probe-map-rows", &framed(base, &pool)), (3, vec![]));
    }
    let mut zero_length = pool.clone();
    zero_length[..4].fill(0);
    let mut oversize = pool.clone();
    oversize[..4].copy_from_slice(&65_537_u32.to_le_bytes());
    let mut trailing = pool.clone();
    trailing.push(0);
    for malformed in [
        vec![],
        vec![1, 0, 0],
        zero_length,
        oversize,
        pool[..pool.len() - 1].to_vec(),
        trailing,
    ] {
        assert_eq!(
            result("probe-map-rows", &framed(0, &malformed)),
            (3, vec![])
        );
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

/// Publicly authored unused functions keep this above the former aggregate
/// and unit-count quotas, requiring a complete multi-page unit map.
pub(super) fn capacity_bundle() -> &'static (Vec<u8>, Vec<u8>) {
    static CAPACITY: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    CAPACITY.get_or_init(|| {
        use crate::platform::compiler::unit::CompiledText;
        use crate::platform::kernel::OwnerKey;

        let author_started = std::time::Instant::now();
        let initial = byte_buffer_tests::author_only(
            "declarations.begin\n(units (module create capacity-entry (function create entry (visibility public) (effect pure) (returns I64) (body (i64 42)))))\ndeclarations.end\n",
        )
        .unwrap();
        let memory = crate::platform::MemProjectStore::new(None).unwrap();
        let repository = GraphRepository::create_memory(&memory, &initial, None)
            .unwrap()
            .repository;
        let text = |index| {
            let prefix = format!("capacity-payload-{index:03}:");
            format!("{prefix}{}", "x".repeat(6144 - prefix.len()))
        };
        for (start, end) in [(0, 250), (250, 500), (500, 799)] {
            let mut proposal = format!(
                "request base={}\ndeclarations.begin\n(units (module create capacity-{start}\n",
                repository.view_current().unwrap().revision()
            );
            for index in start..end {
                proposal.push_str(&format!(
                    "(function create payload-{index} (visibility public) (effect pure) (returns Text) (body (text \"{}\")))\n",
                    text(index)
                ));
            }
            proposal.push_str("))\ndeclarations.end\n");
            assert!(proposal.len() < 4 * 1_048_576);
            let decoded = crate::platform::control::decode_compact_change(
                "artifact capacity",
                proposal.as_bytes(),
            )
            .unwrap();
            let plan = repository
                .prepare_authored_change(&decoded.semantic, decoded.options)
                .unwrap();
            repository.publish(&plan.publication).unwrap();
        }
        let source = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let original = prepare_snapshot(&source);
        let entry = declaration_named(&source, "entry");
        let expected_texts = (0..799)
            .map(|index| {
                (
                    OwnerKey::Declaration(declaration_named(&source, &format!("payload-{index}")).declaration),
                    text(index),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut expected_owners = expected_texts.keys().copied().collect::<BTreeSet<_>>();
        assert!(expected_owners.insert(OwnerKey::Declaration(entry.declaration)));
        assert_eq!(expected_owners.len(), 800);
        assert_eq!(original.functions.len(), 800);
        let mut observed_owners = BTreeSet::new();
        for (key, bytes) in &original.artifact().objects {
            if key.domain != ObjectDomain::CompilerUnit {
                continue;
            }
            let unit = crate::platform::compiler::CompilationUnit::decode(bytes, *key).unwrap();
            assert!(observed_owners.insert(unit.source.owner));
            if let Some(expected) = expected_texts.get(&unit.source.owner) {
                assert_eq!(unit.tables.texts.len(), 1);
                let CompiledText::Inline(actual) = &unit.tables.texts[0] else {
                    panic!("capacity literal was not inline");
                };
                assert_eq!(actual.as_str(), expected.as_str());
            }
        }
        assert_eq!(observed_owners, expected_owners);

        let input = selfhost_artifact_input::bundle(original.artifact()).unwrap();
        let at = |offset| u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap()) as usize;
        assert_eq!(at(8), 5);
        assert_eq!(at(76), 800);
        assert_eq!(&input[32..40], &[0; 8]);
        assert!(input.len() > 8 * 1_048_576, "input_bytes={}", input.len());
        let meta = at(88);
        for count in [at(84), at(meta + 192), at(meta + 264), at(meta + 304)] {
            assert!(count <= 100_000);
        }
        for index in 0..800 {
            assert!(at(160 + index * 8 + 4) <= 8 * 1_048_576);
        }
        let author_ms = author_started.elapsed().as_millis();
        let (compiler_source, compiler_program) = compiler();
        let native = crate::platform::compiler::elf_x86_64::compile_function(
            compiler_program,
            compiler_program
                .function(declaration_named(compiler_source, "compile-elf"))
                .unwrap()
                .0 as usize,
        )
        .unwrap()
        .0;
        let started = std::time::Instant::now();
        let emitted = native_filter_tests::execute_filter(&native, &input, &[], 65521);
        let native_ms = started.elapsed().as_millis();
        assert_eq!(emitted.status.code(), Some(0), "{:?}", emitted.stderr);
        assert!(emitted.stderr.is_empty());
        let rejected = native_filter_tests::execute_filter(
            &native, &capacity_invalid_unused_unit(&input), &[], 65521,
        );
        assert_eq!(rejected.status.code(), Some(65));
        assert!(rejected.stdout.is_empty());
        assert_eq!(rejected.stderr, b"compiler_rejected:10\n");

        let loaded = crate::platform::compiler::load_artifact(&emitted.stdout).unwrap();
        let package = &loaded.manifest.packages[0];
        let compilation = crate::platform::compiler::CompilationManifest::decode(
            loaded.objects.get(&package.compilation.object_key()).unwrap(), package.compilation,
        ).unwrap();
        let mut pages = MemoryPageStore::default();
        for (key, bytes) in &loaded.objects {
            if key.domain == ObjectDomain::MapPage {
                pages.write_page(PageDigest::from_bytes(key.digest.bytes()), bytes).unwrap();
            }
        }
        let map_report = PersistentMap::from_root(compilation.units)
            .verify(&pages, &mut MapWork::default()).unwrap();
        assert_eq!(map_report.entries, 800);
        assert!(map_report.pages > 1, "complete artifact must use a hierarchy");
        eprintln!("selfhost artifact capacity unit_map_pages={} unit_map_root={} logical_content={}",
            map_report.pages, compilation.units.page(), compilation.units.content());

        let preserved = |domain| {
            matches!(
                domain,
                ObjectDomain::CompilerUnit
                    | ObjectDomain::Owner
                    | ObjectDomain::Type
                    | ObjectDomain::Blob
                    | ObjectDomain::PackageRevision
                    | ObjectDomain::PackageInterface
            )
        };
        let expected_keys = original
            .artifact()
            .objects
            .keys()
            .filter(|key| preserved(key.domain))
            .copied()
            .collect::<BTreeSet<_>>();
        let actual_keys = loaded
            .objects
            .keys()
            .filter(|key| preserved(key.domain))
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_keys, expected_keys);
        for key in expected_keys {
            assert!(
                loaded.objects.get(&key) == original.artifact().objects.get(&key),
                "capacity object differs: {key:?}"
            );
        }
        let program = NormalizedProgram::prepare(loaded).unwrap();
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
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
        eprintln!(
            "selfhost artifact capacity units=800 unused_text_functions=799 input_bytes={} input_sha256={} artifact_bytes={} artifact_sha256={} C1_sha256={} author_ms={author_ms} native_ms={native_ms}",
            input.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&input)),
            emitted.stdout.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&emitted.stdout)),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&native)),
        );
        (input, emitted.stdout)
    })
}

pub(super) fn capacity_invalid_unused_unit(input: &[u8]) -> Vec<u8> {
    let at = |offset| u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap()) as usize;
    let count = at(76);
    assert!(count > 700);
    // The entry has no text table; a later text-bearing unit is one of the
    // unused payload functions beyond the former admission boundary.
    let unit = (700..count)
        .map(|index| at(160 + index * 8))
        .find(|unit| at(unit + 256 + 9 * 8 + 4) == 1)
        .unwrap();
    let mut changed = input.to_vec();
    changed[unit + 12..unit + 16].copy_from_slice(&1_u32.to_le_bytes());
    input::rechecksum(&mut changed);
    changed
}

#[test]
fn selfhost_artifact_input_exceeds_former_unit_and_aggregate_byte_quotas() {
    capacity_bundle();
}
