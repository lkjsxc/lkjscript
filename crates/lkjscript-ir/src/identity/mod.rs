mod encoder;
mod function;
mod instruction;
mod memory;
mod metadata;
mod program;
mod runtime;
mod types;
mod witness;

use encoder::Encoder;

const DOMAIN: &[u8] = b"lkjscript.verified-program-identity\0canonical-binary";

/// Exact target-neutral content identity of one verified SSA program.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VerifiedProgramIdentity([u8; 32]);

impl VerifiedProgramIdentity {
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Compute the canonical content identity accepted only through verified authority.
pub fn verified_program_identity(
    verified: &crate::VerifiedProgram,
) -> crate::Result<VerifiedProgramIdentity> {
    let mut encoder = Encoder::new(DOMAIN);
    program::encode_program(&mut encoder, verified.program());
    encoder.finish().map(VerifiedProgramIdentity)
}

#[cfg(test)]
mod tests;
