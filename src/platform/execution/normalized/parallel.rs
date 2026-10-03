//! Structured CPU execution through an explicitly owned reusable executor.
//! Additional worker capacity is never awaited, and a slot survives until join.
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::runtime::structured::StructuredExecutorHandle;

// This bounds native evaluator nesting, independently of cumulative execution fuel.
pub(super) const MAXIMUM_STRUCTURED_DEPTH: usize = 32;

pub(super) struct Pair<L, R> {
    pub(super) left: L,
    pub(super) right: R,
    pub(super) dispatched: bool,
}

/// Prefer the originating failure over its sibling's cooperative cancellation.
/// Deadlines also have the Cancelled class; only execution_cancelled is the
/// generic scope signal, so joining must retain a more specific cancellation cause.
pub(super) fn results<L, R>(
    left: Result<L, ExecutionError>,
    right: Result<R, ExecutionError>,
) -> Result<(L, R), ExecutionError> {
    match (left, right) {
        (Ok(left), Ok(right)) => Ok((left, right)),
        (Err(left), Err(right))
            if left.class == ExecutionFailureClass::Cancelled
                && (right.class != ExecutionFailureClass::Cancelled
                    || (left.code == "execution_cancelled"
                        && right.code != "execution_cancelled")) =>
        {
            Err(right)
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

pub(super) fn include_value_work(
    value: &mut super::value::ValueWork,
    child: &super::value::ValueWork,
) {
    macro_rules! add { ($($field:ident),* $(,)?) => { $(value.$field = value.$field.saturating_add(child.$field);)* }; }
    add!(
        input_admission_nodes,
        raw_result_admission_nodes,
        capture_admission_nodes,
        constructor_child_visits,
        internal_guard_descendant_visits,
        classification_decisions,
        local_value_moves,
        local_value_copies
    );
    let value = &mut value.bytes;
    let child = &child.bytes;
    macro_rules! add_bytes { ($($field:ident),* $(,)?) => { $(value.$field = value.$field.saturating_add(child.$field);)* }; }
    add_bytes!(
        concatenations,
        empty_operand_reuses,
        in_place_appends,
        buffer_growths,
        fresh_buffers,
        payload_bytes_copied,
        requested_capacity_bytes
    );
}

pub(super) fn run<L, R: Send + 'static>(
    executor: &StructuredExecutorHandle,
    control: &ExecutionControl,
    left: impl FnOnce() -> L,
    right: impl FnOnce() -> R + Send + 'static,
) -> Result<Pair<L, R>, ExecutionError> {
    let pair = executor.run(control, left, right)?;
    Ok(Pair {
        left: pair.left,
        right: pair.right,
        dispatched: pair.dispatched,
    })
}

#[cfg(test)]
#[path = "parallel_failure_tests.rs"]
mod failure_tests;
