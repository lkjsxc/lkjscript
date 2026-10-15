use super::super::requirements::conflict;
use super::*;
use lkjscript::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use std::net::{Ipv4Addr, TcpListener};

fn counter(bundle: &Path) -> Result<i64, DevError> {
    let store = DataStore::open(&bundle.join("data"), "foreground", DataLimits::default())
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let transaction = store
        .begin()
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let key = DataKey::new(
        vec![DataKeyPart::Text("executions".into())],
        &DataLimits::default(),
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let value = transaction
        .get("foreground", &key)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    match value {
        Some(entry) => strict_from_slice::<Value>(&entry.value)?
            .as_i64()
            .ok_or_else(|| DevError::corrupt("final foreground counter is not integer")),
        None => Ok(0),
    }
}

fn committed(bundle: &Path) -> Result<Value, DevError> {
    let store = DataStore::open(&bundle.join("data"), "foreground", DataLimits::default())
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let snapshot = store
        .begin()
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let mut values = BTreeMap::new();
    for name in ["executions", "report"] {
        let key = DataKey::new(vec![DataKeyPart::Text(name.into())], &DataLimits::default())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let value = snapshot
            .get("foreground", &key)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        values.insert(
            name,
            value
                .map(|entry| strict_from_slice::<Value>(&entry.value))
                .transpose()?,
        );
    }
    Ok(
        json!({"values":values,"revision":snapshot.base_revision(),"head":fs::read(bundle.join("data/HEAD"))?}),
    )
}

fn conflict_workflow(context: &mut BoundaryContext) -> Result<(), DevError> {
    let bundle = context.root.join("conflict");
    fs::create_dir(&bundle)?;
    fs::copy(
        context.fixture("requirement-consumer.lkja"),
        bundle.join("consumer.lkja"),
    )?;
    context.cli(
        "conflict-data-initialize",
        None,
        &[
            "data",
            "initialize",
            "--root",
            &bundle.join("numbers").display().to_string(),
        ],
        true,
    )?;
    let before = conflict::state(&bundle)?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let address = listener.local_addr()?;
    let descriptor = bundle.join("concurrent.deployment.json");
    let deployment = conflict::deployment(address);
    fs::write(&descriptor, evidence::encode_json(&deployment)?)?;
    fs::write(
        context.evidence.join("conflict.deployment.json"),
        evidence::encode_json(&deployment)?,
    )?;
    let specs = [
        ("conflict-left", "left", 10),
        ("conflict-right", "right", 20),
    ]
    .map(|(name, key, value)| {
        context.specification(
            name,
            &context.root,
            &strings(&[
                "run",
                "--deployment",
                &descriptor.display().to_string(),
                "--arguments",
                &json!([key, value]).to_string(),
            ]),
        )
    });
    let evidence = &context.evidence;
    let (outputs, requests) = std::thread::scope(|scope| -> Result<_, DevError> {
        let oracle = std::thread::Builder::new()
            .name("final-conflict-barrier".into())
            .spawn_scoped(scope, move || conflict::barrier(listener))?;
        let mut children = Vec::new();
        let mut failure = None;
        for spec in &specs {
            match std::thread::Builder::new()
                .name("final-conflict-process".into())
                .spawn_scoped(scope, move || process::run(spec, evidence))
            {
                Ok(child) => children.push(child),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        let outputs = children
            .into_iter()
            .map(|child| child.join())
            .collect::<Vec<_>>();
        conflict::release_blocked_accepts(address);
        let requests = oracle.join();
        match failure {
            Some(error) => Err(error.into()),
            None => Ok((outputs, requests)),
        }
    })?;
    let mut values = Vec::new();
    for ((name, spec), output) in ["conflict-left", "conflict-right"]
        .into_iter()
        .zip(&specs)
        .zip(outputs)
    {
        let output =
            output.map_err(|_| DevError::infrastructure("final conflict observer panicked"))?;
        let records = context.retain(name, spec, true, output)?;
        values.push(strict_from_str::<Value>(&field(
            &records,
            "execution",
            "value",
        )?)?);
    }
    let requests =
        requests.map_err(|_| DevError::infrastructure("final conflict barrier panicked"))??;
    fs::write(
        context.evidence.join("conflict.requests.json"),
        evidence::encode_json(&requests)?,
    )?;
    let winner = conflict::outcomes(&values)?;
    let state = json!({"before":before,"after":conflict::state(&bundle)?,"reopened":conflict::state(&bundle)?});
    conflict::states(&state, winner)?;
    fs::write(
        context.evidence.join("conflict.states.json"),
        evidence::encode_json(&state)?,
    )?;
    context.receipt.assertions.insert(
        "conflict".into(),
        json!({"winner":winner,"requests":2,"released_after_both_reads":true,"joined":true}),
    );
    Ok(())
}

fn signal(
    context: &mut BoundaryContext,
    bundle: &Path,
    name: &str,
    expected: i64,
    terminate: bool,
) -> Result<(), DevError> {
    let before = committed(bundle)?;
    require(
        counter(bundle)? == expected - 1,
        "final signal counter has another pre-invocation value",
    )?;
    let descriptor = bundle.join("cancel.deployment.json");
    let spec = context.specification(
        name,
        &context.root.join("unrelated"),
        &strings(&[
            "run",
            "--deployment",
            &descriptor.display().to_string(),
            "--arguments",
            "[0]",
        ]),
    );
    let control = process::ProcessControl::default();
    let child_control = control.clone();
    let evidence = context.evidence.clone();
    let child_spec = spec.clone();
    let child = std::thread::Builder::new()
        .name(format!("final-{name}"))
        .spawn(move || process::run_controlled(&child_spec, &evidence, &child_control))?;
    let observed = (|| {
        let deadline = Instant::now() + Duration::from_secs(30);
        while counter(bundle)? != expected {
            require(
                !child.is_finished() && Instant::now() < deadline,
                "final process did not commit before its signal",
            )?;
            std::thread::sleep(Duration::from_millis(10));
        }
        if terminate {
            control.terminate();
        } else {
            control.interrupt();
        }
        Ok::<_, DevError>(())
    })();
    if observed.is_err() {
        control.kill();
    }
    let output = child
        .join()
        .map_err(|_| DevError::infrastructure("final signal observer panicked"))?;
    observed?;
    let records = context.retain(name, &spec, false, output)?;
    require(
        field(&records, "diagnostic", "code")? == "execution_cancelled",
        "final OS signal did not become a joined cancellation diagnostic",
    )?;
    let notes: Vec<String> = strict_from_str(&field(&records, "diagnostic", "notes")?)?;
    require(
        notes.iter().any(|note| {
            note == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"
        }) && notes
            .iter()
            .any(|note| note.contains("earlier application effects may already be visible")),
        "final signal lost completed cleanup or effect visibility",
    )?;
    let after = committed(bundle)?;
    let reopened = committed(bundle)?;
    require(
        counter(bundle)? == expected && after == reopened,
        "final OS signal retried or lost the committed invocation",
    )?;
    context.receipt.assertions.insert(name.into(),json!({"before":before,"committed_before_signal":expected,"after":after,"reopened":reopened,"joined":true,"signal":if terminate {"SIGTERM"} else {"SIGINT"}}));
    Ok(())
}

pub(super) fn workflow(context: &mut BoundaryContext) -> Result<(), DevError> {
    let bundle = context.root.join("foreground");
    fs::create_dir(&bundle)?;
    fs::copy(
        context.fixture("foreground-1-original.lkja"),
        bundle.join("application.lkja"),
    )?;
    let descriptor: Value = strict_from_slice(&process::read_bounded(
        &context.fixture("foreground-1.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    fs::write(
        bundle.join("command.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::write(
        context.evidence.join("foreground.command.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    let mut cancel = descriptor;
    cancel["target"] = json!("cancel-after-commit");
    fs::write(
        bundle.join("cancel.deployment.json"),
        evidence::encode_json(&cancel)?,
    )?;
    fs::write(
        context.evidence.join("foreground.cancel.deployment.json"),
        evidence::encode_json(&cancel)?,
    )?;
    context.cli(
        "foreground-data-initialize",
        None,
        &[
            "data",
            "initialize",
            "--root",
            &bundle.join("data").display().to_string(),
        ],
        true,
    )?;
    conflict_workflow(context)?;
    signal(context, &bundle, "sigint-after-commit", 1, false)?;
    signal(context, &bundle, "sigterm-after-commit", 2, true)?;
    let before = committed(&bundle)?;
    let spec = context.specification(
        "closed-stdout-after-commit",
        &context.root.join("unrelated"),
        &strings(&[
            "run",
            "--deployment",
            &bundle.join("command.deployment.json").display().to_string(),
            "--arguments",
            "[0]",
        ]),
    );
    let output = process::run_closed_stdout(&spec, &context.evidence);
    context.retain("closed-stdout-after-commit", &spec, false, output)?;
    let error: Value = strict_from_slice(&process::read_bounded(
        &spec.stderr_path,
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        error["code"] == "cli_output"
            && error["notes"].as_array().is_some_and(|notes| {
                notes.iter().any(|note| {
                    note.as_str().is_some_and(|note| {
                        note.contains(
                            "invocation and cleanup completed; earlier effects may be visible",
                        )
                    })
                })
            })
            && counter(&bundle)? == 3,
        "closed final stdout did not report completion with exactly one visible effect",
    )?;
    let after = committed(&bundle)?;
    require(
        after == committed(&bundle)?,
        "closed final stdout changed committed data on reopen",
    )?;
    context.receipt.assertions.insert(
        "closed-stdout-after-commit".into(),
        json!({"before":before,"after":after,"reopened":committed(&bundle)?,"joined":true}),
    );
    Ok(())
}

fn snapshot(value: &Value, count: i64) -> Result<(), DevError> {
    require(
        value["values"]["executions"] == count
            && value["revision"]
                .as_str()
                .is_some_and(|revision| !revision.is_empty())
            && value["head"].as_array().is_some_and(|head| {
                !head.is_empty()
                    && head
                        .iter()
                        .all(|byte| byte.as_u64().is_some_and(|byte| byte <= 255))
            }),
        "final committed counter/revision/HEAD evidence omitted",
    )
}

pub(super) fn validate(receipt: &BoundaryReceipt, root: &Path) -> Result<(), DevError> {
    let project = |name| Path::new(&receipt.isolated_root).join(name);
    let original: Value = strict_from_slice(&process::read_bounded(
        &root.join("fixture-foreground-1.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        strict_from_slice::<Value>(&process::read_bounded(
            &root.join("foreground.command.deployment.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)? == original,
        "final foreground command descriptor changed its admitted source fixture",
    )?;
    let mut cancel = original;
    cancel["target"] = json!("cancel-after-commit");
    require(
        strict_from_slice::<Value>(&process::read_bounded(
            &root.join("foreground.cancel.deployment.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)? == cancel,
        "final foreground cancellation changed more than its exact target",
    )?;
    for name in [
        "sigint-after-commit",
        "sigterm-after-commit",
        "closed-stdout-after-commit",
    ] {
        require(
            receipt
                .processes
                .iter()
                .find(|call| call.name == name)
                .is_some_and(|call| call.cwd == project("unrelated").display().to_string()),
            "final foreground boundary lost its unrelated working directory",
        )?;
    }

    for (name, bundle, data) in [
        ("foreground-data-initialize", "foreground", "data"),
        ("conflict-data-initialize", "conflict", "numbers"),
    ] {
        records(
            receipt,
            root,
            name,
            None,
            &strings(&[
                "data",
                "initialize",
                "--root",
                &project(bundle).join(data).display().to_string(),
            ]),
            true,
        )?;
    }
    let deployment: Value = strict_from_slice(&process::read_bounded(
        &root.join("conflict.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let address: std::net::SocketAddr = deployment["grants"][1]["adapter"]["endpoint"]
        .as_str()
        .and_then(|endpoint| endpoint.strip_prefix("http://"))
        .and_then(|endpoint| endpoint.strip_suffix("/barrier"))
        .ok_or_else(|| DevError::corrupt("final conflict endpoint omitted"))?
        .parse()
        .map_err(|_| DevError::corrupt("final conflict address invalid"))?;
    require(
        address.ip() == Ipv4Addr::LOCALHOST
            && address.port() != 0
            && deployment == conflict::deployment(address),
        "final conflict target, limits or grants changed",
    )?;
    let requests: Vec<String> = strict_from_slice(&process::read_bounded(
        &root.join("conflict.requests.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        requests.len() == 2
            && requests.iter().all(|request| {
                request.starts_with("GET /barrier HTTP/1.1\r\n")
                    && request.ends_with("\r\n\r\n")
                    && request.len() <= 8192
                    && request
                        .split("\r\n")
                        .filter_map(|line| line.split_once(':'))
                        .any(|(key, value)| {
                            key.eq_ignore_ascii_case("host") && value.trim() == address.to_string()
                        })
            }),
        "final two-process read barrier requests changed",
    )?;
    let artifact =
        lkjscript::platform::contributor::strict_artifact_identity_probe(&process::read_bounded(
            &root.join("fixture-requirement-consumer.lkja"),
            MAXIMUM_CONTAINER_BYTES,
        )?)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let mut values = Vec::new();
    for (name, key, value) in [
        ("conflict-left", "left", 10),
        ("conflict-right", "right", 20),
    ] {
        let output = records(
            receipt,
            root,
            name,
            None,
            &strings(&[
                "run",
                "--deployment",
                &project("conflict")
                    .join("concurrent.deployment.json")
                    .display()
                    .to_string(),
                "--arguments",
                &json!([key, value]).to_string(),
            ]),
            true,
        )?;
        require(
            field(&output, "execution", "artifact")? == artifact,
            "final conflict process used another source fixture",
        )?;
        closed_execution(&output, &artifact, "concurrent-update")?;
        let cleanup: Value = strict_from_str(&field(&output, "execution", "cleanup")?)?;
        let work: Value = strict_from_str(&field(&output, "execution", "production-observation")?)?;
        require(
            cleanup["remaining_tasks"] == 0
                && cleanup["cleanup_failures"] == json!([])
                && work["capability_calls"] == 5,
            "final conflict did not perform one callback with joined cleanup",
        )?;
        for key in [
            "live_call_frames_after",
            "live_handles_after",
            "live_locals_after",
            "live_operands_after",
            "live_transactions_after",
            "live_type_bindings_after",
        ] {
            require(work[key] == 0, "final conflict retained VM custody")?;
        }
        values.push(strict_from_str::<Value>(&field(
            &output,
            "execution",
            "value",
        )?)?);
    }
    let winner = conflict::outcomes(&values)?;
    require(
        receipt.assertions["conflict"]
            == json!({"winner":winner,"requests":2,"released_after_both_reads":true,"joined":true}),
        "final conflict ownership assertion changed",
    )?;
    conflict::states(
        &strict_from_slice::<Value>(&process::read_bounded(
            &root.join("conflict.states.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)?,
        winner,
    )?;
    for (name, expected, signal_name) in [
        ("sigint-after-commit", 1, "SIGINT"),
        ("sigterm-after-commit", 2, "SIGTERM"),
    ] {
        let output = records(
            receipt,
            root,
            name,
            None,
            &strings(&[
                "run",
                "--deployment",
                &project("foreground")
                    .join("cancel.deployment.json")
                    .display()
                    .to_string(),
                "--arguments",
                "[0]",
            ]),
            false,
        )?;
        let witness = &receipt.assertions[name];
        require(
            witness["joined"] == true
                && witness["committed_before_signal"] == expected
                && witness["signal"] == signal_name
                && witness["after"] == witness["reopened"]
                && field(&output, "diagnostic", "code")? == "execution_cancelled",
            "final signal translation, joined cancellation or no-retry witness changed",
        )?;
        if expected > 1 {
            snapshot(&witness["before"], expected - 1)?;
        } else {
            require(
                witness["before"]["values"]["executions"].is_null(),
                "first signal invocation did not start from fresh data",
            )?;
        }
        snapshot(&witness["after"], expected)?;
        let notes: Vec<String> = strict_from_str(&field(&output, "diagnostic", "notes")?)?;
        require(notes.iter().any(|note|note=="foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0") && notes.iter().any(|note|note.contains("earlier application effects may already be visible")),"final signal omitted visibility or cleanup")?;
    }
    let call = receipt
        .processes
        .iter()
        .find(|call| call.name == "closed-stdout-after-commit")
        .ok_or_else(|| DevError::corrupt("closed-output boundary absent"))?;
    require(
        call.command
            == std::iter::once(receipt.executable.clone())
                .chain(strings(&[
                    "run",
                    "--deployment",
                    &project("foreground")
                        .join("command.deployment.json")
                        .display()
                        .to_string(),
                    "--arguments",
                    "[0]",
                ]))
                .collect::<Vec<_>>()
            && !call.expects_success
            && process::read_bounded(
                &root.join("closed-stdout-after-commit.stdout"),
                MAXIMUM_OUTPUT_BYTES,
            )?
            .is_empty(),
        "closed-output boundary command or actual FD capture substituted",
    )?;
    let error: Value = strict_from_slice(&process::read_bounded(
        &root.join("closed-stdout-after-commit.stderr"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        error["code"] == "cli_output"
            && error["notes"].as_array().is_some_and(|notes| {
                notes.iter().any(|note| {
                    note.as_str().is_some_and(|note| {
                        note.contains(
                            "invocation and cleanup completed; earlier effects may be visible",
                        )
                    })
                })
            }),
        "closed final stdout lost completed invocation diagnostic",
    )?;
    let witness = &receipt.assertions["closed-stdout-after-commit"];
    snapshot(&witness["before"], 2)?;
    snapshot(&witness["after"], 3)?;
    require(
        witness["joined"] == true
            && witness["after"] == witness["reopened"]
            && witness["after"]["values"]["report"]
                == json!({"executions":3,"result":{"position":1,"total":0},"payload":{"case":"children","value":[{"case":"item","value":{"value":0}}]}}),
        "closed final stdout replayed, discarded or changed the full committed application result",
    )?;
    Ok(())
}
