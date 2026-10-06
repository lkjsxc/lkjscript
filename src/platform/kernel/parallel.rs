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
    read.validation_work()?;
    let Some(OwnerRecord::Expression(record)) = read.owner(OwnerKey::Expression(expression))?
    else {
        return Err(reject("parallel child expression is absent"));
    };
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
        let owned = super::memory::direct(read, *ty)?;
        if parameter.declaration != function.declaration
            || owned != parameter.constraints.has_owned()
            || substitutions.insert(*id, *ty).is_some()
        {
            return Err(reject(
                "child type arguments disagree with their exact constraints",
            ));
        }
    }
    super::owned_contract::validate_application(
        read,
        function,
        &type_arguments,
        &implementations,
        scope,
    )?;
    let mut applied = super::parallel_types::AppliedTypes::new(read);
    let result = applied.substitute(signature.result, &substitutions, 0)?;
    if signature.parameters.len() != arguments.len() {
        return Err(reject(
            "parallel child argument count differs from its exact signature",
        ));
    }
    let result_owned = admit_result(&applied, result, scope)?;
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
        parameter.ty = applied.substitute(parameter.ty, &substitutions, 0)?;
        match super::memory::direct(&applied, parameter.ty)? {
            true => {
                match parameter.use_mode {
                    ParameterUse::Borrow => {
                        if parameter.header.contract_version
                            < contract::SHARE_GRAPH_CONTRACT_VERSION
                        {
                            return Err(reject("parallel borrowed parameters require Graph 30"));
                        }
                        super::share::admit(&applied, parameter.ty, scope)?;
                    }
                    ParameterUse::Consume => {
                        admit_result(&applied, parameter.ty, scope)?;
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
                admit_result(&applied, parameter.ty, scope)?;
                if seen_owned || parameter.use_mode != ParameterUse::Unrestricted {
                    return Err(reject(
                        "parallel child parameters require transferable data followed by shareable reads or consuming transferable owners",
                    ));
                }
            }
        }
        parameters.push(parameter);
    }
    Ok(ParallelCall {
        function,
        parameters,
        arguments,
        result,
        result_owned,
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
