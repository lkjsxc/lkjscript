use crate::verify::*;
use crate::Program;

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedProgram(Program);

impl VerifiedProgram {
    pub fn program(&self) -> &Program {
        &self.0
    }

    pub const fn prepared_identity(&self) -> lkjscript_contracts::PreparedProgramIdentity {
        self.0.prepared_identity
    }

    pub fn require_prepared_identity(
        &self,
        expected: lkjscript_contracts::PreparedProgramIdentity,
    ) -> crate::Result<()> {
        if !expected.is_bound() || self.0.prepared_identity != expected {
            return Err(crate::IrError::new(
                "SSA prepared program identity mismatch",
            ));
        }
        Ok(())
    }

    pub fn bind_prepared_identity(
        mut self,
        identity: lkjscript_contracts::PreparedProgramIdentity,
    ) -> crate::Result<Self> {
        if !identity.is_bound()
            || (self.0.prepared_identity.is_bound() && self.0.prepared_identity != identity)
        {
            return Err(crate::IrError::new(
                "SSA prepared program identity is zero or stale",
            ));
        }
        self.0.prepared_identity = identity;
        Ok(self)
    }
}

pub fn verify(program: Program) -> crate::Result<VerifiedProgram> {
    verify_program(&program)?;
    Ok(VerifiedProgram(program))
}
