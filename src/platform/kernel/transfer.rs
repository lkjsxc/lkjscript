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
    ordinary_assumptions_in(
        read,
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
    )
}

pub(super) fn ordinary_assumptions_in(
    read: &(impl ExpressionRead + ?Sized),
    scope: Option<DeclarationReference>,
) -> Result<BTreeSet<TypeParameterId>, Diagnostic> {
    let mut assumptions = BTreeSet::new();
    let Some(scope) = scope else {
        return Ok(assumptions);
    };
    read.validation_work()?;
    let key = OwnerKey::Declaration(scope.declaration);
    let parameters = if scope.package == read.package_id() {
        match read.owner(key)? {
            Some(OwnerRecord::Declaration(declaration)) => match declaration.payload {
                DeclarationPayload::Function(function) => function.type_parameters,
                DeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.type_parameters
                }
                _ => return Ok(assumptions),
            },
            _ => {
                return Err(reject(
                    "transfer assumptions require an exact declaration scope",
                ));
            }
        }
    } else {
        match read.package_interface_owner(scope.package, key)? {
            Some(PackageInterfaceRecord::Declaration(declaration)) => match declaration.payload {
                PackageInterfaceDeclarationPayload::Function(function) => function.type_parameters,
                PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.type_parameters
                }
                _ => return Ok(assumptions),
            },
            _ => {
                return Err(reject(
                    "transfer assumptions require an exact imported scope",
                ));
            }
        }
    };
    for parameter in parameters {
        read.validation_work()?;
        let key = OwnerKey::TypeParameter(parameter);
        let record = if scope.package == read.package_id() {
            match read.owner(key)? {
                Some(OwnerRecord::TypeParameter(record)) => record,
                _ => return Err(reject("transfer assumption parameter is absent")),
            }
        } else {
            match read.package_interface_owner(scope.package, key)? {
                Some(PackageInterfaceRecord::TypeParameter(record)) => record,
                _ => return Err(reject("imported transfer assumption parameter is absent")),
            }
        };
        if record.declaration != scope.declaration
            || record.header.owner != OwnerKey::TypeParameter(parameter)
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

fn owned_parameter_in(
    read: &(impl ExpressionRead + ?Sized),
    parameter: TypeParameterId,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    let scope = scope.ok_or_else(|| reject("transferable owner requires an exact scope"))?;
    let key = OwnerKey::TypeParameter(parameter);
    let record = if scope.package == read.package_id() {
        match read.owner(key)? {
            Some(OwnerRecord::TypeParameter(record)) => record,
            _ => return Err(reject("transferable owner parameter is absent")),
        }
    } else {
        match read.package_interface_owner(scope.package, key)? {
            Some(PackageInterfaceRecord::TypeParameter(record)) => record,
            _ => return Err(reject("imported transferable owner parameter is absent")),
        }
    };
    if record.declaration != scope.declaration
        || record.header.owner != OwnerKey::TypeParameter(parameter)
        || !record.constraints.has_owned()
        || !record.constraints.requires_transfer()
    {
        return Err(reject(
            "an open owner requires an exact in-scope owned and transferable constraint",
        ));
    }
    let key = OwnerKey::Declaration(scope.declaration);
    let parameters = if scope.package == read.package_id() {
        match read.owner(key)? {
            Some(OwnerRecord::Declaration(declaration)) => match declaration.payload {
                DeclarationPayload::Function(function) => function.type_parameters,
                DeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.type_parameters
                }
                _ => {
                    return Err(reject(
                        "transferable owner assumptions require a function or scheme",
                    ));
                }
            },
            _ => return Err(reject("transferable owner has no exact declaration")),
        }
    } else {
        match read.package_interface_owner(scope.package, key)? {
            Some(PackageInterfaceRecord::Declaration(declaration)) => match declaration.payload {
                PackageInterfaceDeclarationPayload::Function(function) => function.type_parameters,
                PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.type_parameters
                }
                _ => {
                    return Err(reject(
                        "imported transferable assumptions require a function or scheme",
                    ));
                }
            },
            _ => {
                return Err(reject(
                    "transferable owner has no exact imported declaration",
                ));
            }
        }
    };
    if !function_parameter_listed(read, &parameters, parameter)? {
        return Err(reject(
            "transferable owner escapes its exact parameter inventory",
        ));
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
    admit_in(
        read,
        ty,
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
    )
}

pub(crate) fn admit_in(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<bool, Diagnostic> {
    read.validation_work()?;
    let package = scope.map_or(read.package_id(), |scope| scope.package);
    let root_owned = super::memory::direct_in(read, package, ty)?;
    let assumptions = ordinary_assumptions_in(read, scope)?;
    if matches!(
        read.type_object(ty)?.map(|t| t.form),
        Some(
            TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
        )
    ) {
        super::owned_product::validate_in_scope(read, ty, scope)?;
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
            TypeForm::TypeParameter { parameter }
                if super::memory::direct_in(read, package, current)? =>
            {
                owned_parameter_in(read, parameter, scope)?;
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    read.validation_work()?;
                    pending.push((field.ty, depth + 1));
                }
            }
            TypeForm::OwnedSequence { item } => {
                read.validation_work()?;
                pending.push((item, depth + 1));
            }
            _ if super::owned_contract::ordinary_with_assumptions_in(
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
