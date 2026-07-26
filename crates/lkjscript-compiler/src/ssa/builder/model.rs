use crate::ssa::*;

impl<'a> FunctionBuilder<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn new(
        product_ids: &'a HashMap<String, ProductId>,
        function_ids: &'a HashMap<BindingId, FunctionId>,
        function_effects: &'a HashMap<FunctionId, EffectSet>,
        id: FunctionId,
        name: String,
        signature: Signature,
        function_effect: EffectSet,
        function_origin: Origin,
    ) -> Self {
        Self {
            product_ids,
            function_ids,
            function_effects,
            id,
            name,
            signature,
            function_effect,
            function_origin,
            entry: BlockId::new(0),
            blocks: Vec::new(),
            current: None,
            next_value: 0,
            next_position: 0,
            value_types: Vec::new(),
            places: Vec::new(),
            env: BTreeMap::new(),
            slots: BTreeMap::new(),
            loops: Vec::new(),
        }
    }

    pub(in crate::ssa) fn register_place(
        &mut self,
        place: hir::PlaceId,
        binding: BindingId,
        ty: SsaType,
    ) -> Result<()> {
        let expected = u32::try_from(self.places.len())
            .map_err(|_| Error::msg("SSA place count exceeds u32"))?;
        if place.raw() != expected {
            return Err(Error::msg("HIR PlaceIds are not dense in function order"));
        }
        self.places.push(PlaceMetadata {
            id: SsaPlaceId::new(place.raw()),
            binding: SsaBindingId::new(binding.raw()),
            ty,
        });
        Ok(())
    }

    pub(in crate::ssa) fn owned_place_for_binding(
        &self,
        binding: BindingId,
    ) -> Result<Option<SsaPlaceId>> {
        let binding = SsaBindingId::new(binding.raw());
        let place = self.places.iter().find(|place| place.binding == binding);
        match place {
            Some(place) if is_owned_buf(&place.ty) => Ok(Some(place.id)),
            Some(_) => Ok(None),
            None => Err(Error::msg(format!(
                "HIR binding {} has no registered SSA place",
                binding.raw()
            ))),
        }
    }

    pub(in crate::ssa) fn initialize_owned_place(
        &mut self,
        binding: BindingId,
        value: ValueId,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        if let Some(place) = self.owned_place_for_binding(binding)? {
            let _fact = self.append(
                SsaType::Unit,
                InstructionKind::PlaceInit { place, value },
                EffectSet::PURE,
                expression_origin,
            )?;
        }
        Ok(())
    }

    pub(in crate::ssa) fn end_owned_place(
        &mut self,
        binding: BindingId,
        expression_origin: hir::SourceId,
    ) -> Result<()> {
        if let Some(place) = self.owned_place_for_binding(binding)? {
            let _fact = self.append(
                SsaType::Unit,
                InstructionKind::PlaceEnd { place },
                EffectSet::PURE,
                expression_origin,
            )?;
        }
        Ok(())
    }
}
