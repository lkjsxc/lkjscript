//! Literal compact intent and canonical graph observations are independent authoring oracles.
use super::*;
use crate::platform::change::{CanonicalBaseRead, canonical_authored_intent_bytes};
use crate::platform::publication::GraphRepository;

#[test]
fn parameterized_owned_native_intent_matches_independent_compact_edges() {
    let base = format!("rev_{}", "82".repeat(32));
    let function = format!("pkg_{}/decl_{}", "31".repeat(16), "32".repeat(16));
    let flat = format!(
        r#"request base={base}
create.module as=$module name=parameterized
type.parameter as=@self parameter=$self
type.parameter as=@item parameter=$item
type.owned-sequence as=@sequence item=@item
create.owned-contract as=$contract module=$module name=Storage visibility=public self=@self
owned.contract-parameter parent=$contract index=0 parameter=$item
owned.method parent=$contract index=0 as=%method id=method_85000000000000000000000000000003 name=produce result=@sequence
owned.parameter parent=%method index=0 type=@self use=consume
add.type-parameter as=$self declaration=$contract name=Self constraint=owned
add.type-parameter as=$item declaration=$contract name=Item constraint=owned
create.owned-implementation as=$implementation module=$module name=Cells visibility=public contract=$contract self=owned-i64-cell
owned.type-argument parent=$implementation index=0 type=byte-buffer
owned.mapping parent=$implementation index=0 method=method_85000000000000000000000000000003 function={function}
"#
    );
    let native = format!(
        r#"request base={base}
declarations.begin
(units (module create parameterized (as $module)
  (owned-contract create Storage (as $contract) (visibility public)
    (self Self)
    (type-parameter create Self (as $self) (constraint owned))
    (type-parameter create Item (as $item) (constraint owned))
    (method method_85000000000000000000000000000003 produce
      (parameters (Self consume)) (returns (owned-sequence Item))))
  (owned-implementation create Cells (as $implementation) (visibility public)
    (contract Storage) (self OwnedI64Cell) (types ByteBuffer)
    (method method_85000000000000000000000000000003 {function}))))
declarations.end
"#
    );
    let flat = decode_compact_change("parameterized-flat.lkjc", flat.as_bytes()).unwrap();
    let native = decode_compact_change("parameterized-native.lkjc", native.as_bytes()).unwrap();
    let bytes = canonical_authored_intent_bytes(&native.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR30");
    assert_eq!(
        bytes,
        canonical_authored_intent_bytes(&flat.semantic).unwrap()
    );
    assert_eq!(native.request_commitment, flat.request_commitment);
    assert_eq!(
        commitment_codec_identity(&bytes),
        "lkjscript-authored-change-codec-30"
    );
}

#[test]
fn parameterized_owned_witness_edges_bind_order_and_reject_malformed_children() {
    let declaration = format!("decl_{}", "44".repeat(16));
    let input = format!(
        "request base=rev_{}\nset.implementations as=%set declaration={declaration}\nowned.witness parent=%set index=0 as=%witness id=implparam_85000000000000000000000000000005 name=ops contract={declaration} self=owned-i64-cell\nowned.type-argument parent=%witness index=0 type=byte-buffer\nowned.type-argument parent=%witness index=1 type=owned-i64-cell\n",
        "82".repeat(32)
    );
    let original = decode_compact_change("witness-order.lkjc", input.as_bytes()).unwrap();
    let AuthoredChange::SetImplementationParameters { parameters, .. } =
        &original.semantic.changes[0]
    else {
        panic!("expected witnesses")
    };
    assert_eq!(
        parameters[0].type_arguments,
        vec![AuthoredType::ByteBuffer {}, AuthoredType::OwnedI64Cell {}]
    );
    assert_eq!(
        &canonical_authored_intent_bytes(&original.semantic).unwrap()[..8],
        b"LKJACR30"
    );
    let swapped = input
        .replace("index=0 type=byte-buffer", "index=1 type=byte-buffer")
        .replace("index=1 type=owned-i64-cell", "index=0 type=owned-i64-cell");
    let swapped = decode_compact_change("witness-swap.lkjc", swapped.as_bytes()).unwrap();
    assert_ne!(original.request_commitment, swapped.request_commitment);
    for invalid in [
        input.replace("index=1 type=owned-i64-cell", "index=2 type=owned-i64-cell"),
        input.replace("index=1 type=owned-i64-cell", "index=0 type=owned-i64-cell"),
        input.replace(
            "owned.type-argument parent=%witness",
            "owned.type-argument parent=%unknown",
        ),
        input.replace(" as=%witness", ""),
        format!(
            "{input}owned.witness parent=%set index=1 as=%witness id=implparam_85000000000000000000000000000006 name=second contract={declaration} self=owned-i64-cell\n"
        ),
    ] {
        assert!(
            decode_compact_change("invalid-witness.lkjc", invalid.as_bytes()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn parameterized_owned_drafts_preserve_applications_and_exact_owner_identity() {
    use crate::platform::kernel::{DeclarationPayload as D, OwnerRecord as O};
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let source = r#"declarations.begin
(units (module create parameterized (as $module)
  (owned-contract create Marker (as $contract) (visibility public)
    (self Self)
    (type-parameter create Self (constraint owned))
    (type-parameter create First (constraint owned))
    (type-parameter create Second (constraint owned))
    (method method_85000000000000000000000000000004 identity
      (parameters (Self consume)) (returns Self)))
  (function create identity (visibility public) (effect pure)
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (body (local owner)))
  (owned-implementation create Cells (as $implementation) (visibility public)
    (contract Marker) (self OwnedI64Cell) (types ByteBuffer OwnedI64Cell)
    (method method_85000000000000000000000000000004 identity))
  (function create generic (as $generic) (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (implementation-parameter implparam_85000000000000000000000000000003 ops Marker T (types A B))
    (parameter create owner (type T) (use consume))
    (returns T) (body (sequence (i64 7) (local owner))))))
declarations.end"#;
    let input = format!("request base={}\n{source}\n", created.current.head.revision);
    let request = decode_compact_change("parameterized-create.lkjc", input.as_bytes()).unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let module = prepared.allocated["$module"];
    let implementation = prepared.allocated["$implementation"];
    let generic = prepared.allocated["$generic"];
    created.repository.publish(&prepared.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let before_implementation = view.read_owner(implementation).unwrap().value.unwrap();
    let before_generic = view.read_owner(generic).unwrap().value.unwrap();
    let O::Declaration(contract) = view
        .read_owner(prepared.allocated["$contract"])
        .unwrap()
        .value
        .unwrap()
    else {
        panic!("expected contract")
    };
    let D::OwnedContract(contract) = contract.payload else {
        panic!("expected contract")
    };
    let draft = render_native_draft(
        &view,
        &[module.into()],
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let text = std::str::from_utf8(&draft).unwrap();
    assert!(text.contains("(types ByteBuffer OwnedI64Cell)"), "{text}");
    assert!(
        text.contains("(implementation-parameter implparam_85000000000000000000000000000003"),
        "{text}"
    );
    let no_change = decode_compact_change_in_repository(
        "parameterized-draft.lkjc",
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
    let selected = text.replace("declarations.begin", "reference.owner as=$picked_module package=local class=module name=parameterized\nreference.owner as=$picked_contract package=local class=declaration name=Marker parent=$picked_module\nreference.owner as=$picked package=local class=type-parameter name=Self parent=$picked_contract\ntype.parameter as=@selected_self parameter=$picked\ndeclarations.begin").replace(&format!("(parameter-type {})", contract.self_parameter), "@selected_self");
    assert_ne!(selected, text);
    let selected = decode_compact_change_in_repository(
        "selected-self-draft.lkjc",
        selected.as_bytes(),
        &created.repository,
    )
    .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&selected.semantic, selected.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(
        errors[0].code, "publication_semantic_no_change",
        "{errors:#?}"
    );
    for types in [
        "(types I64 OwnedI64Cell)",
        "(types ByteBuffer)",
        "(types ByteBuffer OwnedI64Cell ByteBuffer)",
    ] {
        let invalid = text.replace("(types ByteBuffer OwnedI64Cell)", types);
        let invalid = decode_compact_change_in_repository(
            "invalid-parameterized-draft.lkjc",
            invalid.as_bytes(),
            &created.repository,
        )
        .unwrap();
        assert!(
            created
                .repository
                .prepare_authored_change(&invalid.semantic, invalid.options)
                .is_err(),
            "{types}"
        );
        assert_eq!(
            created.repository.view_current().unwrap().revision(),
            view.revision()
        );
    }
    let altered = text
        .replace(
            "(types ByteBuffer OwnedI64Cell)",
            "(types OwnedI64Cell ByteBuffer)",
        )
        .replace("(i64 7)", "(i64 8)");
    let changed = decode_compact_change_in_repository(
        "parameterized-edit.lkjc",
        altered.as_bytes(),
        &created.repository,
    )
    .unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&changed.semantic, changed.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert!(prepared.logical_plan.allocations.is_empty());
    assert!(prepared.logical_plan.retirements.is_empty());
    created.repository.publish(&prepared.publication).unwrap();
    let after = created.repository.view_current().unwrap();
    let O::Declaration(before) = before_implementation else {
        panic!("expected implementation")
    };
    let O::Declaration(current) = after.read_owner(implementation).unwrap().value.unwrap() else {
        panic!("expected implementation")
    };
    assert_eq!(before.header.owner, current.header.owner);
    let D::OwnedImplementation(before) = before.payload else {
        panic!("expected implementation")
    };
    let D::OwnedImplementation(current) = current.payload else {
        panic!("expected implementation")
    };
    assert_eq!(
        before.type_arguments,
        current.type_arguments.into_iter().rev().collect::<Vec<_>>()
    );
    let O::Declaration(before) = before_generic else {
        panic!("expected function")
    };
    let O::Declaration(current) = after.read_owner(generic).unwrap().value.unwrap() else {
        panic!("expected function")
    };
    let D::Function(before) = before.payload else {
        panic!("expected function")
    };
    let D::Function(current) = current.payload else {
        panic!("expected function")
    };
    assert_eq!(before.body, current.body);
    assert_eq!(
        before.implementation_parameters,
        current.implementation_parameters
    );
}

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
