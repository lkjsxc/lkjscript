//! Closed child signatures are recomputed from canonical records, not table claims.
use super::*;
use crate::platform::kernel::{ExpressionRead, PackageInterfaceRecord};
use std::cell::RefCell;

pub(super) struct Child {
    pub(super) function: u32,
    pub(super) arguments: u32,
    pub(super) result: TypeObjectDigest,
    pub(super) types: Vec<u32>,
    pub(super) implementations: Vec<crate::platform::kernel::ImplementationOperand>,
}

struct Read<'a, B: ?Sized> {
    source: &'a B,
    package: PackageId,
    work: RefCell<CanonicalReadWork>,
}
impl<B: CodeRead + ?Sized> ExpressionRead for Read<'_, B> {
    fn package_id(&self) -> PackageId {
        self.package
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        let r = self.source.code_owner(owner)?;
        self.work.borrow_mut().add(r.work);
        Ok(r.value)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        let r = self.source.code_type(ty)?;
        self.work.borrow_mut().add(r.work);
        Ok(r.value)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        let r = self.source.code_interface(package, owner)?;
        self.work.borrow_mut().add(r.work);
        Ok(r.value)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        let r = self.source.code_dependency(package)?;
        self.work.borrow_mut().add(r.work);
        Ok(r.value)
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.source.code_step()
    }
}

impl<B: CodeRead + ?Sized> UnitBuilder<'_, B> {
    pub(super) fn parallel_function_result(
        &mut self,
        function: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let read = Read {
            source: self.canonical,
            package: self.package,
            work: RefCell::new(CanonicalReadWork::default()),
        };
        let signature = crate::platform::kernel::function_contract(&read, function)?;
        if signature.type_parameters.len() != arguments.len() {
            return Err(compiler_corrupt(
                "compiler_parallel_function",
                "parallel child type arity differs from its canonical declaration",
            ));
        }
        let bindings = signature
            .type_parameters
            .into_iter()
            .zip(arguments.iter().copied())
            .collect();
        let mut applied = crate::platform::kernel::parallel_types::AppliedTypes::new(&read);
        let result = applied.substitute(signature.result, &bindings, 0)?;
        for (_, object) in applied.into_types() {
            self.canonical.code_step()?;
            if self.derived_type_objects.len() >= MAXIMUM_COMPILER_UNIT_ITEMS {
                return Err(compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_parallel_result_type_limit",
                    "closed child types exceed the compiler-unit bound",
                ));
            }
            let (ty, bytes) = encode_type_object(&object)?;
            self.derived_type_objects.insert(
                ObjectKey::from_digest(
                    crate::platform::storage::object::ObjectDomain::Type,
                    ty.bytes(),
                ),
                bytes,
            );
        }
        self.work.canonical.add(read.work.into_inner());
        Ok(result)
    }

    pub(super) fn parallel_type(&mut self, ty: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        let read = self.canonical.code_type(ty)?;
        self.work.canonical.add(read.work);
        if let Some(object) = read.value {
            return Ok(object);
        }
        let key = ObjectKey::from_digest(
            crate::platform::storage::object::ObjectDomain::Type,
            ty.bytes(),
        );
        let bytes = self.derived_type_objects.get(&key).ok_or_else(|| {
            compiler_corrupt(
                "compiler_parallel_result_type",
                "missing closed child result type",
            )
        })?;
        crate::platform::kernel::decode_type_object(bytes, ty)
    }
}
