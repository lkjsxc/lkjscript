impl Emitter<'_> {
    fn emit_witness_instruction(&mut self, instruction: &InstructionKind) -> Result<()> {
        let (parameter, value, dispose) = match instruction {
            InstructionKind::MemoryWitnessIndependentOwner { parameter, value } => {
                (parameter, *value, false)
            }
            InstructionKind::MemoryWitnessDispose { parameter, value } => {
                (parameter, *value, true)
            }
            _ => return Err(Error::msg("non-witness instruction reached witness emitter")),
        };
        if dispose {
            self.load(value)?;
        } else {
            self.load_observed_structural(value)?;
        }
        let ordinal = self.witness_parameter_ordinal(parameter)?;
        self.proto.emit_op_u8(
            if dispose {
                Op::MemoryWitnessDispose
            } else {
                Op::MemoryWitnessIndependentOwner
            },
            ordinal,
        );
        Ok(())
    }
}
