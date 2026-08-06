impl Emitter<'_> {
    fn emit_witness_instruction(&mut self, instruction: &InstructionKind) -> Result<()> {
        match instruction {
            InstructionKind::MemoryWitnessIndependentOwner { parameter, value } => {
                self.load_observed_structural(*value)?;
                let ordinal = self.witness_parameter_ordinal(parameter)?;
                self.emit_index(Op::MemoryWitnessIndependentOwner, ordinal)?;
            }
            InstructionKind::MemoryWitnessCompare {
                parameter,
                left,
                right,
            } => {
                self.load_observed_structural(*left)?;
                self.load_observed_structural(*right)?;
                let ordinal = self.witness_parameter_ordinal(parameter)?;
                self.emit_index(Op::MemoryWitnessCompare, ordinal)?;
            }
            InstructionKind::MemoryWitnessDispose { parameter, value } => {
                self.load(*value)?;
                let ordinal = self.witness_parameter_ordinal(parameter)?;
                self.emit_index(Op::MemoryWitnessDispose, ordinal)?;
            }
            _ => return Err(Error::msg("non-witness instruction reached witness emitter")),
        }
        Ok(())
    }
}
