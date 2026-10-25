// Included by the source-loader owner; mutations target private meaning and keep
// original public commitments while rehashing every enclosing source identity.

#[test]
fn prerequisite_transport_rejects_rehashed_unused_mappings_and_nested_applications() {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        crate::platform::compiler::tests::prerequisite_implementation_admission_tests::SOURCE,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let original = GraphRepository::create(
        &directory.path().join("prerequisite-source"),
        &snapshot,
        None,
    )
    .unwrap()
    .repository
    .export_package_container()
    .unwrap();
    let accepted =
        crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
    assert_eq!(
        accepted.snapshots[&snapshot.root.package_id].owners,
        snapshot.owners
    );
    let scheme = borrowed_transport_declaration(&snapshot, "Delegating");
    let scope = match &scheme {
        OwnerRecord::Declaration(d) => DeclarationReference {
            package: snapshot.root.package_id,
            declaration: match d.header.owner {
                OwnerKey::Declaration(id) => id,
                _ => unreachable!(),
            },
        },
        _ => unreachable!(),
    };
    let mut attacks = Vec::new();
    for fault in [
        "mapping-missing",
        "mapping-order",
        "prerequisite-self",
        "parameter-scope",
        "parameter-identity",
    ] {
        let mut replacement = scheme.clone();
        let OwnerRecord::Declaration(d) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
            unreachable!()
        };
        match fault {
            "mapping-missing" => {
                i.methods[0].implementations.pop();
            }
            "mapping-order" => i.methods[0].implementations.swap(0, 1),
            "prerequisite-self" => {
                i.implementation_parameters[0].self_type = *snapshot
                    .types
                    .iter()
                    .find_map(|(ty, object)| {
                        matches!(object.form, TypeForm::OwnedI64Cell).then_some(ty)
                    })
                    .unwrap()
            }
            "parameter-scope" => {
                let ImplementationOperand::Parameter { scope, .. } =
                    &mut i.methods[0].implementations[0]
                else {
                    unreachable!()
                };
                let OwnerRecord::Declaration(delegate) =
                    borrowed_transport_declaration(&snapshot, "delegate")
                else {
                    unreachable!()
                };
                let OwnerKey::Declaration(declaration) = delegate.header.owner else {
                    unreachable!()
                };
                scope.declaration = declaration;
            }
            "parameter-identity" => {
                let ImplementationOperand::Parameter { parameter, .. } =
                    &mut i.methods[0].implementations[0]
                else {
                    unreachable!()
                };
                *parameter =
                    crate::platform::semantic_id::ImplementationParameterId::from_bytes([0xed; 16])
                        .unwrap();
            }
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    let expression = snapshot.owners.values().find(|owner| matches!(owner,
        OwnerRecord::Expression(record) if matches!(&record.operation, ExpressionOperation::MethodCall { witness: ImplementationOperand::Concrete { implementation, implementations, .. }, .. }
            if *implementation == scope && !implementations.is_empty())
    )).unwrap().clone();
    let unused = borrowed_transport_declaration(&snapshot, "UnusedNested");
    for fault in [
        "unused-nested-missing",
        "unused-nested-order",
        "unused-nested-phantom",
    ] {
        let mut replacement = unused.clone();
        let OwnerRecord::Declaration(d) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
            unreachable!()
        };
        let ImplementationOperand::Concrete {
            implementations,
            type_arguments,
            ..
        } = &mut i.methods[0].implementations[0]
        else {
            unreachable!()
        };
        match fault {
            "unused-nested-missing" => {
                implementations.pop();
            }
            "unused-nested-order" => implementations.swap(0, 1),
            "unused-nested-phantom" => type_arguments.swap(0, 1),
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    for fault in ["nested-missing", "nested-order", "nested-phantom"] {
        let mut replacement = expression.clone();
        let OwnerRecord::Expression(record) = &mut replacement else {
            unreachable!()
        };
        let ExpressionOperation::MethodCall {
            witness:
                ImplementationOperand::Concrete {
                    implementations,
                    type_arguments,
                    ..
                },
            ..
        } = &mut record.operation
        else {
            unreachable!()
        };
        match fault {
            "nested-missing" => {
                implementations.pop();
            }
            "nested-order" => implementations.swap(0, 1),
            "nested-phantom" => type_arguments.swap(0, 1),
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    for (fault, replacement) in attacks {
        let (digest, bytes) = encode_owner(&replacement).unwrap();
        assert_eq!(
            decode_owner(&bytes, replacement.owner(), replacement.kind(), digest).unwrap(),
            replacement
        );
        let mut invalid = snapshot.clone();
        invalid
            .owners
            .insert(replacement.owner(), replacement.clone());
        assert!(
            !crate::platform::kernel::memory_reference::accepts(&invalid),
            "memory accepted {fault}"
        );
        let hostile = rehash_owner(&original, replacement);
        let bytes = hostile.encode().unwrap();
        let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
        let error = decoded.admit().unwrap_err();
        assert!(
            error.code == "kernel_owned_contract"
                || error.message.contains("kernel_owned_contract"),
            "{fault}: {error:?}"
        );
        let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
        assert!(
            error.message.contains("kernel_owned_contract"),
            "{fault}: {error:?}"
        );
        assert_not_ready(&bytes, hostile.root.transport);
    }
}

const PRIVATE_SCOPED_MAPPING_SOURCE: &str = r#"declarations.begin
(units (module create private-scoped-mapping
  (read-view create Zero (visibility private)
    (scope-parameter read_scope_d8300000000000000000000000000001 s))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (associated-type assoc_type_d8100000000000000000000000000001 View
      (read-view-family (scope-parameter read_scope_d8100000000000000000000000000001 s)))
    (method method_d8100000000000000000000000000001 make
      (parameters (Self borrow) (Self borrow))
      (returns (associated-read-view-member Reader assoc_type_d8100000000000000000000000000001
        (read-scope (contract-method Reader method_d8100000000000000000000000000001) read_scope_d8100000000000000000000000000011))
        (view-from 0 (read-scope (contract-method Reader method_d8100000000000000000000000000001) read_scope_d8100000000000000000000000000011)))
      (effect pure)
      (scope-parameters (scope-parameter read_scope_d8100000000000000000000000000011 s)
        (scope-parameter read_scope_d8100000000000000000000000000012 unused))
      (scope-sources (scope-source 0 (read-scope (contract-method Reader method_d8100000000000000000000000000001) read_scope_d8100000000000000000000000000011))
        (scope-source 1 (read-scope (contract-method Reader method_d8100000000000000000000000000001) read_scope_d8100000000000000000000000000011))))
    (method method_d8100000000000000000000000000002 observe
      (parameters (Self borrow) (Self borrow)) (returns Unit (value)) (effect pure)
      (scope-parameters (scope-parameter read_scope_d8100000000000000000000000000021 s)
        (scope-parameter read_scope_d8100000000000000000000000000022 unused))
      (scope-sources (scope-source 0 (read-scope (contract-method Reader method_d8100000000000000000000000000002) read_scope_d8100000000000000000000000000021))
        (scope-source 1 (read-scope (contract-method Reader method_d8100000000000000000000000000002) read_scope_d8100000000000000000000000000021)))))
  (function create make-private (visibility private) (effect pure)
    (scope-parameter read_scope_d8300000000000000000000000000011 s)
    (scope-parameter read_scope_d8300000000000000000000000000012 unused)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (scope-source a (read-scope (declaration make-private) read_scope_d8300000000000000000000000000011))
    (scope-source b (read-scope (declaration make-private) read_scope_d8300000000000000000000000000011))
    (returns (read-view-application Zero (types) (read-scope (declaration make-private) read_scope_d8300000000000000000000000000011))
      (view-from a (read-scope (declaration make-private) read_scope_d8300000000000000000000000000011)))
    (body (view-create Zero (types) (scope (read-scope (declaration make-private) read_scope_d8300000000000000000000000000011)) (source a))))
  (function create observe-private (visibility private) (effect pure)
    (scope-parameter read_scope_d8300000000000000000000000000021 s)
    (scope-parameter read_scope_d8300000000000000000000000000022 unused)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (scope-source a (read-scope (declaration observe-private) read_scope_d8300000000000000000000000000021))
    (scope-source b (read-scope (declaration observe-private) read_scope_d8300000000000000000000000000021))
    (returns Unit (value)) (body (unit)))
  (owned-implementation create CellReader (visibility public)
    (contract Reader) (self OwnedI64Cell)
    (associated-type assoc_type_d8100000000000000000000000000001
      (read-view-family (scope-parameter read_scope_d8300000000000000000000000000031 s)
        (read-view-application Zero (types)
          (read-scope (associated-binding CellReader assoc_type_d8100000000000000000000000000001) read_scope_d8300000000000000000000000000031))))
    (method method_d8100000000000000000000000000001 make-private
      (scopes (read-scope (contract-method Reader method_d8100000000000000000000000000001) read_scope_d8100000000000000000000000000011)
        (read-scope (contract-method Reader method_d8100000000000000000000000000001) read_scope_d8100000000000000000000000000012)))
    (method method_d8100000000000000000000000000002 observe-private
      (scopes (read-scope (contract-method Reader method_d8100000000000000000000000000002) read_scope_d8100000000000000000000000000021)
        (read-scope (contract-method Reader method_d8100000000000000000000000000002) read_scope_d8100000000000000000000000000022))))))
declarations.end"#;

fn private_scoped_mapping_transport() -> AdmittedClosure {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        PRIVATE_SCOPED_MAPPING_SOURCE,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    GraphRepository::create(
        &directory.path().join("private-scoped-source"),
        &snapshot,
        None,
    )
    .unwrap()
    .repository
    .export_package_container()
    .unwrap()
}

#[test]
fn private_scoped_mapping_transport_admits_hidden_targets_only_after_complete_source_proof() {
    let original = private_scoped_mapping_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let implementation = borrowed_transport_declaration(&package.snapshot, "CellReader");
    assert!(
        package
            .interface_owners
            .contains_key(&implementation.owner())
    );
    for name in ["make-private", "observe-private"] {
        let target = borrowed_transport_declaration(&package.snapshot, name);
        let OwnerRecord::Declaration(record) = &target else {
            unreachable!()
        };
        assert_eq!(record.visibility, DeclarationVisibility::Private);
        assert!(!package.interface_owners.contains_key(&target.owner()));
        let DeclarationPayload::Function(function) = &record.payload else {
            unreachable!()
        };
        for parameter in &function.parameters {
            assert!(
                !package
                    .interface_owners
                    .contains_key(&OwnerKey::Parameter(*parameter))
            );
        }
    }
    let exact = original.container.encode().unwrap();
    let decoded = PackageContainer::decode(&exact, original.container.root.transport).unwrap();
    let admitted = decoded.admit().unwrap();
    assert_eq!(admitted.container.encode().unwrap(), exact);
    assert_eq!(
        admitted.packages[&decoded.root.package_revision]
            .snapshot
            .owners,
        package.snapshot.owners
    );
    let independent = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap();
    assert_eq!(
        independent.snapshots[&package.revision.package].owners,
        package.snapshot.owners
    );
    let directory = tempfile::tempdir().unwrap();
    let target = GraphRepository::create(
        &directory.path().join("consumer"),
        &crate::platform::kernel::tests::transport_snapshot(),
        None,
    )
    .unwrap();
    let staged = target
        .repository
        .stage_package_transport(decoded.root.transport, &exact)
        .unwrap();
    assert_eq!(staged.package_revision, decoded.root.package_revision);

    // This consumer uses only the original fully admitted supplier projection.
    // A DependencyInterface is a relative oracle input, never a body certificate;
    // the retained container and supplier-local admission above discharge it.
    let supplier = &admitted.packages[&decoded.root.package_revision];
    let mut consumer = target
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert_ne!(consumer.root.package_id, supplier.revision.package);
    assert!(consumer.dependencies.is_empty());
    let dependency = DependencyRecord {
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        package: supplier.revision.package,
        semantic_revision: supplier.revision.revision.revision_id().unwrap(),
        package_revision: supplier.binding.package_revision,
    };
    consumer
        .dependencies
        .insert(dependency.package, dependency.clone());
    consumer
        .representation_dependencies
        .insert(dependency.package, dependency);
    consumer.dependency_interfaces.insert(
        supplier.binding.package_revision,
        supplier
            .interface_owners
            .iter()
            .map(|(key, value)| (*key, value.record.clone()))
            .collect::<BTreeMap<_, _>>()
            .into(),
    );
    for (digest, bytes) in &supplier.interface_types {
        let object = decode_type_object(bytes, *digest).unwrap();
        assert_eq!(object, supplier.snapshot.types[digest]);
        assert!(consumer.dependency_types.insert(*digest, object).is_none());
    }
    let OwnerRecord::Declaration(record) = &implementation else {
        unreachable!()
    };
    let OwnerKey::Declaration(id) = record.header.owner else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(definition) = &record.payload else {
        unreachable!()
    };
    let reference = DeclarationReference {
        package: supplier.revision.package,
        declaration: id,
    };
    let concrete = ImplementationOperand::Concrete {
        implementation: reference,
        type_arguments: Vec::new(),
        implementations: Vec::new(),
    };
    let witness = crate::platform::kernel::witness_contract(&consumer, &concrete, None).expect(
        "consumer's exact imported public implementation must admit its hidden unused maps",
    );
    assert_eq!(witness.contract, definition.contract);
    assert_eq!(witness.self_type, definition.self_type);
    assert!(witness.type_arguments.is_empty());
    crate::platform::kernel::validate_implementation_at(&consumer, reference, definition).unwrap();
    assert!(!consumer.owners.contains_key(&OwnerKey::Declaration(
        definition.methods[0].function.declaration
    )));
    for mapping in &definition.methods {
        assert_eq!(mapping.function.package, reference.package);
        assert!(
            !supplier
                .interface_owners
                .contains_key(&OwnerKey::Declaration(mapping.function.declaration))
        );
        assert!(
            crate::platform::kernel::optional_function_contract(&consumer, mapping.function)
                .unwrap()
                .is_none()
        );
        let error =
            crate::platform::kernel::function_contract(&consumer, mapping.function).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "kernel_owned_contract");
        assert_eq!(
            error.message,
            "method target must be an exact visible graph function"
        );
    }
    // Actual direct-call inference sees the exact dependency edge but cannot
    // promote the mapped private function into public naming. Its two borrowed
    // arguments and complete source/formal inventory are otherwise coherent.
    let observer = borrowed_transport_declaration(&supplier.snapshot, "observe-private");
    let OwnerRecord::Declaration(mut caller) = observer else {
        unreachable!()
    };
    let OwnerKey::Declaration(caller_id) = caller.header.owner else {
        unreachable!()
    };
    caller.module = *consumer
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Module(module) => Some(module),
            _ => None,
        })
        .unwrap();
    caller.name = Name::new("private-call-control").unwrap();
    let DeclarationPayload::Function(function) = &mut caller.payload else {
        unreachable!()
    };
    let actual_scopes = function
        .read_scope_parameters
        .iter()
        .map(|parameter| {
            ReadScopeArgument::Parameter(ReadScopeReference {
                binder: ReadScopeBinder::Declaration(DeclarationReference {
                    package: consumer.root.package_id,
                    declaration: caller_id,
                }),
                parameter: parameter.id,
            })
        })
        .collect::<Vec<_>>();
    for source in &mut function.read_scope_sources {
        let ReadScopeArgument::Parameter(scope) = &mut source.scope else {
            unreachable!()
        };
        scope.binder = ReadScopeBinder::Declaration(DeclarationReference {
            package: consumer.root.package_id,
            declaration: caller_id,
        });
    }
    let mut arguments = Vec::new();
    for (index, parameter) in function.parameters.iter().enumerate() {
        let record = supplier.snapshot.owners[&OwnerKey::Parameter(*parameter)].clone();
        let OwnerRecord::Parameter(source_parameter) = &record else {
            unreachable!()
        };
        assert!(matches!(
            supplier.snapshot.types[&source_parameter.ty].form,
            TypeForm::OwnedI64Cell
        ));
        assert_eq!(
            consumer.dependency_types[&source_parameter.ty],
            supplier.snapshot.types[&source_parameter.ty]
        );
        record.validate_local().unwrap();
        assert!(
            consumer
                .owners
                .insert(OwnerKey::Parameter(*parameter), record)
                .is_none()
        );
        let expression = crate::platform::semantic_id::ExpressionId::migrate(
            b"imported-private-call-control",
            index as u64,
        );
        assert!(
            consumer
                .owners
                .insert(
                    OwnerKey::Expression(expression),
                    OwnerRecord::Expression(
                        ExpressionRecord::new(
                            expression,
                            ExpressionOperation::Local {
                                value: LocalValueReference::FunctionParameter(*parameter)
                            }
                        )
                        .unwrap()
                    )
                )
                .is_none()
        );
        arguments.push(expression);
    }
    let direct =
        crate::platform::semantic_id::ExpressionId::migrate(b"imported-private-call-control", 2);
    let callee = definition.methods[1].function;
    assert_eq!(callee.declaration, caller_id);
    function.body = direct;
    let effect = function.effect.clone();
    let call = ExpressionRecord::new(
        direct,
        ExpressionOperation::Call {
            function: callee,
            type_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            read_scope_arguments: actual_scopes,
            arguments,
        },
    )
    .unwrap();
    assert!(
        consumer
            .owners
            .insert(
                OwnerKey::Declaration(caller_id),
                OwnerRecord::Declaration(caller)
            )
            .is_none()
    );
    assert!(
        consumer
            .owners
            .insert(OwnerKey::Expression(direct), OwnerRecord::Expression(call))
            .is_none()
    );
    let mut work = 0;
    let error = crate::platform::kernel::test_work::infer_function_expression_type(
        &consumer,
        caller_id,
        direct,
        &effect,
        &mut work,
        contract::MAXIMUM_VALIDATION_WORK,
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "kernel_type_dependency_owner_missing");
    assert!(
        error
            .message
            .contains("absent from exact dependency interface")
    );
    // A caller-supplied descriptor cannot reuse the genuine provider's absent
    // target obligation for different mapping bytes under the same exact owner.
    let mut forged = definition.clone();
    forged.methods[0].function = forged.methods[1].function;
    forged.validate_local().unwrap();
    let error = crate::platform::kernel::validate_implementation_at(&consumer, reference, &forged)
        .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "kernel_owned_contract");
    assert_eq!(
        error.message,
        "imported implementation descriptor differs from its exact selected inventory"
    );
    // Neither rejection changes the retained source proof or private naming.
    assert!(crate::platform::kernel::witness_contract(&consumer, &concrete, None).is_ok());
}

#[test]
fn private_scoped_mapping_transport_rejects_rehashed_target_contracts_before_readiness() {
    let original = private_scoped_mapping_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let snapshot = &package.snapshot;
    let make = borrowed_transport_declaration(snapshot, "make-private");
    let observer = borrowed_transport_declaration(snapshot, "observe-private");
    let implementation = borrowed_transport_declaration(snapshot, "CellReader");
    let mut attacks = Vec::new();
    for fault in ["unused-formal-missing", "result", "source"] {
        let mut replacement = make.clone();
        let OwnerRecord::Declaration(record) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut record.payload else {
            unreachable!()
        };
        match fault {
            "unused-formal-missing" => {
                function.read_scope_parameters.pop();
            }
            "result" => function.result.relation = ResultRelation::Value,
            "source" => {
                function.result.relation = ResultRelation::ViewFrom {
                    source: function.parameters[1],
                    scope: function.read_scope_sources[0].scope,
                }
            }
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    let mut wrong_kind = observer.clone();
    let OwnerRecord::Declaration(record) = &mut wrong_kind else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        unreachable!()
    };
    record.payload = DeclarationPayload::Constant {
        ty: function.result.ty,
        value: function.body,
    };
    record.header.kind = OwnerKind::Constant;
    attacks.push(("wrong-kind", wrong_kind));
    let mut effect = observer;
    let OwnerRecord::Declaration(record) = &mut effect else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Task {
        requirements: Vec::new(),
        effect_parameters: Vec::new(),
    };
    record.header.kind = OwnerKind::TaskFunction;
    attacks.push(("effect", effect));
    for fault in ["missing-target", "scope-actual-order"] {
        let mut replacement = implementation.clone();
        let OwnerRecord::Declaration(record) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(implementation) = &mut record.payload else {
            unreachable!()
        };
        if fault == "missing-target" {
            implementation.methods[0].function.declaration =
                crate::platform::semantic_id::DeclarationId::migrate(
                    b"missing-private-mapped-function",
                    0,
                );
        } else {
            implementation.methods[0].read_scope_arguments.swap(0, 1);
        }
        attacks.push((fault, replacement));
    }
    for (fault, replacement) in attacks {
        // All mutated owners remain locally decodable and every enclosing source
        // commitment is rehashed. Reduced public metadata cannot discharge them.
        let (digest, bytes) = encode_owner(&replacement).unwrap();
        assert_eq!(
            decode_owner(&bytes, replacement.owner(), replacement.kind(), digest).unwrap(),
            replacement
        );
        let hostile = rehash_owner(&original, replacement);
        let exact = hostile.encode().unwrap();
        let decoded = PackageContainer::decode(&exact, hostile.root.transport).unwrap();
        let error = decoded.admit().unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic, "{fault}: {error:?}");
        assert!(
            crate::platform::package_transport::oracle::reconstruct(&decoded).is_err(),
            "independent source accepted {fault}"
        );
        assert_not_ready(&exact, decoded.root.transport);
    }
    // Refusal is local to the invalid attempt, with no proof or readiness poison.
    assert!(original.container.admit().is_ok());
}

fn assert_indexed_private_representation_source(
    input: &str,
    private_names: &[&str],
    only_public_kind: OwnerKind,
) {
    let snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(input)
            .unwrap();
    let public_representation_roots = snapshot
        .owners
        .values()
        .filter_map(|owner| match owner {
            OwnerRecord::Declaration(record)
                if record.visibility == DeclarationVisibility::Public
                    && matches!(
                        record.header.kind,
                        OwnerKind::OwnedRecord
                            | OwnerKind::ReadView
                            | OwnerKind::OwnedImplementation
                    ) =>
            {
                Some(record.header.kind)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(public_representation_roots, vec![only_public_kind]);
    let directory = tempfile::tempdir().unwrap();
    let original = GraphRepository::create(
        &directory.path().join("indexed-representation"),
        &snapshot,
        None,
    )
    .unwrap()
    .repository
    .export_package_container()
    .unwrap();
    let package = &original.packages[&original.container.root.package_revision];
    assert_eq!(package.snapshot.owners, snapshot.owners);
    assert_eq!(package.snapshot.types, snapshot.types);
    let bytes = original.container.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, original.container.root.transport).unwrap();
    let admitted = decoded.admit().unwrap();
    assert_eq!(admitted.container.encode().unwrap(), bytes);
    let source_package = &admitted.packages[&decoded.root.package_revision];
    assert_eq!(source_package.interface_owners, package.interface_owners);
    let independent = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap();
    assert_eq!(
        independent.snapshots[&snapshot.root.package_id].owners,
        snapshot.owners
    );
    for name in private_names {
        let owner = borrowed_transport_declaration(&snapshot, name);
        let OwnerRecord::Declaration(canonical) = &owner else {
            unreachable!()
        };
        assert_eq!(canonical.visibility, DeclarationVisibility::Private);
        assert!(matches!(
            package.interface_owners[&owner.owner()].record,
            PackageInterfaceRecord::RepresentationDeclaration(_)
        ));
        assert!(matches!(
            independent.interfaces[&snapshot.root.package_id][&owner.owner()],
            PackageInterfaceRecord::RepresentationDeclaration(_)
        ));
    }
}

#[test]
fn indexed_export_retains_private_owned_referents_of_public_read_views_without_public_owned_records()
 {
    assert_indexed_private_representation_source(
        r#"declarations.begin
(units (module create public-view-representation
  (owned-record create HiddenOwned (visibility private)
    (field create value (type I64)))
  (read-view create PublicView (visibility public)
    (scope-parameter read_scope_e1300000000000000000000000000001 s)
    (view-field field_e1300000000000000000000000000001 source (type HiddenOwned)
      (read-reference (read-scope (declaration PublicView) read_scope_e1300000000000000000000000000001))))))
declarations.end"#,
        &["HiddenOwned"],
        OwnerKind::ReadView,
    );
}

#[test]
fn indexed_export_retains_private_view_family_and_refuses_private_ordinary_implementation_types() {
    // A public observer keeps the representation proof independent from omitted
    // private mapped targets. The unused private family remains a complete root.
    assert_indexed_private_representation_source(
        r#"declarations.begin
(units (module create public-implementation-view-representation
  (read-view create HiddenSpan (visibility private)
    (scope-parameter read_scope_e1300000000000000000000000000011 s))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (associated-type assoc_type_e1300000000000000000000000000001 View
      (read-view-family (scope-parameter read_scope_e1300000000000000000000000000012 s)))
    (method method_e1300000000000000000000000000001 observe
      (parameters (Self borrow)) (returns Unit (value)) (effect pure)))
  (function create observe-public (visibility public) (effect pure)
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns Unit (value)) (body (unit)))
  (owned-implementation create VisibleReader (visibility public)
    (contract Reader) (self OwnedI64Cell)
    (associated-type assoc_type_e1300000000000000000000000000001
      (read-view-family (scope-parameter read_scope_e1300000000000000000000000000013 s)
        (read-view-application HiddenSpan (types)
          (read-scope (associated-binding VisibleReader assoc_type_e1300000000000000000000000000001) read_scope_e1300000000000000000000000000013))))
    (method method_e1300000000000000000000000000001 observe-public))))
declarations.end"#,
        &["HiddenSpan"],
        OwnerKind::OwnedImplementation,
    );
    // Ordinary private nominal types remain unavailable as public implementation
    // signatures. Isolate Self from an Owned RHS using intrinsic Self in the latter.
    for (fault, input) in [
        (
            "private-self",
            r#"declarations.begin
(units (module create private-implementation-self-refusal
  (owned-record create HiddenSelf (visibility private)
    (field create value (type I64)))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_e1300000000000000000000000000001 observe
      (parameters (Self borrow)) (returns Unit (value)) (effect pure)))
  (function create observe-private (visibility private) (effect pure)
    (parameter create source (type HiddenSelf) (use borrow))
    (returns Unit (value)) (body (unit)))
  (owned-implementation create VisibleReader (visibility public)
    (contract Reader) (self HiddenSelf)
    (method method_e1300000000000000000000000000001 observe-private))))
declarations.end"#,
        ),
        (
            "private-owned-rhs",
            r#"declarations.begin
(units (module create private-implementation-owned-rhs-refusal
  (owned-record create HiddenBlock (visibility private)
    (field create value (type I64)))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (associated-type assoc_type_e1300000000000000000000000000001 Block (bound owned))
    (method method_e1300000000000000000000000000001 observe
      (parameters (Self borrow)) (returns Unit (value)) (effect pure)))
  (function create observe-private (visibility private) (effect pure)
    (parameter create source (type OwnedI64Cell) (use borrow))
    (returns Unit (value)) (body (unit)))
  (owned-implementation create VisibleReader (visibility public)
    (contract Reader) (self OwnedI64Cell)
    (associated-type assoc_type_e1300000000000000000000000000001 HiddenBlock)
    (method method_e1300000000000000000000000000001 observe-private))))
declarations.end"#,
        ),
    ] {
        let snapshot =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(input)
                .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let error = GraphRepository::create(&directory.path().join(fault), &snapshot, None)
            .unwrap()
            .repository
            .export_package_container()
            .unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic, "{fault}: {error:?}");
        assert_eq!(
            error.code, "package_interface_private_signature",
            "{fault}: {error:?}"
        );
    }
}
