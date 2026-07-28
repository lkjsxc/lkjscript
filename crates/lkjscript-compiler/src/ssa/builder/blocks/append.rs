use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn append(
        &mut self,
        ty: SsaType,
        kind: InstructionKind,
        effects: EffectSet,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        let current = self
            .current
            .ok_or_else(|| Error::msg("cannot append to an ended SSA path"))?;
        let place_init = match &kind {
            InstructionKind::PlaceInit { value, .. } => Some(*value),
            _ => None,
        };
        let call_handoff = match &kind {
            InstructionKind::Call { arguments, .. } => arguments.clone(),
            _ => Vec::new(),
        };
        let consumed_unplaced = match &kind {
            InstructionKind::Drop { value, .. } => vec![*value],
            InstructionKind::Call { arguments, .. } => arguments.clone(),
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } if !matches!(
                operation,
                RuntimeOp::BytesLength
                    | RuntimeOp::BytesByteAt
                    | RuntimeOp::CopyBytesSlice
                    | RuntimeOp::CloneBytes
            ) =>
            {
                arguments.clone()
            }
            _ => Vec::new(),
        };
        let publishes_unplaced_owner = is_owned_value(&ty)
            && !matches!(
                &kind,
                InstructionKind::Constant(Constant::StaticBytes(_))
                    | InstructionKind::Borrow { .. }
                    | InstructionKind::Runtime {
                        operation: RuntimeOp::StdinHandle,
                        ..
                    }
            );
        let id = self.next_value(&ty)?;
        let safepoint = if matches!(kind, InstructionKind::Call { .. })
            || effects.contains(EffectSet::ALLOCATES)
            || effects.contains(EffectSet::HOST_IO)
        {
            Safepoint::Required
        } else {
            Safepoint::None
        };
        let frame_state = if safepoint == Safepoint::Required {
            Some(self.frame_state())
        } else {
            None
        };
        let failure = failure_behavior(effects);
        let failure_cleanup = self.intern_failure_cleanup(&call_handoff)?;
        let metadata = InstructionMetadata {
            origin: self.next_origin(expression_origin.raw()),
            effects,
            safepoint,
            failure,
            failure_cleanup,
            frame_state,
        };
        self.block_mut(current)?.instructions.push(Instruction {
            id,
            ty,
            kind,
            metadata,
        });
        self.unplaced_owners
            .retain(|owner| !consumed_unplaced.contains(owner));
        if let Some(value) = place_init {
            self.unplaced_owners.retain(|owner| *owner != value);
        }
        if publishes_unplaced_owner {
            self.unplaced_owners.push(id);
        }
        Ok(id)
    }

    pub(in crate::ssa) fn next_origin(&mut self, source: u32) -> Origin {
        let position = self.next_position;
        self.next_position = self.next_position.saturating_add(1);
        origin(source, position)
    }

    pub(in crate::ssa) fn frame_state(&self) -> FrameState {
        FrameState {
            bytecode_position: self.next_position,
            locals: self
                .env
                .iter()
                .filter_map(|(binding, value)| {
                    self.slots.get(binding).map(|slot| FrameLocal {
                        binding: SsaBindingId::new(binding.raw()),
                        slot: *slot,
                        value: *value,
                    })
                })
                .collect(),
            operand_stack: Vec::new(),
        }
    }

    pub(in crate::ssa) fn constant(
        &mut self,
        ty: SsaType,
        constant: Constant,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        self.append(
            ty,
            InstructionKind::Constant(constant),
            EffectSet::PURE,
            expression_origin,
        )
    }
}
