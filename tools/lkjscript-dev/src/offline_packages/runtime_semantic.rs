//! Real owned sockets and runtime receipts without launching an executable.
use super::*;
use crate::raw_websocket::{RawMessage, RawWebSocket};
use lkjscript::platform::PreparedDeployment;
use serde_json::{Value, json};
use std::net::SocketAddr;

fn close_executor(
    mut executor: Option<lkjscript::platform::runtime::structured::StructuredExecutor>,
) -> Result<Value, DevError> {
    let executor = executor
        .as_mut()
        .ok_or_else(|| DevError::corrupt("host deployment omitted its executor owner"))?;
    executor.close_dispatch();
    let joined = executor
        .shutdown()
        .map_err(|error| DevError::corrupt(format!("{error:?}")))?;
    require(
        !joined.dispatch_open && joined.active_dispatches == 0 && joined.remaining_workers == 0,
        "host server retained owned workers",
    )?;
    Ok(serde_json::to_value(joined)?)
}

pub(super) fn http_round(
    standalone: &Path,
    evidence_root: &Path,
    label: &str,
    exercise: impl FnOnce(SocketAddr) -> Result<Value, DevError>,
) -> Result<Value, DevError> {
    let started = Instant::now();
    let runtime = semantic_cli::runtime()?;
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))?;
    let address = listener.local_addr()?;
    let mut deployment = PreparedDeployment::load(
        &standalone.join("service.deployment.json"),
        runtime.handle().clone(),
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let application = match deployment.http_application() {
        Ok(application) => application,
        Err(mut error) => {
            deployment.close_uninvoked(&mut error);
            let joined = close_executor(deployment.take_executor());
            return Err(DevError::corrupt(format!(
                "{error}; executor cleanup: {joined:?}"
            )));
        }
    };
    let executor = deployment.take_executor();
    let retained = application.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let running = runtime.spawn(application.serve(listener, async move {
        let _ = receiver.await;
    }));
    let result = exercise(address);
    let _ = sender.send(());
    let stopped = runtime
        .block_on(running)
        .map_err(|error| DevError::infrastructure(error.to_string()))
        .and_then(|result| result.map_err(|error| DevError::corrupt(error.to_string())));
    let host_cleanup = runtime.block_on(retained.shutdown());
    let executor = close_executor(executor);
    require(
        host_cleanup.admission_stopped
            && host_cleanup.remaining_tasks == 0
            && host_cleanup.cleanup_failures.is_empty(),
        "linked HTTP retained application cleanup incomplete",
    )?;
    let executor = executor?;
    let stopped = stopped?;
    require(
        stopped.shutdown.admission_stopped
            && stopped.shutdown.remaining_tasks == 0
            && stopped.shutdown.cleanup_failures.is_empty()
            && !stopped.runtime.resident.accepting
            && stopped.runtime.resident.queued == 0
            && stopped.runtime.resident.active == 0
            && stopped.runtime.admission_permits == 0
            && stopped.runtime.worker_permits == 0,
        "host HTTP shutdown retained runtime ownership",
    )?;
    let resident = &stopped.runtime.resident;
    let elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| DevError::infrastructure("linked HTTP elapsed overflow"))?;
    require(
        elapsed_nanoseconds <= Duration::from_secs(600).as_nanos() as u64,
        "linked HTTP exceeded its original server time bound",
    )?;
    let value = json!({"elapsed_nanoseconds":elapsed_nanoseconds,"observations":result?,"cleanup_complete":true,"matcher_nodes":stopped.matcher_nodes,
        "runtime":{"runs":1,"admitted_tasks":resident.admitted,"completed_tasks":resident.completed,
            "failed_tasks":resident.failed,"cancelled_tasks":resident.cancelled,"overloaded_tasks":resident.overloaded,
            "rejected_after_shutdown_tasks":resident.rejected_after_shutdown,"maximum_queued_tasks":resident.maximum_queued,
            "maximum_active_tasks":resident.maximum_active,"maximum_admission_permits":stopped.runtime.maximum_admission_permits,
            "maximum_worker_permits":stopped.runtime.maximum_worker_permits},
        "linked_runtime":{"server":stopped,"executor":executor,"host_cleanup":host_cleanup,"deployment":deployment.observe_redacted()}});
    evidence::publish_json(&evidence_root.join(format!("{label}.json")), &value)?;
    Ok(value)
}

pub(super) fn session_probe(
    standalone: &Path,
    evidence_root: &Path,
    label: &str,
    session_count: usize,
    steps: &[(usize, &str, Value)],
) -> Result<(crate::service::nominal::Observation, CallObservation), DevError> {
    let started = Instant::now();
    let runtime = semantic_cli::runtime()?;
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))?;
    let address = listener.local_addr()?;
    let mut deployment = PreparedDeployment::load(
        &standalone.join("session.deployment.json"),
        runtime.handle().clone(),
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let application = match deployment.session_application() {
        Ok(application) => application,
        Err(mut error) => {
            deployment.close_uninvoked(&mut error);
            let joined = close_executor(deployment.take_executor());
            return Err(DevError::corrupt(format!(
                "{error}; executor cleanup: {joined:?}"
            )));
        }
    };
    let executor = deployment.take_executor();
    let retained = application.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let running = runtime.spawn(application.serve(listener, async move {
        let _ = receiver.await;
    }));
    let mut observed = crate::service::nominal::Observation {
        accept_matches: true,
        ..Default::default()
    };
    let exercise = (|| {
        let mut clients = Vec::new();
        let map = |error| DevError::corrupt(format!("linked raw WebSocket: {error:?}"));
        for _ in 0..session_count {
            let (client, handshake) =
                RawWebSocket::connect(address, "/", &[], Duration::from_secs(10)).map_err(map)?;
            observed.accept_matches &= handshake.accept_matches;
            clients.push(client);
        }
        let mut trace = Vec::new();
        for (session, input, expected) in steps {
            let client = clients
                .get_mut(*session)
                .ok_or_else(|| DevError::corrupt("linked session index invalid"))?;
            client.send_text(input).map_err(map)?;
            let RawMessage::Text(message) = client.read_message().map_err(map)? else {
                return Err(DevError::corrupt(
                    "linked session returned a non-text message",
                ));
            };
            let actual: Value = strict_from_str(&message)?;
            require(
                &actual == expected,
                "linked session differs from the full independent expected value",
            )?;
            trace.push(json!({"session":session,"sent":input,"received":actual}));
            observed.messages.push(actual);
        }
        for mut client in clients {
            client.send_close(1000, "done").map_err(map)?;
            let RawMessage::Close { code, .. } = client.read_message().map_err(map)? else {
                return Err(DevError::corrupt(
                    "linked session omitted close acknowledgement",
                ));
            };
            observed.close_code = code.unwrap_or(0);
            require(
                observed.close_code == 1000,
                "linked session close acknowledgement changed",
            )?;
            client.disconnect().map_err(map)?;
        }
        evidence::publish_json(&evidence_root.join(format!("{label}-wire.json")), &trace)?;
        Ok::<_, DevError>(())
    })();
    let _ = sender.send(());
    let stopped = runtime
        .block_on(running)
        .map_err(|error| DevError::infrastructure(error.to_string()))
        .and_then(|result| result.map_err(|error| DevError::corrupt(error.to_string())));
    let host_cleanup = runtime.block_on(retained.shutdown());
    let executor = close_executor(executor);
    require(
        host_cleanup.admission_stopped
            && host_cleanup.remaining_tasks == 0
            && host_cleanup.cleanup_failures.is_empty(),
        "linked session retained application cleanup incomplete",
    )?;
    let executor = executor?;
    let stopped = stopped?;
    observed.cleanup_complete = stopped.shutdown.admission_stopped
        && stopped.shutdown.remaining_tasks == 0
        && stopped.shutdown.cleanup_failures.is_empty();
    exercise?;
    require(
        observed.accept_matches && observed.cleanup_complete,
        "linked session handshake or cleanup incomplete",
    )?;
    evidence::publish_json(
        &evidence_root.join(format!("{label}-messages.json")),
        &observed,
    )?;
    let response = json!({"server":stopped,"executor":executor,"host_cleanup":host_cleanup,"deployment":deployment.observe_redacted()});
    let mut call = semantic_cli::observe_json(
        "deployment.session.serve",
        &[standalone
            .join("session.deployment.json")
            .display()
            .to_string()],
        &evidence_root.join(format!("{label}.stdout")),
        &evidence_root.join(format!("{label}.stderr")),
        evidence_root,
        || Ok(response),
    )?;
    if let CallObservation::Semantic(observation) = &mut call {
        observation.elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos())
            .map_err(|_| DevError::infrastructure("session elapsed overflow"))?;
    }
    Ok((observed, call))
}

fn closed(value: &Value, artifact: &str) -> Result<(), DevError> {
    let address: SocketAddr = value["server"]["local_address"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("linked server omitted its actual listener"))?
        .parse()
        .map_err(|_| DevError::corrupt("linked server listener invalid"))?;
    require(
        address.ip() == std::net::Ipv4Addr::LOCALHOST
            && address.port() != 0
            && value["deployment"]["artifact_digest"] == artifact
            && value["server"]["accepted_at_transport"] == true
            && value["server"]["shutdown"]["admission_stopped"] == true
            && value["server"]["shutdown"]["remaining_tasks"] == 0
            && value["server"]["shutdown"]["cleanup_failures"] == json!([])
            && value["executor"]["dispatch_open"] == false
            && value["executor"]["active_dispatches"] == 0
            && value["executor"]["remaining_workers"] == 0
            && value["host_cleanup"]["admission_stopped"] == true
            && value["host_cleanup"]["remaining_tasks"] == 0
            && value["host_cleanup"]["cleanup_failures"] == json!([]),
        "linked server changed admitted artifact or retained runtime/executor custody",
    )
}

fn identity(root: &Path, artifact: &str) -> Result<String, DevError> {
    lkjscript::platform::contributor::strict_artifact_identity_probe(&process::read_bounded(
        &root.join(artifact),
        MAXIMUM_CONTAINER_BYTES,
    )?)
    .map_err(|error| DevError::corrupt(error.to_string()))
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<(), DevError> {
    for (fixture, project, output) in [
        (
            "standalone.lkja",
            "http-consumer",
            "standalone/application.lkja",
        ),
        (
            "nominal-session.lkja",
            "nominal-session",
            "nominal-session-standalone/session.lkja",
        ),
        (
            "foreground-1-original.lkja",
            "foreground-1",
            "foreground-1-bundle/application.lkja",
        ),
        (
            "requirement-consumer.lkja",
            "requirement-consumer",
            "requirement-bundle/consumer.lkja",
        ),
    ] {
        let expected = vec![
            receipt.host_path.clone(),
            "--project".into(),
            Path::new(&receipt.isolated_root)
                .join(project)
                .display()
                .to_string(),
            "build".into(),
            "--output".into(),
            Path::new(&receipt.isolated_root)
                .join(output)
                .display()
                .to_string(),
        ];
        let calls = receipt
            .commands
            .iter()
            .filter(|call| call.command == expected)
            .collect::<Vec<_>>();
        require(
            calls.len() == 1,
            "final source fixture is not bound to one exact original linked build",
        )?;
        let call = calls[0];
        call.observation.require_outcome(true)?;
        call.observation.require_arguments(&expected[1..])?;
        let built = parse_records(
            "original-source-fixture-build",
            &process::read_bounded(
                &root.join(call.observation.stdout().path.clone()),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("original fixture build response malformed"))?;
        require(
            field(&built, "artifact", "bundle")? == identity(root, fixture)?
                && field(&built, "artifact", "bytes")?
                    == fs::metadata(root.join(fixture))?.len().to_string(),
            "final source fixture differs from its original linked build artifact and byte length",
        )?;
    }
    let standalone: Value = strict_from_slice(&process::read_bounded(
        &root.join("standalone.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let authority = standalone["grants"][0]["authority_revision"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("fresh HTTP authority revision missing"))?;
    require(
        authority.len() == 64
            && authority
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "fresh HTTP authority is not a canonical generated revision",
    )?;
    let mut original_starter = standalone_descriptor(authority);
    original_starter["artifact"] = json!("generated/application.lkja");
    require(
        strict_from_slice::<Value>(&process::read_bounded(
            &root.join("standalone.starter.deployment.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)? == original_starter,
        "detached HTTP descriptor changed its original public starter authority or bounded shape",
    )?;
    let authored_new = vec![
        receipt.host_path.clone(),
        "new".into(),
        Path::new(&receipt.isolated_root)
            .join("http-consumer")
            .display()
            .to_string(),
        "--template".into(),
        "http".into(),
        "--name".into(),
        "offline-http".into(),
    ];
    let created = receipt
        .commands
        .iter()
        .filter(|call| call.command == authored_new)
        .collect::<Vec<_>>();
    require(
        created.len() == 1,
        "detached HTTP original starter lacks one exact public new operation",
    )?;
    let created = parse_records(
        "fresh-http-starter",
        &process::read_bounded(
            &root.join(created[0].observation.stdout().path.clone()),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("fresh HTTP starter response malformed"))?;
    require(
        field(&created, "deployment", "descriptor")?
            == Path::new(&receipt.isolated_root)
                .join("http-consumer/service.deployment.json")
                .display()
                .to_string()
            && field(&created, "deployment", "artifact-output")?
                == Path::new(&receipt.isolated_root)
                    .join("http-consumer/generated/application.lkja")
                    .display()
                    .to_string()
            && field(&created, "deployment", "target")? == "serve"
            && field(&created, "deployment", "runner")? == "http"
            && field(&created, "deployment", "listener")? == "127.0.0.1:0",
        "detached HTTP original authored starter binding changed",
    )?;
    for (name, expected) in [
        (
            "foreground-1.deployment.json",
            super::foreground::descriptor(1),
        ),
        (
            "nominal-session.deployment.json",
            super::nominal_session::descriptor(),
        ),
        (
            "standalone.deployment.json",
            standalone_descriptor(authority),
        ),
    ] {
        let bytes = process::read_bounded(&root.join(name), MAXIMUM_OUTPUT_BYTES)?;
        lkjscript::platform::decode_deployment(&bytes)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            strict_from_slice::<Value>(&bytes)? == expected,
            "final source descriptor changed its complete bounded literal grants, target or policy",
        )?;
    }
    let mut rounds = vec![
        ("standalone-runtime".to_owned(), "standalone.lkja"),
        ("effects-initial".to_owned(), "effect-original.lkja"),
        ("effects-restarted".to_owned(), "effect-original.lkja"),
        ("effects-successor".to_owned(), "effect-successor.lkja"),
        ("effects-old-supported".to_owned(), "effect-original.lkja"),
        (
            "recursive-data-initial".to_owned(),
            "recursive-data-initial.lkja",
        ),
        (
            "recursive-data-restarted".to_owned(),
            "recursive-data-initial.lkja",
        ),
        (
            "recursive-data-body-only".to_owned(),
            "recursive-data-body-only.lkja",
        ),
        (
            "recursive-data-layout-change".to_owned(),
            "recursive-data-layout-change.lkja",
        ),
        ("iteration-data".to_owned(), "effect-original.lkja"),
        ("iteration-restart".to_owned(), "effect-original.lkja"),
    ];
    rounds
        .extend((0..9).map(|index| (format!("iteration-config-{index}"), "effect-original.lkja")));
    for (name, artifact) in rounds {
        let round: Value = strict_from_slice(&process::read_bounded(
            &root.join(format!("{name}.json")),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        let value = &round["linked_runtime"];
        closed(value, &identity(root, artifact)?)?;
        require(
            round["elapsed_nanoseconds"]
                .as_u64()
                .is_some_and(|elapsed| {
                    elapsed > 0 && elapsed <= Duration::from_secs(600).as_nanos() as u64
                }),
            "linked HTTP elapsed evidence exceeded its original server time bound",
        )?;
        let runtime = &value["server"]["runtime"];
        let resident = &runtime["resident"];
        require(
            round["cleanup_complete"] == true
                && round["matcher_nodes"] == value["server"]["matcher_nodes"]
                && round["runtime"]["runs"] == 1
                && resident["accepting"] == false
                && resident["queued"] == 0
                && resident["active"] == 0
                && runtime["admission_permits"] == 0
                && runtime["worker_permits"] == 0,
            "linked HTTP runtime summary changed its actual terminal receipt",
        )?;
        for (summary, actual) in [
            ("admitted_tasks", "admitted"),
            ("completed_tasks", "completed"),
            ("failed_tasks", "failed"),
            ("cancelled_tasks", "cancelled"),
            ("overloaded_tasks", "overloaded"),
            ("rejected_after_shutdown_tasks", "rejected_after_shutdown"),
            ("maximum_queued_tasks", "maximum_queued"),
            ("maximum_active_tasks", "maximum_active"),
        ] {
            require(
                resident[actual].as_u64().is_some()
                    && round["runtime"][summary] == resident[actual],
                "linked HTTP summary substituted actual scheduling counters",
            )?;
        }
        for name in ["maximum_admission_permits", "maximum_worker_permits"] {
            require(
                runtime[name].as_u64().is_some() && round["runtime"][name] == runtime[name],
                "linked HTTP permit summary substituted",
            )?;
        }
        if name == "standalone-runtime" {
            let response: Value = strict_from_slice(&process::read_bounded(
                &root.join("standalone.stdout"),
                MAXIMUM_OUTPUT_BYTES,
            )?)?;
            require(
                response == round,
                "standalone linked operation substituted its actual runtime receipt",
            )?;
        }
    }
    for (label, artifact, steps, sessions) in [
        (
            "nominal-session",
            "nominal-session.lkja",
            vec![
                (0, "a", json!({"first":1,"second":"a"})),
                (0, "b", json!({"first":2,"second":"ab"})),
            ],
            1,
        ),
        (
            "recursive-session",
            "recursive-session.lkja",
            super::recursive::session_steps(),
            2,
        ),
    ] {
        let response: Value = strict_from_slice(&process::read_bounded(
            &root.join(format!("{label}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        closed(&response, &identity(root, artifact)?)?;
        require(
            response["server"]["sessions"]["pending_handshakes"] == 0
                && response["server"]["sessions"]["active_sessions"] == 0
                && response["server"]["sessions"]["admitted_sessions"] == sessions
                && response["server"]["sessions"]["completed_sessions"] == sessions,
            "linked WebSocket session inventory or shutdown changed",
        )?;
        let wire: Value = strict_from_slice(&process::read_bounded(
            &root.join(format!("{label}-wire.json")),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        require(wire == json!(steps.iter().map(|(session,sent,received)|json!({"session":session,"sent":sent,"received":received})).collect::<Vec<_>>()),
            "linked session raw wire omitted or changed a full expected transition")?;
    }
    let operations = [
        "deployment.http.serve",
        "deployment.session.serve",
        "contributor.nominal_session_state_probe",
        "deployment.session.serve",
        "contributor.recursive_transaction_probe",
        "contributor.recursive_execution_probe",
        "contributor.effect_execution_probe",
        "contributor.effect_transaction_probe",
        "contributor.finite_callable_execution_probe",
    ];
    let owned = |name| {
        Path::new(&receipt.isolated_root)
            .join(name)
            .display()
            .to_string()
    };
    let session_project = owned("nominal-session");
    let export = receipt
        .commands
        .iter()
        .find(|call| {
            call.command
                .get(1)
                .is_some_and(|value| value == "--project")
                && call.command.get(2) == Some(&session_project)
                && call
                    .command
                    .get(3..6)
                    .is_some_and(|args| args == ["package", "current", "export"])
        })
        .ok_or_else(|| DevError::corrupt("nominal session original export omitted"))?;
    let export_records = parse_records(
        "nominal-session-original-export",
        &process::read_bounded(
            &root.join(export.observation.stdout().path.clone()),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("nominal session export malformed"))?;
    let transport = field(&export_records, "package", "transport")?;
    let transport_index = receipt
        .transport_digests
        .iter()
        .position(|digest| digest == &transport)
        .ok_or_else(|| DevError::corrupt("nominal session transport omitted"))?;
    let arguments = [
        vec![owned("standalone/service.deployment.json")],
        vec![owned("nominal-session-standalone/session.deployment.json")],
        vec![
            owned("nominal-session-standalone/session.deployment.json"),
            root.join(format!("transport-{}.lkjp", transport_index + 1))
                .display()
                .to_string(),
            transport,
        ],
        vec![owned(
            "recursive-session-standalone/session.deployment.json",
        )],
        vec![
            owned("recursive-data-standalone/service.deployment.json"),
            allocated(receipt, root, "recursive-data-producer", "$write")?,
        ],
        vec![owned("recursive-receiver")],
        vec![owned("effect-consumer")],
        vec![
            owned("effect-standalone/service.deployment.json"),
            allocated(receipt, root, "effect-consumer", "$transaction-jobs")?,
        ],
        vec![owned("finite-consumer")],
    ];
    for ((call, operation), arguments) in receipt.runners.iter().zip(operations).zip(arguments) {
        let call = call
            .as_semantic()
            .ok_or_else(|| DevError::corrupt("source runtime probe became process evidence"))?;
        require(
            call.operation == operation
                && call.arguments == arguments
                && call.elapsed_nanoseconds
                    <= Duration::from_secs(match operation {
                        "deployment.http.serve" => 600,
                        "contributor.effect_execution_probe"
                        | "contributor.finite_callable_execution_probe" => 180,
                        _ => 120,
                    })
                    .as_nanos() as u64,
            "source runtime probe operation or explicit fixture argument substituted",
        )?;
    }
    Ok(())
}

fn standalone_descriptor(authority: &str) -> Value {
    json!({"artifact":"application.lkja","target":"serve","listen":"127.0.0.1:0",
        "runtime":{"maximum_concurrent_tasks":16,"maximum_queued_tasks":64,"request_deadline_milliseconds":30000,"shutdown_grace_milliseconds":30000,"cancellation_grace_milliseconds":5000},
        "execution":{"instruction_fuel":null,"maximum_call_depth":4096,"maximum_value_stack":1000000,"maximum_allocated_bytes":null,"maximum_collection_items":null,"maximum_capability_calls":null},
        "http":{"maximum_request_body_bytes":8388608,"maximum_response_body_bytes":4194304,"maximum_header_bytes":32768,"maximum_headers":128},
        "session":null,"worker":null,"streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":67108864,"maximum_live_streams":1024},
        "configuration":{},"secrets":[],"grants":[{"requirement":"streams","sharing_domain":"http-request-streams","authority_revision":authority,"adapter":{"kind":"byte_stream"}}]})
}

fn allocated(
    receipt: &Receipt,
    root: &Path,
    project: &str,
    symbol: &str,
) -> Result<String, DevError> {
    let project = Path::new(&receipt.isolated_root)
        .join(project)
        .display()
        .to_string();
    let mut found = std::collections::BTreeSet::new();
    for call in &receipt.commands {
        if call
            .command
            .get(1)
            .is_some_and(|value| value == "--project")
            && call.command.get(2) == Some(&project)
            && call
                .command
                .get(3..5)
                .is_some_and(|args| args == ["change", "apply"])
        {
            let records = parse_records(
                "source-probe-allocated-symbol",
                &process::read_bounded(
                    &root.join(call.observation.stdout().path.clone()),
                    MAXIMUM_OUTPUT_BYTES,
                )?,
            )
            .map_err(|_| DevError::corrupt("source probe authoring output malformed"))?;
            for record in records
                .iter()
                .filter(|record| record.operation == "identity")
            {
                if record_field(record, "symbol")? == symbol {
                    found.insert(record_field(record, "id")?);
                }
            }
        }
    }
    require(
        found.len() == 1,
        "source probe helper is not bound to one exact public authoring allocation",
    )?;
    found
        .into_iter()
        .next()
        .ok_or_else(|| DevError::corrupt("source probe helper allocation absent"))
}
