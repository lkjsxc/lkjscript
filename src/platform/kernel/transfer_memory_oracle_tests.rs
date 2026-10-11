//! Independent symbolic ownership proof, including uncalled imported templates.
use super::*;
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{DeclarationId, RepositoryId};

const WORKERS: &str = r#"declarations.begin
(units (module create workers
  (function create data (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns T) (body (local value)))
  (function create owner (visibility public) (effect (task))
    (type-parameter create O (constraint owned transferable))
    (parameter create value (type O) (use consume)) (returns O) (body (local value)))
  (function create unused (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns Unit) (body (unit)))))
declarations.end
"#;

const GROUP: &str = r#"declarations.begin
(units (module create caller
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (type-parameter create O (constraint owned transferable))
    (parameter create data (type T))
    (parameter create value (type O) (use consume))
    (returns (owned-product (field left O) (field right T)))
    (body (parallel (call workers::owner (types O) (local value))
                    (call workers::data (types T) (local data)))))))
declarations.end
"#;

fn author(source: &str) -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source).unwrap()
}

fn named(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(d)) if d.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn empty(seed: &[u8]) -> KernelSnapshot {
    let map = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("transfer_oracle").unwrap(),
            owners: map,
            dependencies: map,
            retirements: map,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn publish(repository: &GraphRepository, source: &str) {
    let input = format!(
        "request base={}\n{source}",
        repository.view_current().unwrap().revision()
    );
    let request =
        crate::platform::control::decode_compact_change("transfer-oracle", input.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
}

fn imported() -> KernelSnapshot {
    imported_group(GROUP)
}

fn imported_group(group: &str) -> KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let workers = GraphRepository::create(
        &temporary.path().join("workers"),
        &empty(b"transfer-workers"),
        None,
    )
    .unwrap()
    .repository;
    publish(&workers, WORKERS);
    let exported = workers.export_package_transport().unwrap();
    let caller = GraphRepository::create(
        &temporary.path().join("caller"),
        &empty(b"transfer-caller"),
        None,
    )
    .unwrap()
    .repository;
    caller
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    publish(
        &caller,
        &format!(
            "add.dependency package={} semantic-revision={} package-revision={}\ndeclarations.begin\n(units (use workers {} {}))\ndeclarations.end\n{group}",
            exported.revision.package,
            exported.revision.revision.revision_id().unwrap(),
            exported.revision_digest,
            exported.revision.package,
            exported.revision_digest,
        ),
    );
    caller
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

#[test]
fn transfer_memory_oracle_accepts_generic_only_group_and_exact_imported_bounds() {
    for source in [author(&format!("{WORKERS}{GROUP}")), imported()] {
        assert!(accepts(&source));
        for constraint in [
            TypeParameterConstraints::Transferable,
            TypeParameterConstraints::CaptureSafeTransferable,
        ] {
            let mut changed = source.clone();
            let joined = named(&changed, "joined");
            for owner in changed.owners.values_mut() {
                if let OwnerRecord::TypeParameter(p) = owner
                    && p.declaration == joined
                    && p.name.as_str() == "T"
                {
                    p.constraints = constraint;
                }
            }
            assert!(
                accepts(&changed),
                "ordinary transfer also proves capture safety: {constraint:?}"
            );
        }
    }
}

#[test]
fn transfer_memory_oracle_rejects_weakened_uncalled_template_and_foreign_scope() {
    let source = imported();
    assert!(accepts(&source));
    let joined = named(&source, "joined");
    for (name, constraint) in [
        ("T", TypeParameterConstraints::None),
        ("T", TypeParameterConstraints::CaptureSafe),
        ("O", TypeParameterConstraints::Owned),
    ] {
        let mut changed = source.clone();
        for owner in changed.owners.values_mut() {
            if let OwnerRecord::TypeParameter(p) = owner
                && p.declaration == joined
                && p.name.as_str() == name
            {
                p.constraints = constraint;
            }
        }
        assert!(
            !accepts(&changed),
            "uncalled {name} has no transferable proof"
        );
    }
    let mut changed = source.clone();
    let foreign = changed
        .dependency_interfaces
        .values()
        .flat_map(|i| i.values())
        .find_map(|o| match o {
            PackageInterfaceRecord::TypeParameter(p) if p.name.as_str() == "T" => {
                Some(p.declaration)
            }
            _ => None,
        })
        .unwrap();
    for owner in changed.owners.values_mut() {
        if let OwnerRecord::TypeParameter(p) = owner
            && p.declaration == joined
            && p.name.as_str() == "T"
        {
            p.declaration = foreign;
        }
    }
    assert!(
        !accepts(&changed),
        "imported formals cannot grant caller scope"
    );
}

#[test]
fn transfer_memory_oracle_checks_unused_imported_constraints_and_generation() {
    let source = imported();
    assert!(accepts(&source));
    // This export has no caller. Its bound still requires an exact current
    // graph-function formal; no live root call serves as the rejection oracle.
    for fault in ["predecessor", "foreign-owner", "removed-formal"] {
        let mut changed = source.clone();
        for interface in changed.dependency_interfaces.values_mut() {
            let interface = std::sync::Arc::make_mut(interface);
            let (id, f) = interface
                .iter()
                .find_map(|(key, owner)| match (key, owner) {
                    (OwnerKey::Declaration(id), PackageInterfaceRecord::Declaration(d))
                        if d.name.as_str() == "unused" =>
                    {
                        let PackageInterfaceDeclarationPayload::Function(f) = &d.payload else {
                            unreachable!()
                        };
                        Some((*id, f.clone()))
                    }
                    _ => None,
                })
                .unwrap();
            if fault == "removed-formal" {
                let PackageInterfaceRecord::Declaration(d) =
                    interface.get_mut(&OwnerKey::Declaration(id)).unwrap()
                else {
                    unreachable!()
                };
                let PackageInterfaceDeclarationPayload::Function(f) = &mut d.payload else {
                    unreachable!()
                };
                f.type_parameters.clear();
            } else {
                let PackageInterfaceRecord::TypeParameter(p) = interface
                    .get_mut(&OwnerKey::TypeParameter(f.type_parameters[0]))
                    .unwrap()
                else {
                    unreachable!()
                };
                if fault == "predecessor" {
                    p.header.contract_version = 21;
                } else {
                    p.declaration = named(&source, "joined");
                }
            }
        }
        assert!(
            !accepts(&changed),
            "{fault} must not reinterpret imported bounds"
        );
    }
    for fault in ["root", "owner"] {
        let mut changed = source.clone();
        if fault == "root" {
            changed.root.graph_contract_version = 21;
        } else {
            for owner in changed.owners.values_mut() {
                if let OwnerRecord::TypeParameter(p) = owner
                    && p.constraints.requires_transfer()
                {
                    p.header.contract_version = 21;
                }
            }
        }
        assert!(
            !accepts(&changed),
            "{fault} generation cannot acquire transfer meaning"
        );
    }
}

#[test]
fn transfer_memory_oracle_checks_group_obligations_in_untaken_syntax() {
    let group = r#"declarations.begin
(units (module create untaken
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (type-parameter create O (constraint owned transferable))
    (parameter create data (type T))
    (parameter create value (type O) (use consume)) (returns Unit)
    (body (if (bool true) (unit)
      (sequence (parallel (call workers::owner (types O) (local value))
                          (call workers::data (types T) (local data))) (unit)))))))
declarations.end
"#;
    let source = author(&format!("{WORKERS}{group}"));
    assert!(accepts(&source));
    let joined = named(&source, "joined");
    for constraint in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::CaptureSafe,
    ] {
        let mut changed = source.clone();
        for owner in changed.owners.values_mut() {
            if let OwnerRecord::TypeParameter(p) = owner
                && p.declaration == joined
                && p.name.as_str() == "T"
            {
                p.constraints = constraint;
            }
        }
        assert!(
            !accepts(&changed),
            "an untaken group still needs an ordinary transfer bound"
        );
    }
}

#[test]
fn transfer_memory_oracle_admits_forwarded_witnesses_and_rejects_foreign_operand_scope() {
    let group = r#"declarations.begin
(units (module create witness-caller
  (function create joined (visibility public) (effect (task))
    (type-parameter create O (constraint owned transferable))
    (implementation-parameter implparam_97000000000000000000000000000001 left-ops abstraction::Storage O)
    (implementation-parameter implparam_97000000000000000000000000000002 right-ops abstraction::Storage O)
    (parameter create left (type O) (use consume))
    (parameter create right (type O) (use consume))
    (returns (owned-product (field left O) (field right O)))
    (body (parallel
      (implementation-call generic-workers::forward (types O)
        (implementations parameter@joined@implparam_97000000000000000000000000000001) (i64 1) (local left))
      (implementation-call generic-workers::forward (types O)
        (implementations parameter@joined@implparam_97000000000000000000000000000002) (i64 2) (local right)))))))
declarations.end
"#;
    let source = author(&format!(
        "{}{}{}",
        include_str!("../../../tests/fixtures/owned-witness-library.lkjc"),
        include_str!("../../../tests/fixtures/parallel-generic-workers.lkjc"),
        group
    ));
    assert!(accepts(&source));
    let joined = named(&source, "joined");
    let foreign = named(&source, "forward");
    let mut changed = source.clone();
    for owner in changed.owners.values_mut() {
        if let OwnerRecord::Expression(e) = owner
            && let ExpressionOperation::ImplementationCall {
                implementations, ..
            } = &mut e.operation
        {
            for operand in implementations {
                if let ImplementationOperand::Parameter { scope, .. } = operand
                    && scope.declaration == joined
                {
                    scope.declaration = foreign;
                }
            }
        }
    }
    assert!(
        !accepts(&changed),
        "compatible Self cannot substitute a foreign witness parameter"
    );

    for requirement in [false, true] {
        let mut changed = source.clone();
        for owner in changed.owners.values_mut() {
            if let OwnerRecord::Expression(e) = owner
                && let ExpressionOperation::ImplementationCall {
                    effect_arguments,
                    requirement_arguments,
                    ..
                } = &mut e.operation
            {
                if requirement {
                    requirement_arguments.push(
                        RequirementReference {
                            package: changed.root.package_id,
                            requirement: crate::platform::semantic_id::RequirementId::migrate(
                                b"parallel-forbidden-requirement-operand",
                                0,
                            ),
                        }
                        .into(),
                    );
                } else {
                    effect_arguments.push(EffectRow::default());
                }
            }
        }
        assert!(
            !accepts(&changed),
            "parallel witness calls still forbid explicit authority operands: requirement={requirement}"
        );
    }
}

#[test]
fn transfer_memory_oracle_rejects_hidden_authority_in_phantom_aggregate_metadata() {
    let literal = r#"declarations.begin
(units (module create phantom
  (record create Marker (visibility public)
    (type-parameter create P) (field create tag (type I64)))
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (type-parameter create O (constraint owned transferable))
    (parameter create data (type (Marker T)))
    (parameter create value (type O) (use consume))
    (returns (owned-product (field left O) (field right (Marker T))))
    (body (parallel (call workers::owner (types O) (local value))
                    (call workers::data (types (Marker T)) (local data)))))))
declarations.end
"#;
    let source = author(&format!("{WORKERS}{literal}"));
    assert!(accepts(&source));
    let mut changed = source.clone();
    let object = TypeObject::new(TypeForm::Secret).unwrap();
    let secret = encode_type_object(&object).unwrap().0;
    changed.types.insert(secret, object);
    for object in changed.types.values_mut() {
        if let TypeForm::Applied { arguments, .. } = &mut object.form {
            *arguments = vec![secret];
        }
    }
    assert!(
        !accepts(&changed),
        "a phantom actual is independently checked even when no field uses it"
    );
}

#[test]
fn transfer_memory_oracle_checks_all_generic_owned_choice_alternatives() {
    let choice = "(owned-choice (case accepted T) (case returned O))";
    let group = GROUP
        .replace(
            "(parameter create value (type O) (use consume))",
            &format!("(parameter create value (type {choice}) (use consume))"),
        )
        .replace("(field left O)", &format!("(field left {choice})"))
        .replace("(types O)", &format!("(types {choice})"));
    let source = author(&format!("{WORKERS}{group}"));
    assert!(accepts(&source));
    let joined = named(&source, "joined");
    let mut weakened = source.clone();
    for owner in weakened.owners.values_mut() {
        if let OwnerRecord::TypeParameter(p) = owner
            && p.declaration == joined
            && p.name.as_str() == "O"
        {
            p.constraints = TypeParameterConstraints::Owned;
        }
    }
    assert!(
        !accepts(&weakened),
        "a nested owned alternative needs its own transferable obligation"
    );
    let mut hidden = source;
    let object = TypeObject::new(TypeForm::Secret).unwrap();
    let secret = encode_type_object(&object).unwrap().0;
    hidden.types.insert(secret, object);
    let mut replaced = false;
    for object in hidden.types.values_mut() {
        if let TypeForm::OwnedChoice { cases } = &mut object.form {
            cases[0].ty = secret;
            replaced = true;
        }
    }
    assert!(replaced);
    assert!(
        !accepts(&hidden),
        "ordinary authority cannot hide in an unselected choice case"
    );
}

#[test]
fn transfer_memory_oracle_checks_imported_sequence_elements_and_inactive_choices() {
    let sequence = "(owned-sequence (owned-choice (case ready ByteBuffer) (case pending O)))";
    let group = GROUP
        .replace(
            "(parameter create value (type O) (use consume))",
            &format!("(parameter create value (type {sequence}) (use consume))"),
        )
        .replace("(field left O)", &format!("(field left {sequence})"))
        .replace("(types O)", &format!("(types {sequence})"));
    for source in [author(&format!("{WORKERS}{group}")), imported_group(&group)] {
        assert!(accepts(&source));
        let joined = named(&source, "joined");
        let mut weakened = source.clone();
        for owner in weakened.owners.values_mut() {
            if let OwnerRecord::TypeParameter(p) = owner
                && p.declaration == joined
                && p.name.as_str() == "O"
            {
                p.constraints = TypeParameterConstraints::Owned;
            }
        }
        assert!(
            !accepts(&weakened),
            "all possible sequence elements require their transferable proof"
        );
        for (form, admitted) in [(TypeForm::Secret, false), (TypeForm::I64, true)] {
            let mut changed = source.clone();
            let object = TypeObject::new(form).unwrap();
            let item = encode_type_object(&object).unwrap().0;
            changed.types.insert(item, object);
            let mut replaced = false;
            for object in changed.types.values_mut() {
                if let TypeForm::OwnedSequence { item: existing } = &mut object.form {
                    *existing = item;
                    replaced = true;
                }
            }
            assert!(replaced);
            assert_eq!(
                accepts(&changed),
                admitted,
                "ordinary data is eligible while secret authority is rejected by every sequence element closure"
            );
        }
    }
    let ordinary = GROUP
        .replace(
            "(parameter create value (type O) (use consume))",
            "(parameter create value (type (owned-sequence I64)) (use consume))",
        )
        .replace("(field left O)", "(field left (owned-sequence I64))")
        .replace("(types O)", "(types (owned-sequence I64))");
    for source in [
        author(&format!("{WORKERS}{ordinary}")),
        imported_group(&ordinary),
    ] {
        assert!(
            accepts(&source),
            "fresh local and imported ordinary sequence transfer retains its owner"
        );
    }
}

#[test]
fn transfer_memory_oracle_rejects_sequences_in_unused_transfer_type_arguments() {
    let source = r#"declarations.begin
(units (module create unused-sequence
  (function create ignore (visibility private) (effect pure)
    (type-parameter create T (constraint transferable))
    (returns Unit) (body (unit)))
  (function create invoke (visibility public) (effect pure)
    (returns Unit) (body (call ignore (types I64))))))
declarations.end
"#;
    let original = author(source);
    assert!(accepts(&original));
    for item_form in [TypeForm::OwnedI64Cell, TypeForm::I64] {
        let mut snapshot = original.clone();
        let object = TypeObject::new(item_form).unwrap();
        let item = encode_type_object(&object).unwrap().0;
        snapshot.types.insert(item, object);
        let object = TypeObject::new(TypeForm::OwnedSequence { item }).unwrap();
        let sequence = encode_type_object(&object).unwrap().0;
        snapshot.types.insert(sequence, object);
        let mut changed = false;
        for owner in snapshot.owners.values_mut() {
            if let OwnerRecord::Expression(e) = owner
                && let ExpressionOperation::Call { type_arguments, .. } = &mut e.operation
            {
                *type_arguments = vec![sequence];
                changed = true;
            }
        }
        assert!(changed);
        assert!(
            !accepts(&snapshot),
            "an unused ordinary formal cannot admit a sequence token"
        );
        assert!(validate_full(&snapshot).is_err());
    }
}
