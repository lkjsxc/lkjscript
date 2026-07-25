use super::{
    control::validate_control_flow,
    decode::{decode_function, validate_instruction_operands},
    shape::validate_tables,
    ValidatedChunk,
};
use crate::{Chunk, Error, Result, ValidationLimits};

pub fn validate_chunk(chunk: Chunk, limits: &ValidationLimits) -> Result<ValidatedChunk> {
    validate_tables(&chunk, limits)?;
    let main_instructions = decode_function(&chunk.main, limits)?;
    let mut proto_instructions = Vec::with_capacity(chunk.protos.len());
    for proto in &chunk.protos {
        proto_instructions.push(decode_function(proto, limits)?);
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

    Ok(ValidatedChunk {
        chunk,
        main_instructions,
        proto_instructions,
    })
}
