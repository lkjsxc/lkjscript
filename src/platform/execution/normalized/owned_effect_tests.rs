//! Caller-owned authority stays distinct from exact static ownership witnesses.
use super::*;
use crate::platform::execution::ExecutionFailureClass;

fn input() -> String {
    let input = [
        include_str!("../../../../examples/owned-effects/library.lkjc"),
        include_str!("../../../../examples/owned-effects/carriers.lkjc"),
        include_str!("../../../../examples/owned-effects/application.lkjc"),
    ]
    .join("\n")
    .replace("(use std builtin)", "")
    .replace("std::WallClock", "controlled::WallClock")
    .replace("std::add", "controlled::add");
    format!(
        "declarations.begin\n(units (module create controlled
          (interface create WallClock (visibility public)
            (operation create utc-milliseconds (returns I64)
              (idempotency idempotent) (external-visibility none)))
          (external create add (visibility public) (implementation core.i64.add)
            (parameter create left (type I64)) (parameter create right (type I64))
            (returns I64))))\ndeclarations.end\n{input}"
    )
}

#[derive(Clone, Copy)]
enum Failure {
    None,
    CancelFirst,
    TrapSecond,
}

struct Clock {
    inner: UnitAdapter,
    name: String,
    trace: Arc<Mutex<Vec<String>>>,
    failure: Failure,
}

impl NormalizedCapabilityAdapter for Clock {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::WallClock
    }

    fn interface(&self) -> DeclarationReference {
        self.inner.interface
    }

    fn operations(&self) -> &BTreeSet<crate::platform::kernel::OperationReference> {
        &self.inner.operations
    }

    fn call(
        &self,
        _policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        assert!(arguments.is_empty());
        let mut trace = self.trace.lock().unwrap();
        trace.push(self.name.clone());
        let count = trace.len();
        match self.failure {
            Failure::CancelFirst if count == 1 => control.cancel(),
            Failure::TrapSecond if count == 2 => {
                return Err(ExecutionError::new(
                    ExecutionFailureClass::Trap,
                    "owned_effect_callback_trap",
                    "controlled callback effect failed",
                ));
            }
            _ => {}
        }
        Ok(NormalizedValue::I64(count as i64))
    }

    fn begin_transaction(
        &self,
        policy: &NormalizedTransactionPolicy,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        self.inner.begin_transaction(policy, resources, control)
    }
}

fn capabilities(
    program: &NormalizedProgram,
    failure: Failure,
) -> (NormalizedCapabilities, Arc<Mutex<Vec<String>>>) {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let grants = ["clock-a", "clock-b"]
        .into_iter()
        .map(|name| {
            let requirement = program
                .requirements
                .iter()
                .find(|requirement| requirement.name.as_str() == name)
                .unwrap();
            let operations = requirement
                .operations
                .iter()
                .map(|operation| program.operations[operation.0 as usize].reference)
                .collect::<BTreeSet<_>>();
            NormalizedCapabilityGrant {
                requirement: requirement.reference,
                descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                    requirement.interface,
                    NormalizedAdapterKind::WallClock,
                    operations.clone(),
                    exact_grant_limits(requirement, 4),
                ),
                adapter: Arc::new(Clock {
                    inner: UnitAdapter {
                        interface: requirement.interface,
                        operations,
                        calls: Arc::new(AtomicU64::new(0)),
                    },
                    name: name.to_owned(),
                    trace: Arc::clone(&trace),
                    failure,
                }),
            }
        })
        .collect();
    let component = program
        .root_target(&Name::new("owned-effects").unwrap())
        .unwrap()
        .component;
    (
        NormalizedCapabilities::bind(program, component, grants).unwrap(),
        trace,
    )
}

fn invoke(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    reference: bool,
    policy: NormalizedRunPolicy,
    capabilities: Option<&NormalizedCapabilities>,
    control: &ExecutionControl,
) -> Result<NormalizedValue, ExecutionError> {
    let entry = declaration_named(source, "main");
    if reference {
        let observed = Mutex::new(None);
        let result = NormalizedReferenceInterpreter::new(source, program, policy)
            .observing_checked(&observed)
            .invoke(entry, vec![], capabilities, control);
        if let Some(observed) = observed.into_inner().unwrap() {
            assert_eq!(observed.live_call_frames_after, 0);
            assert_eq!(observed.live_transactions_after, 0);
            assert_eq!(observed.live_handles_after, 0);
        } else {
            assert_admission_refusal(result.as_ref().err(), policy, capabilities);
        }
        result.map(|(value, _)| value)
    } else {
        let observed = Mutex::new(None);
        let result = NormalizedVm::for_test(program, policy)
            .observing_checked(&observed)
            .invoke(entry, vec![], capabilities, control);
        if let Some(observed) = observed.into_inner().unwrap() {
            assert_eq!(observed.live_call_frames_after, 0);
            assert_eq!(observed.live_operands_after, 0);
            assert_eq!(observed.live_transactions_after, 0);
            assert_eq!(observed.live_handles_after, 0);
        } else {
            assert_admission_refusal(result.as_ref().err(), policy, capabilities);
        }
        result.map(|(value, _)| value)
    }
}

fn assert_admission_refusal(
    error: Option<&ExecutionError>,
    policy: NormalizedRunPolicy,
    capabilities: Option<&NormalizedCapabilities>,
) {
    let error = error.expect("successful execution must publish its cleanup observation");
    assert!(
        (policy.maximum_allocated_bytes == Some(0)
            && error.class == ExecutionFailureClass::Resource)
            || (capabilities.is_none() && error.class == ExecutionFailureClass::Capability),
        "failure after activation must publish its cleanup observation: {error:?}"
    );
}

fn expected(value: NormalizedValue) {
    let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) =
        value
    else {
        panic!("expected complete result record");
    };
    let actual = fields
        .iter()
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual,
        BTreeMap::from([
            ("scalar", NormalizedValue::I64(43)),
            ("rebound", NormalizedValue::I64(43)),
            ("alternate", NormalizedValue::I64(100)),
            ("bytes", NormalizedValue::bytes(vec![73, 2])),
        ])
    );
}

#[test]
fn owned_effects_both_engines_preserve_exact_witnesses_and_distinct_bindings() {
    let source = byte_buffer_tests::author_only(&input()).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let transform = declaration_named(&source, "transform");
    let selected = program
        .functions
        .iter()
        .filter(|function| {
            function.declaration == transform && !function.implementation_arguments.is_empty()
        })
        .collect::<Vec<_>>();
    let mut compared_distinct_authority = 0;
    let mut compared_shared_authority = 0;
    for (index, left) in selected.iter().enumerate() {
        for right in &selected[index + 1..] {
            if left.type_arguments != right.type_arguments {
                continue;
            }
            let (NormalizedFunctionBody::Code(left_code), NormalizedFunctionBody::Code(right_code)) =
                (&left.body, &right.body)
            else {
                panic!("closed transform applications contain graph code")
            };
            if left.effect_arguments != right.effect_arguments
                || left.requirement_arguments != right.requirement_arguments
            {
                assert!(!Arc::ptr_eq(
                    &left_code.instructions,
                    &right_code.instructions
                ));
                compared_distinct_authority += 1;
            } else {
                assert!(Arc::ptr_eq(
                    &left_code.instructions,
                    &right_code.instructions
                ));
                assert_ne!(left.callsites, right.callsites);
                compared_shared_authority += 1;
            }
        }
    }
    assert!(compared_distinct_authority > 0);
    assert!(compared_shared_authority > 0);
    for reference in [false, true] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let (capabilities, trace) = capabilities(&program, Failure::None);
        expected(
            invoke(
                &source,
                &program,
                reference,
                NormalizedRunPolicy {
                    maximum_capability_calls: Some(8),
                    ..NormalizedRunPolicy::foreground()
                },
                Some(&capabilities),
                &ExecutionControl::uncancelled(),
            )
            .unwrap(),
        );
        assert_eq!(
            *trace.lock().unwrap(),
            [
                "clock-a", "clock-b", "clock-b", "clock-a", "clock-a", "clock-b", "clock-b",
                "clock-a",
            ]
        );
        assert_eq!(cells.created(), 3);
        assert_eq!(buffers.created(), 1);
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
    }
}

#[test]
fn owned_effects_failures_dispose_memory_without_replaying_completed_effects() {
    let source = byte_buffer_tests::author_only(&input()).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for (failure, quota, allocation, class, expected_calls) in [
            (
                Failure::CancelFirst,
                None,
                None,
                ExecutionFailureClass::Cancelled,
                1,
            ),
            (
                Failure::TrapSecond,
                None,
                None,
                ExecutionFailureClass::Trap,
                2,
            ),
            (
                Failure::None,
                Some(1),
                None,
                ExecutionFailureClass::Resource,
                1,
            ),
            (
                Failure::None,
                Some(7),
                None,
                ExecutionFailureClass::Resource,
                7,
            ),
            (
                Failure::None,
                None,
                Some(0),
                ExecutionFailureClass::Resource,
                0,
            ),
        ] {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let buffers = super::super::byte_buffer::StorageObservation::start();
            let (capabilities, trace) = capabilities(&program, failure);
            let error = invoke(
                &source,
                &program,
                reference,
                NormalizedRunPolicy {
                    maximum_capability_calls: quota,
                    maximum_allocated_bytes: allocation,
                    ..NormalizedRunPolicy::foreground()
                },
                Some(&capabilities),
                &ExecutionControl::uncancelled(),
            )
            .unwrap_err();
            assert_eq!(error.class, class);
            assert_eq!(trace.lock().unwrap().len(), expected_calls);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
        {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let buffers = super::super::byte_buffer::StorageObservation::start();
            let error = invoke(
                &source,
                &program,
                reference,
                NormalizedRunPolicy::foreground(),
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Capability);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
        // A fresh invocation can use the admitted program after every failure;
        // completed operations belong only to their original adapter observations.
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let (capabilities, trace) = capabilities(&program, Failure::None);
        expected(
            invoke(
                &source,
                &program,
                reference,
                NormalizedRunPolicy {
                    maximum_capability_calls: Some(8),
                    ..NormalizedRunPolicy::foreground()
                },
                Some(&capabilities),
                &ExecutionControl::uncancelled(),
            )
            .unwrap(),
        );
        assert_eq!(trace.lock().unwrap().len(), 8);
        assert_eq!(cells.created(), 3);
        assert_eq!(buffers.created(), 1);
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
    }
}
