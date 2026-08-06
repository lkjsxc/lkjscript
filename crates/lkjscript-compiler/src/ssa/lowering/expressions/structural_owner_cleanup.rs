impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn structural_successor_value(
        &self,
        expression: &Expr,
        value: ValueId,
        incoming_unplaced: &[ValueId],
        successor_unplaced: &[ValueId],
    ) -> Result<ValueId> {
        if let Some(successor) = incoming_unplaced
            .iter()
            .position(|candidate| *candidate == value)
            .and_then(|index| successor_unplaced.get(index))
        {
            return Ok(*successor);
        }
        if let ExprKind::Load(reference) = expression.kind {
            return self.env.get(&reference.binding).copied().ok_or_else(|| {
                Error::msg("structural successor state lost its exact loaded owner")
            });
        }
        Err(Error::msg(format!(
            concat!(
                "structural successor state lost temporary owner {:?}; ",
                "incoming={:?}; successor={:?}"
            ),
            value, incoming_unplaced, successor_unplaced,
        )))
    }

    fn synthetic_structural_owner_place(
        &mut self,
        value: ValueId,
        ty: &SsaType,
        expression_origin: hir::SourceId,
    ) -> Result<StructuralOwnerPlace> {
        self.synthetic_owner_place(
            value,
            ty,
            structural_glue(self.structural, ty)?,
            expression_origin,
        )
    }

    fn synthetic_owner_place(
        &mut self,
        value: ValueId,
        ty: &SsaType,
        drop_glue: DropGlueIdentity,
        expression_origin: hir::SourceId,
    ) -> Result<StructuralOwnerPlace> {
        let place = SsaPlaceId::new(
            u64::try_from(self.places.len())
                .map_err(|_| Error::msg("SSA synthetic place count exceeds u64"))?,
        );
        let binding = BindingId::new(self.next_synthetic_binding);
        self.next_synthetic_binding = self
            .next_synthetic_binding
            .checked_add(1)
            .ok_or_else(|| Error::msg("SSA synthetic binding identity overflow"))?;
        self.places.push(PlaceMetadata {
            id: place,
            binding: SsaBindingId::new(binding.raw()),
            ty: ty.clone(),
            drop_glue: Some(drop_glue),
        });
        self.initialize_owned_place(binding, value, expression_origin)?;
        self.env.insert(binding, value);
        Ok(StructuralOwnerPlace { place, binding })
    }

    pub(in crate::ssa) fn finish_consumed_structural_place(
        &mut self,
        owner: StructuralOwnerPlace,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        self.env.remove(&owner.binding);
        self.active_place_bindings
            .retain(|binding| *binding != owner.binding);
        let _end = self.append(
            SsaType::Unit,
            InstructionKind::PlaceEnd { place: owner.place },
            EffectSet::PURE,
            expression_origin,
        )?;
        Ok(())
    }

    pub(in crate::ssa) fn drop_unplaced_structural_owner(
        &mut self,
        value: ValueId,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        if !self.unplaced_owners.contains(&value) {
            return Err(Error::msg("structural cleanup lost its unplaced owner"));
        }
        let ty = self.value_type(value)?;
        if !self.structural.is_owned(&ty) {
            return Err(Error::msg(
                "branch-local affine cleanup requires structural ownership",
            ));
        }
        let owner = self.synthetic_structural_owner_place(value, &ty, expression_origin)?;
        self.end_owned_place(owner.binding, expression_origin)
    }

    pub(in crate::ssa) fn drop_abandoned_structural_owners(
        &mut self,
        returned: ValueId,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        let candidates = self.unplaced_owners.clone();
        let mut abandoned = Vec::new();
        for value in candidates {
            if value == returned {
                continue;
            }
            let ty = self.value_type(value)?;
            if is_owned_value(self.structural, &ty) {
                abandoned.push((value, ty));
            }
        }
        for (value, ty) in abandoned {
            let glue = match ty {
                SsaType::Bytes => DropGlueIdentity::Bytes,
                SsaType::ByteVector => DropGlueIdentity::ByteVector,
                SsaType::Resource(kind) => DropGlueIdentity::Resource(kind),
                SsaType::Str | SsaType::Path | SsaType::Product(_) | SsaType::Enum { .. } => {
                    structural_glue(self.structural, &ty)?
                }
                SsaType::StructuralDestination(_)
                | SsaType::Unit
                | SsaType::Bool
                | SsaType::I64
                | SsaType::F64
                | SsaType::Symbol
                | SsaType::ByteSlice
                | SsaType::ByteSliceMut
                | SsaType::Capability(_)
                | SsaType::List(_)
                | SsaType::Function(_)
                | SsaType::TypeParameter(_) => {
                    return Err(Error::msg(
                        "abandoned affine owner has no executable drop glue",
                    ));
                }
            };
            let owner = self.synthetic_owner_place(value, &ty, glue, expression_origin)?;
            self.end_owned_place(owner.binding, expression_origin)?;
        }
        Ok(())
    }
}
