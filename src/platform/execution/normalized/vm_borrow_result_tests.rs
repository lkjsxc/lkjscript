//! Production borrowed-result custody, provenance and failed handoff coverage.
use super::*;
use crate::platform::execution::normalized::{
    byte_buffer::StorageObservation as Buffers, owned_i64_cell::StorageObservation as Cells,
    owned_product::StorageObservation as Products, tests::byte_buffer_tests,
};
use crate::platform::kernel::{OwnerKey, OwnerRecord};
use crate::platform::publication::GraphRepository;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HandoffFault {
    LeafGuardReservation,
    LexicalGuardReservation,
    AdoptionReservation,
    AfterExtraction,
}

thread_local! {
    static HANDOFF_FAULT: std::cell::Cell<Option<HandoffFault>> = const { std::cell::Cell::new(None) };
}

pub(super) fn handoff_fault(stage: HandoffFault) -> Result<(), ExecutionError> {
    let injected = HANDOFF_FAULT.with(|fault| {
        if fault.get() == Some(stage) {
            fault.set(None);
            true
        } else {
            false
        }
    });
    if injected {
        Err(resource_error(
            "normalized_borrow_handoff_fault",
            "injected packet adoption failure",
        ))
    } else {
        Ok(())
    }
}

pub(crate) const SOURCE: &str = r#"declarations.begin
(units (module create borrowed-results
  (type-alias Packet (owned-product (field cell OwnedI64Cell) (field tag I64)))
  (type-alias Nested (owned-product (field packet Packet) (field tag I64)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create exact (visibility private) (effect pure)
    (parameter create selected (type OwnedI64Cell) (use borrow))
    (parameter create other (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from selected))
    (body (let
      (binding temporary-cell (type OwnedI64Cell) (call new-cell (i64 999)))
      (binding temporary-packet (type Packet) (pack-owned (type Packet)
        (field cell (local temporary-cell)) (field tag (i64 31))))
      (binding temporary-outer (type Nested) (pack-owned (type Nested)
        (field packet (local temporary-packet)) (field tag (i64 37))))
      (in (borrow-owned-field (type Nested) (local temporary-outer)
        (field packet (binding unrelated-inner (type Packet)))
        (in (borrow-owned-field (type Packet) (local unrelated-inner)
          (field cell (binding unrelated-cell (type OwnedI64Cell)))
          (in (local selected)))))))))
  (function create inner (visibility private) (effect pure)
    (parameter create outer (type Nested) (use borrow))
    (returns Packet (borrow-from outer))
    (body (borrow-owned-field (type Nested) (local outer)
      (field packet (binding view (type Packet))) (in (local view)))))
  (function create cell (visibility private) (effect pure)
    (parameter create packet (type Packet) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-owned-field (type Packet) (local packet)
      (field cell (binding view (type OwnedI64Cell))) (in (local view)))))
  (function create deep (visibility private) (effect pure)
    (parameter create outer (type Nested) (use borrow))
    (returns OwnedI64Cell (borrow-from outer))
    (body (borrow-call (call inner (local outer)) (binding packet (type Packet))
      (in (borrow-call (call cell (local packet)) (binding view (type OwnedI64Cell))
        (in (borrow-call (call exact (local view) (local view))
          (binding forwarded (type OwnedI64Cell)) (in (local forwarded)))))))))
  (function create main (visibility public) (effect pure)
    (parameter create fail (type Bool)) (returns I64)
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding packet (type Packet) (pack-owned (type Packet)
        (field cell (local cell)) (field tag (i64 19))))
      (binding outer (type Nested) (pack-owned (type Nested)
        (field packet (local packet)) (field tag (i64 23))))
      (binding observed (type I64)
        (borrow-call (call deep (local outer)) (binding selected (type OwnedI64Cell))
          (in (sequence (field (local outer) (name tag))
            (if (local fail) (call divide (i64 1) (i64 0)) (call read-cell (local selected)))))))
      (binding original (type I64) (unpack-owned (type Nested) (local outer)
        (field packet (binding packet (type Packet))) (field tag (binding tag (type I64)))
        (in (unpack-owned (type Packet) (local packet)
          (field cell (binding cell (type OwnedI64Cell))) (field tag (binding tag (type I64)))
          (in (call finish-cell (local cell)))))))
      (in (call add (local observed) (local original))))))
  (function create panic-main (visibility public) (effect pure) (returns I64)
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding packet (type Packet) (pack-owned (type Packet)
        (field cell (local cell)) (field tag (i64 19))))
      (binding outer (type Nested) (pack-owned (type Nested)
        (field packet (local packet)) (field tag (i64 23))))
      (in (borrow-call (call deep (local outer)) (binding selected (type OwnedI64Cell))
        (in (call add (call read-cell (local selected)) (field (local outer) (name tag)))))))))))
declarations.end"#;

fn fixture() -> (Arc<NormalizedProgram>, BTreeMap<String, FunctionIndex>) {
    static PREPARED: OnceLock<(Arc<NormalizedProgram>, BTreeMap<String, FunctionIndex>)> =
        OnceLock::new();
    let (program, functions) = PREPARED.get_or_init(|| {
        let source = byte_buffer_tests::author_only(SOURCE).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let repository =
            GraphRepository::create(&temporary.path().join("borrowed-results"), &source, None)
                .unwrap()
                .repository;
        let application =
            crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
        let program = application.program;
        let functions = source
            .owners
            .iter()
            .filter_map(|(key, owner)| {
                let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner)) =
                    (key, owner)
                else {
                    return None;
                };
                let index = program.function(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })?;
                Some((owner.name.to_string(), index))
            })
            .collect();
        (program, functions)
    });
    (Arc::clone(program), functions.clone())
}

fn run_clean(
    program: &NormalizedProgram,
    main: FunctionIndex,
    fail: bool,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> (
    Result<NormalizedValue, ExecutionError>,
    Option<NormalizedRunObservation>,
) {
    let products = Products::start();
    let cells = Cells::start();
    let buffers = Buffers::start();
    let observation = Mutex::new(None);
    let result = NormalizedVm::for_test(program, policy)
        .observing_checked(&observation)
        .invoke_entry(
            NormalizedEntryPoint::Function(main),
            vec![NormalizedValue::Bool(fail)],
            None,
            control,
        )
        .map(|pair| pair.0);
    let observation = observation.into_inner().unwrap();
    if let Some(observation) = &observation {
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_operands_after, 0);
        assert_eq!(observation.live_handles_after, 0);
    }
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
    products.assert_owners_released_after_loans();
    if result.is_ok() && products.created() != 0 {
        products.assert_transfers_preserve_allocations();
    }
    (result, observation)
}

#[test]
fn borrowed_result_packets_forward_nested_custody_and_preserve_original_owner() {
    let (program, functions) = fixture();
    let (result, observation) = run_clean(
        &program,
        functions["main"],
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
fn borrowed_result_packets_cancel_exhaust_and_trap_without_residual_loans() {
    let (program, functions) = fixture();
    let main = functions["main"];
    let policy = NormalizedRunPolicy::foreground();
    let (result, observation) = run_clean(
        &program,
        main,
        false,
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap(), NormalizedValue::I64(274));
    let baseline = observation.unwrap();
    let (result, _) = run_clean(
        &program,
        main,
        true,
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Trap);
    let mut cancelled = 0;
    let mut complete = false;
    for checks in 0..8192 {
        let (result, _) = run_clean(
            &program,
            main,
            false,
            policy,
            &ExecutionControl::cancel_after_checks(checks),
        );
        match result {
            Ok(value) => {
                assert_eq!(value, NormalizedValue::I64(274));
                complete = true;
                break;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                cancelled += 1;
            }
        }
    }
    assert!(cancelled > 0 && complete);
    for maximum in (0..baseline.allocated_bytes)
        .step_by(32)
        .chain([baseline.allocated_bytes.saturating_sub(1)])
    {
        let (result, _) = run_clean(
            &program,
            main,
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
    let (result, _) = run_clean(
        &program,
        main,
        false,
        policy,
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap(), NormalizedValue::I64(274));
}

#[test]
fn borrowed_result_runtime_rejects_wrong_parameter_even_when_inputs_alias() {
    let (program, functions) = fixture();
    let mut attacked = (*program).clone();
    let NormalizedFunctionBody::Code(code) =
        &mut Arc::make_mut(&mut attacked.functions)[functions["exact"].0 as usize].body
    else {
        panic!("graph code");
    };
    let mut instructions = code.instructions.to_vec();
    let load = instructions
        .iter_mut()
        .find(|instruction| {
            matches!(
                instruction,
                NormalizedInstruction::LoadLocal {
                    local: 0,
                    use_mode: ParameterUse::Borrow
                }
            )
        })
        .unwrap();
    *load = NormalizedInstruction::LoadLocal {
        local: 1,
        use_mode: ParameterUse::Borrow,
    };
    code.instructions = instructions.into();
    let (result, _) = run_clean(
        &attacked,
        functions["main"],
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    let error = result.unwrap_err();
    assert_eq!(error.code, "normalized_runtime_type");
    assert!(
        error.message.contains("exact source parameter"),
        "{error:?}"
    );
}

#[test]
fn borrowed_result_failed_adoption_packet_owns_cleanup_after_callee_is_gone() {
    let (program, functions) = fixture();
    HANDOFF_FAULT.with(|fault| fault.set(Some(HandoffFault::AfterExtraction)));
    let (failure, _) = run_clean(
        &program,
        functions["main"],
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(failure.unwrap_err().code, "normalized_borrow_handoff_fault");
    assert!(
        HANDOFF_FAULT.with(|fault| fault.get()).is_none(),
        "the failure must occur after packet extraction"
    );
    let (healthy, _) = run_clean(
        &program,
        functions["main"],
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(healthy.unwrap(), NormalizedValue::I64(274));
}

#[test]
fn borrowed_result_reservation_failures_leave_custody_at_each_exact_stage() {
    let (program, functions) = fixture();
    for stage in [
        HandoffFault::LeafGuardReservation,
        HandoffFault::LexicalGuardReservation,
        HandoffFault::AdoptionReservation,
    ] {
        HANDOFF_FAULT.with(|fault| fault.set(Some(stage)));
        let (failure, _) = run_clean(
            &program,
            functions["main"],
            false,
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(
            failure.unwrap_err().code,
            "normalized_borrow_handoff_fault",
            "{stage:?}"
        );
        assert!(
            HANDOFF_FAULT.with(|fault| fault.get()).is_none(),
            "the exact {stage:?} boundary must consume its fault"
        );
        let (healthy, _) = run_clean(
            &program,
            functions["main"],
            false,
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(
            healthy.unwrap(),
            NormalizedValue::I64(274),
            "after {stage:?}"
        );
    }
}

struct PanicHost<'a> {
    fired: std::sync::atomic::AtomicBool,
    products: &'a Products,
    cells: &'a Cells,
}

impl NormalizedHost for PanicHost<'_> {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &super::super::prepare::NormalizedFunction,
        implementation: &crate::platform::kernel::ImplementationName,
        type_arguments: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        if implementation.as_str() == "core.i64.add"
            && arguments == [NormalizedValue::I64(137), NormalizedValue::I64(23)]
        {
            let (owners, loans) = self.products.live();
            assert_eq!(owners, 2, "both selected product ancestors remain live");
            assert!(loans > 0, "ancestor guards remain in the adopted packet");
            let (owners, loans) = self.cells.live();
            assert_eq!(owners, 1, "unrelated callee temporaries have already ended");
            assert!(
                loans > 0,
                "the selected leaf remains borrowed at host unwind"
            );
            self.fired.store(true, std::sync::atomic::Ordering::SeqCst);
            panic!("host unwind after adopting complete borrowed-result packet");
        }
        CoreNormalizedHost.call(
            program,
            function,
            implementation,
            type_arguments,
            arguments,
            control,
        )
    }
}

#[test]
fn borrowed_result_host_unwind_destroys_adopted_packet_before_owning_ancestors() {
    let (program, functions) = fixture();
    let products = Products::start();
    let cells = Cells::start();
    let buffers = Buffers::start();
    let host = PanicHost {
        fired: std::sync::atomic::AtomicBool::new(false),
        products: &products,
        cells: &cells,
    };
    let observation = Mutex::new(None);
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .observing(&observation, &host)
            .invoke_entry(
                NormalizedEntryPoint::Function(functions["panic-main"]),
                vec![],
                None,
                &ExecutionControl::uncancelled(),
            )
    }));
    assert!(unwind.is_err());
    assert!(
        host.fired.load(std::sync::atomic::Ordering::SeqCst),
        "the host must be reached with the adopted packet live"
    );
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
    products.assert_owners_released_after_loans();
    drop((products, cells, buffers));
    let (healthy, _) = run_clean(
        &program,
        functions["main"],
        false,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(healthy.unwrap(), NormalizedValue::I64(274));
}
