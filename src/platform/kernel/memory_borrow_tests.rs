//! Native lexical-read contracts and ancestor-loan accounting.
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create child-read
  (external create length (visibility public) (implementation core.buffer.length)
    (parameter create b (type ByteBuffer) (use borrow)) (returns I64))
  (external create discard (visibility private) (implementation core.buffer.discard)
    (parameter create b (type ByteBuffer) (use consume)) (returns Unit))
  (function create discard-packet (visibility private) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use consume))
    (returns Unit) (body (unit)))
  (function create read (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use consume))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local packet)
      (field payload (binding view (type ByteBuffer))) (in (call length (local view))))))))
declarations.end"#;

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

#[test]
fn scoped_read_accepts_borrowed_sources_nested_views_and_unrelated_owned_results() {
    let source = author(&format!(
        r#"{SOURCE}
declarations.begin
(units (module create composed-read
  (function create relay (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create packet (type (owned-product (field payload T))) (use consume))
    (returns (owned-product (field payload T)))
    (body (sequence
      (borrow-owned-field (type (owned-product (field payload T))) (local packet)
        (field payload (binding view (type T))) (in (unit)))
      (local packet))))
  (function create nested (visibility public) (effect pure)
    (parameter create packet
      (type (owned-product (field inner (owned-product (field payload ByteBuffer))))) (use borrow))
    (returns I64)
    (body (borrow-owned-field
      (type (owned-product (field inner (owned-product (field payload ByteBuffer))))) (local packet)
      (field inner (binding inner (type (owned-product (field payload ByteBuffer)))))
      (in (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local inner)
        (field payload (binding view (type ByteBuffer)))
        (in (call child-read::length (local view))))))))
  (function create other (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use borrow))
    (parameter create other (type ByteBuffer) (use consume)) (returns ByteBuffer)
    (body (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local packet)
      (field payload (binding view (type ByteBuffer))) (in (local other)))))
  (function create choice (visibility public) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (use borrow))
    (returns I64)
    (body (match-borrowed-owned (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (local outcome)
      (case accepted (binding value (type I64)) (in (local value)))
      (case rejected (binding value (type ByteBuffer)) (in (call child-read::length (local value)))))))))
declarations.end"#
    ))
    .unwrap();
    assert!(validate_full(&source).is_ok());
    assert!(memory_reference::accepts(&source));
}

#[test]
fn scoped_read_rejects_source_and_view_consumption_and_owned_escape() {
    for body in [
        "(sequence (call discard-packet (local packet)) (i64 0))",
        "(sequence (call discard (local view)) (i64 0))",
        "(sequence (let (binding saved (type ByteBuffer) (local view)) (in (unit))) (i64 0))",
        "(if (bool false) (sequence (call discard (local view)) (i64 0)) (i64 0))",
    ] {
        let failure = author(&SOURCE.replace("(call length (local view))", body)).unwrap_err();
        assert!(
            failure.contains("kernel_buffer_ownership"),
            "{body}: {failure}"
        );
    }
    let escaped = SOURCE
        .replace(
            "(returns I64)\n    (body",
            "(returns ByteBuffer)\n    (body",
        )
        .replace("(call length (local view))", "(local view)");
    let failure = author(&escaped).unwrap_err();
    assert!(failure.contains("kernel_buffer_ownership"), "{failure}");
}

#[test]
fn copied_choice_payload_keeps_its_whole_source_frozen() {
    let source = r#"declarations.begin
(units (module create copied-case
  (function create discard (visibility private) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (use consume))
    (returns I64) (body (i64 0)))
  (function create inspect (visibility public) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (use consume))
    (returns I64)
    (body (match-borrowed-owned (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (local outcome)
      (case accepted (binding value (type I64)) (in (local value)))
      (case rejected (binding value (type ByteBuffer)) (in (i64 1))))))))
declarations.end"#;
    assert!(author(source).is_ok());
    let failure =
        author(&source.replace("(in (local value))", "(in (call discard (local outcome)))"))
            .unwrap_err();
    assert!(failure.contains("kernel_buffer_ownership"), "{failure}");
}

#[test]
fn lexical_loan_provenance_freezes_every_ancestor_and_balances_nested_scopes() {
    let source = author(SOURCE).unwrap();
    let read = Check {
        read: &source,
        scope: None,
    };
    let root = LocalValueReference::LexicalBinding(
        crate::platform::semantic_id::BindingId::migrate(b"loan-parent-model", 0),
    );
    let child = LocalValueReference::LexicalBinding(
        crate::platform::semantic_id::BindingId::migrate(b"loan-parent-model", 1),
    );
    let leaf = LocalValueReference::LexicalBinding(
        crate::platform::semantic_id::BindingId::migrate(b"loan-parent-model", 2),
    );
    let mut state = State::from([(root, Slot::owner(false))]);
    read.loan(root, &mut state, true).unwrap();
    state.insert(child, Slot::view(root));
    read.loan(child, &mut state, true).unwrap();
    state.insert(leaf, Slot::view(child));
    read.loan(leaf, &mut state, true).unwrap();
    assert_eq!(
        (state[&root].loans, state[&child].loans, state[&leaf].loans),
        (3, 2, 1)
    );
    read.loan(leaf, &mut state, false).unwrap();
    state.remove(&leaf);
    read.loan(child, &mut state, false).unwrap();
    state.remove(&child);
    read.loan(root, &mut state, false).unwrap();
    assert!(state[&root].live);
    assert_eq!(state[&root].loans, 0);
    assert!(read.loan(root, &mut state, false).is_err());
}
