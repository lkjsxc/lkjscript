//! Disposable closed signature types. Derivation never writes accepted meaning.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::TypeParameterId;
use std::cell::RefCell;
use std::collections::BTreeMap;

const MAXIMUM_METADATA_BYTES: usize = 64 * 1_048_576;

/// All selections belong to this one immutable application. Operands have already
/// been rebound into caller context, and their derived actual objects must be in
/// `read`. This is substitution input, not a certificate of callable admission.
pub(crate) struct AssociatedTypeEnvironment<'a> {
    pub ordinary: &'a BTreeMap<TypeParameterId, TypeObjectDigest>,
    pub source: owned_associated::AssociatedTypeContext,
    pub caller: owned_associated::AssociatedTypeContext,
    pub consumer_generation: u16,
    pub contract_operand: Option<&'a ImplementationOperand>,
    pub implementations: &'a BTreeMap<
        crate::platform::semantic_id::ImplementationParameterId,
        ImplementationOperand,
    >,
}

/// Witness-sensitive resolution is scoped by construction to a complete borrowed
/// environment. In particular it never uses the ordinary-only empty-map shortcut.
pub(crate) struct WitnessAppliedTypes<'a, 'e, R: ?Sized> {
    derived: AppliedTypes<'a, R>,
    environment: AssociatedTypeEnvironment<'e>,
    admission: owned_associated::OwnedAssociatedAdmission,
    resolved: BTreeMap<TypeObjectDigest, TypeObjectDigest>,
}

impl<'a, 'e, R: ExpressionRead + ?Sized> WitnessAppliedTypes<'a, 'e, R> {
    pub(crate) fn new(read: &'a R, environment: AssociatedTypeEnvironment<'e>) -> Self {
        Self {
            derived: AppliedTypes::new(read),
            environment,
            admission: owned_associated::OwnedAssociatedAdmission::new(),
            resolved: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_metadata_limit(
        read: &'a R,
        environment: AssociatedTypeEnvironment<'e>,
        maximum_metadata_bytes: usize,
    ) -> Self {
        Self {
            derived: AppliedTypes::with_metadata_limit(read, maximum_metadata_bytes),
            ..Self::new(read, environment)
        }
    }

    #[cfg(test)]
    pub(crate) fn metadata_bytes_for_test(&self) -> usize {
        self.derived.metadata_bytes_for_test()
    }

    pub(crate) fn into_types(self) -> BTreeMap<TypeObjectDigest, TypeObject> {
        self.derived.into_types()
    }

    pub(crate) fn substitute(
        &mut self,
        ty: TypeObjectDigest,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.derived.read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH
            || self.resolved.len() >= contract::MAXIMUM_VALIDATION_WORK
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_parallel_types",
                "witness-selected signature exceeds finite type preparation",
            ));
        }
        if let Some(result) = self.resolved.get(&ty) {
            return Ok(*result);
        }
        let object = self
            .derived
            .type_object(ty)?
            .ok_or_else(|| super::parallel::reject("missing witness-selected signature type"))?;
        if let TypeForm::TypeParameter { parameter } = object.form {
            // Simultaneous replacements are opaque caller actuals. Recursing into
            // one here would capture the caller's parameters or projections.
            return self
                .environment
                .ordinary
                .get(&parameter)
                .copied()
                .ok_or_else(|| {
                    super::parallel::reject(
                        "witness-selected signature has an unbound type parameter",
                    )
                });
        }
        // This also reserves the resolution entry when a selected binding returns
        // an existing digest and creates no new type object at this level.
        self.derived.reserve_type(&object)?;
        let result = if object.form.is_associated() {
            let operand = match &object.form {
                TypeForm::AssociatedContractMember { .. } => self.environment.contract_operand,
                TypeForm::AssociatedWitnessProjection { parameter, .. } => {
                    self.environment.implementations.get(parameter)
                }
                _ => unreachable!("associated forms were checked"),
            }
            .ok_or_else(|| {
                super::parallel::reject("associated member has no exact selected witness")
            })?;
            let resolution = self.admission.resolve_form(
                &self.derived,
                &object.form,
                self.environment.source,
                operand,
                self.environment.caller,
                self.environment.consumer_generation,
            )?;
            match resolution {
                owned_associated::AssociatedResolution::Binding {
                    template, bindings, ..
                } => {
                    // The helper admitted the complete projection-free stored RHS.
                    // Only its own formals are replaced; inserted caller actuals
                    // are never reprocessed under the source context.
                    self.derived.substitute(template, &bindings, depth + 1)?
                }
                owned_associated::AssociatedResolution::Lexical(form) => self.intern(form)?,
            }
        } else {
            let mut child = |ty| self.substitute(ty, depth + 1);
            let form = map_children(object.form, &mut child)?;
            self.intern(form)?
        };
        self.resolved.insert(ty, result);
        Ok(result)
    }

    fn intern(&mut self, form: TypeForm) -> Result<TypeObjectDigest, Diagnostic> {
        self.derived.read.validation_work()?;
        let object = TypeObject::new(form)?;
        let digest = encode_type_object(&object)?.0;
        self.derived.types.borrow_mut().insert(digest, object);
        Ok(digest)
    }
}

pub(crate) struct AppliedTypes<'a, R: ?Sized> {
    read: &'a R,
    types: RefCell<BTreeMap<TypeObjectDigest, TypeObject>>,
    bindings: Option<BTreeMap<TypeParameterId, TypeObjectDigest>>,
    resolved: BTreeMap<TypeObjectDigest, TypeObjectDigest>,
    metadata_bytes: usize,
    maximum_metadata_bytes: usize,
}
impl<'a, R: ExpressionRead + ?Sized> AppliedTypes<'a, R> {
    pub(crate) fn into_types(self) -> BTreeMap<TypeObjectDigest, TypeObject> {
        self.types.into_inner()
    }
    pub(crate) fn new(read: &'a R) -> Self {
        Self {
            read,
            types: RefCell::new(BTreeMap::new()),
            bindings: None,
            resolved: BTreeMap::new(),
            metadata_bytes: 0,
            maximum_metadata_bytes: MAXIMUM_METADATA_BYTES,
        }
    }
    #[cfg(test)]
    pub(crate) fn with_metadata_limit(read: &'a R, maximum_metadata_bytes: usize) -> Self {
        Self {
            maximum_metadata_bytes,
            ..Self::new(read)
        }
    }
    #[cfg(test)]
    pub(crate) fn metadata_bytes_for_test(&self) -> usize {
        self.metadata_bytes
    }

    fn reserve_type(&mut self, object: &TypeObject) -> Result<(), Diagnostic> {
        self.read.validation_work()?;
        let (children, names) = match &object.form {
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => (
                fields
                    .len()
                    .checked_mul(std::mem::size_of::<StructuralTypeField>()),
                fields.iter().try_fold(0usize, |bytes, field| {
                    bytes.checked_add(field.name.as_str().len())
                }),
            ),
            _ => (
                object
                    .child_type_count()
                    .checked_mul(std::mem::size_of::<TypeObjectDigest>()),
                Some(0),
            ),
        };
        // Admit the rebuilt child vector and names, both tree-map entries and
        // temporary canonical encoding before constructing any derived storage.
        // This is cumulative modeled metadata, not allocator traffic or RSS.
        let bytes = children
            .and_then(|n| names.and_then(|m| n.checked_add(m)))
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<TypeObject>()
                        + 3 * std::mem::size_of::<TypeObjectDigest>()
                        + 8 * std::mem::size_of::<usize>()
                        + 256,
                )
            })
            .ok_or_else(storage_exhausted)?;
        self.reserve_metadata(bytes)
    }

    fn reserve_metadata(&mut self, bytes: usize) -> Result<(), Diagnostic> {
        let bytes = self
            .metadata_bytes
            .checked_add(bytes)
            .filter(|n| *n <= self.maximum_metadata_bytes)
            .ok_or_else(storage_exhausted)?;
        self.metadata_bytes = bytes;
        Ok(())
    }

    fn activate_bindings(
        &mut self,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<(), Diagnostic> {
        if bindings.len() > contract::MAXIMUM_CHILDREN {
            return Err(storage_exhausted());
        }
        // Comparisons and copying must remain cancellable even when the map is
        // unchanged. Compare once per signature root, never once per child.
        let mut same = self
            .bindings
            .as_ref()
            .is_some_and(|previous| previous.len() == bindings.len());
        for (parameter, actual) in bindings {
            self.read.validation_work()?;
            same &= self
                .bindings
                .as_ref()
                .and_then(|previous| previous.get(parameter))
                == Some(actual);
        }
        if !same {
            // Retain derived objects for returned digests, but never reuse a
            // resolution computed under a different simultaneous substitution.
            let bytes = bindings
                .len()
                .checked_mul(
                    std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                        + 4 * std::mem::size_of::<usize>(),
                )
                .ok_or_else(storage_exhausted)?;
            self.reserve_metadata(bytes)?;
            self.bindings = Some(bindings.clone());
            self.resolved.clear();
        }
        Ok(())
    }

    pub(crate) fn substitute(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.read.validation_work()?;
        if bindings.is_empty() {
            return Ok(ty);
        }
        self.activate_bindings(bindings)?;
        self.substitute_inner(ty, bindings, depth)
    }

    fn substitute_inner(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.read.validation_work()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH
            || self.resolved.len() >= contract::MAXIMUM_VALIDATION_WORK
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_parallel_types",
                "closed child signature exceeds finite type preparation",
            ));
        }
        if let Some(resolved) = self.resolved.get(&ty) {
            return Ok(*resolved);
        }
        let object = self
            .type_object(ty)?
            .ok_or_else(|| super::parallel::reject("missing child signature type"))?;
        if let TypeForm::TypeParameter { parameter } = object.form {
            return bindings.get(&parameter).copied().ok_or_else(|| {
                super::parallel::reject("child signature contains a foreign type parameter")
            });
        }
        self.reserve_type(&object)?;
        let mut child = |ty| self.substitute_inner(ty, bindings, depth + 1);
        // Closed parallel signatures preserve their existing restricted domain.
        if matches!(
            object.form,
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. } | TypeForm::Stream { .. }
        ) {
            return Err(super::parallel::reject(
                "child signature cannot contain callable or stream types",
            ));
        }
        let form = map_children(object.form, &mut child)?;
        self.read.validation_work()?;
        let object = TypeObject::new(form)?;
        let result = encode_type_object(&object)?.0;
        self.types.borrow_mut().insert(result, object);
        self.resolved.insert(ty, result);
        Ok(result)
    }
}

fn storage_exhausted() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "kernel_parallel_type_storage",
        "substituted signature metadata exceeds its storage admission",
    )
}
impl<R: ExpressionRead + ?Sized> ExpressionRead for AppliedTypes<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.read.owner(owner)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self
            .read
            .type_object(ty)?
            .or_else(|| self.types.borrow().get(&ty).cloned()))
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_interface_owner(package, owner)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_dependency(package)
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

// Rebuilding does not admit a type's category, lexical scope or reference rights.
// Callers establish those boundaries and handle parameters/projections first.
fn map_children(
    form: TypeForm,
    child: &mut impl FnMut(TypeObjectDigest) -> Result<TypeObjectDigest, Diagnostic>,
) -> Result<TypeForm, Diagnostic> {
    Ok(match form {
        TypeForm::StructuralRecord { fields } => TypeForm::StructuralRecord {
            fields: fields
                .into_iter()
                .map(|f| {
                    Ok(StructuralTypeField {
                        name: f.name,
                        ty: child(f.ty)?,
                    })
                })
                .collect::<Result<_, Diagnostic>>()?,
        },
        TypeForm::OwnedProduct { fields } => TypeForm::OwnedProduct {
            fields: fields
                .into_iter()
                .map(|f| {
                    Ok(StructuralTypeField {
                        name: f.name,
                        ty: child(f.ty)?,
                    })
                })
                .collect::<Result<_, Diagnostic>>()?,
        },
        TypeForm::OwnedChoice { cases } => TypeForm::OwnedChoice {
            cases: cases
                .into_iter()
                .map(|f| {
                    Ok(StructuralTypeField {
                        name: f.name,
                        ty: child(f.ty)?,
                    })
                })
                .collect::<Result<_, Diagnostic>>()?,
        },
        TypeForm::Applied {
            declaration,
            arguments,
        } => TypeForm::Applied {
            declaration,
            arguments: arguments
                .into_iter()
                .map(&mut *child)
                .collect::<Result<_, _>>()?,
        },
        TypeForm::List { item } => TypeForm::List { item: child(item)? },
        TypeForm::OwnedSequence { item } => TypeForm::OwnedSequence { item: child(item)? },
        TypeForm::Map { key, value } => TypeForm::Map {
            key: child(key)?,
            value: child(value)?,
        },
        TypeForm::Option { item } => TypeForm::Option { item: child(item)? },
        TypeForm::Result { ok, error } => TypeForm::Result {
            ok: child(ok)?,
            error: child(error)?,
        },
        TypeForm::Function { parameters, result } => TypeForm::Function {
            parameters: parameters
                .into_iter()
                .map(&mut *child)
                .collect::<Result<_, _>>()?,
            result: child(result)?,
        },
        TypeForm::TaskFunction {
            parameters,
            result,
            effect,
        } => TypeForm::TaskFunction {
            parameters: parameters
                .into_iter()
                .map(&mut *child)
                .collect::<Result<_, _>>()?,
            result: child(result)?,
            effect,
        },
        TypeForm::Stream { item } => TypeForm::Stream { item: child(item)? },
        TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. } => {
            return Err(super::owned_associated_wire::unsupported_semantics());
        }
        TypeForm::TypeParameter { .. } => {
            return Err(super::parallel::reject("unsubstituted signature parameter"));
        }
        other => other,
    })
}
