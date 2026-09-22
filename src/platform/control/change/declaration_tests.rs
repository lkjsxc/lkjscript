//! Literal compact intent and canonical graph observations are independent authoring oracles.
use super::*;
use crate::platform::change::canonical_authored_intent_bytes;
use crate::platform::publication::GraphRepository;

#[test]
fn native_complete_declarations_match_independent_flat_intent() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let header = format!("request base={}\n", created.current.head.revision);
    let flat = r#"
create.module as=$m name=native
create.record as=$batch module=$m name=Batch visibility=public
add.type-parameter as=$T declaration=$batch name=T
type.parameter as=@T parameter=$T
type.list as=@items item=@T
add.field as=$items record=$batch name=items type=@items
add.parameter as=$value function=$identity name=value type=i64
create.function as=$identity module=$m name=identity visibility=public result=i64 effect=pure body=$body
expression.block as=$body
(local $value)
expression.end
create.test as=$test module=$m name=identity-test visibility=private actual=$actual expected=$expected
expression.block as=$actual
(call $identity (i64 42))
expression.end
expression.block as=$expected
(i64 42)
expression.end
"#;
    let native = r#"
declarations.begin
(units
  (module create native (as $m)
    (record create Batch (as $batch) (visibility public)
      (type-parameter create T (as $T))
      (field create items (as $items) (type (list T))))
    (function create identity (as $identity) (visibility public)
      (parameter create value (as $value) (type I64))
      (returns I64) (effect pure) (body (local value)))
    (test create identity-test (as $test) (visibility private)
      (actual (call identity (i64 42))) (expected (i64 42)))))
declarations.end
"#;
    let decode = |source: &str| {
        decode_compact_change("literal.lkjc", format!("{header}{source}").as_bytes())
            .unwrap_or_else(|e| panic!("{e:#?}"))
    };
    let a = decode(flat);
    let b = decode(native);
    assert_eq!(
        canonical_authored_intent_bytes(&a.semantic).unwrap(),
        canonical_authored_intent_bytes(&b.semantic).unwrap()
    );
    let a = created
        .repository
        .prepare_authored_change(&a.semantic, a.options)
        .unwrap();
    let b = created
        .repository
        .prepare_authored_change(&b.semantic, b.options)
        .unwrap();
    assert_eq!(a.publication.head_bytes, b.publication.head_bytes);
    assert_eq!(a.publication.objects, b.publication.objects);
    created.repository.publish(&b.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let module = b.allocated["$m"];
    let draft = render_native_draft(
        &view,
        &[module],
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let draft = decode_compact_change_in_repository("draft.lkjc", &draft, &created.repository)
        .unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(!draft.semantic.changes.iter().any(|c| matches!(
        c,
        AuthoredChange::ReplaceFunctionBody { .. } | AuthoredChange::SetTest { .. }
    )));
    let result = created
        .repository
        .prepare_authored_change(&draft.semantic, draft.options);
    assert!(
        result.is_err(),
        "existing publication policy rejects no-op requests"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.code.contains("no_change")),
        "{errors:#?}"
    );
}

#[test]
fn native_target_runners_match_flat_intent_and_canonical_drafts() {
    // These runner kinds already exist in canonical meaning. Input contract 24 makes them
    // authorable and recoverable; accepting them must not change their typed representation.
    for (spelling, expected) in [
        ("command", RunnerKind::Command),
        ("batch", RunnerKind::Batch),
        ("worker", RunnerKind::Worker),
        ("test", RunnerKind::Test),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let header = format!("request base={}\n", created.current.head.revision);
        let flat = format!(
            r#"
create.module as=$m name=runner_example
expression.unit as=$body
create.function as=$f module=$m name=entry visibility=private result=unit effect=pure body=$body
create.component as=$c module=$m name=app visibility=private
type.function as=@entry result=unit
add.port as=$p component=$c name=main type=@entry function=$f
create.target as=$t name=example component=$c port=$p runner={spelling}
"#
        );
        let native = format!(
            r#"
declarations.begin
(units
  (module create runner_example (as $m)
    (function create entry (as $f) (visibility private)
      (returns Unit) (effect pure) (body (unit)))
    (component create app (as $c) (visibility private)
      (port create main (as $p) (type (function () Unit)) (function entry))))
  (target create example (as $t) (component runner_example::app)
    (port runner_example::app::main) (runner {spelling})))
declarations.end
"#
        );
        let decode = |body: &str| {
            decode_compact_change("runner.lkjc", format!("{header}{body}").as_bytes())
                .unwrap_or_else(|errors| panic!("{spelling}: {errors:#?}"))
        };
        let flat = decode(&flat);
        let native = decode(&native);
        assert_eq!(
            canonical_authored_intent_bytes(&flat.semantic).unwrap(),
            canonical_authored_intent_bytes(&native.semantic).unwrap()
        );
        assert!(native.semantic.changes.iter().any(|change| matches!(
            change,
            AuthoredChange::CreateTarget { runner, .. } if *runner == expected
        )));
        let prepared = created
            .repository
            .prepare_authored_change(&native.semantic, native.options)
            .unwrap();
        created.repository.publish(&prepared.publication).unwrap();
        let view = created.repository.view_current().unwrap();
        let draft = render_native_draft(
            &view,
            &[prepared.allocated["$t"]],
            4 * 1_048_576,
            crate::platform::execution::ExecutionControl::uncancelled(),
        )
        .unwrap();
        let decoded =
            decode_compact_change_in_repository("draft.lkjc", &draft, &created.repository).unwrap();
        let errors = created
            .repository
            .prepare_authored_change(&decoded.semantic, decoded.options)
            .unwrap_err();
        assert_eq!(errors.len(), 1, "{spelling}: {errors:#?}");
        assert_eq!(
            errors[0].code, "publication_semantic_no_change",
            "{spelling}"
        );
    }
}

#[test]
fn native_rejects_duplicate_names_and_unbound_lexical_locals() {
    let base = crate::platform::semantic_id::RevisionId::from_digest([1; 32]);
    for (source, code) in [
        (
            "(units (module create app (function create f (visibility public) (parameter misspelled value) (returns I64) (effect pure) (body (i64 1)))))",
            "change_unit_form",
        ),
        (
            "(units (module create app (in ignored)))",
            "change_unit_form",
        ),
        (
            "(units (module create same) (module create same))",
            "change_unit_duplicate",
        ),
        (
            "(units (module create app (type-alias Item I64) (record create Item (visibility public))))",
            "change_unit_duplicate",
        ),
        (
            "(units (module create app (function create f (visibility private) (type-parameter create T) (type-alias T I64) (returns T) (effect pure) (body (unit)))))",
            "change_unit_duplicate",
        ),
        (
            "(units (module create app (function create f (visibility private) (returns i64) (effect pure) (body (local escaped)))))",
            "change_block_local_unbound",
        ),
    ] {
        let input =
            format!("request base={base}\ndeclarations.begin\n{source}\ndeclarations.end\n");
        let errors = decode_compact_change("negative.lkjc", input.as_bytes()).unwrap_err();
        assert_eq!(errors[0].code, code);
        assert!(errors[0].location.is_some());
    }
}

#[test]
fn maintained_native_resource_library_retains_flat_review_identity() {
    let base = crate::platform::semantic_id::RevisionId::from_digest([7; 32]);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/lkjscript-dev/src/offline_packages");
    let flat = format!(
        "request base={base}\n{}{}",
        std::fs::read_to_string(root.join("requirements.producer.lkjc")).unwrap(),
        std::fs::read_to_string(root.join("requirements.resource-library.lkjc")).unwrap()
    );
    let native = format!(
        "request base={base}\n{}{}",
        std::fs::read_to_string(root.join("requirements.producer.structural.lkjc")).unwrap(),
        std::fs::read_to_string(root.join("requirements.resource-library.native.lkjc")).unwrap()
    );
    let flat = decode_compact_change("independent-flat.lkjc", flat.as_bytes()).unwrap();
    let native = decode_compact_change("maintained-native.lkjc", native.as_bytes())
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert_eq!(
        canonical_authored_intent_bytes(&flat.semantic).unwrap(),
        canonical_authored_intent_bytes(&native.semantic).unwrap()
    );
}

#[test]
fn draft_cancellation_is_read_only_and_complete() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let view = created.repository.view_current().unwrap();
    let selected: Vec<_> = initial
        .owners
        .keys()
        .filter(|o| matches!(o, OwnerKey::Module(_) | OwnerKey::Target(_)))
        .copied()
        .collect();
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    control.cancel();
    let error = render_native_draft(&view, &selected, 4 * 1_048_576, control).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
    let control = crate::platform::execution::ExecutionControl::cancel_after_checks(3);
    let error = render_native_draft(&view, &selected, 4 * 1_048_576, control).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}
