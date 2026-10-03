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

fn reject(message: impl Into<String>) -> Diagnostic {
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
    let ExpressionOperation::Call {
        function,
        arguments,
        type_arguments,
        effect_arguments,
        requirement_arguments,
    } = record.operation
    else {
        return Err(reject("parallel children require direct named task calls"));
    };
    if !type_arguments.is_empty()
        || !effect_arguments.is_empty()
        || !requirement_arguments.is_empty()
    {
        return Err(reject("parallel children require monomorphic task calls"));
    }
    read.validation_work()?;
    let key = OwnerKey::Declaration(function.declaration);
    let signature = if function.package == read.package_id() {
        let Some(OwnerRecord::Declaration(record)) = read.owner(key)? else {
            return Err(reject("parallel child function is absent"));
        };
        let DeclarationPayload::Function(f) = record.payload else {
            return Err(reject("parallel child must name a graph function"));
        };
        PackageFunctionSignature {
            implementation_parameters: f.implementation_parameters,
            requirement_parameters: f.requirement_parameters,
            effect_parameters: f.effect_parameters,
            type_parameters: f.type_parameters,
            parameters: f.parameters,
            result: f.result,
            effect: f.effect,
        }
    } else {
        if !read.has_dependency(function.package)? {
            return Err(reject("parallel child belongs to no exact dependency"));
        }
        let Some(PackageInterfaceRecord::Declaration(record)) =
            read.package_interface_owner(function.package, key)?
        else {
            return Err(reject("parallel child function interface is absent"));
        };
        let PackageInterfaceDeclarationPayload::Function(f) = record.payload else {
            return Err(reject(
                "parallel child must name an imported graph function",
            ));
        };
        f
    };
    if !signature.type_parameters.is_empty()
        || !signature.implementation_parameters.is_empty()
        || !signature.requirement_parameters.is_empty()
        || !signature.effect_parameters.is_empty()
        || !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters }
            if requirements.is_empty() && effect_parameters.is_empty())
    {
        return Err(reject(
            "parallel children require monomorphic tasks with closed empty effect rows",
        ));
    }
    if signature.parameters.len() != arguments.len() {
        return Err(reject(
            "parallel child argument count differs from its exact signature",
        ));
    }
    let result_owned = admit_result(read, signature.result)?;
    let mut parameters = Vec::new();
    let mut seen_owned = false;
    for id in signature.parameters {
        read.validation_work()?;
        let key = OwnerKey::Parameter(id);
        let parameter = if function.package == read.package_id() {
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
        let ty = read
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
                    super::owned_product::validate(read, parameter.ty, None)?;
                }
                seen_owned = true;
            }
            _ => {
                if seen_owned
                    || parameter.use_mode != ParameterUse::Unrestricted
                    || !super::owned_contract::ordinary_closed(read, parameter.ty)?
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
        result: signature.result,
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
