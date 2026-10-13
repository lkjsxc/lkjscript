//! Exact static admission for lexical structured child calls.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::DeclarationId;
use crate::platform::semantic_id::ExpressionId;

pub(crate) struct ParallelCall {
    pub function: DeclarationReference,
    pub parameters: Vec<ParameterRecord>,
    pub arguments: Vec<ExpressionId>,
    pub result: TypeObjectDigest,
    pub result_owned: bool,
    pub types: std::collections::BTreeMap<TypeObjectDigest, TypeObject>,
}

pub(super) fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_parallel_call", message)
}

/// The child is a call in accepted meaning, not an arbitrary expression or a
/// callable value. This never grants a capability or changes ownership domains.
pub(crate) fn admit_call(
    read: &(impl ExpressionRead + ?Sized),
    expression: ExpressionId,
    scope: Option<DeclarationId>,
) -> Result<ParallelCall, Diagnostic> {
    admit_call_at(
        read,
        expression,
        scope,
        &super::owned_associated::OwnedAssociatedAdmission::new(),
    )
}

pub(crate) fn admit_call_at(
    read: &(impl ExpressionRead + ?Sized),
    expression: ExpressionId,
    scope: Option<DeclarationId>,
    admission: &super::owned_associated::OwnedAssociatedAdmission,
) -> Result<ParallelCall, Diagnostic> {
    let loans = std::collections::BTreeSet::new();
    let caller = scope.map(|declaration| DeclarationReference {
        package: read.package_id(),
        declaration,
    });
    admit_call_at_with_scope_authority(
        read,
        expression,
        scope,
        super::owned_associated::read_view_admission::ScopeAuthority {
            binder: caller.map(ReadScopeBinder::Declaration),
            lexical: caller,
            live_loans: &loans,
        },
        admission,
    )
}

pub(crate) fn admit_call_at_with_scope_authority(
    read: &(impl ExpressionRead + ?Sized),
    expression: ExpressionId,
    scope: Option<DeclarationId>,
    scope_authority: super::owned_associated::read_view_admission::ScopeAuthority<'_>,
    admission: &super::owned_associated::OwnedAssociatedAdmission,
) -> Result<ParallelCall, Diagnostic> {
    read.validation_work()?;
    let caller = scope.map(|declaration| DeclarationReference {
        package: read.package_id(),
        declaration,
    });
    if scope_authority.binder != caller.map(ReadScopeBinder::Declaration)
        || scope_authority.lexical != caller
    {
        // A retained pre-view function has ordinary declaration scope but no
        // read-scope binder. Re-read that exact owner before accepting an empty
        // authority; Graph 34 and every nonempty/foreign authority stay strict.
        let legacy_empty = if scope_authority.binder.is_none()
            && scope_authority.lexical.is_none()
            && scope_authority.live_loans.is_empty()
            && let Some(caller) = caller
        {
            read.validation_work()?;
            match read.owner(OwnerKey::Declaration(caller.declaration))? {
                Some(OwnerRecord::Declaration(parent))
                    if parent.header.owner == OwnerKey::Declaration(caller.declaration)
                        && contract::supported_graph_contract(parent.header.contract_version)
                        && parent.header.contract_version
                            < contract::READ_VIEW_GRAPH_CONTRACT_VERSION =>
                {
                    match parent.payload {
                        DeclarationPayload::Function(function) => {
                            let exact_kind = match function.effect {
                                FunctionEffect::Pure => {
                                    parent.header.kind == OwnerKind::PureFunction
                                }
                                FunctionEffect::Task { .. } => {
                                    parent.header.kind == OwnerKind::TaskFunction
                                }
                            };
                            exact_kind
                                && function.read_scope_parameters.is_empty()
                                && function.read_scope_sources.is_empty()
                                && !matches!(
                                    function.result.relation,
                                    ResultRelation::ViewFrom { .. }
                                )
                        }
                        _ => false,
                    }
                }
                _ => false,
            }
        } else {
            false
        };
        if !legacy_empty {
            return Err(reject(
                "parallel read-scope authority differs from its exact containing callable",
            ));
        }
    }
    let Some(OwnerRecord::Expression(record)) = read.owner(OwnerKey::Expression(expression))?
    else {
        return Err(reject("parallel child expression is absent"));
    };
    let occurrence = super::owned_associated::OwnedTypeOccurrence::authored(
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
        record.contract_version,
    );
    let (function, arguments, type_arguments, implementations, read_scope_arguments) =
        match record.operation {
            ExpressionOperation::Call {
                function,
                arguments,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
            } if effect_arguments.is_empty() && requirement_arguments.is_empty() => (
                function,
                arguments,
                type_arguments,
                Vec::new(),
                read_scope_arguments,
            ),
            ExpressionOperation::ImplementationCall {
                function,
                arguments,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                implementations,
                read_scope_arguments,
            } if effect_arguments.is_empty() && requirement_arguments.is_empty() => (
                function,
                arguments,
                type_arguments,
                implementations,
                read_scope_arguments,
            ),
            _ => {
                return Err(reject(
                    "parallel children require direct named calls with exact scoped applications",
                ));
            }
        };
    let signature = super::owned_contract::function_contract(read, function).map_err(|error| {
        if error.class == DiagnosticClass::Semantic {
            reject("parallel child must name an exact graph function")
        } else {
            error
        }
    })?;
    if !signature.requirement_parameters.is_empty()
        || !signature.effect_parameters.is_empty()
        || !matches!(signature.result.relation, ResultRelation::Value)
        || !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters }
            if requirements.is_empty() && effect_parameters.is_empty())
    {
        return Err(reject(
            "parallel children require tasks with closed empty effect rows",
        ));
    }
    if signature.type_parameters.len() != type_arguments.len()
        || signature.implementation_parameters.len() != implementations.len()
        || signature.read_scope_parameters.len() != read_scope_arguments.len()
    {
        return Err(reject(
            "parallel child applications require exact type and implementation arities",
        ));
    }
    let mut substitutions = std::collections::BTreeMap::new();
    for (id, ty) in signature.type_parameters.iter().zip(&type_arguments) {
        read.validation_work()?;
        let key = OwnerKey::TypeParameter(*id);
        let parameter = if function.package == read.package_id() {
            match read.owner(key)? {
                Some(OwnerRecord::TypeParameter(p)) => p,
                _ => return Err(reject("missing child type parameter")),
            }
        } else {
            match read.package_interface_owner(function.package, key)? {
                Some(PackageInterfaceRecord::TypeParameter(p)) => p,
                _ => return Err(reject("missing imported child type parameter")),
            }
        };
        // Type operands describe the exact application. Only its declared
        // constraints are obligations; transport modes belong to value carriers.
        let owned = super::memory::direct_at(read, *ty, occurrence, admission)?;
        if super::owned_associated::read_view_admission::contains(read, *ty, admission)?
            || parameter.header.owner != key
            || parameter.header.kind != OwnerKind::TypeParameter
            || parameter.declaration != function.declaration
            || owned != parameter.constraints.has_owned()
            || substitutions.insert(*id, *ty).is_some()
        {
            return Err(reject(
                "child type arguments disagree with their exact constraints",
            ));
        }
    }
    let application = super::owned_contract::admit_function_application_with_scopes(
        read,
        function,
        &type_arguments,
        &implementations,
        super::owned_contract::ScopeApplication {
            arguments: &read_scope_arguments,
            authority: scope_authority,
        },
        occurrence,
        admission,
    )?;
    let overlay = super::owned_contract::AppliedTypeRead {
        read,
        types: &application.types,
    };
    let mut applied = super::parallel_types::WitnessAppliedTypes::with_admission(
        &overlay,
        application.environment(),
        admission,
    );
    let result = applied.substitute(signature.result.ty, 0)?;
    if signature.parameters.len() != arguments.len() {
        return Err(reject(
            "parallel child argument count differs from its exact signature",
        ));
    }
    let result_owned = admit_result_at(&applied, result, occurrence, admission)?;
    let mut parameters = Vec::new();
    let mut seen_owned = false;
    for id in signature.parameters {
        read.validation_work()?;
        let key = OwnerKey::Parameter(id);
        let mut parameter = if function.package == read.package_id() {
            let Some(OwnerRecord::Parameter(p)) = read.owner(key)? else {
                return Err(reject("parallel child parameter is absent"));
            };
            p
        } else {
            let Some(PackageInterfaceRecord::Parameter(p)) =
                read.package_interface_owner(function.package, key)?
            else {
                return Err(reject("parallel child parameter interface is absent"));
            };
            p
        };
        if parameter.header.owner != key
            || parameter.header.kind != OwnerKind::Parameter
            || parameter.parent != ParameterParent::Function(function.declaration)
            || parameter.resource_requirement.is_some()
        {
            return Err(reject(
                "parallel child parameter has a foreign owner or capability requirement",
            ));
        }
        parameter.ty = applied.substitute(parameter.ty, 0)?;
        let object = applied
            .type_object(parameter.ty)?
            .ok_or_else(|| reject("missing exact parallel parameter category"))?;
        if super::owned_associated::read_view_admission::direct(&object.form) {
            if parameter.header.contract_version < contract::READ_VIEW_GRAPH_CONTRACT_VERSION
                || parameter.use_mode != ParameterUse::Borrow
                || seen_owned
            {
                return Err(reject(
                    "parallel view parameters require an exact Graph 34 borrow before the actual Owned suffix",
                ));
            }
            super::owned_associated::read_view_admission::read_shareable_at(
                &applied,
                parameter.ty,
                occurrence,
                admission,
            )?;
            parameters.push(parameter);
            continue;
        }
        match super::memory::direct_at(&applied, parameter.ty, occurrence, admission)? {
            true => {
                match parameter.use_mode {
                    ParameterUse::Borrow => {
                        if parameter.header.contract_version
                            < contract::SHARE_GRAPH_CONTRACT_VERSION
                        {
                            return Err(reject("parallel borrowed parameters require Graph 30"));
                        }
                        super::share::admit_at(&applied, parameter.ty, occurrence, admission)?;
                    }
                    ParameterUse::Consume => {
                        admit_result_at(&applied, parameter.ty, occurrence, admission)?;
                    }
                    ParameterUse::Unrestricted => {
                        return Err(reject(
                            "parallel child owned parameters require borrow or consume",
                        ));
                    }
                }
                seen_owned = true;
            }
            false => {
                admit_result_at(&applied, parameter.ty, occurrence, admission)?;
                if seen_owned || parameter.use_mode != ParameterUse::Unrestricted {
                    return Err(reject(
                        "parallel child parameters require ordinary data and view reads before the actual shareable or consuming Owned suffix",
                    ));
                }
            }
        }
        parameters.push(parameter);
    }
    let mut types = std::collections::BTreeMap::new();
    let mut metadata = super::owned_record::LayoutBudget::default();
    metadata.extend_types(&mut types, applied.into_types())?;
    metadata.extend_types(&mut types, application.types)?;
    Ok(ParallelCall {
        function,
        parameters,
        arguments,
        result,
        result_owned,
        types,
    })
}

/// A returned owner must be a closed transferable carrier. In particular an
/// unselected choice case or a phantom ordinary argument cannot hide authority.
pub(crate) fn admit_result(
    read: &(impl ExpressionRead + ?Sized),
    result: TypeObjectDigest,
    scope: Option<DeclarationId>,
) -> Result<bool, Diagnostic> {
    super::transfer::admit(read, result, scope).map_err(|error| {
        if error.class == DiagnosticClass::Semantic {
            reject(error.message)
        } else {
            error
        }
    })
}

pub(crate) fn admit_result_at(
    read: &(impl ExpressionRead + ?Sized),
    result: TypeObjectDigest,
    occurrence: super::owned_associated::OwnedTypeOccurrence,
    admission: &super::owned_associated::OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    super::transfer::admit_at(read, result, occurrence, admission).map_err(|error| {
        if error.class == DiagnosticClass::Semantic {
            reject(error.message)
        } else {
            error
        }
    })
}

#[cfg(test)]
mod occurrence_tests {
    use super::*;
    use crate::platform::kernel::memory::associated_tests::{FlowKind, fixture};
    use crate::platform::kernel::owned_associated::{
        AssociatedAdmissionLimits, OwnedAssociatedAdmission,
    };

    fn metadata_view_child() -> (
        crate::platform::kernel::memory::associated_tests::Fixture,
        crate::platform::semantic_id::ParameterId,
    ) {
        use crate::platform::semantic_id::ReadScopeParameterId;
        let mut flow = fixture(FlowKind::Direct);
        let callee_scope = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"parallel-view-child", 0),
            name: Name::new("s").unwrap(),
        };
        let caller_scope = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"parallel-view-child", 1),
            name: Name::new("s").unwrap(),
        };
        let formal = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(flow.callee),
            parameter: callee_scope.id,
        });
        let actual = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(flow.caller),
            parameter: caller_scope.id,
        });
        let view = DeclarationReference {
            package: flow.source.root.package_id,
            declaration: DeclarationId::migrate(b"parallel-view-child", 0),
        };
        let module = match &flow.source.owners[&OwnerKey::Declaration(flow.callee.declaration)] {
            OwnerRecord::Declaration(record) => record.module,
            _ => panic!("callee"),
        };
        flow.source.owners.insert(
            OwnerKey::Declaration(view.declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(view.declaration),
                    OwnerKind::ReadView,
                ),
                module,
                name: Name::new("MetadataView").unwrap(),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: vec![],
                    scope_parameter: ReadScopeParameter {
                        id: ReadScopeParameterId::migrate(b"parallel-view-child", 2),
                        name: Name::new("v").unwrap(),
                    },
                    fields: vec![],
                }),
            }),
        );
        let view_type = TypeObject::new(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![],
            read_scope: formal,
        })
        .unwrap();
        let view_digest = encode_type_object(&view_type).unwrap().0;
        flow.source.types.insert(view_digest, view_type);
        let unit = flow
            .source
            .types
            .iter()
            .find_map(|(ty, object)| matches!(object.form, TypeForm::Unit).then_some(*ty))
            .unwrap();
        let OwnerRecord::Declaration(callee) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(flow.callee.declaration))
            .unwrap()
        else {
            panic!("callee");
        };
        callee.header.kind = OwnerKind::TaskFunction;
        let DeclarationPayload::Function(callee) = &mut callee.payload else {
            panic!("callee function");
        };
        callee.type_parameters.clear();
        callee.implementation_parameters.clear();
        callee.result = CallableResult {
            ty: unit,
            relation: ResultRelation::Value,
        };
        callee.effect = FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        };
        callee.read_scope_parameters = vec![callee_scope];
        callee.read_scope_sources.clear();
        let parameter_id = callee.parameters[0];
        let OwnerRecord::Parameter(parameter) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Parameter(parameter_id))
            .unwrap()
        else {
            panic!("parameter");
        };
        parameter.ty = view_digest;
        parameter.use_mode = ParameterUse::Borrow;
        let OwnerRecord::Declaration(caller) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(flow.caller.declaration))
            .unwrap()
        else {
            panic!("caller");
        };
        let DeclarationPayload::Function(caller) = &mut caller.payload else {
            panic!("caller function");
        };
        caller.read_scope_parameters.push(caller_scope);
        let OwnerRecord::Expression(call) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Expression(flow.call))
            .unwrap()
        else {
            panic!("call");
        };
        let ExpressionOperation::Call {
            type_arguments,
            read_scope_arguments,
            ..
        } = &mut call.operation
        else {
            panic!("direct call");
        };
        type_arguments.clear();
        *read_scope_arguments = vec![actual];
        (flow, parameter_id)
    }

    fn append_child_parameter(
        flow: &mut crate::platform::kernel::memory::associated_tests::Fixture,
        ty: TypeObjectDigest,
        use_mode: ParameterUse,
    ) {
        use crate::platform::semantic_id::ParameterId;
        let parameter = ParameterId::migrate(b"parallel-view-child-additional", 0);
        flow.source.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(flow.callee.declaration),
                name: Name::new("additional").unwrap(),
                ty,
                use_mode,
                resource_requirement: None,
            }),
        );
        let OwnerRecord::Declaration(callee) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(flow.callee.declaration))
            .unwrap()
        else {
            panic!("callee");
        };
        let DeclarationPayload::Function(callee) = &mut callee.payload else {
            panic!("callee");
        };
        callee.parameters.push(parameter);
        let OwnerRecord::Expression(call) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Expression(flow.call))
            .unwrap()
        else {
            panic!("call");
        };
        let ExpressionOperation::Call { arguments, .. } = &mut call.operation else {
            panic!("call");
        };
        arguments.push(flow.local);
    }

    #[test]
    fn parallel_borrowed_metadata_view_has_read_sharing_without_source_read_grants() {
        let (mut flow, _) = metadata_view_child();
        let unit = flow
            .source
            .types
            .iter()
            .find_map(|(ty, object)| matches!(object.form, TypeForm::Unit).then_some(*ty))
            .unwrap();
        append_child_parameter(&mut flow, unit, ParameterUse::Unrestricted);
        let admitted = admit_call_at(
            &flow.source,
            flow.call,
            Some(flow.caller.declaration),
            &OwnedAssociatedAdmission::new(),
        )
        .unwrap();
        assert!(!admitted.result_owned);
        assert_eq!(admitted.parameters[0].use_mode, ParameterUse::Borrow);
        assert!(super::super::owned_associated::read_view_admission::direct(
            &admitted.types[&admitted.parameters[0].ty].form
        ));
        assert_eq!(admitted.parameters[1].use_mode, ParameterUse::Unrestricted);
    }

    #[test]
    fn parallel_view_consume_and_unused_scope_omission_are_refused() {
        let (mut flow, parameter_id) = metadata_view_child();
        let OwnerRecord::Parameter(parameter) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Parameter(parameter_id))
            .unwrap()
        else {
            panic!("parameter");
        };
        parameter.use_mode = ParameterUse::Consume;
        assert_eq!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &OwnedAssociatedAdmission::new()
            )
            .err()
            .unwrap()
            .class,
            DiagnosticClass::Semantic
        );
        let (mut flow, _) = metadata_view_child();
        let OwnerRecord::Expression(call) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Expression(flow.call))
            .unwrap()
        else {
            panic!("call");
        };
        let ExpressionOperation::Call {
            read_scope_arguments,
            ..
        } = &mut call.operation
        else {
            panic!("call");
        };
        read_scope_arguments.clear();
        assert_eq!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &OwnedAssociatedAdmission::new()
            )
            .err()
            .unwrap()
            .class,
            DiagnosticClass::Semantic
        );
        let (mut flow, _) = metadata_view_child();
        let buffer = TypeObject::new(TypeForm::ByteBuffer).unwrap();
        let buffer_type = encode_type_object(&buffer).unwrap().0;
        flow.source.types.insert(buffer_type, buffer);
        append_child_parameter(&mut flow, buffer_type, ParameterUse::Borrow);
        let OwnerRecord::Declaration(callee) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(flow.callee.declaration))
            .unwrap()
        else {
            panic!("callee");
        };
        let DeclarationPayload::Function(callee) = &mut callee.payload else {
            panic!("callee");
        };
        callee.parameters.swap(0, 1);
        let error = admit_call_at(
            &flow.source,
            flow.call,
            Some(flow.caller.declaration),
            &OwnedAssociatedAdmission::new(),
        )
        .err()
        .unwrap();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert!(error.message.contains("Owned suffix"));
    }

    #[test]
    fn parallel_scope_authority_cannot_be_relabelled_as_another_callable() {
        let (flow, _) = metadata_view_child();
        let loans = std::collections::BTreeSet::new();
        let error = admit_call_at_with_scope_authority(
            &flow.source,
            flow.call,
            Some(flow.caller.declaration),
            super::super::owned_associated::read_view_admission::ScopeAuthority {
                binder: Some(ReadScopeBinder::Declaration(flow.callee)),
                lexical: Some(flow.callee),
                live_loans: &loans,
            },
            &OwnedAssociatedAdmission::new(),
        )
        .err()
        .unwrap();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert!(error.message.contains("containing callable"));
    }

    #[test]
    fn parallel_legacy_empty_scope_authority_is_exact_and_cannot_authorize_graph34() {
        let mut source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                r#"declarations.begin
(units (module create legacy-parallel
  (function create child (visibility private) (effect (task))
    (returns I64) (body (i64 7)))
  (function create parent (visibility public) (effect (task))
    (returns (record (left I64) (right I64)))
    (body (parallel (call child) (call child))))))
declarations.end"#,
            )
            .unwrap();
        let parent = source
            .owners
            .values()
            .find_map(|owner| match owner {
                OwnerRecord::Declaration(record) if record.name.as_str() == "parent" => {
                    let OwnerKey::Declaration(declaration) = record.header.owner else {
                        panic!("parent declaration");
                    };
                    Some(DeclarationReference {
                        package: source.root.package_id,
                        declaration,
                    })
                }
                _ => None,
            })
            .unwrap();
        let child = source
            .owners
            .values()
            .find_map(|owner| match owner {
                OwnerRecord::Expression(record) => match record.operation {
                    ExpressionOperation::Parallel { left, .. } => Some(left),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        for owner in source.owners.values_mut() {
            let generation = if matches!(owner, OwnerRecord::Expression(record)
                if matches!(record.operation, ExpressionOperation::Parallel { .. }))
            {
                21
            } else {
                20
            };
            owner.set_encoding_for_edit(generation);
        }
        validate_full(&source).unwrap();
        let loans = std::collections::BTreeSet::new();
        let empty = super::super::owned_associated::read_view_admission::ScopeAuthority {
            binder: None,
            lexical: None,
            live_loans: &loans,
        };
        let call = admit_call_at_with_scope_authority(
            &source,
            child,
            Some(parent.declaration),
            empty,
            &OwnedAssociatedAdmission::new(),
        )
        .unwrap();
        assert!(call.parameters.is_empty());
        assert!(!call.result_owned);
        assert!(matches!(source.types[&call.result].form, TypeForm::I64));
        for variant in 0..5 {
            let mut forged = source.clone();
            let mut live = std::collections::BTreeSet::new();
            let authority = if variant == 1 {
                live.insert(crate::platform::semantic_id::BindingId::migrate(
                    b"legacy-parallel",
                    0,
                ));
                super::super::owned_associated::read_view_admission::ScopeAuthority {
                    binder: None,
                    lexical: None,
                    live_loans: &live,
                }
            } else {
                empty
            };
            match variant {
                0 => forged
                    .owners
                    .get_mut(&OwnerKey::Declaration(parent.declaration))
                    .unwrap()
                    .set_encoding_for_edit(contract::READ_VIEW_GRAPH_CONTRACT_VERSION),
                1 => {}
                2 => {
                    forged
                        .owners
                        .remove(&OwnerKey::Declaration(parent.declaration));
                }
                3 => {
                    let OwnerRecord::Declaration(record) = forged
                        .owners
                        .get_mut(&OwnerKey::Declaration(parent.declaration))
                        .unwrap()
                    else {
                        panic!("parent owner");
                    };
                    record.header.kind = OwnerKind::PureFunction;
                }
                4 => forged
                    .owners
                    .get_mut(&OwnerKey::Declaration(parent.declaration))
                    .unwrap()
                    .set_encoding_for_edit(0),
                _ => unreachable!(),
            }
            let error = admit_call_at_with_scope_authority(
                &forged,
                child,
                Some(parent.declaration),
                authority,
                &OwnedAssociatedAdmission::new(),
            )
            .err()
            .unwrap();
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert!(
                error.message.contains("containing callable"),
                "{variant}: {error:?}"
            );
        }
    }

    #[test]
    fn parallel_phantom_projection_uses_exact_child_generation_and_shared_budget() {
        let mut flow = fixture(FlowKind::Direct);
        let unit = flow
            .source
            .types
            .iter()
            .find_map(|(ty, object)| matches!(object.form, TypeForm::Unit).then_some(*ty))
            .unwrap();
        let OwnerRecord::Declaration(callee) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(flow.callee.declaration))
            .unwrap()
        else {
            panic!("callee declaration");
        };
        callee.header.kind = OwnerKind::TaskFunction;
        let DeclarationPayload::Function(callee) = &mut callee.payload else {
            panic!("callee function");
        };
        callee.parameters.clear();
        callee.result = CallableResult {
            ty: unit,
            relation: ResultRelation::Value,
        };
        callee.effect = FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        };
        let OwnerRecord::Expression(call) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Expression(flow.call))
            .unwrap()
        else {
            panic!("call expression");
        };
        let ExpressionOperation::Call { arguments, .. } = &mut call.operation else {
            panic!("direct call");
        };
        arguments.clear();

        // Out:Owned is only a metadata actual here. A phantom actual does not
        // acquire a moving or sharing obligation from this ordinary result.
        let healthy = OwnedAssociatedAdmission::new();
        let call = admit_call_at(
            &flow.source,
            flow.call,
            Some(flow.caller.declaration),
            &healthy,
        )
        .unwrap();
        assert!(!call.result_owned);
        assert_eq!(call.result, unit);
        let (steps, _) = healthy.work();
        assert!(steps > 0);
        let exact = OwnedAssociatedAdmission::with_limits(AssociatedAdmissionLimits {
            maximum_steps: steps,
            ..AssociatedAdmissionLimits::default()
        });
        assert!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &exact,
            )
            .is_ok()
        );
        assert_eq!(exact.work().0, steps);
        assert_eq!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &exact,
            )
            .err()
            .unwrap()
            .class,
            DiagnosticClass::Resource,
        );
        assert!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &OwnedAssociatedAdmission::new(),
            )
            .is_ok()
        );

        let OwnerRecord::Expression(call) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Expression(flow.call))
            .unwrap()
        else {
            panic!("call expression");
        };
        // Owned associated projections remain supported in Graph 33. The
        // actual refusal boundary is Graph 32, independently of newer views.
        call.contract_version = contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION;
        assert!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &OwnedAssociatedAdmission::new(),
            )
            .is_ok()
        );
        let OwnerRecord::Expression(call) = flow
            .source
            .owners
            .get_mut(&OwnerKey::Expression(flow.call))
            .unwrap()
        else {
            panic!("call expression");
        };
        call.contract_version = contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION - 1;
        assert_eq!(
            admit_call_at(
                &flow.source,
                flow.call,
                Some(flow.caller.declaration),
                &OwnedAssociatedAdmission::new(),
            )
            .err()
            .unwrap()
            .class,
            DiagnosticClass::Semantic,
        );
    }
}
