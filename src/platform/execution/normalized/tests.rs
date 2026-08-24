//! Focused normalized artifact-preparation and dense-execution tests.

use super::capability::{
    NormalizedCapabilities, NormalizedCapabilityAdapter, NormalizedCapabilityGrant,
    NormalizedCapabilityTransaction,
};
use super::codec::{decode_typed, encode_typed};
use super::prepare::{NormalizedFunctionBody, NormalizedInstruction, NormalizedProgram};
use super::reference::{
    NormalizedReferenceBinding, NormalizedReferenceInterpreter, NormalizedReferenceOwnerRead,
    NormalizedReferenceRead,
};
use super::runner::{
    NormalizedCommandPolicy, run_effectful_command, run_graph_tests, run_pure_command,
};
use super::value::NormalizedValue;
use super::vm::{NormalizedRunPolicy, NormalizedVm};
use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact, load_artifact};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::JsonLimits;
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, DeclarationReference, DeclarationVisibility,
    ExpressionOperation, Name, OperationReference, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord,
    PortImplementation, PortRecord, PortReference, StructuralTypeField, TargetRecord, TypeForm,
    TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::package::RunnerKind;
use crate::platform::publication::{GraphRepository, RepositoryView};
use crate::platform::semantic_id::{DeclarationId, PortId, RevisionId, TargetId};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn declaration_named(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    name: &str,
) -> DeclarationReference {
    let declaration = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*declaration)
            }
            _ => None,
        })
        .expect("named fixture declaration");
    DeclarationReference {
        package: snapshot.root.package_id,
        declaration,
    }
}

fn prepare_snapshot(snapshot: &crate::platform::kernel::KernelSnapshot) -> NormalizedProgram {
    prepare_repository(snapshot).2
}

fn prepare_repository(
    snapshot: &crate::platform::kernel::KernelSnapshot,
) -> (tempfile::TempDir, GraphRepository, NormalizedProgram) {
    let temporary = tempfile::tempdir().expect("normalized runtime parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), snapshot, None)
        .expect("Graph 5 repository");
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("normalized compilation");
    let linked = link_artifact(&created.repository, compilation.manifest_digest, &[])
        .expect("Graph 5 artifact");
    let loaded = load_artifact(&linked.artifact.bytes).expect("strict Graph 5 artifact");
    let program = NormalizedProgram::prepare(loaded).expect("dense runtime preparation");
    (temporary, created.repository, program)
}

fn pure_command_snapshot() -> crate::platform::kernel::KernelSnapshot {
    const SEED: &[u8] = b"normalized-pure-command-runner";

    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let implementation = declaration_named(&snapshot, "with_binding");
    let module = match snapshot
        .owners
        .get(&OwnerKey::Declaration(implementation.declaration))
        .expect("pure command implementation")
    {
        OwnerRecord::Declaration(record) => record.module,
        _ => panic!("pure command implementation owner kind"),
    };
    let function_type = snapshot
        .types
        .iter()
        .find_map(|(digest, object)| match &object.form {
            TypeForm::Function { parameters, result }
                if parameters.is_empty()
                    && snapshot
                        .types
                        .get(result)
                        .is_some_and(|object| matches!(object.form, TypeForm::Unit)) =>
            {
                Some(*digest)
            }
            _ => None,
        })
        .expect("fixture unit command function type");
    let component = DeclarationId::migrate(SEED, 0);
    let port = PortId::migrate(SEED, 0);
    let target = TargetId::migrate(SEED, 0);
    let package = snapshot.root.package_id;

    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Port(port),
                OwnerRecord::Port(PortRecord {
                    header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
                    declaration: component,
                    name: Name::new("run").unwrap(),
                    function_type,
                    implementation: PortImplementation::Function(implementation),
                }),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Declaration(component),
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(
                        OwnerKey::Declaration(component),
                        OwnerKind::Component,
                    ),
                    module,
                    name: Name::new("PureApplication").unwrap(),
                    visibility: DeclarationVisibility::Public,
                    payload: DeclarationPayload::Component {
                        requirements: Vec::new(),
                        ports: vec![port],
                    },
                }),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Target(target),
                OwnerRecord::Target(TargetRecord {
                    header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
                    name: Name::new("pure").unwrap(),
                    component: DeclarationReference {
                        package,
                        declaration: component,
                    },
                    port: PortReference { package, port },
                    runner: RunnerKind::Command,
                }),
            )
            .is_none()
    );
    snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
    );
    snapshot
}

#[derive(Clone)]
struct UnitAdapter {
    interface: DeclarationReference,
    calls: Arc<AtomicU64>,
}

struct WrongRevisionReader<'a>(&'a RepositoryView);

impl NormalizedReferenceRead for WrongRevisionReader<'_> {
    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError> {
        let mut binding = NormalizedReferenceRead::binding(self.0)?;
        binding.revision = Some(RevisionId::from_digest([0x55; 32]));
        Ok(binding)
    }

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        NormalizedReferenceRead::owner(self.0, owner)
    }
}

impl NormalizedCapabilityAdapter for UnitAdapter {
    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn call(
        &self,
        _operation: OperationReference,
        _arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(NormalizedValue::Unit)
    }

    fn begin_transaction(
        &self,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        Ok(Box::new(UnitTransaction))
    }
}

struct UnitTransaction;

impl NormalizedCapabilityTransaction for UnitTransaction {
    fn call(
        &mut self,
        _operation: OperationReference,
        _arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        Ok(NormalizedValue::Unit)
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        Ok(())
    }
}

#[derive(Default)]
struct TransactionStats {
    begins: AtomicU64,
    calls: AtomicU64,
    commits: AtomicU64,
    rollbacks: AtomicU64,
}

#[derive(Clone)]
struct TrackingAdapter {
    interface: DeclarationReference,
    stats: Arc<TransactionStats>,
    fail_transaction_call: bool,
}

impl NormalizedCapabilityAdapter for TrackingAdapter {
    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn call(
        &self,
        _operation: OperationReference,
        _arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        Ok(NormalizedValue::Unit)
    }

    fn begin_transaction(
        &self,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        self.stats.begins.fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(TrackingTransaction {
            stats: Arc::clone(&self.stats),
            fail_call: self.fail_transaction_call,
        }))
    }
}

struct TrackingTransaction {
    stats: Arc<TransactionStats>,
    fail_call: bool,
}

impl NormalizedCapabilityTransaction for TrackingTransaction {
    fn call(
        &mut self,
        _operation: OperationReference,
        _arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.stats.calls.fetch_add(1, Ordering::Relaxed);
        if self.fail_call {
            Err(ExecutionError::new(
                ExecutionFailureClass::PossibleVisibility,
                "normalized_test_possible_visibility",
                "injected capability failure after possible visibility",
            ))
        } else {
            Ok(NormalizedValue::Unit)
        }
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()?;
        self.stats.commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.stats.rollbacks.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn bind_fixture_capability(
    program: &NormalizedProgram,
    maximum_calls: u64,
) -> (NormalizedCapabilities, Arc<AtomicU64>) {
    let target = program
        .root_target(&Name::new("command").unwrap())
        .expect("fixture target");
    let component = &program.components[target.component.0 as usize];
    let requirement = component.requirements[0];
    let requirement_record = &program.requirements[requirement.0 as usize];
    let calls = Arc::new(AtomicU64::new(0));
    let grant = NormalizedCapabilityGrant {
        requirement: requirement_record.reference,
        operations: requirement_record
            .operations
            .iter()
            .map(|operation| program.operations[operation.0 as usize].reference)
            .collect(),
        maximum_calls,
        adapter: Arc::new(UnitAdapter {
            interface: requirement_record.interface,
            calls: Arc::clone(&calls),
        }),
    };
    (
        NormalizedCapabilities::bind(program, target.component, vec![grant])
            .expect("exact fixture grant"),
        calls,
    )
}

fn bind_tracking_capability(
    program: &NormalizedProgram,
    maximum_calls: u64,
    fail_transaction_call: bool,
) -> (NormalizedCapabilities, Arc<TransactionStats>) {
    let target = program
        .root_target(&Name::new("command").unwrap())
        .expect("fixture target");
    let requirement = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement.0 as usize];
    let stats = Arc::new(TransactionStats::default());
    let grant = NormalizedCapabilityGrant {
        requirement: requirement.reference,
        operations: requirement
            .operations
            .iter()
            .map(|operation| program.operations[operation.0 as usize].reference)
            .collect(),
        maximum_calls,
        adapter: Arc::new(TrackingAdapter {
            interface: requirement.interface,
            stats: Arc::clone(&stats),
            fail_transaction_call,
        }),
    };
    (
        NormalizedCapabilities::bind(program, target.component, vec![grant])
            .expect("exact tracked fixture grant"),
        stats,
    )
}

fn transaction_call_snapshot() -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let (body, requirement) = snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Expression(record) => match record.operation {
                ExpressionOperation::Transaction {
                    requirement, body, ..
                } => Some((body, requirement)),
                _ => None,
            },
            _ => None,
        })
        .expect("coverage transaction");
    let operation = snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Expression(record) => match record.operation {
                ExpressionOperation::CapabilityCall { operation, .. } => Some(operation),
                _ => None,
            },
            _ => None,
        })
        .expect("coverage capability operation");
    let OwnerRecord::Expression(body) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body))
        .expect("transaction body expression")
    else {
        panic!("transaction body owner kind")
    };
    body.operation = ExpressionOperation::CapabilityCall {
        requirement,
        operation,
        arguments: Vec::new(),
    };
    snapshot
}

fn transaction_result_snapshot() -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let transaction = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Expression(expression), OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Transaction { .. }) =>
            {
                Some(*expression)
            }
            _ => None,
        })
        .expect("coverage transaction expression");
    let caller = declaration_named(&snapshot, "caller").declaration;
    let root = match &snapshot.owners[&OwnerKey::Declaration(caller)] {
        OwnerRecord::Declaration(record) => match record.payload {
            crate::platform::kernel::DeclarationPayload::Function(ref function) => function.body,
            _ => panic!("caller declaration kind"),
        },
        _ => panic!("caller owner kind"),
    };
    let OwnerRecord::Expression(root) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(root))
        .expect("caller root expression")
    else {
        panic!("caller root owner kind")
    };
    let ExpressionOperation::Sequence { items } = &mut root.operation else {
        panic!("caller root operation")
    };
    let position = items
        .iter()
        .position(|expression| *expression == transaction)
        .expect("transaction sequence item");
    items.remove(position);
    items.push(transaction);
    snapshot
}

#[test]
fn strict_graph5_artifact_prepares_only_dense_runtime_bindings() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == "caller" =>
            {
                Some(*declaration)
            }
            _ => None,
        })
        .expect("caller declaration");
    let program = prepare_snapshot(&snapshot);

    assert_eq!(program.work.packages, 1);
    assert_eq!(program.work.compiler_units, 11);
    assert_eq!(program.work.runtime_owners, 8);
    assert_eq!(program.work.type_objects, 2);
    assert_eq!(program.work.functions, 5);
    assert_eq!(program.work.record_layouts, 1);
    assert_eq!(program.work.variant_layouts, 1);
    assert_eq!(program.work.requirements, 1);
    assert_eq!(program.work.operations, 1);
    assert_eq!(program.work.components, 1);
    assert_eq!(program.work.ports, 1);
    assert_eq!(program.work.targets, 1);
    assert_eq!(program.work.tests, 1);
    assert_eq!(program.records[0].fields[0].name.as_str(), "value");
    assert_eq!(program.variants[0].cases[0].name.as_str(), "Ready");
    assert_eq!(program.operations[0].name.as_str(), "read");
    assert_eq!(program.requirements[0].name.as_str(), "store");
    assert_eq!(program.ports[0].name.as_str(), "run");
    assert_eq!(
        program
            .root_target(&crate::platform::kernel::Name::new("command").unwrap())
            .expect("root command target")
            .runner,
        crate::platform::package::RunnerKind::Command
    );
    let caller = program
        .function(DeclarationReference {
            package: snapshot.root.package_id,
            declaration: caller,
        })
        .expect("dense caller function");
    let function = &program.functions[caller.0 as usize];
    let NormalizedFunctionBody::Code(code) = &function.body else {
        panic!("caller must have normalized code")
    };
    assert!(
        code.instructions
            .iter()
            .any(|instruction| matches!(instruction, NormalizedInstruction::Call { .. }))
    );
    assert!(code.instructions.iter().any(|instruction| matches!(
        instruction,
        NormalizedInstruction::Record {
            layout: Some(_),
            ..
        }
    )));
    assert!(
        code.instructions
            .iter()
            .any(|instruction| matches!(instruction, NormalizedInstruction::Variant { .. }))
    );
    assert!(
        code.instructions
            .iter()
            .any(|instruction| matches!(instruction, NormalizedInstruction::Perform { .. }))
    );
}

fn admit_runtime_type(program: &mut NormalizedProgram, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("valid runtime boundary type");
    let (digest, _) = encode_type_object(&object).expect("canonical runtime boundary type");
    assert!(program.types.insert(digest, object).is_none());
    digest
}

#[test]
fn normalized_json_codec_uses_exact_runtime_layouts_and_bounds() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let mut program = prepare_snapshot(&snapshot);
    let unit = program
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::Unit).then_some(*digest))
        .expect("unit type");
    assert_eq!(
        decode_typed(&program, b"null", unit, JsonLimits::default()).unwrap(),
        NormalizedValue::Unit
    );
    assert_eq!(
        encode_typed(
            &program,
            &NormalizedValue::Unit,
            unit,
            JsonLimits::default(),
        )
        .unwrap(),
        b"null"
    );

    let record_declaration = program.records[0].declaration;
    let record = admit_runtime_type(
        &mut program,
        TypeForm::Named {
            declaration: record_declaration,
        },
    );
    let record_value = decode_typed(
        &program,
        br#"{"value":null}"#,
        record,
        JsonLimits::default(),
    )
    .expect("decode exact nominal record");
    assert_eq!(
        encode_typed(&program, &record_value, record, JsonLimits::default()).unwrap(),
        br#"{"value":null}"#
    );
    assert_eq!(
        decode_typed(
            &program,
            br#"{"other":null}"#,
            record,
            JsonLimits::default(),
        )
        .expect_err("wrong nominal field must reject")
        .code,
        "normalized_json_type"
    );

    let variant_declaration = program.variants[0].declaration;
    let variant = admit_runtime_type(
        &mut program,
        TypeForm::Named {
            declaration: variant_declaration,
        },
    );
    let variant_value = decode_typed(
        &program,
        br#"{"case":"Ready"}"#,
        variant,
        JsonLimits::default(),
    )
    .expect("decode exact nominal variant");
    assert_eq!(
        encode_typed(&program, &variant_value, variant, JsonLimits::default(),).unwrap(),
        br#"{"case":"Ready"}"#
    );

    let boolean = admit_runtime_type(&mut program, TypeForm::Bool);
    let structural = admit_runtime_type(
        &mut program,
        TypeForm::StructuralRecord {
            fields: vec![StructuralTypeField {
                name: Name::new("flag").unwrap(),
                ty: boolean,
            }],
        },
    );
    let structural_value = decode_typed(
        &program,
        br#"{"flag":true}"#,
        structural,
        JsonLimits::default(),
    )
    .expect("decode structural record");
    assert_eq!(
        encode_typed(
            &program,
            &structural_value,
            structural,
            JsonLimits::default(),
        )
        .unwrap(),
        br#"{"flag":true}"#
    );

    let option = admit_runtime_type(&mut program, TypeForm::Option { item: unit });
    assert_eq!(
        decode_typed(&program, b"null", option, JsonLimits::default())
            .expect_err("unrepresented Option boundary must reject exactly")
            .code,
        "normalized_json_type"
    );
    assert_eq!(
        encode_typed(
            &program,
            &NormalizedValue::Unit,
            unit,
            JsonLimits {
                maximum_bytes: 3,
                ..JsonLimits::default()
            },
        )
        .expect_err("output byte bound must reject")
        .code,
        "normalized_json_output_bytes"
    );
}

#[test]
fn normalized_runners_execute_pure_commands_and_graph_owned_tests_differentially() {
    let effectful_snapshot = crate::platform::kernel::tests::witness_snapshot();
    let effectful_program = prepare_snapshot(&effectful_snapshot);
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::default();
    let json_limits = JsonLimits::default();
    let command_policy = NormalizedCommandPolicy {
        execution: policy,
        json: json_limits,
    };
    assert_eq!(
        run_pure_command(
            &effectful_snapshot,
            &effectful_program,
            &Name::new("command").unwrap(),
            b"[]",
            command_policy,
            &control,
        )
        .expect_err("effectful command must not run twice against live grants")
        .code,
        "normalized_runner_grants_required"
    );

    let snapshot = pure_command_snapshot();
    let program = prepare_snapshot(&snapshot);
    let receipt = run_pure_command(
        &snapshot,
        &program,
        &Name::new("pure").unwrap(),
        b"[]",
        command_policy,
        &control,
    )
    .expect("pure command runs through both execution tiers");
    assert_eq!(receipt.target.as_str(), "pure");
    assert_eq!(receipt.revision, None);
    assert_eq!(receipt.result_json, b"null");
    assert_eq!(receipt.differential, "equal");
    assert!(receipt.production.instructions > 0);
    assert!(receipt.reference.expressions > 0);

    assert_eq!(
        run_pure_command(
            &snapshot,
            &program,
            &Name::new("pure").unwrap(),
            b"[null]",
            command_policy,
            &control,
        )
        .expect_err("command argument arity is exact")
        .code,
        "normalized_runner_argument_count"
    );

    let tests = run_graph_tests(&snapshot, &program, None, policy, &control)
        .expect("graph-owned tests agree in both execution tiers");
    assert_eq!(tests.revision, None);
    assert_eq!(tests.passed, 1);
    assert_eq!(tests.failed, 0);
    assert!(tests.production_instructions > 0);
    assert!(tests.reference_expressions > 0);
    assert_eq!(tests.differential, "equal");
}

#[test]
fn normalized_reference_runner_uses_revision_pinned_owner_reads() {
    let snapshot = pure_command_snapshot();
    let (_temporary, repository, program) = prepare_repository(&snapshot);
    let view = repository
        .view_current()
        .expect("exact normalized runner repository view");
    let control = ExecutionControl::uncancelled();
    let receipt = run_pure_command(
        &view,
        &program,
        &Name::new("pure").unwrap(),
        b"[]",
        NormalizedCommandPolicy::default(),
        &control,
    )
    .expect("revision-pinned pure command");

    assert_eq!(receipt.revision, Some(view.revision()));
    assert_eq!(receipt.result_json, b"null");
    assert_eq!(receipt.reference.canonical_owner_reads, 7);
    assert!(receipt.reference.canonical_owner_reads < snapshot.owners.len() as u64);
    assert!(receipt.reference.canonical_map_pages_read > 0);
    assert_eq!(
        receipt.reference.canonical_objects_read,
        receipt
            .reference
            .canonical_map_pages_read
            .saturating_add(receipt.reference.canonical_owner_reads),
        "each exact semantic read touches one bounded map path and one owner object"
    );
    assert!(receipt.reference.canonical_bytes_read > 0);

    let tests = run_graph_tests(
        &view,
        &program,
        None,
        NormalizedRunPolicy::default(),
        &control,
    )
    .expect("revision-pinned graph-owned tests");
    assert_eq!(tests.revision, Some(view.revision()));
    assert_eq!(tests.passed, 1);

    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let effectful = run_effectful_command(
        &view,
        &program,
        &Name::new("command").unwrap(),
        b"[]",
        &capabilities,
        NormalizedCommandPolicy::default(),
        &control,
    )
    .expect("effectful command runs once through production");
    assert_eq!(effectful.revision, Some(view.revision()));
    assert_eq!(effectful.result_json, b"null");
    assert_eq!(effectful.production.capability_calls, 1);
    assert_eq!(effectful.verification, "production_only_live_effects");
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    let pure_target = program
        .root_target(&Name::new("pure").unwrap())
        .expect("pure fixture target");
    let pure_capabilities =
        NormalizedCapabilities::bind(&program, pure_target.component, Vec::new())
            .expect("empty grants bind the pure component");
    assert_eq!(
        run_effectful_command(
            &view,
            &program,
            &Name::new("command").unwrap(),
            b"[]",
            &pure_capabilities,
            NormalizedCommandPolicy::default(),
            &control,
        )
        .expect_err("grants for another component must reject")
        .code,
        "normalized_runner_grant_component"
    );
    assert_eq!(
        run_effectful_command(
            &view,
            &program,
            &Name::new("pure").unwrap(),
            b"[]",
            &pure_capabilities,
            NormalizedCommandPolicy::default(),
            &control,
        )
        .expect_err("pure targets require differential execution")
        .code,
        "normalized_runner_pure_target"
    );

    let (stale_capabilities, stale_calls) = bind_fixture_capability(&program, 1);
    assert_eq!(
        run_effectful_command(
            &WrongRevisionReader(&view),
            &program,
            &Name::new("command").unwrap(),
            b"[]",
            &stale_capabilities,
            NormalizedCommandPolicy::default(),
            &control,
        )
        .expect_err("foreign revision binding must reject before production effects")
        .code,
        "normalized_reference_authority_binding"
    );
    assert_eq!(stale_calls.load(Ordering::Relaxed), 0);
}

#[test]
fn dense_vm_executes_pure_external_test_and_capability_paths() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let program = prepare_snapshot(&snapshot);
    let vm = NormalizedVm::new(&program, NormalizedRunPolicy::default());
    let control = ExecutionControl::uncancelled();

    let (pure, pure_observation) = vm
        .invoke(
            declaration_named(&snapshot, "with_binding"),
            Vec::new(),
            None,
            &control,
        )
        .expect("pure dense invocation");
    assert_eq!(pure, NormalizedValue::Unit);
    assert!(pure_observation.instructions > 0);
    assert_eq!(pure_observation.capability_calls, 0);

    let (external, external_observation) = vm
        .invoke(
            declaration_named(&snapshot, "identity_external"),
            Vec::new(),
            None,
            &control,
        )
        .expect("direct external dense invocation");
    assert_eq!(external, NormalizedValue::Unit);
    assert_eq!(external_observation.external_calls, 1);

    let (actual, expected) = vm
        .invoke_test(declaration_named(&snapshot, "caller_test"), None, &control)
        .expect("graph-owned dense test");
    assert_eq!(actual.0, NormalizedValue::Unit);
    assert_eq!(actual.0, expected.0);

    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let (result, observation) = vm
        .invoke_root_target(
            &Name::new("command").unwrap(),
            Vec::new(),
            Some(&capabilities),
            &control,
        )
        .expect("effectful dense target");
    assert_eq!(result, NormalizedValue::Unit);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(observation.capability_calls, 1);
    assert_eq!(observation.calls, 2);
    assert!(observation.collection_items >= 2);
    assert_eq!(observation.production_tier, "graph5_dense_bytecode_1");
}

#[test]
fn dense_vm_enforces_exact_grants_cancellation_and_separate_budgets() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let program = prepare_snapshot(&snapshot);
    let target_name = Name::new("command").unwrap();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::new(&program, NormalizedRunPolicy::default());

    let missing = vm
        .invoke_root_target(&target_name, Vec::new(), None, &control)
        .expect_err("task invocation requires an exact deployment grant");
    assert_eq!(missing.class, ExecutionFailureClass::Capability);
    assert_eq!(missing.code, "normalized_capability_unbound");

    let target = program.root_target(&target_name).expect("fixture target");
    let requirement = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement.0 as usize];
    let invalid = NormalizedCapabilities::bind(
        &program,
        target.component,
        vec![NormalizedCapabilityGrant {
            requirement: requirement.reference,
            operations: Default::default(),
            maximum_calls: 1,
            adapter: Arc::new(UnitAdapter {
                interface: requirement.interface,
                calls: Arc::new(AtomicU64::new(0)),
            }),
        }],
    )
    .expect_err("partial operation grants are forbidden");
    assert_eq!(invalid.class, ExecutionFailureClass::Capability);
    assert_eq!(invalid.code, "normalized_grant_operation_set");

    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    let cancelled_error = vm
        .invoke(
            declaration_named(&snapshot, "identity_external"),
            Vec::new(),
            None,
            &cancelled,
        )
        .expect_err("cancellation reaches normalized external execution");
    assert_eq!(cancelled_error.class, ExecutionFailureClass::Cancelled);

    let step_limited = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            instruction_steps: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(
        declaration_named(&snapshot, "with_binding"),
        Vec::new(),
        None,
        &control,
    )
    .expect_err("instruction budget is independent");
    assert_eq!(step_limited.code, "normalized_instruction_steps");

    let allocation_limited = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_allocated_bytes: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(
        declaration_named(&snapshot, "with_binding"),
        Vec::new(),
        None,
        &control,
    )
    .expect_err("allocation budget is independent");
    assert_eq!(allocation_limited.code, "normalized_allocation");

    let (capabilities, _) = bind_fixture_capability(&program, 1);
    let call_depth_limited = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_call_depth: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke_root_target(&target_name, Vec::new(), Some(&capabilities), &control)
    .expect_err("call-depth budget is independent");
    assert_eq!(call_depth_limited.code, "normalized_call_depth");
}

#[test]
fn canonical_reference_and_dense_vm_agree_on_fixture_execution() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let program = prepare_snapshot(&snapshot);
    let policy = NormalizedRunPolicy::default();
    let vm = NormalizedVm::new(&program, policy);
    let reference = NormalizedReferenceInterpreter::new(&snapshot, &program, policy);
    let control = ExecutionControl::uncancelled();

    let pure = declaration_named(&snapshot, "with_binding");
    let vm_pure = vm
        .invoke(pure, Vec::new(), None, &control)
        .expect("dense pure execution");
    let reference_pure = reference
        .invoke(pure, Vec::new(), None, &control)
        .expect("canonical pure execution");
    assert_eq!(vm_pure.0, reference_pure.0);
    assert_eq!(
        reference_pure.1.production_tier,
        "graph5_reference_records_1"
    );

    let test = declaration_named(&snapshot, "caller_test");
    let vm_test = vm
        .invoke_test(test, None, &control)
        .expect("dense test execution");
    let reference_test = reference
        .invoke_test(test, None, &control)
        .expect("canonical test execution");
    assert_eq!(vm_test.0.0, reference_test.0.0);
    assert_eq!(vm_test.1.0, reference_test.1.0);

    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let target = Name::new("command").unwrap();
    let vm_task = vm
        .invoke_root_target(&target, Vec::new(), Some(&capabilities), &control)
        .expect("dense task execution");
    let reference_task = reference
        .invoke_root_target(&target, Vec::new(), Some(&capabilities), &control)
        .expect("canonical task execution");
    assert_eq!(vm_task.0, reference_task.0);
    assert_eq!(vm_task.1.capability_calls, 1);
    assert_eq!(reference_task.1.capability_calls, 1);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn every_graph5_expression_form_executes_equally_in_both_tiers() {
    let snapshot = transaction_result_snapshot();
    let program = prepare_snapshot(&snapshot);
    let policy = NormalizedRunPolicy::default();
    let vm = NormalizedVm::new(&program, policy);
    let reference = NormalizedReferenceInterpreter::new(&snapshot, &program, policy);
    let control = ExecutionControl::uncancelled();
    let caller = declaration_named(&snapshot, "caller");
    let (capabilities, calls) = bind_fixture_capability(&program, 2);

    let vm_result = vm
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("all-form dense execution");
    let reference_result = reference
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("all-form canonical execution");

    assert_eq!(vm_result.0, NormalizedValue::Unit);
    assert_eq!(vm_result.0, reference_result.0);
    assert_eq!(vm_result.1.capability_calls, 2);
    assert_eq!(reference_result.1.capability_calls, 2);
    assert!(vm_result.1.collection_items >= 4);
    assert!(reference_result.1.collection_items >= 4);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn both_graph5_execution_tiers_commit_and_rollback_exact_transactions() {
    let snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let program = prepare_snapshot(&snapshot);
    let policy = NormalizedRunPolicy::default();
    let control = ExecutionControl::uncancelled();
    let caller = declaration_named(&snapshot, "caller");
    let (capabilities, stats) = bind_tracking_capability(&program, 2, false);

    NormalizedVm::new(&program, policy)
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("dense transaction commit");
    NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("reference transaction commit");
    assert_eq!(stats.begins.load(Ordering::Relaxed), 2);
    assert_eq!(stats.commits.load(Ordering::Relaxed), 2);
    assert_eq!(stats.rollbacks.load(Ordering::Relaxed), 0);

    let failing_snapshot = transaction_call_snapshot();
    let failing_program = prepare_snapshot(&failing_snapshot);
    let failing_caller = declaration_named(&failing_snapshot, "caller");
    let (failing_capabilities, failing_stats) = bind_tracking_capability(&failing_program, 3, true);
    let dense_error = NormalizedVm::new(&failing_program, policy)
        .invoke(
            failing_caller,
            Vec::new(),
            Some(&failing_capabilities),
            &control,
        )
        .expect_err("dense transaction failure");
    let reference_error =
        NormalizedReferenceInterpreter::new(&failing_snapshot, &failing_program, policy)
            .invoke(
                failing_caller,
                Vec::new(),
                Some(&failing_capabilities),
                &control,
            )
            .expect_err("reference transaction failure");
    assert_eq!(dense_error.class, ExecutionFailureClass::PossibleVisibility);
    assert_eq!(dense_error, reference_error);
    assert!(dense_error.possibly_visible);
    assert_eq!(failing_stats.begins.load(Ordering::Relaxed), 2);
    assert_eq!(failing_stats.calls.load(Ordering::Relaxed), 2);
    assert_eq!(failing_stats.commits.load(Ordering::Relaxed), 0);
    assert_eq!(failing_stats.rollbacks.load(Ordering::Relaxed), 2);
}

#[test]
fn dense_vm_reports_stack_collection_and_capability_budget_dimensions() {
    let snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let program = prepare_snapshot(&snapshot);
    let caller = declaration_named(&snapshot, "caller");
    let control = ExecutionControl::uncancelled();
    let (capabilities, _) = bind_fixture_capability(&program, 2);

    let stack = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_value_stack: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(caller, Vec::new(), Some(&capabilities), &control)
    .expect_err("value-stack budget is independent");
    assert_eq!(stack.code, "normalized_value_stack");

    let collection = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_collection_items: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(caller, Vec::new(), Some(&capabilities), &control)
    .expect_err("collection-item budget is independent");
    assert_eq!(collection.code, "normalized_collection_items");

    let capability = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_capability_calls: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(caller, Vec::new(), Some(&capabilities), &control)
    .expect_err("capability-call budget is independent");
    assert_eq!(capability.code, "normalized_capability_calls");
}

#[test]
fn declaration_rename_and_move_do_not_change_dense_runtime_dispatch() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let before = prepare_snapshot(&snapshot);
    let mut moved = snapshot.clone();
    let destination = moved
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Module(module), OwnerRecord::Module(record))
                if record.name.as_str() == "second" =>
            {
                Some(*module)
            }
            _ => None,
        })
        .expect("destination module");
    let callee = declaration_named(&moved, "callee").declaration;
    let OwnerRecord::Declaration(callee_record) = moved
        .owners
        .get_mut(&OwnerKey::Declaration(callee))
        .expect("callee declaration")
    else {
        panic!("callee owner kind")
    };
    callee_record.name = Name::new("renamed_callee").unwrap();
    callee_record.module = destination;
    let after = prepare_snapshot(&moved);

    assert_eq!(before.functions, after.functions);
    assert_eq!(before.records, after.records);
    assert_eq!(before.variants, after.variants);
    assert_eq!(before.requirements, after.requirements);
    assert_eq!(before.operations, after.operations);
    assert_eq!(before.components, after.components);
    assert_eq!(before.ports, after.ports);

    let control = ExecutionControl::uncancelled();
    let target = Name::new("command").unwrap();
    let (before_capabilities, _) = bind_fixture_capability(&before, 1);
    let (after_capabilities, _) = bind_fixture_capability(&after, 1);
    let before_result = NormalizedVm::new(&before, NormalizedRunPolicy::default())
        .invoke_root_target(&target, Vec::new(), Some(&before_capabilities), &control)
        .expect("base dense target");
    let after_result = NormalizedVm::new(&after, NormalizedRunPolicy::default())
        .invoke_root_target(&target, Vec::new(), Some(&after_capabilities), &control)
        .expect("renamed and moved dense target");
    assert_eq!(before_result, after_result);
}
