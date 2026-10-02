//! Exact task methods preserve custody and never obtain authority from a witness.
use super::*;
use crate::platform::kernel::KernelSnapshot;

pub(super) fn input() -> String {
    let library = [
        include_str!("../../../../tests/fixtures/owned-task-method-library.lkjc"),
        include_str!("../../../../tests/fixtures/owned-task-method-carriers.lkjc"),
        include_str!("../../../../tests/fixtures/owned-task-method-consumer.lkjc"),
    ]
    .join("\n")
    .replace("(use std builtin)", "")
    .replace("std::WallClock", "test_clock::WallClock");
    // Local activation cannot borrow another local component's authority. Invoke
    // the test function directly with the authority component's explicit grant;
    // public tests independently exercise imported entry-port effect closure.
    let port = "    (component create command (visibility private)\n      (port create main\n        (type (task-function (I64) (record (value I64) (bytes I64))\n          (row (requirement task-method::authority::clock))))\n        (function method-main))))\n  (target create task-methods (component task-method-consumer::command)\n    (runner command) (port task-method-consumer::command::main)))";
    assert!(library.contains(port));
    let library = library.replace(port, ")\n  (target create task-methods (component task-method::authority)\n    (runner command) (port task-method::authority::ready)))");
    // A controlled source-defined interface, not the live wall clock. Public tests
    // independently import the real builtin and exercise its deployment adapter.
    format!(
        "declarations.begin\n(units (module create test_clock
      (interface create WallClock (visibility public)
        (operation create utc-milliseconds (returns I64)
          (idempotency idempotent) (external-visibility none)))))\ndeclarations.end\n{library}"
    )
}

pub(super) fn source() -> KernelSnapshot {
    let source = byte_buffer_tests::author_only(&input()).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    source
}

struct ClockAdapter {
    inner: UnitAdapter,
    cancel_after_first: bool,
}
impl NormalizedCapabilityAdapter for ClockAdapter {
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
        let n = self.inner.calls.fetch_add(1, Ordering::Relaxed);
        if self.cancel_after_first && n == 0 {
            control.cancel();
        }
        Ok(NormalizedValue::I64(n as i64))
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

pub(super) fn capabilities(
    program: &NormalizedProgram,
) -> (NormalizedCapabilities, Arc<AtomicU64>) {
    controlled_capabilities(program, false)
}

pub(super) fn controlled_capabilities(
    program: &NormalizedProgram,
    cancel_after_first: bool,
) -> (NormalizedCapabilities, Arc<AtomicU64>) {
    let requirement = program
        .requirements
        .iter()
        .find(|r| r.name.as_str() == "clock")
        .unwrap();
    let operations = requirement
        .operations
        .iter()
        .map(|op| program.operations[op.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let calls = Arc::new(AtomicU64::new(0));
    let grant = NormalizedCapabilityGrant {
        requirement: requirement.reference,
        descriptor: NormalizedCapabilityGrantDescriptor::for_test(
            requirement.interface,
            NormalizedAdapterKind::WallClock,
            operations.clone(),
            exact_grant_limits(requirement, 4),
        ),
        adapter: Arc::new(ClockAdapter {
            inner: UnitAdapter {
                interface: requirement.interface,
                operations,
                calls: Arc::clone(&calls),
            },
            cancel_after_first,
        }),
    };
    let target = program
        .root_target(&Name::new("task-methods").unwrap())
        .unwrap();
    (
        NormalizedCapabilities::bind(program, target.component, vec![grant]).unwrap(),
        calls,
    )
}

pub(super) fn expected(value: NormalizedValue, n: i64) {
    let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) =
        value
    else {
        panic!("expected record");
    };
    let actual = fields
        .iter()
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual,
        BTreeMap::from([
            ("value", NormalizedValue::I64(n)),
            ("bytes", NormalizedValue::I64(1)),
        ])
    );
}
