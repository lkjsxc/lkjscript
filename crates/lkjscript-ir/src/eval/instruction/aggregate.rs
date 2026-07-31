use super::*;

impl Evaluator<'_> {
    pub(super) fn aggregate_instruction(
        &mut self,
        function: &crate::Function,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::ProductValue { .. }
            | InstructionKind::ProductField { .. }
            | InstructionKind::WithProductField { .. } => {
                self.product_instruction(instruction, values)
            }
            InstructionKind::EnumValue { .. }
            | InstructionKind::EnumIsVariant { .. }
            | InstructionKind::EnumField { .. } => {
                self.enum_instruction(function, instruction, values)
            }
            _ => Err(Flow::Trap("aggregate instruction dispatch mismatch".into())),
        }
    }
}

fn function_value_type(
    function: &crate::Function,
    value: ValueId,
) -> Result<&crate::SsaType, Flow> {
    function
        .blocks
        .iter()
        .flat_map(|block| {
            block
                .parameters
                .iter()
                .map(|parameter| (parameter.id, &parameter.ty))
                .chain(
                    block
                        .instructions
                        .iter()
                        .map(|instruction| (instruction.id, &instruction.ty)),
                )
        })
        .find(|(id, _)| *id == value)
        .map(|(_, ty)| ty)
        .ok_or_else(|| Flow::Trap("evaluator value type metadata is missing".into()))
}

mod enumeration;
mod product;
