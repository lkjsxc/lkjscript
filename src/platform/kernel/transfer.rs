//! Bounded structural transfer obligations under exact lexical type assumptions.
//! This proves language meaning; it neither grants capabilities nor moves tokens.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{DeclarationId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "kernel_transfer_constraint",
        message,
    )
}

/// Charge exact signature membership even when the parameter is absent or unused.
pub(super) fn function_parameter_listed(
    read: &(impl ExpressionRead + ?Sized),
    parameters: &[TypeParameterId],
    parameter: TypeParameterId,
) -> Result<bool, Diagnostic> {
    let mut listed = false;
    for id in parameters {
        read.validation_work()?;
        listed |= *id == parameter;
    }
    Ok(listed)
}

pub(super) fn ordinary_assumptions(
    read: &(impl ExpressionRead + ?Sized),
    scope: Option<DeclarationId>,
) -> Result<BTreeSet<TypeParameterId>, Diagnostic> {
    let mut assumptions = BTreeSet::new();
    let Some(scope) = scope else {
        return Ok(assumptions);
    };
    read.validation_work()?;
    let Some(OwnerRecord::Declaration(declaration)) = read.owner(OwnerKey::Declaration(scope))?
    else {
        return Err(reject(
            "transfer assumptions require an exact function scope",
        ));
    };
    let DeclarationPayload::Function(function) = declaration.payload else {
        // Closed carriers also occur as implementation Self and other declaration
        // annotations. Such contexts contribute no open transfer assumptions.
        return Ok(assumptions);
    };
    for parameter in function.type_parameters {
        read.validation_work()?;
        let Some(OwnerRecord::TypeParameter(record)) =
            read.owner(OwnerKey::TypeParameter(parameter))?
        else {
            return Err(reject("transfer assumption parameter is absent"));
        };
        if record.declaration != scope || record.header.owner != OwnerKey::TypeParameter(parameter)
        {
            return Err(reject(
                "transfer assumption has a foreign declaration owner",
            ));
        }
        if record.constraints.requires_transfer() && !record.constraints.has_owned() {
            read.validation_work()?;
            assumptions.insert(parameter);
        }
    }
    Ok(assumptions)
}

fn owned_parameter(
    read: &(impl ExpressionRead + ?Sized),
    parameter: TypeParameterId,
    scope: Option<DeclarationId>,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    let Some(OwnerRecord::TypeParameter(record)) =
        read.owner(OwnerKey::TypeParameter(parameter))?
    else {
        return Err(reject("transferable owner parameter is absent"));
    };
    if Some(record.declaration) != scope
        || record.header.owner != OwnerKey::TypeParameter(parameter)
        || !record.constraints.has_owned()
        || !record.constraints.requires_transfer()
    {
        return Err(reject(
            "an open owner requires an exact in-scope owned and transferable constraint",
        ));
    }
    let Some(OwnerRecord::Declaration(declaration)) =
        read.owner(OwnerKey::Declaration(record.declaration))?
    else {
        return Err(reject("transferable owner has no function declaration"));
    };
    let DeclarationPayload::Function(function) = declaration.payload else {
        return Err(reject(
            "transferable owner assumptions belong to graph functions",
        ));
    };
    if !function_parameter_listed(read, &function.type_parameters, parameter)? {
        return Err(reject("transferable owner escapes its function signature"));
    }
    Ok(())
}

pub(crate) fn ordinary(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationId>,
) -> Result<bool, Diagnostic> {
    super::owned_contract::ordinary_transfer(read, ty, scope)
}

/// Admit the complete transferable closure and return its static ownership class.
/// Ordinary nominal proof checks phantom actuals and all recursive members.
pub(crate) fn admit(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationId>,
) -> Result<bool, Diagnostic> {
    read.validation_work()?;
    let root_owned = super::memory::direct(read, ty)?;
    let assumptions = ordinary_assumptions(read, scope)?;
    if matches!(
        read.type_object(ty)?.map(|t| t.form),
        Some(TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. })
    ) {
        super::owned_product::validate(read, ty, scope)?;
    }
    read.validation_work()?;
    let mut pending = vec![(ty, 0usize)];
    let mut depths = BTreeMap::new();
    while let Some((current, depth)) = pending.pop() {
        read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(reject(
                "transferable owned structure exceeds the type depth bound",
            ));
        }
        if depths
            .get(&current)
            .is_some_and(|previous| *previous >= depth)
        {
            continue;
        }
        read.validation_work()?;
        depths.insert(current, depth);
        let object = read
            .type_object(current)?
            .ok_or_else(|| reject("transfer type is absent"))?;
        match object.form {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => {}
            TypeForm::TypeParameter { parameter } if super::memory::direct(read, current)? => {
                owned_parameter(read, parameter, scope)?;
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    read.validation_work()?;
                    pending.push((field.ty, depth + 1));
                }
            }
            _ if super::owned_contract::ordinary_with_assumptions(
                read,
                current,
                scope,
                &assumptions,
            )? => {}
            _ => {
                return Err(reject(
                    "transfer requires closed first-order data or explicit scoped transferable types",
                ));
            }
        }
    }
    Ok(root_owned)
}
