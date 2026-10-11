//! Exact nominal custody and module-owned representation. Layout is derived, never structural identity.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{FieldId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "owned_record_depth_tests.rs"]
mod depth_tests;

const MAXIMUM_LAYOUT_METADATA_BYTES: usize = 64 * 1_048_576;

/// An internal property proof may inspect the exact transitive evidence retained
/// by admitted imports. The caller's package and lexical parameter scope stay
/// unchanged; this reader is never used for authored-name/reference validation.
pub(crate) struct RepresentationRead<'a, R: ?Sized> {
    pub(crate) read: &'a R,
}

impl<R: ExpressionRead + ?Sized> ExpressionRead for RepresentationRead<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.read.owner(owner)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.read.type_object(ty)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_representation_owner(package, owner)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_representation_dependency(package)
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_representation_owner(package, owner)
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_representation_dependency(package)
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.read.validation_work()
    }
}

/// Cumulative admission for derived traversal storage, separate from proof work.
pub(crate) struct LayoutBudget {
    bytes: usize,
    maximum_bytes: usize,
}

impl Default for LayoutBudget {
    fn default() -> Self {
        Self {
            bytes: 0,
            maximum_bytes: MAXIMUM_LAYOUT_METADATA_BYTES,
        }
    }
}

impl LayoutBudget {
    #[cfg(test)]
    fn with_metadata_limit(maximum_bytes: usize) -> Self {
        Self {
            maximum_bytes,
            ..Self::default()
        }
    }

    fn reserve_bytes(&mut self, bytes: usize) -> Result<(), Diagnostic> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|bytes| *bytes <= self.maximum_bytes)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_owned_record_storage",
                    "owned representation metadata exceeds its storage admission",
                )
            })?;
        Ok(())
    }

    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_owned_record_storage",
                    "owned representation metadata size overflow",
                )
            })?;
        self.reserve_bytes(bytes)
    }

    pub(crate) fn extend_types(
        &mut self,
        target: &mut BTreeMap<TypeObjectDigest, TypeObject>,
        source: BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<(), Diagnostic> {
        for (digest, object) in source {
            if target.contains_key(&digest) {
                continue;
            }
            self.reserve::<(TypeObjectDigest, TypeObject)>(1)?;
            self.reserve::<TypeObjectDigest>(object.child_type_count())?;
            match &object.form {
                TypeForm::StructuralRecord { fields }
                | TypeForm::OwnedProduct { fields }
                | TypeForm::OwnedChoice { cases: fields } => {
                    self.reserve::<StructuralTypeField>(fields.len())?;
                    for field in fields {
                        self.reserve_bytes(field.name.as_str().len())?;
                    }
                }
                _ => {}
            }
            target.insert(digest, object);
        }
        Ok(())
    }
}

pub(crate) fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_owned_record", message)
}

pub(crate) struct AppliedOwnedRecord {
    pub(crate) fields: Vec<(FieldReference, TypeObjectDigest)>,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
}

pub(crate) fn nominal(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<(DeclarationReference, Vec<TypeObjectDigest>), Diagnostic> {
    read.validation_work()?;
    match read
        .type_object(ty)?
        .ok_or_else(|| reject("missing owned record type"))?
        .form
    {
        TypeForm::Named { declaration } => Ok((declaration, Vec::new())),
        TypeForm::Applied {
            declaration,
            arguments,
        } => Ok((declaration, arguments)),
        _ => Err(reject(
            "operation requires an exact nominal owned record type",
        )),
    }
}

pub(crate) fn definition(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<(Vec<TypeParameterId>, Vec<FieldId>), Diagnostic> {
    read.validation_work()?;
    if reference.package == read.package_id() {
        if let Some(OwnerRecord::Declaration(record)) =
            read.owner(OwnerKey::Declaration(reference.declaration))?
            && record.header.contract_version >= 32
            && let DeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } = record.payload
        {
            return Ok((type_parameters, fields));
        }
    } else if read.has_representation_dependency(reference.package)?
        && let Some(
            PackageInterfaceRecord::Declaration(record)
            | PackageInterfaceRecord::RepresentationDeclaration(record),
        ) = read.package_representation_owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )?
        && record.header.contract_version >= 32
        && let PackageInterfaceDeclarationPayload::OwnedRecord {
            type_parameters,
            fields,
        } = record.payload
    {
        return Ok((type_parameters, fields));
    }
    Err(reject("missing admitted nominal owned record declaration"))
}

pub(crate) fn is_owned(
    read: &(impl ExpressionRead + ?Sized),
    _package: PackageId,
    ty: TypeObjectDigest,
) -> Result<bool, Diagnostic> {
    read.validation_work()?;
    let Some(object) = read.type_object(ty)? else {
        return Err(reject("missing owned type"));
    };
    let declaration = match object.form {
        TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => declaration,
        _ => return Ok(false),
    };
    if declaration.package == read.package_id() {
        Ok(
            matches!(read.owner(OwnerKey::Declaration(declaration.declaration))?,
            Some(OwnerRecord::Declaration(record)) if matches!(record.payload, DeclarationPayload::OwnedRecord { .. })),
        )
    } else {
        if !read.has_representation_dependency(declaration.package)? {
            return Err(reject("owned type is outside the exact package closure"));
        }
        Ok(
            matches!(read.package_representation_owner(declaration.package, OwnerKey::Declaration(declaration.declaration))?,
            Some(PackageInterfaceRecord::Declaration(record) | PackageInterfaceRecord::RepresentationDeclaration(record)) if matches!(record.payload, PackageInterfaceDeclarationPayload::OwnedRecord { .. })),
        )
    }
}

pub(crate) fn field(
    read: &(impl ExpressionRead + ?Sized),
    reference: FieldReference,
) -> Result<FieldRecord, Diagnostic> {
    read.validation_work()?;
    if reference.package == read.package_id() {
        if let Some(OwnerRecord::Field(field)) = read.owner(OwnerKey::Field(reference.field))? {
            return Ok(field);
        }
    } else if read.has_representation_dependency(reference.package)?
        && let Some(PackageInterfaceRecord::Field(field)) =
            read.package_representation_owner(reference.package, OwnerKey::Field(reference.field))?
    {
        return Ok(field);
    }
    Err(reject("missing exact owned record field"))
}

pub(crate) fn parameter(
    read: &(impl ExpressionRead + ?Sized),
    scope: DeclarationReference,
    id: TypeParameterId,
) -> Result<TypeParameterRecord, Diagnostic> {
    read.validation_work()?;
    let record = if scope.package == read.package_id() {
        match read.owner(OwnerKey::TypeParameter(id))? {
            Some(OwnerRecord::TypeParameter(record)) => record,
            _ => return Err(reject("missing owned record type parameter")),
        }
    } else {
        match read.package_representation_owner(scope.package, OwnerKey::TypeParameter(id))? {
            Some(PackageInterfaceRecord::TypeParameter(record)) => record,
            _ => return Err(reject("missing imported owned record type parameter")),
        }
    };
    if record.declaration != scope.declaration || record.header.owner != OwnerKey::TypeParameter(id)
    {
        return Err(reject(
            "owned record parameter escapes its exact declaration",
        ));
    }
    Ok(record)
}

pub(crate) fn layout(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<AppliedOwnedRecord, Diagnostic> {
    let (declaration, arguments) = nominal(read, ty)?;
    let (parameters, fields) = definition(read, declaration)?;
    if parameters.len() != arguments.len() {
        return Err(reject(
            "owned record application requires all ordered arguments",
        ));
    }
    let mut budget = LayoutBudget::default();
    budget.reserve::<(TypeParameterId, TypeObjectDigest)>(parameters.len())?;
    budget.reserve::<(FieldReference, TypeObjectDigest)>(fields.len())?;
    let substitutions = parameters
        .into_iter()
        .zip(arguments)
        .collect::<BTreeMap<_, _>>();
    let mut derived = super::parallel_types::AppliedTypes::new(read);
    let mut resolved = Vec::with_capacity(fields.len());
    for id in fields {
        read.validation_work()?;
        let reference = FieldReference {
            package: declaration.package,
            field: id,
        };
        let field = field(read, reference)?;
        if field.declaration != declaration.declaration {
            return Err(reject("owned record field belongs to another declaration"));
        }
        resolved.push((reference, derived.substitute(field.ty, &substitutions, 0)?));
    }
    Ok(AppliedOwnedRecord {
        fields: resolved,
        types: derived.into_types(),
    })
}

/// Count one global path through syntactic children and fully instantiated owned
/// representation fields. Independent local schema limits do not bound a combined path.
pub(crate) fn validate_type_depth(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    initial_depth: usize,
) -> Result<(), Diagnostic> {
    let mut budget = LayoutBudget::default();
    budget.reserve::<(TypeObjectDigest, usize)>(1)?;
    let mut pending = vec![(ty, initial_depth)];
    let mut depths = BTreeMap::new();
    let mut derived_types = BTreeMap::new();
    while let Some((current, depth)) = pending.pop() {
        read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_owned_record_depth",
                "combined owned representation exceeds the type-depth bound",
            ));
        }
        if depths
            .get(&current)
            .is_some_and(|previous| *previous >= depth)
        {
            continue;
        }
        if !depths.contains_key(&current) {
            budget.reserve::<(TypeObjectDigest, usize)>(1)?;
        }
        depths.insert(current, depth);
        let applied = super::owned_contract::AppliedTypeRead {
            read,
            types: &derived_types,
        };
        let read: &dyn ExpressionRead = &applied;
        let object = read
            .type_object(current)?
            .ok_or_else(|| reject("missing owned representation child type"))?;
        budget.reserve::<(TypeObjectDigest, usize)>(object.child_type_count())?;
        for child in object.child_types() {
            pending.push((child, depth + 1));
        }
        if is_owned(read, read.package_id(), current)? {
            let layout = layout(read, current)?;
            budget.reserve::<(TypeObjectDigest, usize)>(layout.fields.len())?;
            for (_, child) in layout.fields {
                pending.push((child, depth + 1));
            }
            budget.extend_types(&mut derived_types, layout.types)?;
        }
    }
    Ok(())
}

pub(crate) fn require_access(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    let (declaration, _) = nominal(read, ty)?;
    definition(read, declaration)?;
    let scope = scope.ok_or_else(|| {
        reject("owned representation access requires an exact enclosing declaration")
    })?;
    if scope.package != read.package_id() || declaration.package != scope.package {
        return Err(reject(
            "owned representation belongs to another package or module",
        ));
    }
    let module = |reference: DeclarationReference| -> Result<_, Diagnostic> {
        read.validation_work()?;
        match read.owner(OwnerKey::Declaration(reference.declaration))? {
            Some(OwnerRecord::Declaration(record)) => Ok(record.module),
            _ => Err(reject("missing exact representation access scope")),
        }
    };
    if module(scope)? != module(declaration)? {
        return Err(reject(
            "owned representation belongs to another package or module",
        ));
    }
    Ok(())
}

pub(crate) fn validate_application(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    let representation = RepresentationRead { read };
    let read: &dyn ExpressionRead = &representation;
    let (declaration, arguments) = nominal(read, ty)?;
    validate_definition(read, declaration)?;
    validate_type_depth(read, ty, 0)?;
    let (parameters, _) = definition(read, declaration)?;
    if parameters.len() != arguments.len() {
        return Err(reject(
            "owned record application requires all ordered arguments",
        ));
    }
    let assumptions = super::transfer::ordinary_assumptions_in(read, scope)?;
    for (id, ty) in parameters.into_iter().zip(arguments) {
        read.validation_work()?;
        let formal = parameter(read, declaration, id)?;
        if formal.constraints.has_owned() {
            super::owned_contract::validate_owned_type(read, ty, scope)?;
        } else if !super::owned_contract::ordinary_with_assumptions_in(
            read,
            ty,
            scope,
            &assumptions,
        )? {
            return Err(reject(
                "ordinary owned-record arguments require first-order Transferable data",
            ));
        }
        if formal.constraints.requires_transfer() {
            super::transfer::admit_in(read, ty, scope)?;
        }
        if formal.constraints.requires_share() {
            super::share::admit_in(read, ty, scope)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_definition(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<(), Diagnostic> {
    let representation = RepresentationRead { read };
    let read: &dyn ExpressionRead = &representation;
    let (parameters, fields) = definition(read, reference)?;
    let mut budget = LayoutBudget::default();
    let mut names = BTreeSet::new();
    for id in parameters {
        let parameter = parameter(read, reference, id)?;
        budget.reserve::<Name>(1)?;
        budget.reserve_bytes(parameter.name.as_str().len())?;
        if !names.insert(parameter.name)
            || !(parameter.constraints.has_owned() || parameter.constraints.requires_transfer())
        {
            return Err(reject(
                "owned records require distinct Owned or ordinary Transferable parameters",
            ));
        }
    }
    validate_representation_cycles(read, reference, &mut budget)?;
    let assumptions = super::transfer::ordinary_assumptions_in(read, Some(reference))?;
    for id in fields {
        let field = field(
            read,
            FieldReference {
                package: reference.package,
                field: id,
            },
        )?;
        if field.declaration != reference.declaration {
            return Err(reject("owned record field belongs to another declaration"));
        }
        validate_type_depth(read, field.ty, 1)?;
        if super::memory::direct_in(read, reference.package, field.ty)? {
            super::owned_contract::validate_owned_type(read, field.ty, Some(reference))?;
        } else if !super::owned_contract::ordinary_with_assumptions_in(
            read,
            field.ty,
            Some(reference),
            &assumptions,
        )? {
            return Err(reject(
                "owned record fields require exact Owned or ordinary first-order data",
            ));
        }
    }
    Ok(())
}

/// Follow declaration references independently of argument substitution. Active
/// declarations detect cycles; completed declarations let shared tails be proved
/// once. Instantiated type depth remains a separate check over exact type objects.
fn validate_representation_cycles(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    budget: &mut LayoutBudget,
) -> Result<(), Diagnostic> {
    budget.reserve::<(DeclarationReference, bool)>(1)?;
    let mut pending = vec![(reference, false)];
    let mut active = BTreeSet::new();
    let mut completed = BTreeSet::new();
    while let Some((current, exiting)) = pending.pop() {
        read.validation_work()?;
        if exiting {
            active.remove(&current);
            budget.reserve::<DeclarationReference>(1)?;
            completed.insert(current);
            continue;
        }
        if completed.contains(&current) {
            continue;
        }
        if active.contains(&current) {
            return Err(reject(
                "recursive owned record representations are unsupported",
            ));
        }
        budget.reserve::<DeclarationReference>(1)?;
        active.insert(current);
        budget.reserve::<(DeclarationReference, bool)>(1)?;
        pending.push((current, true));
        let (_, fields) = definition(read, current)?;
        budget.reserve::<TypeObjectDigest>(fields.len())?;
        let mut types = Vec::new();
        for id in fields {
            types.push(
                field(
                    read,
                    FieldReference {
                        package: current.package,
                        field: id,
                    },
                )?
                .ty,
            );
        }
        let mut seen = BTreeSet::new();
        while let Some(ty) = types.pop() {
            read.validation_work()?;
            if seen.contains(&ty) {
                continue;
            }
            budget.reserve::<TypeObjectDigest>(1)?;
            seen.insert(ty);
            let object = read
                .type_object(ty)?
                .ok_or_else(|| reject("missing owned representation type"))?;
            budget.reserve::<TypeObjectDigest>(object.child_type_count())?;
            types.extend(object.child_types());
            if is_owned(read, current.package, ty)? {
                let (target, _) = nominal(read, ty)?;
                budget.reserve::<(DeclarationReference, bool)>(1)?;
                pending.push((target, false));
            }
        }
    }
    Ok(())
}
