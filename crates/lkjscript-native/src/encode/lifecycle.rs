use super::*;

impl FunctionEncoder<'_> {
    pub(super) fn emit_function(&mut self) -> Result<(), NativeError> {
        self.emit_prologue()?;
        self.emit_parameters()?;
        self.emit_register_frame()?;
        let entry = self
            .function
            .entry
            .ok_or(NativeError::Encode(EncodeError::MissingEntry))?;
        self.emit_jump(FixupTarget::Block(entry))?;

        for block in &self.function.blocks {
            let index = block.id.index as usize;
            let offset = self.bytes.len();
            let slot = self
                .block_offsets
                .get_mut(index)
                .ok_or(NativeError::Encode(EncodeError::InvalidLabel))?;
            *slot = Some(offset);
            for instruction in &block.instructions {
                let start = self.bytes.len();
                self.emit_instruction(instruction)?;
                let end = self.bytes.len();
                self.source_map.push(source_map_entry(
                    self.function.id,
                    to_u32(start)?,
                    to_u32(end)?,
                    instruction.source,
                ));
            }
            let terminator = block
                .terminator
                .as_ref()
                .ok_or(NativeError::Encode(EncodeError::InvalidLabel))?;
            self.emit_terminator(terminator)?;
        }

        for trap in [
            TrapCode::I64Overflow,
            TrapCode::DivisionByZero,
            TrapCode::Explicit,
        ] {
            let offset = self.bytes.len();
            self.trap_offsets[trap_index(trap)] = Some(offset);
            self.trap_map.push(trap_map_entry(
                self.function.id,
                to_u32(offset)?,
                trap,
                None,
            ));
            self.outcome_map.push(outcome_map_entry(
                self.function.id,
                to_u32(offset)?,
                OutcomeKind::Trap(trap),
            ));
            self.emit_trap_stub(trap)?;
        }
        self.status_return_offset = Some(self.bytes.len());
        self.emit_registered_zero_return()?;
        self.unregistered_status_return_offset = Some(self.bytes.len());
        self.emit_unregistered_zero_return()?;
        self.patch_fixups()?;
        self.check_code_limit()
    }

    pub(super) fn emit_prologue(&mut self) -> Result<(), NativeError> {
        // The only stack touch before reservation is the ABI return address and
        // saved frame pointer. Incoming volatile arguments are copied to
        // invocation-owned scratch, never to the requested generated frame.
        self.emit(&[0x55])?;
        self.emit(&[0x48, 0x89, 0xe5])?;
        self.emit(&[0x48, 0x89, 0x77, SCRATCH_INTEGER_ARGUMENT_0])?;
        self.emit(&[0x48, 0x89, 0x57, SCRATCH_INTEGER_ARGUMENT_1])?;
        self.emit(&[0xf2, 0x0f, 0x11, 0x47, SCRATCH_FLOAT_ARGUMENT_0])?;
        self.emit(&[0xf2, 0x0f, 0x11, 0x4f, SCRATCH_FLOAT_ARGUMENT_1])?;
        self.emit_reserve_frame()?;
        // ReserveFrame returns the invocation context in RAX.
        self.emit(&[0x48, 0x85, 0xc0])?;
        self.emit_conditional_jump(0x84, FixupTarget::UnregisteredStatusReturn)?;
        self.emit(&[0x83, 0x38, 0x00])?;
        self.emit_conditional_jump(0x85, FixupTarget::UnregisteredStatusReturn)?;
        self.emit(&[0x48, 0x81, 0xec])?;
        self.emit(&self.frame_bytes.to_le_bytes())?;
        self.store_rax(self.context_offset())?;
        self.zero_rax()?;
        for home in build_frame_homes(self.function)? {
            self.store_rax(home.rbp_displacement())?;
        }
        Ok(())
    }

    pub(super) fn emit_reserve_frame(&mut self) -> Result<(), NativeError> {
        self.runtime_calls.insert(RuntimeCallSlot::ReserveFrame);
        self.load_integer_register_immediate(6, u64::from(self.function_ordinal))?;
        self.load_integer_register_immediate(2, u64::from(self.frame_bytes))?;
        // mov rcx, rbp. RDI still carries the invocation context.
        self.emit(&[0x48, 0x89, 0xe9])?;
        self.emit_runtime_call_target(RuntimeCallSlot::ReserveFrame)
    }

    pub(super) fn emit_parameters(&mut self) -> Result<(), NativeError> {
        let mut integer_index = 0_usize;
        let mut float_index = 0_usize;
        for (index, parameter) in self.function.signature.parameters().iter().enumerate() {
            let value = self
                .function
                .values
                .get(index)
                .ok_or(NativeError::Encode(EncodeError::InvalidValue))?
                .id;
            let offset = self.value_offset(value)?;
            match parameter {
                ValueType::I64
                | ValueType::Bool
                | ValueType::StaticBytes
                | ValueType::StaticString(_)
                | ValueType::Capability(_)
                | ValueType::Resource(_)
                | ValueType::Unique(_)
                | ValueType::Loan(_)
                | ValueType::StructuralOwner(_)
                | ValueType::StructuralView(_)
                | ValueType::StructuralDestination(_)
                | ValueType::Reference(_) => {
                    let scratch = [SCRATCH_INTEGER_ARGUMENT_0, SCRATCH_INTEGER_ARGUMENT_1]
                        .get(integer_index)
                        .copied()
                        .ok_or(NativeError::Encode(EncodeError::UnsupportedSignature))?;
                    self.load_rax_from_context(scratch)?;
                    self.store_rax(offset)?;
                    integer_index += 1;
                }
                ValueType::F64 => {
                    let scratch = [SCRATCH_FLOAT_ARGUMENT_0, SCRATCH_FLOAT_ARGUMENT_1]
                        .get(float_index)
                        .copied()
                        .ok_or(NativeError::Encode(EncodeError::UnsupportedSignature))?;
                    self.load_xmm0_from_context(scratch)?;
                    self.store_xmm0(offset)?;
                    float_index += 1;
                }
                ValueType::Unit => {
                    self.zero_rax()?;
                    self.store_rax(offset)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn emit_register_frame(&mut self) -> Result<(), NativeError> {
        self.runtime_calls.insert(RuntimeCallSlot::RegisterFrame);
        self.load_integer_register(7, self.context_offset())?;
        self.load_integer_register_immediate(6, u64::from(self.function_ordinal))?;
        // mov rdx, rbp. The raw frame base is consumed only by the private sys
        // runtime trampoline and never appears in a safe API.
        self.emit(&[0x48, 0x89, 0xea])?;
        self.emit_runtime_call_target(RuntimeCallSlot::RegisterFrame)?;
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0x83, 0x39, 0x00])?;
        self.emit_conditional_jump(0x85, FixupTarget::UnregisteredStatusReturn)
    }
}
