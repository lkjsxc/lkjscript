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
            match &cleanup.operation {
                crate::plan::FailureCleanupOperation::Runtime(slot) => {
                    self.runtime_calls.insert(*slot);
                    self.load_integer_register(7, self.context_offset())?;
                    self.load_integer_register(6, self.local_offset(cleanup.local)?)?;
                    self.emit_runtime_call_target(*slot)?;
                }
                crate::plan::FailureCleanupOperation::Structural(descriptor) => {
                    let site_id =
                        u64::try_from(self.structural_runtime_sites.len()).map_err(|_| {
                            EncodeError::LimitExceeded("u64 structural runtime-site width")
                        })?;
                    self.structural_runtime_sites.push(structural_runtime_site(
                        site_id,
                        self.function.id,
                        descriptor.as_ref().clone(),
                        None,
                    ));
                    self.runtime_calls
                        .insert(RuntimeCallSlot::StructuralDispatch);
                    self.load_integer_register(7, self.context_offset())?;
                    self.load_integer_register_immediate(6, site_id)?;
                    self.load_integer_register(2, self.local_offset(cleanup.local)?)?;
                    self.load_integer_register_immediate(1, 0)?;
                    self.load_integer_register_immediate(8, 0)?;
                    self.emit_runtime_call_target(RuntimeCallSlot::StructuralDispatch)?;
                }
            }
        }
        Ok(())
    }
}
