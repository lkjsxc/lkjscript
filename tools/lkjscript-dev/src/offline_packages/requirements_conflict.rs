//! Two public imported updates share a base, with a host that only schedules HTTP replies.
use super::*;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};

const MATERIAL: &str = "requirement-conflict";
const REQUESTS: [(&str, i64); 2] = [("left", 10), ("right", 20)];
const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

fn barrier(listener: TcpListener) -> Result<Vec<String>, DevError> {
    let mut connections = Vec::new();
    let mut requests = Vec::new();
    for _ in REQUESTS {
        let (mut stream, peer) = listener.accept()?;
        require(
            peer.ip().is_loopback(),
            "conflict barrier admitted a non-loopback peer",
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut byte = [0_u8; 1];
            require(bytes.len() < 8192, "conflict barrier request exceeds bound")?;
            stream.read_exact(&mut byte)?;
            bytes.push(byte[0]);
        }
        let request = String::from_utf8(bytes)
            .map_err(|_| DevError::corrupt("conflict barrier request is not HTTP text"))?;
        require(
            request.starts_with("GET /barrier HTTP/1.1\r\n"),
            "conflict barrier did not receive the authored GET",
        )?;
        requests.push(request);
        connections.push(stream);
    }
    // Neither callback can finish until both public bodies have read the shared base snapshot.
    for mut stream in connections {
        stream.write_all(RESPONSE)?;
        stream.flush()?;
    }
    Ok(requests)
}

fn release_blocked_accepts(address: SocketAddr) {
    // Called only after both owned product children have joined. A failed preflight may have
    // made no request; bounded sentinels wake an accept without leaving a detached oracle.
    for _ in REQUESTS {
        if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(200)) {
            let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
            let _ = stream.write_all(b"cancel\r\n\r\n");
        }
    }
}

fn state(bundle: &Path) -> Result<Value, DevError> {
    let root = bundle.join("numbers");
    let store = DataStore::open(&root, "race-cells", DataLimits::default())
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let verified = store
        .verify()
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let snapshot = store
        .begin()
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let mut cells = BTreeMap::new();
    for (key, _) in REQUESTS {
        let lookup = DataKey::new(vec![DataKeyPart::Text(key.into())], store.limits())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let entry = snapshot
            .get("typed-cell", &lookup)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        cells.insert(
            key,
            entry.map(|entry| json!({"bytes":entry.value,"revision":entry.revision})),
        );
    }
    Ok(
        json!({"revision":snapshot.base_revision(),"head":fs::read(root.join("HEAD"))?,"verify":verified,"cells":cells}),
    )
}

fn outcomes(values: &[Value]) -> Result<usize, DevError> {
    require(values.len() == 2, "public conflict outcomes omitted")?;
    let conflict = json!({"case":"Aborted","value":{"case":"Conflict"}});
    for winner in 0..REQUESTS.len() {
        let loser = 1 - winner;
        if values[winner] == json!({"case":"Committed","value":REQUESTS[winner].1 + 3})
            && values[loser] == conflict
        {
            return Ok(winner);
        }
    }
    Err(DevError::corrupt(
        "public writers did not yield exactly one committed value and one definite conflict",
    ))
}

fn states(value: &Value, winner: usize) -> Result<(), DevError> {
    for name in ["before", "after", "reopened"] {
        let snapshot = &value[name];
        require(
            revision_identity(&snapshot["revision"])
                && byte_array(&snapshot["head"], None)
                && snapshot["head"]
                    .as_array()
                    .is_some_and(|bytes| !bytes.is_empty() && bytes.len() <= 160)
                && snapshot["verify"]["revisions"]
                    .as_u64()
                    .is_some_and(|count| count > 0)
                && snapshot["cells"].as_object().is_some_and(|cells| {
                    cells.len() == REQUESTS.len()
                        && REQUESTS
                            .iter()
                            .all(|(key, _)| cells.get(*key).is_some_and(retained_cell))
                }),
            "public conflict physical HEAD/revision or complete cell observations omitted",
        )?;
    }
    let before = &value["before"];
    let after = &value["after"];
    require(
        after == &value["reopened"],
        "public conflict state changed on close/reopen",
    )?;
    require(
        before["cells"]["left"].is_null()
            && before["cells"]["right"].is_null()
            && before["verify"]["records"] == 0
            && after["verify"]["records"] == 1
            && after["verify"]["revisions"].as_u64()
                == before["verify"]["revisions"]
                    .as_u64()
                    .and_then(|count| count.checked_add(1))
            && before["revision"] != after["revision"]
            && before["head"] != after["head"]
            && before["verify"]["revision"] == before["revision"]
            && after["verify"]["revision"] == after["revision"],
        "public conflict did not publish exactly one physical revision",
    )?;
    require(
        after["cells"][REQUESTS[winner].0]["bytes"]
            == json!(typed_bytes(&json!(REQUESTS[winner].1 + 3))?)
            && after["cells"][REQUESTS[1 - winner].0].is_null(),
        "public conflict winner bytes or losing-key absence differ",
    )
}

fn deployment(address: SocketAddr) -> Value {
    let limits = lkjscript::platform::HttpClientLimits {
        total_timeout_milliseconds: 30_000,
        ..Default::default()
    };
    json!({
        "artifact":"consumer.lkja","target":"concurrent-update","listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],
        "grants":[
            {"requirement":"race-store","sharing_domain":"requirement-conflict-data","authority_revision":"a4".repeat(32),"adapter":{"kind":"data","root":"numbers","namespace":"race-cells","limits":DataLimits::default()}},
            {"requirement":"barrier","sharing_domain":"requirement-conflict-http","authority_revision":"a5".repeat(32),"adapter":{"kind":"http_client","endpoint":format!("http://{address}/barrier"),"address_policy":"loopback_only","trust":{"kind":"webpki_roots"},"limits":limits}}
        ]
    })
}

pub(super) fn workflow(context: &mut Context, source_bundle: &Path) -> Result<(), DevError> {
    let bundle = context.root.join(MATERIAL);
    fs::create_dir(&bundle)?;
    fs::copy(
        source_bundle.join("consumer.lkja"),
        bundle.join("consumer.lkja"),
    )?;
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &bundle.join("numbers").display().to_string(),
        ],
        true,
    )?;
    let before = state(&bundle)?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let address = listener.local_addr()?;
    let descriptor = bundle.join("concurrent.deployment.json");
    fs::write(&descriptor, evidence::encode_json(&deployment(address))?)?;
    fs::copy(
        &descriptor,
        context.evidence.join(format!("{MATERIAL}.deployment.json")),
    )?;
    let first_index = context.receipt.commands.len();
    let specs = REQUESTS
        .iter()
        .enumerate()
        .map(|(offset, (key, default))| process::ProcessSpec {
            command: vec![
                context.receipt.pinned_runtime_path.clone(),
                "run".into(),
                "--deployment".into(),
                descriptor.display().to_string(),
                "--arguments".into(),
                json!([key, default]).to_string(),
            ],
            cwd: context.root.clone(),
            environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
            timeout: Duration::from_secs(40),
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: context
                .evidence
                .join(format!("command-{:04}.stdout", first_index + offset)),
            stderr_path: context
                .evidence
                .join(format!("command-{:04}.stderr", first_index + offset)),
            unavailable_exit_code: None,
        })
        .collect::<Vec<_>>();
    let evidence_root = &context.evidence;
    let (observations, requests) = std::thread::scope(|scope| -> Result<_, DevError> {
        let oracle = std::thread::Builder::new()
            .name("requirement-conflict-barrier".into())
            .spawn_scoped(scope, move || barrier(listener))?;
        let mut children = Vec::new();
        let mut spawn_failure = None;
        for spec in &specs {
            match std::thread::Builder::new()
                .name("requirement-conflict-command".into())
                .spawn_scoped(scope, move || process::run(spec, evidence_root))
            {
                Ok(child) => children.push(child),
                Err(error) => {
                    spawn_failure = Some(error);
                    break;
                }
            }
        }
        let observations = children
            .into_iter()
            .map(|child| child.join())
            .collect::<Vec<_>>();
        release_blocked_accepts(address);
        let requests = oracle.join();
        match spawn_failure {
            Some(error) => Err(error.into()),
            None => Ok((observations, requests)),
        }
    })?;
    // All process and oracle resources have joined before any error is returned below.
    for (spec, observation) in specs.iter().zip(observations) {
        let observation = observation
            .map_err(|_| DevError::infrastructure("public conflict process observer panicked"))?;
        context.receipt.commands.push(CommandEvidence {
            command: spec.command.clone(),
            cwd: spec.cwd.display().to_string(),
            expects_success: true,
            observation,
        });
    }
    let requests =
        requests.map_err(|_| DevError::infrastructure("public conflict barrier panicked"))??;
    fs::write(
        context.evidence.join(format!("{MATERIAL}-requests.json")),
        evidence::encode_json(&requests)?,
    )?;
    let mut values = Vec::new();
    for (offset, spec) in specs.iter().enumerate() {
        require(
            context.receipt.commands[first_index + offset]
                .observation
                .status
                == process::ProcessStatus::Passed,
            "public conflict invocation failed instead of returning a normal outcome",
        )?;
        let output = parse_records(
            "public-conflict",
            &process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT_BYTES)?,
        )
        .map_err(|_| DevError::corrupt("public conflict output malformed"))?;
        values.push(serde_json::from_str::<Value>(&field(
            &output,
            "execution",
            "value",
        )?)?);
    }
    let winner = outcomes(&values)?;
    let observed = json!({"before":before,"after":state(&bundle)?,"reopened":state(&bundle)?});
    states(&observed, winner)?;
    fs::write(
        context.evidence.join(format!("{MATERIAL}-states.json")),
        evidence::encode_json(&observed)?,
    )?;
    context.receipt.observations.insert("requirement_conflict".into(),json!({"commands":[first_index,first_index + 1],"requests":2,"released_after_both_reads":true,"joined":true,"winner":winner}).to_string());
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let observed: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_conflict")
            .ok_or_else(|| DevError::corrupt("public transaction conflict evidence missing"))?,
    )?;
    let commands: Vec<usize> = serde_json::from_value(observed["commands"].clone())?;
    require(
        commands.len() == 2
            && commands[0] < commands[1]
            && observed["requests"] == 2
            && observed["released_after_both_reads"] == true
            && observed["joined"] == true,
        "public conflict invocation/barrier ownership evidence differs",
    )?;
    let requests: Vec<String> = serde_json::from_slice(&process::read_bounded(
        &root.join(format!("{MATERIAL}-requests.json")),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let descriptor: Value = serde_json::from_slice(&process::read_bounded(
        &root.join(format!("{MATERIAL}.deployment.json")),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let address: SocketAddr = descriptor["grants"][1]["adapter"]["endpoint"]
        .as_str()
        .and_then(|endpoint| endpoint.strip_prefix("http://"))
        .and_then(|endpoint| endpoint.strip_suffix("/barrier"))
        .ok_or_else(|| DevError::corrupt("public conflict endpoint omitted"))?
        .parse()
        .map_err(|_| DevError::corrupt("public conflict endpoint is not a socket address"))?;
    require(
        address.ip() == Ipv4Addr::LOCALHOST
            && address.port() != 0
            && descriptor == deployment(address),
        "public conflict descriptor changed target, exact grants, or endpoint policy",
    )?;
    require(
        requests.len() == 2
            && requests.iter().all(|request| {
                request.starts_with("GET /barrier HTTP/1.1\r\n")
                    && request.ends_with("\r\n\r\n")
                    && request.len() <= 8192
                    && request
                        .split("\r\n")
                        .filter_map(|line| line.split_once(':'))
                        .any(|(name, value)| {
                            name.eq_ignore_ascii_case("host") && value.trim() == address.to_string()
                        })
            }),
        "public conflict callback count or retained requests differ",
    )?;
    let artifact =
        lkjscript::platform::contributor::strict_artifact_identity_probe(&process::read_bounded(
            &root.join("requirement-consumer.lkja"),
            MAXIMUM_CONTAINER_BYTES,
        )?)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let mut values = Vec::new();
    for (index, (key, default)) in commands.iter().zip(REQUESTS) {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("public conflict command missing"))?;
        let descriptor = Path::new(&receipt.isolated_root)
            .join(MATERIAL)
            .join("concurrent.deployment.json");
        require(
            command.expects_success
                && command.observation.status == process::ProcessStatus::Passed
                && command.command
                    == [
                        receipt.pinned_runtime_path.clone(),
                        "run".into(),
                        "--deployment".into(),
                        descriptor.display().to_string(),
                        "--arguments".into(),
                        json!([key, default]).to_string(),
                    ],
            "public conflict command changed runtime, artifact, arguments or outcome class",
        )?;
        let output = parse_records(
            "public-conflict-output",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("public conflict invocation output malformed"))?;
        let work: Value =
            serde_json::from_str(&field(&output, "execution", "production-observation")?)?;
        let cleanup: Value = serde_json::from_str(&field(&output, "execution", "cleanup")?)?;
        require(
            field(&output, "execution", "artifact")? == artifact
                && work["capability_calls"] == 5
                && cleanup["remaining_tasks"] == 0
                && cleanup["cleanup_failures"] == json!([]),
            "public conflict result was not the selected single callback invocation or joined completion",
        )?;
        for key in [
            "live_call_frames_after",
            "live_handles_after",
            "live_locals_after",
            "live_operands_after",
            "live_transactions_after",
            "live_type_bindings_after",
        ] {
            require(
                work[key] == 0,
                "public conflict retained owned execution state",
            )?;
        }
        values.push(serde_json::from_str::<Value>(&field(
            &output,
            "execution",
            "value",
        )?)?);
    }
    let winner = outcomes(&values)?;
    require(
        observed["winner"] == winner,
        "public conflict winner binding changed",
    )?;
    let observed_states: Value = serde_json::from_slice(&process::read_bounded(
        &root.join(format!("{MATERIAL}-states.json")),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    states(&observed_states, winner)?;
    Ok(commands)
}
