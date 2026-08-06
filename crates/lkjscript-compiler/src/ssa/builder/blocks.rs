mod append;

use std::collections::BTreeSet;

use crate::ssa::*;

type EdgeStateParameters = (BTreeMap<BindingId, ValueId>, Vec<ValueId>, Vec<ValueId>);

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn new_block(
        &mut self,
        block_origin: Origin,
        loop_header: bool,
    ) -> Result<BlockId> {
        let raw = u64::try_from(self.blocks.len())
            .map_err(|_| Error::msg("SSA block count exceeds u64"))?;
        let id = BlockId::new(raw);
        self.blocks.push(PendingBlock {
            id,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
            metadata: BlockMetadata {
                loop_header,
                origin: block_origin,
                failure_cleanup: None,
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

    pub(in crate::ssa) fn add_edge_state_parameters(
        &mut self,
        block: BlockId,
        incoming_environment: &BTreeMap<BindingId, ValueId>,
        incoming_unplaced: &[ValueId],
        parameter_origin: Origin,
    ) -> Result<EdgeStateParameters> {
        let mut owner_places = BTreeMap::new();
        for (binding, value) in incoming_environment {
            let ty = self.value_type(*value)?;
            if !is_owned_value(self.structural, &ty) {
                continue;
            }
            let place = self.owned_place_for_binding(*binding)?;
            match owner_places.get_mut(value) {
                None => {
                    owner_places.insert(*value, place);
                }
                Some(current) if current.is_none() => *current = place,
                Some(current) if place.is_some() && *current != place => {
                    return Err(Error::msg(
                        "SSA environment aliases one owner through distinct places",
                    ));
                }
                Some(_) => {}
            }
        }
        let mut environment = BTreeMap::new();
        let mut mapped = BTreeMap::new();
        let mut arguments = Vec::new();
        for (binding, value) in incoming_environment {
            let ty = self.value_type(*value)?;
            let owned = is_owned_value(self.structural, &ty);
            if owned {
                if let Some(parameter) = mapped.get(value).copied() {
                    environment.insert(*binding, parameter);
                    continue;
                }
            }
            let place = if owned {
                owner_places.get(value).copied().flatten()
            } else {
                self.owned_place_for_binding(*binding)?
            };
            let parameter = self.add_block_parameter(block, ty, place, parameter_origin)?;
            environment.insert(*binding, parameter);
            if owned {
                mapped.insert(*value, parameter);
            }
            arguments.push(*value);
        }
        for value in incoming_unplaced {
            if mapped.contains_key(value) {
                continue;
            }
            let parameter =
                self.add_block_parameter(block, self.value_type(*value)?, None, parameter_origin)?;
            mapped.insert(*value, parameter);
            arguments.push(*value);
        }
        let unplaced = incoming_unplaced
            .iter()
            .map(|value| {
                mapped
                    .get(value)
                    .copied()
                    .ok_or_else(|| Error::msg("SSA edge lost an unplaced owner parameter"))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut seen_owners = BTreeSet::new();
        for argument in &arguments {
            if is_owned_value(self.structural, &self.value_type(*argument)?)
                && !seen_owners.insert(*argument)
            {
                return Err(Error::msg(format!(
                    "SSA edge-state builder duplicated owner {argument:?}; \
                     environment={incoming_environment:?}; unplaced={incoming_unplaced:?}"
                )));
            }
        }
        Ok((environment, unplaced, arguments))
    }

    pub(in crate::ssa) fn next_value(&mut self, ty: &SsaType) -> Result<ValueId> {
        let id = ValueId::new(self.next_value);
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or_else(|| Error::msg("SSA value count exceeds u64"))?;
        self.value_types.push(ty.clone());
        Ok(id)
    }

    pub(in crate::ssa) fn terminate(&mut self, terminator: Terminator) -> Result<()> {
        let current = self
            .current
            .ok_or_else(|| Error::msg("cannot terminate an ended SSA path"))?;
        let failure_cleanup = self.intern_failure_cleanup(&[])?;
        let block = self.block_mut(current)?;
        block.metadata.failure_cleanup = failure_cleanup;
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
}
