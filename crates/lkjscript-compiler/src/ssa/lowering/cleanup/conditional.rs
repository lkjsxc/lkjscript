use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn verify_conditional_absent_branch(
        &self,
        binding: BindingId,
        block: Option<BlockId>,
        result: ValueId,
    ) -> Result<()> {
        let place = self
            .owned_place_for_binding(binding)?
            .ok_or_else(|| Error::msg("conditional branch has no owned SSA place"))?;
        let block = block
            .and_then(|id| self.blocks.get(id.index().unwrap_or(usize::MAX)))
            .ok_or_else(|| Error::msg("conditional branch lost its reachable SSA block"))?;
        let returned_move = block.instructions.iter().any(|instruction| {
            instruction.id == result
                && matches!(
                    instruction.kind,
                    InstructionKind::Move { place: moved, .. } if moved == place
                )
        });
        let explicit_close = self.blocks.iter().any(|candidate| {
            candidate.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    InstructionKind::Drop {
                        place: dropped,
                        kind: DropEventKind::ExplicitClose,
                        ..
                    } if dropped == place
                )
            })
        });
        let discharged = returned_move || explicit_close;
        if discharged {
            Ok(())
        } else {
            Err(Error::msg(
                "conditional branch join requires an explicit close or transferred branch result",
            ))
        }
    }

    pub(in crate::ssa) fn end_conditional_branch_place(
        &mut self,
        binding: BindingId,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        let place = self
            .owned_place_for_binding(binding)?
            .ok_or_else(|| Error::msg("conditional cleanup has no owned SSA place"))?;
        let _end = self.append(
            SsaType::Unit,
            InstructionKind::PlaceEnd { place },
            EffectSet::PURE,
            expression_origin,
        )?;
        self.active_place_bindings
            .retain(|active| *active != binding);
        Ok(())
    }

    pub(in crate::ssa) fn drop_conditional_branch_owner(
        &mut self,
        binding: BindingId,
        expression_origin: hir::Origin,
    ) -> Result<()> {
        let Some(value) = self.env.get(&binding).copied() else {
            return Ok(());
        };
        let place = self
            .owned_place_for_binding(binding)?
            .ok_or_else(|| Error::msg("conditional cleanup binding has no owned SSA place"))?;
        let drop_class = self
            .cleanup
            .place_drop_classes
            .get(&place)
            .copied()
            .ok_or_else(|| Error::msg("conditional cleanup place lost its HIR drop class"))?;
        if drop_class != MemoryDropClass::Conditional {
            return Err(Error::msg(
                "SSA branch mismatch is not authorized by a conditional HIR drop class",
            ));
        }
        let glue = self
            .places
            .get(place.index().unwrap_or(usize::MAX))
            .and_then(|metadata| metadata.drop_glue)
            .ok_or_else(|| Error::msg("conditional cleanup place lost its drop glue"))?;
        if glue == DropGlueIdentity::Resource(lkjscript_core::ResourceKind::InputStream) {
            return Err(Error::msg(
                "borrowed standard input cannot enter conditional guest cleanup",
            ));
        }
        let _drop = self.append(
            SsaType::Unit,
            InstructionKind::Drop {
                place,
                value,
                glue,
                kind: DropEventKind::ImplicitCleanup,
            },
            EffectSet::PURE,
            expression_origin,
        )?;
        self.env.remove(&binding);
        Ok(())
    }
}
