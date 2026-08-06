use super::{
    control::validate_control_flow,
    decode::{decode_function, validate_instruction_operands},
    shape::validate_tables,
    ValidatedChunk,
};
use crate::{Chunk, Error, Result, ValidationPolicy};

pub fn bind_prepared_identity(
    validated: ValidatedChunk,
    identity: lkjscript_contracts::PreparedProgramIdentity,
) -> Result<ValidatedChunk> {
    if !identity.is_bound()
        || (validated.chunk.prepared_identity.is_bound()
            && validated.chunk.prepared_identity != identity)
    {
        return Err(Error::msg(
            "bytecode prepared program identity is zero or stale",
        ));
    }
    let mut chunk = validated.chunk;
    chunk.prepared_identity = identity;
    validate_chunk(chunk, ValidationPolicy::Unrestricted)
}

pub fn validate_chunk(chunk: Chunk, policy: ValidationPolicy) -> Result<ValidatedChunk> {
    let total_bytes = validate_tables(&chunk)?;
    let main_instructions = decode_function(&chunk.main)?;
    let mut proto_instructions = Vec::new();
    proto_instructions
        .try_reserve_exact(chunk.protos.len())
        .map_err(|_| Error::host("prototype decode metadata reservation failed"))?;
    for proto in &chunk.protos {
        proto_instructions.push(decode_function(proto)?);
    }

    validate_instruction_operands(&chunk, &chunk.main, &main_instructions)?;
    validate_control_flow(&chunk, &chunk.main, &main_instructions, true)?;
    for (index, proto) in chunk.protos.iter().enumerate() {
        let instructions = proto_instructions
            .get(index)
            .ok_or_else(|| Error::msg("validator prototype decode metadata is inconsistent"))?;
        validate_instruction_operands(&chunk, proto, instructions)?;
        validate_control_flow(&chunk, proto, instructions, false)?;
    }
    policy.check_total_bytes(total_bytes)?;

    Ok(ValidatedChunk {
        chunk,
        main_instructions,
        proto_instructions,
    })
}
