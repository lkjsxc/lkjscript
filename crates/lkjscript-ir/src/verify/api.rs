use crate::verify::*;
use crate::Program;

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedProgram(Program);

impl VerifiedProgram {
    pub fn program(&self) -> &Program {
        &self.0
    }

    pub fn into_program(self) -> Program {
        self.0
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
}

pub fn bind_prepared_identity(
    verified: VerifiedProgram,
    identity: lkjscript_contracts::PreparedProgramIdentity,
) -> crate::Result<VerifiedProgram> {
    if !identity.is_bound()
        || (verified.0.prepared_identity.is_bound() && verified.0.prepared_identity != identity)
    {
        return Err(crate::IrError::new(
            "SSA prepared program identity is zero or stale",
        ));
    }
    let mut program = verified.into_program();
    program.prepared_identity = identity;
    verify_program(&program)?;
    Ok(VerifiedProgram(program))
}

pub fn verify(program: Program) -> crate::Result<VerifiedProgram> {
    verify_program(&program)?;
    Ok(VerifiedProgram(program))
}

pub const TRAIT_VERIFY_MAX_DEPTH: usize = 32;
pub const TRAIT_VERIFY_MAX_WORK: usize = 256;
pub const SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION: usize = 4_096;
pub const SSA_VERIFY_MAX_CFG_WORK: usize = 4_194_304;
pub(crate) const TYPE_VERIFY_MAX_DEPTH: usize = 64;
pub(crate) const TYPE_VERIFY_MAX_WORK: usize = 4_096;
