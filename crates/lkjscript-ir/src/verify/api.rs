use crate::verify::*;
use crate::Program;

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedProgram(Program);

impl VerifiedProgram {
    pub fn program(&self) -> &Program {
        &self.0
    }
}

pub fn verify(program: Program) -> crate::Result<VerifiedProgram> {
    verify_program(&program)?;
    Ok(VerifiedProgram(program))
}
