use super::*;

impl FunctionEncoder<'_> {
    pub(in crate::encode) fn emit_call_status_cleanup(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), NativeError> {
        if instruction.unentered_cleanup.is_empty() {
            return self.emit_status_cleanup(instruction);
        }
        self.emit(&[0x0f, 0x84])?;
        let success_displacement = self.reserve_i32()?;
        self.runtime_calls
            .insert(RuntimeCallSlot::TakeRejectedEntry);
        self.load_integer_register(7, self.context_offset())?;
        self.emit_runtime_call_target(RuntimeCallSlot::TakeRejectedEntry)?;
        self.emit(&[0x48, 0x85, 0xc0])?;
        self.emit(&[0x0f, 0x84])?;
        let entered_displacement = self.reserve_i32()?;
        self.emit_cleanup_call_list(&instruction.unentered_cleanup)?;
        let entered = self.bytes.len();
        patch_relative(self.bytes, entered_displacement, entered)?;
        self.emit_cleanup_call_list(&instruction.failure_cleanup)?;
        self.emit_jump(FixupTarget::StatusReturn)?;
        let success = self.bytes.len();
        patch_relative(self.bytes, success_displacement, success)
    }

    pub(in crate::encode) fn emit_status_cleanup(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), NativeError> {
        if instruction.failure_cleanup.is_empty() {
            return self.emit_conditional_jump(0x85, FixupTarget::StatusReturn);
        }
        self.emit(&[0x0f, 0x84])?;
        let success_displacement = self.reserve_i32()?;
        self.emit_cleanup_call_list(&instruction.failure_cleanup)?;
        self.emit_jump(FixupTarget::StatusReturn)?;
        let success = self.bytes.len();
        patch_relative(self.bytes, success_displacement, success)
    }

    pub(in crate::encode) fn emit_cleanup_calls(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), NativeError> {
        self.emit_cleanup_call_list(&instruction.failure_cleanup)
    }

    fn emit_cleanup_call_list(
        &mut self,
        cleanups: &[crate::plan::FailureCleanupCall],
    ) -> Result<(), NativeError> {
        for cleanup in cleanups {
            self.runtime_calls.insert(cleanup.slot);
            self.load_integer_register(7, self.context_offset())?;
            self.load_integer_register(6, self.local_offset(cleanup.local)?)?;
            self.emit_runtime_call_target(cleanup.slot)?;
        }
        Ok(())
    }
}
