//! A completed capability effect survives later failure and lexical loan cleanup.
use super::super::{owned_i64_cell, owned_product};
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create loan-effect
  (interface create Clock (visibility private)
    (operation create record (returns I64)
      (idempotency non-idempotent) (external-visibility possible)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create left (type I64)) (parameter create right (type I64)) (returns I64))
  (function create effect-main (visibility private) (returns I64)
    (effect (task (requirement command::clock)))
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding packet (type (owned-product (field payload OwnedI64Cell)))
        (pack-owned (type (owned-product (field payload OwnedI64Cell))) (field payload (local cell))))
      (in (borrow-owned-field (type (owned-product (field payload OwnedI64Cell))) (local packet)
        (field payload (binding view (type OwnedI64Cell)))
        (in (sequence
          (capability-call command::clock Clock::record)
          (call divide (i64 1) (i64 0)))))))))
  (function create healthy (visibility private) (returns I64) (effect pure)
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding packet (type (owned-product (field payload OwnedI64Cell)))
        (pack-owned (type (owned-product (field payload OwnedI64Cell))) (field payload (local cell))))
      (in (borrow-owned-field (type (owned-product (field payload OwnedI64Cell))) (local packet)
        (field payload (binding view (type OwnedI64Cell)))
        (in (call read-cell (local view))))))))
  (component create command (visibility private)
    (requirement create clock (interface Clock) (operations Clock::record)
      (limits (maximum_calls 1 calls)))
    (port create main (type (task-function () I64 (row (requirement clock))))
      (function effect-main))))
  (target create borrow-effect (component loan-effect::command) (runner command)
    (port loan-effect::command::main)))
declarations.end"#;

struct RecordingEffect {
    inner: UnitAdapter,
    products: owned_product::StorageObservation,
    cells: owned_i64_cell::StorageObservation,
}

impl NormalizedCapabilityAdapter for RecordingEffect {
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
        assert_eq!(self.cells.live(), (1, 1));
        self.inner.calls.fetch_add(1, Ordering::SeqCst);
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

#[test]
fn owned_borrow_completed_authorized_effect_is_retained_after_later_trap() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    let requirement = program
        .requirements
        .iter()
        .find(|r| r.name.as_str() == "clock")
        .unwrap();
    let operations = requirement
        .operations
        .iter()
        .map(|index| program.operations[index.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let adapter = Arc::new(RecordingEffect {
        inner: UnitAdapter {
            interface: requirement.interface,
            operations: operations.clone(),
            calls: Arc::new(AtomicU64::new(0)),
        },
        products: owned_product::StorageObservation::start(),
        cells: owned_i64_cell::StorageObservation::start(),
    });
    let capabilities = NormalizedCapabilities::bind(
        &program,
        program
            .root_target(&Name::new("borrow-effect").unwrap())
            .unwrap()
            .component,
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
    // Execute this recorded effect once through one engine. The healthy follow-up
    // is a separate pure invocation and cannot replay the capability operation.
    let failed = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground()).invoke(
        declaration_named(&source, "effect-main"),
        vec![],
        Some(&capabilities),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(failed.unwrap_err().class, ExecutionFailureClass::Trap);
    assert_eq!(adapter.inner.calls.load(Ordering::SeqCst), 1);
    assert_eq!(adapter.products.live(), (0, 0));
    assert_eq!(adapter.cells.live(), (0, 0));
    adapter.products.assert_owners_released_after_loans();
    let healthy = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .invoke(
            declaration_named(&source, "healthy"),
            vec![],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .0;
    assert_eq!(healthy, NormalizedValue::I64(137));
    assert_eq!(adapter.inner.calls.load(Ordering::SeqCst), 1);
    assert_eq!(adapter.products.live(), (0, 0));
    assert_eq!(adapter.cells.live(), (0, 0));
    adapter.products.assert_owners_released_after_loans();
}
