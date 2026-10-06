//! Nonempty task rows retain synchronous read custody and exact effect grants.
use super::*;
use crate::platform::execution::ExecutionFailureClass;

const INPUT: &str = r#"declarations.begin
(units (module create borrowed-task-effect
  (type-alias Packet (owned-product (field cell OwnedI64Cell)))
  (type-alias Observation (record (before I64) (recorded I64) (after I64)))
  (interface create Clock (visibility private)
    (operation create record (returns I64)
      (idempotency non-idempotent) (external-visibility possible)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create left (type I64)) (parameter create right (type I64)) (returns I64))
  (function create observe (visibility private)
    (parameter create fail (type Bool))
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns Observation)
    (effect (task (requirement command::clock)))
    (body (let
      (binding before (type I64) (call read-cell (local value)))
      (binding recorded (type I64) (capability-call command::clock Clock::record))
      (binding after (type I64) (call read-cell (local value)))
      (binding checked (type I64) (if (local fail) (call divide (i64 1) (i64 0)) (i64 0)))
      (in (sequence (local checked) (record structural
        (field before (local before)) (field recorded (local recorded)) (field after (local after))))))))
  (owned-contract create Reader (visibility private)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_95000000000000000000000000000001 observe
      (parameters (Bool unrestricted) (Self borrow)) (returns Observation)
      (effect (task (requirement command::clock)))))
  (owned-implementation create Scalar (visibility private)
    (contract Reader) (self OwnedI64Cell)
    (method method_95000000000000000000000000000001 observe))
  (function create borrowed-main (visibility private)
    (parameter create method (type Bool)) (parameter create fail (type Bool))
    (returns (record (before I64) (recorded I64) (after I64) (original I64)))
    (effect (task (requirement command::clock)))
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding packet (type Packet) (pack-owned (type Packet) (field cell (local cell))))
      (binding observed (type Observation)
        (borrow-owned-field (type Packet) (local packet)
          (field cell (binding view (type OwnedI64Cell)))
          (in (if (local method)
            (method-call concrete@Scalar Reader method_95000000000000000000000000000001
              (local fail) (local view))
            (call observe (local fail) (local view))))))
      (binding original (type I64) (unpack-owned (type Packet) (local packet)
        (field cell (binding cell (type OwnedI64Cell)))
        (in (call finish-cell (local cell)))))
      (in (record structural
        (field before (field (local observed) (name before)))
        (field recorded (field (local observed) (name recorded)))
        (field after (field (local observed) (name after)))
        (field original (local original)))))))
  (function create healthy (visibility private) (effect pure) (returns I64)
    (body (let (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (in (call finish-cell (local cell))))))
  (component create command (visibility private)
    (requirement create clock (interface Clock) (operations Clock::record)
      (limits (maximum_calls 1 calls)))
    (port create main
      (type (task-function (Bool Bool)
        (record (before I64) (recorded I64) (after I64) (original I64))
        (row (requirement clock)))) (function borrowed-main)))
  (component create foreign (visibility private)
    (requirement create foreign-clock (interface Clock) (operations Clock::record)
      (limits (maximum_calls 1 calls)))
    (port create ready (type (function () I64)) (function healthy))))
  (target create borrowed-effect (component borrowed-task-effect::command) (runner command)
    (port borrowed-task-effect::command::main))
  (target create foreign-effect (component borrowed-task-effect::foreign) (runner command)
    (port borrowed-task-effect::foreign::ready)))
declarations.end"#;

struct Clock {
    inner: UnitAdapter,
    products: owned_product::StorageObservation,
    cells: owned_i64_cell::StorageObservation,
    cancel: bool,
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
        _: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        assert!(arguments.is_empty());
        assert_eq!(self.products.live(), (1, 1));
        let (owners, readers) = self.cells.live();
        assert_eq!(owners, 1);
        assert!(
            readers >= 2,
            "the lexical view and borrowed task input must remain live"
        );
        self.inner.calls.fetch_add(1, Ordering::SeqCst);
        if self.cancel {
            control.cancel();
        }
        Ok(NormalizedValue::I64(257))
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
    foreign: bool,
    cancel: bool,
) -> (NormalizedCapabilities, Arc<Clock>) {
    let requirement = program
        .requirements
        .iter()
        .find(|requirement| {
            requirement.name.as_str() == if foreign { "foreign-clock" } else { "clock" }
        })
        .unwrap();
    let operations = requirement
        .operations
        .iter()
        .map(|index| program.operations[index.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let adapter = Arc::new(Clock {
        inner: UnitAdapter {
            interface: requirement.interface,
            operations: operations.clone(),
            calls: Arc::new(AtomicU64::new(0)),
        },
        products: owned_product::StorageObservation::start(),
        cells: owned_i64_cell::StorageObservation::start(),
        cancel,
    });
    let target = program
        .root_target(
            &Name::new(if foreign {
                "foreign-effect"
            } else {
                "borrowed-effect"
            })
            .unwrap(),
        )
        .unwrap();
    let capabilities = NormalizedCapabilities::bind(
        program,
        target.component,
        vec![NormalizedCapabilityGrant {
            requirement: requirement.reference,
            descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                requirement.interface,
                NormalizedAdapterKind::WallClock,
                operations,
                exact_grant_limits(requirement, 1),
            ),
            adapter: adapter.clone(),
        }],
    )
    .unwrap();
    (capabilities, adapter)
}

fn invoke(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    reference: bool,
    name: &str,
    arguments: Vec<NormalizedValue>,
    capabilities: Option<&NormalizedCapabilities>,
    control: &ExecutionControl,
) -> Result<NormalizedValue, ExecutionError> {
    let entry = declaration_named(source, name);
    if reference {
        NormalizedReferenceInterpreter::new(source, program, NormalizedRunPolicy::foreground())
            .invoke(entry, arguments, capabilities, control)
            .map(|value| value.0)
    } else {
        NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
            .invoke(entry, arguments, capabilities, control)
            .map(|value| value.0)
    }
}

fn clean(adapter: &Clock) {
    assert_eq!(adapter.products.live(), (0, 0));
    assert_eq!(adapter.cells.live(), (0, 0));
    adapter.products.assert_owners_released_after_loans();
}

#[test]
fn borrowed_task_effects_and_methods_preserve_grants_reads_and_original_owner() {
    let source = byte_buffer_tests::author_only(INPUT).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for method in [false, true] {
            let (capabilities, adapter) = capabilities(&program, false, false);
            let result = invoke(
                &source,
                &program,
                reference,
                "borrowed-main",
                vec![NormalizedValue::Bool(method), NormalizedValue::Bool(false)],
                Some(&capabilities),
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
            let NormalizedValue::Record(super::super::super::value::NormalizedRecord::Structural {
                fields,
            }) = result
            else {
                panic!("expected before/after/original read observations");
            };
            let actual = fields
                .iter()
                .map(|(name, value)| (name.as_str(), value.clone()))
                .collect::<BTreeMap<_, _>>();
            assert_eq!(
                actual,
                BTreeMap::from([
                    ("before", NormalizedValue::I64(137)),
                    ("recorded", NormalizedValue::I64(257)),
                    ("after", NormalizedValue::I64(137)),
                    ("original", NormalizedValue::I64(137)),
                ])
            );
            assert_eq!(adapter.inner.calls.load(Ordering::SeqCst), 1);
            clean(&adapter);
        }
    }
}

#[test]
fn borrowed_task_effect_failures_end_loans_without_replay_or_foreign_authority() {
    let source = byte_buffer_tests::author_only(INPUT).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for method in [false, true] {
            for (foreign, cancel, trap, missing, expected_class, expected_calls) in [
                (
                    false,
                    false,
                    false,
                    true,
                    ExecutionFailureClass::Capability,
                    0,
                ),
                (
                    true,
                    false,
                    false,
                    false,
                    ExecutionFailureClass::Capability,
                    0,
                ),
                (
                    false,
                    true,
                    false,
                    false,
                    ExecutionFailureClass::PossibleVisibility,
                    1,
                ),
                (false, false, true, false, ExecutionFailureClass::Trap, 1),
            ] {
                let (capabilities, adapter) = capabilities(&program, foreign, cancel);
                let result = invoke(
                    &source,
                    &program,
                    reference,
                    "borrowed-main",
                    vec![NormalizedValue::Bool(method), NormalizedValue::Bool(trap)],
                    if missing { None } else { Some(&capabilities) },
                    &ExecutionControl::uncancelled(),
                );
                let error = result.unwrap_err();
                assert_eq!(error.class, expected_class);
                assert!(!error.retryable);
                assert_eq!(error.possibly_visible, cancel);
                if cancel {
                    assert_eq!(error.code, "execution_cancelled");
                }
                if trap {
                    assert_eq!(
                        error.code,
                        if reference {
                            "reference_integer_division"
                        } else {
                            "normalized_integer_division"
                        }
                    );
                }
                assert_eq!(adapter.inner.calls.load(Ordering::SeqCst), expected_calls);
                clean(&adapter);
                let healthy = invoke(
                    &source,
                    &program,
                    reference,
                    "healthy",
                    vec![],
                    None,
                    &ExecutionControl::uncancelled(),
                )
                .unwrap();
                assert_eq!(healthy, NormalizedValue::I64(137));
                assert_eq!(adapter.inner.calls.load(Ordering::SeqCst), expected_calls);
                clean(&adapter);
            }
        }
    }
}
