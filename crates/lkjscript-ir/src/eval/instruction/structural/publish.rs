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
        self.publish_explicit(values, source, expected, facts.storage)
    }

    fn publish_explicit(
        &mut self,
        values: &mut [Option<EvalValue>],
        source: ValueId,
        expected: StructuralType,
        storage: crate::StructuralStorage,
    ) -> Result<EvalValue, Flow> {
        let input = match take_value(values, source)? {
            EvalValue::StructuralOwner(owner) => {
                if owner.value_type == expected {
                    if storage != crate::StructuralStorage::SealedRegion {
                        return Ok(EvalValue::StructuralOwner(owner));
                    }
                    return match self.structural.runtime.seal_owned(owner.key, expected) {
                        Ok(sealed) => Ok(EvalValue::StructuralOwner(EvalStructuralOwner {
                            key: sealed.owner,
                            value_type: expected,
                        })),
                        Err(error) => {
                            restore_slot(values, source, EvalValue::StructuralOwner(owner))?;
                            Err(map_structural_error(error))
                        }
                    };
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
            Ok(key) => {
                let key = if storage == crate::StructuralStorage::SealedRegion {
                    match self.structural.runtime.seal_owned(key, expected) {
                        Ok(sealed) => sealed.owner,
                        Err(error) => {
                            if let Err(cleanup) =
                                self.structural.runtime.dispose_owner(key, expected)
                            {
                                self.note_structural_cleanup_failure(cleanup.to_string());
                            }
                            return Err(map_structural_error(error));
                        }
                    }
                } else {
                    key
                };
                Ok(EvalValue::StructuralOwner(EvalStructuralOwner {
                    key,
                    value_type: expected,
                }))
            }
            Err(failure) => {
                let restored = self.semantic_to_eval(failure.value)?;
                restore_slot(values, source, restored)?;
                Err(map_structural_error(failure.error))
            }
        }
    }
}
