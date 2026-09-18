use super::*;

#[test]
fn parameter_type_record_matches_existing_typed_operation_and_commitment() {
    let parameter = OwnerKey::Parameter(ParameterId::migrate(b"compact-parameter-type", 0));
    let base = RevisionId::from_digest([7; 32]);
    let input = format!(
        "request base={base} idempotency=input-evolution\nset.parameter-type parameter={parameter} type=f64\n"
    );
    let decoded = decode_compact_change("parameter.lkjc", input.as_bytes()).unwrap();
    let typed = normalize_change_request(
        AuthoredChangeSet {
            base,
            preconditions: Vec::new(),
            changes: vec![AuthoredChange::SetParameterType {
                parameter: OwnerSelector::Exact { owner: parameter },
                ty: AuthoredType::F64 {},
            }],
            budget: Default::default(),
        },
        PublicationOptions {
            idempotency_key: Some("input-evolution".to_owned()),
            intent: None,
        },
    )
    .unwrap();
    assert_eq!(decoded.semantic.changes, typed.semantic.changes);
    assert_eq!(
        crate::platform::change::canonical_authored_intent_bytes(&decoded.semantic).unwrap(),
        crate::platform::change::canonical_authored_intent_bytes(&typed.semantic).unwrap()
    );
    assert_eq!(decoded.request_commitment, typed.request_commitment);
    let bytes = crate::platform::change::canonical_authored_intent_bytes(&typed.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR17");
    // Frozen authored encoding: header, exact base, empty preconditions, one change, tag 25.
    assert_eq!(bytes[56], 25);

    let changed = input.replace("type=f64", "type=bool");
    let changed = decode_compact_change("parameter.lkjc", changed.as_bytes()).unwrap();
    assert_ne!(decoded.request_commitment, changed.request_commitment);
}

#[test]
fn parameter_type_record_fields_and_type_references_fail_at_the_new_record() {
    let parameter = OwnerKey::Parameter(ParameterId::migrate(b"compact-parameter-type", 0));
    for (record, code) in [
        (
            "set.parameter-type type=f64".to_owned(),
            "change_field_missing",
        ),
        (
            format!("set.parameter-type parameter={parameter}"),
            "change_field_missing",
        ),
        (
            format!("set.parameter-type parameter={parameter} type=f64 surprise=true"),
            "change_field_unknown",
        ),
        (
            format!("set.parameter-type parameter={parameter} type=f64 type=bool"),
            "control_duplicate_field",
        ),
        (
            format!("set.parameter-type parameter={parameter} type=@Absent"),
            "change_type_undefined",
        ),
        (
            "set.parameter-type parameter=not-an-owner type=f64".to_owned(),
            "change_field_value",
        ),
    ] {
        let input = format!(
            "request base={}\ncreate.module as=$unrelated name=unrelated\n{record}\n",
            RevisionId::from_digest([7; 32]),
        );
        let errors = decode_compact_change("parameter-errors.lkjc", input.as_bytes()).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, code, "{record}: {errors:?}");
        let location = errors[0].location.as_ref().unwrap();
        assert_eq!(location.path, "parameter-errors.lkjc");
        assert_eq!(location.line, 3, "{record}: {errors:?}");
    }
}

#[test]
fn parameter_type_record_uses_shared_foreign_reference_admission() {
    let package = PackageId::migrate(b"foreign-parameter-type", 0);
    let revision = PackageRevisionDigest::from_bytes([3; 32]);
    let input = format!(
        "request base={}\n\
         reference.package as=$library package={package} package-revision={revision}\n\
         reference.owner as=$function package=$library class=declaration name=energy\n\
         reference.owner as=$items package=$library class=parameter parent=$function name=items\n\
         set.parameter-type parameter=$items type=f64\n",
        RevisionId::from_digest([7; 32]),
    );
    let errors = decode_compact_change("foreign.lkjc", input.as_bytes()).unwrap_err();
    assert_eq!(errors[0].code, "change_reference_foreign_local");
    assert_eq!(errors[0].location.as_ref().unwrap().line, 5);
}

#[test]
fn parameter_type_exact_and_named_routes_have_the_typed_oracles_lowering_and_meaning() {
    use crate::platform::change::{CanonicalBaseRead, CanonicalDelta, lower_authored_changes};
    use crate::platform::kernel::{OwnerRecord, TypeForm};
    use crate::platform::publication::GraphRepository;

    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let initial = format!(
        "request base={}\n\
         add.parameter as=$input function=$function name=input type=unit\n\
         expression.unit as=$body\n\
         create.function as=$function module=first name=parameter_fixture visibility=private result=unit effect=pure body=$body\n",
        created.current.head.revision,
    );
    let initial = decode_compact_change("fixture.lkjc", initial.as_bytes()).unwrap();
    let prepared = created
        .repository
        .prepare_authored_change(&initial.semantic, initial.options)
        .unwrap();
    let parameter = prepared.allocated["$input"];
    created.repository.publish(&prepared.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let base = view.exact_revision().unwrap();
    let before = view.read_owner(parameter).unwrap().value.unwrap();
    let exact = format!("request base={base}\nset.parameter-type parameter={parameter} type=f64\n");
    let named = format!(
        "request base={base}\n\
         reference.owner as=$module package=local class=module name=first\n\
         reference.owner as=$function package=local class=declaration parent=$module name=parameter_fixture\n\
         reference.owner as=$input package=local class=parameter parent=$function name=input\n\
         set.parameter-type parameter=$input type=f64\n"
    );
    let exact = decode_compact_change("exact.lkjc", exact.as_bytes()).unwrap();
    let named = decode_compact_change("named.lkjc", named.as_bytes()).unwrap();
    let typed = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::SetParameterType {
            parameter: OwnerSelector::Exact { owner: parameter },
            ty: AuthoredType::F64 {},
        }],
        budget: Default::default(),
    };
    let oracle = lower_authored_changes(&view, &view, &typed).unwrap();
    let oracle = CanonicalDelta::normalize_from(&view, oracle.edits)
        .unwrap()
        .canonical;
    let oracle_prepared = view
        .prepare_authored_change(&typed, PublicationOptions::default())
        .unwrap();
    for request in [exact, named] {
        let lowered = lower_authored_changes(&view, &view, &request.semantic).unwrap();
        assert!(lowered.allocated.is_empty());
        let lowered = CanonicalDelta::normalize_from(&view, lowered.edits)
            .unwrap()
            .canonical;
        assert_eq!(lowered.owners, oracle.owners);
        assert_eq!(lowered.type_additions, oracle.type_additions);
        assert!(lowered.dependencies.is_empty());
        assert!(lowered.retirements.is_empty());
        let prepared = view
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        assert_eq!(
            prepared.publication.authority.semantic.digest,
            oracle_prepared.publication.authority.semantic.digest
        );
        assert!(prepared.allocated.is_empty());
    }
    created
        .repository
        .publish(&oracle_prepared.publication)
        .unwrap();
    let after_view = created.repository.view_current().unwrap();
    let OwnerRecord::Parameter(mut after) =
        after_view.read_owner(parameter).unwrap().value.unwrap()
    else {
        panic!("parameter keeps its owner kind");
    };
    let OwnerRecord::Parameter(before) = before else {
        panic!("fixture parameter");
    };
    assert_eq!(
        after_view
            .read_type_object(after.ty)
            .unwrap()
            .value
            .unwrap()
            .form,
        TypeForm::F64,
    );
    after.ty = before.ty;
    assert_eq!(after, before);
}

#[test]
fn parameter_type_semantic_target_errors_point_to_the_mutation_site() {
    use crate::platform::publication::GraphRepository;

    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let base = created.current.head.revision;
    let view = created.repository.view_current().unwrap();
    let module = OwnerKey::Module(ModuleId::migrate(b"graph-5-normalization-prototype", 0));
    let missing = OwnerKey::Parameter(ParameterId::migrate(b"missing-parameter-type", 0));
    for (selector, code) in [
        (module.to_string(), "change_mutation_owner_kind"),
        ("$module".to_owned(), "change_mutation_owner_kind"),
        (missing.to_string(), "change_authored_owner_missing"),
    ] {
        let input = format!(
            "request base={base}\n\
             reference.owner as=$module package=local class=module name=first\n\
             create.module as=$unrelated name=unrelated\n\
             set.parameter-type parameter={selector} type=f64\n\
             set.parameter-type parameter={selector} type=bool\n"
        );
        let request = decode_compact_change("target-errors.lkjc", input.as_bytes()).unwrap();
        let mut owners = BTreeMap::new();
        let mut errors = view
            .prepare_authored_change_with_source_owners(
                &request.semantic,
                request.options,
                Some(&mut owners),
            )
            .unwrap_err();
        for error in &mut errors {
            request.origins.locate(error, &owners);
        }
        let error = errors.iter().find(|error| error.code == code).unwrap();
        let location = error
            .location
            .as_ref()
            .unwrap_or_else(|| panic!("{selector}: {errors:?}"));
        assert_eq!(location.path, "target-errors.lkjc");
        assert_eq!(location.line, 4, "{errors:?}");
        assert_eq!(created.repository.current().unwrap().head.revision, base);
    }
}

#[test]
fn parameter_type_same_request_generic_uses_its_defining_scope_and_locates_failure() {
    use crate::platform::publication::GraphRepository;

    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let base = created.current.head.revision;
    let view = created.repository.view_current().unwrap();
    let request = |ty: &str| {
        let input = format!(
            "request base={base}\n\
             type.parameter as=@T parameter=$T\n\
             type.parameter as=@U parameter=$U\n\
             add.type-parameter as=$T declaration=$first name=T\n\
             add.type-parameter as=$U declaration=$last name=U\n\
             add.parameter as=$input function=$first name=input type=unit\n\
             expression.unit as=$first-body\n\
             create.function as=$first module=first name=generic_input visibility=private result=unit effect=pure body=$first-body\n\
             expression.unit as=$last-body\n\
             create.function as=$last module=first name=unrelated_generic visibility=private result=unit effect=pure body=$last-body\n\
             set.parameter-type parameter=$input type={ty}\n"
        );
        decode_compact_change("scope.lkjc", input.as_bytes()).unwrap()
    };
    let invalid = request("@U");
    let mut owners = BTreeMap::new();
    let mut errors = view
        .prepare_authored_change_with_source_owners(
            &invalid.semantic,
            invalid.options,
            Some(&mut owners),
        )
        .unwrap_err();
    for error in &mut errors {
        invalid.origins.locate(error, &owners);
    }
    let error = errors
        .iter()
        .find(|error| error.code == "kernel_type_parameter_scope")
        .unwrap();
    assert_eq!(error.location.as_ref().unwrap().line, 11, "{errors:?}");
    assert_eq!(created.repository.current().unwrap().head.revision, base);

    let valid = request("@T");
    let prepared = view
        .prepare_authored_change(&valid.semantic, valid.options)
        .unwrap();
    created.repository.publish(&prepared.publication).unwrap();
}
