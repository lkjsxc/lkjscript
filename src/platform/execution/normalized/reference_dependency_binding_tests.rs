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
        // This older binding generation is valid even when the supplier body is
        // current. It is an encoding generation, not the supplier's generation.
        graph_contract_version: 32,
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
                f.result_borrow = None;
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
            "member" => i.associated_types[0].ty = TypeObjectDigest::from_bytes([87; 32]),
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
        shadow.graph_contract_version = 33;
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
