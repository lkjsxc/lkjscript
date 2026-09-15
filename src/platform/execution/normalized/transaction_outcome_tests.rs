//! Controlled adapters prove evaluator completion and cleanup; physical tests own visibility.
use super::*;
use crate::platform::kernel::{OperationReference, TransactionOutcomeContract};

pub(crate) fn snapshot() -> crate::platform::kernel::KernelSnapshot {
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
    let repository = GraphRepository::create(&temporary.path().join("completion"), &standard, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let c = TransactionOutcomeContract::standard().unwrap();
    let request = format!(
        r#"request base={base}
create.module as=$module name=transaction-completion-tests
create.component as=$component module=$module name=completion-component visibility=public
add.requirement as=$store component=$component name=completion-store interface=decl_640e96fa57dee1c09557eb4bc7b53398
requirement.operation parent=$store index=0 operation=pkg_10000000000000000000000000000001/op_1c083402875f8f088541c27751f61d22
requirement.operation parent=$store index=1 operation=pkg_10000000000000000000000000000001/op_447868fc8e76bf7946b47b5869cb6131
requirement.limit parent=$store index=0 name=maximum_calls maximum=100 unit=calls
type.application as=@Outcome declaration={outcome}
type.argument parent=@Outcome index=0 type=i64
type.application as=@BoolOutcome declaration={outcome}
type.argument parent=@BoolOutcome index=0 type=bool
expression.static-text as=$space value=completion
expression.capability-call as=$read requirement=$store operation=pkg_10000000000000000000000000000001/op_447868fc8e76bf7946b47b5869cb6131
expression.argument parent=$read index=0 expression=$space
expression.i64 as=$value value=42
expression.sequence as=$body
expression.argument parent=$body index=0 expression=$read
expression.argument parent=$body index=1 expression=$value
expression.transaction-outcome as=$result requirement=$store binding=$owner name=owner body=$body type=i64 outcome={outcome} abort-reason={reason} committed={committed} aborted={aborted} condition-failed={condition} conflict={conflict}
create.function as=$main module=$module name=observe-completion visibility=public result=@Outcome effect=task body=$result
effect.requirement parent=$main index=0 requirement=$store
expression.static-text as=$old-space value=completion
expression.capability-call as=$old-read requirement=$store operation=pkg_10000000000000000000000000000001/op_447868fc8e76bf7946b47b5869cb6131
expression.argument parent=$old-read index=0 expression=$old-space
expression.i64 as=$old-value value=41
expression.sequence as=$old-body
expression.argument parent=$old-body index=0 expression=$old-read
expression.argument parent=$old-body index=1 expression=$old-value
expression.transaction as=$old-result requirement=$store binding=$old-owner name=owner body=$old-body
create.function as=$legacy module=$module name=observe-legacy-completion visibility=public result=i64 effect=task body=$old-result
effect.requirement parent=$legacy index=0 requirement=$store
expression.bool as=$false value=false
expression.transaction-outcome as=$no-write requirement=$store binding=$read-owner name=owner body=$false type=bool outcome={outcome} abort-reason={reason} committed={committed} aborted={aborted} condition-failed={condition} conflict={conflict}
create.function as=$readonly module=$module name=observe-no-write visibility=public result=@BoolOutcome effect=task body=$no-write
effect.requirement parent=$readonly index=0 requirement=$store
create.record as=$Payload module=$module name=CompletionPayload visibility=public
add.field as=$message record=$Payload name=message type=text
type.named as=@Payload declaration=$Payload
expression.text as=$message-value value=ordinary-payload
expression.record as=$payload type=$Payload
expression.record-field parent=$payload index=0 field=$message value=$message-value
expression.transaction-outcome as=$discarded requirement=$store binding=$discard-owner name=owner body=$payload type=@Payload outcome={outcome} abort-reason={reason} committed={committed} aborted={aborted} condition-failed={condition} conflict={conflict}
expression.unit as=$done
expression.sequence as=$discard-body
expression.argument parent=$discard-body index=0 expression=$discarded
expression.argument parent=$discard-body index=1 expression=$done
create.function as=$discard-function module=$module name=discard-outcome visibility=public result=unit effect=task body=$discard-body
effect.requirement parent=$discard-function index=0 requirement=$store
expression.i64 as=$supplied value=7
create.function as=$supplier module=$module name=completion-supplier visibility=private result=i64 effect=pure body=$supplied
type.function as=@Supplier result=i64
add.port as=$supplier-port component=$component name=supplier type=@Supplier function=$supplier
type.application as=@CallableOutcome declaration={outcome}
type.argument parent=@CallableOutcome index=0 type=@Supplier
expression.function-value as=$callable function=$supplier
expression.transaction-outcome as=$callable-result requirement=$store binding=$callable-owner name=owner body=$callable type=@Supplier outcome={outcome} abort-reason={reason} committed={committed} aborted={aborted} condition-failed={condition} conflict={conflict}
create.function as=$callable-function module=$module name=observe-callable-outcome visibility=public result=@CallableOutcome effect=task body=$callable-result
effect.requirement parent=$callable-function index=0 requirement=$store
type.application as=@NestedOutcome declaration={outcome}
type.argument parent=@NestedOutcome index=0 type=@Outcome
expression.call as=$reentry-call function=$main
expression.transaction-outcome as=$reentry-result requirement=$store binding=$reentry-owner name=owner body=$reentry-call type=@Outcome outcome={outcome} abort-reason={reason} committed={committed} aborted={aborted} condition-failed={condition} conflict={conflict}
create.function as=$reentry module=$module name=observe-reentry visibility=public result=@NestedOutcome effect=task body=$reentry-result
effect.requirement parent=$reentry index=0 requirement=$store
expression.call as=$accepted-call function=$main
expression.i64 as=$one value=1
expression.i64 as=$zero value=0
expression.call as=$lost-result function=decl_2359f838521f88ca35a944d96a3152ac
expression.argument parent=$lost-result index=0 expression=$one
expression.argument parent=$lost-result index=1 expression=$zero
expression.sequence as=$after-commit-body
expression.argument parent=$after-commit-body index=0 expression=$accepted-call
expression.argument parent=$after-commit-body index=1 expression=$lost-result
create.function as=$after-commit module=$module name=completion-then-trap visibility=public result=i64 effect=task body=$after-commit-body
effect.requirement parent=$after-commit index=0 requirement=$store
"#,
        outcome = c.outcome.declaration,
        reason = c.abort_reason.declaration,
        committed = format!("{}/{}", c.committed.package, c.committed.case),
        aborted = format!("{}/{}", c.aborted.package, c.aborted.case),
        condition = format!("{}/{}", c.condition_failed.package, c.condition_failed.case),
        conflict = format!("{}/{}", c.conflict.package, c.conflict.case)
    );
    let decoded = crate::platform::control::decode_compact_change(
        "transaction-completion",
        request.as_bytes(),
    )
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

struct Script {
    interface: DeclarationReference,
    operations: BTreeSet<OperationReference>,
    completion: NormalizedTransactionCompletion,
    fault: &'static str,
    trace: Arc<Mutex<Vec<&'static str>>>,
    independent: bool,
    independent_commits: Arc<AtomicU64>,
}
impl NormalizedCapabilityAdapter for Script {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Data
    }
    fn interface(&self) -> DeclarationReference {
        self.interface
    }
    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.operations
    }
    fn call(
        &self,
        _: &NormalizedCallPolicy,
        _: Vec<NormalizedValue>,
        _: &NormalizedResourceScope,
        _: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        panic!("body call must use transaction")
    }
    fn begin_transaction(
        &self,
        _: &NormalizedTransactionPolicy,
        _: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        self.trace.lock().unwrap().push(if self.independent {
            "other-begin"
        } else {
            "begin"
        });
        Ok(Box::new(Scope {
            completion: self.completion,
            fault: self.fault,
            trace: Arc::clone(&self.trace),
            independent: self.independent,
            independent_commits: Arc::clone(&self.independent_commits),
        }))
    }
}
struct Scope {
    completion: NormalizedTransactionCompletion,
    fault: &'static str,
    trace: Arc<Mutex<Vec<&'static str>>>,
    independent: bool,
    independent_commits: Arc<AtomicU64>,
}
impl NormalizedCapabilityTransaction for Scope {
    fn call(
        &mut self,
        _: &NormalizedCallPolicy,
        _: Vec<NormalizedValue>,
        _: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.trace.lock().unwrap().push(if self.independent {
            "other-body"
        } else {
            "body"
        });
        if self.fault == "cancel" {
            control.cancel();
        }
        if self.fault == "trap" {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Trap,
                "completion_body_trap",
                "controlled body failure",
            ));
        }
        NormalizedValue::list(vec![])
    }
    fn commit(
        &mut self,
        control: &ExecutionControl,
    ) -> Result<NormalizedTransactionCompletion, ExecutionError> {
        control.check()?;
        self.trace.lock().unwrap().push(if self.independent {
            "other-commit"
        } else {
            "commit"
        });
        if self.independent && self.completion == NormalizedTransactionCompletion::Committed {
            self.independent_commits.fetch_add(1, Ordering::Relaxed);
        }
        match self.fault {
            "visible" => Err(ExecutionError::new(
                ExecutionFailureClass::PossibleVisibility,
                "data_head_durability_unknown",
                "controlled uncertain durability",
            )),
            "infrastructure" => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "data_head_stage",
                "controlled previsibility failure",
            )),
            _ => Ok(self.completion),
        }
    }
    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.trace.lock().unwrap().push(if self.independent {
            "other-rollback"
        } else {
            "rollback"
        });
        Ok(())
    }
}
fn capabilities(
    program: &NormalizedProgram,
    completion: NormalizedTransactionCompletion,
    fault: &'static str,
) -> (NormalizedCapabilities, Arc<Mutex<Vec<&'static str>>>) {
    let (caps, trace, _) = capabilities_with_state(program, completion, fault);
    (caps, trace)
}
fn capabilities_with_state(
    program: &NormalizedProgram,
    completion: NormalizedTransactionCompletion,
    fault: &'static str,
) -> (
    NormalizedCapabilities,
    Arc<Mutex<Vec<&'static str>>>,
    Arc<AtomicU64>,
) {
    let component = program
        .components
        .iter()
        .position(|component| {
            component.requirements.iter().any(|index| {
                program.requirements[index.0 as usize].name.as_str() == "completion-store"
            })
        })
        .unwrap();
    let trace = Arc::new(Mutex::new(Vec::new()));
    let independent_commits = Arc::new(AtomicU64::new(0));
    let grants = program.components[component]
        .requirements
        .iter()
        .map(|index| {
            let requirement = &program.requirements[index.0 as usize];
            let independent = requirement.name.as_str() == "independent-store";
            let operations = requirement
                .operations
                .iter()
                .map(|index| program.operations[index.0 as usize].reference)
                .collect::<BTreeSet<_>>();
            NormalizedCapabilityGrant {
                requirement: requirement.reference,
                descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                    requirement.interface,
                    NormalizedAdapterKind::Data,
                    operations.clone(),
                    exact_grant_limits(requirement, 100),
                ),
                adapter: Arc::new(Script {
                    interface: requirement.interface,
                    operations,
                    completion: if independent {
                        NormalizedTransactionCompletion::Committed
                    } else {
                        completion
                    },
                    fault: if independent { "none" } else { fault },
                    trace: Arc::clone(&trace),
                    independent,
                    independent_commits: Arc::clone(&independent_commits),
                }),
            }
        })
        .collect();
    (
        NormalizedCapabilities::bind(
            program,
            super::super::value::ComponentIndex(component as u32),
            grants,
        )
        .unwrap(),
        trace,
        independent_commits,
    )
}
fn invoke(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    reference: bool,
    name: &str,
    capabilities: &NormalizedCapabilities,
    policy: NormalizedRunPolicy,
) -> Result<NormalizedValue, ExecutionError> {
    let entry = declaration_named(snapshot, name);
    let control = ExecutionControl::uncancelled();
    if reference {
        NormalizedReferenceInterpreter::new(snapshot, program, policy)
            .invoke(entry, vec![], Some(capabilities), &control)
            .map(|(value, _)| value)
    } else {
        NormalizedVm::new(program, policy)
            .invoke(entry, vec![], Some(capabilities), &control)
            .map(|(value, _)| value)
    }
}
fn selected_case<'a>(
    program: &NormalizedProgram,
    value: &'a NormalizedValue,
) -> (
    crate::platform::kernel::CaseReference,
    Option<&'a NormalizedValue>,
) {
    let NormalizedValue::Variant {
        layout,
        case,
        payload,
    } = value
    else {
        panic!("expected nominal completion: {value:?}");
    };
    (
        program.variants[layout.0 as usize].cases[*case as usize].reference,
        payload.as_deref(),
    )
}

fn redirected_requirement_snapshot(alias: bool) -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = snapshot();
    let main = declaration_named(&snapshot, "observe-completion");
    let OwnerRecord::Declaration(main_record) =
        &snapshot.owners[&OwnerKey::Declaration(main.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(main_function) = &main_record.payload else {
        unreachable!()
    };
    let root = main_function.body;
    let original = main_function.effect.row().requirements[0];
    let OwnerRecord::Requirement(mut requirement) = snapshot.owners[&original.owner()].clone()
    else {
        unreachable!()
    };
    let component = requirement.declaration;
    let id = crate::platform::semantic_id::RequirementId::migrate(
        b"transaction-outcome-alias-and-independent-store",
        u64::from(alias),
    );
    let replacement = crate::platform::kernel::RequirementOperand::Concrete(
        crate::platform::kernel::RequirementReference {
            package: snapshot.root.package_id,
            requirement: id,
        },
    );
    requirement.header = OwnerHeader::new(OwnerKey::Requirement(id), OwnerKind::Requirement);
    if alias {
        requirement.declaration = main.declaration;
    } else {
        requirement.name = Name::new("independent-store").unwrap();
        let OwnerRecord::Declaration(component) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(component))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Component { requirements, .. } = &mut component.payload else {
            unreachable!()
        };
        requirements.push(id);
        requirements.sort();
    }
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Requirement(id),
                OwnerRecord::Requirement(requirement)
            )
            .is_none()
    );
    for owner in snapshot.owners.values_mut() {
        if let OwnerRecord::Declaration(declaration) = owner
            && let DeclarationPayload::Function(function) = &mut declaration.payload
            && let FunctionEffect::Task { requirements, .. } = &mut function.effect
            && requirements.contains(&original)
        {
            if declaration.header.owner == OwnerKey::Declaration(main.declaration) {
                *requirements = vec![replacement];
            } else if !alias {
                requirements.push(replacement);
                requirements.sort();
                requirements.dedup();
            }
        }
    }
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let OwnerRecord::Expression(expression) =
            snapshot.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
        else {
            unreachable!()
        };
        pending.extend(
            expression
                .children()
                .into_iter()
                .map(|child| child.expression),
        );
        match &mut expression.operation {
            ExpressionOperation::TransactionOutcome { requirement, .. }
            | ExpressionOperation::CapabilityCall { requirement, .. } => *requirement = replacement,
            _ => {}
        }
    }
    let root = snapshot.root.owners;
    snapshot.root.owners =
        MapRoot::from_parts(root.page(), snapshot.owners.len() as u64, root.content());
    crate::platform::kernel::validate_full(&snapshot)
        .expect("admitted separate requirement meanings");
    snapshot
}

#[test]
fn transaction_outcome_canonical_alias_reentry_and_independent_store_visibility() {
    for alias in [false, true] {
        let snapshot = redirected_requirement_snapshot(alias);
        let program = prepare_snapshot(&snapshot);
        for reference in [false, true] {
            let (caps, trace, independent_commits) = capabilities_with_state(
                &program,
                NormalizedTransactionCompletion::ConditionFailed,
                "none",
            );
            let result = invoke(
                &snapshot,
                &program,
                reference,
                "observe-reentry",
                &caps,
                NormalizedRunPolicy::default(),
            );
            if alias {
                let error = result.unwrap_err();
                assert_eq!(
                    error.code,
                    if reference {
                        "normalized_reference_transaction_nested"
                    } else {
                        "normalized_transaction_nested"
                    }
                );
                assert_eq!(*trace.lock().unwrap(), ["begin", "rollback"]);
                assert_eq!(independent_commits.load(Ordering::Relaxed), 0);
            } else {
                let value = result.unwrap();
                let contract = TransactionOutcomeContract::standard().unwrap();
                let (case, reason) = selected_case(&program, &value);
                assert_eq!(case, contract.aborted);
                assert_eq!(
                    selected_case(&program, reason.unwrap()),
                    (contract.condition_failed, None)
                );
                assert_eq!(
                    *trace.lock().unwrap(),
                    [
                        "begin",
                        "other-begin",
                        "other-body",
                        "other-commit",
                        "commit"
                    ]
                );
                assert_eq!(
                    independent_commits.load(Ordering::Relaxed),
                    1,
                    "outer condition failure does not undo the independent store completion"
                );
            }
        }
    }
}

#[test]
fn transaction_outcome_completed_choices_preserve_legacy_and_body_once() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    let contract = TransactionOutcomeContract::standard().unwrap();
    for reference in [false, true] {
        for completion in [
            NormalizedTransactionCompletion::Committed,
            NormalizedTransactionCompletion::ConditionFailed,
            NormalizedTransactionCompletion::Conflict,
        ] {
            for legacy in [false, true] {
                let (capabilities, trace) = capabilities(&program, completion, "none");
                let result = invoke(
                    &snapshot,
                    &program,
                    reference,
                    if legacy {
                        "observe-legacy-completion"
                    } else {
                        "observe-completion"
                    },
                    &capabilities,
                    NormalizedRunPolicy::default(),
                );
                assert_eq!(*trace.lock().unwrap(), ["begin", "body", "commit"]);
                if legacy {
                    if completion == NormalizedTransactionCompletion::Conflict {
                        let error = result.unwrap_err();
                        assert_eq!(error.code, "normalized_data_transaction_conflict");
                        assert!(error.retryable);
                    } else {
                        assert_eq!(result.unwrap(), NormalizedValue::I64(41));
                    }
                } else {
                    let value = result.unwrap();
                    let (case, payload) = selected_case(&program, &value);
                    if completion == NormalizedTransactionCompletion::Committed {
                        assert_eq!(
                            (case, payload),
                            (contract.committed, Some(&NormalizedValue::I64(42)))
                        );
                    } else {
                        assert_eq!(case, contract.aborted);
                        let (reason, payload) = selected_case(&program, payload.unwrap());
                        assert_eq!(
                            reason,
                            if completion == NormalizedTransactionCompletion::ConditionFailed {
                                contract.condition_failed
                            } else {
                                contract.conflict
                            }
                        );
                        assert!(payload.is_none());
                    }
                }
            }
        }
        for (name, expected) in [("observe-no-write", false), ("discard-outcome", true)] {
            let (capabilities, trace) =
                capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
            let value = invoke(
                &snapshot,
                &program,
                reference,
                name,
                &capabilities,
                NormalizedRunPolicy::default(),
            )
            .unwrap();
            if expected {
                assert_eq!(value, NormalizedValue::Unit);
            } else {
                assert_eq!(
                    selected_case(&program, &value),
                    (contract.committed, Some(&NormalizedValue::Bool(false)))
                );
            }
            assert_eq!(*trace.lock().unwrap(), ["begin", "commit"]);
        }
        let (caps, trace) =
            capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
        let value = invoke(
            &snapshot,
            &program,
            reference,
            "observe-callable-outcome",
            &caps,
            NormalizedRunPolicy::default(),
        )
        .unwrap();
        let (case, payload) = selected_case(&program, &value);
        assert_eq!(case, contract.committed);
        assert!(matches!(payload, Some(NormalizedValue::Function { .. })));
        let function = program
            .function(declaration_named(&snapshot, "observe-callable-outcome"))
            .unwrap();
        assert!(
            super::super::data_codec::encode_typed(
                &program,
                &value,
                program.functions[function.0 as usize].result
            )
            .is_err(),
            "completion grants no durable encoding to callables"
        );
        assert_eq!(*trace.lock().unwrap(), ["begin", "commit"]);
    }
}

#[test]
fn transaction_outcome_failures_never_become_abort_data_and_cleanup_is_joined() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    for reference in [false, true] {
        for (fault, class, committed) in [
            ("cancel", ExecutionFailureClass::Cancelled, false),
            ("trap", ExecutionFailureClass::Trap, false),
            ("visible", ExecutionFailureClass::PossibleVisibility, true),
            (
                "infrastructure",
                ExecutionFailureClass::Infrastructure,
                true,
            ),
        ] {
            let (capabilities, trace) = capabilities(
                &program,
                NormalizedTransactionCompletion::ConditionFailed,
                fault,
            );
            let error = invoke(
                &snapshot,
                &program,
                reference,
                "observe-completion",
                &capabilities,
                NormalizedRunPolicy::default(),
            )
            .unwrap_err();
            assert_eq!(error.class, class, "{reference} {fault}: {error:?}");
            assert_eq!(
                *trace.lock().unwrap(),
                if committed {
                    vec!["begin", "body", "commit", "rollback"]
                } else {
                    vec!["begin", "body", "rollback"]
                }
            );
        }
    }
}

#[test]
fn transaction_outcome_reentry_is_rejected_and_later_failure_does_not_undo_completion() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    for reference in [false, true] {
        let (caps, trace) =
            capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
        let error = invoke(
            &snapshot,
            &program,
            reference,
            "observe-reentry",
            &caps,
            NormalizedRunPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            if reference {
                "normalized_reference_transaction_nested"
            } else {
                "normalized_transaction_nested"
            }
        );
        assert_eq!(*trace.lock().unwrap(), ["begin", "rollback"]);
        let (caps, trace) =
            capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
        let error = invoke(
            &snapshot,
            &program,
            reference,
            "completion-then-trap",
            &caps,
            NormalizedRunPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Trap);
        assert_eq!(*trace.lock().unwrap(), ["begin", "body", "commit"]);
    }
}

#[test]
fn transaction_outcome_wrapper_capacity_is_reserved_before_physical_commit() {
    let snapshot = snapshot();
    let program = prepare_snapshot(&snapshot);
    let entry = declaration_named(&snapshot, "observe-completion");
    for reference in [false, true] {
        let (caps, _) = capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
        let control = ExecutionControl::uncancelled();
        let bytes = if reference {
            NormalizedReferenceInterpreter::new(&snapshot, &program, NormalizedRunPolicy::default())
                .invoke(entry, vec![], Some(&caps), &control)
                .unwrap()
                .1
                .allocated_bytes
        } else {
            NormalizedVm::new(&program, NormalizedRunPolicy::default())
                .invoke(entry, vec![], Some(&caps), &control)
                .unwrap()
                .1
                .allocated_bytes
        };
        for short in [false, true] {
            let (caps, trace) =
                capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
            let result = invoke(
                &snapshot,
                &program,
                reference,
                "observe-completion",
                &caps,
                NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(bytes - u64::from(short)),
                    ..Default::default()
                },
            );
            if short {
                assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
                assert_eq!(*trace.lock().unwrap(), ["begin", "body", "rollback"]);
            } else {
                assert!(result.is_ok());
                assert_eq!(*trace.lock().unwrap(), ["begin", "body", "commit"]);
            }
        }
    }
}

#[test]
fn transaction_outcome_vm_rejects_forged_completion_before_commit() {
    use super::super::prepare::{NormalizedFunctionBody, NormalizedInstruction};
    let snapshot = snapshot();
    let original = prepare_snapshot(&snapshot);
    let entry = declaration_named(&snapshot, "observe-completion");
    for fault in ["missing", "legacy", "mismatched"] {
        let mut program = original.clone();
        let function = Arc::make_mut(&mut program.functions)
            .iter_mut()
            .find(|function| function.declaration == entry)
            .unwrap();
        let NormalizedFunctionBody::Code(code) = &mut function.body else {
            unreachable!()
        };
        let mut instructions = code.instructions.to_vec();
        let index = instructions
            .iter()
            .position(|instruction| {
                matches!(
                    instruction,
                    NormalizedInstruction::CommitTransactionOutcome { .. }
                )
            })
            .unwrap();
        match fault {
            "missing" => {
                instructions.remove(index);
            }
            "legacy" => {
                let NormalizedInstruction::CommitTransactionOutcome {
                    requirement:
                        super::super::prepare::NormalizedTransactionRequirement::Concrete(requirement),
                    binding,
                    ..
                } = instructions[index]
                else {
                    unreachable!()
                };
                instructions[index] = NormalizedInstruction::CommitTransaction {
                    requirement,
                    binding,
                };
            }
            _ => {
                let NormalizedInstruction::CommitTransactionOutcome {
                    ref mut outcome, ..
                } = instructions[index]
                else {
                    unreachable!()
                };
                outcome.contract.committed = outcome.contract.aborted;
            }
        }
        code.instructions = instructions.into();
        let (caps, trace) =
            capabilities(&program, NormalizedTransactionCompletion::Committed, "none");
        let error = invoke(
            &snapshot,
            &program,
            false,
            "observe-completion",
            &caps,
            NormalizedRunPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            if fault == "missing" {
                "normalized_transaction_leak"
            } else {
                "normalized_transaction_binding"
            }
        );
        assert_eq!(*trace.lock().unwrap(), ["begin", "body", "rollback"]);
    }
}
