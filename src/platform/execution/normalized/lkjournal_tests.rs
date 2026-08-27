//! Maintained `lkjournal` policy tests over the checked-in artifact-10 bundle.

use super::byte_stream::{NormalizedByteStreamAdapter, NormalizedByteStreamOperation};
use super::capability::{
    NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter,
    NormalizedCapabilityGrant, NormalizedCapabilityGrantDescriptor,
    NormalizedCapabilityTransaction, NormalizedGrantLimit, NormalizedTransactionPolicy,
};
use super::configuration::NormalizedConfigurationAdapter;
use super::deployment::{NormalizedDeploymentResourcePolicy, NormalizedPreparedDeployment};
use super::http::NormalizedHttpApplication;
use super::object::NormalizedObjectStorageAdapter;
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resident::NormalizedResidentDeployment;
use super::resource::NormalizedResourceScope;
use super::security::NormalizedSecurityAdapter;
use super::value::{NormalizedRecord, NormalizedValue, VariantLayoutIndex};
use super::vm::NormalizedRunPolicy;
use super::worker::NormalizedWorkerApplication;
use crate::platform::compiler::load_artifact;
use crate::platform::configuration::{ConfigurationOperation, ConfigurationValue};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{Name, OperationReference, ResourceUnit, TypeForm};
use crate::platform::object::{ObjectEngine, ObjectLimits};
use crate::platform::runtime::ResidentLimits;
use crate::platform::stream::StreamLimits;
use crate::platform::{HttpHeader, HttpLimits, HttpRequest, WorkerLimits};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

const LKJOURNAL_ARTIFACT: &[u8] =
    include_bytes!("../../../../applications/lkjournal/generated/lkjournal.lkja");

fn program() -> Arc<NormalizedProgram> {
    static PROGRAM: OnceLock<Arc<NormalizedProgram>> = OnceLock::new();
    Arc::clone(PROGRAM.get_or_init(|| {
        let loaded = load_artifact(LKJOURNAL_ARTIFACT).expect("strict maintained artifact-10");
        Arc::new(NormalizedProgram::prepare(loaded).expect("prepared maintained artifact-10"))
    }))
}

fn target_requirement<'a>(
    program: &'a NormalizedProgram,
    target: &str,
    alias: &str,
) -> &'a NormalizedRequirement {
    let target = program
        .root_target(&Name::new(target).expect("target name"))
        .expect("maintained target");
    let component = &program.components[target.component.0 as usize];
    component
        .requirements
        .iter()
        .map(|index| &program.requirements[index.0 as usize])
        .find(|requirement| requirement.name.as_str() == alias)
        .expect("maintained requirement alias")
}

fn requirement_operation<'a>(
    program: &'a NormalizedProgram,
    requirement: &NormalizedRequirement,
    name: &str,
) -> &'a NormalizedOperation {
    requirement
        .operations
        .iter()
        .map(|index| &program.operations[index.0 as usize])
        .find(|operation| operation.name.as_str() == name)
        .expect("maintained requirement operation")
}

fn exact_operations(
    program: &NormalizedProgram,
    requirement: &NormalizedRequirement,
) -> BTreeSet<OperationReference> {
    requirement
        .operations
        .iter()
        .map(|index| program.operations[index.0 as usize].reference)
        .collect()
}

fn exact_limits(requirement: &NormalizedRequirement) -> BTreeMap<Name, NormalizedGrantLimit> {
    let mut limits = requirement
        .limits
        .iter()
        .map(|limit| {
            (
                limit.name.clone(),
                NormalizedGrantLimit {
                    maximum: limit.maximum,
                    unit: limit.unit,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    limits
        .entry(Name::new("maximum_calls").expect("call limit name"))
        .or_insert(NormalizedGrantLimit {
            maximum: 10_000,
            unit: ResourceUnit::Calls,
        });
    limits
}

#[derive(Debug)]
struct ScriptedCall {
    operation: OperationReference,
    display_name: &'static str,
    result: Result<NormalizedValue, ExecutionError>,
}

#[derive(Clone)]
struct ScriptedNormalizedAdapter {
    kind: NormalizedAdapterKind,
    interface: crate::platform::kernel::DeclarationReference,
    operations: BTreeSet<OperationReference>,
    calls: Arc<Mutex<VecDeque<ScriptedCall>>>,
    transactions: Arc<Mutex<VecDeque<VecDeque<ScriptedCall>>>>,
    observed: Arc<Mutex<Vec<String>>>,
}

impl ScriptedNormalizedAdapter {
    fn empty(
        kind: NormalizedAdapterKind,
        interface: crate::platform::kernel::DeclarationReference,
        operations: BTreeSet<OperationReference>,
    ) -> Self {
        Self::new(kind, interface, operations, Vec::new(), Vec::new())
    }

    fn new(
        kind: NormalizedAdapterKind,
        interface: crate::platform::kernel::DeclarationReference,
        operations: BTreeSet<OperationReference>,
        calls: Vec<ScriptedCall>,
        transactions: Vec<Vec<ScriptedCall>>,
    ) -> Self {
        Self {
            kind,
            interface,
            operations,
            calls: Arc::new(Mutex::new(calls.into())),
            transactions: Arc::new(Mutex::new(
                transactions.into_iter().map(VecDeque::from).collect(),
            )),
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn observed(&self) -> Vec<String> {
        self.observed
            .lock()
            .expect("script observation lock")
            .clone()
    }

    fn next_call(&self, policy: &NormalizedCallPolicy) -> Result<NormalizedValue, ExecutionError> {
        let Some(script) = self.calls.lock().expect("script call lock").pop_front() else {
            return Err(script_error(
                "normalized_script_call_missing",
                "deterministic adapter observed an unexpected capability call",
            ));
        };
        if script.operation != policy.operation {
            return Err(script_error(
                "normalized_script_operation",
                "deterministic adapter observed a foreign exact operation",
            ));
        }
        self.observed
            .lock()
            .expect("script observation lock")
            .push(script.display_name.to_owned());
        script.result
    }
}

impl NormalizedCapabilityAdapter for ScriptedNormalizedAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        self.kind
    }

    fn interface(&self) -> crate::platform::kernel::DeclarationReference {
        self.interface
    }

    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.operations
    }

    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        _arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.next_call(policy)
    }

    fn begin_transaction(
        &self,
        _policy: &NormalizedTransactionPolicy,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        let Some(calls) = self
            .transactions
            .lock()
            .expect("transaction script lock")
            .pop_front()
        else {
            return Err(script_error(
                "normalized_script_transaction_missing",
                "deterministic adapter observed an unexpected transaction",
            ));
        };
        self.observed
            .lock()
            .expect("script observation lock")
            .push("transaction.begin".to_owned());
        Ok(Box::new(ScriptedNormalizedTransaction {
            calls,
            observed: Arc::clone(&self.observed),
            finished: false,
        }))
    }
}

struct ScriptedNormalizedTransaction {
    calls: VecDeque<ScriptedCall>,
    observed: Arc<Mutex<Vec<String>>>,
    finished: bool,
}

impl NormalizedCapabilityTransaction for ScriptedNormalizedTransaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        _arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        let Some(script) = self.calls.pop_front() else {
            return Err(script_error(
                "normalized_script_transaction_call_missing",
                "deterministic transaction observed an unexpected call",
            ));
        };
        if script.operation != policy.operation {
            return Err(script_error(
                "normalized_script_transaction_operation",
                "deterministic transaction observed a foreign exact operation",
            ));
        }
        self.observed
            .lock()
            .expect("script observation lock")
            .push(format!("transaction.{}", script.display_name));
        script.result
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()?;
        if !self.calls.is_empty() || self.finished {
            return Err(script_error(
                "normalized_script_transaction_commit",
                "deterministic transaction committed in a foreign state",
            ));
        }
        self.finished = true;
        self.observed
            .lock()
            .expect("script observation lock")
            .push("transaction.commit".to_owned());
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        if !self.finished {
            self.finished = true;
            self.observed
                .lock()
                .expect("script observation lock")
                .push("transaction.rollback".to_owned());
        }
        Ok(())
    }
}

fn script_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn adapter_kind(alias: &str) -> NormalizedAdapterKind {
    match alias {
        "bootstrap" => NormalizedAdapterKind::SecretVerifier,
        "clock" => NormalizedAdapterKind::WallClock,
        "config" => NormalizedAdapterKind::Configuration,
        "db" => NormalizedAdapterKind::Postgres,
        "identifiers" => NormalizedAdapterKind::Identifier,
        "jobs" => NormalizedAdapterKind::DurableQueueMemory,
        "objects" => NormalizedAdapterKind::ObjectMemory,
        "passwords" => NormalizedAdapterKind::PasswordHash,
        "random" => NormalizedAdapterKind::SecureRandom,
        "streams" => NormalizedAdapterKind::ByteStream,
        _ => panic!("unknown maintained requirement alias {alias}"),
    }
}

fn exact_bindings(
    program: &NormalizedProgram,
    target: &str,
    mut adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>>,
) -> Vec<NormalizedCapabilityGrant> {
    let target = program
        .root_target(&Name::new(target).expect("target name"))
        .expect("maintained target");
    program.components[target.component.0 as usize]
        .requirements
        .iter()
        .map(|index| &program.requirements[index.0 as usize])
        .map(|requirement| {
            let operations = exact_operations(program, requirement);
            let adapter = adapters
                .remove(requirement.name.as_str())
                .unwrap_or_else(|| {
                    Arc::new(ScriptedNormalizedAdapter::empty(
                        adapter_kind(requirement.name.as_str()),
                        requirement.interface,
                        operations.clone(),
                    ))
                });
            NormalizedCapabilityGrant {
                requirement: requirement.reference,
                descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                    requirement.interface,
                    adapter.kind(),
                    operations,
                    exact_limits(requirement),
                ),
                adapter,
            }
        })
        .collect()
}

fn stream_adapter(
    program: &NormalizedProgram,
    target: &str,
) -> Arc<dyn NormalizedCapabilityAdapter> {
    let requirement = target_requirement(program, target, "streams");
    let operation = requirement_operation(program, requirement, "read-all");
    Arc::new(
        NormalizedByteStreamAdapter::new_selected(
            requirement.reference,
            requirement.interface,
            BTreeMap::from([(operation.reference, NormalizedByteStreamOperation::ReadAll)]),
        )
        .expect("maintained exact stream adapter"),
    )
}

fn configuration_adapter(
    program: &NormalizedProgram,
    values: BTreeMap<String, ConfigurationValue>,
) -> Arc<dyn NormalizedCapabilityAdapter> {
    let requirement = target_requirement(program, "serve", "config");
    let operation = requirement_operation(program, requirement, "text");
    Arc::new(
        NormalizedConfigurationAdapter::new_selected(
            requirement.interface,
            BTreeMap::from([(operation.reference, ConfigurationOperation::Text)]),
            values,
        )
        .expect("maintained exact configuration adapter"),
    )
}

fn deterministic_clock(
    program: &NormalizedProgram,
    target: &str,
    values: Vec<i64>,
) -> Arc<dyn NormalizedCapabilityAdapter> {
    let requirement = target_requirement(program, target, "clock");
    let operation = requirement_operation(program, requirement, "utc-milliseconds");
    Arc::new(
        NormalizedSecurityAdapter::deterministic_clock(
            requirement.interface,
            operation.reference,
            values,
        )
        .expect("maintained exact clock adapter"),
    )
}

fn deterministic_identifier(
    program: &NormalizedProgram,
    values: Vec<[u8; 16]>,
) -> Arc<dyn NormalizedCapabilityAdapter> {
    let requirement = target_requirement(program, "serve", "identifiers");
    let operation = requirement_operation(program, requirement, "uuid-v4");
    Arc::new(
        NormalizedSecurityAdapter::deterministic_identifier(
            requirement.interface,
            operation.reference,
            values,
        )
        .expect("maintained exact identifier adapter"),
    )
}

fn deterministic_random(
    program: &NormalizedProgram,
    values: Vec<Vec<u8>>,
) -> Arc<dyn NormalizedCapabilityAdapter> {
    let requirement = target_requirement(program, "serve", "random");
    let operation = requirement_operation(program, requirement, "bytes");
    Arc::new(
        NormalizedSecurityAdapter::deterministic_random(
            requirement.interface,
            operation.reference,
            values,
        )
        .expect("maintained exact random adapter"),
    )
}

fn application(
    program: Arc<NormalizedProgram>,
    mut adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>>,
    maximum_chunk_bytes: usize,
) -> NormalizedHttpApplication {
    adapters
        .entry("streams".to_owned())
        .or_insert_with(|| stream_adapter(&program, "serve"));
    let deployment = NormalizedPreparedDeployment::prepare_exact_for_test(
        &program,
        Name::new("serve").expect("serve target"),
        exact_bindings(&program, "serve", adapters),
        NormalizedDeploymentResourcePolicy {
            streams: StreamLimits {
                maximum_chunk_bytes,
                ..StreamLimits::default()
            },
        },
    )
    .expect("exact normalized lkjournal deployment");
    let resident = NormalizedResidentDeployment::prepare(
        Arc::clone(&program),
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::default(),
    )
    .expect("normalized lkjournal resident");
    NormalizedHttpApplication::new(resident, HttpLimits::default())
        .expect("normalized lkjournal HTTP application")
}

fn request(method: &str, path: &str, query: &str, body: &[u8]) -> HttpRequest {
    HttpRequest {
        method: method.to_owned(),
        path: path.to_owned(),
        query: query.to_owned(),
        headers: Vec::new(),
        body: body.to_vec(),
    }
}

fn authenticated_request(method: &str, path: &str, query: &str, body: &[u8]) -> HttpRequest {
    let mut request = request(method, path, query, body);
    request.headers.push(HttpHeader {
        name: "authorization".to_owned(),
        value: b"Bearer session".to_vec(),
    });
    request
}

fn sql_value(
    program: &NormalizedProgram,
    case_name: &str,
    payload: NormalizedValue,
) -> NormalizedValue {
    let requirement = target_requirement(program, "serve", "db");
    let operation = requirement_operation(program, requirement, "query");
    let list = &program.types[&operation.parameters[1].ty];
    let TypeForm::List { item } = list.form else {
        panic!("maintained database parameter list")
    };
    let TypeForm::Named { declaration } = program.types[&item].form else {
        panic!("maintained SqlValue nominal type")
    };
    let (layout, variant) = program
        .variants
        .iter()
        .enumerate()
        .find(|(_, variant)| variant.declaration == declaration)
        .expect("maintained SqlValue layout");
    let case = variant
        .cases
        .iter()
        .position(|case| case.name.as_str() == case_name)
        .expect("maintained SqlValue case");
    NormalizedValue::Variant {
        layout: VariantLayoutIndex(layout as u32),
        case: case as u32,
        payload: Some(Box::new(payload)),
    }
}

fn sql_text(program: &NormalizedProgram, value: &str) -> NormalizedValue {
    sql_value(program, "Text", NormalizedValue::text(value))
}

fn sql_i64(program: &NormalizedProgram, value: i64) -> NormalizedValue {
    sql_value(program, "I64", NormalizedValue::I64(value))
}

fn sql_bool(program: &NormalizedProgram, value: bool) -> NormalizedValue {
    sql_value(program, "Bool", NormalizedValue::Bool(value))
}

fn rows(rows: Vec<Vec<NormalizedValue>>) -> NormalizedValue {
    NormalizedValue::List(Arc::new(
        rows.into_iter()
            .map(|row| NormalizedValue::List(Arc::new(row)))
            .collect(),
    ))
}

fn scripted_call(
    program: &NormalizedProgram,
    target: &str,
    alias: &str,
    operation: &'static str,
    result: NormalizedValue,
) -> ScriptedCall {
    let requirement = target_requirement(program, target, alias);
    ScriptedCall {
        operation: requirement_operation(program, requirement, operation).reference,
        display_name: operation,
        result: Ok(result),
    }
}

fn scripted_adapter(
    program: &NormalizedProgram,
    target: &str,
    alias: &str,
    calls: Vec<ScriptedCall>,
    transactions: Vec<Vec<ScriptedCall>>,
) -> ScriptedNormalizedAdapter {
    let requirement = target_requirement(program, target, alias);
    ScriptedNormalizedAdapter::new(
        adapter_kind(alias),
        requirement.interface,
        exact_operations(program, requirement),
        calls,
        transactions,
    )
}

fn worker_lease(program: &NormalizedProgram) -> NormalizedValue {
    let requirement = target_requirement(program, "work", "jobs");
    let operation = requirement_operation(program, requirement, "claim");
    let TypeForm::List { item } = program.types[&operation.result].form else {
        panic!("maintained queue claim list")
    };
    let TypeForm::StructuralRecord { fields } = &program.types[&item].form else {
        panic!("maintained queue lease structure")
    };
    let mut values = fields
        .iter()
        .map(|field| {
            let value = match field.name.as_str() {
                "attempt_id" => NormalizedValue::text("attempt-1"),
                "attempt_number" => NormalizedValue::I64(1),
                "job_id" => NormalizedValue::text("job-1"),
                "lease_until_milliseconds" => NormalizedValue::I64(1_100),
                "payload" => NormalizedValue::bytes(b"resource-1".to_vec()),
                _ => panic!("unknown maintained lease field"),
            };
            (field.name.clone(), value)
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.0.cmp(&right.0));
    NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(values),
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_home_escapes_configuration_and_health_is_independent() {
    let program = program();
    let config = configuration_adapter(
        &program,
        BTreeMap::from([
            (
                "service-title".to_owned(),
                ConfigurationValue::Text("<script>alert(1)</script>".to_owned()),
            ),
            (
                "initial-actor".to_owned(),
                ConfigurationValue::Text("alice".to_owned()),
            ),
        ]),
    );
    let application = application(
        Arc::clone(&program),
        BTreeMap::from([("config".to_owned(), config)]),
        StreamLimits::default().maximum_chunk_bytes,
    );

    let (home, _) = application
        .dispatch(request("GET", "/", "", b""))
        .await
        .expect("artifact-10 home response");
    assert_eq!(home.status, 200);
    let body = String::from_utf8(home.body).expect("home HTML UTF-8");
    assert!(body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!body.contains("<script>alert(1)</script>"));

    let (health, _) = application
        .dispatch(request("GET", "/health", "", b""))
        .await
        .expect("artifact-10 health response");
    assert_eq!(health.body, b"ready");
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_bootstrap_denial_precedes_password_body_and_database_work() {
    let program = program();
    let bootstrap = Arc::new(scripted_adapter(
        &program,
        "serve",
        "bootstrap",
        vec![scripted_call(
            &program,
            "serve",
            "bootstrap",
            "matches",
            NormalizedValue::Bool(false),
        )],
        Vec::new(),
    ));
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        Vec::new(),
        Vec::new(),
    ));
    let config = configuration_adapter(
        &program,
        BTreeMap::from([
            (
                "initial-actor".to_owned(),
                ConfigurationValue::Text("operator".to_owned()),
            ),
            (
                "service-title".to_owned(),
                ConfigurationValue::Text("lkjournal".to_owned()),
            ),
        ]),
    );
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "bootstrap".to_owned(),
            bootstrap.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
        ("config".to_owned(), config),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
    ]);
    let application = application(
        Arc::clone(&program),
        adapters,
        StreamLimits::default().maximum_chunk_bytes,
    );

    let (response, _) = application
        .dispatch(authenticated_request(
            "POST",
            "/initialize",
            "actor=operator",
            &[b'x'; 4096],
        ))
        .await
        .expect("artifact-10 typed bootstrap denial");
    assert_eq!(response.status, 403);
    assert_eq!(bootstrap.observed(), vec!["matches"]);
    assert!(database.observed().is_empty());
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_create_route_owns_auth_transaction_and_job_policy() {
    let program = program();
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        vec![scripted_call(
            &program,
            "serve",
            "db",
            "query",
            rows(vec![vec![sql_text(&program, "alice")]]),
        )],
        vec![vec![
            scripted_call(&program, "serve", "db", "execute", NormalizedValue::I64(1)),
            scripted_call(&program, "serve", "db", "execute", NormalizedValue::I64(1)),
        ]],
    ));
    let jobs = Arc::new(scripted_adapter(
        &program,
        "serve",
        "jobs",
        vec![scripted_call(
            &program,
            "serve",
            "jobs",
            "enqueue",
            NormalizedValue::Bool(true),
        )],
        Vec::new(),
    ));
    let mut identifier = [0u8; 16];
    identifier[15] = 1;
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "serve", vec![1_000]),
        ),
        ("db".to_owned(), database.clone()),
        (
            "identifiers".to_owned(),
            deterministic_identifier(&program, vec![identifier]),
        ),
        ("jobs".to_owned(), jobs.clone()),
    ]);
    let application = application(Arc::clone(&program), adapters, 4);

    let (response, _) = application
        .dispatch(authenticated_request(
            "POST",
            "/resources",
            "",
            br##"{"title":"First entry","body":"# Hello\nThis is maintained meaning."}"##,
        ))
        .await
        .expect("artifact-10 create route");
    assert_eq!(response.status, 201);
    let body = String::from_utf8(response.body).expect("create response JSON UTF-8");
    assert!(body.contains("00000000-0000-4000-8000-000000000001"));
    assert_eq!(
        database.observed(),
        vec![
            "query",
            "transaction.begin",
            "transaction.execute",
            "transaction.execute",
            "transaction.commit",
        ]
    );
    assert_eq!(jobs.observed(), vec!["enqueue"]);
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_application_owner_check_denies_cross_actor_read() {
    let program = program();
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        vec![
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![sql_text(&program, "alice")]]),
            ),
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![
                    sql_text(&program, "bob"),
                    sql_text(&program, "Private"),
                    sql_text(&program, "body"),
                    sql_i64(&program, 3),
                ]]),
            ),
        ],
        Vec::new(),
    ));
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "serve", vec![10]),
        ),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
    ]);
    let application = application(
        Arc::clone(&program),
        adapters,
        StreamLimits::default().maximum_chunk_bytes,
    );

    let (response, _) = application
        .dispatch(authenticated_request(
            "GET",
            "/resource",
            "id=resource-1",
            b"",
        ))
        .await
        .expect("artifact-10 typed owner denial");
    assert_eq!(response.status, 403);
    assert_eq!(response.body, br#"{"error":"resource_owner_denied"}"#);
    assert_eq!(database.observed(), vec!["query", "query"]);
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_list_route_projects_bounded_typed_rows() {
    let program = program();
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        vec![
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![sql_text(&program, "alice")]]),
            ),
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![
                    vec![
                        sql_text(&program, "resource-2"),
                        sql_text(&program, "Second"),
                        sql_i64(&program, 4),
                    ],
                    vec![
                        sql_text(&program, "resource-1"),
                        sql_text(&program, "First"),
                        sql_i64(&program, 0),
                    ],
                ]),
            ),
        ],
        Vec::new(),
    ));
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "serve", vec![10]),
        ),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
    ]);
    let application = application(
        Arc::clone(&program),
        adapters,
        StreamLimits::default().maximum_chunk_bytes,
    );

    let (response, _) = application
        .dispatch(authenticated_request("GET", "/resources", "", b""))
        .await
        .expect("artifact-10 list route");
    assert_eq!(response.status, 200);
    let value: serde_json::Value =
        serde_json::from_slice(&response.body).expect("strict list JSON");
    let items = value.as_array().expect("list response");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "resource-2");
    assert_eq!(items[0]["revision"], 4);
    assert_eq!(database.observed(), vec!["query", "query"]);
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_malformed_typed_json_precedes_transaction_work() {
    let program = program();
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        vec![scripted_call(
            &program,
            "serve",
            "db",
            "query",
            rows(vec![vec![sql_text(&program, "alice")]]),
        )],
        Vec::new(),
    ));
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "serve", vec![10]),
        ),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
    ]);
    let application = application(
        Arc::clone(&program),
        adapters,
        StreamLimits::default().maximum_chunk_bytes,
    );

    let (response, _) = application
        .dispatch(authenticated_request(
            "POST",
            "/resource/update",
            "id=resource-1",
            br#"{"title":"Title","body":"new body","base":"01"}"#,
        ))
        .await
        .expect("artifact-10 malformed update response");
    assert_eq!(response.status, 400);
    assert_eq!(response.body, br#"{"error":"json_invalid"}"#);
    assert_eq!(database.observed(), vec!["query"]);
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_login_uses_time_randomness_and_the_database_handler_path() {
    let program = program();
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        vec![
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![sql_text(&program, "encoded-password")]]),
            ),
            scripted_call(&program, "serve", "db", "execute", NormalizedValue::I64(1)),
        ],
        Vec::new(),
    ));
    let passwords = Arc::new(scripted_adapter(
        &program,
        "serve",
        "passwords",
        vec![scripted_call(
            &program,
            "serve",
            "passwords",
            "verify",
            NormalizedValue::Bool(true),
        )],
        Vec::new(),
    ));
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "serve", vec![2_000]),
        ),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
        (
            "passwords".to_owned(),
            passwords.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
        (
            "random".to_owned(),
            deterministic_random(&program, vec![vec![0xab; 32]]),
        ),
    ]);
    let application = application(
        Arc::clone(&program),
        adapters,
        StreamLimits::default().maximum_chunk_bytes,
    );

    let (response, _) = application
        .dispatch(request("POST", "/login", "actor=alice", b"password"))
        .await
        .expect("artifact-10 login");
    assert_eq!(response.status, 200);
    let body = String::from_utf8(response.body).expect("login JSON UTF-8");
    assert!(body.contains(&"ab".repeat(32)));
    assert!(body.contains("86402000"));
    assert_eq!(database.observed(), vec!["query", "execute"]);
    assert_eq!(passwords.observed(), vec!["verify"]);
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_object_route_streams_and_malformed_query_precedes_admission() {
    let program = program();
    let database = Arc::new(scripted_adapter(
        &program,
        "serve",
        "db",
        vec![
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![sql_text(&program, "alice")]]),
            ),
            scripted_call(&program, "serve", "db", "execute", NormalizedValue::I64(1)),
            scripted_call(&program, "serve", "db", "execute", NormalizedValue::I64(1)),
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![sql_text(&program, "alice")]]),
            ),
            scripted_call(
                &program,
                "serve",
                "db",
                "query",
                rows(vec![vec![sql_bool(&program, true)]]),
            ),
            scripted_call(&program, "serve", "db", "execute", NormalizedValue::I64(1)),
        ],
        Vec::new(),
    ));
    let object_requirement = target_requirement(&program, "serve", "objects");
    let stream_requirement = target_requirement(&program, "serve", "streams");
    let object = Arc::new(
        NormalizedObjectStorageAdapter::prepare(
            &program,
            object_requirement,
            NormalizedAdapterKind::ObjectMemory,
            &[stream_requirement.reference],
            ObjectEngine::in_memory(
                tokio::runtime::Handle::current(),
                String::new(),
                ObjectLimits::default(),
            )
            .expect("memory object engine"),
        )
        .expect("maintained normalized object adapter"),
    );
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "serve", vec![5_000, 5_001]),
        ),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
        (
            "objects".to_owned(),
            object as Arc<dyn NormalizedCapabilityAdapter>,
        ),
    ]);
    let application = application(
        Arc::clone(&program),
        adapters,
        StreamLimits::default().maximum_chunk_bytes,
    );

    let payload = vec![0x5a; 200_000];
    let (response, _) = application
        .dispatch(authenticated_request(
            "POST",
            "/objects",
            "name=attachment.bin",
            &payload,
        ))
        .await
        .expect("artifact-10 object publication");
    assert_eq!(response.status, 201);
    let (reconciled, _) = application
        .dispatch(authenticated_request(
            "POST",
            "/objects/reconcile",
            "name=attachment.bin",
            b"",
        ))
        .await
        .expect("artifact-10 object reconciliation");
    assert_eq!(reconciled.status, 200);
    assert_eq!(reconciled.body, br#"{"reconciled":true,"bytes":200000}"#);
    assert_eq!(
        database.observed(),
        vec!["query", "execute", "execute", "query", "query", "execute"]
    );
    let admitted = application.resident().observe().admitted;
    let error = application
        .dispatch(request("GET", "/health", "broken=%zz", b""))
        .await
        .expect_err("malformed query rejects before normalized admission");
    assert_eq!(error.code, "http_query_decode");
    assert_eq!(application.resident().observe().admitted, admitted);
    assert_eq!(application.resident().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn artifact10_worker_consumes_one_exact_queue_attempt() {
    let program = program();
    let jobs = Arc::new(scripted_adapter(
        &program,
        "work",
        "jobs",
        vec![
            scripted_call(
                &program,
                "work",
                "jobs",
                "claim",
                NormalizedValue::List(Arc::new(vec![worker_lease(&program)])),
            ),
            scripted_call(
                &program,
                "work",
                "jobs",
                "complete",
                NormalizedValue::Bool(true),
            ),
            scripted_call(
                &program,
                "work",
                "jobs",
                "claim",
                NormalizedValue::List(Arc::new(Vec::new())),
            ),
            scripted_call(
                &program,
                "work",
                "jobs",
                "claim",
                NormalizedValue::List(Arc::new(Vec::new())),
            ),
        ],
        Vec::new(),
    ));
    let adapters: BTreeMap<String, Arc<dyn NormalizedCapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            deterministic_clock(&program, "work", vec![100, 101, 102]),
        ),
        (
            "jobs".to_owned(),
            jobs.clone() as Arc<dyn NormalizedCapabilityAdapter>,
        ),
    ]);
    let deployment = NormalizedPreparedDeployment::prepare_exact_for_test(
        &program,
        Name::new("work").expect("work target"),
        exact_bindings(&program, "work", adapters),
        NormalizedDeploymentResourcePolicy::default(),
    )
    .expect("exact normalized worker deployment");
    let resident = NormalizedResidentDeployment::prepare(
        Arc::clone(&program),
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::default(),
    )
    .expect("normalized lkjournal worker resident");
    let worker = NormalizedWorkerApplication::new(
        resident,
        WorkerLimits {
            maximum_workers: 1,
            idle_wait_milliseconds: 1,
            ..WorkerLimits::default()
        },
    )
    .expect("normalized lkjournal worker");
    let observer = worker.resident().clone();
    let shutdown = async move {
        loop {
            if observer.observe().completed >= 3 {
                break;
            }
            tokio::task::yield_now().await;
        }
    };
    let receipt = tokio::time::timeout(std::time::Duration::from_secs(2), worker.run(shutdown))
        .await
        .expect("bounded artifact-10 worker run")
        .expect("artifact-10 worker topology");
    assert_eq!(receipt.productive_iterations, 1);
    assert!(receipt.idle_iterations >= 1);
    assert_eq!(jobs.observed(), vec!["claim", "complete", "claim", "claim"]);
    assert_eq!(receipt.shutdown.remaining_tasks, 0);
}
