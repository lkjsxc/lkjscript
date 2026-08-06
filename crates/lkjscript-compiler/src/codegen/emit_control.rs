use crate::codegen::*;

impl Emitter<'_> {
    pub(in crate::codegen) fn emit_terminator(
        &mut self,
        block: BlockId,
        terminator: &Terminator,
    ) -> Result<()> {
        match terminator {
            Terminator::Branch { target, arguments } => {
                self.emit_edge_arguments(*target, arguments)?;
                self.emit_jump(Op::Jump, *target);
            }
            Terminator::ConditionalBranch {
                condition,
                true_target,
                true_arguments,
                false_target,
                false_arguments,
            } => {
                self.load(*condition)?;
                self.proto.emit(Op::JumpIfFalse);
                let false_patch = self.proto.len();
                self.proto.emit_u16(0);
                self.emit_edge_arguments(*true_target, true_arguments)?;
                self.emit_jump(Op::Jump, *true_target);
                let false_offset = self.offset()?;
                self.patch_at(false_patch, false_offset)?;
                self.emit_edge_arguments(*false_target, false_arguments)?;
                self.emit_jump(Op::Jump, *false_target);
            }
            Terminator::Return(value) => {
                self.load(*value)?;
                self.proto.emit(Op::Return);
            }
            Terminator::Trap { value } => {
                self.load(*value)?;
                self.proto.emit(Op::Trap);
            }
            Terminator::Exit { code } => {
                self.load(*code)?;
                self.proto.emit(Op::Exit);
            }
            Terminator::Outcome { outcome, .. } => {
                return Err(Error::msg(format!(
                    "SSA structured outcome {outcome:?} has no source bytecode representation"
                )));
            }
        }
        if self.proto.is_empty() {
            return Err(Error::msg(format!(
                "SSA block {} emitted no bytecode",
                block.raw()
            )));
        }
        Ok(())
    }

    pub(in crate::codegen) fn emit_edge_arguments(
        &mut self,
        target: BlockId,
        arguments: &[ValueId],
    ) -> Result<()> {
        let target_block = self
            .function
            .blocks
            .iter()
            .find(|block| block.id == target)
            .ok_or_else(|| Error::msg("SSA bytecode edge target is missing"))?;
        if target_block.parameters.len() != arguments.len() {
            return Err(Error::msg("SSA bytecode edge argument count mismatch"));
        }
        for (index, argument) in arguments.iter().enumerate() {
            self.load(*argument)?;
            if arguments[index.saturating_add(1)..].contains(argument) {
                self.emit_independent_owner(*argument)?;
            }
        }
        for (parameter, argument) in target_block.parameters.iter().zip(arguments).rev() {
            let slot = self.slot(parameter.id)?;
            if self.structural_local_kind(*argument)?.is_some() {
                self.emit_index(Op::StoreStructuralLocal, slot)?;
                continue;
            }
            match parameter.ty {
                SsaType::Bytes | SsaType::ByteVector => {
                    self.emit_index(Op::StoreUniqueLocal, slot)?;
                }
                SsaType::ByteSlice | SsaType::ByteSliceMut => {
                    self.emit_index(Op::StoreViewLocal, slot)?;
                }
                _ => {
                    self.emit_index(Op::StoreLocal, slot)?;
                    self.proto.emit(Op::Pop);
                }
            }
        }
        Ok(())
    }

    fn emit_independent_owner(&mut self, value: ValueId) -> Result<()> {
        let SsaType::TypeParameter(parameter) = self.value_type(value)? else {
            return Ok(());
        };
        let Some(requirement) = self
            .function
            .signature
            .memory_witness_parameters
            .iter()
            .find(|requirement| requirement.parameter == *parameter)
            .filter(|requirement| {
                requirement
                    .operations
                    .contains(&lkjscript_contracts::MemoryWitnessOperation::IndependentOwner)
            })
        else {
            return Ok(());
        };
        let ordinal = self
            .function
            .signature
            .type_parameters
            .iter()
            .position(|candidate| candidate == &requirement.parameter)
            .ok_or_else(|| Error::msg("memory witness parameter is not declared"))?;
        self.emit_index(Op::MemoryWitnessIndependentOwner, ordinal)?;
        Ok(())
    }

    pub(in crate::codegen) fn emit_jump(&mut self, operation: Op, target: BlockId) {
        self.proto.emit(operation);
        let patch = self.proto.len();
        self.proto.emit_u16(0);
        self.patches.push((patch, target));
    }

    pub(in crate::codegen) fn patch_jumps(&mut self) -> Result<()> {
        let patches = std::mem::take(&mut self.patches);
        for (patch, target) in patches {
            let offset = self.block_offsets.get(&target).copied().ok_or_else(|| {
                Error::msg(format!(
                    "SSA jump target block {} was not emitted",
                    target.raw()
                ))
            })?;
            self.patch_at(patch, offset)?;
        }
        Ok(())
    }

    pub(in crate::codegen) fn patch_at(&mut self, patch: usize, offset: u16) -> Result<()> {
        let end = patch
            .checked_add(2)
            .ok_or_else(|| Error::msg("bytecode jump patch overflow"))?;
        let bytes = self
            .proto
            .code
            .get_mut(patch..end)
            .ok_or_else(|| Error::msg("bytecode jump patch is out of range"))?;
        bytes.copy_from_slice(&offset.to_le_bytes());
        Ok(())
    }

    pub(in crate::codegen) fn global(&self, function: FunctionId) -> Result<u16> {
        self.globals.get(&function).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA function {} has no bytecode closure slot",
                function.raw()
            ))
        })
    }
}
