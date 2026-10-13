//! Structural affine products. Ordinary metadata uses the existing closed-data proof.
use super::owned_associated::{OwnedAssociatedAdmission, OwnedTypeOccurrence, read_view_admission};
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::DeclarationId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_owned_product", message)
}

/// The language bound belongs to the enclosing owned structure, independently
/// of an associated-type traversal's configurable resource admission. Callers
/// with a derived overlay must supply that same overlay for this proof.
pub(crate) fn validate_owning_type_depth(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<(), Diagnostic> {
    let object = read
        .type_object(ty)?
        .ok_or_else(|| reject("missing owned depth type"))?;
    let code = match object.form {
        TypeForm::OwnedProduct { .. } => "kernel_owned_product",
        TypeForm::OwnedChoice { .. } => "kernel_owned_choice",
        TypeForm::OwnedSequence { .. } => "kernel_owned_sequence",
        TypeForm::Named { .. } | TypeForm::Applied { .. }
            if super::owned_record::is_owned(read, read.package_id(), ty)? =>
        {
            return super::owned_record::validate_type_depth(read, ty, 0);
        }
        _ => return Ok(()),
    };
    validate_composite_depth(read, ty, code)
}

fn validate_composite_depth(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    code: &str,
) -> Result<(), Diagnostic> {
    // One path can cross both structural syntax and private nominal layouts.
    // Keep the enclosing composite diagnostic while using the shared depth proof.
    super::owned_record::validate_type_depth(read, ty, 0).map_err(|mut diagnostic| {
        if diagnostic.code == "kernel_owned_record_depth" {
            diagnostic.code = code.to_owned();
        }
        diagnostic
    })
}

/// An extension type cannot acquire older graph authority merely by using old
/// expression tags (for example a product-only relay consisting of Local).
pub(crate) fn require_generation(
    read: &(impl ExpressionRead + ?Sized),
    roots: Vec<TypeObjectDigest>,
    generation: u16,
) -> Result<(), Diagnostic> {
    if generation >= contract::GRAPH_CONTRACT_VERSION {
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
        if object.form.is_read_view() {
            read_view_admission::require_generation(generation)?;
        }
        if generation < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION
            && object.form.is_owned_associated()
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_owned_associated_generation",
                "associated type closure requires Graph 33",
            ));
        }
        if generation < 32 && super::owned_record::is_owned(read, read.package_id(), ty)? {
            return Err(super::owned_record::reject(
                "nominal owned record type closure requires Graph 32",
            ));
        }
        if generation < 31 && requires_generalized_sequence_generation(read, ty, read.package_id())?
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_sequence_generation",
                "ordinary-element owned sequence type closure requires Graph 31",
            ));
        }
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

/// Classify the element under its defining package. Complete element admission
/// is separate: an inadmissible ordinary candidate cannot acquire older authority.
pub(crate) fn requires_generalized_sequence_generation(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    package: PackageId,
) -> Result<bool, Diagnostic> {
    let Some(object) = read.type_object(ty)? else {
        return Err(reject("missing sequence generation type"));
    };
    let TypeForm::OwnedSequence { item } = object.form else {
        return Ok(false);
    };
    Ok(!super::memory::direct_in(read, package, item)?)
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
    let ordinary_assumptions = super::transfer::ordinary_assumptions_in(read, scope)?;
    let code = match read.type_object(ty)?.map(|t| t.form) {
        Some(TypeForm::OwnedSequence { .. }) => "kernel_owned_sequence",
        Some(TypeForm::OwnedChoice { .. }) => "kernel_owned_choice",
        _ => "kernel_owned_product",
    };
    let reject = |message| Diagnostic::new(DiagnosticClass::Semantic, code, message);
    validate_composite_depth(read, ty, code)?;
    if read_view_admission::contains(read, ty, &OwnedAssociatedAdmission::new())? {
        return Err(reject("owned composites cannot contain read views"));
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
        // The sequence allocation owns its custody independently of element
        // ownership and independently of its dynamic cardinality.
        let mut owned = sequence_item.is_some();
        for child_type in sequence_item
            .into_iter()
            .chain(fields.into_iter().map(|field| field.ty))
        {
            read.validation_work()?;
            let child = read
                .type_object(child_type)?
                .ok_or_else(|| reject("missing owned product field type"))?;
            match child.form {
                TypeForm::Named { .. } | TypeForm::Applied { .. }
                    if super::owned_record::is_owned(
                        read,
                        scope.map_or(read.package_id(), |s| s.package),
                        child_type,
                    )? =>
                {
                    super::owned_record::validate_application(read, child_type, scope)?;
                    owned = true;
                }
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
                _ if !super::owned_contract::ordinary_with_assumptions_in(
                    read,
                    child_type,
                    scope,
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

/// Complete composite admission retains caller authority for actuals. Raw
/// nominal templates use their explicit representation context in record proof.
pub(crate) fn validate_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let code = match read.type_object(ty)?.map(|object| object.form) {
        Some(TypeForm::OwnedSequence { .. }) => "kernel_owned_sequence",
        Some(TypeForm::OwnedChoice { .. }) => "kernel_owned_choice",
        _ => "kernel_owned_product",
    };
    let reject = |message| Diagnostic::new(DiagnosticClass::Semantic, code, message);
    validate_composite_depth(read, ty, code)?;
    associated.validate_occurrence_at(read, &[ty], occurrence)?;
    if read_view_admission::contains(read, ty, associated)? {
        return Err(reject("owned composites cannot contain read views"));
    }
    associated.reserve::<TypeObjectDigest>(1)?;
    require_generation(read, vec![ty], occurrence.consumer_generation)?;
    let assumptions = super::transfer::ordinary_assumptions_at(read, occurrence.parameters)?;
    associated.reserve::<(TypeObjectDigest, usize)>(1)?;
    let mut pending = vec![(ty, 0usize)];
    let mut depths = BTreeMap::new();
    while let Some((ty, depth)) = pending.pop() {
        read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(reject(
                "owned composite exceeds the structural type depth bound",
            ));
        }
        if depths.get(&ty).is_some_and(|previous| *previous >= depth) {
            continue;
        }
        if !depths.contains_key(&ty) {
            associated.reserve::<(TypeObjectDigest, usize)>(1)?;
        }
        depths.insert(ty, depth);
        let object = read
            .type_object(ty)?
            .ok_or_else(|| reject("missing owned composite type"))?;
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
        let mut owned = sequence_item.is_some();
        for child in sequence_item
            .into_iter()
            .chain(fields.into_iter().map(|field| field.ty))
        {
            read.validation_work()?;
            let form = read
                .type_object(child)?
                .ok_or_else(|| reject("missing owned composite child"))?
                .form;
            match form {
                TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. } => {
                    owned = true;
                    associated.reserve::<(TypeObjectDigest, usize)>(1)?;
                    pending.push((child, depth + 1));
                }
                _ if super::memory::direct_at(read, child, occurrence, associated)? => {
                    super::owned_contract::validate_owned_type_at(
                        read, child, occurrence, associated,
                    )?;
                    owned = true;
                }
                _ if !super::owned_contract::ordinary_with_assumptions_at(
                    read,
                    child,
                    occurrence.parameters,
                    &assumptions,
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
