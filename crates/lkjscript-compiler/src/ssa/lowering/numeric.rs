use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_numeric_conversion(
        &mut self,
        conversion: &ExprKind,
        result: SsaType,
        origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let (operation, input) = match conversion {
            ExprKind::F64FromI64Exact(input) => (Operation::F64FromI64Exact, input),
            ExprKind::F64FromI64Rounded(input) => (Operation::F64FromI64Rounded, input),
            ExprKind::I64FromF64Exact(input) => (Operation::I64FromF64Exact, input),
            ExprKind::I64FromF64Trunc(input) => (Operation::I64FromF64Trunc, input),
            _ => return Err(Error::msg("non-conversion reached numeric SSA lowering")),
        };
        let Some(value) = self.lower_expr(input)? else {
            return Ok(None);
        };
        let (kind, effects) = match operation {
            Operation::F64FromI64Exact => (
                InstructionKind::F64FromI64Exact { value },
                EffectSet::ALLOCATES,
            ),
            Operation::F64FromI64Rounded => (
                InstructionKind::F64FromI64Rounded { value },
                EffectSet::PURE,
            ),
            Operation::I64FromF64Exact => (
                InstructionKind::I64FromF64Exact { value },
                EffectSet::ALLOCATES,
            ),
            Operation::I64FromF64Trunc => (
                InstructionKind::I64FromF64Trunc { value },
                EffectSet::ALLOCATES,
            ),
            _ => return Err(Error::msg("numeric conversion operation mismatch")),
        };
        self.append(result, kind, effects, origin).map(Some)
    }
}
