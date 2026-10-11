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
    read.validation_work()?;
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
    let (function, arguments, type_arguments, implementations) = match record.operation {
        ExpressionOperation::Call {
            function,
            arguments,
            type_arguments,
            effect_arguments,
            requirement_arguments,
        } if effect_arguments.is_empty() && requirement_arguments.is_empty() => {
            (function, arguments, type_arguments, Vec::new())
        }
        ExpressionOperation::ImplementationCall {
            function,
            arguments,
            type_arguments,
            effect_arguments,
            requirement_arguments,
            implementations,
        } if effect_arguments.is_empty() && requirement_arguments.is_empty() => {
            (function, arguments, type_arguments, implementations)
        }
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
        || signature.result_borrow.is_some()
        || !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters }
            if requirements.is_empty() && effect_parameters.is_empty())
    {
        return Err(reject(
            "parallel children require tasks with closed empty effect rows",
        ));
    }
    if signature.type_parameters.len() != type_arguments.len()
        || signature.implementation_parameters.len() != implementations.len()
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
        if parameter.declaration != function.declaration
            || owned != parameter.constraints.has_owned()
            || substitutions.insert(*id, *ty).is_some()
        {
            return Err(reject(
                "child type arguments disagree with their exact constraints",
            ));
        }
    }
    let application = super::owned_contract::admit_function_application(
        read,
        function,
        &type_arguments,
        &implementations,
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
    let result = applied.substitute(signature.result, 0)?;
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
        if parameter.parent != ParameterParent::Function(function.declaration)
            || parameter.resource_requirement.is_some()
        {
            return Err(reject(
                "parallel child parameter has a foreign owner or capability requirement",
            ));
        }
        parameter.ty = applied.substitute(parameter.ty, 0)?;
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
                        "parallel child parameters require transferable data followed by shareable reads or consuming transferable owners",
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
        callee.result = unit;
        callee.result_borrow = None;
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
        call.contract_version = contract::GRAPH_CONTRACT_VERSION - 1;
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
