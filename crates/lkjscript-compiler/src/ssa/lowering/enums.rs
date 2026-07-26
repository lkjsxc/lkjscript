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
            origin,
        )
        .map(Some)
    }
}
