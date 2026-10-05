//! Reference-only lifetime observations and fault injection over canonical records.
use super::*;
use crate::platform::execution::normalized::{
    byte_buffer::StorageObservation as Buffers,
    owned_i64_cell::StorageObservation as Cells,
    owned_product::StorageObservation as Products,
    tests::{FaultedReferenceRead, byte_buffer_tests},
};
use crate::platform::publication::GraphRepository;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum FailureStage {
    AfterExtraction,
    ParentReservation,
    AdoptionReservation,
}

thread_local! {
    static FAILURE: std::cell::Cell<Option<FailureStage>> = const { std::cell::Cell::new(None) };
}

pub(super) fn fault(stage: FailureStage) -> Result<(), ExecutionError> {
    let refused = FAILURE.with(|fault| {
        if fault.get() == Some(stage) {
            fault.set(None);
            true
        } else {
            false
        }
    });
    if refused {
        let code = match stage {
            FailureStage::AfterExtraction => "reference_borrow_result_adoption_fault",
            FailureStage::ParentReservation => "reference_borrow_result_parent_reservation_fault",
            FailureStage::AdoptionReservation => {
                "reference_borrow_result_binding_reservation_fault"
            }
        };
        Err(reference_resource(
            code,
            "injected reference custody reservation or adoption failure",
        ))
    } else {
        Ok(())
    }
}

fn fixture() -> (
    KernelSnapshot,
    Arc<NormalizedProgram>,
    Arc<NormalizedReferenceSchema>,
) {
    static FIXTURE: OnceLock<(
        KernelSnapshot,
        Arc<NormalizedProgram>,
        Arc<NormalizedReferenceSchema>,
    )> = OnceLock::new();
    let (source, program, schema) = FIXTURE.get_or_init(|| {
        // Both interpreters receive the same literal native workload; this
        // reader and all lifetime/result observations remain reference-owned.
        let source =
            byte_buffer_tests::author_only(super::super::vm::borrow_result_tests::SOURCE).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &temporary.path().join("reference-borrow-results"),
            &source,
            None,
        )
        .unwrap()
        .repository;
        let program = crate::platform::normalized_lifecycle::prepare_repository(repository)
            .unwrap()
            .program;
        let schema = Arc::new(NormalizedReferenceSchema::reconstruct([&source]).unwrap());
        (source, program, schema)
    });
    (source.clone(), Arc::clone(program), Arc::clone(schema))
}

fn named(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == name =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn run_clean(
    source: &KernelSnapshot,
    program: &NormalizedProgram,
    schema: Arc<NormalizedReferenceSchema>,
    fail: bool,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> (
    Result<NormalizedValue, ExecutionError>,
    Option<NormalizedReferenceObservation>,
) {
    let products = Products::start();
    let cells = Cells::start();
    let buffers = Buffers::start();
    let reader = FaultedReferenceRead { source, schema };
    let observation = Mutex::new(None);
    let result = NormalizedReferenceInterpreter::from_reader(&reader, program, policy)
        .observing_checked(&observation)
        .invoke(
            named(source, "main"),
            vec![NormalizedValue::Bool(fail)],
            None,
            control,
        )
        .map(|pair| pair.0);
    let observation = observation.into_inner().unwrap();
    if let Some(observation) = &observation {
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_control_frames_after, 0);
        assert_eq!(observation.live_local_scopes_after, 0);
        assert_eq!(observation.live_type_scopes_after, 0);
        assert_eq!(observation.live_handles_after, 0);
    }
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
    products.assert_owners_released_after_loans();
    (result, observation)
}

#[test]
fn reference_borrow_result_retains_nested_source_and_disables_tail_replacement() {
    let (source, program, schema) = fixture();
    let (result, observation) = run_clean(
        &source,
        &program,
        schema,
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap(), NormalizedValue::I64(274));
    let observation = observation.unwrap();
    assert!(observation.maximum_call_depth >= 3);
    assert_eq!(observation.tail_transfers, 0);
}

#[test]
fn reference_borrow_result_failed_adoption_keeps_packet_cleanup_owner() {
    let (source, program, schema) = fixture();
    FAILURE.with(|fault| fault.set(Some(FailureStage::AfterExtraction)));
    let (failure, _) = run_clean(
        &source,
        &program,
        Arc::clone(&schema),
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(
        failure.unwrap_err().code,
        "reference_borrow_result_adoption_fault"
    );
    assert!(FAILURE.with(std::cell::Cell::get).is_none());
    let (healthy, _) = run_clean(
        &source,
        &program,
        schema,
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(healthy.unwrap(), NormalizedValue::I64(274));
}

#[test]
fn reference_borrow_result_refuses_each_custody_reservation_before_detachment() {
    let (source, program, schema) = fixture();
    for (stage, code) in [
        (
            FailureStage::ParentReservation,
            "reference_borrow_result_parent_reservation_fault",
        ),
        (
            FailureStage::AdoptionReservation,
            "reference_borrow_result_binding_reservation_fault",
        ),
    ] {
        FAILURE.with(|fault| fault.set(Some(stage)));
        let (failure, _) = run_clean(
            &source,
            &program,
            Arc::clone(&schema),
            false,
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(failure.unwrap_err().code, code);
        assert!(
            FAILURE.with(std::cell::Cell::get).is_none(),
            "the exact requested reservation stage must execute"
        );
        let (healthy, _) = run_clean(
            &source,
            &program,
            Arc::clone(&schema),
            false,
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(healthy.unwrap(), NormalizedValue::I64(274));
    }
}

struct InspectPanic<'a> {
    products: &'a Products,
    cells: &'a Cells,
    unwind: bool,
    reached: std::sync::atomic::AtomicBool,
}

impl NormalizedReferenceHost for InspectPanic<'_> {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        function: &ReferenceSignature,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        if implementation.as_str() == "core.i64.add"
            && arguments == [NormalizedValue::I64(137), NormalizedValue::I64(23)]
        {
            // This ordinary callback can run only after deep's callee has
            // departed and its returned cell has been adopted by the caller.
            assert_eq!(self.products.live().0, 2);
            assert!(self.products.live().1 > 0);
            assert_eq!(self.cells.live().0, 1);
            assert!(self.cells.live().1 > 0);
            self.reached
                .store(true, std::sync::atomic::Ordering::SeqCst);
            if self.unwind {
                panic!("reference host unwind with adopted borrowed-result ancestry");
            }
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
fn reference_borrow_result_host_unwind_releases_adopted_custody_before_owner() {
    let (source, program, schema) = fixture();
    for unwind in [true, false] {
        let products = Products::start();
        let cells = Cells::start();
        let buffers = Buffers::start();
        let reader = FaultedReferenceRead {
            source: &source,
            schema: Arc::clone(&schema),
        };
        let host = InspectPanic {
            products: &products,
            cells: &cells,
            unwind,
            reached: std::sync::atomic::AtomicBool::new(false),
        };
        let observation = Mutex::new(None);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            NormalizedReferenceInterpreter::from_reader(
                &reader,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .observing(&observation, &host)
            .invoke(
                named(&source, "panic-main"),
                vec![],
                None,
                &ExecutionControl::uncancelled(),
            )
            .map(|pair| pair.0)
        }));
        assert!(
            host.reached.load(std::sync::atomic::Ordering::SeqCst),
            "the callback must observe an adopted packet's live ancestry"
        );
        if unwind {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap().unwrap(), NormalizedValue::I64(160));
        }
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
        products.assert_owners_released_after_loans();
    }
}

#[test]
fn reference_borrow_result_rejects_wrong_parameter_even_when_inputs_alias() {
    let (mut source, program, schema) = fixture();
    let exact = named(&source, "exact");
    let OwnerRecord::Declaration(owner) = &source.owners[&OwnerKey::Declaration(exact.declaration)]
    else {
        panic!("function owner")
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        panic!("function payload")
    };
    let body = function.body;
    let other = function.parameters[1];
    let OwnerRecord::Expression(expression) =
        source.owners.get_mut(&OwnerKey::Expression(body)).unwrap()
    else {
        panic!("expression owner")
    };
    expression.operation = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(other),
    };
    // This reader keeps the admitted schema to isolate the independent dynamic
    // identity check, as the VM counterpart isolates a changed LoadLocal.
    let (result, _) = run_clean(
        &source,
        &program,
        schema,
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap_err().code, "normalized_reference_type");
}

#[test]
fn reference_borrow_result_trap_cancellation_and_reservation_leave_no_loans() {
    let (source, program, schema) = fixture();
    let policy = NormalizedRunPolicy::foreground();
    let (healthy, observation) = run_clean(
        &source,
        &program,
        Arc::clone(&schema),
        false,
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(healthy.unwrap(), NormalizedValue::I64(274));
    let baseline = observation.unwrap();
    let (trap, _) = run_clean(
        &source,
        &program,
        Arc::clone(&schema),
        true,
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(trap.unwrap_err().class, ExecutionFailureClass::Trap);
    let mut completed = false;
    let mut cancelled = 0;
    for checks in 0..8192 {
        let (result, _) = run_clean(
            &source,
            &program,
            Arc::clone(&schema),
            false,
            policy,
            &ExecutionControl::cancel_after_checks(checks),
        );
        match result {
            Ok(value) => {
                assert_eq!(value, NormalizedValue::I64(274));
                completed = true;
                break;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                cancelled += 1;
            }
        }
    }
    assert!(completed && cancelled > 0);
    for maximum in (0..baseline.allocated_bytes)
        .step_by(128)
        .chain([baseline.allocated_bytes.saturating_sub(1)])
    {
        let (result, _) = run_clean(
            &source,
            &program,
            Arc::clone(&schema),
            false,
            NormalizedRunPolicy {
                maximum_allocated_bytes: Some(maximum),
                ..policy
            },
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(
            result.unwrap_err().class,
            ExecutionFailureClass::Resource,
            "limit={maximum}"
        );
    }
    let (healthy, _) = run_clean(
        &source,
        &program,
        schema,
        false,
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(healthy.unwrap(), NormalizedValue::I64(274));
}
