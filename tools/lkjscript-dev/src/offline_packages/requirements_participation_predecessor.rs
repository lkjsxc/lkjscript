//! Authentic official v0.1.40 exact-closure and raw-participation compatibility.
use super::*;

const MATERIAL: &str = "requirement-participation-predecessor";
const TRANSPORT: &str =
    "package_transport_4c3f2f8387ee3ed16864681d951fc35317179d81d895bd60a43f1e9b603eed56";
const ARTIFACT: &str =
    "artifact_bundle_4659d83df8f33b7eacd596328fccfdf6ab1893ad399c2e82582d1aca37ed2ea2";
const NEW_GUARD_ARTIFACT: &str =
    "artifact_bundle_fa1f2ff01c98c9a72b423192010956d9b7770921eda4c7d73f7c605500e9fb11";
const PACKAGE: &str = "pkg_7c6771ecd2421f731a15aae77e081a05";
const REVISION: &str = "rev_ee3cf2ae2f8bbf35a9fc0bc8341f9a3766221983ab3b39b3d983394b4a427c5e";
const LOGICAL: &str =
    "package_revision_e36a4694f1c83ca086d39df692f4898637f974bf150f038619a2258f42e87f95";

macro_rules! fixture {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/fixtures/cell-participation-predecessor/",
                $name
            )) as &[u8],
        )
    };
}

const FILES: &[(&str, &[u8])] = &[
    fixture!("predecessor.lkja"),
    fixture!("predecessor.lkjp"),
    fixture!("producer.request"),
    fixture!("consumer.request"),
    fixture!("consumer-native-rejected.request"),
    fixture!("consumer-native-rejection.stdout"),
    fixture!("producer-native-draft-rejection.stdout"),
    fixture!("official-nested-owner.stdout"),
    fixture!("official-raw-owner.stdout"),
    fixture!("official-read-credit.stdout"),
    fixture!("official-store-verify.stdout"),
    fixture!("read-credit.deployment.json"),
    fixture!("expected.json"),
    fixture!("provenance.json"),
    fixture!("new-guard.lkja"),
    fixture!("new-guard.request"),
    fixture!("new-guard-provenance.json"),
    fixture!("official-new-guard.deployment.json"),
    fixture!("official-new-guard-preflight.stdout"),
    fixture!("official-new-guard-preflight.exit"),
];

fn material_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{MATERIAL}--{name}"))
}

fn descriptor(target: &str) -> Result<Value, DevError> {
    let mut value: Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/cell-participation-predecessor/read-credit.deployment.json"
    )))?;
    value["artifact"] = json!("predecessor.lkja");
    value["target"] = json!(target);
    Ok(value)
}

fn cases() -> [(&'static str, Option<Value>); 5] {
    [
        ("read-credit", Some(json!([]))),
        ("nested-owner", None),
        ("read-credit", Some(json!([]))),
        ("raw-owner", Some(json!({"case":"Committed","value":60}))),
        ("read-credit", Some(json!([60]))),
    ]
}

fn observe(root: &Path) -> Result<Value, DevError> {
    let store = DataStore::open(
        &root.join("store"),
        "predecessor-cells",
        DataLimits::default(),
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let transaction = store
        .begin()
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let key = DataKey::new(vec![DataKeyPart::Text("credit".into())], store.limits())
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let entry = transaction
        .get("baseline", &key)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    // A fresh read-only snapshot on every observation; the observer never commits.
    Ok(json!({
        "head":fs::read(root.join("store/HEAD"))?,
        "revision":transaction.base_revision(),
        "entry":entry.map(|entry| json!({"bytes":entry.value,"revision":entry.revision}))
    }))
}

fn expected_states(rows: &[Value]) -> Result<(), DevError> {
    require(
        rows.len() == 6
            && rows.iter().all(|row| {
                revision_identity(&row["revision"])
                    && byte_array(&row["head"], None)
                    && retained_cell(&row["entry"])
            }),
        "participation predecessor independent store observations are incomplete",
    )?;
    require(
        rows[0]["entry"].is_null()
            && rows[..4].iter().all(|row| row == &rows[0])
            && rows[4] == rows[5]
            && rows[0]["head"] != rows[4]["head"]
            && rows[0]["revision"] != rows[4]["revision"]
            && rows[4]["entry"]["bytes"] == json!(typed_bytes(&json!(60))?),
        "old raw participation or nested-owner suppression changed data or HEAD",
    )
}

fn run_result(
    output: &[CompactRecord],
    target: &str,
    expected: Option<Value>,
) -> Result<(), DevError> {
    if let Some(expected) = expected {
        require(
            serde_json::from_str::<Value>(&field(output, "execution", "value")?)? == expected
                && field(output, "execution", "artifact")? == ARTIFACT
                && field(output, "execution", "package")? == PACKAGE
                && field(output, "execution", "revision")? == REVISION
                && field(output, "execution", "execution-mode")? == "production"
                && field(output, "execution", "verification")? == "not-performed",
            "authentic participation artifact result or exact closure changed",
        )?;
        if target == "raw-owner" {
            let counters: Value =
                serde_json::from_str(&field(output, "execution", "production-observation")?)?;
            require(
                counters["capability_calls"] == 5
                    && counters["maximum_live_transactions"] == 1
                    && counters["live_transactions_after"] == 0,
                "old raw helpers changed transaction ownership or ordinary accounting",
            )?;
        }
    } else {
        require(
            field(output, "diagnostic", "class")? == "infrastructure"
                && field(output, "diagnostic", "code")? == "normalized_transaction_nested",
            "old same-canonical owner reentry lost its classified rejection",
        )?;
    }
    Ok(())
}

fn verify_result(output: &[CompactRecord]) -> Result<(), DevError> {
    require(
        field(output, "data", "revisions")? == "2"
            && field(output, "data", "objects")? == "2"
            && field(output, "data", "records")? == "1"
            && field(output, "data", "schemas")? == "0"
            && field(output, "data", "staging-leftovers")? == "0",
        "old two-helper participation did not publish exactly one physical revision",
    )
}

pub(super) fn workflow(context: &mut Context) -> Result<(), DevError> {
    let root = context.root.join(MATERIAL);
    fs::create_dir(&root)?;
    for (name, bytes) in FILES {
        fs::write(root.join(name), bytes)?;
        fs::write(material_path(&context.evidence, name), bytes)?;
    }
    let importer = context.new_package("participation-predecessor-importer")?;
    let stage = context.receipt.commands.len();
    context.cli(
        Some(&importer.path),
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            TRANSPORT,
            "--input-file",
            &root.join("predecessor.lkjp").display().to_string(),
        ],
        true,
    )?;
    let initialize = context.receipt.commands.len();
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &root.join("store").display().to_string(),
        ],
        true,
    )?;
    let mut states = vec![observe(&root)?];
    let mut commands = Vec::new();
    for (ordinal, (target, expected)) in cases().into_iter().enumerate() {
        let name = format!("run-{ordinal}-{target}.deployment.json");
        let path = root.join(&name);
        let bytes = evidence::encode_json(&descriptor(target)?)?;
        fs::write(&path, &bytes)?;
        fs::write(material_path(&context.evidence, &name), bytes)?;
        commands.push(context.receipt.commands.len());
        let output = context.cli(
            None,
            &[
                "run",
                "--deployment",
                &path.display().to_string(),
                "--arguments",
                "[]",
            ],
            expected.is_some(),
        )?;
        run_result(&output, target, expected)?;
        states.push(observe(&root)?);
    }
    expected_states(&states)?;
    fs::write(
        material_path(&context.evidence, "states.json"),
        evidence::encode_json(&states)?,
    )?;
    let verify = context.receipt.commands.len();
    let output = context.cli(
        None,
        &[
            "data",
            "verify",
            "--root",
            &root.join("store").display().to_string(),
        ],
        true,
    )?;
    verify_result(&output)?;
    context.receipt.observations.insert(
        MATERIAL.into(),
        json!({"stage":stage,"initialize":initialize,"commands":commands,"verify":verify})
            .to_string(),
    );
    Ok(())
}

fn read_output(
    receipt: &Receipt,
    root: &Path,
    index: usize,
) -> Result<Vec<CompactRecord>, DevError> {
    require(
        receipt.commands.get(index).is_some(),
        "participation predecessor command evidence omitted",
    )?;
    parse_records(
        "participation-predecessor-output",
        &process::read_bounded(
            &root.join(format!("command-{index:04}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("participation predecessor compact output"))
}

fn original_old_runtime_preflight(root: &Path) -> Result<(), DevError> {
    // Retained original official-v0.1.40 evidence, not a fresh execution by the current candidate.
    // Admission binds its exact new artifact separately from the classified old-runtime output.
    let artifact = process::read_bounded(
        &material_path(root, "new-guard.lkja"),
        MAXIMUM_CONTAINER_BYTES,
    )?;
    require(
        lkjscript::platform::contributor::strict_artifact_identity_probe(&artifact)
            .map_err(|error| DevError::corrupt(error.to_string()))?
            == NEW_GUARD_ARTIFACT,
        "old-runtime new-operation preflight artifact identity changed",
    )?;
    let output = parse_records(
        "official-new-guard-preflight",
        &process::read_bounded(
            &material_path(root, "official-new-guard-preflight.stdout"),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("official old-runtime preflight output"))?;
    require(
        field(&output, "diagnostic", "class")? == "capability"
            && field(&output, "diagnostic", "code")? == "normalized_data_operation"
            && field(&output, "diagnostic", "message")?
                == "first-party data adapter does not implement exact operation 'require-transaction'"
            && process::read_bounded(
                &material_path(root, "official-new-guard-preflight.exit"),
                MAXIMUM_OUTPUT_BYTES,
            )? == b"3\n",
        "official old runtime no longer retains its before-secret unsupported-operation rejection",
    )
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let value: Value =
        serde_json::from_str(receipt.observations.get(MATERIAL).ok_or_else(|| {
            DevError::corrupt("authentic v0.1.40 participation evidence missing")
        })?)?;
    for (name, bytes) in FILES {
        require(
            process::read_bounded(&material_path(root, name), MAXIMUM_CONTAINER_BYTES)? == *bytes,
            "authentic participation predecessor bytes changed",
        )?;
    }
    original_old_runtime_preflight(root)?;
    require(
        lkjscript::platform::contributor::strict_artifact_identity_probe(&process::read_bounded(
            &material_path(root, "predecessor.lkja"),
            MAXIMUM_CONTAINER_BYTES,
        )?)
        .map_err(|error| DevError::corrupt(error.to_string()))?
            == ARTIFACT,
        "authentic participation artifact identity changed",
    )?;
    let states: Vec<Value> = serde_json::from_slice(&process::read_bounded(
        &material_path(root, "states.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    expected_states(&states)?;
    let commands: Vec<usize> = serde_json::from_value(value["commands"].clone())?;
    let stage: usize = serde_json::from_value(value["stage"].clone())?;
    let initialize: usize = serde_json::from_value(value["initialize"].clone())?;
    let verify: usize = serde_json::from_value(value["verify"].clone())?;
    let all_commands: Vec<usize> = [stage, initialize]
        .into_iter()
        .chain(commands.iter().copied())
        .chain([verify])
        .collect();
    require(
        commands.len() == cases().len() && all_commands.windows(2).all(|pair| pair[0] < pair[1]),
        "participation predecessor execution sequence omitted or reordered",
    )?;
    let isolated = Path::new(&receipt.isolated_root);
    let material = isolated.join(MATERIAL);
    for (index, arguments) in [
        (
            stage,
            vec![
                "--project".into(),
                isolated
                    .join("participation-predecessor-importer")
                    .display()
                    .to_string(),
                "package".into(),
                "dependency".into(),
                "stage".into(),
                "--transport".into(),
                TRANSPORT.into(),
                "--input-file".into(),
                material.join("predecessor.lkjp").display().to_string(),
            ],
        ),
        (
            initialize,
            vec![
                "data".into(),
                "initialize".into(),
                "--root".into(),
                material.join("store").display().to_string(),
            ],
        ),
        (
            verify,
            vec![
                "data".into(),
                "verify".into(),
                "--root".into(),
                material.join("store").display().to_string(),
            ],
        ),
    ] {
        let command = receipt
            .commands
            .get(index)
            .ok_or_else(|| DevError::corrupt("participation predecessor setup command absent"))?;
        require(
            command.expects_success
                && command.command
                    == std::iter::once(isolated.join("lkjscript").display().to_string())
                        .chain(arguments)
                        .collect::<Vec<_>>(),
            "participation predecessor transport or data lifecycle invocation changed",
        )?;
    }
    let stage_output = read_output(receipt, root, stage)?;
    require(
        field(&stage_output, "package", "id")? == PACKAGE
            && field(&stage_output, "package", "revision")? == REVISION
            && field(&stage_output, "package", "package-revision")? == LOGICAL
            && field(&stage_output, "package", "transport")? == TRANSPORT
            && field(&stage_output, "authority", "semantic-head-changed")? == "false",
        "old exact dependency closure staging changed identity or accepted authority",
    )?;
    verify_result(&read_output(receipt, root, verify)?)?;
    for (ordinal, (index, (target, expected))) in commands.iter().zip(cases()).enumerate() {
        let name = format!("run-{ordinal}-{target}.deployment.json");
        require(
            serde_json::from_slice::<Value>(&process::read_bounded(
                &material_path(root, &name),
                MAXIMUM_OUTPUT_BYTES,
            )?)? == descriptor(target)?,
            "participation predecessor deployment changed its target or exact grant",
        )?;
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("participation predecessor execution omitted"))?;
        require(
            command.expects_success == expected.is_some()
                && (expected.is_some() || command.observation.exit_code == Some(6))
                && command.command
                    == [
                        receipt.pinned_runtime_path.clone(),
                        "run".into(),
                        "--deployment".into(),
                        material.join(name).display().to_string(),
                        "--arguments".into(),
                        "[]".into(),
                    ],
            "participation predecessor invocation changed runtime, artifact or arguments",
        )?;
        run_result(&read_output(receipt, root, *index)?, target, expected)?;
    }
    Ok(commands)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "adversarial copies of authentic bounded predecessor evidence"
)]
mod tests {
    use super::*;

    fn copy_originals(root: &Path) {
        for (name, bytes) in FILES {
            fs::write(material_path(root, name), bytes).unwrap();
        }
    }

    #[test]
    fn old_runtime_preflight_rejects_artifact_substitution_and_later_secret_failure() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        copy_originals(root);
        original_old_runtime_preflight(root).unwrap();
        fs::copy(
            material_path(root, "predecessor.lkja"),
            material_path(root, "new-guard.lkja"),
        )
        .unwrap();
        assert!(original_old_runtime_preflight(root).is_err());
        copy_originals(root);
        fs::write(
            material_path(root, "official-new-guard-preflight.stdout"),
            b"result status=failure command=run\ndiagnostic class=capability code=secret_missing message=missing\n",
        )
        .unwrap();
        assert!(original_old_runtime_preflight(root).is_err());
    }

    #[test]
    #[ignore = "requires an authentic retained requirement participation receipt"]
    fn participation_reader_requires_originals_and_exact_execution_paths() {
        let path = PathBuf::from(std::env::var_os("LKJSCRIPT_REQUIREMENT_READER_RECEIPT").unwrap());
        let root = path.parent().unwrap();
        let receipt: Receipt = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let commands = validate(&receipt, root).unwrap();
        let mut omitted = receipt.clone();
        omitted.observations.remove(MATERIAL);
        assert!(validate(&omitted, root).is_err());
        let value: Value = serde_json::from_str(&receipt.observations[MATERIAL]).unwrap();
        let stage: usize = serde_json::from_value(value["stage"].clone()).unwrap();
        for index in std::iter::once(stage).chain(commands.iter().copied()) {
            let mut substituted = receipt.clone();
            substituted.commands[index].command[0] = "/unrelated/runtime".into();
            assert!(validate(&substituted, root).is_err());
        }
        let temporary = tempfile::tempdir().unwrap();
        copy_originals(temporary.path());
        fs::copy(
            material_path(root, "states.json"),
            material_path(temporary.path(), "states.json"),
        )
        .unwrap();
        let verify: usize = serde_json::from_value(value["verify"].clone()).unwrap();
        for index in [stage, verify].into_iter().chain(commands) {
            let name = format!("command-{index:04}.stdout");
            fs::copy(root.join(&name), temporary.path().join(name)).unwrap();
        }
        for (ordinal, (target, _)) in cases().iter().enumerate() {
            let name = format!("run-{ordinal}-{target}.deployment.json");
            fs::copy(
                material_path(root, &name),
                material_path(temporary.path(), &name),
            )
            .unwrap();
        }
        validate(&receipt, temporary.path()).unwrap();
        fs::remove_file(material_path(
            temporary.path(),
            "official-new-guard-preflight.stdout",
        ))
        .unwrap();
        assert!(validate(&receipt, temporary.path()).is_err());
    }
}
