//! Disposable closed signature types. Derivation never writes accepted meaning.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::TypeParameterId;
use std::cell::RefCell;
use std::collections::BTreeMap;

pub(crate) struct AppliedTypes<'a, R: ?Sized> {
    read: &'a R,
    types: RefCell<BTreeMap<TypeObjectDigest, TypeObject>>,
    resolved: BTreeMap<TypeObjectDigest, TypeObjectDigest>,
}
impl<'a, R: ExpressionRead + ?Sized> AppliedTypes<'a, R> {
    pub(crate) fn into_types(self) -> BTreeMap<TypeObjectDigest, TypeObject> {
        self.types.into_inner()
    }
    pub(crate) fn new(read: &'a R) -> Self {
        Self {
            read,
            types: RefCell::new(BTreeMap::new()),
            resolved: BTreeMap::new(),
        }
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
        let mut child = |ty| self.substitute(ty, bindings, depth + 1);
        let form = match object.form {
            TypeForm::TypeParameter { parameter } => {
                return bindings.get(&parameter).copied().ok_or_else(|| {
                    super::parallel::reject("child signature contains a foreign type parameter")
                });
            }
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
                    .map(&mut child)
                    .collect::<Result<_, _>>()?,
            },
            TypeForm::List { item } => TypeForm::List { item: child(item)? },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: child(key)?,
                value: child(value)?,
            },
            TypeForm::Option { item } => TypeForm::Option { item: child(item)? },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: child(ok)?,
                error: child(error)?,
            },
            // Callables, streams and authority are rejected by complete transfer
            // admission; do not construct an application that could hide them.
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. } | TypeForm::Stream { .. } => {
                return Err(super::parallel::reject(
                    "child signature cannot contain callable or stream types",
                ));
            }
            other => other,
        };
        self.read.validation_work()?;
        let object = TypeObject::new(form)?;
        let result = encode_type_object(&object)?.0;
        self.types.borrow_mut().insert(result, object);
        self.resolved.insert(ty, result);
        Ok(result)
    }
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
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.read.validation_work()
    }
}
