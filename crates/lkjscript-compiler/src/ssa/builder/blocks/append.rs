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
        let ownership = success_ownership(self.structural, &self.value_types, &ty, &kind);
        let call_handoff = match &kind {
            InstructionKind::Call {
                arguments,
                consuming,
                ..
            } => arguments
                .iter()
                .zip(consuming)
                .filter_map(|(argument, consuming)| consuming.then_some(*argument))
                .collect(),
            _ => Vec::new(),
        };
        let id = self.next_value(&ty)?;
        let needs_frame_state = matches!(kind, InstructionKind::Call { .. })
            || effects.contains(EffectSet::ALLOCATES)
            || effects.contains(EffectSet::HOST_IO);
        let frame_state = if needs_frame_state {
            Some(self.frame_state())
        } else {
            None
        };
        let failure = failure_behavior(effects);
        let failure_cleanup = self.intern_failure_cleanup(&call_handoff)?;
        let metadata = InstructionMetadata {
            origin: self.next_origin(expression_origin.raw()),
            effects,
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
            .retain(|owner| !ownership.consumed.contains(owner));
        if ownership.publishes_owner {
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
        let source = self.append(
            ty.clone(),
            InstructionKind::Constant(constant),
            EffectSet::PURE,
            expression_origin,
        )?;
        self.publish_structural_source(ty, source, expression_origin)
    }
}

struct SuccessOwnership {
    consumed: Vec<ValueId>,
    publishes_owner: bool,
}

fn success_ownership(
    structural: &StructuralMemoryMetadata,
    value_types: &[SsaType],
    ty: &SsaType,
    kind: &InstructionKind,
) -> SuccessOwnership {
    let consumed = match kind {
        InstructionKind::PlaceInit { value, .. }
        | InstructionKind::Drop { value, .. }
        | InstructionKind::Move { value, .. }
        | InstructionKind::StructuralPublish { value, .. }
        | InstructionKind::DestinationFinish { destination: value }
        | InstructionKind::DestinationAbort { destination: value }
        | InstructionKind::AggregateConsumePayload { value, .. } => vec![*value],
        InstructionKind::DestinationFieldInit {
            destination, value, ..
        }
        | InstructionKind::WithProductField {
            value: destination,
            replacement: value,
            ..
        } => vec![*destination, *value],
        InstructionKind::Call {
            arguments,
            consuming,
            ..
        } => arguments
            .iter()
            .zip(consuming)
            .filter_map(|(argument, consuming)| consuming.then_some(*argument))
            .collect(),
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } => arguments
            .iter()
            .copied()
            .filter(|argument| {
                let ty = value_types.get(argument.index().unwrap_or(usize::MAX));
                ty.is_some_and(|ty| {
                    !structural.is_immutable(ty)
                        && (is_owned_value(structural, ty)
                            && (!matches!(ty, SsaType::Resource(_))
                                || operation.consumes_affine_arguments()))
                })
            })
            .collect(),
        _ => Vec::new(),
    };
    let raw_structural_source = structural.is_owned(ty)
        && matches!(
            kind,
            InstructionKind::Constant(_) | InstructionKind::Runtime { .. }
        );
    let publishes_owner = is_owned_value(structural, ty)
        && !raw_structural_source
        && !matches!(
            kind,
            InstructionKind::Constant(Constant::StaticBytes(_))
                | InstructionKind::Borrow { .. }
                | InstructionKind::AggregateFieldBorrow { .. }
                | InstructionKind::StringUtf8View { .. }
                | InstructionKind::Runtime {
                    operation: RuntimeOp::StdinHandle,
                    ..
                }
        );
    SuccessOwnership {
        consumed,
        publishes_owner,
    }
}
