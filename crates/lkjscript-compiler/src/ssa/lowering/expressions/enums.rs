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
        let variant = lkjscript_ir::VariantId::new(variant.bytes());
        if self.structural.type_for(&ty).is_some() {
            return self.construct_structural_aggregate(ty, Some(variant), fields, origin);
        }
        let value = self.append(
            ty,
            InstructionKind::EnumValue {
                enum_id: lkjscript_ir::EnumId::new(enum_id.bytes()),
                variant,
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
        let ty = self.value_type(value)?;
        if self.structural.type_for(&ty).is_some() {
            let variant = lkjscript_ir::VariantId::new(variant.bytes());
            let representation =
                self.structural_representation(&ty, StructuralValueCategory::View)?;
            let tag = self.append(
                SsaType::I64,
                InstructionKind::AggregateTag {
                    representation,
                    value,
                },
                EffectSet::READS_MEMORY,
                origin,
            )?;
            let expected = self.constant(
                SsaType::I64,
                Constant::I64(self.structural_variant_tag(&ty, variant)?),
                origin,
            )?;
            return self
                .append(
                    SsaType::Bool,
                    InstructionKind::Runtime {
                        operation: RuntimeOp::EqualValue,
                        arguments: vec![tag, expected],
                        signature: Signature::monomorphic(
                            vec![SsaType::I64, SsaType::I64],
                            SsaType::Bool,
                        ),
                    },
                    RuntimeOp::EqualValue.effects(),
                    origin,
                )
                .map(Some);
        }
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
}
