//! Domain-derived hashing needed by current artifact object/container identities.
use super::*;
use crate::platform::compiler::elf_x86_64;

pub(super) fn fixture(context: &str) -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let context = serde_json::to_string(context).unwrap();
    let source = byte_buffer_tests::author_only(&format!(r#"
declarations.begin
(units (module create derived-hash
  (external create primitive (visibility private) (implementation core.bytes.blake3-derive-key)
    (parameter create context (type Text)) (parameter create data (type Bytes)) (returns Bytes))
  (function create hash (visibility public) (effect pure)
    (parameter create data (type Bytes)) (returns Bytes)
    (body (call primitive (text {context}) (local data))))
  (function create filter (visibility public) (effect pure)
    (parameter create data (type Bytes)) (returns (record (diagnostic I64) (image Bytes)))
    (body (record structural (field diagnostic (i64 0)) (field image (call hash (local data))))))))
declarations.end
"#)).unwrap();
    let program = prepare_snapshot(&source);
    (source, program)
}

fn agrees(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    image: &[u8],
    input: &[u8],
    expected: &str,
) {
    let output = native_filter_tests::execute_filter(image, input, &[], 127);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(crate::platform::semantic_id::encode_hex(&output.stdout), expected);
    let declaration = declaration_named(source, "hash");
    let values = vec![NormalizedValue::bytes(input)];
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::default();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(declaration, values.clone(), None, &control).unwrap().0;
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(declaration, values, None, &control).unwrap().0;
    assert_eq!(vm, reference);
    assert_eq!(vm, NormalizedValue::bytes(output.stdout.as_slice()));
}

#[test]
fn native_elf_blake3_derivation_matches_independent_block_chunk_and_tree_vectors() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/blake3-derive-vectors.json"
    )).unwrap();
    let (source, program) = fixture(vectors["context"].as_str().unwrap());
    let index = program.function(declaration_named(&source, "filter")).unwrap().0 as usize;
    let image = elf_x86_64::compile_function(&program, index).unwrap().0;
    for vector in vectors["cases"].as_array().unwrap() {
        let length = vector["input_len"].as_u64().unwrap() as usize;
        let input: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
        agrees(&source, &program, &image, &input, vector["hash"].as_str().unwrap());
    }
}

#[test]
fn native_elf_blake3_derivation_preserves_context_bytes_and_tree_boundaries() {
    let mut contexts = vec![String::new(), "a\0b".into(), "a\0c".into(), "λ雪🦀".into()];
    contexts.extend([63, 64, 65, 1023, 1024, 1025, 2048, 2049].map(|n| "x".repeat(n)));
    for context in contexts {
        let (source, program) = fixture(&context);
        let index = program.function(declaration_named(&source, "filter")).unwrap().0 as usize;
        let image = elf_x86_64::compile_function(&program, index).unwrap().0;
        for length in [0, 65, 2049] {
            let input: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
            let expected = blake3::derive_key(&context, &input);
            agrees(&source, &program, &image, &input,
                &crate::platform::semantic_id::encode_hex(&expected));
        }
    }
}
