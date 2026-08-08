impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn lower_structural_enum_unwrap(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        input: &Expr,
        trap: &str,
        ty: SsaType,
        source: hir::Origin,
        value: ValueId,
        owner_ty: SsaType,
    ) -> Result<ValueId> {
        let variant = lkjscript_ir::VariantId::new(ids.1.bytes());
        let tag = self.append(
            SsaType::I64,
            InstructionKind::AggregateTag {
                representation: self
                    .structural_representation(&owner_ty, StructuralValueCategory::View)?,
                value,
            },
            EffectSet::READS_MEMORY,
            source,
        )?;
        let expected = self.constant(
            SsaType::I64,
            Constant::I64(self.structural_variant_tag(&owner_ty, variant)?),
            source,
        )?;
        let test = self.append(
            SsaType::Bool,
            InstructionKind::Runtime {
                operation: RuntimeOp::EqualValue,
                arguments: vec![tag, expected],
                signature: Signature::monomorphic(vec![SsaType::I64, SsaType::I64], SsaType::Bool),
            },
            RuntimeOp::EqualValue.effects(),
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
            return Err(Error::msg("SSA structural branch edge schemas diverged"));
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
        self.unplaced_owners = failure_unplaced.clone();
        self.env = failure_env;
        self.slots = incoming_slots.clone();
        let failure_value =
            self.structural_successor_value(input, value, &incoming_unplaced, &failure_unplaced)?;
        if self.structural.is_owned(&owner_ty) && !matches!(input.kind, ExprKind::Load(_)) {
            let _owner =
                self.synthetic_structural_owner_place(failure_value, &owner_ty, source)?;
        }
        let message = self.constant(SsaType::Str, Constant::Str(trap.into()), source)?;
        self.cleanup_all_places(source)?;
        self.terminate(Terminator::Trap { value: message })?;

        self.switch_to(success)?;
        self.active_place_bindings = incoming_places;
        self.unplaced_owners = success_unplaced.clone();
        self.env = success_env;
        self.slots = incoming_slots;
        let value =
            self.structural_successor_value(input, value, &incoming_unplaced, &success_unplaced)?;
        let owner = self.structural_owner_place(input, value, &owner_ty, source)?;
        let payload = self.append(
            ty,
            InstructionKind::AggregateConsumePayload {
                representation: self
                    .structural_representation(&owner_ty, StructuralValueCategory::Owner)?,
                place: Some(owner.place),
                variant,
                value,
            },
            EffectSet::PURE,
            source,
        )?;
        self.finish_consumed_structural_place(owner, source)?;
        Ok(payload)
    }
}
