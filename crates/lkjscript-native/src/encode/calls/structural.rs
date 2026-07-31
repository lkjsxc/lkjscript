use super::*;

impl FunctionEncoder<'_> {
    pub(in crate::encode) fn emit_structural_call(
        &mut self,
        instruction: &Instruction,
        descriptor: &crate::StructuralCallDescriptor,
        arguments: &[ValueId],
    ) -> Result<(), NativeError> {
        let site_id = to_u32(self.structural_runtime_sites.len())?;
        self.structural_runtime_sites.push(structural_runtime_site(
            site_id,
            self.function.id,
            descriptor.clone(),
            instruction.source,
        ));
        self.runtime_calls
            .insert(RuntimeCallSlot::StructuralDispatch);
        self.load_integer_register(7, self.context_offset())?;
        self.load_integer_register_immediate(6, u64::from(site_id))?;
        for register in [2_u8, 1_u8, 8_u8] {
            self.load_integer_register_immediate(register, 0)?;
        }
        for (argument, register) in arguments.iter().zip([2_u8, 1_u8, 8_u8]) {
            self.load_integer_register(register, self.value_offset(*argument)?)?;
        }
        self.emit_call_target(RelocationTarget::Runtime(
            RuntimeCallSlot::StructuralDispatch,
        ))?;
        self.emit(&[0x41, 0xff, 0xd3])?;
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0x83, 0x39, 0x00])?;
        self.emit_status_cleanup(instruction)?;
        self.store_rax(self.value_offset(instruction.output)?)
    }
}
