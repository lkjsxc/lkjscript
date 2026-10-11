use super::*;

fn rejected(public: &Native, label: &str, body: &str, before: &str) {
    let input = public.input(
        &format!("rejected-{label}.lkjc"),
        &format!("request base={before}\n{body}"),
    );
    let records = public.plan(&input, false);
    let diagnostic = compact_record(&records, "diagnostic");
    assert_eq!(
        compact_field(diagnostic, "class"),
        "semantic",
        "{label}: {records:?}"
    );
    assert!(
        compact_field(diagnostic, "code").starts_with("kernel_"),
        "{label}: {records:?}"
    );
    assert_eq!(
        public.revision(),
        before,
        "failed request changed accepted meaning: {label}"
    );
}

#[test]
fn native_data_sequences_reject_hidden_types_active_loans_and_unsafe_substitutions() {
    let data = DataSequences::new();
    let public = &data.consumer;
    let before = public.revision();
    let types = [
        ("unused-secret", "Secret"),
        (
            "unused-hidden-secret",
            "(record (visible I64) (hidden (list Secret)))",
        ),
        ("unused-callable", "(function () I64)"),
        ("unused-stream", "(stream I64)"),
    ];
    for (label, element) in types {
        rejected(
            public,
            label,
            &format!(
                "declarations.begin\n(units (module create rejected-data\n  (function create unused (visibility private) (effect pure)\n    (parameter create ignored (type (owned-sequence {element})) (use consume))\n    (returns Unit) (body (unit)))))\ndeclarations.end\n"
            ),
            &before,
        );
    }
    let cases = [
        (
            "double-consumption",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use consume)) (returns Unit)
    (body (sequence
      (sequence-pop (type (owned-sequence I64)) (local values))
      (sequence-pop (type (owned-sequence I64)) (local values)) (unit))))))
declarations.end
"#,
        ),
        (
            "ordinary-element-borrow",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use borrow)) (returns I64)
    (body (borrow-owned-item (type (owned-sequence I64)) (local values)
      (index (i64 0)) (binding view (type I64)) (in (local view)))))))
declarations.end
"#,
        ),
        (
            "protected-sequence-replacement",
            r#"declarations.begin
(units (module create rejected-data
  (type-alias Inner (owned-sequence I64)) (type-alias Outer (owned-sequence Inner))
  (function create bad (visibility private) (effect pure)
    (parameter create values (type Outer) (use consume))
    (parameter create replacement (type Inner) (use consume)) (returns Unit)
    (body (borrow-owned-item (type Outer) (local values) (index (i64 0)) (binding view (type Inner))
      (in (sequence (sequence-replace (type Outer) (index (i64 0)) (local replacement) (local values)) (unit))))))))
declarations.end
"#,
        ),
        (
            "owning-get",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence OwnedI64Cell)) (use borrow)) (returns OwnedI64Cell)
    (body (sequence-get (type (owned-sequence OwnedI64Cell)) (local values) (index (i64 0)))))))
declarations.end
"#,
        ),
        (
            "untaken-inadmissible-type",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure) (returns Unit)
    (body (if (bool false) (sequence (sequence-empty (type (owned-sequence Secret))) (unit)) (unit))))))
declarations.end
"#,
        ),
        (
            "unconstrained-generic",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure) (type-parameter create T)
    (returns (owned-sequence T)) (body (sequence-empty (type (owned-sequence T)))))))
declarations.end
"#,
        ),
        (
            "phantom-argument",
            r#"declarations.begin
(units (module create rejected-data
  (record create Marker (visibility private) (type-parameter create T) (field create tag (type I64)))
  (function create bad (visibility private) (effect pure) (returns Unit)
    (body (sequence (sequence-empty (type (owned-sequence (Marker (stream I64))))) (unit))))))
declarations.end
"#,
        ),
        (
            "unused-substitution",
            r#"declarations.begin
(units (module create rejected-data
  (function create ignored (visibility private) (effect pure)
    (type-parameter create T (constraint owned)) (returns Unit) (body (unit)))
  (function create bad (visibility private) (effect pure) (returns Unit)
    (body (call ignored (types (owned-sequence (record (hidden Secret)))))))))
declarations.end
"#,
        ),
        (
            "borrowed-source-consume",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use borrow)) (returns Unit)
    (body (sequence (sequence-replace (type (owned-sequence I64)) (index (i64 0)) (i64 9) (local values)) (unit))))))
declarations.end
"#,
        ),
        (
            "ordinary-container-owner",
            r#"declarations.begin
(units (module create rejected-data
  (function create unused (visibility private) (effect pure)
    (parameter create ignored (type (list (owned-sequence I64)))) (returns Unit) (body (unit)))))
declarations.end
"#,
        ),
        (
            "wrong-index",
            r#"declarations.begin
(units (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use borrow)) (returns I64)
    (body (sequence-get (type (owned-sequence I64)) (local values) (index (bool false)))))))
declarations.end
"#,
        ),
        (
            "push-value-consumes-source",
            r#"declarations.begin
(units (use std builtin) (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use consume)) (returns (owned-sequence I64))
    (body (sequence-push (type (owned-sequence I64))
      (sequence (call std::data-sequence-discard (types I64) (local values)) (i64 9)) (local values))))))
declarations.end
"#,
        ),
        (
            "index-consumes-source",
            r#"declarations.begin
(units (use std builtin) (module create rejected-data
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use consume)) (returns I64)
    (body (sequence-get (type (owned-sequence I64)) (local values)
      (index (sequence (call std::data-sequence-discard (types I64) (local values)) (i64 0))))))))
declarations.end
"#,
        ),
        (
            "replacement-must-own-local",
            r#"declarations.begin
(units (module create rejected-data
  (external create new (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (function create bad (visibility private) (effect pure)
    (parameter create values (type (owned-sequence OwnedI64Cell)) (use consume))
    (returns (owned-product (field rest (owned-sequence OwnedI64Cell)) (field value OwnedI64Cell)))
    (body (sequence-replace (type (owned-sequence OwnedI64Cell)) (index (i64 0)) (call new (i64 7)) (local values))))))
declarations.end
"#,
        ),
    ];
    for (label, body) in cases {
        rejected(public, label, body, &before);
    }
    data.expected(
        "data-scalar",
        &json!([7, 42]),
        &oracle::snapshot(&json!(7), &json!(42)),
        false,
    );
    data.expected(
        "data-owned-replace",
        &json!([11, 31]),
        &json!({"displaced":11,"first":31,"last":9,"length":2}),
        false,
    );
    data.verify_identity();
}
