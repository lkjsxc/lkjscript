#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use lkjscript::platform::execution::ExecutionControl;
use lkjscript::platform::language::{Idempotency, Visibility};
use lkjscript::platform::{
    BoundCapabilities, ByteStreamAdapter, CAPABILITY_GRANT_CONTRACT_VERSION, CallPolicy,
    CapabilityAdapter, CapabilityGrant, CapabilityGrantDescriptor, ConfigurationAdapter,
    ConfigurationValue, DeterministicClockAdapter, DeterministicIdentifierAdapter,
    DeterministicRandomAdapter, DurableQueueAdapter, HttpApplication, HttpHeader, HttpRequest,
    ObjectLimits, ObjectStorageAdapter, OwnerId, PreparedProgram, QueueLimits, ResidentDeployment,
    ResidentLimits, RunPolicy, ScriptedAdapter, ScriptedCall, StreamLimits, StreamRegistry, Value,
    WorkerApplication, WorkerLimits, load_artifact,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn program() -> Arc<PreparedProgram> {
    let bytes =
        include_bytes!("../applications/lkjournal/frozen-service/lkjournal-artifact-v4.lkja");
    Arc::new(
        PreparedProgram::prepare(load_artifact(bytes).expect("maintained service artifact"))
            .expect("prepared maintained service"),
    )
}

fn component<'a>(
    program: &'a PreparedProgram,
    target: &str,
) -> &'a lkjscript::platform::PreparedComponent {
    let target = program.target(target).expect("target");
    program
        .components()
        .get(&target.component)
        .expect("component")
}

fn grants(
    program: &PreparedProgram,
    target: &str,
    mut adapters: BTreeMap<String, Arc<dyn CapabilityAdapter>>,
) -> Vec<CapabilityGrant> {
    component(program, target)
        .requirements
        .iter()
        .map(|(alias, requirement)| {
            let adapter = adapters.remove(alias).unwrap_or_else(|| {
                Arc::new(ScriptedAdapter::new(requirement.interface.clone(), vec![]))
            });
            CapabilityGrant {
                requirement: alias.clone(),
                descriptor: CapabilityGrantDescriptor {
                    contract_version: CAPABILITY_GRANT_CONTRACT_VERSION,
                    interface: requirement.interface.clone(),
                    adapter_kind: "deterministic-fake".to_owned(),
                    sharing_domain: "service-test".to_owned(),
                    authority_revision: "a".repeat(64),
                    descriptor_digest: "b".repeat(64),
                    operations: requirement.operations.keys().cloned().collect(),
                    limits: requirement.limits.clone(),
                },
                adapter,
            }
        })
        .collect()
}

fn interface(program: &PreparedProgram, target: &str, alias: &str) -> OwnerId {
    component(program, target).requirements[alias]
        .interface
        .clone()
}

fn sql_value_owner(program: &PreparedProgram) -> OwnerId {
    program
        .resolve_name(
            &lkjscript::platform::PackageId::parse("10000000000000000000000000000001")
                .expect("standard package id"),
            "database",
            "SqlValue",
        )
        .expect("standard SqlValue owner")
}

fn sql_bool(program: &PreparedProgram, value: bool) -> Value {
    Value::variant(sql_value_owner(program), "Bool", Some(Value::Bool(value)))
}

fn sql_text(program: &PreparedProgram, value: &str) -> Value {
    Value::variant(sql_value_owner(program), "Text", Some(Value::text(value)))
}

fn sql_i64(program: &PreparedProgram, value: i64) -> Value {
    Value::variant(sql_value_owner(program), "I64", Some(Value::I64(value)))
}

fn rows(row: Vec<Value>) -> Value {
    Value::List(Arc::new(vec![Value::List(Arc::new(row))]))
}

fn request(method: &str, path: &str, query: &str, body: &[u8]) -> HttpRequest {
    HttpRequest {
        method: method.to_owned(),
        path: path.to_owned(),
        query: query.to_owned(),
        headers: vec![],
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

fn application(
    program: Arc<PreparedProgram>,
    streams: StreamRegistry,
    adapters: BTreeMap<String, Arc<dyn CapabilityAdapter>>,
) -> HttpApplication {
    let deployment = ResidentDeployment::prepare(
        program.clone(),
        "serve",
        grants(&program, "serve", adapters),
        ResidentLimits::default(),
        RunPolicy::default(),
    )
    .expect("resident deployment");
    HttpApplication::new(
        deployment,
        lkjscript::platform::HttpLimits::default(),
        streams,
    )
    .expect("HTTP application")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authored_home_escapes_configuration_and_health_is_independent() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let config = ConfigurationAdapter::new(
        interface(&program, "serve", "config"),
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
    )
    .expect("configuration");
    let app = application(
        program,
        streams,
        BTreeMap::from([(
            "config".to_owned(),
            Arc::new(config) as Arc<dyn CapabilityAdapter>,
        )]),
    );
    let (home, _) = app
        .dispatch(request("GET", "/", "", b""))
        .await
        .expect("home");
    assert_eq!(home.status, 200);
    let body = String::from_utf8(home.body).expect("HTML UTF-8");
    assert!(body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!body.contains("<script>alert(1)</script>"));
    let (health, _) = app
        .dispatch(request("GET", "/health", "", b""))
        .await
        .expect("health");
    assert_eq!(health.body, b"ready");
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bootstrap_denial_precedes_password_body_and_database_work() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let bootstrap = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "bootstrap"),
        vec![ScriptedCall {
            operation: "matches".to_owned(),
            result: Ok(Value::Bool(false)),
        }],
    ));
    let database = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "db"),
        vec![],
    ));
    let config = ConfigurationAdapter::new(
        interface(&program, "serve", "config"),
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
    )
    .expect("configuration");
    let adapters: BTreeMap<String, Arc<dyn CapabilityAdapter>> = BTreeMap::from([
        (
            "bootstrap".to_owned(),
            bootstrap.clone() as Arc<dyn CapabilityAdapter>,
        ),
        (
            "config".to_owned(),
            Arc::new(config) as Arc<dyn CapabilityAdapter>,
        ),
        (
            "db".to_owned(),
            database.clone() as Arc<dyn CapabilityAdapter>,
        ),
    ]);
    let app = application(program.clone(), streams, adapters);
    let (response, _) = app
        .dispatch(authenticated_request(
            "POST",
            "/initialize",
            "actor=operator",
            &[b'x'; 4096],
        ))
        .await
        .expect("typed bootstrap denial");
    assert_eq!(response.status, 403);
    assert_eq!(bootstrap.observed(), vec!["matches"]);
    assert!(database.observed().is_empty());
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authored_create_route_owns_auth_transaction_and_job_policy() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits {
        maximum_chunk_bytes: 4,
        ..StreamLimits::default()
    })
    .expect("streams");
    let database = Arc::new(ScriptedAdapter::with_transactions(
        interface(&program, "serve", "db"),
        vec![ScriptedCall {
            operation: "query".to_owned(),
            result: Ok(rows(vec![sql_text(&program, "alice")])),
        }],
        vec![vec![
            ScriptedCall {
                operation: "execute".to_owned(),
                result: Ok(Value::I64(1)),
            },
            ScriptedCall {
                operation: "execute".to_owned(),
                result: Ok(Value::I64(1)),
            },
        ]],
    ));
    let jobs = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "jobs"),
        vec![ScriptedCall {
            operation: "enqueue".to_owned(),
            result: Ok(Value::Bool(true)),
        }],
    ));
    let mut id = [0u8; 16];
    id[15] = 1;
    let adapters: BTreeMap<String, Arc<dyn CapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            Arc::new(DeterministicClockAdapter::new(
                interface(&program, "serve", "clock"),
                vec![1_000],
            )) as Arc<dyn CapabilityAdapter>,
        ),
        ("db".to_owned(), database.clone()),
        (
            "identifiers".to_owned(),
            Arc::new(DeterministicIdentifierAdapter::new(
                interface(&program, "serve", "identifiers"),
                vec![id],
            )),
        ),
        ("jobs".to_owned(), jobs.clone()),
        (
            "streams".to_owned(),
            Arc::new(ByteStreamAdapter::new(
                interface(&program, "serve", "streams"),
                streams.clone(),
            )),
        ),
    ]);
    let app = application(program, streams, adapters);
    let (response, _) = app
        .dispatch(authenticated_request(
            "POST",
            "/resources",
            "",
            br##"{"title":"First entry","body":"# Hello\nThis is maintained meaning."}"##,
        ))
        .await
        .expect("create route");
    assert_eq!(response.status, 201);
    let body = String::from_utf8(response.body).expect("JSON UTF-8");
    assert!(body.contains("00000000-0000-4000-8000-000000000001"));
    assert_eq!(
        database.observed(),
        vec![
            "query",
            "transaction.begin",
            "transaction.execute",
            "transaction.execute",
            "transaction.commit"
        ]
    );
    assert_eq!(jobs.observed(), vec!["enqueue"]);
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn application_owned_owner_check_denies_cross_actor_read() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let database = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "db"),
        vec![
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![sql_text(&program, "alice")])),
            },
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![
                    sql_text(&program, "bob"),
                    sql_text(&program, "Private"),
                    sql_text(&program, "body"),
                    sql_i64(&program, 3),
                ])),
            },
        ],
    ));
    let app = application(
        program.clone(),
        streams.clone(),
        BTreeMap::from([
            (
                "clock".to_owned(),
                Arc::new(DeterministicClockAdapter::new(
                    interface(&program, "serve", "clock"),
                    vec![10],
                )) as Arc<dyn CapabilityAdapter>,
            ),
            ("db".to_owned(), database),
        ]),
    );
    let (response, _) = app
        .dispatch(authenticated_request(
            "GET",
            "/resource",
            "id=resource-1",
            b"",
        ))
        .await
        .expect("typed denial response");
    assert_eq!(response.status, 403);
    assert_eq!(response.body, br#"{"error":"resource_owner_denied"}"#);
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authored_list_route_projects_bounded_typed_rows() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let database = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "db"),
        vec![
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![sql_text(&program, "alice")])),
            },
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(Value::List(Arc::new(vec![
                    Value::List(Arc::new(vec![
                        sql_text(&program, "resource-2"),
                        sql_text(&program, "Second"),
                        sql_i64(&program, 4),
                    ])),
                    Value::List(Arc::new(vec![
                        sql_text(&program, "resource-1"),
                        sql_text(&program, "First"),
                        sql_i64(&program, 0),
                    ])),
                ]))),
            },
        ],
    ));
    let app = application(
        program.clone(),
        streams,
        BTreeMap::from([
            (
                "clock".to_owned(),
                Arc::new(DeterministicClockAdapter::new(
                    interface(&program, "serve", "clock"),
                    vec![10],
                )) as Arc<dyn CapabilityAdapter>,
            ),
            ("db".to_owned(), database.clone()),
        ]),
    );
    let (response, _) = app
        .dispatch(authenticated_request("GET", "/resources", "", b""))
        .await
        .expect("list route");
    assert_eq!(response.status, 200);
    let value: serde_json::Value = serde_json::from_slice(&response.body).expect("strict JSON");
    let items = value.as_array().expect("list response");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "resource-2");
    assert_eq!(items[0]["revision"], 4);
    assert_eq!(database.observed(), vec!["query", "query"]);
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_typed_json_is_a_response_without_transaction_work() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let database = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "db"),
        vec![ScriptedCall {
            operation: "query".to_owned(),
            result: Ok(rows(vec![sql_text(&program, "alice")])),
        }],
    ));
    let app = application(
        program.clone(),
        streams.clone(),
        BTreeMap::from([
            (
                "clock".to_owned(),
                Arc::new(DeterministicClockAdapter::new(
                    interface(&program, "serve", "clock"),
                    vec![10],
                )) as Arc<dyn CapabilityAdapter>,
            ),
            ("db".to_owned(), database.clone()),
            (
                "streams".to_owned(),
                Arc::new(ByteStreamAdapter::new(
                    interface(&program, "serve", "streams"),
                    streams,
                )),
            ),
        ]),
    );
    let (response, _) = app
        .dispatch(authenticated_request(
            "POST",
            "/resource/update",
            "id=resource-1",
            br#"{"title":"Title","body":"new body","base":"01"}"#,
        ))
        .await
        .expect("typed malformed base response");
    assert_eq!(response.status, 400);
    assert_eq!(response.body, br#"{"error":"json_invalid"}"#);
    assert_eq!(database.observed(), vec!["query"]);
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_uses_deterministic_time_randomness_and_same_database_handler_path() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let database = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "db"),
        vec![
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![sql_text(&program, "encoded-password")])),
            },
            ScriptedCall {
                operation: "execute".to_owned(),
                result: Ok(Value::I64(1)),
            },
        ],
    ));
    let password = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "passwords"),
        vec![ScriptedCall {
            operation: "verify".to_owned(),
            result: Ok(Value::Bool(true)),
        }],
    ));
    let app = application(
        program.clone(),
        streams.clone(),
        BTreeMap::from([
            (
                "clock".to_owned(),
                Arc::new(DeterministicClockAdapter::new(
                    interface(&program, "serve", "clock"),
                    vec![2_000],
                )) as Arc<dyn CapabilityAdapter>,
            ),
            ("db".to_owned(), database),
            ("passwords".to_owned(), password),
            (
                "random".to_owned(),
                Arc::new(DeterministicRandomAdapter::new(
                    interface(&program, "serve", "random"),
                    vec![vec![0xab; 32]],
                )),
            ),
            (
                "streams".to_owned(),
                Arc::new(ByteStreamAdapter::new(
                    interface(&program, "serve", "streams"),
                    streams,
                )),
            ),
        ]),
    );
    let (response, _) = app
        .dispatch(request("POST", "/login", "actor=alice", b"password"))
        .await
        .expect("login");
    assert_eq!(response.status, 200);
    let body = String::from_utf8(response.body).expect("JSON UTF-8");
    assert!(body.contains(&"ab".repeat(32)));
    assert!(body.contains("86402000"));
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn object_route_streams_through_generic_store_and_malformed_query_precedes_admission() {
    let program = program();
    let streams = StreamRegistry::new(StreamLimits::default()).expect("streams");
    let database = Arc::new(ScriptedAdapter::new(
        interface(&program, "serve", "db"),
        vec![
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![sql_text(&program, "alice")])),
            },
            ScriptedCall {
                operation: "execute".to_owned(),
                result: Ok(Value::I64(1)),
            },
            ScriptedCall {
                operation: "execute".to_owned(),
                result: Ok(Value::I64(1)),
            },
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![sql_text(&program, "alice")])),
            },
            ScriptedCall {
                operation: "query".to_owned(),
                result: Ok(rows(vec![sql_bool(&program, true)])),
            },
            ScriptedCall {
                operation: "execute".to_owned(),
                result: Ok(Value::I64(1)),
            },
        ],
    ));
    let object = Arc::new(
        ObjectStorageAdapter::in_memory(
            interface(&program, "serve", "objects"),
            tokio::runtime::Handle::current(),
            streams.clone(),
            String::new(),
            ObjectLimits::default(),
        )
        .expect("object store"),
    );
    let app = application(
        program.clone(),
        streams,
        BTreeMap::from([
            (
                "clock".to_owned(),
                Arc::new(DeterministicClockAdapter::new(
                    interface(&program, "serve", "clock"),
                    vec![5_000, 5_001],
                )) as Arc<dyn CapabilityAdapter>,
            ),
            ("db".to_owned(), database.clone()),
            ("objects".to_owned(), object.clone()),
        ]),
    );
    let payload = vec![0x5a; 200_000];
    let (response, _) = app
        .dispatch(authenticated_request(
            "POST",
            "/objects",
            "name=attachment.bin",
            &payload,
        ))
        .await
        .expect("object publication");
    assert_eq!(response.status, 201);
    let (reconciled, _) = app
        .dispatch(authenticated_request(
            "POST",
            "/objects/reconcile",
            "name=attachment.bin",
            b"",
        ))
        .await
        .expect("object reconciliation");
    assert_eq!(reconciled.status, 200);
    assert_eq!(reconciled.body, br#"{"reconciled":true,"bytes":200000}"#);
    assert_eq!(
        database.observed(),
        vec!["query", "execute", "execute", "query", "query", "execute"]
    );
    let admitted = app.deployment().observe().admitted;
    let error = app
        .dispatch(request("GET", "/health", "broken=%zz", b""))
        .await
        .expect_err("malformed query must reject");
    assert_eq!(error.code, "http_query_decode");
    assert_eq!(app.deployment().observe().admitted, admitted);
    assert_eq!(app.deployment().shutdown().await.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn maintained_worker_consumes_one_exact_queue_attempt() {
    let program = program();
    let queue = Arc::new(
        DurableQueueAdapter::in_memory(interface(&program, "work", "jobs"), QueueLimits::default())
            .expect("queue"),
    );
    let enqueue_policy = CallPolicy {
        requirement: "jobs".to_owned(),
        interface: interface(&program, "work", "jobs"),
        operation: "enqueue".to_owned(),
        idempotency: Idempotency::IdempotentWithKey,
        visibility: Visibility::Possible,
        limits: BTreeMap::new(),
        control: ExecutionControl::uncancelled(),
    };
    queue
        .call(
            &enqueue_policy,
            vec![
                Value::text("job-1"),
                Value::text("key-1"),
                Value::bytes(b"resource-1".to_vec()),
                Value::I64(0),
                Value::I64(0),
            ],
        )
        .expect("enqueue");
    let adapters: BTreeMap<String, Arc<dyn CapabilityAdapter>> = BTreeMap::from([
        (
            "clock".to_owned(),
            Arc::new(DeterministicClockAdapter::new(
                interface(&program, "work", "clock"),
                vec![100, 101],
            )) as Arc<dyn CapabilityAdapter>,
        ),
        ("jobs".to_owned(), queue.clone()),
    ]);
    let deployment = ResidentDeployment::prepare(
        program.clone(),
        "work",
        grants(&program, "work", adapters),
        ResidentLimits::default(),
        RunPolicy::default(),
    )
    .expect("worker deployment");
    let worker = WorkerApplication::new(deployment, WorkerLimits::default()).expect("worker");
    let receipt = worker
        .run(async { tokio::time::sleep(std::time::Duration::from_millis(5)).await })
        .await
        .expect("worker topology");
    assert!(receipt.productive_iterations >= 1);
    assert!(receipt.idle_iterations >= 1);
    let inspect_policy = CallPolicy {
        operation: "inspect".to_owned(),
        visibility: Visibility::None,
        ..enqueue_policy
    };
    let inspected = queue
        .call(&inspect_policy, vec![Value::text("job-1")])
        .expect("inspect");
    assert!(
        matches!(inspected, Value::List(items) if matches!(items[0].field("state"), Some(Value::Text(state)) if state.as_ref() == "completed"))
    );
    assert_eq!(receipt.shutdown.remaining_tasks, 0);
}

#[test]
fn generic_native_platform_contains_no_service_product_vocabulary() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform");
    for entry in std::fs::read_dir(root).expect("platform directory") {
        let entry = entry.expect("platform entry");
        if entry.path().extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(entry.path()).expect("platform source");
        for forbidden in [
            "lkjournal_",
            "resource_owner_denied",
            "initial_actor_denied",
            "route_missing",
        ] {
            assert!(
                !source.contains(forbidden),
                "generic adapter contains product vocabulary {forbidden}"
            );
        }
    }
}

#[test]
fn every_service_requirement_is_bound_before_execution() {
    let program = program();
    let component = component(&program, "serve");
    let error = BoundCapabilities::bind(component, vec![])
        .expect_err("missing grants must reject before work");
    assert_eq!(error.code, "grant_requirement_missing");
    assert_eq!(
        component
            .requirements
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "clock".to_owned(),
            "bootstrap".to_owned(),
            "config".to_owned(),
            "db".to_owned(),
            "identifiers".to_owned(),
            "jobs".to_owned(),
            "objects".to_owned(),
            "passwords".to_owned(),
            "random".to_owned(),
            "streams".to_owned(),
        ])
    );
}
