//! Structural affine products. Ordinary metadata uses the existing closed-data proof.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::DeclarationId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_owned_product", message)
}

/// An extension type cannot acquire older graph authority merely by using old
/// expression tags (for example a product-only relay consisting of Local).
pub(crate) fn require_generation(
    read: &(impl ExpressionRead + ?Sized),
    roots: Vec<TypeObjectDigest>,
    generation: u16,
) -> Result<(), Diagnostic> {
    if generation >= 19 {
        return Ok(());
    }
    let mut pending = roots;
    let mut seen = BTreeSet::new();
    while let Some(ty) = pending.pop() {
        read.validation_work()?;
        if !seen.insert(ty) {
            continue;
        }
        let object = read
            .type_object(ty)?
            .ok_or_else(|| reject("missing graph-bound type"))?;
        if matches!(object.form, TypeForm::OwnedProduct { .. }) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_product_generation",
                "owned product type closure requires Graph 19",
            ));
        }
        pending.extend(object.child_types());
    }
    Ok(())
}

pub(crate) fn validate(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationId>,
) -> Result<(), Diagnostic> {
    // A digest first reached through a short path may occur on a longer path
    // later. A plain visited set is not a depth proof for a shared type DAG.
    let mut depths = BTreeMap::new();
    let mut closure = vec![(ty, 0usize)];
    while let Some((current, depth)) = closure.pop() {
        read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(reject(
                "owned product exceeds the structural type depth bound",
            ));
        }
        if depths
            .get(&current)
            .is_some_and(|previous| *previous >= depth)
        {
            continue;
        }
        depths.insert(current, depth);
        let object = read
            .type_object(current)?
            .ok_or_else(|| reject("missing product type child"))?;
        for child in object.child_types() {
            read.validation_work()?;
            closure.push((child, depth + 1));
        }
    }
    let mut pending = vec![(ty, 0)];
    let mut seen = BTreeSet::new();
    while let Some((ty, depth)) = pending.pop() {
        read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(reject(
                "owned product exceeds the structural type depth bound",
            ));
        }
        if !seen.insert(ty) {
            continue;
        }
        let object = read
            .type_object(ty)?
            .ok_or_else(|| reject("missing owned product type"))?;
        let TypeForm::OwnedProduct { fields } = object.form else {
            return Err(reject(
                "explicit product operand requires an owned product type",
            ));
        };
        let mut owned = false;
        for field in fields {
            read.validation_work()?;
            let child = read
                .type_object(field.ty)?
                .ok_or_else(|| reject("missing owned product field type"))?;
            match child.form {
                TypeForm::OwnedProduct { .. } => {
                    owned = true;
                    pending.push((field.ty, depth + 1));
                }
                TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => owned = true,
                TypeForm::TypeParameter { parameter } => {
                    let Some(OwnerRecord::TypeParameter(p)) =
                        read.owner(OwnerKey::TypeParameter(parameter))?
                    else {
                        return Err(reject("missing owned product parameter"));
                    };
                    if p.constraints != TypeParameterConstraints::Owned
                        || Some(p.declaration) != scope
                    {
                        return Err(reject(
                            "product parameters require exact in-scope Owned constraints",
                        ));
                    }
                    let Some(OwnerRecord::Declaration(d)) =
                        read.owner(OwnerKey::Declaration(p.declaration))?
                    else {
                        return Err(reject("missing owned parameter declaration"));
                    };
                    if !matches!(d.payload, DeclarationPayload::Function(f) if f.type_parameters.contains(&parameter))
                    {
                        return Err(reject(
                            "product parameter is outside its function signature",
                        ));
                    }
                    owned = true;
                }
                _ if !super::owned_contract::ordinary_closed(read, field.ty)? => {
                    return Err(reject(
                        "product metadata must be closed ordinary first-order data",
                    ));
                }
                _ => {}
            }
        }
        if !owned {
            return Err(reject(
                "owned product requires at least one direct owned field",
            ));
        }
    }
    Ok(())
}
