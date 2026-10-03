//! Literal compact intent and canonical graph observations are independent authoring oracles.
use super::*;
use crate::platform::change::canonical_authored_intent_bytes;
use crate::platform::publication::GraphRepository;

#[test]
fn native_transfer_constraint_sets_are_canonical_and_select_only_codec26() {
    use crate::platform::kernel::TypeParameterConstraints as C;
    for (names, expected, magic) in [
        ("none", C::None, b"LKJACR14"),
        ("capture-safe", C::CaptureSafe, b"LKJACR14"),
        ("owned", C::Owned, b"LKJACR21"),
        ("transferable", C::Transferable, b"LKJACR26"),
        (
            "capture-safe transferable",
            C::CaptureSafeTransferable,
            b"LKJACR26",
        ),
        ("owned transferable", C::OwnedTransferable, b"LKJACR26"),
        (
            "transferable capture-safe",
            C::CaptureSafeTransferable,
            b"LKJACR26",
        ),
        ("transferable owned", C::OwnedTransferable, b"LKJACR26"),
    ] {
        let input = format!(
            "request base=rev_{}\ndeclarations.begin\n(units (module create constraints (function create f (visibility public) (type-parameter create T (constraint {names})) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n",
            "82".repeat(32)
        );
        let decoded = decode_compact_change("constraint-set.lkjc", input.as_bytes()).unwrap();
        assert!(decoded.semantic.changes.iter().any(|change| matches!(change,
            AuthoredChange::AddTypeParameter { parameter, .. } if parameter.constraints == expected)));
        let bytes = canonical_authored_intent_bytes(&decoded.semantic).unwrap();
        assert_eq!(&bytes[..8], magic, "{names}");
        let owner = OwnerKey::TypeParameter(
            crate::platform::semantic_id::TypeParameterId::migrate(b"constraint-set", 0),
        );
        let flat = format!(
            "request base=rev_{}\nset.type-parameter-constraint parameter={owner} constraint=\"{names}\"\n",
            "82".repeat(32)
        );
        let decoded = decode_compact_change("constraint-flat.lkjc", flat.as_bytes()).unwrap();
        assert!(
            matches!(&decoded.semantic.changes[0], AuthoredChange::SetTypeParameterConstraint { constraints, .. } if *constraints == expected)
        );
        assert_eq!(
            &canonical_authored_intent_bytes(&decoded.semantic).unwrap()[..8],
            magic
        );
    }
    for names in [
        "",
        "unknown",
        "transferable transferable",
        "none transferable",
        "owned capture-safe",
        "capture-safe owned",
        "owned transferable capture-safe",
    ] {
        let input = format!(
            "request base=rev_{}\ndeclarations.begin\n(units (module create constraints (function create f (visibility public) (type-parameter create T (constraint {names})) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n",
            "82".repeat(32)
        );
        assert!(
            decode_compact_change("invalid-constraint-set.lkjc", input.as_bytes()).is_err(),
            "{names}"
        );
    }
}

#[test]
fn transfer_constraints_round_trip_native_drafts_without_concrete_callers() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let input = format!("request base={}\ndeclarations.begin\n(units (module create transferable_bounds (as $module)
      (function create ordinary (visibility public) (type-parameter create T (constraint transferable)) (returns Unit) (effect pure) (body (unit)))
      (function create explicit_capture (visibility public) (type-parameter create T (constraint capture-safe transferable)) (returns Unit) (effect pure) (body (unit)))
      (function create owner (visibility public) (type-parameter create T (constraint owned transferable)) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n", created.current.head.revision);
    let decoded = decode_compact_change("bounds.lkjc", input.as_bytes()).unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    created.repository.publish(&prepared.publication).unwrap();
    let draft = render_native_draft(
        &created.repository.view_current().unwrap(),
        &[prepared.allocated["$module"].into()],
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let text = std::str::from_utf8(&draft).unwrap();
    for clause in [
        "(constraint transferable)",
        "(constraint capture-safe transferable)",
        "(constraint owned transferable)",
    ] {
        assert!(text.contains(clause), "{text}");
    }
    let decoded =
        decode_compact_change_in_repository("bounds-draft.lkjc", &draft, &created.repository)
            .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "publication_semantic_no_change");
}

#[test]
fn owned_effect_requirement_forwarding_round_trips_drafts_before_concrete_callers() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let standard = crate::platform::builtin_standard::BuiltinStandard::load().unwrap();
    created
        .repository
        .stage_package_transport(standard.package_transport, &standard.transport().container)
        .unwrap();
    let source = r#"declarations.begin
(units (use std builtin)
  (module create owned_authority (as $module)
    (owned-contract create Marker (visibility public)
      (self Self) (type-parameter create Self (constraint owned))
      (method method_85000000000000000000000000000001 read
        (parameters (Self borrow)) (returns I64) (effect pure)))
    (function create identity (visibility public)
      (type-parameter create T (constraint owned))
      (effect-parameter create E)
      (requirement-parameter create R (interface std::WallClock)
        (operations std::WallClock::utc-milliseconds))
      (implementation-parameter implparam_85000000000000000000000000000001 ops Marker T)
      (parameter create owner (type T) (use consume))
      (returns T) (effect (task (requirement R) (parameter E)))
      (body (local owner)))
    (function create forward (as $forward) (visibility public)
      (type-parameter create T (constraint owned))
      (effect-parameter create E)
      (requirement-parameter create R (interface std::WallClock)
        (operations std::WallClock::utc-milliseconds))
      (implementation-parameter implparam_85000000000000000000000000000002 ops Marker T)
      (parameter create owner (type T) (use consume))
      (returns T) (effect (task (requirement R) (parameter E)))
      (body (sequence (i64 7)
        (implementation-call identity (types T) (effects (row (parameter E)))
          (requirements R)
          (implementations parameter@forward@implparam_85000000000000000000000000000002)
          (local owner)))))))
declarations.end"#;
    let input = format!(
        "request base={}\n{source}\nadd.dependency package={} semantic-revision={} package-revision={}\n",
        created.current.head.revision,
        standard.package,
        standard.semantic_revision,
        standard.package_revision,
    );
    let decoded = decode_compact_change("owned-authority.lkjc", input.as_bytes()).unwrap();
    assert_eq!(
        &canonical_authored_intent_bytes(&decoded.semantic).unwrap()[..8],
        b"LKJACR27"
    );
    let prepared = created
        .repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap_or_else(|errors| panic!("generic library admission: {errors:#?}"));
    let module = prepared.allocated["$module"];
    created.repository.publish(&prepared.publication).unwrap();
    let draft = render_native_draft(
        &created.repository.view_current().unwrap(),
        &[module.into()],
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let text = std::str::from_utf8(&draft).unwrap();
    assert!(text.contains("(effects (row (parameter "), "{text}");
    assert!(text.contains("(requirements "), "{text}");
    assert!(text.contains("(implementations parameter@"), "{text}");
    let no_change = decode_compact_change_in_repository(
        "owned-authority-draft.lkjc",
        &draft,
        &created.repository,
    )
    .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&no_change.semantic, no_change.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "publication_semantic_no_change");

    let literal = text.replace("(i64 7)", "(i64 8)");
    let literal = decode_compact_change_in_repository(
        "authority-literal.lkjc",
        literal.as_bytes(),
        &created.repository,
    )
    .unwrap();
    assert!(
        literal
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::SetFunctionLiterals { .. }))
    );
    assert!(
        !literal
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::ReplaceFunctionBody { .. }))
    );
    created
        .repository
        .prepare_authored_change(&literal.semantic, literal.options)
        .unwrap();

    // Changing only the application's row retains the declared allowance but changes intent.
    let application = text.split_once("(effects ").unwrap().1;
    let end = application.find(" (requirements ").unwrap();
    let clause = format!("(effects {}", &application[..end]);
    let changed = text
        .replace(&clause, "(effects (row))")
        .replace("(i64 7)", "(i64 8)");
    assert_ne!(changed, text);
    let changed = decode_compact_change_in_repository(
        "authority-rebinding.lkjc",
        changed.as_bytes(),
        &created.repository,
    )
    .unwrap();
    assert!(
        !changed
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::SetFunctionLiterals { .. }))
    );
    assert!(
        changed
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::ReplaceFunctionBody { .. }))
    );
    created
        .repository
        .prepare_authored_change(&changed.semantic, changed.options)
        .unwrap();

    let application = text.split_once("(requirements ").unwrap().1;
    let end = application.find(')').unwrap();
    let clause = format!("(requirements {})", &application[..end]);
    let invalid = text.replace(&clause, "(requirements)");
    let invalid = decode_compact_change_in_repository(
        "authority-erased.lkjc",
        invalid.as_bytes(),
        &created.repository,
    )
    .unwrap();
    assert!(
        created
            .repository
            .prepare_authored_change(&invalid.semantic, invalid.options)
            .is_err()
    );
    assert_eq!(
        created
            .repository
            .view_current()
            .unwrap()
            .current()
            .head
            .revision,
        prepared.publication.head.revision
    );
}

#[test]
fn native_owned_task_method_intent_is_distinct_and_pure_predecessor_stays_stable() {
    let literal = "declarations.begin\n(units (module create methods
      (owned-contract create Storage (visibility public)
        (self Self) (type-parameter create Self (constraint owned))
        (method method_82000000000000000000000000000001 create
          (parameters (I64 unrestricted)) (returns Self)))))\ndeclarations.end\n";
    let encode = |input: &str| {
        let request = format!("request base=rev_{}\n{input}", "82".repeat(32));
        let decoded = decode_compact_change("method-codec.lkjc", request.as_bytes()).unwrap();
        canonical_authored_intent_bytes(&decoded.semantic).unwrap()
    };
    let implicit = encode(literal);
    let pure = encode(&literal.replace("(returns Self)", "(returns Self) (effect pure)"));
    let task = encode(&literal.replace("(returns Self)", "(returns Self) (effect (task))"));
    assert_eq!(implicit, pure);
    assert_eq!(&pure[..8], b"LKJACR21");
    assert_eq!(&task[..8], b"LKJACR24");
    assert_ne!(pure, task);
    for effect in ["(effect pure ignored)", "(effect)", "(other (task))"] {
        let invalid = literal.replace("(returns Self)", &format!("(returns Self) {effect}"));
        let request = format!("request base=rev_{}\n{invalid}", "82".repeat(32));
        assert!(decode_compact_change("invalid-method.lkjc", request.as_bytes()).is_err());
    }
}

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
        &[module.into()],
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
            &[prepared.allocated["$t"].into()],
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
        .map(NativeDraftSelection::from)
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
