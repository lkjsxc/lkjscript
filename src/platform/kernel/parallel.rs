//! Exact static admission for lexical structured child calls.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
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
            implementations,
        } => (function, arguments, type_arguments, implementations),
        _ => {
            return Err(reject(
                "parallel children require direct named calls with closed applications",
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
        || !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters }
            if requirements.is_empty() && effect_parameters.is_empty())
    {
        return Err(reject(
            "parallel children require tasks with closed empty effect rows",
        ));
    }
    if signature.type_parameters.len() != type_arguments.len()
        || signature.implementation_parameters.len() != implementations.len()
        || implementations
            .iter()
            .any(|i| !matches!(i, ImplementationOperand::Concrete { .. }))
    {
        return Err(reject(
            "parallel child applications require exact concrete types and implementations",
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
        let owned = admit_result(read, *ty)?;
        if parameter.declaration != function.declaration
            || owned != (parameter.constraints == TypeParameterConstraints::Owned)
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
        None,
    )?;
    let mut applied = super::parallel_types::AppliedTypes::new(read);
    let result = applied.substitute(signature.result, &substitutions, 0)?;
    if signature.parameters.len() != arguments.len() {
        return Err(reject(
            "parallel child argument count differs from its exact signature",
        ));
    }
    let result_owned = admit_result(&applied, result)?;
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
        let ty = applied
            .type_object(parameter.ty)?
            .ok_or_else(|| reject("parallel child parameter type is absent"))?;
        match ty.form {
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. } => {
                if parameter.use_mode != ParameterUse::Consume {
                    return Err(reject("parallel child owned parameters must consume"));
                }
                if matches!(
                    ty.form,
                    TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. }
                ) {
                    super::owned_product::validate(&applied, parameter.ty, None)?;
                }
                seen_owned = true;
            }
            _ => {
                if seen_owned
                    || parameter.use_mode != ParameterUse::Unrestricted
                    || !super::owned_contract::ordinary_closed(&applied, parameter.ty)?
                {
                    return Err(reject(
                        "parallel child parameters require closed ordinary data followed by consuming owned values",
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
) -> Result<bool, Diagnostic> {
    read.validation_work()?;
    let form = read
        .type_object(result)?
        .ok_or_else(|| reject("parallel child result type is absent"))?
        .form;
    match form {
        TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Ok(true),
        TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. } => {
            super::owned_product::validate(read, result, None)?;
            Ok(true)
        }
        _ if super::owned_contract::ordinary_closed(read, result)? => Ok(false),
        _ => Err(reject(
            "parallel child results require closed ordinary data or closed owned values",
        )),
    }
}
