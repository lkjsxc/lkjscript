//! Canonical consumer-before-supplier, imported descendant and projected group
//! witnesses. These are helper proofs over raw records, not admission tokens or
//! evidence that the deliberately closed global associated gate is available.
use super::closure_tests::{expression, function_mut, value_parameter};
use super::*;
use crate::platform::semantic_id::{
    BindingId, ExpressionId, FieldId, ModuleId, ParameterId, RevisionId,
};

struct Vertical {
    consumer: KernelSnapshot,
    supplier: KernelSnapshot,
    client: KernelSnapshot,
    contract: DeclarationReference,
    generic_read: DeclarationReference,
    task_read: DeclarationReference,
    task_read_data: DeclarationReference,
    task_drop: DeclarationReference,
    record: DeclarationReference,
    field: FieldReference,
    schemes: Vec<DeclarationReference>,
    nominal: TypeObjectDigest,
    cell: TypeObjectDigest,
    buffer: TypeObjectDigest,
    scalar: TypeObjectDigest,
    caller: DeclarationReference,
    parent: ParameterId,
    call: ExpressionId,
    borrow: ExpressionId,
    view: BindingId,
}

fn empty_snapshot(n: u64) -> KernelSnapshot {
    let mut source = super::super::super::tests::transport_snapshot();
    source.root.package_id = PackageId::migrate(SEED, n);
    source.owners.clear();
    source.types.clear();
    source.dependencies.clear();
    source.dependency_interfaces.clear();
    source.dependency_types.clear();
    source.representation_dependencies.clear();
    let module = ModuleId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: name(&format!("module{n}")),
        }),
    );
    source
}

fn bare_function(
    source: &mut KernelSnapshot,
    n: u64,
    task: bool,
    result: TypeObjectDigest,
) -> DeclarationReference {
    let body = expression(source, n, ExpressionOperation::I64 { value: 0 });
    insert_declaration(
        source,
        n,
        if task {
            OwnerKind::TaskFunction
        } else {
            OwnerKind::PureFunction
        },
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![],
            implementation_parameters: vec![],
            effect_parameters: vec![],
            requirement_parameters: vec![],
            parameters: vec![],
            result: crate::platform::kernel::CallableResult {
                ty: result,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: if task {
                FunctionEffect::Task {
                    requirements: vec![],
                    effect_parameters: vec![],
                }
            } else {
                FunctionEffect::Pure
            },
            body,

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
        }),
    )
}

fn binding(source: &mut KernelSnapshot, n: u64, ty: TypeObjectDigest) -> BindingId {
    let id = BindingId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Binding(id),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
            name: name(&format!("view{n}")),
            kind: BindingKind::OwnedBorrow,
            value: None,
            declared_type: Some(ty),
        }),
    );
    id
}

fn local(source: &mut KernelSnapshot, n: u64, value: LocalValueReference) -> ExpressionId {
    expression(source, n, ExpressionOperation::Local { value })
}

// Project literal producer records ourselves. No production carrier, resolver,
// classification or inferred flags participate. The fixture revision is bound
// to the exact canonical producer bytes, not a constant shared by packages.
fn import(source: &mut KernelSnapshot, producer: &KernelSnapshot) -> PackageRevisionDigest {
    let mut records = BTreeMap::new();
    // This is a test-local fixture revision, not the product package transport
    // revision. Include the complete raw root as well as the exported records,
    // so even byte-identical inventories in different packages bind separately.
    let mut bytes = encode_root(&producer.root).unwrap().1;
    for (key, owner) in &producer.owners {
        bytes.extend(encode_owner(owner).unwrap().1);
        let record = match owner {
            OwnerRecord::Declaration(d) => {
                let payload = match &d.payload {
                    DeclarationPayload::OwnedContract(c) => {
                        PackageInterfaceDeclarationPayload::OwnedContract(c.clone())
                    }
                    DeclarationPayload::OwnedImplementation(i) => {
                        PackageInterfaceDeclarationPayload::OwnedImplementation(i.clone())
                    }
                    DeclarationPayload::OwnedRecord {
                        type_parameters,
                        fields,
                    } => PackageInterfaceDeclarationPayload::OwnedRecord {
                        type_parameters: type_parameters.clone(),
                        fields: fields.clone(),
                    },
                    DeclarationPayload::Function(f) => {
                        PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                            type_parameters: f.type_parameters.clone(),
                            implementation_parameters: f.implementation_parameters.clone(),
                            effect_parameters: f.effect_parameters.clone(),
                            requirement_parameters: f.requirement_parameters.clone(),
                            parameters: f.parameters.clone(),
                            result: f.result,
                            effect: f.effect.clone(),

                            read_scope_parameters: f.read_scope_parameters.clone(),
                            read_scope_sources: f.read_scope_sources.clone(),
                        })
                    }
                    _ => panic!("fixture exports only declared memory vocabulary"),
                };
                Some(PackageInterfaceRecord::Declaration(
                    PackageInterfaceDeclaration {
                        header: d.header,
                        name: d.name.clone(),
                        payload,
                    },
                ))
            }
            OwnerRecord::TypeParameter(p) => Some(PackageInterfaceRecord::TypeParameter(p.clone())),
            OwnerRecord::Parameter(p) => Some(PackageInterfaceRecord::Parameter(p.clone())),
            OwnerRecord::Field(f) => Some(PackageInterfaceRecord::Field(f.clone())),
            OwnerRecord::Module(_) | OwnerRecord::Binding(_) | OwnerRecord::Expression(_) => None,
            _ => panic!("unexpected fixture owner"),
        };
        if let Some(record) = record {
            records.insert(*key, record);
        }
    }
    for object in producer.types.values() {
        bytes.extend(encode_type_object(object).unwrap().1);
    }
    for dependency in producer.dependencies.values() {
        bytes.extend(encode_dependency(dependency).unwrap().1);
    }
    let semantic_revision = RevisionId::from_digest(*blake3::hash(&bytes).as_bytes());
    bytes.extend_from_slice(b"independent-memory-fixture-package");
    let revision = PackageRevisionDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
    source.dependencies.insert(
        producer.root.package_id,
        DependencyRecord {
            package: producer.root.package_id,
            graph_contract_version: producer.root.graph_contract_version,
            semantic_revision,
            package_revision: revision,
        },
    );
    source
        .dependency_interfaces
        .insert(revision, std::sync::Arc::new(records));
    source.dependency_types.extend(producer.types.clone());
    source
        .dependency_types
        .extend(producer.dependency_types.clone());
    revision
}

fn call(
    source: &mut KernelSnapshot,
    n: u64,
    function: DeclarationReference,
    types: Vec<TypeObjectDigest>,
    witness: Option<ImplementationOperand>,
    arguments: Vec<ExpressionId>,
) -> ExpressionId {
    expression(
        source,
        n,
        match witness {
            Some(witness) => ExpressionOperation::ImplementationCall {
                function,
                type_arguments: types,
                implementations: vec![witness],
                arguments,
                effect_arguments: vec![],
                requirement_arguments: vec![],

                read_scope_arguments: Vec::new(),
            },
            None => ExpressionOperation::Call {
                function,
                type_arguments: types,
                arguments,
                effect_arguments: vec![],
                requirement_arguments: vec![],

                read_scope_arguments: Vec::new(),
            },
        },
    )
}

fn selected(f: &Vertical, index: usize) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: f.schemes[index],
        type_arguments: vec![f.cell, f.buffer],
        implementations: vec![],
    }
}

fn vertical() -> Vertical {
    let mut consumer = empty_snapshot(100);
    let scalar = intern(&mut consumer, TypeForm::I64);
    let cell = intern(&mut consumer, TypeForm::OwnedI64Cell);
    let buffer = intern(&mut consumer, TypeForm::ByteBuffer);
    let contract_ref = DeclarationReference {
        package: consumer.root.package_id,
        declaration: DeclarationId::migrate(SEED, 100),
    };
    let member = AssociatedTypeMemberId::migrate(SEED, 100);
    let self_parameter = TypeParameterId::migrate(SEED, 100);
    let self_type = intern(
        &mut consumer,
        TypeForm::TypeParameter {
            parameter: self_parameter,
        },
    );
    let output = intern(
        &mut consumer,
        TypeForm::AssociatedContractMember {
            contract: contract_ref,
            member,
        },
    );
    let method = MethodId::migrate(SEED, 100);
    let contract = insert_declaration(
        &mut consumer,
        100,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter,
            type_parameters: vec![],
            associated_types: vec![AssociatedTypeMember {
                id: member,
                name: name("Out"),
                bound: AssociatedTypeBound::Owned,
            }],
            methods: vec![OwnedMethod {
                id: method,
                name: name("read"),
                parameters: vec![OwnedMethodParameter {
                    ty: self_type,
                    use_mode: ParameterUse::Borrow,
                }],
                result: crate::platform::kernel::CallableResult {
                    ty: output,
                    relation: crate::platform::kernel::ResultRelation::ReadFrom { source: 0 },
                },
                effect: FunctionEffect::Pure,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
            }],
        }),
    );
    insert_parameter(&mut consumer, contract, 100);
    let generic_read = bare_function(&mut consumer, 110, false, scalar);
    let task_read = bare_function(&mut consumer, 120, true, scalar);
    let task_read_data = bare_function(&mut consumer, 130, true, scalar);
    for (ordinal, scope) in [(110, generic_read), (120, task_read), (130, task_read_data)] {
        let (t, ty) = insert_parameter(&mut consumer, scope, ordinal);
        let witness = ImplementationParameterId::migrate(SEED, ordinal);
        let projection = intern(
            &mut consumer,
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter: witness,
                contract,
                member,
            },
        );
        let parameter = value_parameter(
            &mut consumer,
            scope,
            ordinal,
            if scope == generic_read {
                ty
            } else {
                projection
            },
            ParameterUse::Borrow,
        );
        let mut parameters = vec![parameter];
        if scope == task_read_data {
            let ordinary = value_parameter(
                &mut consumer,
                scope,
                131,
                scalar,
                ParameterUse::Unrestricted,
            );
            parameters.insert(0, ordinary);
        }
        let f = function_mut(&mut consumer, scope);
        f.type_parameters = vec![t];
        f.implementation_parameters = vec![ImplementationParameter {
            id: witness,
            name: name("reader"),
            contract,
            self_type: ty,
            type_arguments: vec![],
        }];
        f.parameters = parameters;
        if scope == generic_read {
            let argument = local(
                &mut consumer,
                111,
                LocalValueReference::FunctionParameter(parameter),
            );
            let invocation = expression(
                &mut consumer,
                112,
                ExpressionOperation::MethodCall {
                    witness: ImplementationOperand::Parameter {
                        scope,
                        parameter: witness,
                    },
                    contract,
                    method,
                    arguments: vec![argument],

                    read_scope_arguments: Vec::new(),
                },
            );
            let view = binding(&mut consumer, 110, projection);
            let result = local(
                &mut consumer,
                113,
                LocalValueReference::LexicalBinding(view),
            );
            let body = expression(
                &mut consumer,
                114,
                ExpressionOperation::BorrowCall {
                    call: invocation,
                    binding: view,
                    body: result,
                },
            );
            let f = function_mut(&mut consumer, scope);
            f.result.ty = projection;
            f.result.relation = ResultRelation::ReadFrom { source: parameter };
            f.body = body;
            consumer
                .owners
                .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, ordinal)));
        }
    }
    let task_drop = bare_function(&mut consumer, 140, true, scalar);
    let (t, ty) = insert_parameter(&mut consumer, task_drop, 140);
    let input = value_parameter(&mut consumer, task_drop, 140, ty, ParameterUse::Consume);
    let f = function_mut(&mut consumer, task_drop);
    f.type_parameters = vec![t];
    f.parameters = vec![input];
    // The abstract consumer exists before any nominal supplier is authored.
    assert!(accepts(&consumer));

    let mut supplier = empty_snapshot(200);
    import(&mut supplier, &consumer);
    intern(&mut supplier, TypeForm::I64);
    intern(&mut supplier, TypeForm::OwnedI64Cell);
    intern(&mut supplier, TypeForm::ByteBuffer);
    let a = FieldId::migrate(SEED, 200);
    let b = FieldId::migrate(SEED, 201);
    let mut fields = vec![a, b];
    fields.sort();
    let record = insert_declaration(
        &mut supplier,
        200,
        OwnerKind::OwnedRecord,
        DeclarationPayload::OwnedRecord {
            type_parameters: vec![
                TypeParameterId::migrate(SEED, 200),
                TypeParameterId::migrate(SEED, 201),
            ],
            fields,
        },
    );
    let (_, first) = insert_parameter(&mut supplier, record, 200);
    let (_, second) = insert_parameter(&mut supplier, record, 201);
    for (id, ty, label) in [(a, first, "first"), (b, second, "second")] {
        supplier.owners.insert(
            OwnerKey::Field(id),
            OwnerRecord::Field(FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(id), OwnerKind::Field),
                declaration: record.declaration,
                name: name(label),
                ty,
            }),
        );
    }
    let field = FieldReference {
        package: supplier.root.package_id,
        field: a,
    };
    let mut schemes = Vec::new();
    for (n, selected_field) in [(210, Some(a)), (220, Some(b)), (230, None)] {
        let method_scope = bare_function(&mut supplier, n + 1, false, scalar);
        let (p, pty) = insert_parameter(&mut supplier, method_scope, n + 1);
        let (q, qty) = insert_parameter(&mut supplier, method_scope, n + 2);
        let parent_ty = intern(
            &mut supplier,
            TypeForm::Applied {
                declaration: record,
                arguments: vec![pty, qty],
            },
        );
        let result_ty = match selected_field {
            Some(id) if id == a => pty,
            Some(_) => qty,
            None => parent_ty,
        };
        let parameter = value_parameter(
            &mut supplier,
            method_scope,
            n + 1,
            parent_ty,
            ParameterUse::Borrow,
        );
        let source = local(
            &mut supplier,
            n * 10,
            LocalValueReference::FunctionParameter(parameter),
        );
        let body = if let Some(field_id) = selected_field {
            let view = binding(&mut supplier, n + 1, result_ty);
            let result = local(
                &mut supplier,
                n * 10 + 1,
                LocalValueReference::LexicalBinding(view),
            );
            let field = FieldReference {
                package: supplier.root.package_id,
                field: field_id,
            };
            expression(
                &mut supplier,
                n * 10 + 2,
                ExpressionOperation::BorrowOwnedRecordField {
                    record_type: parent_ty,
                    source,
                    field,
                    binding: view,
                    body: result,
                },
            )
        } else {
            source
        };
        let f = function_mut(&mut supplier, method_scope);
        f.type_parameters = vec![p, q];
        f.parameters = vec![parameter];
        f.result.ty = result_ty;
        f.result.relation = ResultRelation::ReadFrom { source: parameter };
        f.body = body;
        supplier
            .owners
            .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, n + 1)));
        let scheme = insert_declaration(
            &mut supplier,
            n,
            OwnerKind::OwnedImplementation,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![
                    TypeParameterId::migrate(SEED, n + 3),
                    TypeParameterId::migrate(SEED, n + 4),
                ],
                implementation_parameters: vec![],
                contract,
                self_type: parent_ty,
                type_arguments: vec![],
                associated_types: vec![],
                methods: vec![],
            }),
        );
        let (_, sty) = insert_parameter(&mut supplier, scheme, n + 3);
        let (_, uty) = insert_parameter(&mut supplier, scheme, n + 4);
        let nominal = intern(
            &mut supplier,
            TypeForm::Applied {
                declaration: record,
                arguments: vec![sty, uty],
            },
        );
        let output = match selected_field {
            Some(id) if id == a => sty,
            Some(_) => uty,
            None => nominal,
        };
        let i = implementation_mut(&mut supplier, scheme);
        i.self_type = nominal;
        i.associated_types = vec![AssociatedTypeBinding {
            member,
            value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: output },
        }];
        i.methods = vec![OwnedMethodImplementation {
            method,
            function: method_scope,
            type_arguments: vec![sty, uty],
            implementations: vec![],

            read_scope_arguments: Vec::new(),
        }];
        schemes.push(scheme);
    }
    assert!(accepts(&supplier));

    let mut client = empty_snapshot(300);
    import(&mut client, &consumer);
    import(&mut client, &supplier);
    intern(&mut client, TypeForm::I64);
    intern(&mut client, TypeForm::OwnedI64Cell);
    intern(&mut client, TypeForm::ByteBuffer);
    let nominal = intern(
        &mut client,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![cell, buffer],
        },
    );
    let caller = bare_function(&mut client, 300, false, cell);
    let parent = value_parameter(&mut client, caller, 300, nominal, ParameterUse::Borrow);
    let source = local(
        &mut client,
        301,
        LocalValueReference::FunctionParameter(parent),
    );
    let witness = ImplementationOperand::Concrete {
        implementation: schemes[0],
        type_arguments: vec![cell, buffer],
        implementations: vec![],
    };
    let call = call(
        &mut client,
        302,
        generic_read,
        vec![nominal],
        Some(witness),
        vec![source],
    );
    let view = binding(&mut client, 300, cell);
    let result = local(&mut client, 303, LocalValueReference::LexicalBinding(view));
    let borrow = expression(
        &mut client,
        304,
        ExpressionOperation::BorrowCall {
            call,
            binding: view,
            body: result,
        },
    );
    let f = function_mut(&mut client, caller);
    f.parameters = vec![parent];
    f.result.ty = cell;
    f.result.relation = ResultRelation::ReadFrom { source: parent };
    f.body = borrow;
    client
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 300)));
    Vertical {
        consumer,
        supplier,
        client,
        contract,
        generic_read,
        task_read,
        task_read_data,
        task_drop,
        record,
        field,
        schemes,
        nominal,
        cell,
        buffer,
        scalar,
        caller,
        parent,
        call,
        borrow,
        view,
    }
}

#[test]
fn associated_memory_vertical_private_supplier_returns_the_actual_imported_descendant() {
    let f = vertical();
    assert!(f.consumer.dependencies.is_empty());
    assert!(!f.consumer.types.values().any(|object| matches!(
        &object.form,
        TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. }
            if *declaration == f.record
    )));
    assert_eq!(
        f.supplier.dependencies[&f.contract.package],
        f.client.dependencies[&f.contract.package]
    );
    assert!(accepts(&f.consumer));
    assert!(accepts(&f.supplier));
    assert!(accepts(&f.client));
    let oracle = Oracle(&f.client, Some(f.caller));
    assert!(!oracle.record_access(f.nominal));
    let invocation = oracle.associated_invocation(f.call).unwrap();
    assert_eq!(invocation.result, f.cell);
    assert_eq!(invocation.result_borrow, Some(0));
    assert_eq!(invocation.parameters[0].1, f.nominal);
    let mut wrong = f.client.clone();
    let Some(OwnerRecord::Expression(e)) = wrong.owners.get_mut(&OwnerKey::Expression(f.call))
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut e.operation
    else {
        unreachable!()
    };
    *implementations = vec![selected(&f, 1)];
    assert_eq!(
        Oracle(&wrong, Some(f.caller))
            .associated_invocation(f.call)
            .unwrap()
            .result,
        f.buffer
    );
    assert!(!accepts(&wrong));
    let mut private = f.client.clone();
    let source = local(
        &mut private,
        305,
        LocalValueReference::FunctionParameter(f.parent),
    );
    let result = local(
        &mut private,
        306,
        LocalValueReference::LexicalBinding(f.view),
    );
    let Some(OwnerRecord::Expression(e)) = private.owners.get_mut(&OwnerKey::Expression(f.borrow))
    else {
        unreachable!()
    };
    e.operation = ExpressionOperation::BorrowOwnedRecordField {
        record_type: f.nominal,
        source,
        field: f.field,
        binding: f.view,
        body: result,
    };
    private.owners.remove(&OwnerKey::Expression(f.call));
    private
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 301)));
    private
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 303)));
    Resolver::new(&private).admit_canonical_closure().unwrap();
    assert!(!accepts(&private));

    // Property traversal may use the private field inventory. That does not
    // let a stored field template borrow a consumer's lexical member anchor.
    let mut hidden = f.supplier.clone();
    let projected = Oracle(&f.consumer, Some(f.generic_read))
        .function(f.generic_read)
        .unwrap()
        .result
        .ty;
    let Some(OwnerRecord::Field(field)) = hidden.owners.get_mut(&OwnerKey::Field(f.field.field))
    else {
        unreachable!()
    };
    field.ty = projected;
    assert_eq!(
        Resolver::new(&hidden).admit_canonical_closure(),
        Err(Failure::Authority)
    );
    assert!(!accepts(&hidden));
}

#[test]
fn associated_memory_vertical_same_typed_parents_keep_exact_descendant_provenance() {
    let f = vertical();
    let mut source = f.client.clone();
    let second = value_parameter(&mut source, f.caller, 309, f.nominal, ParameterUse::Borrow);
    function_mut(&mut source, f.caller).parameters.push(second);
    let Some(OwnerRecord::Expression(argument)) = source
        .owners
        .get_mut(&OwnerKey::Expression(ExpressionId::migrate(SEED, 301)))
    else {
        unreachable!()
    };
    argument.operation = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(second),
    };
    Resolver::new(&source).admit_canonical_closure().unwrap();
    assert!(!accepts(&source));
    function_mut(&mut source, f.caller).result.relation =
        ResultRelation::ReadFrom { source: second };
    assert!(accepts(&source));
}

struct Group {
    source: KernelSnapshot,
    caller: DeclarationReference,
    group: ExpressionId,
    first: ParameterId,
    second: ParameterId,
    cell: ParameterId,
}

fn group(f: &Vertical, descendant: bool, conflict: bool, reverse: bool) -> Group {
    let mut source = f.client.clone();
    let result = intern(
        &mut source,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: name("left"),
                    ty: f.scalar,
                },
                StructuralTypeField {
                    name: name("right"),
                    ty: f.scalar,
                },
            ],
        },
    );
    let caller = bare_function(&mut source, 310, true, result);
    let first = value_parameter(
        &mut source,
        caller,
        310,
        f.nominal,
        if descendant {
            ParameterUse::Borrow
        } else {
            ParameterUse::Consume
        },
    );
    let second = value_parameter(&mut source, caller, 311, f.nominal, ParameterUse::Consume);
    let cell = value_parameter(&mut source, caller, 312, f.cell, ParameterUse::Borrow);
    function_mut(&mut source, caller).parameters = vec![first, second, cell];
    let view = if descendant {
        Some(binding(&mut source, 310, f.cell))
    } else {
        None
    };
    let mut children = Vec::new();
    for index in 0..2 {
        let value = if descendant {
            LocalValueReference::LexicalBinding(view.unwrap())
        } else if index == 0 {
            LocalValueReference::FunctionParameter(first)
        } else {
            LocalValueReference::FunctionParameter(cell)
        };
        let argument = local(&mut source, 3100 + index * 10, value);
        let mut arguments = vec![argument];
        let target = if !descendant && index == 1 {
            let parent = local(
                &mut source,
                3120,
                LocalValueReference::FunctionParameter(if conflict { first } else { second }),
            );
            let drop = call(
                &mut source,
                3121,
                f.task_drop,
                vec![f.nominal],
                None,
                vec![parent],
            );
            arguments.insert(0, drop);
            f.task_read_data
        } else {
            f.task_read
        };
        let witness = selected(f, if !descendant && index == 0 { 2 } else { 0 });
        children.push(call(
            &mut source,
            3101 + index * 10,
            target,
            vec![f.nominal],
            Some(witness),
            arguments,
        ));
    }
    if reverse {
        children.reverse();
    }
    let group = expression(
        &mut source,
        3130,
        ExpressionOperation::Parallel {
            left: children[0],
            right: children[1],
        },
    );
    let body = if let Some(view) = view {
        let parent = local(
            &mut source,
            3140,
            LocalValueReference::FunctionParameter(first),
        );
        let read = call(
            &mut source,
            3141,
            f.generic_read,
            vec![f.nominal],
            Some(selected(f, 0)),
            vec![parent],
        );
        expression(
            &mut source,
            3142,
            ExpressionOperation::BorrowCall {
                call: read,
                binding: view,
                body: group,
            },
        )
    } else {
        group
    };
    function_mut(&mut source, caller).body = body;
    source
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 310)));
    Group {
        source,
        caller,
        group,
        first,
        second,
        cell,
    }
}

fn group_rights(f: &Group) -> Rights {
    let mut rights = Rights::default();
    for id in [f.first, f.second] {
        let root = LocalValueReference::FunctionParameter(id);
        rights.memory.insert(root);
        rights.owned.insert(root);
    }
    let cell = LocalValueReference::FunctionParameter(f.cell);
    rights.memory.insert(cell);
    rights.borrowed.insert(cell);
    rights
}

#[test]
fn associated_memory_vertical_projected_parallel_reads_keep_descendant_ancestor_loans() {
    let f = vertical();
    for reverse in [false, true] {
        let g = group(&f, true, false, reverse);
        Resolver::new(&g.source).admit_canonical_closure().unwrap();
        assert!(accepts(&g.source));
        let mut rights = group_rights(&g);
        let root = LocalValueReference::FunctionParameter(g.first);
        rights.owned.remove(&root);
        rights.borrowed.insert(root);
        let initial = rights.clone();
        // An imported interface intentionally carries only the signature. The
        // group body belongs to this exact local canonical function owner.
        assert_eq!(g.caller.package, g.source.root.package_id);
        let OwnerRecord::Declaration(declaration) =
            &g.source.owners[&OwnerKey::Declaration(g.caller.declaration)]
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &declaration.payload else {
            unreachable!()
        };
        let body = function.body;
        assert_eq!(
            Oracle(&g.source, Some(g.caller)).flow(body, &mut rights, Demand::Data, 0),
            Some(false)
        );
        assert!(rights == initial);
    }
}

#[test]
fn associated_memory_vertical_projected_parallel_rejects_nested_parent_consume_in_both_orders() {
    let f = vertical();
    for reverse in [false, true] {
        let positive = group(&f, false, false, reverse);
        assert!(accepts(&positive.source));
        let mut rights = group_rights(&positive);
        assert_eq!(
            Oracle(&positive.source, Some(positive.caller)).flow(
                positive.group,
                &mut rights,
                Demand::Data,
                0
            ),
            Some(false)
        );
        assert!(
            rights
                .owned
                .contains(&LocalValueReference::FunctionParameter(positive.first))
        );
        assert!(
            !rights
                .owned
                .contains(&LocalValueReference::FunctionParameter(positive.second))
        );
        assert!(rights.loans.is_empty());
        let negative = group(&f, false, true, reverse);
        Resolver::new(&negative.source)
            .admit_canonical_closure()
            .unwrap();
        assert!(!accepts(&negative.source));
        let mut refused = group_rights(&negative);
        let initial = refused.clone();
        assert_eq!(
            Oracle(&negative.source, Some(negative.caller)).flow(
                negative.group,
                &mut refused,
                Demand::Data,
                0
            ),
            None
        );
        assert!(refused == initial);
        // The same drop, admitted outside a live group read footprint, is
        // allowed. The rejected candidate changes only which parent it drops.
        let mut outside = group_rights(&negative);
        assert_eq!(
            Oracle(&negative.source, Some(negative.caller)).flow(
                ExpressionId::migrate(SEED, 3121),
                &mut outside,
                Demand::Data,
                0
            ),
            Some(false)
        );
        assert!(
            !outside
                .owned
                .contains(&LocalValueReference::FunctionParameter(negative.first))
        );
        assert!(outside.loans.is_empty());
    }
}

#[test]
fn associated_memory_vertical_imported_closure_capacity_is_shared_and_has_a_fresh_retry() {
    let f = vertical();
    let g = group(&f, true, false, false);
    let source = g.source.clone();
    let mut healthy = Resolver::new(&g.source);
    healthy.admit_canonical_closure().unwrap();
    let entries = healthy.retained_entries();
    let work = contract::MAXIMUM_VALIDATION_WORK - healthy.remaining();
    assert!(entries > g.source.owners.len());
    assert!(work > g.source.owners.len());
    for limits in [
        Limits {
            work: contract::MAXIMUM_VALIDATION_WORK,
            objects: entries - 1,
        },
        Limits {
            work: work - 1,
            objects: entries,
        },
    ] {
        assert_eq!(
            Resolver::with_limits(&g.source, limits).admit_canonical_closure(),
            Err(Failure::Capacity)
        );
    }
    Resolver::with_limits(
        &g.source,
        Limits {
            work,
            objects: entries,
        },
    )
    .admit_canonical_closure()
    .unwrap();
    assert_eq!(g.source.owners, source.owners);
    assert_eq!(g.source.types, source.types);
    assert_eq!(g.source.dependencies, source.dependencies);
    assert_eq!(g.source.dependency_interfaces, source.dependency_interfaces);
    assert!(accepts(&g.source));
}
