//! Canonical-source evaluation must not use the host stack for ordinary calls.
//! Authoring, compilation and the independent VM run finish before each worker.

use super::*;
use crate::platform::execution::normalized::reference::{
    CoreNormalizedReferenceHost, NormalizedReferenceBinding, NormalizedReferenceHost,
    NormalizedReferenceOwnerRead, NormalizedReferenceRead, ReferenceSignature,
};
use crate::platform::execution::normalized::reference_schema::NormalizedReferenceSchema;
use crate::platform::execution::normalized::value_schema::NormalizedValueSchema;
use crate::platform::execution::normalized::vm::{CoreNormalizedHost, NormalizedHost};
use crate::platform::kernel::{BlobObjectDigest, PackageId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const WORKER_STACK_BYTES: usize = 512 * 1024;

struct Fixture {
    source: KernelSnapshot,
    program: NormalizedProgram,
    schema: Arc<NormalizedReferenceSchema>,
}

impl Fixture {
    fn new() -> Check<Self> {
        let case = case_from_snapshot(fixture()?)?;
        let host = host_oracle(&case)?;
        let schema = NormalizedReferenceRead::schema(&case.source)
            .map_err(|error| format!("independent canonical schema: {error:?}"))?;
        Ok(Self {
            source: case.source,
            program: host.program,
            schema,
        })
    }
}

// Reconstructing the independent canonical schema is setup, outside the small
// worker. Reads still come from the exact source snapshot; each invocation
// retains the ordinary schema-work charges and creates fresh execution state.
impl NormalizedReferenceRead for Fixture {
    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError> {
        NormalizedReferenceRead::binding(&self.source)
    }

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        NormalizedReferenceRead::owner(&self.source, owner)
    }

    fn schema(&self) -> Result<Arc<NormalizedReferenceSchema>, ExecutionError> {
        Ok(Arc::clone(&self.schema))
    }

    fn blob(&self, digest: BlobObjectDigest) -> Result<Vec<u8>, ExecutionError> {
        NormalizedReferenceRead::blob(&self.source, digest)
    }

    fn owner_in_package(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        NormalizedReferenceRead::owner_in_package(&self.source, package, owner)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Work {
    steps: u64,
    calls: u64,
    external_calls: u64,
    allocated_bytes: u64,
    allocation_charges: u64,
    collection_items: u64,
    maximum_call_depth: usize,
    maximum_control_frames: usize,
    tail_transfers: u64,
}

struct Outcome {
    answer: Result<i64, ExecutionError>,
    work: Work,
}

fn answer(value: Result<NormalizedValue, ExecutionError>) -> Check<Result<i64, ExecutionError>> {
    match value {
        Ok(value) => {
            let (diagnostic, bytes) = filter_value(&value)?;
            equal(diagnostic, 0, "stack control returned a diagnostic")?;
            let bytes: [u8; 8] = bytes
                .try_into()
                .map_err(|_| "stack control answer is not exactly one I64")?;
            Ok(Ok(i64::from_le_bytes(bytes)))
        }
        Err(error) => Ok(Err(error)),
    }
}

fn vm(
    fixture: &Fixture,
    behavior: Behavior,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
    host: Option<&CancelAtRelay<'_>>,
) -> Check<Outcome> {
    let entry = function_named(&fixture.source, MODULE, "binary")?;
    let sink = Mutex::new(None);
    let interpreter = NormalizedVm::for_test(&fixture.program, policy);
    let interpreter = if let Some(host) = host {
        interpreter.observing(&sink, host)
    } else {
        interpreter.observing_checked(&sink)
    };
    let value = interpreter
        .invoke(
            entry,
            vec![NormalizedValue::bytes(behavior.input().as_slice())],
            None,
            control,
        )
        .map(|pair| pair.0);
    let observed = sink
        .into_inner()
        .map_err(|_| "VM stack-control observer lock poisoned")?
        .ok_or("VM stack-control observation missing")?;
    equal(
        observed.live_call_frames_after
            + observed.live_locals_after
            + observed.live_type_bindings_after
            + observed.live_operands_after
            + observed.live_transactions_after
            + observed.live_handles_after,
        0,
        "VM stack-control cleanup",
    )?;
    ensure(
        !observed.instructions_saturated
            && !observed.allocated_bytes_saturated
            && !observed.collection_items_saturated,
        "VM stack-control observation saturated",
    )?;
    Ok(Outcome {
        answer: answer(value)?,
        work: Work {
            steps: observed.instructions,
            calls: observed.calls,
            external_calls: observed.external_calls,
            allocated_bytes: observed.allocated_bytes,
            allocation_charges: observed.allocation_charges,
            collection_items: observed.collection_items,
            maximum_call_depth: observed.maximum_call_depth,
            maximum_control_frames: observed.maximum_control_frames,
            tail_transfers: observed.tail_transfers,
        },
    })
}

fn reference(
    fixture: &Fixture,
    behavior: Behavior,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
    host: Option<&CancelAtRelay<'_>>,
) -> Check<Outcome> {
    let entry = function_named(&fixture.source, MODULE, "binary")?;
    let sink = Mutex::new(None);
    let interpreter =
        NormalizedReferenceInterpreter::from_reader(fixture, &fixture.program, policy);
    let interpreter = if let Some(host) = host {
        interpreter.observing(&sink, host)
    } else {
        interpreter.observing_checked(&sink)
    };
    let value = interpreter
        .invoke(
            entry,
            vec![NormalizedValue::bytes(behavior.input().as_slice())],
            None,
            control,
        )
        .map(|pair| pair.0);
    let observed = sink
        .into_inner()
        .map_err(|_| "reference stack-control observer lock poisoned")?
        .ok_or("reference stack-control observation missing")?;
    equal(
        observed.live_call_frames_after
            + observed.live_control_frames_after
            + observed.live_local_scopes_after
            + observed.live_type_scopes_after
            + observed.live_effect_scopes_after
            + observed.live_allowances_after
            + observed.live_transactions_after
            + observed.live_handles_after,
        0,
        "reference continuation cleanup",
    )?;
    ensure(
        !observed.expressions_saturated
            && !observed.allocated_bytes_saturated
            && !observed.collection_items_saturated,
        "reference stack-control observation saturated",
    )?;
    Ok(Outcome {
        answer: answer(value)?,
        work: Work {
            steps: observed.expressions,
            calls: observed.calls,
            external_calls: observed.external_calls,
            allocated_bytes: observed.allocated_bytes,
            allocation_charges: observed.allocation_charges,
            collection_items: observed.collection_items,
            maximum_call_depth: observed.maximum_call_depth,
            maximum_control_frames: observed.maximum_control_frames,
            tail_transfers: observed.tail_transfers,
        },
    })
}

fn on_small_stack<T: Send>(work: impl FnOnce() -> Check<T> + Send) -> Check<T> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("source-callable-reference-continuations".to_owned())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, work)
            .map_err(|error| format!("reference continuation worker: {error}"))?
            .join()
            .map_err(|_| "reference continuation worker panicked".to_owned())?
    })
}

fn success(outcome: Outcome, behavior: Behavior) -> Check<Work> {
    equal(
        outcome
            .answer
            .map_err(|error| format!("{}({}): {error:?}", behavior.name, behavior.count))?,
        behavior.expected,
        "independent callable stack-control answer",
    )?;
    Ok(outcome.work)
}

fn failure(outcome: Outcome, class: ExecutionFailureClass, code: &str) -> Check<Work> {
    let error = outcome.answer.err().ok_or("expected execution refusal")?;
    equal(
        error.class,
        class,
        &format!("stack-control refusal class (actual code={})", error.code),
    )?;
    equal(error.code.as_ref(), code, "stack-control refusal code")?;
    Ok(outcome.work)
}

fn chain(selector: i64, count: i64) -> Behavior {
    Behavior::new(
        if selector == 7 {
            "fn0-environment-depth"
        } else {
            "record-fn0-environment-depth"
        },
        selector,
        count,
        2 * (7 + count),
    )
}

#[test]
fn selfhost_source_callable_reference_non_tail_chains_use_a_bounded_host_stack() -> Check<()> {
    let fixture = Fixture::new()?;
    for count in [255, 256, 257, 2048] {
        for selector in [7, 8] {
            let behavior = chain(selector, count);
            let expected_depth = usize::try_from(count).map_err(|_| "call count")? + 3;
            // binary and deep remain live; each relay adds one ordinary call,
            // followed by the base call. The wrap constructor itself is tail-recursive.
            let vm_work = success(
                vm(
                    &fixture,
                    behavior,
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled(),
                    None,
                )?,
                behavior,
            )?;
            equal(
                vm_work.maximum_call_depth,
                expected_depth,
                "VM real call chain",
            )?;
            let reference_work = on_small_stack(|| {
                success(
                    reference(
                        &fixture,
                        behavior,
                        NormalizedRunPolicy::foreground(),
                        &ExecutionControl::uncancelled(),
                        None,
                    )?,
                    behavior,
                )
            })?;
            equal(
                reference_work.maximum_call_depth,
                expected_depth,
                "reference real call chain",
            )?;
            eprintln!(
                "source callable reference stack case={} depth={} expected={} \
                 host_stack_bytes={} source_graph_schema_and_c0_build_outside_worker=true \
                 vm_call_depth={} reference_call_depth={} cleanup=empty",
                behavior.name,
                count,
                behavior.expected,
                WORKER_STACK_BYTES,
                vm_work.maximum_call_depth,
                reference_work.maximum_call_depth,
            );
        }
    }
    Ok(())
}

#[test]
fn selfhost_source_callable_reference_preserves_tail_and_non_tail_call_geometry() -> Check<()> {
    let fixture = Fixture::new()?;
    let tail = Behavior::new("indirect-varying-frames", 9, 257, 1519);
    let tail_policy = NormalizedRunPolicy::foreground();
    let vm_tail = success(
        vm(
            &fixture,
            tail,
            tail_policy,
            &ExecutionControl::uncancelled(),
            None,
        )?,
        tail,
    )?;
    let reference_tail = on_small_stack(|| {
        success(
            reference(
                &fixture,
                tail,
                tail_policy,
                &ExecutionControl::uncancelled(),
                None,
            )?,
            tail,
        )
    })?;
    for work in [vm_tail, reference_tail] {
        ensure(
            work.maximum_call_depth <= 16,
            "tail chain grew its live frame count",
        )?;
        ensure(
            work.tail_transfers >= 257,
            "indirect tail chain did not transfer",
        )?;
    }

    for selector in [7, 8] {
        let behavior = chain(selector, 257);
        let policy = NormalizedRunPolicy::foreground();
        let vm_work = success(
            vm(
                &fixture,
                behavior,
                policy,
                &ExecutionControl::uncancelled(),
                None,
            )?,
            behavior,
        )?;
        equal(vm_work.maximum_call_depth, 260, "VM non-tail call depth")?;
        on_small_stack(|| {
            let work = success(
                reference(
                    &fixture,
                    behavior,
                    policy,
                    &ExecutionControl::uncancelled(),
                    None,
                )?,
                behavior,
            )?;
            equal(
                work.maximum_call_depth,
                260,
                "reference non-tail call depth",
            )
        })?;
    }
    Ok(())
}

struct CancelAtRelay<'a> {
    fixture: &'a Fixture,
    read: DeclarationReference,
    entered: AtomicUsize,
}

impl<'a> CancelAtRelay<'a> {
    fn new(fixture: &'a Fixture) -> Check<Self> {
        Ok(Self {
            fixture,
            read: function_named(&fixture.source, MODULE, "read")?,
            entered: AtomicUsize::new(0),
        })
    }

    fn checkpoint(
        &self,
        implementation: &ImplementationName,
        arguments: &[NormalizedValue],
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        if implementation.as_str() == "core.i64.add"
            && matches!(
                arguments,
                [NormalizedValue::I64(7), NormalizedValue::I64(1)]
            )
        {
            self.entered.fetch_add(1, Ordering::Relaxed);
            // The first relay has received the base answer. All outer relays
            // still await this result, so cancellation occurs after admission
            // and while ordinary call continuations are actually pending.
            control.cancel();
            control.check()?;
        }
        Ok(())
    }
}

impl NormalizedHost for CancelAtRelay<'_> {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &crate::platform::execution::normalized::prepare::NormalizedFunction,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.checkpoint(implementation, &arguments, control)?;
        // The raw Core host omits the word intrinsic used by the binary entry.
        // Keep its exact authored declaration and delegate to the same engine
        // without a custom host, so decoding uses its ordinary checked route.
        if implementation.as_str() == "core.bytes.read-i64-le" {
            return NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
                .invoke(self.read, arguments, None, control)
                .map(|(value, _)| value);
        }
        CoreNormalizedHost.call(program, function, implementation, types, arguments, control)
    }
}

impl NormalizedReferenceHost for CancelAtRelay<'_> {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        function: &ReferenceSignature,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.checkpoint(implementation, &arguments, control)?;
        // Preserve canonical-reference independence for the same checked decode.
        if implementation.as_str() == "core.bytes.read-i64-le" {
            return NormalizedReferenceInterpreter::from_reader(
                self.fixture,
                &self.fixture.program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(self.read, arguments, None, control)
            .map(|(value, _)| value);
        }
        CoreNormalizedReferenceHost.call(
            schema,
            function,
            implementation,
            types,
            arguments,
            control,
        )
    }
}

#[test]
fn selfhost_source_callable_reference_cancels_pending_calls_and_retries_cleanly() -> Check<()> {
    let fixture = Fixture::new()?;
    for selector in [7, 8] {
        let behavior = chain(selector, 257);
        let policy = NormalizedRunPolicy::foreground();
        let vm_before = success(
            vm(
                &fixture,
                behavior,
                policy,
                &ExecutionControl::uncancelled(),
                None,
            )?,
            behavior,
        )?;
        let host = CancelAtRelay::new(&fixture)?;
        let stopped = failure(
            vm(
                &fixture,
                behavior,
                policy,
                &ExecutionControl::uncancelled(),
                Some(&host),
            )?,
            ExecutionFailureClass::Cancelled,
            "execution_cancelled",
        )?;
        equal(
            host.entered.load(Ordering::Relaxed),
            1,
            "VM reached selected pending relay",
        )?;
        equal(
            stopped.maximum_call_depth,
            260,
            "VM cancelled after deep call entry",
        )?;
        equal(
            success(
                vm(
                    &fixture,
                    behavior,
                    policy,
                    &ExecutionControl::uncancelled(),
                    None,
                )?,
                behavior,
            )?,
            vm_before,
            "VM cancellation retry accounting",
        )?;

        on_small_stack(|| {
            let before = success(
                reference(
                    &fixture,
                    behavior,
                    policy,
                    &ExecutionControl::uncancelled(),
                    None,
                )?,
                behavior,
            )?;
            let host = CancelAtRelay::new(&fixture)?;
            let stopped = failure(
                reference(
                    &fixture,
                    behavior,
                    policy,
                    &ExecutionControl::uncancelled(),
                    Some(&host),
                )?,
                ExecutionFailureClass::Cancelled,
                "execution_cancelled",
            )?;
            equal(
                host.entered.load(Ordering::Relaxed),
                1,
                "reference reached selected pending relay",
            )?;
            equal(
                stopped.maximum_call_depth,
                260,
                "reference cancelled after deep call entry",
            )?;
            equal(
                success(
                    reference(
                        &fixture,
                        behavior,
                        policy,
                        &ExecutionControl::uncancelled(),
                        None,
                    )?,
                    behavior,
                )?,
                before,
                "reference cancellation retry accounting",
            )
        })?;
    }
    Ok(())
}

#[test]
fn selfhost_source_callable_reference_frontier_refusal_releases_pending_work() -> Check<()> {
    let fixture = Fixture::new()?;
    let behavior = chain(8, 257);
    let policy = NormalizedRunPolicy::foreground();
    success(
        vm(
            &fixture,
            behavior,
            policy,
            &ExecutionControl::uncancelled(),
            None,
        )?,
        behavior,
    )?;
    on_small_stack(|| {
        let before = success(
            reference(
                &fixture,
                behavior,
                policy,
                &ExecutionControl::uncancelled(),
                None,
            )?,
            behavior,
        )?;
        for allowed_reservations in [0, 8] {
            let refusal = crate::platform::execution::normalized::reference::continuations::
                refuse_frontier_after(allowed_reservations);
            let stopped = failure(
                reference(
                    &fixture,
                    behavior,
                    policy,
                    &ExecutionControl::uncancelled(),
                    None,
                )?,
                ExecutionFailureClass::Resource,
                "normalized_reference_continuation_allocation",
            )?;
            drop(refusal);
            if allowed_reservations != 0 {
                ensure(
                    stopped.maximum_call_depth > 0 && stopped.maximum_control_frames > 0,
                    "late frontier refusal did not enter canonical evaluation",
                )?;
            }
            equal(
                success(
                    reference(
                        &fixture,
                        behavior,
                        policy,
                        &ExecutionControl::uncancelled(),
                        None,
                    )?,
                    behavior,
                )?,
                before.clone(),
                "reference frontier refusal retry accounting",
            )?;
        }
        Ok(())
    })
}

fn established_accounting(work: &Work) -> ([u64; 7], usize) {
    (
        [
            work.steps,
            work.calls,
            work.external_calls,
            work.allocated_bytes,
            work.allocation_charges,
            work.collection_items,
            work.tail_transfers,
        ],
        work.maximum_call_depth,
    )
}

fn selected_reference(
    fixture: &Fixture,
    behavior: Behavior,
    policy: NormalizedRunPolicy,
    legacy: bool,
) -> Check<Outcome> {
    let selection = legacy
        .then(crate::platform::execution::normalized::reference::continuations::force_legacy_route);
    let result = reference(
        fixture,
        behavior,
        policy,
        &ExecutionControl::uncancelled(),
        None,
    );
    drop(selection);
    result
}

#[test]
fn selfhost_source_callable_reference_preserves_established_modeled_accounting() -> Check<()> {
    let fixture = Fixture::new()?;
    let mut cases: Vec<_> = behaviors()
        .into_iter()
        .filter(|behavior| behavior.count <= 32 || behavior.selector == 9)
        .collect();
    for count in [8, 16] {
        cases.push(chain(7, count));
        cases.push(chain(8, count));
    }
    for behavior in &cases {
        let policy = NormalizedRunPolicy::foreground();
        let previous = success(
            selected_reference(&fixture, *behavior, policy, true)?,
            *behavior,
        )?;
        let current = success(
            selected_reference(&fixture, *behavior, policy, false)?,
            *behavior,
        )?;
        equal(
            established_accounting(&current),
            established_accounting(&previous),
            &format!(
                "{}({}): existing reference accounting",
                behavior.name, behavior.count
            ),
        )?;
    }

    // The old evaluator supplies the exact finite allowances independently of
    // the new scheduler. Scratch frontier and owner-read observations are not
    // part of these expression, modeled-allocation or semantic work metrics.
    let behavior = Behavior::new("copyable-capture-universe", 4, 0, 282);
    let baseline = success(
        selected_reference(&fixture, behavior, NormalizedRunPolicy::foreground(), true)?,
        behavior,
    )?;
    for (allocation, exact) in [(false, baseline.steps), (true, baseline.allocated_bytes)] {
        ensure(
            exact > 0,
            "reference accounting control has no metered work",
        )?;
        for allowance in [None, Some(0), Some(exact - 1), Some(exact), Some(u64::MAX)] {
            let mut policy = NormalizedRunPolicy::foreground();
            if allocation {
                policy.maximum_allocated_bytes = allowance;
            } else {
                policy.instruction_steps = allowance;
            }
            let mut observed = Vec::new();
            for legacy in [true, false] {
                let outcome = selected_reference(&fixture, behavior, policy, legacy)?;
                let work = if allowance.is_none_or(|limit| limit >= exact) {
                    let work = success(outcome, behavior)?;
                    equal(
                        established_accounting(&work),
                        established_accounting(&baseline),
                        "exact or unbounded allowance preserves reference work",
                    )?;
                    work
                } else {
                    failure(
                        outcome,
                        ExecutionFailureClass::Resource,
                        if allocation {
                            "normalized_reference_allocation"
                        } else {
                            "normalized_reference_expression_steps"
                        },
                    )?
                };
                observed.push(established_accounting(&work));
            }
            equal(
                observed.first(),
                observed.get(1),
                "legacy and continuation admission stop at the same semantic work",
            )?;
        }
    }
    eprintln!(
        "source callable reference accounting shallow_cases={} finite_dimensions=2 \
         allowance_controls=none,zero,one-short,exact,u64-max \
         comparison=previous-canonical-evaluator-versus-canonical-continuations",
        cases.len(),
    );
    Ok(())
}
