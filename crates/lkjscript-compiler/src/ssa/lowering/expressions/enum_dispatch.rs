use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(super) fn lower_enum_expression(
        &mut self,
        kind: &ExprKind,
        ty: SsaType,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        match kind {
            ExprKind::EnumValue {
                enum_id,
                variant,
                layout,
                fields,
            } => self.lower_enum_value(*enum_id, *variant, *layout, fields, ty, origin),
            ExprKind::EnumIsVariant {
                enum_id,
                variant,
                layout,
                value,
            } => self.lower_enum_test(*enum_id, *variant, *layout, value, origin),
            ExprKind::EnumField {
                enum_id,
                variant,
                field,
                field_index,
                layout,
                value,
            } => self.lower_enum_field(
                (*enum_id, *variant, *field),
                *field_index,
                *layout,
                value,
                ty,
                origin,
            ),
            ExprKind::EnumUnwrap {
                enum_id,
                variant,
                field,
                field_index,
                layout,
                value,
                trap,
            } => self.lower_enum_unwrap(
                (*enum_id, *variant, *field),
                *field_index,
                *layout,
                value,
                trap,
                ty,
                origin,
            ),
            _ => unreachable!("non-enum expression dispatched as enum"),
        }
    }
}
