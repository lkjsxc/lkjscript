impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn lower_enum_unwrap(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        field_index: u64,
        _layout: hir::RuntimeLayoutId,
        input: &Expr,
        trap: &str,
        ty: SsaType,
        source: hir::Origin,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(input)? else {
            return Ok(None);
        };
        let owner_ty = self.value_type(value)?.clone();
        if self.structural.is_owned(&owner_ty) {
            return self
                .lower_structural_enum_unwrap(ids, input, trap, ty, source, value, owner_ty)
                .map(Some);
        }
        if self.structural.type_for(&owner_ty).is_some() {
            return self
                .lower_structural_copy_enum_unwrap(ids, input, trap, ty, source, value, owner_ty)
                .map(Some);
        }
        if resource_result_adapter(ids.0, &owner_ty) {
            return self
                .lower_resource_result_unwrap(
                    ids,
                    field_index,
                    _layout,
                    input,
                    trap,
                    ty,
                    source,
                    value,
                )
                .map(Some);
        }
        Err(Error::msg(
            "enum unwrap lacks deterministic structural metadata",
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_resource_result_unwrap(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        field_index: u64,
        layout: hir::RuntimeLayoutId,
        input: &Expr,
        trap: &str,
        ty: SsaType,
        source: hir::Origin,
        value: ValueId,
    ) -> Result<ValueId> {
        let test = self.append(
            SsaType::Bool,
            InstructionKind::EnumIsVariant {
                enum_id: lkjscript_ir::EnumId::new(ids.0.bytes()),
                variant: lkjscript_ir::VariantId::new(ids.1.bytes()),
                layout: lkjscript_ir::RuntimeLayoutId::new(layout.bytes()),
                value,
            },
            EffectSet::READS_MEMORY,
            source,
        )?;
        let block_origin = origin(source, self.next_position);
        let success = self.new_block(block_origin, false)?;
        let failure = self.new_block(block_origin, false)?;
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();
        let incoming_places = self.active_place_bindings.clone();
        let incoming_unplaced = self.unplaced_owners.clone();
        let (success_env, success_unplaced, arguments) = self.add_edge_state_parameters(
            success,
            &incoming_env,
            &incoming_unplaced,
            block_origin,
        )?;
        let (failure_env, failure_unplaced, failure_arguments) = self.add_edge_state_parameters(
            failure,
            &incoming_env,
            &incoming_unplaced,
            block_origin,
        )?;
        if arguments != failure_arguments {
            return Err(Error::msg("SSA resource-result branch edge schemas diverged"));
        }
        self.terminate(Terminator::ConditionalBranch {
            condition: test,
            true_target: success,
            true_arguments: arguments.clone(),
            false_target: failure,
            false_arguments: arguments,
        })?;

        self.switch_to(failure)?;
        self.active_place_bindings = incoming_places.clone();
        self.unplaced_owners = failure_unplaced;
        self.env = failure_env;
        self.slots = incoming_slots.clone();
        let message = self.constant(SsaType::Str, Constant::Str(trap.into()), source)?;
        self.cleanup_all_places(source)?;
        self.terminate(Terminator::Trap { value: message })?;

        self.switch_to(success)?;
        self.active_place_bindings = incoming_places;
        self.unplaced_owners = success_unplaced;
        self.env = success_env;
        self.slots = incoming_slots;
        let value = if let ExprKind::Load(reference) = input.kind {
            self.env
                .get(&reference.binding)
                .copied()
                .ok_or_else(|| Error::msg("resource-result unwrap lost loaded value"))?
        } else {
            incoming_unplaced
                .iter()
                .copied()
                .find(|candidate| *candidate == value)
                .unwrap_or(value)
        };
        self.append_enum_field(ids, field_index, layout, value, ty, source)
    }

    fn append_enum_field(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        field_index: u64,
        _layout: hir::RuntimeLayoutId,
        value: ValueId,
        ty: SsaType,
        source: hir::Origin,
    ) -> Result<ValueId> {
        let owner_ty = self.value_type(value)?.clone();
        if self.structural.type_for(&owner_ty).is_some() {
            return self.append(
                ty,
                InstructionKind::EnumField {
                    enum_id: lkjscript_ir::EnumId::new(ids.0.bytes()),
                    variant: lkjscript_ir::VariantId::new(ids.1.bytes()),
                    field: lkjscript_ir::VariantFieldId::new(ids.2.bytes()),
                    field_index,
                    layout: lkjscript_ir::RuntimeLayoutId::new(_layout.bytes()),
                    value,
                },
                EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES),
                source,
            );
        }
        if resource_result_adapter(ids.0, &owner_ty) {
            return self.append(
                ty,
                InstructionKind::EnumField {
                    enum_id: lkjscript_ir::EnumId::new(ids.0.bytes()),
                    variant: lkjscript_ir::VariantId::new(ids.1.bytes()),
                    field: lkjscript_ir::VariantFieldId::new(ids.2.bytes()),
                    field_index,
                    layout: lkjscript_ir::RuntimeLayoutId::new(_layout.bytes()),
                    value,
                },
                EffectSet::READS_MEMORY,
                source,
            );
        }
        Err(Error::msg(
            "enum field projection lacks deterministic structural metadata",
        ))
    }
}

fn resource_result_adapter(id: hir::EnumId, ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Enum {
            id: enum_id,
            arguments,
        } if id.bytes() == lkjscript_core::RESULT_ID
            && enum_id.bytes() == lkjscript_core::RESULT_ID
            && matches!(arguments.as_slice(), [SsaType::Resource(_), _])
    )
}
