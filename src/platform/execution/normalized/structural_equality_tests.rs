//! Raw representation controls. They do not assert source-level Function equality.

use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::execution::normalized::child_storage::{Captures, Fields};
use crate::platform::execution::normalized::value::{FunctionIndex, ValueOrigin};
use crate::platform::kernel::Name;
use std::borrow::Cow;
use std::thread;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn function(child: Option<NormalizedValue>) -> TestResult<NormalizedValue> {
    Ok(NormalizedValue::Function {
        function: FunctionIndex(3, ValueOrigin::default()),
        type_arguments: Default::default(),
        effect_arguments: Default::default(),
        requirement_arguments: Default::default(),
        read_scope_arguments: Default::default(),
        bound_arguments: child
            .map(|value| Captures::try_new(vec![value]))
            .transpose()?,
    })
}

fn chain(depth: usize, leaf: i64) -> TestResult<NormalizedValue> {
    let mut value = NormalizedValue::I64(leaf);
    let field = Name::new("captured").map_err(|error| error.message)?;
    for _ in 0..depth {
        value = NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Fields::try_new(vec![(field.clone(), value)])?,
        });
        value = function(Some(value))?;
    }
    Ok(value)
}

fn thread_result(run: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(run)?
        .join()
        .map_err(|_| "raw comparison worker panicked")?
}

#[test]
fn raw_comparison_distinct_deep_capture_content_and_late_difference() -> TestResult {
    let left = chain(16_384, 7)?;
    let equal = chain(16_384, 7)?;
    let changed = chain(16_384, 8)?;
    thread_result(move || {
        let control = ExecutionControl::uncancelled();
        assert_eq!(raw_structural_equal(&left, &equal, &control), Ok(true));
        assert_eq!(raw_structural_equal(&left, &changed, &control), Ok(false));
        Ok(())
    })
}

#[test]
fn raw_comparison_metadata_and_empty_prefix_remain_structural() -> TestResult {
    let left = function(Some(NormalizedValue::I64(7)))?;
    let equal = function(Some(NormalizedValue::I64(7)))?;
    let mut changed = equal.clone();
    if let NormalizedValue::Function { function, .. } = &mut changed {
        function.0 += 1;
    }
    let control = ExecutionControl::uncancelled();
    assert_eq!(raw_structural_equal(&left, &equal, &control), Ok(true));
    assert_eq!(raw_structural_equal(&left, &changed, &control), Ok(false));
    let unbound = function(None)?;
    let mut empty_prefix = function(None)?;
    if let NormalizedValue::Function {
        bound_arguments, ..
    } = &mut empty_prefix
    {
        *bound_arguments = Some(Captures::try_new(Vec::new())?);
    }
    assert_eq!(
        raw_structural_equal(&unbound, &empty_prefix, &control),
        Ok(false)
    );
    Ok(())
}

#[test]
fn raw_comparison_cancellation_is_an_error() -> TestResult {
    let left = chain(1_024, 7)?;
    let right = chain(1_024, 7)?;
    let result = raw_structural_equal(&left, &right, &ExecutionControl::cancel_after_checks(8));
    assert!(matches!(result, Err(error) if error.class == ExecutionFailureClass::Cancelled));
    Ok(())
}

struct RefuseFrontier(bool);

impl RefuseFrontier {
    fn enter() -> Self {
        Self(REFUSE_FRONTIER_RESERVATION.with(|slot| slot.replace(true)))
    }
}

impl Drop for RefuseFrontier {
    fn drop(&mut self) {
        REFUSE_FRONTIER_RESERVATION.with(|slot| slot.set(self.0));
    }
}

#[test]
fn raw_comparison_frontier_refusal_is_explicit_and_unary_path_needs_no_frontier() -> TestResult {
    let left = chain(1_024, 7)?;
    let right = chain(1_024, 7)?;
    let first = Name::new("first").map_err(|error| error.message)?;
    let second = Name::new("second").map_err(|error| error.message)?;
    let fields = || -> TestResult<NormalizedValue> {
        Ok(NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Fields::try_new(vec![
                (first.clone(), NormalizedValue::I64(1)),
                (second.clone(), NormalizedValue::I64(2)),
            ])?,
        }))
    };
    let wide_left = fields()?;
    let wide_right = fields()?;
    let _refusal = RefuseFrontier::enter();
    let control = ExecutionControl::uncancelled();
    assert_eq!(raw_structural_equal(&left, &right, &control), Ok(true));
    let result = raw_structural_equal(&wide_left, &wide_right, &control);
    let Err(error) = result else {
        return Err("injected frontier refusal did not propagate".into());
    };
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "normalized_raw_comparison_allocation");
    assert!(matches!(error.code, Cow::Borrowed(_)));
    assert!(matches!(error.message, Cow::Borrowed(_)));
    // This is deterministic factory refusal, not a physical OOM experiment.
    Ok(())
}
