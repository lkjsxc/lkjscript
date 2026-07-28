use super::*;

impl FunctionEncoder<'_> {
    pub(in crate::encode) fn emit_instruction_trap_branch(
        &mut self,
        condition: u8,
        trap: TrapCode,
        instruction: &Instruction,
    ) -> Result<(), NativeError> {
        if instruction.failure_cleanup.is_empty() {
            return self.emit_conditional_jump(condition, FixupTarget::Trap(trap));
        }
        let inverse = match condition {
            0x80 => 0x81,
            0x84 => 0x85,
            _ => return Err(NativeError::Encode(EncodeError::InvalidValue)),
        };
        self.emit(&[0x0f, inverse])?;
        let success_displacement = self.reserve_i32()?;
        let trap_offset = self.bytes.len();
        self.trap_map.push(trap_map_entry(
            self.function.id,
            to_u32(trap_offset)?,
            trap,
            None,
        ));
        self.outcome_map.push(outcome_map_entry(
            self.function.id,
            to_u32(trap_offset)?,
            OutcomeKind::Trap(trap),
        ));
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0xc7, 0x01])?;
        self.emit(&1_u32.to_le_bytes())?;
        self.emit(&[0xc7, 0x41, 0x04])?;
        self.emit(&trap.as_u32().to_le_bytes())?;
        self.emit_cleanup_calls(instruction)?;
        self.emit_jump(FixupTarget::StatusReturn)?;
        let success = self.bytes.len();
        patch_relative(self.bytes, success_displacement, success)
    }
}
