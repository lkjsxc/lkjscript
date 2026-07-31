use lkjscript_core::StructuralKind;

use crate::eval::{map_structural_error, structural_root, EvalValue, Evaluator, Flow};

impl Evaluator<'_> {
    pub(crate) fn structural_value_equal(
        &mut self,
        left: &EvalValue,
        right: &EvalValue,
    ) -> Option<Result<bool, Flow>> {
        if is_string(left) || is_string(right) {
            return Some(self.string_equal(left, right));
        }
        if is_path(left) || is_path(right) {
            return Some(self.path_equal(left, right));
        }
        if is_structural(left) || is_structural(right) {
            return Some(self.aggregate_equal(left, right));
        }
        None
    }

    fn string_equal(&mut self, left: &EvalValue, right: &EvalValue) -> Result<bool, Flow> {
        if !is_string(left) || !is_string(right) {
            return Err(Flow::Trap("equal-value category mismatch".into()));
        }
        Ok(self.string_bytes_copy(left)? == self.string_bytes_copy(right)?)
    }

    fn path_equal(&mut self, left: &EvalValue, right: &EvalValue) -> Result<bool, Flow> {
        if !is_path(left) || !is_path(right) {
            return Err(Flow::Trap("equal-value category mismatch".into()));
        }
        Ok(self.path_bytes_copy(left)? == self.path_bytes_copy(right)?)
    }

    fn aggregate_equal(&mut self, left: &EvalValue, right: &EvalValue) -> Result<bool, Flow> {
        let (left_key, left_type) = any_structural_root(left)?;
        let (right_key, right_type) = any_structural_root(right)?;
        if left_type != right_type {
            return Err(Flow::Trap("equal-value structural type mismatch".into()));
        }
        let left_view = self.borrow_whole(left_key, left_type)?;
        let right_view = match self.borrow_whole(right_key, right_type) {
            Ok(view) => view,
            Err(error) => {
                if let Err(cleanup) = self.structural.runtime.end_view(left_view) {
                    self.note_structural_cleanup_failure(cleanup.to_string());
                }
                return Err(error);
            }
        };
        let result = self
            .structural
            .runtime
            .projected(left_view)
            .map_err(map_structural_error)
            .and_then(|left| {
                self.structural
                    .runtime
                    .projected(right_view)
                    .map_err(map_structural_error)
                    .map(|right| left == right)
            });
        let right_end = self
            .structural
            .runtime
            .end_view(right_view)
            .map_err(map_structural_error);
        let left_end = self
            .structural
            .runtime
            .end_view(left_view)
            .map_err(map_structural_error);
        match (result, right_end, left_end) {
            (Ok(equal), Ok(()), Ok(())) => Ok(equal),
            (Err(primary), _, _) => Err(primary),
            (Ok(_), Err(cleanup), _) | (Ok(_), Ok(()), Err(cleanup)) => Err(cleanup),
        }
    }
}

fn is_structural(value: &EvalValue) -> bool {
    matches!(
        value,
        EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
    )
}

fn is_string(value: &EvalValue) -> bool {
    matches!(value, EvalValue::Str(_) | EvalValue::StaticString(_))
        || matches!(
            value,
            EvalValue::StructuralOwner(owner) if owner.value_type.kind == StructuralKind::String
        )
        || matches!(
            value,
            EvalValue::StructuralView(view) if view.value_type.kind == StructuralKind::String
        )
}

fn is_path(value: &EvalValue) -> bool {
    matches!(value, EvalValue::Path(_))
        || matches!(
            value,
            EvalValue::StructuralOwner(owner) if owner.value_type.kind == StructuralKind::Path
        )
        || matches!(
            value,
            EvalValue::StructuralView(view) if view.value_type.kind == StructuralKind::Path
        )
}

fn any_structural_root(
    value: &EvalValue,
) -> Result<
    (
        lkjscript_core::StructuralValueKey,
        lkjscript_core::StructuralType,
    ),
    Flow,
> {
    let kind = match value {
        EvalValue::StructuralOwner(owner) => owner.value_type.kind,
        EvalValue::StructuralView(view) => view.value_type.kind,
        _ => return Err(Flow::Trap("expected structural value".into())),
    };
    structural_root(value, kind)
}
