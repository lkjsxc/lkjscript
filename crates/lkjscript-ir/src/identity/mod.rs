mod functions;
mod instruction_values;
mod instructions;
mod metadata;
mod operations;
mod types;
mod writer;

use std::fmt;

use crate::{Program, VerifiedProgram};
use writer::Writer;

const MAX_IDENTITY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityError(&'static str);

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for IdentityError {}

pub fn verified_program_digest(program: &VerifiedProgram) -> Result<[u8; 32], IdentityError> {
    program_digest(program.program())
}

fn program_digest(program: &Program) -> Result<[u8; 32], IdentityError> {
    let mut writer = Writer::new(MAX_IDENTITY_BYTES);
    writer.bytes(b"lkjscript.verified-program")?;
    metadata::program(&mut writer, program)?;
    Ok(lkjscript_contracts::sha256(writer.finish()))
}

#[cfg(test)]
mod tests;
