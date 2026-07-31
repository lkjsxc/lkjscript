use super::*;

impl Evaluator<'_> {
    pub(super) fn structural_instruction(
        &mut self,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match instruction.kind {
            InstructionKind::StructuralPublish { .. } => {
                self.publish_instruction(instruction, values)
            }
            InstructionKind::DestinationCreate { .. }
            | InstructionKind::DestinationFieldInit { .. }
            | InstructionKind::DestinationFinish { .. }
            | InstructionKind::DestinationAbort { .. } => {
                self.destination_instruction(instruction, values)
            }
            InstructionKind::AggregateFieldBorrow { .. }
            | InstructionKind::AggregateTag { .. }
            | InstructionKind::AggregateConsumePayload { .. }
            | InstructionKind::StringUtf8View { .. } => {
                self.projection_instruction(instruction, values)
            }
            InstructionKind::StructuralCopy { value: source, .. } => {
                self.copy_eval_value(value(values, source)?)
            }
            _ => Err(Flow::Trap(
                "structural instruction dispatch mismatch".into(),
            )),
        }
    }
}

mod access;
mod destination;
mod metadata;
mod projection;
mod publish;

use metadata::*;
