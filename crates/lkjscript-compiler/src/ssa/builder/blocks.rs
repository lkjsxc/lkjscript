use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn new_block(
        &mut self,
        block_origin: Origin,
        loop_header: bool,
    ) -> Result<BlockId> {
        let raw = u32::try_from(self.blocks.len())
            .map_err(|_| Error::msg("SSA block count exceeds u32"))?;
        let id = BlockId::new(raw);
        self.blocks.push(PendingBlock {
            id,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
            metadata: BlockMetadata {
                loop_header,
                origin: block_origin,
                frame_state: None,
            },
        });
        Ok(id)
    }

    pub(in crate::ssa) fn block_mut(&mut self, id: BlockId) -> Result<&mut PendingBlock> {
        self.blocks
            .get_mut(id.index().unwrap_or(usize::MAX))
            .filter(|block| block.id == id)
            .ok_or_else(|| Error::msg(format!("missing SSA block {}", id.raw())))
    }

    pub(in crate::ssa) fn add_block_parameter(
        &mut self,
        block: BlockId,
        ty: SsaType,
        owner_place: Option<SsaPlaceId>,
        parameter_origin: Origin,
    ) -> Result<ValueId> {
        let id = self.next_value(&ty)?;
        self.block_mut(block)?.parameters.push(BlockParameter {
            id,
            ty,
            owner_place,
            origin: parameter_origin,
        });
        Ok(id)
    }

    pub(in crate::ssa) fn add_environment_parameters(
        &mut self,
        block: BlockId,
        incoming: &BTreeMap<BindingId, ValueId>,
        parameter_origin: Origin,
    ) -> Result<BTreeMap<BindingId, ValueId>> {
        let mut environment = BTreeMap::new();
        for (binding, value) in incoming {
            let parameter = self.add_block_parameter(
                block,
                self.value_type(*value)?,
                self.owned_place_for_binding(*binding)?,
                parameter_origin,
            )?;
            environment.insert(*binding, parameter);
        }
        Ok(environment)
    }

    pub(in crate::ssa) fn environment_arguments(
        environment: &BTreeMap<BindingId, ValueId>,
    ) -> Vec<ValueId> {
        environment.values().copied().collect()
    }

    pub(in crate::ssa) fn next_value(&mut self, ty: &SsaType) -> Result<ValueId> {
        let id = ValueId::new(self.next_value);
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or_else(|| Error::msg("SSA value count exceeds u32"))?;
        self.value_types.push(ty.clone());
        Ok(id)
    }

    pub(in crate::ssa) fn terminate(&mut self, terminator: Terminator) -> Result<()> {
        let current = self
            .current
            .ok_or_else(|| Error::msg("cannot terminate an ended SSA path"))?;
        let block = self.block_mut(current)?;
        if block.terminator.replace(terminator).is_some() {
            return Err(Error::msg("SSA block has duplicate terminators"));
        }
        self.current = None;
        Ok(())
    }

    pub(in crate::ssa) fn switch_to(&mut self, block: BlockId) -> Result<()> {
        if self.block_mut(block)?.terminator.is_some() {
            return Err(Error::msg("cannot switch to terminated SSA block"));
        }
        self.current = Some(block);
        Ok(())
    }

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
        let metadata = InstructionMetadata {
            origin: self.next_origin(expression_origin.raw()),
            effects,
            safepoint,
            failure: failure_behavior(effects),
            frame_state,
        };
        self.block_mut(current)?.instructions.push(Instruction {
            id,
            ty,
            kind,
            metadata,
        });
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
