use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_concrete_operation(
        &mut self,
        operation: Operation,
        resolved_signature: &Type,
        args: &[Expr],
        arguments: Vec<ValueId>,
        ty: SsaType,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let runtime = runtime_operation(operation)?;
        let signature = signature_from_type(resolved_signature, self.product_ids)?;
        let consumed_resource = arguments.first().copied();
        let result_ty = ty.clone();
        let result = self.append(
            ty,
            InstructionKind::Runtime {
                operation: runtime,
                arguments,
                signature,
            },
            effects(operation.effects()),
            expression.origin,
        )?;
        self.forget_consumed_ref_mut_arguments(args);
        if matches!(
            operation,
            Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
        ) {
            let [Expr {
                kind: ExprKind::Load(reference),
                ..
            }] = args
            else {
                return Err(Error::msg(
                    "resource close lowering requires one direct typed resource local",
                ));
            };
            let value = consumed_resource
                .ok_or_else(|| Error::msg("resource close lost its SSA operand"))?;
            self.record_explicit_close(reference.binding, value, expression.origin)?;
        }
        self.publish_structural_source(result_ty, result, expression.origin)
            .map(Some)
    }
}
