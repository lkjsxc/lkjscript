//! A lexical transaction outlives a terminal helper chain and commits only at its own exit.
use super::*;
use crate::platform::kernel::{BindingKind, BindingRecord, OperationReference};
use crate::platform::semantic_id::BindingId;

pub(crate) fn fixture(
    nested: bool,
) -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let OwnerRecord::Declaration(mut helper) =
        snapshot.owners[&OwnerKey::Declaration(caller.declaration)].clone()
    else {
        panic!("caller")
    };
    let DeclarationPayload::Function(ref f) = helper.payload else {
        panic!("function")
    };
    let unit = f.result;
    let requirement = f.effect.row().requirements[0];
    let OwnerRecord::Requirement(req) =
        &snapshot.owners[&OwnerKey::Requirement(requirement.requirement)]
    else {
        panic!("requirement")
    };
    let operation = req.operations[0];
    let boolean = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    let OwnerRecord::Operation(op) = snapshot
        .owners
        .get_mut(&OwnerKey::Operation(operation.operation))
        .unwrap()
    else {
        panic!("operation")
    };
    op.result = boolean;
    let seed = b"task-tail-ancestor-transaction";
    let relay = DeclarationId::migrate(seed, 1);
    let owner = BindingId::migrate(seed, 1);
    let child = BindingId::migrate(seed, 2);
    let mut next = 0;
    let mut expression = |snapshot: &mut crate::platform::kernel::KernelSnapshot, operation| {
        next += 1;
        let id = ExpressionId::migrate(seed, next);
        snapshot.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
        );
        id
    };
    let test = expression(
        &mut snapshot,
        ExpressionOperation::CapabilityCall {
            requirement,
            operation,
            arguments: vec![],
        },
    );
    let done = expression(&mut snapshot, ExpressionOperation::Unit {});
    let again = expression(
        &mut snapshot,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package: caller.package,
                declaration: relay,
            },
            type_arguments: vec![],
            effect_arguments: vec![],
            arguments: vec![],
        },
    );
    let body = expression(
        &mut snapshot,
        ExpressionOperation::If {
            condition: test,
            when_true: again,
            when_false: done,
        },
    );
    let body = if nested {
        expression(
            &mut snapshot,
            ExpressionOperation::Transaction {
                requirement,
                binding: child,
                body,
            },
        )
    } else {
        body
    };
    for binding in if nested {
        vec![owner, child]
    } else {
        vec![owner]
    } {
        snapshot.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("lexical-owner").unwrap(),
                kind: BindingKind::Transaction,
                value: None,
                declared_type: Some(unit),
            }),
        );
    }
    helper.header = OwnerHeader::new(OwnerKey::Declaration(relay), OwnerKind::TaskFunction);
    helper.name = Name::new("terminal-helper").unwrap();
    helper.visibility = DeclarationVisibility::Private;
    let DeclarationPayload::Function(ref mut f) = helper.payload else {
        panic!("function")
    };
    f.body = body;
    snapshot.owners.insert(
        OwnerKey::Declaration(relay),
        OwnerRecord::Declaration(helper),
    );
    let call = expression(
        &mut snapshot,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package: caller.package,
                declaration: relay,
            },
            type_arguments: vec![],
            effect_arguments: vec![],
            arguments: vec![],
        },
    );
    let transaction = expression(
        &mut snapshot,
        ExpressionOperation::Transaction {
            requirement,
            binding: owner,
            body: call,
        },
    );
    let OwnerRecord::Declaration(c) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!("caller")
    };
    let DeclarationPayload::Function(f) = &mut c.payload else {
        panic!("function")
    };
    f.body = transaction;
    let mut retained = BTreeSet::new();
    let mut pending = snapshot
        .owners
        .values()
        .flat_map(OwnerRecord::expression_roots)
        .collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if retained.insert(id) {
            let OwnerRecord::Expression(e) = &snapshot.owners[&OwnerKey::Expression(id)] else {
                panic!("expression")
            };
            pending.extend(e.children().into_iter().map(|c| c.expression));
        }
    }
    snapshot
        .owners
        .retain(|key, _| !matches!(key,OwnerKey::Expression(id) if !retained.contains(id)));
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    let root = program.function(caller).unwrap();
    let NormalizedFunctionBody::Code(code) = &program.functions[root.0 as usize].body else {
        panic!("code")
    };
    assert!(code.instructions.iter().any(|i|matches!(i,NormalizedInstruction::Call{function,..} if program.functions[function.0 as usize].declaration.declaration==relay)),"call with pending commit must remain ordinary");
    (snapshot, program)
}

struct Script {
    interface: DeclarationReference,
    operations: BTreeSet<OperationReference>,
    trace: Arc<Mutex<Vec<String>>>,
    mode: &'static str,
}
impl NormalizedCapabilityAdapter for Script {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Configuration
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
        panic!("loop escaped its ancestor transaction")
    }
    fn begin_transaction(
        &self,
        _: &NormalizedTransactionPolicy,
        _: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        self.trace.lock().unwrap().push("begin".into());
        Ok(Box::new(Transaction {
            trace: Arc::clone(&self.trace),
            mode: self.mode,
            calls: 0,
        }))
    }
}
struct Transaction {
    trace: Arc<Mutex<Vec<String>>>,
    mode: &'static str,
    calls: usize,
}
impl NormalizedCapabilityTransaction for Transaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        _: Vec<NormalizedValue>,
        _: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        assert_eq!(policy.requirement, policy.grant_requirement);
        self.trace
            .lock()
            .unwrap()
            .push(format!("step:{}", self.calls));
        self.calls += 1;
        if self.calls == 5 {
            if self.mode == "cancel" {
                control.cancel();
            } else if self.mode == "failure" {
                return Err(ExecutionError::resource(
                    "iteration_transaction_trap",
                    "chosen later failure",
                ));
            }
        }
        Ok(NormalizedValue::Bool(self.calls <= 8192))
    }
    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()?;
        self.trace.lock().unwrap().push("commit".into());
        Ok(())
    }
    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.trace.lock().unwrap().push("rollback".into());
        Ok(())
    }
}

#[test]
fn task_tail_chain_retains_one_ancestor_transaction_until_commit_or_reverse_cleanup() {
    for mode in ["success", "failure", "cancel", "nested"] {
        let (snapshot, program) = fixture(mode == "nested");
        for reference in [false, true] {
            let target = program.root_target(&Name::new("command").unwrap()).unwrap();
            let req = &program.requirements
                [program.components[target.component.0 as usize].requirements[0].0 as usize];
            let operations = req
                .operations
                .iter()
                .map(|i| program.operations[i.0 as usize].reference)
                .collect::<BTreeSet<_>>();
            let trace = Arc::new(Mutex::new(Vec::new()));
            let grants = NormalizedCapabilities::bind(
                &program,
                target.component,
                vec![NormalizedCapabilityGrant {
                    requirement: req.reference,
                    descriptor: exact_grant_descriptor(
                        req,
                        operations.clone(),
                        exact_grant_limits(req, 10_000),
                    ),
                    adapter: Arc::new(Script {
                        interface: req.interface,
                        operations,
                        trace: Arc::clone(&trace),
                        mode,
                    }),
                }],
            )
            .unwrap();
            let policy = NormalizedRunPolicy {
                maximum_call_depth: 64,
                ..Default::default()
            };
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                    .invoke(
                        declaration_named(&snapshot, "caller"),
                        vec![],
                        Some(&grants),
                        &control,
                    )
                    .map(|(v, w)| (v, w.maximum_call_depth))
            } else {
                NormalizedVm::new(&program, policy)
                    .invoke(
                        declaration_named(&snapshot, "caller"),
                        vec![],
                        Some(&grants),
                        &control,
                    )
                    .map(|(v, w)| (v, w.maximum_call_depth))
            };
            let count = if mode == "success" {
                8193
            } else if mode == "nested" {
                0
            } else {
                5
            };
            let mut expected = vec!["begin".to_owned()];
            expected.extend((0..count).map(|n| format!("step:{n}")));
            expected.push(
                if mode == "success" {
                    "commit"
                } else {
                    "rollback"
                }
                .into(),
            );
            assert_eq!(*trace.lock().unwrap(), expected, "{mode}/{reference}");
            if mode == "success" {
                let (v, depth) = result.unwrap();
                assert_eq!(v, NormalizedValue::Unit);
                assert!(depth <= 3);
            } else {
                let error = result.unwrap_err();
                assert!(
                    match mode {
                        "failure" => error.code == "iteration_transaction_trap",
                        "cancel" => error.code == "execution_cancelled",
                        "nested" => error.code.contains("transaction_nested"),
                        _ => false,
                    },
                    "{mode}/{reference}: {error:?}"
                );
            }
        }
    }
}
