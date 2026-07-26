use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_enum_value(
        &mut self,
        enum_id: hir::EnumId,
        variant: hir::VariantId,
        layout: hir::RuntimeLayoutId,
        fields: &[Expr],
        ty: SsaType,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(fields) = self.lower_arguments(fields)? else {
            return Ok(None);
        };
        let value = self.append(
            ty,
            InstructionKind::EnumValue {
                enum_id: lkjscript_ir::EnumId::new(enum_id.bytes()),
                variant: lkjscript_ir::VariantId::new(variant.bytes()),
                layout: lkjscript_ir::RuntimeLayoutId::new(layout.bytes()),
                fields,
            },
            EffectSet::ALLOCATES,
            origin,
        )?;
        Ok(Some(value))
    }

    pub(in crate::ssa) fn lower_enum_test(
        &mut self,
        enum_id: hir::EnumId,
        variant: hir::VariantId,
        layout: hir::RuntimeLayoutId,
        input: &Expr,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(input)? else {
            return Ok(None);
        };
        self.append(
            SsaType::Bool,
            InstructionKind::EnumIsVariant {
                enum_id: lkjscript_ir::EnumId::new(enum_id.bytes()),
                variant: lkjscript_ir::VariantId::new(variant.bytes()),
                layout: lkjscript_ir::RuntimeLayoutId::new(layout.bytes()),
                value,
            },
            EffectSet::READS_MEMORY,
            origin,
        )
        .map(Some)
    }

    pub(in crate::ssa) fn lower_enum_field(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        layout: hir::RuntimeLayoutId,
        input: &Expr,
        ty: SsaType,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(input)? else {
            return Ok(None);
        };
        self.append_enum_field(ids, layout, value, ty, origin)
            .map(Some)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::ssa) fn lower_enum_unwrap(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        layout: hir::RuntimeLayoutId,
        input: &Expr,
        trap: &str,
        ty: SsaType,
        source: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(input)? else {
            return Ok(None);
        };
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
        let block_origin = origin(source.raw(), self.next_position);
        let success = self.new_block(block_origin, false)?;
        let failure = self.new_block(block_origin, false)?;
        let incoming = self.env.clone();
        let slots = self.slots.clone();
        let success_env = self.add_environment_parameters(success, &incoming, block_origin)?;
        let failure_env = self.add_environment_parameters(failure, &incoming, block_origin)?;
        let arguments = Self::environment_arguments(&incoming);
        self.terminate(Terminator::ConditionalBranch {
            condition: test,
            true_target: success,
            true_arguments: arguments.clone(),
            false_target: failure,
            false_arguments: arguments,
        })?;

        self.switch_to(failure)?;
        self.env = failure_env;
        self.slots = slots.clone();
        let message = self.constant(SsaType::Str, Constant::Str(trap.into()), source)?;
        self.terminate(Terminator::Trap { value: message })?;

        self.switch_to(success)?;
        self.env = success_env;
        self.slots = slots;
        self.append_enum_field(ids, layout, value, ty, source)
            .map(Some)
    }

    fn append_enum_field(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        layout: hir::RuntimeLayoutId,
        value: ValueId,
        ty: SsaType,
        source: hir::SourceId,
    ) -> Result<ValueId> {
        self.append(
            ty,
            InstructionKind::EnumField {
                enum_id: lkjscript_ir::EnumId::new(ids.0.bytes()),
                variant: lkjscript_ir::VariantId::new(ids.1.bytes()),
                field: lkjscript_ir::VariantFieldId::new(ids.2.bytes()),
                layout: lkjscript_ir::RuntimeLayoutId::new(layout.bytes()),
                value,
            },
            EffectSet::READS_MEMORY,
            source,
        )
    }
}
