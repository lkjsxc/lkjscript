//! Reconstruct finite generic composition from the complete admitted canonical package closure.
//! Public interfaces still own ordinary semantic lookup; private bodies are read only for this proof.
use super::*;
use crate::platform::kernel::callable_flow::{
    CallableClosureRead, CallableFlowWork, validate_callable_closure,
};

struct Read<'a> {
    reference: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    runtime: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
}

impl CallableClosureRead for Read<'_> {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for declaration in self
            .reference
            .keys()
            .chain(
                self.runtime
                    .keys()
                    .filter(|key| !self.reference.contains_key(key)),
            )
            .filter_map(|(package, owner)| match owner {
                OwnerKey::Declaration(declaration) => Some(DeclarationReference {
                    package: *package,
                    declaration: *declaration,
                }),
                _ => None,
            })
        {
            visitor(declaration)?;
        }
        Ok(())
    }

    fn owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self
            .reference
            .get(&(package, owner))
            .or_else(|| self.runtime.get(&(package, owner)))
            .cloned())
    }

    fn type_object(
        &self,
        _package: PackageId,
        digest: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self.types.get(&digest).cloned())
    }
}

pub(super) fn validate(
    reference: &BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    runtime: &BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
) -> Result<(usize, CallableFlowWork), Diagnostic> {
    let read = Read {
        reference,
        runtime,
        types,
    };
    let mut work = 0;
    let analysis = validate_callable_closure(
        &read,
        &mut work,
        crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
    )?;
    Ok((work, analysis))
}
