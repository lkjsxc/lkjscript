use super::*;

impl Evaluator<'_> {
    pub(crate) fn instruction(
        &mut self,
        function: &crate::Function,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
        depth: usize,
    ) -> std::result::Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::Constant(constant) => self.constant(constant),
            InstructionKind::Copy(source) => self.copy_eval_value(value(values, *source)?),
            InstructionKind::Move { value: source, .. } => {
                let moved = take_value(values, *source)?;
                self.move_eval_value(moved)
            }
            InstructionKind::Borrow {
                kind,
                value: source,
                ..
            } => self.borrow_eval_value(
                value(values, *source)?,
                matches!(kind, crate::BorrowKind::Mutable),
            ),
            InstructionKind::EndBorrow { value: source, .. } => {
                let view = take_value(values, *source)?;
                self.end_eval_borrow(view)?;
                Ok(EvalValue::Unit)
            }
            InstructionKind::Drop {
                value: source,
                glue: crate::DropGlueIdentity::Resource(kind),
                kind: crate::DropEventKind::ImplicitCleanup,
                ..
            } => self.drop_resource(values, *source, *kind),
            InstructionKind::Drop {
                value: source,
                glue: crate::DropGlueIdentity::Resource(_),
                kind: crate::DropEventKind::ExplicitClose,
                ..
            } => Self::finish_explicit_resource_close(values, *source),
            InstructionKind::Drop {
                value: source,
                glue: crate::DropGlueIdentity::Structural(_),
                ..
            } => {
                let owner = take_value(values, *source)?;
                self.cleanup_eval_value(owner)?;
                Ok(EvalValue::Unit)
            }
            InstructionKind::Drop { value: source, .. } => self.drop_unique(values, *source),
            InstructionKind::PlaceInit { .. } | InstructionKind::PlaceEnd { .. } => {
                Ok(EvalValue::Unit)
            }
            InstructionKind::StructuralPublish { .. }
            | InstructionKind::DestinationCreate { .. }
            | InstructionKind::DestinationFieldInit { .. }
            | InstructionKind::DestinationFinish { .. }
            | InstructionKind::DestinationAbort { .. }
            | InstructionKind::AggregateFieldBorrow { .. }
            | InstructionKind::AggregateTag { .. }
            | InstructionKind::AggregateConsumePayload { .. }
            | InstructionKind::StringUtf8View { .. }
            | InstructionKind::StructuralCopy { .. } => {
                self.structural_instruction(instruction, values)
            }
            InstructionKind::FunctionRef(function) => Ok(EvalValue::Function(*function)),
            InstructionKind::Runtime {
                operation,
                arguments,
                signature,
            } => {
                let arguments = values_for_refs(values, arguments)?;
                self.runtime(*operation, arguments, signature.result.as_ref())
            }
            kind @ (InstructionKind::F64FromI64Exact { value: input }
            | InstructionKind::F64FromI64Rounded { value: input }
            | InstructionKind::I64FromF64Exact { value: input }
            | InstructionKind::I64FromF64Trunc { value: input }) => {
                self.numeric_conversion(kind, value(values, *input)?, &instruction.ty)
            }
            InstructionKind::Call {
                target, arguments, ..
            } => {
                let target = match target {
                    CallTarget::Direct(target) => *target,
                    CallTarget::Indirect(target) => match value(values, *target)? {
                        EvalValue::Function(function) => *function,
                        _ => {
                            return Err(Flow::Trap(
                                "evaluator call target is not a function".into(),
                            ))
                        }
                    },
                };
                let arguments = self.call_arguments(function, target, values, arguments)?;
                self.call(target, arguments, depth.saturating_add(1))
            }
            InstructionKind::ProductValue { .. }
            | InstructionKind::ProductField { .. }
            | InstructionKind::WithProductField { .. }
            | InstructionKind::EnumValue { .. }
            | InstructionKind::EnumIsVariant { .. }
            | InstructionKind::EnumField { .. } => {
                self.aggregate_instruction(function, instruction, values)
            }
        }
    }
}

mod aggregate;
mod structural;

include!("constants.rs");
include!("resource_drop.rs");
