impl Emitter<'_> {
    fn emit_numeric(&mut self, instruction: &Instruction, value: ValueId) -> Result<()> {
        self.load(value)?;
        self.proto.emit(match &instruction.kind {
            InstructionKind::F64FromI64Exact { .. } => Op::F64FromI64Exact,
            InstructionKind::F64FromI64Rounded { .. } => Op::F64FromI64Rounded,
            InstructionKind::I64FromF64Exact { .. } => Op::I64FromF64Exact,
            InstructionKind::I64FromF64Trunc { .. } => Op::I64FromF64Trunc,
            _ => return Err(Error::msg("numeric opcode lowering mismatch")),
        });
        Ok(())
    }
}
