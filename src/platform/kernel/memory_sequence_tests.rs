//! Exact owner admission and lexical sequence item custody through native authoring.
use super::*;

const SOURCE: &str = include_str!("sequence_memory_test_source.lkjc");

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

#[test]
fn sequence_memory_accepts_generic_operations_reads_and_preserved_ancestors() {
    let snapshot = author(SOURCE).unwrap();
    validate_full(&snapshot).unwrap();
    assert!(memory_reference::accepts(&snapshot));
}

#[test]
fn sequence_memory_rejects_source_moves_view_moves_and_escapes_in_read_scopes() {
    for body in [
        "(call dispose-sequence (local seq))",
        "(sequence (sequence-pop (type (owned-sequence ByteBuffer)) (local seq)) (i64 0))",
        "(sequence (let (binding escaped (type ByteBuffer) (local view)) (in (unit))) (i64 0))",
        "(if (bool false) (call dispose-sequence (local seq)) (i64 0))",
    ] {
        let source = SOURCE.replacen("(call length (local view))", body, 1);
        let failure = author(&source).unwrap_err();
        assert!(
            failure.contains("kernel_buffer_ownership"),
            "{body}: {failure}"
        );
    }
    let source = SOURCE.replace(
        "(in (sequence (call length (local view)) (local other)))",
        "(in (local view))",
    );
    let failure = author(&source).unwrap_err();
    assert!(failure.contains("kernel_buffer_ownership"), "{failure}");
}

#[test]
fn sequence_memory_checks_source_liveness_after_index_effects() {
    let source = SOURCE.replacen(
        "(index (i64 0)) (binding view (type ByteBuffer))",
        "(index (sequence (call dispose-sequence (local seq)) (i64 0))) (binding view (type ByteBuffer))",
        1,
    );
    let failure = author(&source).unwrap_err();
    assert!(failure.contains("kernel_buffer_ownership"), "{failure}");
}

#[test]
fn sequence_memory_rejects_ordinary_elements_and_temporary_read_sources() {
    for source in [
        SOURCE.replace("(constraint owned)", "(constraint transferable)"),
        SOURCE.replace("(owned-sequence ByteBuffer)", "(owned-sequence I64)"),
        SOURCE.replace(
            "(sequence-length (type (owned-sequence T)) (local seq))",
            "(sequence-length (type (owned-sequence T)) (sequence-empty (type (owned-sequence T))))",
        ),
        SOURCE.replace(
            "(sequence-push (type (owned-sequence T)) (local value) (local seq))",
            "(sequence-push (type (owned-sequence T)) (sequence-empty (type (owned-sequence T))) (local seq))",
        ),
    ] {
        assert!(author(&source).is_err(), "unexpected accepted source: {source}");
    }
}
