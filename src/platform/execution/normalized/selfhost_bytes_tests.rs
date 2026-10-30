//! Independent execution of the compiler's byte and aggregate backend closure.
use super::*;

fn foreground(payload: &[u8]) -> (i64, Vec<u8>) {
    let (source, program) = compiler();
    compile_with_policy(source, program, payload, NormalizedRunPolicy::foreground())
}

#[test]
fn selfhost_byte_runtime_emits_an_independent_integer_program() {
    let payload = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    let (code, image) = foreground(&payload);
    assert_eq!(code, 0);
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"42");
    assert!(output.stderr.is_empty());
}

fn build_named(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    name: &str,
) -> Vec<u8> {
    let index = program.function(declaration_named(source, name)).unwrap().0 as usize;
    let payload = input::export(program, index).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let (code, image) = foreground(&payload);
    assert_eq!(code, 0, "{name}");
    image
}

#[test]
fn selfhost_byte_borrowed_final_call_retains_lexical_cleanup() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-buffer-borrow-tail.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let image = build_named(&source, &program, "entry");
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"0");
    assert!(output.stderr.is_empty());
}

#[test]
fn selfhost_byte_aggregates_preserve_field_order_nesting_and_list_bounds() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-aggregates.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    for (name, expected) in [
        ("reordered-fields", Ok(42)),
        ("nested-fields", Ok(42)),
        ("list-value", Ok(42)),
        ("list-empty", Ok(0)),
        ("list-end", Err("list_index")),
        ("list-negative", Err("list_index")),
        ("field-evaluation-order", Err("integer_overflow")),
    ] {
        let image = build_named(&source, &program, name);
        let output = execute(&image, &[]);
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let host = execute(
            &crate::platform::compiler::elf_x86_64::compile_function(&program, index)
                .unwrap()
                .0,
            &[],
        );
        assert_eq!(output.status.code(), host.status.code(), "{name}");
        assert_eq!(output.stdout, host.stdout, "{name}");
        assert_eq!(output.stderr, host.stderr, "{name}");
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        let vm =
            NormalizedVm::for_test(&program, policy).invoke(declaration, vec![], None, &control);
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy).invoke(
            declaration,
            vec![],
            None,
            &control,
        );
        match expected {
            Ok(value) => {
                assert_eq!(output.status.code(), Some(0), "{name}: {output:?}");
                assert_eq!(output.stdout, value.to_string().as_bytes());
                assert!(output.stderr.is_empty());
                assert_eq!(vm.unwrap().0, NormalizedValue::I64(value));
                assert_eq!(reference.unwrap().0, NormalizedValue::I64(value));
            }
            Err(trap) => {
                assert_eq!(output.status.code(), Some(70), "{name}: {output:?}");
                assert!(output.stdout.is_empty());
                assert_eq!(output.stderr, format!("normalized_{trap}\n").as_bytes());
                assert_eq!(vm.unwrap_err().code, format!("normalized_{trap}"));
                assert_eq!(reference.unwrap_err().code, format!("reference_{trap}"));
            }
        }
    }
}

#[test]
fn selfhost_byte_filter_grows_input_and_preserves_usage_and_traps() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/native-filter.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let echo = build_named(&source, &program, "echo");
    for length in [
        0,
        1,
        256,
        65_535,
        65_536,
        65_537,
        256 * 1024,
        16 * 1024 * 1024,
        16 * 1024 * 1024 + 1,
        20 * 1024 * 1024 + 17,
    ] {
        let bytes: Vec<_> = (0..length)
            .map(|i| (i ^ (i >> 8) ^ (i >> 16)) as u8)
            .collect();
        let output = native_filter_tests::execute_filter(
            &echo,
            &bytes,
            &[],
            if length < 1024 { 1 } else { 65521 },
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "length {length}: stderr={:?}",
            output.stderr
        );
        assert!(output.stderr.is_empty());
        assert!(output.stdout == bytes, "echo length {length}");
    }
    native_filter_tests::check_filter_host_failures(&echo);
    let output = native_filter_tests::execute_filter(&echo, b"", &["extra"], 1);
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"native_argument\n");
    let output = native_filter_tests::execute_filter(
        &build_named(&source, &program, "reject"),
        b"secret output",
        &[],
        1,
    );
    assert_eq!(output.status.code(), Some(65));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"compiler_rejected:-123\n");
    let output =
        native_filter_tests::execute_filter(&build_named(&source, &program, "trap"), b"", &[], 1);
    assert_eq!(output.status.code(), Some(70));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"normalized_integer_division\n");
}

#[test]
fn selfhost_byte_blake3_matches_independent_block_chunk_and_tree_vectors() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/native-filter.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let image = build_named(&source, &program, "hash");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/blake3-unkeyed-vectors.json"
    ))
    .unwrap();
    for vector in vectors["cases"].as_array().unwrap() {
        let length = vector["input_len"].as_u64().unwrap() as usize;
        let data: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
        let output = native_filter_tests::execute_filter(&image, &data, &[], 127);
        assert_eq!(output.status.code(), Some(0), "length {length}: {output:?}");
        assert!(output.stderr.is_empty());
        assert_eq!(
            crate::platform::semantic_id::encode_hex(&output.stdout),
            vector["hash"].as_str().unwrap(),
            "length {length}"
        );
    }
}

#[test]
fn selfhost_byte_operations_preserve_independent_values_and_traps() {
    let cases = [
        (
            "roundtrip",
            "(call native-bytes::buffer-roundtrip (i64 255))",
            Ok(255),
        ),
        (
            "growth",
            "(call native-bytes::buffer-growth (i64 4097) (i64 4096))",
            Ok(255),
        ),
        (
            "size",
            "(call native-bytes::buffer-size (i64 8193))",
            Ok(8193),
        ),
        (
            "discard",
            "(sequence (call native-bytes::buffer-discard) (i64 42))",
            Ok(42),
        ),
        ("octet", "(call native-bytes::octet (i64 128))", Ok(128)),
        ("index", "(call native-bytes::index (i64 2))", Ok(255)),
        ("empty", "(call native-bytes::empty-length)", Ok(0)),
        (
            "same",
            "(if (call native-bytes::same-octets) (i64 42) (i64 0))",
            Ok(42),
        ),
        (
            "different",
            "(if (call native-bytes::different-octets) (i64 0) (i64 42))",
            Ok(42),
        ),
        (
            "different-length",
            "(if (call native-bytes::different-length) (i64 0) (i64 42))",
            Ok(42),
        ),
        (
            "empty-equal",
            "(if (call native-bytes::empty-equal) (i64 42) (i64 0))",
            Ok(42),
        ),
        ("nul", "(call native-bytes::text-octets (i64 0))", Ok(0)),
        (
            "unicode",
            "(call native-bytes::text-octets (i64 4))",
            Ok(231),
        ),
        (
            "range",
            "(call native-bytes::range-length (i64 1) (i64 3))",
            Ok(2),
        ),
        (
            "empty-range",
            "(call native-bytes::range-length (i64 3) (i64 3))",
            Ok(0),
        ),
        (
            "copied",
            "(if (call native-bytes::copied-range) (i64 42) (i64 0))",
            Ok(42),
        ),
        (
            "concat",
            "(if (call native-bytes::concatenated) (i64 42) (i64 0))",
            Ok(42),
        ),
        (
            "empty-concat",
            "(if (call native-bytes::empty-concat) (i64 42) (i64 0))",
            Ok(42),
        ),
        ("empty-copy", "(call native-bytes::copied-empty)", Ok(0)),
        ("hash-size", "(call native-bytes::hash-length)", Ok(32)),
        (
            "octet-negative",
            "(call native-bytes::octet (i64 -1))",
            Err("bytes_octet"),
        ),
        (
            "octet-large",
            "(call native-bytes::octet (i64 256))",
            Err("bytes_octet"),
        ),
        (
            "index-negative",
            "(call native-bytes::index (i64 -1))",
            Err("bytes_index"),
        ),
        (
            "index-end",
            "(call native-bytes::index (i64 3))",
            Err("bytes_index"),
        ),
        (
            "index-empty",
            "(call native-bytes::empty-index)",
            Err("bytes_index"),
        ),
        (
            "range-negative",
            "(call native-bytes::range-length (i64 -1) (i64 2))",
            Err("bytes_range"),
        ),
        (
            "range-reversed",
            "(call native-bytes::range-length (i64 2) (i64 1))",
            Err("bytes_range"),
        ),
        (
            "range-end",
            "(call native-bytes::range-length (i64 0) (i64 4))",
            Err("bytes_range"),
        ),
        (
            "buffer-negative",
            "(call native-bytes::buffer-roundtrip (i64 -1))",
            Err("buffer_octet"),
        ),
        (
            "buffer-large",
            "(call native-bytes::buffer-roundtrip (i64 256))",
            Err("buffer_octet"),
        ),
        (
            "buffer-empty",
            "(call native-bytes::buffer-growth (i64 0) (i64 0))",
            Err("buffer_index"),
        ),
        (
            "buffer-end",
            "(call native-bytes::buffer-growth (i64 8) (i64 8))",
            Err("buffer_index"),
        ),
    ];
    let mut proposal = include_str!("../../../../tests/fixtures/native-bytes.lkjc").to_owned();
    proposal.push_str("\ndeclarations.begin\n(units (module create wrappers\n");
    for (name, body, _) in cases {
        proposal.push_str(&format!("(function create case-{name} (visibility public) (effect pure) (returns I64) (body {body}))\n"));
    }
    proposal.push_str("))\ndeclarations.end\n");
    let source = byte_buffer_tests::author_only(&proposal).unwrap();
    let program = prepare_snapshot(&source);
    for (name, _, expected) in cases {
        eprintln!("selfhost byte case={name}");
        let declaration = declaration_named(&source, &format!("case-{name}"));
        let index = program.function(declaration).unwrap().0 as usize;
        let image = build_named(&source, &program, &format!("case-{name}"));
        let output = execute(&image, &[]);
        let host = execute(
            &crate::platform::compiler::elf_x86_64::compile_function(&program, index)
                .unwrap()
                .0,
            &[],
        );
        assert_eq!(output.status.code(), host.status.code(), "{name}");
        assert_eq!(output.stdout, host.stdout, "{name}");
        assert_eq!(output.stderr, host.stderr, "{name}");
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        let vm =
            NormalizedVm::for_test(&program, policy).invoke(declaration, vec![], None, &control);
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy).invoke(
            declaration,
            vec![],
            None,
            &control,
        );
        if let Ok(expected) = expected {
            assert_eq!(output.status.code(), Some(0), "{name}: {output:?}");
            assert_eq!(output.stdout, expected.to_string().as_bytes(), "{name}");
            assert!(output.stderr.is_empty());
            assert_eq!(vm.unwrap().0, NormalizedValue::I64(expected));
            assert_eq!(reference.unwrap().0, NormalizedValue::I64(expected));
        } else if let Err(trap) = expected {
            assert_eq!(output.status.code(), Some(70), "{name}: {output:?}");
            assert!(output.stdout.is_empty());
            assert_eq!(
                output.stderr,
                format!("normalized_{trap}\n").as_bytes(),
                "{name}"
            );
            assert_eq!(vm.unwrap_err().code, format!("normalized_{trap}"));
            let reference_prefix = if trap.starts_with("buffer_") {
                "normalized"
            } else {
                "reference"
            };
            assert_eq!(
                reference.unwrap_err().code,
                format!("{reference_prefix}_{trap}")
            );
        }
    }
}

#[test]
fn selfhost_byte_blake3_derivation_preserves_independent_vectors_and_contexts() {
    let vectors = native_derive_tests::vectors();
    let (source, program) = native_derive_tests::fixture(vectors["context"].as_str().unwrap());
    native_derive_tests::check_vectors(&build_named(&source, &program, "filter"));
    for context in native_derive_tests::contexts() {
        let (source, program) = native_derive_tests::fixture(&context);
        let image = build_named(&source, &program, "filter");
        for length in [0, 65, 2049] {
            let input: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
            let output = native_filter_tests::execute_filter(&image, &input, &[], 127);
            assert_eq!(
                output.status.code(),
                Some(0),
                "context length {}: {output:?}",
                context.len()
            );
            assert!(output.stderr.is_empty());
            assert_eq!(output.stdout, blake3::derive_key(&context, &input));
        }
    }
}

#[test]
fn selfhost_byte_backend_reproduces_its_complete_native_closure() {
    let (source, program) = compiler();
    let entry = program
        .function(declaration_named(source, "compile-elf"))
        .unwrap()
        .0 as usize;
    let payload = input::export(program, entry).unwrap();
    let (code, c2) = foreground(&payload);
    assert_eq!(code, 0);
    let output = native_filter_tests::execute_filter(&c2, &payload, &[], 4093);
    assert_eq!(output.status.code(), Some(0), "stderr={:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert!(
        c2 == output.stdout,
        "native backend C2/C3 differ: C2={} C3={}",
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&c2)),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&output.stdout))
    );
    eprintln!(
        "selfhost tier=backend native_closure functions={} nodes={} normalized_bytes={} C2_C3_bytes={} C2_C3_sha256={}",
        wide_at(&payload, 96),
        wide_at(&payload, 120),
        payload.len(),
        c2.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&c2))
    );
    let (artifact_input, artifact_expected) = artifact::complete_compiler();
    let (capacity_input, capacity_expected) = artifact::capacity_bundle();
    let independent_artifact_source = byte_buffer_tests::author_only(
        &include_str!("../../../../tests/fixtures/selfhost-artifact-empty-runtime.lkjc")
            .replace("(i64 42)", "(i64 -91)"),
    )
    .unwrap();
    let independent_artifact_program = prepare_snapshot(&independent_artifact_source);
    let independent_artifact_input = crate::platform::compiler::selfhost_artifact_input::bundle(
        independent_artifact_program.artifact(),
    )
    .unwrap();
    let independent_artifact_expected = artifact::image("compile-elf", &independent_artifact_input);
    for (name, stage) in [("C2", &c2), ("C3", &output.stdout)] {
        let produced = native_filter_tests::execute_filter(stage, artifact_input, &[], 4093);
        assert_eq!(
            produced.status.code(),
            Some(0),
            "{name}: {:?}",
            produced.stderr
        );
        assert!(produced.stderr.is_empty());
        assert!(
            produced.stdout == *artifact_expected,
            "{name} artifact differs"
        );
        crate::platform::compiler::load_artifact(&produced.stdout).unwrap();
        eprintln!(
            "selfhost tier=backend stage={name} artifact_input_bytes={} artifact_input_sha256={} artifact_bytes={} artifact_sha256={}",
            artifact_input.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(artifact_input)),
            produced.stdout.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&produced.stdout))
        );
        let capacity = native_filter_tests::execute_filter(stage, capacity_input, &[], 65521);
        assert_eq!(
            capacity.status.code(),
            Some(0),
            "{name}: {:?}",
            capacity.stderr
        );
        assert!(capacity.stderr.is_empty());
        assert!(
            capacity.stdout == *capacity_expected,
            "{name} capacity artifact differs"
        );
        eprintln!(
            "selfhost tier=backend stage={name} capacity_input_bytes={} capacity_input_sha256={} capacity_artifact_bytes={} capacity_artifact_sha256={}",
            capacity_input.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(capacity_input)),
            capacity.stdout.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&capacity.stdout)),
        );
        let rejected_capacity = native_filter_tests::execute_filter(
            stage,
            &artifact::capacity_invalid_unused_unit(capacity_input),
            &[],
            65521,
        );
        assert_eq!(rejected_capacity.status.code(), Some(65));
        assert!(rejected_capacity.stdout.is_empty());
        assert_eq!(rejected_capacity.stderr, b"compiler_rejected:10\n");

        let mut malformed = artifact_input.clone();
        malformed[40] ^= 1;
        let rejected = native_filter_tests::execute_filter(stage, &malformed, &[], 127);
        assert_eq!(rejected.status.code(), Some(65));
        assert!(rejected.stdout.is_empty());
        assert_eq!(rejected.stderr, b"compiler_rejected:4\n");
        for offset in [72, 88] {
            let mut malformed = artifact_input.clone();
            write_wide(
                &mut malformed,
                offset,
                wide_at(artifact_input, offset) ^ (1_u64 << 32),
            );
            input::rechecksum(&mut malformed);
            let rejected = native_filter_tests::execute_filter(stage, &malformed, &[], 127);
            assert_eq!(rejected.status.code(), Some(65));
            assert!(rejected.stdout.is_empty());
            assert_eq!(rejected.stderr, b"compiler_rejected:10\n");
        }
        let produced =
            native_filter_tests::execute_filter(stage, &independent_artifact_input, &[], 13);
        assert_eq!(produced.status.code(), Some(0));
        assert!(produced.stderr.is_empty());
        assert!(produced.stdout == independent_artifact_expected);
        let loaded = crate::platform::compiler::load_artifact(&produced.stdout).unwrap();
        let prepared = NormalizedProgram::prepare(loaded).unwrap();
        let entry = declaration_named(&independent_artifact_source, "entry");
        let control = ExecutionControl::uncancelled();
        let policy = NormalizedRunPolicy::foreground();
        assert_eq!(
            NormalizedVm::for_test(&prepared, policy)
                .invoke(entry, vec![], None, &control)
                .unwrap()
                .0,
            NormalizedValue::I64(-91)
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(&independent_artifact_source, &prepared, policy)
                .invoke(entry, vec![], None, &control)
                .unwrap()
                .0,
            NormalizedValue::I64(-91)
        );
    }
    let independent = input::arithmetic(&[(1, 6), (1, 7), (4, 0), (6, 0)]).unwrap();
    let buffer_source = byte_buffer_tests::author_only(&format!("{}\ndeclarations.begin\n(units (module create standalone (function create entry (visibility public) (effect pure) (returns I64) (body (call native-bytes::buffer-growth (i64 4097) (i64 4096))))))\ndeclarations.end\n", include_str!("../../../../tests/fixtures/native-bytes.lkjc"))).unwrap();
    let buffer_program = prepare_snapshot(&buffer_source);
    let buffer_entry = buffer_program
        .function(declaration_named(&buffer_source, "entry"))
        .unwrap()
        .0 as usize;
    let buffer_input = input::export(&buffer_program, buffer_entry).unwrap();
    let overflow_input = input::arithmetic(&[(1, i64::MAX), (1, 1), (2, 0), (6, 0)]).unwrap();
    let vectors = native_derive_tests::vectors();
    let (derived_source, derived_program) =
        native_derive_tests::fixture(vectors["context"].as_str().unwrap());
    let derived_entry = derived_program
        .function(declaration_named(&derived_source, "filter"))
        .unwrap()
        .0 as usize;
    let derived_input = input::export(&derived_program, derived_entry).unwrap();
    let (derived_code, derived_expected) = foreground(&derived_input);
    assert_eq!(derived_code, 0);
    for stage in [&c2, &output.stdout] {
        let derived = native_filter_tests::execute_filter(stage, &derived_input, &[], 127);
        assert_eq!(derived.status.code(), Some(0), "{derived:?}");
        assert!(derived.stderr.is_empty());
        assert!(derived.stdout == derived_expected);
        native_derive_tests::check_vectors(&derived.stdout);
        let compiled = native_filter_tests::execute_filter(stage, &independent, &[], 1);
        assert_eq!(
            compiled.status.code(),
            Some(0),
            "stderr={:?}",
            compiled.stderr
        );
        assert!(compiled.stderr.is_empty());
        assert_eq!(execute(&compiled.stdout, &[]).stdout, b"42");
        let mut malformed = independent.clone();
        malformed[40] ^= 1;
        let rejected = native_filter_tests::execute_filter(stage, &malformed, &[], 13);
        assert_eq!(rejected.status.code(), Some(65));
        assert!(rejected.stdout.is_empty());
        assert_eq!(rejected.stderr, b"compiler_rejected:4\n");
        let rejected = native_filter_tests::execute_filter(
            stage,
            &admission::forgotten_buffer_input(),
            &[],
            13,
        );
        assert_eq!(rejected.status.code(), Some(65));
        assert!(rejected.stdout.is_empty());
        assert_eq!(rejected.stderr, b"compiler_rejected:8\n");
        let buffer = native_filter_tests::execute_filter(stage, &buffer_input, &[], 127);
        assert_eq!(buffer.status.code(), Some(0));
        assert!(buffer.stderr.is_empty());
        assert_eq!(execute(&buffer.stdout, &[]).stdout, b"255");
        let overflow = native_filter_tests::execute_filter(stage, &overflow_input, &[], 127);
        assert_eq!(overflow.status.code(), Some(0));
        assert!(overflow.stderr.is_empty());
        let trap = execute(&overflow.stdout, &[]);
        assert_eq!(trap.status.code(), Some(70));
        assert!(trap.stdout.is_empty());
        assert_eq!(trap.stderr, b"normalized_integer_overflow\n");
    }
    // Feed changed compiler source through the same Rust-normalized boundary.
    // C2 and C3 themselves perform all subsequent native lowering and emission.
    let (mutant_source, mutant_program) = proposed_compiler(true);
    let mutant_entry = mutant_program
        .function(declaration_named(mutant_source, "compile-elf"))
        .unwrap()
        .0 as usize;
    let mutant_input = input::export(mutant_program, mutant_entry).unwrap();
    let (code, mutant_expected) = foreground(&mutant_input);
    assert_eq!(code, 0);
    let (code, changed_expected) = compile_with_policy(
        mutant_source,
        mutant_program,
        &independent,
        NormalizedRunPolicy::foreground(),
    );
    assert_eq!(code, 0);
    let mutant_artifact_input =
        crate::platform::compiler::selfhost_artifact_input::bundle(mutant_program.artifact())
            .unwrap();
    let mutant_artifact_expected = artifact::image("compile-elf", &mutant_artifact_input);
    assert!(mutant_artifact_expected != *artifact_expected);
    for stage in [&c2, &output.stdout] {
        let produced =
            native_filter_tests::execute_filter(stage, &mutant_artifact_input, &[], 4093);
        assert_eq!(produced.status.code(), Some(0), "{:?}", produced.stderr);
        assert!(produced.stderr.is_empty());
        assert!(produced.stdout == mutant_artifact_expected);
        let loaded = crate::platform::compiler::load_artifact(&produced.stdout).unwrap();
        let prepared = NormalizedProgram::prepare(loaded).unwrap();
        let (code, changed_artifact_output) = compile_with_policy(
            mutant_source,
            &prepared,
            &independent,
            NormalizedRunPolicy::foreground(),
        );
        assert_eq!(code, 0);
        assert!(changed_artifact_output == changed_expected);
        let mutant = native_filter_tests::execute_filter(stage, &mutant_input, &[], 4093);
        assert_eq!(mutant.status.code(), Some(0), "stderr={:?}", mutant.stderr);
        assert!(mutant.stderr.is_empty());
        assert!(mutant.stdout == mutant_expected);
        assert!(mutant.stdout != c2);
        let changed = native_filter_tests::execute_filter(&mutant.stdout, &independent, &[], 127);
        assert_eq!(
            changed.status.code(),
            Some(0),
            "stderr={:?}",
            changed.stderr
        );
        assert!(changed.stderr.is_empty());
        assert!(changed.stdout == changed_expected);
        assert_eq!(execute(&changed.stdout, &[]).stdout, b"42");
        let usage = execute(&changed.stdout, &["unexpected"]);
        assert_eq!(usage.status.code(), Some(64));
        assert!(usage.stdout.is_empty());
        assert_eq!(usage.stderr, b"mutant_argument\n");
    }
    eprintln!(
        "selfhost tier=backend mutated_artifact input_bytes={} input_sha256={} artifact_bytes={} artifact_sha256={}; C2/C3 bytes and mutated compiler behavior passed",
        mutant_artifact_input.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&mutant_artifact_input)),
        mutant_artifact_expected.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&mutant_artifact_expected))
    );
    eprintln!(
        "selfhost tier=backend mutated_source_input_sha256={} mutated_compiler_sha256={} mutated_compiler_bytes={}",
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&mutant_input)),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&mutant_expected)),
        mutant_expected.len()
    );
}
