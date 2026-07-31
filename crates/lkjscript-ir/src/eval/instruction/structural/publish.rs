use lkjscript_core::StructuralType;

use super::*;

impl Evaluator<'_> {
    pub(super) fn publish_instruction(
        &mut self,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        let InstructionKind::StructuralPublish {
            representation,
            value: source,
        } = instruction.kind
        else {
            return Err(Flow::Trap("structural publish dispatch mismatch".into()));
        };
        let facts =
            self.representation_facts(representation, crate::StructuralValueCategory::Owner)?;
        let expected = self.structural_type(&facts.ty)?;
        self.publish_explicit(values, source, expected)
    }

    fn publish_explicit(
        &mut self,
        values: &mut [Option<EvalValue>],
        source: ValueId,
        expected: StructuralType,
    ) -> Result<EvalValue, Flow> {
        let input = match take_value(values, source)? {
            EvalValue::StructuralOwner(owner) => {
                if owner.value_type == expected {
                    return Ok(EvalValue::StructuralOwner(owner));
                }
                restore_slot(values, source, EvalValue::StructuralOwner(owner))?;
                return Err(Flow::Trap("structural publish type mismatch".into()));
            }
            input => input,
        };
        let semantic = match self.take_semantic(input, expected) {
            Ok(semantic) => semantic,
            Err(error) => {
                let (flow, original) = *error;
                restore_slot(values, source, original)?;
                return Err(flow);
            }
        };
        match self.structural.runtime.publish_owned(semantic) {
            Ok(key) => Ok(EvalValue::StructuralOwner(EvalStructuralOwner {
                key,
                value_type: expected,
            })),
            Err(failure) => {
                let restored = self.semantic_to_eval(failure.value)?;
                restore_slot(values, source, restored)?;
                Err(map_structural_error(failure.error))
            }
        }
    }
}
