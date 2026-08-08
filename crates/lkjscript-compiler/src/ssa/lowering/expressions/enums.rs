impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_enum_value(
        &mut self,
        _enum_id: hir::EnumId,
        variant: hir::VariantId,
        _layout: hir::RuntimeLayoutId,
        fields: &[Expr],
        ty: SsaType,
        origin: hir::Origin,
    ) -> Result<Option<ValueId>> {
        if self.structural.type_for(&ty).is_none() {
            return Err(Error::msg(
                "enum construction lacks deterministic structural metadata",
            ));
        }
        let Some(fields) = self.lower_arguments(fields)? else {
            return Ok(None);
        };
        let variant = lkjscript_ir::VariantId::new(variant.bytes());
        self.construct_structural_aggregate(ty, Some(variant), fields, origin)
    }

    pub(in crate::ssa) fn lower_enum_test(
        &mut self,
        _enum_id: hir::EnumId,
        variant: hir::VariantId,
        _layout: hir::RuntimeLayoutId,
        input: &Expr,
        origin: hir::Origin,
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
        Err(Error::msg(
            "enum variant test lacks deterministic structural metadata",
        ))
    }

    pub(in crate::ssa) fn lower_enum_field(
        &mut self,
        ids: (hir::EnumId, hir::VariantId, hir::VariantFieldId),
        field_index: u64,
        layout: hir::RuntimeLayoutId,
        input: &Expr,
        ty: SsaType,
        origin: hir::Origin,
    ) -> Result<Option<ValueId>> {
        let Some(value) = self.lower_expr(input)? else {
            return Ok(None);
        };
        self.append_enum_field(ids, field_index, layout, value, ty, origin)
            .map(Some)
    }
}
