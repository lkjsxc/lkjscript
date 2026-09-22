//! Exact transaction participation uses ordinary calls and independently routed evaluators.

use super::super::data::NormalizedDataAdapter;
use super::super::value::{ComponentIndex, OperationIndex, RequirementIndex};
use super::*;
use crate::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use crate::platform::kernel::{OperationReference, TransactionOutcomeContract};

fn member(snapshot: &crate::platform::kernel::KernelSnapshot, parent: &str, name: &str) -> String {
    let declaration = declaration_named(snapshot, parent).declaration;
    snapshot
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Operation(id), OwnerRecord::Operation(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(format!("{}/{id}", snapshot.root.package_id))
            }
            (OwnerKey::Case(id), OwnerRecord::Case(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(format!("{}/{id}", snapshot.root.package_id))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing exact {parent}.{name}"))
}

fn snapshot() -> crate::platform::kernel::KernelSnapshot {
    let standard = GraphRepository::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"),
    )
    .unwrap()
    .view_current()
    .unwrap()
    .reconstruct_full_oracle()
    .unwrap()
    .value;
    let temporary = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&temporary.path().join("participation"), &standard, None)
            .unwrap()
            .repository;
    let base = repository.view_current().unwrap().revision();
    let contract = TransactionOutcomeContract::standard().unwrap();
    let mut request = format!(
        r#"request base={base}
create.module as=$module name=transaction-participation-tests
create.component as=$component module=$module name=participation-component visibility=public
add.requirement as=$store component=$component name=participation-store interface={store}
add.requirement as=$other component=$component name=independent-store interface={store}
"#,
        store = declaration_named(&standard, "DataStore").declaration
    );
    for requirement in ["store", "other"] {
        for (index, name) in ["transaction", "require-transaction", "schema-read", "put"]
            .iter()
            .enumerate()
        {
            request.push_str(&format!(
                "requirement.operation parent=${requirement} index={index} operation={}\n",
                member(&standard, "DataStore", name)
            ));
        }
        request.push_str(&format!("requirement.limit parent=${requirement} index=0 name=maximum_calls maximum=100 unit=calls\n"));
    }
    request.push_str(&format!(r#"
effect.row as=@Store
effect.requirement parent=@Store index=0 requirement=$store
type.task-function as=@Participant result=unit effect=@Store
type.application as=@Outcome declaration={outcome}
type.argument parent=@Outcome index=0 type=unit
type.named as=@KeyPart declaration={key_part}
expression.block as=$callback-body
  (sequence (capability-call $store {read} (static-text "callback")) (unit))
expression.end
create.function as=$callback module=$module name=participation-callback visibility=private result=unit effect=task body=$callback-body
effect.requirement parent=$callback index=0 requirement=$store
expression.block as=$participant-body
  (sequence
    (capability-call $store {guard})
    (capability-call $store {read} (local $label))
    (call $callback))
expression.end
create.function as=$participant module=$module name=participation-helper visibility=public result=unit effect=task body=$participant-body
add.parameter as=$label function=$participant name=label type=static-text
effect.requirement parent=$participant index=0 requirement=$store
expression.block as=$relay-body
  (call $participant (static-text "read"))
expression.end
create.function as=$relay module=$module name=participation-tail-relay visibility=private result=unit effect=task body=$relay-body
effect.requirement parent=$relay index=0 requirement=$store
expression.block as=$owner-body
  (transaction $store (binding owner) (call $relay))
expression.end
create.function as=$owner module=$module name=participation-owner visibility=public result=unit effect=task body=$owner-body
effect.requirement parent=$owner index=0 requirement=$store
expression.block as=$named-body
  (transaction $store (binding named-owner)
    (invoke (function-value $participant) (static-text "read")))
expression.end
create.function as=$named module=$module name=participation-named visibility=public result=unit effect=task body=$named-body
effect.requirement parent=$named index=0 requirement=$store
expression.block as=$factory-body
  (bind (function-value $participant) (static-text "read"))
expression.end
create.function as=$factory module=$module name=participation-factory visibility=public result=@Participant effect=pure body=$factory-body
expression.block as=$bound-body
  (let (binding prepared (type @Participant) (call $factory))
    (in (transaction $store (binding bound-owner) (invoke (local prepared)))))
expression.end
create.function as=$bound module=$module name=participation-bound visibility=public result=unit effect=task body=$bound-body
effect.requirement parent=$bound index=0 requirement=$store
expression.block as=$argument-body
  (call $participant
    (sequence (capability-call $store {read} (static-text "argument")) (static-text "read")))
expression.end
create.function as=$argument module=$module name=participation-effectful-argument visibility=public result=unit effect=task body=$argument-body
effect.requirement parent=$argument index=0 requirement=$store
expression.block as=$retained-body
  (invoke (transaction $store (binding expired-owner) (call $factory)))
expression.end
create.function as=$retained module=$module name=participation-after-exit visibility=public result=unit effect=task body=$retained-body
effect.requirement parent=$retained index=0 requirement=$store
expression.block as=$other-body
  (transaction $other (binding unrelated-owner) (call $relay))
expression.end
create.function as=$other-owner module=$module name=participation-other-owner visibility=public result=unit effect=task body=$other-body
effect.requirement parent=$other-owner index=0 requirement=$store
effect.requirement parent=$other-owner index=1 requirement=$other
expression.block as=$nested-body
  (transaction $store (binding nested-owner) (call $owner))
expression.end
create.function as=$nested module=$module name=participation-nested-owner visibility=public result=unit effect=task body=$nested-body
effect.requirement parent=$nested index=0 requirement=$store
expression.block as=$outcome-body
  (transaction-outcome $store (types unit)
    (outcome {outcome} {reason} {committed} {aborted} {condition} {conflict})
    (binding outcome-owner) (call $relay))
expression.end
create.function as=$outcome-main module=$module name=participation-outcome visibility=public result=@Outcome effect=task body=$outcome-body
effect.requirement parent=$outcome-main index=0 requirement=$store
expression.block as=$condition-body
  (transaction-outcome $store (types unit)
    (outcome {outcome} {reason} {committed} {aborted} {condition} {conflict})
    (binding failed-condition-owner)
    (sequence
      (capability-call $store {put} (static-text "cells")
        (list @KeyPart (variant {text_key} (text "one")))
        (call {encode} (types i64) (i64 1)) (variant {missing}))
      (capability-call $store {put} (static-text "cells")
        (list @KeyPart (variant {text_key} (text "one")))
        (call {encode} (types i64) (i64 2)) (variant {missing}))
      (call $relay)))
expression.end
create.function as=$condition-main module=$module name=participation-after-failed-condition visibility=public result=@Outcome effect=task body=$condition-body
effect.requirement parent=$condition-main index=0 requirement=$store
expression.block as=$staged-body
  (transaction-outcome $store (types unit)
    (outcome {outcome} {reason} {committed} {aborted} {condition} {conflict})
    (binding staged-owner)
    (sequence
      (capability-call $store {put} (static-text "cells")
        (list @KeyPart (variant {text_key} (text "one")))
        (call {encode} (types i64) (i64 1)) (variant {missing}))
      (call $relay)))
expression.end
create.function as=$staged module=$module name=participation-after-staging visibility=public result=@Outcome effect=task body=$staged-body
effect.requirement parent=$staged index=0 requirement=$store
add.port as=$port component=$component name=participation type=@Participant function=$owner
create.target as=$target name=participation component=$component port=$port runner=command
"#,
        guard=member(&standard, "DataStore", "require-transaction"),
        read=member(&standard, "DataStore", "schema-read"),
        put=member(&standard, "DataStore", "put"),
        key_part=declaration_named(&standard, "DataKeyPart").declaration,
        text_key=member(&standard, "DataKeyPart", "Text"),
        missing=member(&standard, "DataExpectation", "Missing"),
        encode=declaration_named(&standard, "data-encode").declaration,
        outcome=contract.outcome.declaration,
        reason=contract.abort_reason.declaration,
        committed=format!("{}/{}", contract.committed.package, contract.committed.case),
        aborted=format!("{}/{}", contract.aborted.package, contract.aborted.case),
        condition=format!("{}/{}", contract.condition_failed.package, contract.condition_failed.case),
        conflict=format!("{}/{}", contract.conflict.package, contract.conflict.case),
    ));
    let decoded =
        crate::platform::control::decode_compact_change("participation", request.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

#[derive(Clone)]
struct ObservedAdapter {
    inner: NormalizedDataAdapter,
    name: &'static str,
    trace: Arc<Mutex<Vec<String>>>,
    fault: &'static str,
    fault_pending: Arc<std::sync::atomic::AtomicBool>,
}

impl NormalizedCapabilityAdapter for ObservedAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        self.inner.kind()
    }
    fn interface(&self) -> DeclarationReference {
        self.inner.interface()
    }
    fn operations(&self) -> &BTreeSet<OperationReference> {
        self.inner.operations()
    }
    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.trace
            .lock()
            .unwrap()
            .push(format!("{}:outside:{}", self.name, policy.operation_name));
        self.inner.call(policy, arguments, resources, control)
    }
    fn begin_transaction(
        &self,
        policy: &NormalizedTransactionPolicy,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        let inner = self.inner.begin_transaction(policy, resources, control)?;
        self.trace
            .lock()
            .unwrap()
            .push(format!("{}:begin", self.name));
        Ok(Box::new(ObservedTransaction {
            inner,
            name: self.name,
            trace: self.trace.clone(),
            fault: self.fault,
            fault_pending: self.fault_pending.clone(),
        }))
    }
}

struct ObservedTransaction {
    inner: Box<dyn NormalizedCapabilityTransaction>,
    name: &'static str,
    trace: Arc<Mutex<Vec<String>>>,
    fault: &'static str,
    fault_pending: Arc<std::sync::atomic::AtomicBool>,
}

impl NormalizedCapabilityTransaction for ObservedTransaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let label = match arguments.first() {
            Some(NormalizedValue::StaticText(label))
                if policy.operation_name.as_str() == "schema-read" =>
            {
                label.as_ref()
            }
            _ => policy.operation_name.as_str(),
        };
        self.trace
            .lock()
            .unwrap()
            .push(format!("{}:{label}", self.name));
        if label == "callback" && self.fault_pending.swap(false, Ordering::Relaxed) {
            match self.fault {
                "cancel" => control.cancel(),
                "trap" => {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Trap,
                        "participation_callback_trap",
                        "controlled callback failure",
                    ));
                }
                _ => {}
            }
        }
        self.inner.call(policy, arguments, resources, control)
    }
    fn commit(
        &mut self,
        control: &ExecutionControl,
    ) -> Result<NormalizedTransactionCompletion, ExecutionError> {
        let completed = self.inner.commit(control)?;
        self.trace
            .lock()
            .unwrap()
            .push(format!("{}:commit:{completed:?}", self.name));
        Ok(completed)
    }
    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.trace
            .lock()
            .unwrap()
            .push(format!("{}:rollback", self.name));
        self.inner.rollback()
    }
}

struct Bound {
    _directory: tempfile::TempDir,
    root: std::path::PathBuf,
    store: DataStore,
    capabilities: NormalizedCapabilities,
    trace: Arc<Mutex<Vec<String>>>,
}

fn component(program: &NormalizedProgram) -> ComponentIndex {
    ComponentIndex(
        program
            .components
            .iter()
            .position(|component| {
                component.requirements.iter().any(|index| {
                    program.requirements[index.0 as usize].name.as_str() == "participation-store"
                })
            })
            .unwrap() as u32,
    )
}

fn bind(program: &NormalizedProgram, maximum_calls: u64, fault: &'static str) -> Bound {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("data");
    DataStore::initialize(&root).unwrap();
    // Both grants deliberately use the same physical store and namespace. Authority, rather
    // than the path, interface, or live engine snapshot, selects an active transaction.
    let store = DataStore::open(&root, "participation", DataLimits::default()).unwrap();
    let trace = Arc::new(Mutex::new(Vec::new()));
    let component = component(program);
    let grants = program.components[component.0 as usize]
        .requirements
        .iter()
        .map(|index| {
            let requirement = &program.requirements[index.0 as usize];
            let adapter =
                NormalizedDataAdapter::admit(program, requirement).unwrap()(store.clone());
            let name = if requirement.name.as_str() == "participation-store" {
                "store"
            } else {
                "other"
            };
            NormalizedCapabilityGrant {
                requirement: requirement.reference,
                descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                    requirement.interface,
                    NormalizedAdapterKind::Data,
                    adapter.operations().clone(),
                    exact_grant_limits(requirement, maximum_calls),
                ),
                adapter: Arc::new(ObservedAdapter {
                    inner: adapter,
                    name,
                    trace: trace.clone(),
                    fault,
                    fault_pending: Arc::new(std::sync::atomic::AtomicBool::new(true)),
                }),
            }
        })
        .collect();
    let capabilities = NormalizedCapabilities::bind(program, component, grants).unwrap();
    Bound {
        _directory: directory,
        root,
        store,
        capabilities,
        trace,
    }
}

fn invoke(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    reference: bool,
    name: &str,
    bound: &Bound,
    policy: NormalizedRunPolicy,
) -> Result<(NormalizedValue, u64), ExecutionError> {
    let entry = declaration_named(snapshot, name);
    let arguments = if name == "participation-helper" {
        vec![NormalizedValue::StaticText("read".into())]
    } else {
        vec![]
    };
    let control = ExecutionControl::uncancelled();
    if reference {
        NormalizedReferenceInterpreter::new(snapshot, program, policy)
            .invoke(entry, arguments, Some(&bound.capabilities), &control)
            .map(|(value, observation)| (value, observation.capability_calls))
    } else {
        NormalizedVm::new(program, policy)
            .invoke(entry, arguments, Some(&bound.capabilities), &control)
            .map(|(value, observation)| (value, observation.capability_calls))
    }
}

fn case(
    program: &NormalizedProgram,
    value: &NormalizedValue,
) -> crate::platform::kernel::CaseReference {
    let NormalizedValue::Variant { layout, case, .. } = value else {
        panic!("expected completion variant: {value:?}")
    };
    program.variants[layout.0 as usize].cases[*case as usize].reference
}

#[test]
fn data_transaction_participation_uses_exact_scope_and_rechecks_retained_descriptors() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    for reference in [false, true] {
        for name in [
            "participation-owner",
            "participation-named",
            "participation-bound",
            "participation-outcome",
        ] {
            let bound = bind(&program, 100, "none");
            let before = bound.store.current_revision().unwrap();
            let (value, observation) = invoke(
                &snapshot,
                &program,
                reference,
                name,
                &bound,
                NormalizedRunPolicy::foreground(),
            )
            .unwrap();
            if name == "participation-outcome" {
                assert_eq!(
                    case(&program, &value),
                    TransactionOutcomeContract::standard().unwrap().committed
                );
            } else {
                assert_eq!(value, NormalizedValue::Unit);
            }
            assert_eq!(
                observation, 4,
                "owner and guard each spend an ordinary call"
            );
            assert_eq!(
                *bound.trace.lock().unwrap(),
                [
                    "store:begin",
                    "store:require-transaction",
                    "store:read",
                    "store:callback",
                    "store:commit:Committed"
                ]
            );
            assert_eq!(bound.store.current_revision().unwrap(), before);
        }
        for (name, expected) in [
            (
                "participation-helper",
                vec!["store:outside:require-transaction"],
            ),
            (
                "participation-other-owner",
                vec![
                    "other:begin",
                    "store:outside:require-transaction",
                    "other:rollback",
                ],
            ),
            (
                "participation-after-exit",
                vec![
                    "store:begin",
                    "store:commit:Committed",
                    "store:outside:require-transaction",
                ],
            ),
            (
                "participation-effectful-argument",
                vec![
                    "store:outside:schema-read",
                    "store:outside:require-transaction",
                ],
            ),
        ] {
            let bound = bind(&program, 100, "none");
            let before = bound.store.current_revision().unwrap();
            let error = invoke(
                &snapshot,
                &program,
                reference,
                name,
                &bound,
                NormalizedRunPolicy::foreground(),
            )
            .unwrap_err();
            assert_eq!(error.code, "normalized_data_transaction_required");
            assert_eq!(error.class, ExecutionFailureClass::Capability);
            assert_eq!(*bound.trace.lock().unwrap(), expected);
            assert_eq!(bound.store.current_revision().unwrap(), before);
        }
        let bound = bind(&program, 100, "none");
        let error = invoke(
            &snapshot,
            &program,
            reference,
            "participation-nested-owner",
            &bound,
            NormalizedRunPolicy::foreground(),
        )
        .unwrap_err();
        assert!(error.code.ends_with("transaction_nested"), "{error:?}");
        assert_eq!(
            *bound.trace.lock().unwrap(),
            ["store:begin", "store:rollback"]
        );
    }
}

#[test]
fn data_transaction_participation_after_false_condition_remains_tentative() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    for reference in [false, true] {
        let bound = bind(&program, 100, "none");
        let before = bound.store.current_revision().unwrap();
        let (value, observation) = invoke(
            &snapshot,
            &program,
            reference,
            "participation-after-failed-condition",
            &bound,
            NormalizedRunPolicy::foreground(),
        )
        .unwrap();
        let contract = TransactionOutcomeContract::standard().unwrap();
        assert_eq!(case(&program, &value), contract.aborted);
        let NormalizedValue::Variant {
            payload: Some(reason),
            ..
        } = value
        else {
            unreachable!()
        };
        assert_eq!(case(&program, &reason), contract.condition_failed);
        assert_eq!(observation, 6);
        assert_eq!(
            *bound.trace.lock().unwrap(),
            [
                "store:begin",
                "store:put",
                "store:put",
                "store:require-transaction",
                "store:read",
                "store:callback",
                "store:commit:ConditionFailed"
            ]
        );
        assert_eq!(bound.store.current_revision().unwrap(), before);
        let reopened =
            DataStore::open(&bound.root, "participation", DataLimits::default()).unwrap();
        let key = DataKey::new(vec![DataKeyPart::Text("one".into())], reopened.limits()).unwrap();
        assert!(
            reopened
                .begin()
                .unwrap()
                .get("cells", &key)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn data_transaction_participation_preserves_call_quotas_cancellation_and_cleanup() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    for reference in [false, true] {
        for (fault, quota, expected) in [
            ("trap", None, ExecutionFailureClass::Trap),
            ("cancel", None, ExecutionFailureClass::Cancelled),
            ("none", Some(2), ExecutionFailureClass::Resource),
            ("none", Some(3), ExecutionFailureClass::Resource),
            ("none", Some(4), ExecutionFailureClass::Resource),
        ] {
            let bound = bind(&program, 100, fault);
            let before = bound.store.current_revision().unwrap();
            let policy = NormalizedRunPolicy {
                maximum_capability_calls: quota,
                ..NormalizedRunPolicy::foreground()
            };
            let error = invoke(
                &snapshot,
                &program,
                reference,
                "participation-after-staging",
                &bound,
                policy,
            )
            .unwrap_err();
            assert_eq!(
                error.class, expected,
                "{reference} {fault} {quota:?}: {error:?}"
            );
            let trace = bound.trace.lock().unwrap().clone();
            assert_eq!(trace.first().unwrap(), "store:begin");
            assert_eq!(trace.last().unwrap(), "store:rollback");
            assert_eq!(
                trace.contains(&"store:require-transaction".to_owned()),
                quota != Some(2)
            );
            assert_eq!(bound.store.current_revision().unwrap(), before);
            let reopened =
                DataStore::open(&bound.root, "participation", DataLimits::default()).unwrap();
            let key =
                DataKey::new(vec![DataKeyPart::Text("one".into())], reopened.limits()).unwrap();
            assert!(
                reopened
                    .begin()
                    .unwrap()
                    .get("cells", &key)
                    .unwrap()
                    .is_none()
            );
            // A separate invocation after the one-shot injected fault succeeds using the
            // same grant and physical engine, with a fresh cancellation/accounting scope.
            let (value, calls) = invoke(
                &snapshot,
                &program,
                reference,
                "participation-after-staging",
                &bound,
                NormalizedRunPolicy::foreground(),
            )
            .unwrap();
            assert_eq!(
                case(&program, &value),
                TransactionOutcomeContract::standard().unwrap().committed
            );
            assert_eq!(calls, 5);
            assert_ne!(bound.store.current_revision().unwrap(), before);
        }
        let bound = bind(&program, 3, "none");
        let error = invoke(
            &snapshot,
            &program,
            reference,
            "participation-owner",
            &bound,
            NormalizedRunPolicy::foreground(),
        )
        .unwrap_err();
        assert!(error.code.contains("grant"), "{error:?}");
        assert_eq!(
            *bound.trace.lock().unwrap(),
            [
                "store:begin",
                "store:require-transaction",
                "store:read",
                "store:rollback"
            ]
        );
    }
}

fn store_requirement(program: &NormalizedProgram) -> RequirementIndex {
    RequirementIndex(
        program
            .requirements
            .iter()
            .position(|requirement| requirement.name.as_str() == "participation-store")
            .unwrap() as u32,
    )
}

fn guard_operation(program: &NormalizedProgram) -> OperationIndex {
    OperationIndex(
        program
            .operations
            .iter()
            .position(|operation| operation.name.as_str() == "require-transaction")
            .unwrap() as u32,
    )
}

#[test]
fn data_transaction_participation_does_not_open_a_snapshot_or_outlive_adapter_completion() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    let bound = bind(&program, 100, "none");
    let requirement = store_requirement(&program);
    let guard = guard_operation(&program);
    let limits = DataLimits {
        maximum_live_transactions: 1,
        ..DataLimits::default()
    };
    let store = DataStore::open(&bound.root, "participation", limits).unwrap();
    let adapter =
        NormalizedDataAdapter::admit(&program, &program.requirements[requirement.0 as usize])
            .unwrap()(store.clone());
    let scope = NormalizedResourceScope::new().unwrap();
    let control = ExecutionControl::uncancelled();
    let call = bound
        .capabilities
        .call_policy(&program, requirement, guard)
        .unwrap();
    let owner = bound
        .capabilities
        .transaction_policy(&program, requirement)
        .unwrap();
    let before = store.current_revision().unwrap();
    let mut transaction = adapter.begin_transaction(&owner, &scope, &control).unwrap();
    assert_eq!(store.begin().err().unwrap().code, "data_live_transactions");
    // A guard outside the evaluator's selected scope cannot borrow this physically live
    // snapshot or try to open another one, even while the physical limit is exhausted.
    let error = adapter.call(&call, vec![], &scope, &control).unwrap_err();
    assert_eq!(error.code, "normalized_data_transaction_required");
    assert_eq!(
        transaction.call(&call, vec![], &scope, &control).unwrap(),
        NormalizedValue::Unit
    );
    assert_eq!(
        transaction
            .call(&call, vec![NormalizedValue::Unit], &scope, &control)
            .unwrap_err()
            .code,
        "normalized_data_argument"
    );
    let cancelled = ExecutionControl::cancel_after_checks(0);
    assert_eq!(
        transaction
            .call(&call, vec![], &scope, &cancelled)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(
        transaction.commit(&control).unwrap(),
        NormalizedTransactionCompletion::Committed
    );
    assert_eq!(
        transaction
            .call(&call, vec![], &scope, &control)
            .unwrap_err()
            .code,
        "normalized_data_transaction_closed"
    );
    assert_eq!(store.current_revision().unwrap(), before);
    transaction.rollback().unwrap();
    let mut fresh = adapter.begin_transaction(&owner, &scope, &control).unwrap();
    assert_eq!(
        fresh.call(&call, vec![], &scope, &control).unwrap(),
        NormalizedValue::Unit
    );
    fresh.rollback().unwrap();
}

#[test]
fn data_transaction_participation_preserves_exact_alias_authority() {
    let mut snapshot = snapshot();
    let helper = declaration_named(&snapshot, "participation-callback");
    let OwnerRecord::Declaration(declaration) =
        &snapshot.owners[&OwnerKey::Declaration(helper.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    let body = function.body;
    let original = function.effect.row().requirements[0];
    let OwnerRecord::Requirement(mut alias) = snapshot.owners[&original.owner()].clone() else {
        unreachable!()
    };
    let alias_id = crate::platform::semantic_id::RequirementId::migrate(
        b"data-participation-canonical-alias",
        0,
    );
    alias.header = OwnerHeader::new(OwnerKey::Requirement(alias_id), OwnerKind::Requirement);
    alias.declaration = helper.declaration;
    let alias_reference = crate::platform::kernel::RequirementReference {
        package: helper.package,
        requirement: alias_id,
    };
    snapshot.owners.insert(
        OwnerKey::Requirement(alias_id),
        OwnerRecord::Requirement(alias),
    );
    let OwnerRecord::Declaration(declaration) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(helper.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Task {
        requirements: vec![alias_reference.into()],
        effect_parameters: vec![],
    };
    let mut pending = vec![body];
    while let Some(expression) = pending.pop() {
        let OwnerRecord::Expression(record) = snapshot
            .owners
            .get_mut(&OwnerKey::Expression(expression))
            .unwrap()
        else {
            unreachable!()
        };
        pending.extend(record.children().into_iter().map(|child| child.expression));
        if let ExpressionOperation::CapabilityCall { requirement, .. } = &mut record.operation {
            *requirement = alias_reference.into();
        }
    }
    let guard_operation = snapshot
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Operation(operation), OwnerRecord::Operation(record))
                if record.name.as_str() == "require-transaction" =>
            {
                Some(OperationReference {
                    package: helper.package,
                    operation: *operation,
                })
            }
            _ => None,
        })
        .unwrap();
    let guard = ExpressionId::migrate(b"data-participation-canonical-alias", 0);
    snapshot.owners.insert(
        OwnerKey::Expression(guard),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                guard,
                ExpressionOperation::CapabilityCall {
                    requirement: alias_reference.into(),
                    operation: guard_operation,
                    arguments: vec![],
                },
            )
            .unwrap(),
        ),
    );
    let OwnerRecord::Expression(root) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Sequence { items } = &mut root.operation else {
        unreachable!()
    };
    items.insert(0, guard);
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    for reference in [false, true] {
        let bound = bind(&program, 100, "none");
        assert_eq!(
            bound
                .capabilities
                .canonical_requirement_exact(&program, alias_reference)
                .unwrap(),
            bound
                .capabilities
                .canonical_requirement_exact(&program, original.concrete().unwrap())
                .unwrap()
        );
        assert_eq!(
            invoke(
                &snapshot,
                &program,
                reference,
                "participation-owner",
                &bound,
                NormalizedRunPolicy::foreground()
            )
            .unwrap()
            .0,
            NormalizedValue::Unit
        );
        assert_eq!(
            *bound.trace.lock().unwrap(),
            [
                "store:begin",
                "store:require-transaction",
                "store:read",
                "store:require-transaction",
                "store:callback",
                "store:commit:Committed"
            ]
        );
    }
}

#[test]
fn data_transaction_participation_cannot_expand_operations_grants_or_task_allowances() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    let helper = declaration_named(&snapshot, "participation-helper");
    for reference in [false, true] {
        // Deliberately narrow already-prepared metadata to probe both runtime defenses,
        // independently of the source validator that normally rejects this body.
        let mut narrow = program.clone();
        let index = store_requirement(&narrow);
        let guard = guard_operation(&narrow);
        let requirement = &mut Arc::make_mut(&mut narrow.requirements)[index.0 as usize];
        requirement.operations = requirement
            .operations
            .iter()
            .copied()
            .filter(|operation| *operation != guard)
            .collect::<Vec<_>>()
            .into();
        let bound = bind(&narrow, 100, "none");
        let error = invoke(
            &snapshot,
            &narrow,
            reference,
            "participation-helper",
            &bound,
            NormalizedRunPolicy::foreground(),
        )
        .unwrap_err();
        assert_eq!(error.code, "normalized_capability_operation");
        assert!(bound.trace.lock().unwrap().is_empty());
        let no_grant = if reference {
            NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(
                helper,
                vec![NormalizedValue::StaticText("read".into())],
                None,
                &ExecutionControl::uncancelled(),
            )
            .map(|(value, _)| value)
        } else {
            NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(
                    helper,
                    vec![NormalizedValue::StaticText("read".into())],
                    None,
                    &ExecutionControl::uncancelled(),
                )
                .map(|(value, _)| value)
        }
        .unwrap_err();
        assert_eq!(no_grant.class, ExecutionFailureClass::Capability);
        assert!(no_grant.code.contains("unbound"), "{no_grant:?}");
    }
    for pure in [false, true] {
        let schema = super::super::NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
        let mut narrowed = snapshot.clone();
        let OwnerRecord::Declaration(declaration) = narrowed
            .owners
            .get_mut(&OwnerKey::Declaration(helper.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut declaration.payload else {
            unreachable!()
        };
        function.effect = if pure {
            FunctionEffect::Pure
        } else {
            FunctionEffect::Task {
                requirements: vec![],
                effect_parameters: vec![],
            }
        };
        if pure {
            declaration.header.kind = OwnerKind::PureFunction;
        }
        assert!(
            crate::platform::kernel::validate_full(&narrowed).is_err(),
            "the current source validator rejects capability calls without task allowances"
        );
        let reader = FaultedReferenceRead {
            source: &narrowed,
            schema: Arc::new(schema),
        };
        let bound = bind(&program, 100, "none");
        let error = NormalizedReferenceInterpreter::from_reader(
            &reader,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(
            helper,
            vec![NormalizedValue::StaticText("read".into())],
            Some(&bound.capabilities),
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert!(error.message.contains("allowance"), "{error:?}");
        assert!(bound.trace.lock().unwrap().is_empty());
        let mut narrowed = program.clone();
        let index = narrowed.function(helper).unwrap();
        let function = &mut Arc::make_mut(&mut narrowed.functions)[index.0 as usize];
        function.effect = if pure {
            FunctionEffect::Pure
        } else {
            FunctionEffect::Task {
                requirements: vec![],
                effect_parameters: vec![],
            }
        };
        function.task_requirements = Arc::from([]);
        let bound = bind(&narrowed, 100, "none");
        let error = invoke(
            &snapshot,
            &narrowed,
            false,
            "participation-helper",
            &bound,
            NormalizedRunPolicy::foreground(),
        )
        .unwrap_err();
        assert!(error.message.contains("declared row"), "{error:?}");
        assert!(bound.trace.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn data_transaction_participation_contract_is_admitted_before_opening_unavailable_stores() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    let directory = tempfile::tempdir().unwrap();
    let requirement = store_requirement(&program);
    let guard = guard_operation(&program);
    for fault in [
        "parameter",
        "result",
        "idempotency",
        "visibility",
        "unknown",
        "foreign-identity",
    ] {
        let mut malformed = program.clone();
        let parameter = program
            .operations
            .iter()
            .find_map(|operation| operation.parameters.first())
            .unwrap()
            .clone();
        let boolean = program
            .types
            .iter()
            .find_map(|(digest, object)| matches!(object.form, TypeForm::Bool).then_some(*digest))
            .unwrap();
        let operation = &mut Arc::make_mut(&mut malformed.operations)[guard.0 as usize];
        match fault {
            "parameter" => operation.parameters = Arc::from([parameter]),
            "result" => operation.result = boolean,
            "idempotency" => operation.idempotency = Idempotency::NonIdempotent,
            "visibility" => operation.external_visibility = ExternalVisibility::Possible,
            "foreign-identity" => {
                operation.reference.operation =
                    OperationId::migrate(b"forged-transaction-guard", 0);
            }
            _ => operation.name = Name::new("require-unknown-transaction").unwrap(),
        }
        let expected = if matches!(fault, "unknown" | "foreign-identity") {
            "normalized_data_operation"
        } else {
            "normalized_data_signature"
        };
        assert_eq!(
            NormalizedDataAdapter::admit(
                &malformed,
                &malformed.requirements[requirement.0 as usize]
            )
            .err()
            .unwrap()
            .code,
            expected
        );
        let grants = malformed.components[component(&malformed).0 as usize]
            .requirements
            .iter()
            .map(|index| {
                let requirement = &malformed.requirements[index.0 as usize];
                NormalizedDeploymentGrant {
                    requirement: requirement.reference,
                    sharing_domain: NormalizedSharingDomain::new("participation-preflight")
                        .unwrap(),
                    authority_revision: NormalizedGrantAuthorityRevision::of(
                        b"participation preflight",
                    ),
                    limits: exact_grant_limits(requirement, 100),
                    adapter: NormalizedAdapterDescriptor::Data {
                        root: "unavailable-data-root".into(),
                        namespace: "participation".into(),
                        limits: DataLimits::default(),
                    },
                }
            })
            .collect();
        let error = NormalizedPreparedDeployment::prepare_with_host(
            &malformed,
            Name::new("participation").unwrap(),
            grants,
            NormalizedDeploymentResourcePolicy::default(),
            &SecretCatalog::from_environment(&[]).unwrap(),
            directory.path(),
            tokio::runtime::Handle::current(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(
            error.code, expected,
            "{fault}: malformed contract must fail before unavailable store acquisition"
        );
        assert!(!directory.path().join("unavailable-data-root").exists());
    }
    let mut old_subset = program.requirements[requirement.0 as usize].clone();
    old_subset.operations = old_subset
        .operations
        .iter()
        .copied()
        .filter(|operation| *operation != guard)
        .collect::<Vec<_>>()
        .into();
    assert!(
        NormalizedDataAdapter::admit(&program, &old_subset).is_ok(),
        "older exact operation subsets do not need the new guard"
    );
}
