use super::*;
use crate::platform::kernel::callable_flow::CallableClosureRead;

impl CallableClosureRead for Fixture {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for (package, owner) in self.owners.keys() {
            self.validation_checkpoint()?;
            if let OwnerKey::Declaration(declaration) = owner {
                visitor(DeclarationReference {
                    package: *package,
                    declaration: *declaration,
                })?;
            }
        }
        Ok(())
    }

    fn owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.validation_checkpoint()?;
        if super::super::IN_PROOF.with(Cell::get)
            && package == self.mapping.package
            && owner == OwnerKey::Declaration(self.mapping.declaration)
        {
            self.mapping_reads.set(self.mapping_reads.get() + 1);
        }
        Ok(self.owners.get(&(package, owner)).cloned())
    }

    fn type_object(
        &self,
        _: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        self.validation_checkpoint()?;
        Ok(self.types.get(&ty).cloned())
    }

    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        if super::super::IN_PROOF.with(Cell::get) {
            let count = self.proof_checkpoints.get() + 1;
            self.proof_checkpoints.set(count);
            if self.stop_at.get() == Some(count) {
                return Err(Diagnostic::new(
                    DiagnosticClass::Cancelled,
                    "input_proof_cancelled",
                    "controlled proof cancellation",
                ));
            }
        }
        Ok(())
    }
}
