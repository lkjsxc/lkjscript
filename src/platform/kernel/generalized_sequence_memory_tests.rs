//! Ordinary element values are unrestricted while sequence custody remains affine.
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create generalized-sequence-memory
  (function create empty (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (returns (owned-sequence T))
    (body (sequence-empty (type (owned-sequence T)))))
  (function create push (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create value (type T))
    (parameter create seq (type (owned-sequence T)) (use consume))
    (returns (owned-sequence T))
    (body (sequence-push (type (owned-sequence T))
      (sequence (i64 2) (local value)) (local seq))))
  (function create get (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create index (type I64))
    (parameter create seq (type (owned-sequence T)) (use borrow))
    (returns T)
    (body (sequence-get (type (owned-sequence T)) (local seq) (index (local index)))))
  (function create replace (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create index (type I64))
    (parameter create value (type T))
    (parameter create seq (type (owned-sequence T)) (use consume))
    (returns (owned-product (field rest (owned-sequence T)) (field value T)))
    (body (sequence-replace (type (owned-sequence T)) (index (local index))
      (sequence (i64 3) (local value)) (local seq))))
  (function create pop (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create seq (type (owned-sequence T)) (use consume))
    (returns (owned-choice (case empty (owned-sequence T))
      (case item (owned-product (field rest (owned-sequence T)) (field value T)))))
    (body (sequence-pop (type (owned-sequence T)) (local seq))))
  (function create replace-owned (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use consume))
    (parameter create seq (type (owned-sequence T)) (use consume))
    (returns (owned-product (field rest (owned-sequence T)) (field value T)))
    (body (sequence-replace (type (owned-sequence T)) (index (i64 0)) (local value) (local seq))))
  (function create discard (visibility private) (effect pure)
    (parameter create seq (type (owned-sequence I64)) (use consume))
    (returns I64) (body (i64 0)))
  (function create read (visibility public) (effect pure)
    (parameter create seq (type (owned-sequence I64)) (use consume))
    (returns I64)
    (body (let (binding saved (type I64)
      (sequence-get (type (owned-sequence I64)) (local seq) (index (i64 0))))
      (in (sequence (call discard (local seq)) (local saved))))))
  (function create aggregate (visibility public) (effect pure)
    (parameter create seq (type (owned-sequence (list (option I64)))) (use borrow))
    (returns (list (option I64)))
    (body (sequence-get (type (owned-sequence (list (option I64)))) (local seq) (index (i64 0)))))))
declarations.end"#;

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

#[test]
fn generalized_sequence_memory_accepts_ordinary_generics_snapshots_and_owned_replacement() {
    let snapshot = author(SOURCE).unwrap();
    validate_full(&snapshot).unwrap();
    assert!(memory_reference::accepts(&snapshot));
}

#[test]
fn generalized_sequence_memory_rechecks_source_after_index_and_value_effects() {
    for replacement in [
        "(index (sequence (call discard (local seq)) (i64 0)))",
        "(index (sequence (if (bool false) (call discard (local seq)) (i64 0)) (i64 0)))",
    ] {
        let changed = SOURCE.replace(
            "(index (i64 0))))\n      (in",
            &format!("{replacement}))\n      (in"),
        );
        assert_ne!(changed, SOURCE);
        assert!(
            author(&changed)
                .unwrap_err()
                .contains("kernel_buffer_ownership")
        );
    }
    let changed = SOURCE.replace(
        "(sequence (i64 3) (local value)) (local seq)",
        "(sequence (local seq) (local value)) (local seq)",
    );
    assert!(
        author(&changed).is_err(),
        "ordinary operand evaluation cannot move its source owner"
    );
}

#[test]
fn generalized_sequence_memory_rejects_owned_get_and_consumed_read_sources() {
    let owned_get = r#"declarations.begin
(units (module create owned-get
  (function create get (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create seq (type (owned-sequence T)) (use borrow))
    (returns T)
    (body (sequence-get (type (owned-sequence T)) (local seq) (index (i64 0)))))))
declarations.end"#;
    assert!(
        author(owned_get)
            .unwrap_err()
            .contains("kernel_owned_sequence"),
        "get cannot copy an owned element"
    );
    let changed = SOURCE.replace(
        "(sequence-get (type (owned-sequence T)) (local seq)",
        "(sequence-get (type (owned-sequence T)) (sequence-empty (type (owned-sequence T)))",
    );
    assert_ne!(changed, SOURCE);
    assert!(
        author(&changed).is_err(),
        "get requires an exact local source"
    );
    let changed = SOURCE.replace(
        "(index (i64 0)) (local value) (local seq)",
        "(index (i64 0)) (sequence (unit) (local value)) (local seq)",
    );
    assert_ne!(changed, SOURCE);
    assert!(
        author(&changed).is_err(),
        "owned replacement requires an exact owning local"
    );
    let changed = SOURCE.replace(
        "(body (sequence-get (type (owned-sequence T)) (local seq) (index (local index))))",
        "(body (borrow-owned-item (type (owned-sequence T)) (local seq) (index (local index)) (binding view (type T)) (in (local view))))",
    );
    assert_ne!(changed, SOURCE);
    assert!(
        author(&changed).is_err(),
        "ordinary elements do not acquire lexical owner views"
    );
}

#[test]
fn generalized_sequence_memory_rejects_mutation_through_borrowed_owners() {
    let changed = SOURCE.replace(
        "(parameter create seq (type (owned-sequence T)) (use consume))",
        "(parameter create seq (type (owned-sequence T)) (use borrow))",
    );
    assert!(
        author(&changed)
            .unwrap_err()
            .contains("kernel_buffer_ownership")
    );
}

#[test]
fn generalized_sequence_memory_oracle_rejects_changed_replace_envelopes() {
    let snapshot = author(SOURCE).unwrap();
    let replacements = snapshot
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            matches!(
                owner,
                OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::SequenceReplace { .. },
                    ..
                })
            )
            .then_some(*key)
        })
        .collect::<Vec<_>>();
    assert_eq!(replacements.len(), 2);
    for key in replacements {
        let mut changed = snapshot.clone();
        let OwnerRecord::Expression(expression) = changed.owners.get_mut(&key).unwrap() else {
            unreachable!()
        };
        let ExpressionOperation::SequenceReplace {
            sequence_type,
            result_type,
            ..
        } = &mut expression.operation
        else {
            unreachable!()
        };
        *result_type = *sequence_type;
        assert!(!memory_reference::accepts(&changed));
        assert!(validate_full(&changed).is_err());
    }
}
