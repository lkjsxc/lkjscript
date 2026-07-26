use super::*;

impl FunctionEncoder<'_> {
    pub(super) fn emit_terminator(&mut self, terminator: &Terminator) -> Result<(), NativeError> {
        match terminator {
            Terminator::Branch(target) => self.emit_jump(FixupTarget::Block(*target)),
            Terminator::BranchIf {
                condition,
                when_true,
                when_false,
            } => {
                self.load_rax(self.value_offset(*condition)?)?;
                self.emit(&[0x48, 0x85, 0xc0])?;
                self.emit_conditional_jump(0x85, FixupTarget::Block(*when_true))?;
                self.emit_jump(FixupTarget::Block(*when_false))
            }
            Terminator::Return(value) => {
                let offset = self.bytes.len();
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    OutcomeKind::Return,
                ));
                self.emit_unregister_frame()?;
                match self.value_type(*value)? {
                    ValueType::I64 | ValueType::Bool | ValueType::Reference(_) => {
                        self.load_rax(self.value_offset(*value)?)?;
                    }
                    ValueType::F64 => {
                        self.load_xmm0(self.value_offset(*value)?)?;
                    }
                    ValueType::Unit => self.zero_rax()?,
                }
                self.emit_epilogue()
            }
            Terminator::Trap { trap, site, value } => {
                let offset = self.bytes.len();
                self.trap_map.push(trap_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    *trap,
                    *site,
                ));
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    OutcomeKind::Trap(*trap),
                ));
                if let Some(value) = value {
                    self.load_rax(self.value_offset(*value)?)?;
                    self.load_integer_register(1, self.context_offset())?;
                    self.emit(&[0x48, 0x89, 0x41, 0x08])?;
                } else if let Some(site) = site {
                    self.load_rax_immediate(u64::from(*site))?;
                    self.load_integer_register(1, self.context_offset())?;
                    self.emit(&[0x48, 0x89, 0x41, 0x08])?;
                }
                self.emit_jump(FixupTarget::Trap(*trap))
            }
            Terminator::Exit(code) => {
                let offset = self.bytes.len();
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    OutcomeKind::Exit,
                ));
                self.load_rax(self.value_offset(*code)?)?;
                self.load_integer_register(1, self.context_offset())?;
                self.emit(&[0xc7, 0x01])?;
                self.emit(&2_u32.to_le_bytes())?;
                self.emit(&[0x48, 0x89, 0x41, 0x08])?;
                self.emit_registered_zero_return()
            }
            Terminator::Outcome(outcome) => {
                let offset = self.bytes.len();
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    match outcome {
                        RuntimeOutcome::DeadlineExceeded => OutcomeKind::DeadlineExceeded,
                        RuntimeOutcome::ResourceLimitExceeded => OutcomeKind::ResourceLimitExceeded,
                        RuntimeOutcome::HostFailure => OutcomeKind::HostFailure,
                    },
                ));
                let status = match outcome {
                    RuntimeOutcome::DeadlineExceeded => 3_u32,
                    RuntimeOutcome::ResourceLimitExceeded => 4_u32,
                    RuntimeOutcome::HostFailure => 5_u32,
                };
                self.load_integer_register(1, self.context_offset())?;
                self.emit(&[0xc7, 0x01])?;
                self.emit(&status.to_le_bytes())?;
                if matches!(outcome, RuntimeOutcome::ResourceLimitExceeded) {
                    self.emit(&[0x48, 0xc7, 0x41, 0x08])?;
                    self.emit(&1_u32.to_le_bytes())?;
                }
                self.emit_registered_zero_return()
            }
        }
    }

    pub(super) fn emit_trap_stub(&mut self, trap: TrapCode) -> Result<(), NativeError> {
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0xc7, 0x01])?;
        self.emit(&1_u32.to_le_bytes())?;
        self.emit(&[0xc7, 0x41, 0x04])?;
        self.emit(&trap.as_u32().to_le_bytes())?;
        self.emit_registered_zero_return()
    }

    pub(super) fn emit_unregister_frame(&mut self) -> Result<(), NativeError> {
        self.runtime_calls
            .insert(RuntimeCallSlot::UnregisterFrameV1);
        self.load_integer_register(7, self.context_offset())?;
        self.load_integer_register_immediate(6, u64::from(self.function_ordinal))?;
        // mov rdx, rbp; sys validates both the descriptor and frame base.
        self.emit(&[0x48, 0x89, 0xea])?;
        self.emit_runtime_call_target(RuntimeCallSlot::UnregisterFrameV1)
    }

    pub(super) fn emit_registered_zero_return(&mut self) -> Result<(), NativeError> {
        self.emit_unregister_frame()?;
        self.emit_unregistered_zero_return()
    }

    pub(super) fn emit_unregistered_zero_return(&mut self) -> Result<(), NativeError> {
        self.zero_rax()?;
        self.emit(&[0x66, 0x0f, 0xef, 0xc0])?;
        self.emit_epilogue()
    }

    pub(super) fn emit_epilogue(&mut self) -> Result<(), NativeError> {
        self.emit(&[0xc9, 0xc3])
    }

    pub(super) fn emit_jump(&mut self, target: FixupTarget) -> Result<(), NativeError> {
        self.emit(&[0xe9])?;
        let displacement_offset = self.reserve_i32()?;
        self.fixups.push(BranchFixup {
            displacement_offset,
            target,
        });
        Ok(())
    }

    pub(super) fn emit_conditional_jump(
        &mut self,
        condition: u8,
        target: FixupTarget,
    ) -> Result<(), NativeError> {
        self.emit(&[0x0f, condition])?;
        let displacement_offset = self.reserve_i32()?;
        self.fixups.push(BranchFixup {
            displacement_offset,
            target,
        });
        Ok(())
    }
}
