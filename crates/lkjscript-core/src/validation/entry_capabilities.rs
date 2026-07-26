use crate::{Chunk, Error, Result};

pub(super) fn validate(chunk: &Chunk) -> Result<()> {
    let required_arity = u8::try_from(chunk.required_capabilities.len())
        .map_err(|_| Error::msg("bytecode capability arity exceeds u8"))?;
    if chunk.main.arity != required_arity {
        return Err(Error::msg(
            "bytecode main arity must equal its exact capability requirements",
        ));
    }
    if !chunk
        .required_capabilities
        .windows(2)
        .all(|pair| pair[0] < pair[1])
    {
        return Err(Error::msg(
            "bytecode capability requirements must be sorted and unique",
        ));
    }
    Ok(())
}

pub(super) fn metadata_bytes(chunk: &Chunk) -> Result<usize> {
    chunk
        .main
        .name
        .len()
        .checked_add(chunk.required_capabilities.len())
        .ok_or_else(|| Error::msg("bytecode metadata byte size overflow"))
}
