use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::kernel::{
    OwnerKey, PackageId, PackageRevisionDigest, ParameterUse, TypeParameterConstraints,
};
use crate::platform::semantic_id::RevisionId;
use std::sync::Arc;

const SOURCE: &str = r#"declarations.begin
(units (module create supplier
  (owned-record create Hidden (visibility private)
    (type-parameter create T (constraint owned))
    (field create value (type T)))
  (function create read (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use borrow))
    (returns T (borrow-from value)) (body (local value)))))
declarations.end
"#;

fn consumer_of(supplier: &KernelSnapshot) -> KernelSnapshot {
    let mut consumer = supplier.clone();
    consumer.root.package_id = PackageId::migrate(b"reference-dependency-consumer", 0);
    consumer.owners.clear();
    consumer.types.clear();
    consumer.dependencies.clear();
    consumer.representation_dependencies.clear();
    consumer.dependency_interfaces.clear();
    consumer.dependency_types = supplier.types.clone();
    let selection = DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: supplier.root.package_id,
        semantic_revision: RevisionId::from_digest([61; 32]),
        package_revision: PackageRevisionDigest::from_bytes([62; 32]),
    };
    let mut interface = BTreeMap::new();
    for (key, owner) in &supplier.owners {
        if matches!(owner, OwnerRecord::Declaration(d) if d.visibility != DeclarationVisibility::Public
            && !matches!(d.payload, Source::Record { .. } | Source::OwnedRecord { .. } | Source::Variant { .. }))
        {
            continue;
        }
        if let Some(record) = PackageInterfaceRecord::project_public(owner).unwrap() {
            interface.insert(*key, record);
        }
    }
    consumer
        .dependency_interfaces
        .insert(selection.package_revision, Arc::new(interface));
    consumer.dependencies.insert(selection.package, selection);
    consumer
}

fn fixture() -> (KernelSnapshot, KernelSnapshot) {
    let supplier =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap();
    let consumer = consumer_of(&supplier);
    (supplier, consumer)
}

fn imported(consumer: &mut KernelSnapshot) -> &mut BTreeMap<OwnerKey, PackageInterfaceRecord> {
    Arc::make_mut(consumer.dependency_interfaces.values_mut().next().unwrap())
}

fn accept(supplier: &KernelSnapshot, consumer: &KernelSnapshot) {
    validate(
        &[supplier, consumer],
        &ExecutionControl::uncancelled(),
        &mut 0,
    )
    .unwrap();
    validate(
        &[consumer, supplier],
        &ExecutionControl::uncancelled(),
        &mut 0,
    )
    .unwrap();
}

fn reject(supplier: &KernelSnapshot, consumer: &KernelSnapshot) {
    let error = validate(
        &[supplier, consumer],
        &ExecutionControl::uncancelled(),
        &mut 0,
    )
    .unwrap_err();
    assert_eq!(error.code, "normalized_reference_inventory", "{error:?}");
}

#[test]
fn exact_supplier_binding_checks_generic_children_and_private_representation() {
    let (supplier, consumer) = fixture();
    accept(&supplier, &consumer);
    for fault in [
        "constraint",
        "parameter",
        "field",
        "visibility",
        "generation",
        "missing",
        "borrow-source",
    ] {
        let mut bad = consumer.clone();
        let records = imported(&mut bad);
        match fault {
            "constraint" => {
                let p = records
                    .values_mut()
                    .find_map(|r| match r {
                        PackageInterfaceRecord::TypeParameter(p) => Some(p),
                        _ => None,
                    })
                    .unwrap();
                p.constraints = TypeParameterConstraints::OwnedTransferable;
            }
            "parameter" => {
                let p = records
                    .values_mut()
                    .find_map(|r| match r {
                        PackageInterfaceRecord::Parameter(p) => Some(p),
                        _ => None,
                    })
                    .unwrap();
                p.use_mode = ParameterUse::Consume;
            }
            "field" => {
                let p = records
                    .values_mut()
                    .find_map(|r| match r {
                        PackageInterfaceRecord::Field(p) => Some(p),
                        _ => None,
                    })
                    .unwrap();
                p.ty = TypeObjectDigest::from_bytes([89; 32]);
            }
            "visibility" => {
                let p = records
                    .values_mut()
                    .find(|r| matches!(r, PackageInterfaceRecord::RepresentationDeclaration(_)))
                    .unwrap();
                let PackageInterfaceRecord::RepresentationDeclaration(d) = p else {
                    unreachable!()
                };
                *p = PackageInterfaceRecord::Declaration(d.clone());
            }
            "generation" => {
                let p = records
                    .values_mut()
                    .find_map(|r| match r {
                        PackageInterfaceRecord::TypeParameter(p) => Some(p),
                        _ => None,
                    })
                    .unwrap();
                p.header.contract_version = 31;
            }
            "missing" => {
                let key = *records.keys().next().unwrap();
                let mut changed_supplier = supplier.clone();
                changed_supplier.owners.remove(&key);
                reject(&changed_supplier, &bad);
                continue;
            }
            "borrow-source" => {
                let f = records
                    .values_mut()
                    .find_map(|r| match r {
                        PackageInterfaceRecord::Declaration(d) => match &mut d.payload {
                            Interface::Function(f) => Some(f),
                            _ => None,
                        },
                        _ => None,
                    })
                    .unwrap();
                f.result.relation = crate::platform::kernel::ResultRelation::Value;
            }
            _ => unreachable!(),
        }
        reject(&supplier, &bad);
    }
    let mut package_visible = supplier.clone();
    for owner in package_visible.owners.values_mut() {
        if let OwnerRecord::Declaration(d) = owner
            && d.visibility == DeclarationVisibility::Private
        {
            d.visibility = DeclarationVisibility::Package;
        }
    }
    accept(&package_visible, &consumer_of(&package_visible));
}

#[test]
fn exact_supplier_binding_checks_unused_associated_members_and_mappings() {
    let fixture = crate::platform::execution::normalized::reference_types::associated_preparation::test_support::make_fixture();
    let supplier = fixture.source;
    let consumer = consumer_of(&supplier);
    accept(&supplier, &consumer);
    for fault in ["member", "mapping", "formal-order"] {
        let mut bad = consumer.clone();
        let i = imported(&mut bad)
            .values_mut()
            .find_map(|r| match r {
                PackageInterfaceRecord::Declaration(d) => match &mut d.payload {
                    Interface::OwnedImplementation(i) if !i.associated_types.is_empty() => Some(i),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        match fault {
            "member" => {
                *i.associated_types[0].value.ty_mut() = TypeObjectDigest::from_bytes([87; 32])
            }
            "mapping" => i.methods[0].type_arguments.push(fixture.block),
            "formal-order" => i.type_parameters.push(i.type_parameters[0]),
            _ => unreachable!(),
        }
        reject(&supplier, &bad);
    }
}

#[test]
fn exact_supplier_binding_requires_consistent_authored_and_representation_selections() {
    let (supplier, consumer) = fixture();
    let package = supplier.root.package_id;
    let selected = consumer.dependencies[&package].clone();
    let mut representation = consumer.clone();
    representation.dependencies.clear();
    representation
        .representation_dependencies
        .insert(package, selected.clone());
    reject(&supplier, &representation);
    // A private representation selection may be reached through an authored
    // intermediary, without becoming an authored import of the outer consumer.
    let mut outer = consumer_of(&consumer);
    outer.root.package_id = PackageId::migrate(b"reference-dependency-outer", 0);
    let direct = outer.dependencies.values_mut().next().unwrap();
    let old_revision = direct.package_revision;
    direct.package_revision = PackageRevisionDigest::from_bytes([63; 32]);
    direct.semantic_revision = RevisionId::from_digest([64; 32]);
    let new_revision = direct.package_revision;
    let interface = outer.dependency_interfaces.remove(&old_revision).unwrap();
    outer.dependency_interfaces.insert(new_revision, interface);
    outer
        .representation_dependencies
        .insert(package, selected.clone());
    outer.dependency_interfaces.insert(
        selected.package_revision,
        consumer.dependency_interfaces[&selected.package_revision].clone(),
    );
    outer
        .dependency_types
        .clone_from(&consumer.dependency_types);
    validate(
        &[&outer, &supplier, &consumer],
        &ExecutionControl::uncancelled(),
        &mut 0,
    )
    .unwrap();
    for fault in [
        "semantic-revision",
        "package-revision",
        "key",
        "interface",
        "orphan",
        "type",
    ] {
        let mut bad = consumer.clone();
        let mut shadow = selected.clone();

        match fault {
            "semantic-revision" => shadow.semantic_revision = RevisionId::from_digest([98; 32]),
            "package-revision" => {
                shadow.package_revision = PackageRevisionDigest::from_bytes([99; 32])
            }
            "key" => shadow.package = PackageId::migrate(b"other-supplier", 0),
            "interface" => {
                bad.dependency_interfaces.clear();
            }
            "orphan" => {
                bad.dependency_interfaces.insert(
                    PackageRevisionDigest::from_bytes([99; 32]),
                    Arc::new(BTreeMap::new()),
                );
            }
            "type" => {
                bad.dependency_types.insert(
                    TypeObjectDigest::from_bytes([99; 32]),
                    TypeObject::new(TypeForm::I64).unwrap(),
                );
            }
            _ => unreachable!(),
        }
        bad.representation_dependencies.insert(package, shadow);
        reject(&supplier, &bad);
    }
    assert!(validate(&[&consumer], &ExecutionControl::uncancelled(), &mut 0).is_err());
}

#[test]
fn exact_supplier_binding_compares_and_charges_requirement_limits() {
    use crate::platform::kernel::{Name, ResourceLimit, ResourceUnit};
    let mut supplier = crate::platform::kernel::tests::witness_snapshot();
    let requirement = supplier
        .owners
        .iter_mut()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Requirement(requirement) => {
                requirement.limits = vec![ResourceLimit {
                    name: Name::new("maximum_calls").unwrap(),
                    maximum: 2,
                    unit: ResourceUnit::Calls,
                }];
                Some(*key)
            }
            _ => None,
        })
        .unwrap();
    let consumer = consumer_of(&supplier);
    accept(&supplier, &consumer);
    let mut bad = consumer.clone();
    let PackageInterfaceRecord::Requirement(record) =
        imported(&mut bad).get_mut(&requirement).unwrap()
    else {
        unreachable!()
    };
    record.limits[0].maximum = 3;
    reject(&supplier, &bad);

    let canonical = &supplier.owners[&requirement];
    let record = &consumer.dependency_interfaces.values().next().unwrap()[&requirement];
    let control = ExecutionControl::uncancelled();
    let PackageInterfaceRecord::Requirement(value) = record else {
        unreachable!()
    };
    // Literal required comparisons: every operation plus the single limit.
    let expected_work = value.operations.len() + 1;
    for maximum_work in [expected_work, expected_work - 1] {
        let mut work = 0;
        let result = same_owner(
            record,
            canonical,
            &mut Budget {
                control: &control,
                work: &mut work,
                maximum_work,
                bytes: 0,
                maximum_bytes: 0,
            },
        );
        if maximum_work == expected_work {
            assert!(result.unwrap());
            assert_eq!(work, expected_work);
        } else {
            assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        }
    }
}

#[test]
fn exact_supplier_binding_cancellation_and_cumulative_limits_remain_distinct() {
    let (supplier, consumer) = fixture();
    let inputs = [&supplier, &consumer];
    let mut completed = false;
    for cutoff in 0..2048 {
        match validate(
            &inputs,
            &ExecutionControl::cancel_after_checks(cutoff),
            &mut 0,
        ) {
            Ok(()) => {
                completed = true;
                break;
            }
            Err(error) => assert_eq!(error.class, ExecutionFailureClass::Cancelled, "{error:?}"),
        }
    }
    assert!(
        completed,
        "finite fixture did not complete within its harness guard"
    );
    for (initial_work, maximum_work, maximum_bytes) in [
        (0, 1, MAXIMUM_BYTES),
        (1, 1, MAXIMUM_BYTES),
        (0, usize::MAX, 0),
    ] {
        let mut work = initial_work;
        let error = validate_with_budget(
            &inputs,
            &mut Budget {
                control: &ExecutionControl::uncancelled(),
                work: &mut work,
                maximum_work,
                bytes: 0,
                maximum_bytes,
            },
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        assert_eq!(error.code, "normalized_reference_dependency_bound");
    }
    let mut work = 37;
    validate(&inputs, &ExecutionControl::uncancelled(), &mut work).unwrap();
    assert!(work > 37);

    let control = ExecutionControl::uncancelled();
    let mut measured_work = 0;
    let mut measured = Budget {
        control: &control,
        work: &mut measured_work,
        maximum_work: usize::MAX,
        bytes: 0,
        maximum_bytes: MAXIMUM_BYTES,
    };
    validate_with_budget(&inputs, &mut measured).unwrap();
    let exact_bytes = measured.bytes;
    for (maximum_work, maximum_bytes, success) in [
        (measured_work, exact_bytes, true),
        (measured_work - 1, exact_bytes, false),
        (measured_work, exact_bytes - 1, false),
    ] {
        let mut work = 0;
        let result = validate_with_budget(
            &inputs,
            &mut Budget {
                control: &control,
                work: &mut work,
                maximum_work,
                bytes: 0,
                maximum_bytes,
            },
        );
        if success {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        }
    }
}

#[test]
fn exact_supplier_binding_accepts_private_read_view_representation_without_private_callable_authority()
 {
    use crate::platform::kernel::{Name, ReadViewFieldKind};
    use crate::platform::semantic_id::ReadScopeParameterId;

    // Genuine authored generic view metadata; the existing relative import
    // fixture attaches its exact projected private layout, never a body seal.
    let marker = "(module create supplier\n";
    assert_eq!(SOURCE.matches(marker).count(), 1);
    let view_source = r#"(module create supplier
  (read-view create HiddenView (visibility private)
    (type-parameter create T (constraint owned))
    (scope-parameter read_scope_d8a00000000000000000000000000001 s)
    (view-field field_d8a00000000000000000000000000001 source (type T)
      (read-reference (read-scope (declaration HiddenView) read_scope_d8a00000000000000000000000000001)))
    (view-field field_d8a00000000000000000000000000002 length (type I64) (ordinary)))
"#;
    let input = SOURCE.replacen(marker, view_source, 1);
    let supplier =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input)
            .unwrap();
    let view_key = supplier
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Declaration(record) if record.name.as_str() == "HiddenView" => Some(*key),
            _ => None,
        })
        .unwrap();
    let OwnerRecord::Declaration(canonical) = &supplier.owners[&view_key] else {
        unreachable!()
    };
    assert_eq!(canonical.visibility, DeclarationVisibility::Private);
    let Source::ReadView(view) = &canonical.payload else {
        unreachable!()
    };
    assert_eq!(view.type_parameters.len(), 1);
    assert_eq!(view.fields.len(), 2);
    assert!(matches!(
        view.fields[0].kind,
        ReadViewFieldKind::ReadReference { .. }
    ));
    assert_eq!(view.fields[1].kind, ReadViewFieldKind::Ordinary);
    let scalar = view.fields[1].ty;
    assert!(matches!(supplier.types[&scalar].form, TypeForm::I64));
    let projected = PackageInterfaceRecord::project_public(&supplier.owners[&view_key])
        .unwrap()
        .unwrap();
    projected.validate_local().unwrap();
    assert!(matches!(
        projected,
        PackageInterfaceRecord::RepresentationDeclaration(_)
    ));
    let mut consumer = consumer_of(&supplier);
    assert!(!imported(&mut consumer).contains_key(&view_key));
    assert!(
        imported(&mut consumer)
            .insert(view_key, projected)
            .is_none()
    );
    accept(&supplier, &consumer);

    // Exact correspondence still charges every formal and inline field when
    // the private representation category is admitted. No metadata is allocated.
    let record = &consumer.dependency_interfaces.values().next().unwrap()[&view_key];
    let control = ExecutionControl::uncancelled();
    let expected_work = view.type_parameters.len() + view.fields.len();
    for maximum_work in [expected_work, expected_work - 1] {
        let mut work = 0;
        let result = same_owner(
            record,
            &supplier.owners[&view_key],
            &mut Budget {
                control: &control,
                work: &mut work,
                maximum_work,
                bytes: 0,
                maximum_bytes: 0,
            },
        );
        if maximum_work == expected_work {
            assert!(result.unwrap());
            assert_eq!(work, expected_work);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Resource);
            assert_eq!(error.code, "normalized_reference_dependency_bound");
        }
    }

    for fault in [
        "public-naming",
        "name",
        "header",
        "generation",
        "formal",
        "scope",
        "field",
        "field-type",
        "field-kind",
        "field-order",
        "type-root",
        "type-conflict",
    ] {
        let mut bad = consumer.clone();
        match fault {
            "public-naming" => {
                let PackageInterfaceRecord::RepresentationDeclaration(record) =
                    imported(&mut bad).get_mut(&view_key).unwrap()
                else {
                    unreachable!()
                };
                let record = record.clone();
                imported(&mut bad).insert(view_key, PackageInterfaceRecord::Declaration(record));
            }
            "type-root" => {
                assert!(bad.dependency_types.remove(&scalar).is_some());
            }
            "type-conflict" => {
                bad.dependency_types
                    .insert(scalar, TypeObject::new(TypeForm::Bool).unwrap());
            }
            _ => {
                let PackageInterfaceRecord::RepresentationDeclaration(record) =
                    imported(&mut bad).get_mut(&view_key).unwrap()
                else {
                    unreachable!()
                };
                match fault {
                    "name" => record.name = Name::new("forged-view").unwrap(),
                    "header" => {
                        record.header.owner = OwnerKey::Declaration(
                            crate::platform::semantic_id::DeclarationId::migrate(
                                b"private-view-correspondence-fault",
                                0,
                            ),
                        );
                    }
                    "generation" => record.header.contract_version = 33,
                    _ => {
                        let Interface::ReadView(view) = &mut record.payload else {
                            unreachable!()
                        };
                        match fault {
                            "formal" => view.type_parameters.push(view.type_parameters[0]),
                            "scope" => {
                                view.scope_parameter.id = ReadScopeParameterId::migrate(
                                    b"private-view-correspondence-fault",
                                    0,
                                );
                            }
                            "field" => view.fields[0].name = Name::new("forged-source").unwrap(),
                            "field-type" => view.fields[0].ty = scalar,
                            "field-kind" => view.fields[0].kind = ReadViewFieldKind::Ordinary,
                            "field-order" => view.fields.swap(0, 1),
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
        reject(&supplier, &bad);
    }

    // A package-visible nominal layout obeys the same private representation
    // rule; public nominal declarations require the public record category.
    let mut package_visible = supplier.clone();
    let OwnerRecord::Declaration(record) = package_visible.owners.get_mut(&view_key).unwrap()
    else {
        unreachable!()
    };
    record.visibility = DeclarationVisibility::Package;
    accept(&package_visible, &consumer);
    let mut public = supplier.clone();
    let OwnerRecord::Declaration(record) = public.owners.get_mut(&view_key).unwrap() else {
        unreachable!()
    };
    record.visibility = DeclarationVisibility::Public;
    reject(&public, &consumer);
    let mut public_consumer = consumer.clone();
    let PackageInterfaceRecord::RepresentationDeclaration(record) =
        imported(&mut public_consumer).get_mut(&view_key).unwrap()
    else {
        unreachable!()
    };
    let record = record.clone();
    imported(&mut public_consumer).insert(view_key, PackageInterfaceRecord::Declaration(record));
    accept(&public, &public_consumer);

    // Even exact private callable bytes are never nominal representation
    // evidence. The producer and independent canonical comparison both refuse.
    let function_key = supplier
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Declaration(record) if matches!(record.payload, Source::Function(_)) => {
                Some(*key)
            }
            _ => None,
        })
        .unwrap();
    let mut private_function_source = supplier.clone();
    let OwnerRecord::Declaration(record) = private_function_source
        .owners
        .get_mut(&function_key)
        .unwrap()
    else {
        unreachable!()
    };
    record.visibility = DeclarationVisibility::Private;
    assert_eq!(
        PackageInterfaceRecord::project_public(&private_function_source.owners[&function_key])
            .unwrap_err()
            .code,
        "kernel_package_interface_visibility"
    );
    let mut forged_function = consumer.clone();
    let PackageInterfaceRecord::Declaration(record) = imported(&mut forged_function)
        .get_mut(&function_key)
        .unwrap()
    else {
        unreachable!()
    };
    let record = record.clone();
    imported(&mut forged_function).insert(
        function_key,
        PackageInterfaceRecord::RepresentationDeclaration(record),
    );
    reject(&private_function_source, &forged_function);
    // Every failed correspondence leaves the original exact private view healthy.
    accept(&supplier, &consumer);
}
