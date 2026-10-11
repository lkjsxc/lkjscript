//! Literal canonical controls for closure, prerequisite and parallel boundaries.
//! Expected results come from the source records, not a production carrier.
use super::*;
use crate::platform::semantic_id::{ExpressionId, FieldId, ParameterId};

fn expression(source: &mut KernelSnapshot, n: u64, operation: ExpressionOperation) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
    );
    id
}
fn function_mut(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
) -> &mut FunctionDeclaration {
    let Some(OwnerRecord::Declaration(d)) = source
        .owners
        .get_mut(&OwnerKey::Declaration(scope.declaration))
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        unreachable!()
    };
    f
}
fn value_parameter(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
    n: u64,
    ty: TypeObjectDigest,
    mode: ParameterUse,
) -> ParameterId {
    let id = ParameterId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Parameter(id),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
            parent: ParameterParent::Function(scope.declaration),
            name: name(&format!("value{n}")),
            ty,
            use_mode: mode,
            resource_requirement: None,
        }),
    );
    id
}

#[test]
fn associated_memory_closure_checks_untaken_orphan_and_shared_contexts() {
    use super::super::super::memory::associated_tests::{FlowKind, fixture as flow_fixture};
    let mut f = flow_fixture(FlowKind::Direct);
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
    let condition = expression(&mut f.source, 50, ExpressionOperation::Bool { value: true });
    let other = expression(
        &mut f.source,
        51,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(f.first),
        },
    );
    let body = expression(
        &mut f.source,
        52,
        ExpressionOperation::If {
            condition,
            when_true: f.borrow,
            when_false: other,
        },
    );
    function_mut(&mut f.source, f.caller).body = body;
    assert!(accepts(&f.source));
    let mut old = f.source.clone();
    old.owners
        .get_mut(&OwnerKey::Expression(other))
        .unwrap()
        .set_encoding_for_edit(32);
    assert_eq!(
        Resolver::new(&old).admit_canonical_closure(),
        Err(Failure::Generation)
    );
    assert!(!accepts(&old));
    let mut orphan = f.source.clone();
    expression(
        &mut orphan,
        53,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(f.first),
        },
    );
    assert_eq!(
        Resolver::new(&orphan).admit_canonical_closure(),
        Err(Failure::Authority)
    );
    let mut shared = f.source.clone();
    shared.owners.remove(&OwnerKey::Expression(other));
    let Some(OwnerRecord::Expression(e)) = shared.owners.get_mut(&OwnerKey::Expression(body))
    else {
        unreachable!()
    };
    let ExpressionOperation::If { when_false, .. } = &mut e.operation else {
        unreachable!()
    };
    *when_false = f.local;
    assert_eq!(
        Resolver::new(&shared).admit_canonical_closure(),
        Err(Failure::Authority)
    );
}

#[test]
fn associated_memory_closure_checks_imported_selection_and_owner_authority() {
    let mut f = fixture();
    let package = PackageId::migrate(SEED, 90);
    let revision = PackageRevisionDigest::from_bytes([90; 32]);
    let selection = DependencyRecord {
        package,
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([91; 32]),
        package_revision: revision,
    };
    let Some(OwnerRecord::Declaration(d)) = f
        .source
        .owners
        .get(&OwnerKey::Declaration(f.contract.declaration))
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(c) = &d.payload else {
        unreachable!()
    };
    let imported = PackageInterfaceDeclaration {
        header: d.header,
        name: d.name.clone(),
        payload: PackageInterfaceDeclarationPayload::OwnedContract(c.clone()),
    };
    let parameter = c.self_parameter;
    let Some(OwnerRecord::TypeParameter(p)) =
        f.source.owners.get(&OwnerKey::TypeParameter(parameter))
    else {
        unreachable!()
    };
    f.source.dependency_interfaces.insert(
        revision,
        std::sync::Arc::new(
            [
                (
                    imported.header.owner,
                    PackageInterfaceRecord::Declaration(imported),
                ),
                (
                    OwnerKey::TypeParameter(parameter),
                    PackageInterfaceRecord::TypeParameter(p.clone()),
                ),
            ]
            .into_iter()
            .collect(),
        ),
    );
    f.source.dependencies.insert(package, selection.clone());
    let imported_function = DeclarationReference {
        package,
        declaration: f.source_scope.declaration,
    };
    let projected = intern(
        &mut f.source,
        TypeForm::AssociatedWitnessProjection {
            scope: imported_function,
            parameter: f.source_witness,
            contract: DeclarationReference {
                package,
                declaration: f.contract.declaration,
            },
            member: f.member,
        },
    );
    let imported_value = ParameterId::migrate(SEED, 90);
    let mut signature = Oracle(&f.source, Some(f.source_scope))
        .function(f.source_scope)
        .unwrap();
    signature.parameters = vec![imported_value];
    signature.implementation_parameters[0].contract.package = package;
    let Some(OwnerRecord::TypeParameter(formal)) = f
        .source
        .owners
        .get(&OwnerKey::TypeParameter(f.source_parameter))
    else {
        unreachable!()
    };
    let formal = formal.clone();
    let records =
        std::sync::Arc::make_mut(f.source.dependency_interfaces.get_mut(&revision).unwrap());
    records.insert(
        OwnerKey::Declaration(imported_function.declaration),
        PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
            header: OwnerHeader::new(
                OwnerKey::Declaration(imported_function.declaration),
                OwnerKind::PureFunction,
            ),
            name: name("projected"),
            payload: PackageInterfaceDeclarationPayload::Function(signature),
        }),
    );
    records.insert(
        OwnerKey::TypeParameter(f.source_parameter),
        PackageInterfaceRecord::TypeParameter(formal),
    );
    let imported_parameter = ParameterRecord {
        header: OwnerHeader::new(OwnerKey::Parameter(imported_value), OwnerKind::Parameter),
        parent: ParameterParent::Function(imported_function.declaration),
        name: name("value"),
        ty: projected,
        use_mode: ParameterUse::Borrow,
        resource_requirement: None,
    };
    records.insert(
        OwnerKey::Parameter(imported_value),
        PackageInterfaceRecord::Parameter(imported_parameter.clone()),
    );
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
    let mut orphan = f.source.clone();
    let orphan_id = ParameterId::migrate(SEED, 91);
    let mut orphan_parameter = imported_parameter;
    orphan_parameter.header.owner = OwnerKey::Parameter(orphan_id);
    orphan_parameter.name = name("orphan");
    std::sync::Arc::make_mut(orphan.dependency_interfaces.get_mut(&revision).unwrap()).insert(
        OwnerKey::Parameter(orphan_id),
        PackageInterfaceRecord::Parameter(orphan_parameter),
    );
    assert_eq!(
        Resolver::new(&orphan).admit_canonical_closure(),
        Err(Failure::Authority)
    );
    for generation in [32, 34] {
        let mut old_selection = f.source.clone();
        old_selection
            .dependencies
            .get_mut(&package)
            .unwrap()
            .graph_contract_version = generation;
        assert_eq!(
            Resolver::new(&old_selection).admit_canonical_closure(),
            Err(Failure::Generation)
        );
        let mut old_owner = f.source.clone();
        let records =
            std::sync::Arc::make_mut(old_owner.dependency_interfaces.get_mut(&revision).unwrap());
        let Some(PackageInterfaceRecord::Declaration(d)) =
            records.get_mut(&OwnerKey::Declaration(f.contract.declaration))
        else {
            unreachable!()
        };
        d.header.contract_version = generation;
        assert_eq!(
            Resolver::new(&old_owner).admit_canonical_closure(),
            Err(Failure::Generation)
        );
    }
    let mut property_only = f.source.clone();
    property_only.dependencies.remove(&package);
    property_only
        .representation_dependencies
        .insert(package, selection);
    assert_eq!(
        Resolver::new(&property_only).admit_canonical_closure(),
        Err(Failure::Authority)
    );
}

#[test]
fn associated_memory_closure_checks_unused_method_actuals() {
    let mut f = fixture();
    let method = Oracle(&f.source, Some(f.contract))
        .contract(f.contract)
        .unwrap()
        .methods[0]
        .id;
    implementation_mut(&mut f.source, f.scheme).methods[0] = OwnedMethodImplementation {
        method,
        function: f.source_scope,
        type_arguments: vec![f.cell],
        implementations: vec![ImplementationOperand::Concrete {
            implementation: f.other_scheme,
            type_arguments: vec![f.cell],
            implementations: vec![],
        }],
    };
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
    assert!(Oracle(&f.source, Some(f.caller_scope)).valid_implementation_at(f.scheme));
    // The observer's ordinary actual is phantom in its zero-parameter I64
    // signature. It still belongs to the scheme definition's lexical scope.
    implementation_mut(&mut f.source, f.scheme).methods[0].type_arguments[0] = f.caller_projection;
    assert_eq!(
        Resolver::new(&f.source).admit_canonical_closure(),
        Err(Failure::Authority)
    );
}

#[test]
fn associated_memory_prerequisites_match_projected_arguments_by_exact_witness() {
    let mut f = fixture();
    let argument = TypeParameterId::migrate(SEED, 60);
    let q = insert_declaration(
        &mut f.source,
        60,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: TypeParameterId::migrate(SEED, 61),
            type_parameters: vec![argument],
            associated_types: vec![],
            methods: vec![OwnedMethod {
                id: MethodId::migrate(SEED, 60),
                name: name("observe"),
                parameters: vec![],
                result: f.scalar,
                result_borrow: None,
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    insert_parameter(&mut f.source, q, 60);
    insert_parameter(&mut f.source, q, 61);
    let source_q = ImplementationParameterId::migrate(SEED, 60);
    let caller_q = ImplementationParameterId::migrate(SEED, 61);
    let source_self = intern(
        &mut f.source,
        TypeForm::TypeParameter {
            parameter: f.source_parameter,
        },
    );
    function_mut(&mut f.source, f.source_scope)
        .implementation_parameters
        .push(ImplementationParameter {
            id: source_q,
            name: name("q"),
            contract: q,
            self_type: source_self,
            type_arguments: vec![f.source_projection],
        });
    function_mut(&mut f.source, f.caller_scope)
        .implementation_parameters
        .push(ImplementationParameter {
            id: caller_q,
            name: name("q"),
            contract: q,
            self_type: f.caller_parameter_type,
            type_arguments: vec![f.caller_projection],
        });
    let operands = vec![
        ImplementationOperand::Parameter {
            scope: f.caller_scope,
            parameter: f.caller_witness,
        },
        ImplementationOperand::Parameter {
            scope: f.caller_scope,
            parameter: caller_q,
        },
    ];
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
    assert!(Oracle(&f.source, Some(f.caller_scope)).witnesses(
        f.source_scope,
        &[f.caller_parameter_type],
        &operands
    ));
    let other = ImplementationParameterId::migrate(SEED, 62);
    function_mut(&mut f.source, f.caller_scope)
        .implementation_parameters
        .push(ImplementationParameter {
            id: other,
            name: name("other"),
            contract: f.contract,
            self_type: f.caller_parameter_type,
            type_arguments: vec![],
        });
    let different = intern(
        &mut f.source,
        TypeForm::AssociatedWitnessProjection {
            scope: f.caller_scope,
            parameter: other,
            contract: f.contract,
            member: f.member,
        },
    );
    function_mut(&mut f.source, f.caller_scope)
        .implementation_parameters
        .iter_mut()
        .find(|p| p.id == caller_q)
        .unwrap()
        .type_arguments[0] = different;
    // Both anchors have exactly the same Self and contract. Their selected
    // member authority is nevertheless distinct.
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
    assert!(!Oracle(&f.source, Some(f.caller_scope)).witnesses(
        f.source_scope,
        &[f.caller_parameter_type],
        &operands
    ));
}

#[test]
fn associated_memory_parallel_resolves_carriers_without_granting_phantom_rights() {
    for mode in [ParameterUse::Borrow, ParameterUse::Consume] {
        let mut f = fixture();
        let source_value =
            value_parameter(&mut f.source, f.source_scope, 70, f.source_projection, mode);
        let caller_value = value_parameter(
            &mut f.source,
            f.caller_scope,
            71,
            f.cell,
            ParameterUse::Consume,
        );
        let source_body = if mode == ParameterUse::Consume {
            expression(
                &mut f.source,
                70,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(source_value),
                },
            )
        } else {
            expression(&mut f.source, 70, ExpressionOperation::I64 { value: 0 })
        };
        {
            let Some(OwnerRecord::Declaration(d)) = f
                .source
                .owners
                .get_mut(&OwnerKey::Declaration(f.source_scope.declaration))
            else {
                unreachable!()
            };
            d.header.kind = OwnerKind::TaskFunction;
        }
        let source_function = function_mut(&mut f.source, f.source_scope);
        source_function.parameters = vec![source_value];
        source_function.effect = FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        };
        source_function.result = if mode == ParameterUse::Consume {
            f.source_projection
        } else {
            f.scalar
        };
        source_function.body = source_body;
        let local = expression(
            &mut f.source,
            71,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(caller_value),
            },
        );
        let call = expression(
            &mut f.source,
            72,
            ExpressionOperation::ImplementationCall {
                function: f.source_scope,
                type_arguments: vec![f.cell],
                implementations: vec![ImplementationOperand::Concrete {
                    implementation: f.scheme,
                    type_arguments: vec![f.cell],
                    implementations: vec![],
                }],
                arguments: vec![local],
                effect_arguments: vec![],
                requirement_arguments: vec![],
            },
        );
        function_mut(&mut f.source, f.caller_scope).parameters = vec![caller_value];
        function_mut(&mut f.source, f.caller_scope).body = call;
        Resolver::new(&f.source).admit_canonical_closure().unwrap();
        let oracle = Oracle(&f.source, Some(f.caller_scope));
        assert_eq!(
            oracle.parallel_child(call),
            Some(mode == ParameterUse::Consume)
        );
        let root = LocalValueReference::FunctionParameter(caller_value);
        let mut rights = Rights::default();
        rights.memory.insert(root);
        rights.owned.insert(root);
        let loans = oracle.prepare_parallel_child(call, &mut rights, 0).unwrap();
        if mode == ParameterUse::Borrow {
            assert_eq!(rights.loans.get(&root), Some(&1));
            assert!(rights.consume(root).is_none());
            for loan in loans {
                rights.release(loan).unwrap();
            }
            assert!(rights.loans.is_empty());
            rights.consume(root).unwrap();
        } else {
            assert!(loans.is_empty());
            assert!(!rights.owned.contains(&root));
        }
        let mut unknown = f.source.clone();
        let Some(OwnerRecord::Parameter(p)) =
            unknown.owners.get_mut(&OwnerKey::Parameter(caller_value))
        else {
            unreachable!()
        };
        p.ty = f.caller_projection;
        let Some(OwnerRecord::Expression(e)) = unknown.owners.get_mut(&OwnerKey::Expression(call))
        else {
            unreachable!()
        };
        let ExpressionOperation::ImplementationCall {
            type_arguments,
            implementations,
            ..
        } = &mut e.operation
        else {
            unreachable!()
        };
        *type_arguments = vec![f.caller_parameter_type];
        *implementations = vec![ImplementationOperand::Parameter {
            scope: f.caller_scope,
            parameter: f.caller_witness,
        }];
        Resolver::new(&unknown).admit_canonical_closure().unwrap();
        assert_eq!(
            Oracle(&unknown, Some(f.caller_scope)).parallel_child(call),
            None
        );
        let mut phantom = f.source.clone();
        let Some(OwnerRecord::Parameter(p)) =
            phantom.owners.get_mut(&OwnerKey::Parameter(caller_value))
        else {
            unreachable!()
        };
        p.ty = f.buffer;
        let Some(OwnerRecord::Expression(e)) = phantom.owners.get_mut(&OwnerKey::Expression(call))
        else {
            unreachable!()
        };
        let ExpressionOperation::ImplementationCall {
            implementations, ..
        } = &mut e.operation
        else {
            unreachable!()
        };
        *implementations = vec![ImplementationOperand::Concrete {
            implementation: f.other_scheme,
            type_arguments: vec![f.caller_projection],
            implementations: vec![],
        }];
        Resolver::new(&phantom).admit_canonical_closure().unwrap();
        assert_eq!(
            Oracle(&phantom, Some(f.caller_scope)).parallel_child(call),
            Some(mode == ParameterUse::Consume)
        );
    }
}

#[test]
fn associated_memory_nominal_actuals_are_opaque_and_private_templates_forbidden() {
    let mut f = fixture();
    let field = FieldId::migrate(SEED, 80);
    let record = insert_declaration(
        &mut f.source,
        80,
        OwnerKind::OwnedRecord,
        DeclarationPayload::OwnedRecord {
            type_parameters: vec![TypeParameterId::migrate(SEED, 80)],
            fields: vec![field],
        },
    );
    let (_, template) = insert_parameter(&mut f.source, record, 80);
    f.source.owners.insert(
        OwnerKey::Field(field),
        OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
            declaration: record.declaration,
            name: name("private"),
            ty: template,
        }),
    );
    let actual = intern(
        &mut f.source,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![f.caller_projection],
        },
    );
    let p = value_parameter(
        &mut f.source,
        f.caller_scope,
        80,
        actual,
        ParameterUse::Borrow,
    );
    function_mut(&mut f.source, f.caller_scope).parameters = vec![p];
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
    let oracle = Oracle(&f.source, Some(f.caller_scope));
    assert!(oracle.product_shape(actual));
    assert!(Oracle(&f.source, None).valid_record_application(actual));
    assert_eq!(
        oracle.record_fields(actual).unwrap().1[0].1,
        f.caller_projection
    );
    assert!(oracle.record_access(actual));
    let private_module = crate::platform::semantic_id::ModuleId::migrate(SEED, 80);
    f.source.owners.insert(
        OwnerKey::Module(private_module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(private_module), OwnerKind::Module),
            name: name("private_module"),
        }),
    );
    let Some(OwnerRecord::Declaration(d)) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(record.declaration))
    else {
        unreachable!()
    };
    d.module = private_module;
    assert!(!Oracle(&f.source, Some(f.caller_scope)).record_access(actual));
    // Changing access cannot remove the independent property proof.
    assert!(Oracle(&f.source, Some(f.caller_scope)).product_shape(actual));
    let Some(OwnerRecord::Field(field)) = f.source.owners.get_mut(&OwnerKey::Field(field)) else {
        unreachable!()
    };
    field.ty = f.caller_projection;
    assert_eq!(
        Resolver::new(&f.source).admit_canonical_closure(),
        Err(Failure::Authority)
    );
}

#[test]
fn associated_memory_closure_capacity_is_cumulative_and_retry_is_independent() {
    let f = fixture();
    let original = f.source.clone();
    let mut healthy = Resolver::new(&f.source);
    healthy.admit_canonical_closure().unwrap();
    let entries = healthy.retained_entries();
    assert!(entries > 1);
    assert_eq!(
        Resolver::with_limits(
            &f.source,
            Limits {
                work: contract::MAXIMUM_VALIDATION_WORK,
                objects: entries - 1
            }
        )
        .admit_canonical_closure(),
        Err(Failure::Capacity)
    );
    assert_eq!(f.source.owners, original.owners);
    assert_eq!(f.source.types, original.types);
    Resolver::with_limits(
        &f.source,
        Limits {
            work: contract::MAXIMUM_VALIDATION_WORK,
            objects: entries,
        },
    )
    .admit_canonical_closure()
    .unwrap();
    Resolver::new(&f.source).admit_canonical_closure().unwrap();
}
