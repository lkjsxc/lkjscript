use super::*;
use std::net::SocketAddr;

fn ready(path: &Path, finished: impl Fn() -> bool) -> Result<Value, DevError> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        require(
            !finished() && Instant::now() < deadline,
            "final server exited or timed out before readiness",
        )?;
        if path.exists() {
            let bytes = process::read_bounded(path, MAXIMUM_OUTPUT_BYTES)?;
            if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                let value: Value = strict_from_slice(&bytes[..end])?;
                require(
                    value["event"] == "ready" && value["ok"] == true,
                    "final server did not publish a ready event",
                )?;
                return Ok(value);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn workflow(context: &mut BoundaryContext) -> Result<(), DevError> {
    let http = context.root.join("http");
    fs::create_dir(&http)?;
    fs::copy(
        context.fixture("standalone.lkja"),
        http.join("application.lkja"),
    )?;
    fs::copy(
        context.fixture("standalone.deployment.json"),
        http.join("service.deployment.json"),
    )?;
    let spec = context.specification(
        "http-start-stop",
        &context.root.join("unrelated"),
        &strings(&[
            "serve",
            "--deployment",
            &http.join("service.deployment.json").display().to_string(),
        ]),
    );
    let control = process::ProcessControl::default();
    let child_control = control.clone();
    let child_spec = spec.clone();
    let evidence = context.evidence.clone();
    let child = std::thread::Builder::new()
        .name("final-http-observer".into())
        .spawn(move || process::run_controlled(&child_spec, &evidence, &child_control))?;
    let result = (|| {
        let ready = ready(&spec.stdout_path, || child.is_finished())?;
        let address: SocketAddr = ready["local_address"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("final ready omitted listener"))?
            .parse()
            .map_err(|_| DevError::corrupt("final listener invalid"))?;
        let response = crate::http_probe::request(address, "GET", "/", &[], &[])?;
        require(
            response.status == 200 && response.body == b"offline-package-closure",
            "final HTTP transport changed the independent literal response",
        )?;
        fs::write(context.evidence.join("http-response.body"), &response.body)?;
        Ok::<_, DevError>(())
    })();
    control.interrupt();
    let output = child
        .join()
        .map_err(|_| DevError::infrastructure("final HTTP observer panicked"))?;
    result?;
    context.retain("http-start-stop", &spec, true, output)?;
    let session = context.root.join("session");
    fs::create_dir(&session)?;
    fs::copy(
        context.fixture("nominal-session.lkja"),
        session.join("session.lkja"),
    )?;
    fs::copy(
        context.fixture("nominal-session.deployment.json"),
        session.join("session.deployment.json"),
    )?;
    let (observation, output) = crate::service::nominal::probe_script(
        &context.binary,
        &session,
        &context.evidence,
        "websocket-start-stop",
        1,
        &[(0, "a", json!({"first":1,"second":"a"}))],
    )?;
    require(
        observation.messages == [json!({"first":1,"second":"a"})]
            && observation.accept_matches
            && observation.close_code == 1000
            && observation.cleanup_complete,
        "final WebSocket handshake, message or close changed",
    )?;
    let spec = context.specification(
        "websocket-start-stop",
        &session,
        &strings(&[
            "serve",
            "--deployment",
            &session
                .join("session.deployment.json")
                .display()
                .to_string(),
        ]),
    );
    context.retain("websocket-start-stop", &spec, true, output)?;
    context.receipt.assertions.insert(
        "services".into(),
        json!({"http_requests":1,"websocket_messages":1,"joined":true}),
    );
    Ok(())
}

fn events(root: &Path, name: &str, artifact: &Path) -> Result<(Value, Value), DevError> {
    let bytes = process::read_bounded(&root.join(format!("{name}.stdout")), MAXIMUM_OUTPUT_BYTES)?;
    let framed = bytes
        .strip_suffix(b"\n")
        .ok_or_else(|| DevError::corrupt("final server omitted complete JSONL framing"))?;
    let values = framed
        .split(|byte| *byte == b'\n')
        .map(strict_from_slice::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    require(
        values.len() == 2
            && values[0]["event"] == "ready"
            && values[0]["ok"] == true
            && values[1]["event"] == "stopped"
            && values[1]["ok"] == true,
        "final server did not retain exactly ready and joined stopped events",
    )?;
    let artifact = lkjscript::platform::contributor::strict_artifact_identity_probe(
        &process::read_bounded(artifact, MAXIMUM_CONTAINER_BYTES)?,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        values[0]["deployment"]["artifact_digest"] == artifact,
        "final server used another admitted source fixture",
    )?;
    let stopped = &values[1];
    let address: SocketAddr = values[0]["local_address"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("final server ready address missing"))?
        .parse()
        .map_err(|_| DevError::corrupt("final server ready address invalid"))?;
    require(
        address.ip() == std::net::Ipv4Addr::LOCALHOST
            && address.port() != 0
            && stopped["receipt"]["local_address"] == address.to_string()
            && stopped["receipt"]["accepted_at_transport"] == true
            && stopped["receipt"]["shutdown"]["admission_stopped"] == true
            && stopped["receipt"]["shutdown"]["remaining_tasks"] == 0
            && stopped["receipt"]["shutdown"]["cleanup_failures"] == json!([])
            && stopped["executor"]["dispatch_open"] == false
            && stopped["executor"]["active_dispatches"] == 0
            && stopped["executor"]["remaining_workers"] == 0,
        "final server retained transport/runtime/executor custody",
    )?;
    Ok((values[0].clone(), values[1].clone()))
}

pub(super) fn validate(receipt: &BoundaryReceipt, root: &Path) -> Result<(), DevError> {
    let project = |name| Path::new(&receipt.isolated_root).join(name);
    for (name, bundle, descriptor, cwd) in [
        (
            "http-start-stop",
            "http",
            "service.deployment.json",
            "unrelated",
        ),
        (
            "websocket-start-stop",
            "session",
            "session.deployment.json",
            "session",
        ),
    ] {
        let call = receipt
            .processes
            .iter()
            .find(|call| call.name == name)
            .ok_or_else(|| DevError::corrupt("final server process omitted"))?;
        require(
            call.expects_success
                && call.command
                    == [
                        receipt.executable.clone(),
                        "serve".into(),
                        "--deployment".into(),
                        project(bundle).join(descriptor).display().to_string(),
                    ]
                && call.cwd == project(cwd).display().to_string(),
            "final server argv or cwd substituted",
        )?;
    }
    let (_, http) = events(
        root,
        "http-start-stop",
        &root.join("fixture-standalone.lkja"),
    )?;
    let resident = &http["receipt"]["runtime"]["resident"];
    require(
        resident["accepting"] == false
            && resident["queued"] == 0
            && resident["active"] == 0
            && resident["admitted"].as_u64().is_some_and(|count| count > 0)
            && resident["admitted"] == resident["completed"]
            && http["receipt"]["runtime"]["admission_permits"] == 0
            && http["receipt"]["runtime"]["worker_permits"] == 0
            && process::read_bounded(&root.join("http-response.body"), MAXIMUM_OUTPUT_BYTES)?
                == b"offline-package-closure",
        "final HTTP request or owned runtime completion missing",
    )?;
    let (_, session) = events(
        root,
        "websocket-start-stop",
        &root.join("fixture-nominal-session.lkja"),
    )?;
    let sessions = &session["receipt"]["sessions"];
    require(
        sessions["pending_handshakes"] == 0
            && sessions["active_sessions"] == 0
            && sessions["admitted_sessions"] == 1
            && sessions["completed_sessions"] == 1
            && sessions["inbound_messages"]
                .as_u64()
                .is_some_and(|count| count >= 1)
            && sessions["outbound_messages"]
                .as_u64()
                .is_some_and(|count| count >= 1),
        "final WebSocket did not complete its one actual session",
    )?;
    let observation: crate::service::nominal::Observation =
        strict_from_slice(&process::read_bounded(
            &root.join("websocket-start-stop-messages.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
    let wire: Value = strict_from_slice(&process::read_bounded(
        &root.join("websocket-start-stop-wire.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        observation.messages == [json!({"first":1,"second":"a"})]
            && observation.accept_matches
            && observation.close_code == 1000
            && observation.cleanup_complete
            && wire == json!([{"session":0,"sent":"a","received":{"first":1,"second":"a"}}])
            && receipt.assertions["services"]
                == json!({"http_requests":1,"websocket_messages":1,"joined":true}),
        "final WebSocket full reply, wire or clean close substituted",
    )?;
    Ok(())
}
