use crate::{IrError, Result, SsaMemoryInventory, VerifiedProgram};

/// Derives Current direct-affine obligations from already verified SSA.
pub fn derive_memory_inventory(program: &VerifiedProgram) -> SsaMemoryInventory {
    SsaMemoryInventory::from_program(program.program())
}

/// Independently recomputes direct-affine obligations and rejects stale producer facts.
pub fn verify_memory_inventory(
    program: &VerifiedProgram,
    inventory: &SsaMemoryInventory,
) -> Result<()> {
    let expected = derive_memory_inventory(program);
    if expected == *inventory {
        Ok(())
    } else {
        Err(IrError::new(
            "SSA memory-obligation inventory does not match verified ownership facts",
        ))
    }
}
