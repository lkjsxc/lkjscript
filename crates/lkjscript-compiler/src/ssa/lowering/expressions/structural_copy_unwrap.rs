impl FunctionBuilder<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn lower_structural_copy_enum_unwrap(
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
        let owner_representation =
            self.structural_representation(&owner_ty, StructuralValueCategory::Owner)?;
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
        let temporary = !matches!(input.kind, ExprKind::Load(_));
        let mut edge_owners = incoming_unplaced.clone();
        if temporary {
            edge_owners.push(value);
        }
        let (success_env, success_edge_owners, arguments) =
            self.add_edge_state_parameters(success, &incoming_env, &edge_owners, block_origin)?;
        let (failure_env, failure_edge_owners, failure_arguments) =
            self.add_edge_state_parameters(failure, &incoming_env, &edge_owners, block_origin)?;
        if arguments != failure_arguments {
            return Err(Error::msg("SSA copy enum branch edge schemas diverged"));
        }
        let success_unplaced = success_edge_owners[..incoming_unplaced.len()].to_vec();
        let failure_unplaced = failure_edge_owners[..incoming_unplaced.len()].to_vec();
        let success_temporary = if temporary {
            success_edge_owners
                .last()
                .copied()
                .ok_or_else(|| Error::msg("SSA copy enum branch lost its temporary owner"))?
        } else {
            value
        };
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
        let copied = if let ExprKind::Load(reference) = input.kind {
            let loaded = self
                .env
                .get(&reference.binding)
                .copied()
                .ok_or_else(|| Error::msg("structural copy successor lost its loaded value"))?;
            self.append(
                owner_ty.clone(),
                InstructionKind::StructuralCopy {
                    representation: owner_representation,
                    value: loaded,
                },
                EffectSet::ALLOCATES,
                source,
            )?
        } else {
            success_temporary
        };
        self.append(
            ty,
            InstructionKind::AggregateConsumePayload {
                representation: owner_representation,
                place: None,
                variant,
                value: copied,
            },
            EffectSet::PURE,
            source,
        )
    }
}
