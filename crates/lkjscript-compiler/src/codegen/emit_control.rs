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
            Terminator::Trap { message } => {
                let diagnostic = add_constant(self.chunk, BytecodeConstant::Str(message.clone()))?;
                self.proto.emit_op_u16(Op::Trap, diagnostic);
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
        for argument in arguments {
            self.load(*argument)?;
        }
        for parameter in target_block.parameters.iter().rev() {
            let slot = self.slot(parameter.id)?;
            self.proto.emit_op_u8(Op::StoreLocal, slot);
            self.proto.emit(Op::Pop);
        }
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
