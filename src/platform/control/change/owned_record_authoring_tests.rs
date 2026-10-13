//! Exact compact edges and accepted graph identities are separate authoring oracles.
use super::*;
use crate::platform::kernel::{
    DeclarationPayload as D, ExpressionOperation as E, OwnerRecord as O,
};

#[test]
fn native_owned_record_matches_independent_compact_declaration_and_codec() {
    let header = format!("request base=rev_{}\n", "82".repeat(32));
    let flat = r#"create.module as=$module name=nominal
create.owned-record as=$packet module=$module name=Packet visibility=public
add.type-parameter as=$T declaration=$packet name=T constraint=owned
add.type-parameter as=$K declaration=$packet name=K constraint=transferable
type.parameter as=@T parameter=$T
type.parameter as=@K parameter=$K
add.field as=$payload record=$packet name=payload type=@T
add.field as=$label record=$packet name=label type=@K
"#;
    let native = r#"declarations.begin
(units (module create nominal (as $module)
  (owned-record create Packet (as $packet) (visibility public)
    (type-parameter create T (as $T) (constraint owned))
    (type-parameter create K (as $K) (constraint transferable))
    (field create payload (as $payload) (type T))
    (field create label (as $label) (type K)))))
declarations.end
"#;
    let decode = |body| {
        decode_compact_change("nominal-intent.lkjc", format!("{header}{body}").as_bytes()).unwrap()
    };
    let flat = decode(flat);
    let native = decode(native);
    let bytes = canonical_authored_intent_bytes(&native.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR39");
    assert_eq!(
        bytes,
        canonical_authored_intent_bytes(&flat.semantic).unwrap()
    );
    assert_eq!(native.request_commitment, flat.request_commitment);
    assert_eq!(
        commitment_codec_identity(&bytes),
        "lkjscript-authored-change-codec-39"
    );
    assert!(
        native
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::CreateOwnedRecord { .. }))
    );
}

const SOURCE: &str = r#"declarations.begin
(units (module create nominal (as $module)
  (owned-record create Packet (as $packet) (visibility public)
    (field create payload (as $payload) (type ByteBuffer))
    (field create stamp (as $stamp) (type I64)))
  (owned-record create Empty (as $empty) (visibility private))
  (function create make (as $make) (visibility public) (effect pure)
    (parameter create bytes (type ByteBuffer) (use consume))
    (returns Packet)
    (body (pack-owned (type Packet) (field stamp (i64 7)) (field payload (local bytes)))))
  (function create take (as $take) (visibility public) (effect pure)
    (parameter create packet (type Packet) (use consume))
    (returns I64)
    (body (unpack-owned (type Packet) (local packet)
      (field stamp (binding stamp (type I64)))
      (field payload (binding bytes (type ByteBuffer)))
      (in (local stamp)))))
  (function create inspect (as $inspect) (visibility public) (effect pure)
    (parameter create packet (type Packet) (use borrow))
    (returns I64)
    (body (borrow-owned-field (type Packet) (local packet)
      (field payload (binding bytes (type ByteBuffer))) (in (i64 3)))))
  (function create empty (visibility private) (effect pure) (returns Unit)
    (body (let (binding empty (type Empty) (pack-owned (type Empty)))
      (in (unpack-owned (type Empty) (local empty) (in (unit)))))))))
declarations.end
"#;

#[test]
fn nominal_owned_native_scopes_drafts_and_renames_preserve_exact_fields() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let invalid = SOURCE.replace(
        "(let (binding empty (type Empty) (pack-owned (type Empty)))\n      (in (unpack-owned (type Empty) (local empty) (in (unit)))))",
        "(unpack-owned (type Empty) (pack-owned (type Empty)) (in (unit)))",
    );
    assert_ne!(SOURCE, invalid);
    let invalid = decode_compact_change(
        "nominal-temporary-source.lkjc",
        format!("request base={}\n{invalid}", created.current.head.revision).as_bytes(),
    )
    .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&invalid.semantic, invalid.options)
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_buffer_ownership"
                && error.message.contains("exact local source")),
        "{errors:#?}"
    );
    let request = decode_compact_change(
        "nominal-create.lkjc",
        format!("request base={}\n{SOURCE}", created.current.head.revision).as_bytes(),
    )
    .unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let allocated = prepared.allocated.clone();
    created.repository.publish(&prepared.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let field = |name| match allocated[name] {
        OwnerKey::Field(id) => id,
        _ => panic!("field"),
    };
    let expression = |name| {
        let O::Declaration(record) = view.read_owner(allocated[name]).unwrap().value.unwrap()
        else {
            panic!("function")
        };
        let D::Function(function) = record.payload else {
            panic!("function")
        };
        let O::Expression(expression) = view
            .read_owner(OwnerKey::Expression(function.body))
            .unwrap()
            .value
            .unwrap()
        else {
            panic!("body")
        };
        expression.operation
    };
    let E::PackOwnedRecord { fields, .. } = expression("$make") else {
        panic!("pack")
    };
    assert_eq!(
        fields
            .iter()
            .map(|entry| entry.field.field)
            .collect::<Vec<_>>(),
        vec![field("$stamp"), field("$payload")]
    );
    let E::UnpackOwnedRecord { fields, .. } = expression("$take") else {
        panic!("unpack")
    };
    assert!(fields.windows(2).all(|pair| pair[0].field < pair[1].field));
    let E::BorrowOwnedRecordField {
        field: selected, ..
    } = expression("$inspect")
    else {
        panic!("borrow")
    };
    assert_eq!(selected.field, field("$payload"));
    let draft = render_native_draft(
        &view,
        &[allocated["$module"].into()],
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let text = std::str::from_utf8(&draft).unwrap();
    assert!(text.contains("(owned-record edit"), "{text}");
    assert!(text.contains(&field("$payload").to_string()), "{text}");
    let unchanged =
        decode_compact_change_in_repository("nominal-draft.lkjc", &draft, &created.repository)
            .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&unchanged.semantic, unchanged.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "publication_semantic_no_change");
    let rename = format!(
        "request base={}\nrename.owner owner={} name=storage\n",
        view.revision(),
        allocated["$payload"]
    );
    let rename = decode_compact_change("nominal-rename.lkjc", rename.as_bytes()).unwrap();
    let renamed = created
        .repository
        .prepare_authored_change(&rename.semantic, rename.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    created.repository.publish(&renamed.publication).unwrap();
    let renamed_view = created.repository.view_current().unwrap();
    for name in ["$make", "$take", "$inspect"] {
        assert_eq!(
            view.read_owner(allocated[name]).unwrap().value,
            renamed_view.read_owner(allocated[name]).unwrap().value
        );
    }
    let renamed_draft = render_native_draft(
        &renamed_view,
        &[allocated["$module"].into()],
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let unchanged = decode_compact_change_in_repository(
        "nominal-renamed-draft.lkjc",
        &renamed_draft,
        &created.repository,
    )
    .unwrap();
    let errors = created
        .repository
        .prepare_authored_change(&unchanged.semantic, unchanged.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "publication_semantic_no_change");

    let edited = std::str::from_utf8(&renamed_draft)
        .unwrap()
        .replace("(i64 7)", "(i64 8)")
        .replace("(i64 3)", "(i64 4)");
    let edited = decode_compact_change_in_repository(
        "nominal-literal-edit.lkjc",
        edited.as_bytes(),
        &created.repository,
    )
    .unwrap();
    assert_eq!(
        edited
            .semantic
            .changes
            .iter()
            .filter(|change| matches!(change, AuthoredChange::SetFunctionLiterals { .. }))
            .count(),
        2
    );
    assert!(
        !edited
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::ReplaceFunctionBody { .. }))
    );
    let edited = created
        .repository
        .prepare_authored_change(&edited.semantic, edited.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    created.repository.publish(&edited.publication).unwrap();
    let edited_view = created.repository.view_current().unwrap();
    for name in ["$make", "$take", "$inspect"] {
        assert_eq!(
            renamed_view.read_owner(allocated[name]).unwrap().value,
            edited_view.read_owner(allocated[name]).unwrap().value
        );
    }
}

#[test]
fn legacy_parameter_migration_promotes_complete_signature_without_rewriting_body() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("origin"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let source = r#"declarations.begin
(units (module create legacy (as $module)
  (function create discard (as $discard) (visibility private) (effect pure)
    (parameter create queue (as $consumed) (type (owned-product (field payload ByteBuffer))) (use consume))
    (returns Unit) (body (unit)))
  (function create read (as $read) (visibility private) (effect pure)
    (parameter create queue (as $borrowed) (type (owned-product (field payload ByteBuffer))) (use borrow))
    (returns I64) (body (i64 0)))))
declarations.end
"#;
    let source = decode_compact_change(
        "legacy-origin.lkjc",
        format!("request base={}\n{source}", created.current.head.revision).as_bytes(),
    )
    .unwrap();
    let source = created
        .repository
        .prepare_authored_change(&source.semantic, source.options)
        .unwrap();
    let allocated = source.allocated.clone();
    created.repository.publish(&source.publication).unwrap();
    let mut historical = created
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    for name in ["$discard", "$read", "$consumed", "$borrowed"] {
        historical
            .owners
            .get_mut(&allocated[name])
            .unwrap()
            .set_encoding_for_edit(31);
    }
    let legacy =
        GraphRepository::create(&temporary.path().join("legacy"), &historical, None).unwrap();
    let before = legacy.repository.view_current().unwrap();
    let unchanged = format!(
        "request base={}\ntype.owned-product as=@Carrier\ntype.field parent=@Carrier index=0 name=payload type=byte-buffer\nset.parameter-type parameter={} type=@Carrier\nset.parameter-type parameter={} type=@Carrier\n",
        before.revision(),
        allocated["$consumed"],
        allocated["$borrowed"]
    );
    let unchanged = decode_compact_change("legacy-noop.lkjc", unchanged.as_bytes()).unwrap();
    let errors = legacy
        .repository
        .prepare_authored_change(&unchanged.semantic, unchanged.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "publication_semantic_no_change");

    let changed = format!(
        "request base={}\ncreate.owned-record as=$queue module={} name=Queue visibility=private\nadd.field as=$storage record=$queue name=payload type=byte-buffer\ntype.named as=@Queue declaration=$queue\nset.parameter-type parameter={} type=@Queue\nset.parameter-type parameter={} type=@Queue\n",
        before.revision(),
        allocated["$module"],
        allocated["$consumed"],
        allocated["$borrowed"]
    );
    let changed = decode_compact_change("legacy-nominal.lkjc", changed.as_bytes()).unwrap();
    let changed = legacy
        .repository
        .prepare_authored_change(&changed.semantic, changed.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    legacy.repository.publish(&changed.publication).unwrap();
    let after = legacy.repository.view_current().unwrap();
    for name in ["$discard", "$read"] {
        let O::Declaration(previous) = before.read_owner(allocated[name]).unwrap().value.unwrap()
        else {
            panic!("previous function")
        };
        let O::Declaration(current) = after.read_owner(allocated[name]).unwrap().value.unwrap()
        else {
            panic!("current function")
        };
        assert_eq!(
            previous.payload, current.payload,
            "signature promotion preserves the result, body and parameter identities"
        );
        assert_eq!(previous.header.contract_version, 31);
        assert_eq!(
            current.header.contract_version,
            crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        );
    }
}

#[test]
fn generic_nominal_drafts_resolve_typed_field_aliases_without_selecting_the_record() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let source = r#"declarations.begin
(units (module create generic (as $module)
  (owned-record create Packet (visibility private)
    (type-parameter create T (constraint owned))
    (field create payload (type T)))
  (function create make (as $make) (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use consume))
    (returns (Packet T))
    (body (pack-owned (type (Packet T)) (field payload (local value)))))
  (function create take (as $take) (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create packet (type (Packet T)) (use consume))
    (returns T)
    (body (unpack-owned (type (Packet T)) (local packet)
      (field payload (binding value (type T))) (in (local value)))))
  (function create inspect (as $inspect) (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create packet (type (Packet T)) (use borrow))
    (returns I64)
    (body (borrow-owned-field (type (Packet T)) (local packet)
      (field payload (binding value (type T))) (in (i64 9)))))))
declarations.end
"#;
    let source = decode_compact_change(
        "generic-nominal-create.lkjc",
        format!("request base={}\n{source}", created.current.head.revision).as_bytes(),
    )
    .unwrap();
    let source = created
        .repository
        .prepare_authored_change(&source.semantic, source.options)
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let allocated = source.allocated.clone();
    created.repository.publish(&source.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let selected = [
        allocated["$make"].into(),
        allocated["$take"].into(),
        allocated["$inspect"].into(),
    ];
    let draft = render_native_draft(
        &view,
        &selected,
        4 * 1_048_576,
        crate::platform::execution::ExecutionControl::uncancelled(),
    )
    .unwrap();
    let draft = std::str::from_utf8(&draft).unwrap();
    assert!(!draft.contains("(owned-record edit"), "{draft}");
    assert!(draft.contains("(reference ref_payload_"), "{draft}");
    assert!(draft.contains("(type-alias type_"), "{draft}");
    let renamed = draft
        .replace("ref_payload_", "exact_payload_")
        .replace("ref_Packet_", "exact_Packet_")
        .replace("type_", "applied_type_");
    assert_ne!(draft, renamed);
    for source in [draft, renamed.as_str()] {
        let request = decode_compact_change_in_repository(
            "generic-nominal-draft.lkjc",
            source.as_bytes(),
            &created.repository,
        )
        .unwrap_or_else(|error| panic!("{error:#?}\n{source}"));
        let errors = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert_eq!(errors[0].code, "publication_semantic_no_change");
    }
}
