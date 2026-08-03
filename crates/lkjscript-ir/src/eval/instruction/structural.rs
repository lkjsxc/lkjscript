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
            InstructionKind::StructuralCopy {
                representation,
                value: source,
            } => self.copy_structural_instruction(representation, source, values),
            _ => Err(Flow::Trap(
                "structural instruction dispatch mismatch".into(),
            )),
        }
    }
}

impl Evaluator<'_> {
    fn copy_structural_instruction(
        &mut self,
        representation: crate::StructuralRepresentationId,
        source: ValueId,
        values: &[Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        let facts =
            self.representation_facts(representation, crate::StructuralValueCategory::Owner)?;
        let expected = self.structural_type(&facts.ty)?;
        let EvalValue::StructuralOwner(owner) = value(values, source)? else {
            return self.copy_eval_value(value(values, source)?);
        };
        if owner.value_type != expected {
            return Err(Flow::Trap(
                "structural copy representation type mismatch".into(),
            ));
        }
        if facts.storage == crate::StructuralStorage::SealedRegion {
            self.structural
                .runtime
                .acquire_sealed(owner.key, expected)
                .map(|key| {
                    EvalValue::StructuralOwner(EvalStructuralOwner {
                        key,
                        value_type: expected,
                    })
                })
                .map_err(map_structural_error)
        } else {
            self.copy_eval_value(value(values, source)?)
        }
    }
}

mod access;
mod destination;
mod metadata;
mod projection;
mod publish;

use metadata::*;
