use lkjscript_core::StructuralType;

use super::*;

impl Evaluator<'_> {
    pub(super) fn explicit_structural_owner(
        &self,
        value: &EvalValue,
        expected: StructuralType,
    ) -> Result<lkjscript_core::StructuralValueKey, Flow> {
        match value {
            EvalValue::StructuralOwner(owner) if owner.value_type == expected => Ok(owner.key),
            EvalValue::StructuralView(view)
                if view.root_type == expected && view.value_type == expected =>
            {
                Ok(view.owner)
            }
            _ => Err(Flow::Trap(
                "structural aggregate owner type mismatch".into(),
            )),
        }
    }

    pub(super) fn explicit_structural_payload(
        &self,
        value: &EvalValue,
        expected: StructuralType,
    ) -> Result<lkjscript_core::SemanticValue, Flow> {
        match value {
            EvalValue::StructuralOwner(owner) if owner.value_type == expected => self
                .structural
                .runtime
                .value(owner.key, expected)
                .cloned()
                .map_err(map_structural_error),
            EvalValue::StructuralView(view) if view.value_type == expected => self
                .structural
                .runtime
                .projected(view.key)
                .cloned()
                .map_err(map_structural_error),
            _ => Err(Flow::Trap(
                "structural aggregate value type mismatch".into(),
            )),
        }
    }
}
