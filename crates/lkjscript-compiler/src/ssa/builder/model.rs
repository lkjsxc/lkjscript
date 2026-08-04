use crate::ssa::*;

impl<'a> FunctionBuilder<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn new(
        product_ids: &'a HashMap<String, ProductId>,
        function_ids: &'a HashMap<BindingId, FunctionId>,
        function_effects: &'a HashMap<FunctionId, EffectSet>,
        function_parameter_modes: &'a HashMap<FunctionId, Vec<MemoryParameterMode>>,
        function_witness_parameters: &'a HashMap<FunctionId, Vec<MemoryWitnessParameter>>,
        structural: &'a StructuralMemoryMetadata,
        id: FunctionId,
        name: String,
        signature: Signature,
        function_effect: EffectSet,
        function_origin: Origin,
        cleanup: CleanupPlan,
    ) -> Self {
        let next_synthetic_binding = cleanup
            .places
            .iter()
            .map(|place| place.binding.raw())
            .max()
            .map_or(0, |binding| binding.saturating_add(1));
        let places = cleanup.places.clone();
        Self {
            product_ids,
            function_ids,
            function_effects,
            function_parameter_modes,
            function_witness_parameters,
            structural,
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
            next_synthetic_binding,
            value_types: Vec::new(),
            places,
            failure_cleanups: Vec::new(),
            cleanup,
            current_memory_expression: None,
            current_placement: None,
            borrowed_call_argument: false,
            active_place_bindings: Vec::new(),
            active_loans: BTreeMap::new(),
            unplaced_owners: Vec::new(),
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
        let id = SsaPlaceId::new(place.raw());
        let declared = self
            .places
            .get(id.index().unwrap_or(usize::MAX))
            .filter(|declared| declared.id == id)
            .ok_or_else(|| Error::msg("HIR place is absent from verified memory metadata"))?;
        if declared.binding != SsaBindingId::new(binding.raw()) || declared.ty != ty {
            return Err(Error::msg(
                "HIR place disagrees with verified SSA place metadata",
            ));
        }
        Ok(())
    }

    pub(in crate::ssa) fn owned_place_for_binding(
        &self,
        binding: BindingId,
    ) -> Result<Option<SsaPlaceId>> {
        let binding = SsaBindingId::new(binding.raw());
        let place = self.places.iter().find(|place| place.binding == binding);
        match place {
            Some(place) if place.drop_glue.is_some() => Ok(Some(place.id)),
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
            if !self.active_place_bindings.contains(&binding) {
                self.active_place_bindings.push(binding);
            }
        }
        Ok(())
    }

    pub(in crate::ssa) fn mark_entry_owner(&mut self, binding: BindingId) {
        if !self.active_place_bindings.contains(&binding) {
            self.active_place_bindings.push(binding);
        }
    }
}
