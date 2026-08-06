use crate::{Chunk, Error, Result};

pub(super) fn validate(chunk: &Chunk) -> Result<()> {
    if chunk.main.arity != chunk.required_capabilities.len() {
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
        .ok_or_else(|| Error::host("bytecode metadata byte size overflow"))
}
