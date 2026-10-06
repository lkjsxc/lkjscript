//! Bounded structural immutable-sharing proof under exact declaration scopes.
//! Sharing grants no transfer authority and never changes a backing owner's origin.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{DeclarationId, TypeParameterId};
use std::collections::BTreeMap;

fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "kernel_share_constraint",
        message,
    )
}

fn owned_parameter(
    read: &(impl ExpressionRead + ?Sized),
    parameter: TypeParameterId,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    let scope =
        scope.ok_or_else(|| reject("shareable owner requires an exact declaration scope"))?;
    let record = if scope.package == read.package_id() {
        match read.owner(OwnerKey::TypeParameter(parameter))? {
            Some(OwnerRecord::TypeParameter(record)) => record,
            _ => return Err(reject("shareable owner parameter is absent")),
        }
    } else {
        match read.package_interface_owner(scope.package, OwnerKey::TypeParameter(parameter))? {
            Some(PackageInterfaceRecord::TypeParameter(record)) => record,
            _ => return Err(reject("imported shareable owner parameter is absent")),
        }
    };
    if record.declaration != scope.declaration
        || record.header.owner != OwnerKey::TypeParameter(parameter)
        || !record.constraints.has_owned()
        || !record.constraints.requires_share()
    {
        return Err(reject(
            "an open owner requires an exact in-scope owned and shareable constraint",
        ));
    }
    read.validation_work()?;
    let listed = if scope.package == read.package_id() {
        match read.owner(OwnerKey::Declaration(scope.declaration))? {
            Some(OwnerRecord::Declaration(declaration)) => match declaration.payload {
                DeclarationPayload::Function(function) => {
                    super::transfer::function_parameter_listed(
                        read,
                        &function.type_parameters,
                        parameter,
                    )?
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    super::transfer::function_parameter_listed(
                        read,
                        &implementation.type_parameters,
                        parameter,
                    )?
                }
                _ => false,
            },
            _ => false,
        }
    } else {
        match read
            .package_interface_owner(scope.package, OwnerKey::Declaration(scope.declaration))?
        {
            Some(PackageInterfaceRecord::Declaration(declaration)) => match declaration.payload {
                PackageInterfaceDeclarationPayload::Function(function) => {
                    super::transfer::function_parameter_listed(
                        read,
                        &function.type_parameters,
                        parameter,
                    )?
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                    super::transfer::function_parameter_listed(
                        read,
                        &implementation.type_parameters,
                        parameter,
                    )?
                }
                _ => false,
            },
            _ => false,
        }
    };
    if !listed {
        return Err(reject(
            "shareable owner is absent from its declaration's exact parameter inventory",
        ));
    }
    Ok(())
}

/// Admit a complete shareable Owned closure under a local defining declaration.
pub(crate) fn admit(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationId>,
) -> Result<bool, Diagnostic> {
    admit_in(
        read,
        ty,
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
    )
}

/// Imported symbolic assumptions retain their exact defining package and declaration.
/// The returned ownership class is always Owned; an ordinary root has no share loan.
pub(crate) fn admit_in(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<bool, Diagnostic> {
    read.validation_work()?;
    let package = scope.map_or(read.package_id(), |scope| scope.package);
    let root_owned = super::memory::direct_in(read, package, ty)?;
    if !root_owned {
        return Err(reject("shareable admission requires an owned root"));
    }
    // Ordinary structural metadata retains the existing first-order-data proof.
    // Its transfer assumptions do not confer shareability on any owned member.
    let ordinary_assumptions = super::transfer::ordinary_assumptions_in(read, scope)?;
    read.validation_work()?;
    let mut pending = vec![(ty, 0usize)];
    let mut depths = BTreeMap::new();
    while let Some((current, depth)) = pending.pop() {
        read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(reject(
                "shareable owned structure exceeds the type depth bound",
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
            .ok_or_else(|| reject("share type is absent"))?;
        match object.form {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => {}
            TypeForm::TypeParameter { parameter }
                if super::memory::direct_in(read, package, current)? =>
            {
                owned_parameter(read, parameter, scope)?;
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                let mut owned = false;
                for field in fields {
                    read.validation_work()?;
                    owned |= super::memory::direct_in(read, package, field.ty)?;
                    pending.push((field.ty, depth + 1));
                }
                if !owned {
                    return Err(reject(
                        "shareable owned structure requires a direct owned field or case",
                    ));
                }
            }
            TypeForm::OwnedSequence { item } => {
                read.validation_work()?;
                if !super::memory::direct_in(read, package, item)? {
                    return Err(reject("shareable owned sequence requires an owned element"));
                }
                pending.push((item, depth + 1));
            }
            _ if super::owned_contract::ordinary_with_assumptions_in(
                read,
                current,
                scope,
                &ordinary_assumptions,
            )? =>
            {
                // Charge and bound ordinary argument metadata as well, including
                // phantom actuals; the ordinary proof checks all nominal members.
                for child in object.child_types() {
                    read.validation_work()?;
                    pending.push((child, depth + 1));
                }
            }
            _ => {
                return Err(reject(
                    "sharing requires owned shareable types and complete first-order data",
                ));
            }
        }
    }
    Ok(root_owned)
}
