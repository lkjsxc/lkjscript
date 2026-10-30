//! Numeric test receipts for fixtures whose expected work is known to remain exact.
//! Every adapter calls the production validator and refuses a saturated observation.

use super::*;
use crate::platform::diagnostic::Diagnostic;
use crate::platform::semantic_id::{DeclarationId, ExpressionId};

pub(crate) fn observe<T>(work: &mut usize, run: impl FnOnce(&mut ValidationWork) -> T) -> T {
    let mut observation = ValidationWork::exact(*work);
    let result = run(&mut observation);
    assert!(
        !observation.saturated,
        "numeric fixture requires an exact work observation"
    );
    *work = observation.consumed;
    result
}

pub(crate) fn validate_full_with_limit(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
) -> Result<FullValidationReport, Vec<Diagnostic>> {
    super::validate_full_with_limit(snapshot, Some(maximum_work))
}

pub(crate) fn validate_full_observed(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
    consumed: &mut u64,
) -> Result<FullValidationReport, Vec<Diagnostic>> {
    validate_full_checked(snapshot, maximum_work, consumed, &|| Ok(()))
}

pub(crate) fn validate_full_checked(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
    consumed: &mut u64,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<FullValidationReport, Vec<Diagnostic>> {
    let mut observation = ValidationWork::default();
    let result =
        super::validate_full_checked(snapshot, Some(maximum_work), &mut observation, checkpoint);
    assert!(!observation.saturated, "finite validation cannot saturate");
    *consumed = observation.consumed as u64;
    result
}

pub(crate) fn validate_expression_roots<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
) {
    observe(work, |work| {
        super::validate_expression_roots(read, roots, diagnostics, work)
    });
}

pub(crate) fn validate_expression_roots_with_limits<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    limits: ExpressionValidationLimits,
) -> Result<(), ExpressionValidationExhaustion> {
    observe(work, |work| {
        super::validate_expression_roots_with_limits(read, roots, diagnostics, work, limits)
    })
}

pub(crate) fn validate_affine_roots_with_limits<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    limits: ExpressionValidationLimits,
) -> Result<(), ExpressionValidationExhaustion> {
    observe(work, |work| {
        super::validate_affine_roots_with_limits(read, roots, diagnostics, work, limits)
    })
}

pub(crate) fn validate_affine_meaning(
    snapshot: &KernelSnapshot,
    read: &impl ExpressionRead,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    maximum_steps: usize,
) {
    observe(work, |work| {
        super::affine::validate_affine_meaning(
            snapshot,
            read,
            diagnostics,
            work,
            Some(maximum_steps),
        )
    });
}

pub(crate) fn infer_function_expression_type<R: ExpressionRead>(
    read: &R,
    declaration: DeclarationId,
    expression: ExpressionId,
    effect: &FunctionEffect,
    work: &mut usize,
    maximum_steps: usize,
) -> Result<TypeObjectDigest, Diagnostic> {
    observe(work, |work| {
        super::infer_function_expression_type(
            read,
            declaration,
            expression,
            effect,
            work,
            Some(maximum_steps),
        )
    })
}

pub(crate) fn infer_function_expression_type_object<R: ExpressionRead>(
    read: &R,
    declaration: DeclarationId,
    expression: ExpressionId,
    effect: &FunctionEffect,
    work: &mut usize,
    maximum_steps: usize,
) -> Result<TypeObject, Diagnostic> {
    observe(work, |work| {
        super::infer_function_expression_type_object(
            read,
            declaration,
            expression,
            effect,
            work,
            Some(maximum_steps),
        )
    })
}

pub(crate) fn validate_callable_flow<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    work: &mut usize,
    maximum_work: usize,
) -> Result<callable_flow::CallableFlowWork, Diagnostic> {
    observe(work, |work| {
        callable_flow::validate_callable_flow(read, roots, work, Some(maximum_work))
    })
}

pub(crate) fn validate_callable_closure<R: callable_flow::CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut usize,
    maximum_work: Option<usize>,
) -> Result<callable_flow::CallableFlowWork, Diagnostic> {
    observe(work, |work| {
        callable_flow::validate_callable_closure(read, work, maximum_work)
    })
}

pub(crate) fn validate_snapshot_callable_closure(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut usize,
    maximum_work: Option<usize>,
) -> Result<callable_flow::CallableFlowWork, Diagnostic> {
    observe(work, |work| {
        callable_flow::validate_snapshot_callable_closure(snapshots, checkpoint, work, maximum_work)
    })
}

pub(crate) fn validate_snapshot_callable_closure_eager(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut usize,
    maximum_work: Option<usize>,
) -> Result<callable_flow::CallableFlowWork, Diagnostic> {
    observe(work, |work| {
        callable_flow::validate_snapshot_callable_closure_eager(
            snapshots,
            checkpoint,
            work,
            maximum_work,
        )
    })
}

pub(crate) fn validate_snapshot_callable_closure_with_limits(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut usize,
    maximum_work: Option<usize>,
    maximum_metadata_bytes: usize,
    eager: bool,
) -> Result<callable_flow::CallableFlowWork, Diagnostic> {
    observe(work, |work| {
        callable_flow::validate_snapshot_callable_closure_with_limits(
            snapshots,
            checkpoint,
            work,
            maximum_work,
            Some(maximum_metadata_bytes),
            eager,
        )
    })
}
