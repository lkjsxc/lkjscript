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
}

pub fn verify(program: Program) -> crate::Result<VerifiedProgram> {
    verify_program(&program)?;
    Ok(VerifiedProgram(program))
}

pub const TRAIT_VERIFY_MAX_DEPTH: usize = 32;
pub const TRAIT_VERIFY_MAX_WORK: usize = 256;
pub const OWNERSHIP_VERIFY_MAX_WORK: usize = 131_072;
pub const SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION: usize = 4_096;
pub const SSA_VERIFY_MAX_CFG_WORK: usize = 4_194_304;
pub(crate) const OWNERSHIP_VERIFY_MAX_RETAINED_STATE_CELLS: usize = 131_072;
pub(crate) const TYPE_VERIFY_MAX_DEPTH: usize = 64;
pub(crate) const TYPE_VERIFY_MAX_WORK: usize = 4_096;
