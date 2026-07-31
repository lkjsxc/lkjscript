use lkjscript_core::{StructuralNode, StructuralType};

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

    pub(super) fn explicit_structural_node<'a>(
        &'a self,
        value: &EvalValue,
        expected: StructuralType,
    ) -> Result<StructuralNode<'a>, Flow> {
        match value {
            EvalValue::StructuralOwner(owner) if owner.value_type == expected => self
                .structural
                .runtime
                .value_node(owner.key, expected)
                .map_err(map_structural_error),
            EvalValue::StructuralView(view) if view.value_type == expected => self
                .structural
                .runtime
                .projected_node(view.key)
                .map_err(map_structural_error),
            _ => Err(Flow::Trap(
                "structural aggregate value type mismatch".into(),
            )),
        }
    }
}
