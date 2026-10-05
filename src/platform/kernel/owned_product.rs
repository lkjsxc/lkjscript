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
    if generation >= 25 {
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
        if generation < 25 && matches!(object.form, TypeForm::OwnedSequence { .. }) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_sequence_generation",
                "owned sequence type closure requires Graph 25",
            ));
        }
        if generation < 20 && matches!(object.form, TypeForm::OwnedChoice { .. }) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_choice_generation",
                "owned choice type closure requires Graph 20",
            ));
        }
        if generation < 19 && matches!(object.form, TypeForm::OwnedProduct { .. }) {
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
    validate_in_scope(
        read,
        ty,
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
    )
}

/// The exact defining package participates in symbolic contract-type admission.
pub(crate) fn validate_in_scope(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    let local_scope = scope
        .filter(|s| s.package == read.package_id())
        .map(|s| s.declaration);
    let ordinary_assumptions = super::transfer::ordinary_assumptions(read, local_scope)?;
    let code = match read.type_object(ty)?.map(|t| t.form) {
        Some(TypeForm::OwnedSequence { .. }) => "kernel_owned_sequence",
        Some(TypeForm::OwnedChoice { .. }) => "kernel_owned_choice",
        _ => "kernel_owned_product",
    };
    let reject = |message| Diagnostic::new(DiagnosticClass::Semantic, code, message);
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
        let (sequence_item, fields) = match object.form {
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                (None, fields)
            }
            TypeForm::OwnedSequence { item } => (Some(item), Vec::new()),
            _ => {
                return Err(reject(
                    "explicit composite operand requires an owned product, choice or sequence type",
                ));
            }
        };
        let require_owned = sequence_item.is_some();
        let mut owned = false;
        for child_type in sequence_item
            .into_iter()
            .chain(fields.into_iter().map(|field| field.ty))
        {
            read.validation_work()?;
            let child = read
                .type_object(child_type)?
                .ok_or_else(|| reject("missing owned product field type"))?;
            match child.form {
                TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. } => {
                    owned = true;
                    pending.push((child_type, depth + 1));
                }
                TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => owned = true,
                TypeForm::TypeParameter { parameter }
                    if super::memory::direct_in(
                        read,
                        scope.map_or(read.package_id(), |s| s.package),
                        child_type,
                    )? =>
                {
                    super::owned_contract::validate_owned_parameter(read, parameter, scope)?;
                    owned = true;
                }
                _ if require_owned => {
                    return Err(reject("owned sequence elements require exact owned types"));
                }
                _ if !super::owned_contract::ordinary_with_assumptions(
                    read,
                    child_type,
                    local_scope,
                    &ordinary_assumptions,
                )? =>
                {
                    return Err(reject(
                        "ordinary composite children require closed data or exact in-scope transferable data",
                    ));
                }
                _ => {}
            }
        }
        if !owned {
            return Err(reject(
                "owned structure requires at least one direct owned field or case",
            ));
        }
    }
    Ok(())
}
