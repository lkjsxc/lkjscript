//! Two final-byte boundaries consume the original complete semantic fixture.

use super::*;
use lkjscript::platform::data::{DataLimits, DataScanDirection, DataStore};

const FIXTURES: [(&str, &str); 5] = [
    ("command.lkja", "command.lkja"),
    ("command.deployment.json", "command.deployment.json"),
    ("command.arguments.json", "command.arguments.json"),
    ("standalone.lkja", "application.lkja"),
    ("standalone.deployment.json", "service.deployment.json"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    schema: String,
    status: String,
    pub(crate) candidate_sha256: String,
    copied_candidate_sha256: String,
    pub(crate) verifier_sha256: String,
    source_receipt: evidence::FileProof,
    source_fixtures: Vec<evidence::FileProof>,
    isolated_root: String,
    evidence_root: String,
    pub(crate) elapsed_nanoseconds: u64,
    commands: Vec<Command>,
    http: Vec<Request>,
    files: Vec<evidence::FileProof>,
    pub(crate) cleanup_complete: bool,
    failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    command: Vec<String>,
    observation: process::ProcessObservation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    path: String,
    input: serde_json::Value,
    status: u16,
    body: Vec<u8>,
    data: serde_json::Value,
    revisions: usize,
    head_sha256: String,
}

impl Receipt {
    pub(crate) fn source_receipt_proof(&self) -> &evidence::FileProof {
        &self.source_receipt
    }

    pub(crate) fn command_count(&self) -> u64 {
        self.commands.len() as u64
    }

    pub(crate) fn runner_count(&self) -> u64 {
        self.commands.len() as u64
    }

    pub(crate) fn request_count(&self) -> u64 {
        self.http.len() as u64
    }
}

pub(super) fn command(binary: &Path, source: &Path, output: &Path) -> Result<u8, DevError> {
    let verifier = std::env::current_exe()?;
    let admitted = read_semantic_receipt(source, &verifier)?;
    require(
        output.is_absolute()
            && output
                .parent()
                .is_some_and(|parent| parent.canonicalize().is_ok_and(|real| real == parent))
            && !output
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir)),
        "boundary evidence root must be absent, absolute, and canonical",
    )?;
    fs::create_dir(output)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(output, fs::Permissions::from_mode(0o700))?;
    let root = tempfile::Builder::new()
        .prefix("lkjscript-pure-tail-boundary-")
        .tempdir()?;
    let copied = root.path().join("lkjscript");
    fs::copy(binary, &copied)?;
    let started = Instant::now();
    let mut receipt = Receipt {
        schema: "lkjscript-pure-tail-boundary-1".to_owned(),
        status: "failed".to_owned(),
        candidate_sha256: digest(binary)?,
        copied_candidate_sha256: digest(&copied)?,
        verifier_sha256: digest(&verifier)?,
        source_receipt: admitted.source_receipt.clone(),
        source_fixtures: Vec::new(),
        isolated_root: root.path().display().to_string(),
        evidence_root: output.display().to_string(),
        elapsed_nanoseconds: 0,
        commands: Vec::new(),
        http: Vec::new(),
        files: Vec::new(),
        cleanup_complete: false,
        failure: None,
    };
    let result: Result<(), DevError> = (|| {
        require(
            receipt.candidate_sha256 == receipt.copied_candidate_sha256,
            "candidate copy changed",
        )?;
        for (source_name, name) in FIXTURES {
            let path = admitted.root.join(source_name);
            receipt
                .source_fixtures
                .push(evidence::proof(&path, source_name.to_owned())?);
            fs::copy(path, root.path().join(name))?;
            fs::copy(root.path().join(name), output.join(name))?;
        }
        let spec = spec(
            &copied,
            root.path(),
            output,
            "command",
            vec![
                "run".to_owned(),
                "--deployment".to_owned(),
                root.path()
                    .join("command.deployment.json")
                    .display()
                    .to_string(),
                "--arguments-file".to_owned(),
                root.path()
                    .join("command.arguments.json")
                    .display()
                    .to_string(),
            ],
        );
        let observation = process::run(&spec, output);
        receipt.commands.push(Command {
            command: spec.command,
            observation,
        });
        inspect_command(&receipt.commands[0], output)?;
        http(&copied, root.path(), output, &admitted, &mut receipt)?;
        require(
            digest(binary)? == receipt.candidate_sha256
                && digest(&copied)? == receipt.candidate_sha256,
            "final or copied candidate changed during boundary execution",
        )?;
        require(
            evidence::proof(source, source.canonicalize()?.display().to_string())?
                == admitted.source_receipt,
            "original semantic receipt changed during candidate execution",
        )?;
        read_semantic_receipt(source, &verifier)?;
        Ok(())
    })();
    receipt.elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| DevError::corrupt("boundary elapsed overflow"))?;
    root.close()?;
    receipt.cleanup_complete = !Path::new(&receipt.isolated_root).exists();
    match result {
        Ok(()) => receipt.status = "fresh passed".to_owned(),
        Err(error) => receipt.failure = Some(error.to_string()),
    }
    let mut entries = fs::read_dir(output)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        receipt.files.push(evidence::proof(
            &entry.path(),
            entry.file_name().to_string_lossy().into_owned(),
        )?);
    }
    let proof = evidence::publish_json(&output.join("receipt.json"), &receipt)?;
    let success = receipt.status == "fresh passed" && receipt.cleanup_complete;
    if success {
        read(&output.join("receipt.json"), binary, &verifier)?;
    }
    println!(
        "{}",
        serde_json::json!({"status":if success {"passed"} else {"failed"},
        "classification":receipt.status,"receipt":proof.path,"digest":proof.digest,"failure":receipt.failure})
    );
    Ok(if success { 0 } else { 1 })
}

fn spec(
    binary: &Path,
    root: &Path,
    output: &Path,
    label: &str,
    args: Vec<String>,
) -> process::ProcessSpec {
    let mut command = vec![binary.display().to_string()];
    command.extend(args);
    process::ProcessSpec {
        command,
        cwd: root.to_path_buf(),
        environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT,
        maximum_stderr_bytes: MAXIMUM_OUTPUT,
        stdout_path: output.join(format!("{label}.stdout")),
        stderr_path: output.join(format!("{label}.stderr")),
        unavailable_exit_code: None,
    }
}

fn inspect_command(command: &Command, output: &Path) -> Result<(), DevError> {
    require(
        command.observation.status == process::ProcessStatus::Passed
            && command.observation.exit_code == Some(0)
            && command.observation.signal.is_none(),
        "actual source-free Command process failed",
    )?;
    let records = parse_records(
        "final pure-tail Command",
        &process::read_bounded(
            &output.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT,
        )?,
    )
    .map_err(|error| DevError::corrupt(format!("Command response: {error:?}")))?;
    let artifact = lkjscript::platform::contributor::strict_artifact_identity_probe(
        &process::read_bounded(&output.join("command.lkja"), MAXIMUM_FILES_BYTES)?,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        records.len() == 2
            && records[0].operation == "result"
            && records[1].operation == "execution"
            && field(&records, "result", "status")? == "success"
            && field(&records, "result", "command")? == "run"
            && field(&records, "execution", "value")? == "33558528"
            && field(&records, "execution", "target")? == "sum"
            && field(&records, "execution", "artifact")? == artifact
            && field(&records, "execution", "execution-mode")? == "production"
            && field(&records, "execution", "verification")? == "not-performed",
        "final Command differs from independent sum or claims a source differential",
    )?;
    let production =
        strict_record(field(&records, "execution", "production-observation")?.as_bytes())?;
    let cleanup = strict_record(field(&records, "execution", "cleanup")?.as_bytes())?;
    let executor = strict_record(field(&records, "execution", "executor-observation")?.as_bytes())?;
    require(
        production["maximum_call_depth"]
            .as_u64()
            .is_some_and(|n| n > 0 && n <= 8)
            && production["tail_transfers"].as_u64().is_some_and(|n| n > 0)
            && production["live_call_frames_after"] == 0
            && production["live_locals_after"] == 0
            && production["live_type_bindings_after"] == 0
            && production["live_operands_after"] == 0
            && production["live_transactions_after"] == 0
            && production["live_handles_after"] == 0
            && cleanup["admission_stopped"] == true
            && cleanup["remaining_tasks"] == 0
            && cleanup["cleanup_failures"] == serde_json::json!([])
            && executor["active_dispatches"] == 0
            && executor["remaining_workers"] == 0
            && executor["dispatch_open"] == false,
        "final Command did not prove its tail/frame/actual cleanup boundary",
    )
}

fn scan(store: &DataStore) -> Result<serde_json::Value, DevError> {
    serde_json::to_value(
        store
            .begin()
            .map_err(|error| DevError::corrupt(error.to_string()))?
            .scan(
                "tail",
                &[],
                DataScanDirection::Forward,
                10,
                1_048_576,
                1000,
                None,
            )
            .map_err(|error| DevError::corrupt(error.to_string()))?,
    )
    .map_err(Into::into)
}

fn http(
    binary: &Path,
    root: &Path,
    output: &Path,
    admitted: &AdmittedFixture,
    receipt: &mut Receipt,
) -> Result<(), DevError> {
    let data_root = root.join("data");
    DataStore::initialize(&data_root).map_err(|error| DevError::corrupt(error.to_string()))?;
    let spec = spec(
        binary,
        root,
        output,
        "http",
        vec![
            "serve".to_owned(),
            "--deployment".to_owned(),
            root.join("service.deployment.json").display().to_string(),
        ],
    );
    let command = spec.command.clone();
    let stdout = spec.stdout_path.clone();
    let control = process::ProcessControl::default();
    let child_control = control.clone();
    let evidence_root = output.to_path_buf();
    let (sender, receiver) = std::sync::mpsc::channel();
    let owner = std::thread::Builder::new()
        .name("pure-tail-final-http-observer".to_owned())
        .spawn(move || {
            let _ = sender.send(process::run_controlled(
                &spec,
                &evidence_root,
                &child_control,
            ));
        })?;
    let result = (|| {
        let started = Instant::now();
        let ready: serde_json::Value = loop {
            require(
                started.elapsed() < Duration::from_secs(30) && !owner.is_finished(),
                "final HTTP readiness failed",
            )?;
            if stdout.exists() {
                let bytes = process::read_bounded(&stdout, MAXIMUM_OUTPUT)?;
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    break strict_record(&bytes[..end])?;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let artifact = lkjscript::platform::contributor::strict_artifact_identity_probe(
            &process::read_bounded(&root.join("application.lkja"), MAXIMUM_FILES_BYTES)?,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            ready["event"] == "ready"
                && ready["ok"] == true
                && ready["deployment"]["artifact_digest"] == artifact,
            "final HTTP readiness is foreign to the admitted source fixture",
        )?;
        let address = ready["local_address"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("final listener missing"))?
            .parse()
            .map_err(|_| DevError::corrupt("final listener invalid"))?;
        let store = DataStore::open(&data_root, "tail", DataLimits::default())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            scan(&store)?["items"] == serde_json::json!([]),
            "final HTTP data was not initially empty",
        )?;
        for (path, input) in requests() {
            let response = crate::http_probe::request(
                address,
                "GET",
                &path,
                &serde_json::to_vec(&input)?,
                &[],
            )?;
            receipt.http.push(Request {
                path,
                input,
                status: response.status,
                body: response.body,
                data: scan(&store)?,
                revisions: store
                    .verify()
                    .map_err(|error| DevError::corrupt(error.to_string()))?
                    .revisions,
                head_sha256: digest(&data_root.join("HEAD"))?,
            });
        }
        inspect_http(&receipt.http, admitted)
    })();
    control.interrupt();
    let terminal = receiver.recv_timeout(Duration::from_secs(35)).or_else(|_| {
        control.kill();
        receiver.recv_timeout(Duration::from_secs(5))
    });
    owner
        .join()
        .map_err(|_| DevError::corrupt("final HTTP observer failed"))?;
    let observation = terminal.map_err(|_| DevError::corrupt("final HTTP did not terminate"))?;
    let stopped = observation.status == process::ProcessStatus::Passed;
    receipt.commands.push(Command {
        command,
        observation,
    });
    result?;
    require(stopped, "final HTTP process did not shut down cleanly")
}

fn requests() -> Vec<(String, serde_json::Value)> {
    let values = (0_i64..33).collect::<Vec<_>>();
    let mut trapping = values.clone();
    if let Some(last) = trapping.last_mut() {
        *last = i64::MAX;
    }
    [
        ("success", values.clone()),
        ("trapped", trapping),
        ("recovery", values),
        ("configured", vec![1, 2, 4]),
    ]
    .into_iter()
    .map(|(name, items)| {
        (
            format!("/?{name}"),
            serde_json::json!({"items":items,"scale":3,"bias":5}),
        )
    })
    .collect()
}

fn inspect_http(rows: &[Request], admitted: &AdmittedFixture) -> Result<(), DevError> {
    require(
        rows.len() == 4,
        "final HTTP must retain success/trap/recovery/configuration",
    )?;
    let identities: nominal_data::Identities = serde_json::from_value(
        admitted.receipt.outcomes["nominal_typed_data"]["identities"].clone(),
    )?;
    for (index, ((path, input), row)) in requests().into_iter().zip(rows).enumerate() {
        require(
            row.path == path && row.input == input,
            "final HTTP input matrix changed",
        )?;
        let count = [1, 1, 2, 3][index];
        require(
            row.data["items"]
                .as_array()
                .is_some_and(|items| items.len() == count)
                && row.data["continuation"].is_null()
                && row.revisions == count + 1
                && evidence::hex_identity(&row.head_sha256, "", 64),
            "final HTTP commit inventory differs",
        )?;
        if index == 1 {
            require(
                row.status == 500
                    && row.data == rows[0].data
                    && row.head_sha256 == rows[0].head_sha256,
                "final HTTP trap published a staged write",
            )?;
        } else {
            let expected = if index == 3 {
                vec![8_i64, 11, 17]
            } else {
                (0_i64..33).map(|x| 3 * x + 5).collect()
            };
            require(
                row.status == 200 && row.body == serde_json::to_vec(&expected)?,
                "final HTTP complete body differs",
            )?;
            let expected_keys: &[&str] = match index {
                0 => &["success"],
                2 => &["recovery", "success"],
                _ => &["configured", "recovery", "success"],
            };
            let items = row.data["items"]
                .as_array()
                .ok_or_else(|| DevError::corrupt("final HTTP scan items missing"))?;
            for (item, key) in items.iter().zip(expected_keys) {
                let mapped = if *key == "configured" {
                    vec![8_i64, 11, 17]
                } else {
                    (0_i64..33).map(|x| 3 * x + 5).collect()
                };
                let bytes = nominal_data::expected(&identities, &mapped)?;
                require(
                    item["key"] == serde_json::json!([{"kind":"text","value":key}])
                        && item["value"] == serde_json::json!(bytes),
                    "final HTTP cumulative keyed bytes differ from independent nominal layout/payload",
                )?;
            }
            if index > 0 {
                let previous = rows[index - 1].data["items"]
                    .as_array()
                    .ok_or_else(|| DevError::corrupt("prior HTTP scan items missing"))?;
                require(
                    previous.iter().all(|item| items.contains(item)),
                    "final HTTP recovery/configuration changed a prior committed entry",
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn read(path: &Path, candidate: &Path, verifier: &Path) -> Result<Receipt, DevError> {
    let bytes = process::read_bounded(path, 16 * 1024 * 1024)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("boundary receipt parent missing"))?
        .canonicalize()?;
    require(
        evidence::encode_json(&receipt)? == bytes
            && receipt.schema == "lkjscript-pure-tail-boundary-1"
            && receipt.status == "fresh passed"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && receipt.evidence_root == root.display().to_string()
            && path.canonicalize()? == root.join("receipt.json")
            && !Path::new(&receipt.isolated_root).exists()
            && receipt.elapsed_nanoseconds <= MAXIMUM_SECONDS * 1_000_000_000
            && receipt.candidate_sha256 == digest(candidate)?
            && receipt.copied_candidate_sha256 == receipt.candidate_sha256
            && receipt.verifier_sha256 == digest(verifier)?,
        "final pure-tail receipt/bindings are incomplete",
    )?;
    let source = Path::new(&receipt.source_receipt.path);
    require(
        source.is_absolute()
            && evidence::proof(source, source.canonicalize()?.display().to_string())?
                == receipt.source_receipt,
        "original semantic receipt changed or is absent",
    )?;
    let admitted = read_semantic_receipt(source, verifier)?;
    require(
        receipt.source_fixtures.len() == FIXTURES.len(),
        "final fixture inventory incomplete",
    )?;
    for ((source_name, name), proof) in FIXTURES.into_iter().zip(&receipt.source_fixtures) {
        require(
            *proof == evidence::proof(&admitted.root.join(source_name), source_name.to_owned())?
                && process::read_bounded(&root.join(name), MAXIMUM_FILES_BYTES)?
                    == process::read_bounded(
                        &admitted.root.join(source_name),
                        MAXIMUM_FILES_BYTES,
                    )?,
            "final fixture bytes differ from source authority",
        )?;
    }
    require(
        receipt.commands.len() == 2,
        "final pure-tail must contain exactly two actual candidate children",
    )?;
    let isolated = Path::new(&receipt.isolated_root);
    let expected = [
        vec![
            isolated.join("lkjscript").display().to_string(),
            "run".to_owned(),
            "--deployment".to_owned(),
            isolated
                .join("command.deployment.json")
                .display()
                .to_string(),
            "--arguments-file".to_owned(),
            isolated
                .join("command.arguments.json")
                .display()
                .to_string(),
        ],
        vec![
            isolated.join("lkjscript").display().to_string(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            isolated
                .join("service.deployment.json")
                .display()
                .to_string(),
        ],
    ];
    for (index, command) in receipt.commands.iter().enumerate() {
        evidence::verify_process_files(&root, &command.observation)?;
        require(
            command.command == expected[index]
                && command.observation.status == process::ProcessStatus::Passed
                && command.observation.exit_code == Some(0)
                && command.observation.signal.is_none()
                && command.observation.stdout.path
                    == format!("{}.stdout", ["command", "http"][index])
                && command.observation.stderr.path
                    == format!("{}.stderr", ["command", "http"][index]),
            "final boundary used foreign arguments or did not terminate successfully",
        )?;
    }
    inspect_command(&receipt.commands[0], &root)?;
    inspect_server(&receipt.commands[1], &root)?;
    inspect_http(&receipt.http, &admitted)?;
    let mut actual = fs::read_dir(&root)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    actual.retain(|name| name != "receipt.json");
    actual.sort();
    let expected_names = FIXTURES
        .iter()
        .map(|(_, name)| name.to_string())
        .chain(
            [
                "command.stdout",
                "command.stderr",
                "http.stdout",
                "http.stderr",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .collect::<BTreeSet<_>>();
    require(
        actual
            == receipt
                .files
                .iter()
                .map(|proof| proof.path.clone())
                .collect::<Vec<_>>()
            && actual.iter().cloned().collect::<BTreeSet<_>>() == expected_names,
        "final evidence inventory is not exact",
    )?;
    for proof in &receipt.files {
        require(
            proof.kind == evidence::FileKind::File
                && evidence::proof(&root.join(&proof.path), proof.path.clone())? == *proof,
            "final evidence file changed",
        )?;
    }
    Ok(receipt)
}

fn inspect_server(command: &Command, root: &Path) -> Result<(), DevError> {
    let bytes =
        process::read_bounded(&root.join(&command.observation.stdout.path), MAXIMUM_OUTPUT)?;
    let framed = bytes
        .strip_suffix(b"\n")
        .ok_or_else(|| DevError::corrupt("final HTTP terminal JSONL is truncated"))?;
    let records = framed
        .split(|byte| *byte == b'\n')
        .map(strict_record)
        .collect::<Result<Vec<_>, _>>()?;
    require(
        records.len() == 2,
        "final HTTP must retain exact ready/stopped events",
    )?;
    let artifact = lkjscript::platform::contributor::strict_artifact_identity_probe(
        &process::read_bounded(&root.join("application.lkja"), MAXIMUM_FILES_BYTES)?,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let ready = &records[0];
    let stopped = &records[1];
    require(
        ready["event"] == "ready"
            && ready["ok"] == true
            && ready["deployment"]["artifact_digest"] == artifact
            && stopped["event"] == "stopped"
            && stopped["ok"] == true
            && stopped["receipt"]["shutdown"]["admission_stopped"] == true
            && stopped["receipt"]["shutdown"]["remaining_tasks"] == 0
            && stopped["receipt"]["shutdown"]["cleanup_failures"] == serde_json::json!([])
            && stopped["receipt"]["runtime"]["resident"]["active"] == 0
            && stopped["receipt"]["runtime"]["resident"]["queued"] == 0
            && stopped["receipt"]["runtime"]["admission_permits"] == 0
            && stopped["receipt"]["runtime"]["worker_permits"] == 0
            && stopped["executor"]["active_dispatches"] == 0
            && stopped["executor"]["remaining_workers"] == 0
            && stopped["executor"]["dispatch_open"] == false,
        "final HTTP did not retain its actual artifact-bound shutdown and executor cleanup",
    )
}

fn strict_record(bytes: &[u8]) -> Result<serde_json::Value, DevError> {
    use lkjscript::platform::json::{
        JsonIntegerPolicy, JsonLimits, decode_strict_with_integer_policy,
    };
    decode_strict_with_integer_policy(
        bytes,
        JsonLimits {
            maximum_bytes: MAXIMUM_OUTPUT as usize,
            maximum_string_bytes: MAXIMUM_OUTPUT as usize,
            ..Default::default()
        },
        JsonIntegerPolicy::SignedOrUnsigned64,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every positive receipt and process observation comes from an actual owner run.
    #[test]
    #[ignore = "requires original current source/final receipts and exact candidate/verifier executables"]
    fn actual_final_receipt_rejects_boundary_relabeling_and_semantic_forgery() {
        let fixture = PathBuf::from(
            std::env::var_os("LKJSCRIPT_PURE_TAIL_BOUNDARY_FIXTURE").expect("actual final receipt"),
        );
        let candidate = PathBuf::from(
            std::env::var_os("LKJSCRIPT_PURE_TAIL_BOUNDARY_BINARY").expect("actual candidate"),
        );
        let verifier = PathBuf::from(
            std::env::var_os("LKJSCRIPT_PURE_TAIL_BOUNDARY_VERIFIER").expect("actual verifier"),
        );
        let original =
            read(&fixture, &candidate, &verifier).expect("actual original owner evidence");
        let source = Path::new(&original.source_receipt.path);
        let admitted =
            read_semantic_receipt(source, &verifier).expect("actual full source evidence");
        assert_eq!(admitted.process_count(), 1);
        let template = process::read_bounded(
            &admitted.root.join("http-template.deployment.json"),
            MAXIMUM_OUTPUT,
        )
        .unwrap();
        let descriptor = process::read_bounded(
            &admitted.root.join("standalone.deployment.json"),
            MAXIMUM_OUTPUT,
        )
        .unwrap();
        require_http_fixture(&template, &descriptor)
            .expect("actual source-tested exact HTTP authorities");
        for mutation in ["limit", "grant"] {
            let mut changed: serde_json::Value = serde_json::from_slice(&descriptor).unwrap();
            if mutation == "limit" {
                changed["execution"]["maximum_call_depth"] = serde_json::json!(9);
            } else {
                let mut extra = changed["grants"][1].clone();
                extra["requirement"] = serde_json::json!("additional");
                extra["sharing_domain"] = serde_json::json!("additional");
                extra["authority_revision"] = serde_json::json!("8".repeat(64));
                extra["adapter"]["root"] = serde_json::json!("other-data");
                changed["grants"].as_array_mut().unwrap().push(extra);
            }
            let changed = evidence::encode_json(&changed).unwrap();
            lkjscript::platform::deployment::decode_deployment(&changed)
                .expect("independently valid broadened descriptor");
            assert!(require_http_fixture(&template, &changed).is_err());
        }
        for original in admitted.receipt.commands.iter().filter(|command| {
            command.observation.as_process().is_some()
                || command
                    .command
                    .first()
                    .is_some_and(|name| name == "pure-tail-transaction-probe")
        }) {
            require_positive_probe(original).expect("actual mandatory successful source probe");
            let mut changed = original.clone();
            changed.expected_success = false;
            match &mut changed.observation {
                CallObservation::Process(call) => {
                    call.status = process::ProcessStatus::Failed;
                    call.exit_code = Some(1);
                }
                CallObservation::Semantic(call) => {
                    call.returned_success = false;
                    call.protocol_code = 1;
                }
            }
            changed
                .observation
                .require_outcome(false)
                .expect("consistent deliberately relabeled failure");
            assert!(require_positive_probe(&changed).is_err());
        }

        // Substitute another genuine, independently admissible artifact while
        // refreshing all mutable outcome/file digests. The original actual
        // build response remains the independent binding that must reject it.
        let substituted = tempfile::tempdir().unwrap();
        let mut semantic = admitted.receipt.clone();
        for proof in &semantic.files {
            fs::copy(
                admitted.root.join(&proof.path),
                substituted.path().join(&proof.path),
            )
            .unwrap();
        }
        let foreign = admitted.root.join("clean.lkja");
        lkjscript::platform::contributor::strict_artifact_identity_probe(
            &process::read_bounded(&foreign, MAXIMUM_FILES_BYTES).unwrap(),
        )
        .expect("genuine valid artifact from the pre-edit independent source lane");
        assert_ne!(
            digest(&foreign).unwrap(),
            digest(&admitted.root.join("command.lkja")).unwrap()
        );
        for name in ["command.lkja", "reviewed-clean.lkja", "standalone.lkja"] {
            fs::copy(&foreign, substituted.path().join(name)).unwrap();
            *semantic
                .files
                .iter_mut()
                .find(|proof| proof.path == name)
                .unwrap() =
                evidence::proof(&substituted.path().join(name), name.to_owned()).unwrap();
        }
        semantic.outcomes.insert(
            "reviewed_artifact_sha256".to_owned(),
            serde_json::json!(digest(&foreign).unwrap()),
        );
        semantic.outcomes.get_mut("standalone_http").unwrap()["artifact_sha256"] =
            serde_json::json!(digest(&foreign).unwrap());
        let isolated = Path::new(&semantic.isolated_root);
        assert!(
            require_build_binding(
                &semantic,
                substituted.path(),
                &isolated.join("consumer"),
                &admitted.root.join("reviewed-clean.lkja"),
                "command.lkja"
            )
            .is_err()
        );
        assert!(
            require_build_binding(
                &semantic,
                substituted.path(),
                &isolated.join("http"),
                &isolated.join("standalone/application.lkja"),
                "standalone.lkja"
            )
            .is_err()
        );

        let reject = |mutate: &dyn Fn(&mut Receipt, &Path)| {
            let copied = tempfile::tempdir().expect("isolated mutated receipt");
            let mut receipt = original.clone();
            receipt.evidence_root = copied.path().display().to_string();
            for proof in &original.files {
                fs::copy(
                    fixture.parent().unwrap().join(&proof.path),
                    copied.path().join(&proof.path),
                )
                .unwrap();
            }
            mutate(&mut receipt, copied.path());
            evidence::publish_json(&copied.path().join("receipt.json"), &receipt).unwrap();
            assert!(read(&copied.path().join("receipt.json"), &candidate, &verifier).is_err());
        };
        reject(&|receipt, _| receipt.schema = "lkjscript-pure-tail-semantic-1".to_owned());
        reject(&|receipt, _| {
            receipt.commands.pop();
        });
        reject(&|receipt, _| {
            receipt.commands.push(receipt.commands[0].clone());
        });
        reject(&|receipt, _| receipt.commands[0].command[1] = "pure-tail-probe".to_owned());
        reject(&|receipt, _| receipt.commands[0].observation.exit_code = Some(1));
        reject(&|receipt, _| receipt.commands[0].observation.signal = Some(2));
        reject(&|receipt, _| receipt.source_receipt.bytes = Some(0));
        reject(&|receipt, _| receipt.source_fixtures[0].path = "reviewed-clean.lkja".to_owned());
        reject(&|receipt, _| receipt.http[0].body[0] = b'0');
        reject(&|receipt, _| receipt.http[1].head_sha256 = "0".repeat(64));
        reject(&|receipt, _| receipt.http[2].revisions += 1);
        reject(&|receipt, _| receipt.http[3].data["items"][2]["value"][0] = serde_json::json!(0));
        reject(&|receipt, _| receipt.http[2].data["items"][1]["value"][0] = serde_json::json!(0));
        reject(&|receipt, root| {
            let path = root.join("command.stdout");
            let bytes = process::read_bounded(&path, MAXIMUM_OUTPUT).unwrap();
            let altered = String::from_utf8(bytes)
                .unwrap()
                .replace("33558528", "33558529");
            fs::write(&path, altered).unwrap();
            let proof = evidence::proof(&path, "command.stdout".to_owned()).unwrap();
            receipt.commands[0].observation.stdout = proof.clone();
            *receipt
                .files
                .iter_mut()
                .find(|file| file.path == "command.stdout")
                .unwrap() = proof;
        });
        reject(&|receipt, root| {
            let path = root.join("command.stdout");
            let original =
                String::from_utf8(process::read_bounded(&path, MAXIMUM_OUTPUT).unwrap()).unwrap();
            let altered = original.replace(
                "\\\"live_call_frames_after\\\":0",
                "\\\"live_call_frames_after\\\":1,\\\"live_call_frames_after\\\":0",
            );
            assert_ne!(
                original, altered,
                "mutate the actual embedded VM observation"
            );
            fs::write(&path, altered).unwrap();
            let proof = evidence::proof(&path, "command.stdout".to_owned()).unwrap();
            receipt.commands[0].observation.stdout = proof.clone();
            *receipt
                .files
                .iter_mut()
                .find(|file| file.path == "command.stdout")
                .unwrap() = proof;
        });
        reject(&|receipt, root| {
            let path = root.join("http.stdout");
            let bytes = process::read_bounded(&path, MAXIMUM_OUTPUT).unwrap();
            let mut events = bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
                .collect::<Vec<_>>();
            events[1]["executor"]["remaining_workers"] = serde_json::json!(1);
            let altered = events
                .into_iter()
                .flat_map(|event| {
                    let mut line = serde_json::to_vec(&event).unwrap();
                    line.push(b'\n');
                    line
                })
                .collect::<Vec<_>>();
            fs::write(&path, altered).unwrap();
            let proof = evidence::proof(&path, "http.stdout".to_owned()).unwrap();
            receipt.commands[1].observation.stdout = proof.clone();
            *receipt
                .files
                .iter_mut()
                .find(|file| file.path == "http.stdout")
                .unwrap() = proof;
        });
        for mutation in ["duplicate", "blank", "truncated"] {
            reject(&|receipt, root| {
                let path = root.join("http.stdout");
                let bytes = process::read_bounded(&path, MAXIMUM_OUTPUT).unwrap();
                let altered = match mutation {
                    "duplicate" => String::from_utf8(bytes)
                        .unwrap()
                        .replace(
                            "\"remaining_workers\":0",
                            "\"remaining_workers\":1,\"remaining_workers\":0",
                        )
                        .into_bytes(),
                    "blank" => {
                        let mut changed = bytes;
                        changed.push(b'\n');
                        changed
                    }
                    _ => bytes[..bytes.len() - 1].to_vec(),
                };
                fs::write(&path, altered).unwrap();
                let proof = evidence::proof(&path, "http.stdout".to_owned()).unwrap();
                receipt.commands[1].observation.stdout = proof.clone();
                *receipt
                    .files
                    .iter_mut()
                    .find(|file| file.path == "http.stdout")
                    .unwrap() = proof;
            });
        }
        assert_eq!(
            evidence::proof(source, source.canonicalize().unwrap().display().to_string()).unwrap(),
            original.source_receipt
        );
    }
}
