//! Actual candidate source batches. Rows are execution evidence, never process observations.
use super::*;
use serde_json::Value;

const MAXIMUM_BATCH_BYTES: u64 = 64 * 1024 * 1024;

#[cfg(test)]
#[path = "source_batch_tests.rs"]
mod tests;

pub(super) struct Case {
    pub id: String,
    pub target: String,
    pub arguments: String,
    pub expected: Result<Value, String>,
}

impl Case {
    pub fn success(id: &str, target: &str, arguments: Value, expected: Value) -> Self {
        Self {
            id: id.to_owned(),
            target: target.to_owned(),
            arguments: arguments.to_string(),
            expected: Ok(expected),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    schema: String,
    command: usize,
    smoke: usize,
    inventory: usize,
    input: evidence::FileProof,
    output: evidence::FileProof,
    head: evidence::FileProof,
    authority_before: crate::authority::AuthorityObservation,
    authority_after: crate::authority::AuthorityObservation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    id: String,
    exit_code: u8,
    stdout: String,
    result_json: Value,
}

pub(super) struct Row {
    pub records: Vec<CompactRecord>,
    pub result: Option<Value>,
    pub result_json: Option<String>,
}

pub(super) struct Validated {
    pub rows: Vec<Row>,
    pub command: usize,
    pub smoke: usize,
    pub smoke_records: Vec<CompactRecord>,
    pub package: String,
    pub revision: String,
    pub bundle: String,
    pub inventory: usize,
}

fn filename(label: &str, suffix: &str) -> String {
    format!("source-batch-{label}.{suffix}")
}

fn requests(cases: &[Case]) -> Result<Vec<u8>, DevError> {
    let mut ids = std::collections::BTreeSet::new();
    let mut bytes = Vec::new();
    for case in cases {
        require(ids.insert(&case.id), "duplicate source batch case identity")?;
        serde_json::to_writer(
            &mut bytes,
            &serde_json::json!({"id":case.id,"target":case.target,"arguments_json":case.arguments}),
        )?;
        bytes.push(b'\n');
    }
    require(!cases.is_empty(), "source batch cases missing")?;
    Ok(bytes)
}

pub(super) fn execute(
    context: &mut Context,
    package: &Package,
    label: &str,
    artifact: &str,
    cases: &[Case],
    smoke: &Case,
) -> Result<Validated, DevError> {
    require(smoke.expected.is_ok(), "source batch smoke must succeed")?;
    let before = crate::authority::observe_graph_authority(&package.path)?;
    let head_name = filename(label, "head");
    fs::copy(package.path.join("HEAD"), context.evidence.join(&head_name))?;
    let smoke_index = context.receipt.commands.len();
    context.cli(
        Some(&package.path),
        &["run", &smoke.target, "--arguments", &smoke.arguments],
        true,
    )?;
    let input_name = filename(label, "input.jsonl");
    let output_name = filename(label, "result.jsonl");
    let input = context.root.join(&input_name);
    let output = context.root.join(&output_name);
    require(
        !input.exists() && !output.exists(),
        "source batch evidence already exists",
    )?;
    fs::write(&input, requests(cases)?)?;
    let command = context.receipt.commands.len();
    context.cli(
        Some(&package.path),
        &[
            "run",
            &smoke.target,
            "--batch-input-file",
            &input.display().to_string(),
            "--batch-result-file",
            &output.display().to_string(),
        ],
        true,
    )?;
    let retained_input = context.evidence.join(&input_name);
    let retained_output = context.evidence.join(&output_name);
    fs::copy(&input, &retained_input)?;
    fs::copy(&output, &retained_output)?;
    let inventory = context
        .receipt
        .producer_inventories
        .iter()
        .position(|source| {
            source.package == package.id && source.semantic_revision == package.revision
        })
        .ok_or_else(|| DevError::corrupt("source batch accepted export missing"))?;
    let witness = Witness {
        schema: "lkjscript-offline-source-batch-1".to_owned(),
        command,
        smoke: smoke_index,
        inventory,
        input: evidence::proof(&retained_input, input_name)?,
        output: evidence::proof(&retained_output, output_name)?,
        head: evidence::proof(&context.evidence.join(&head_name), head_name)?,
        authority_before: before,
        authority_after: crate::authority::observe_graph_authority(&package.path)?,
    };
    require(
        context
            .receipt
            .observations
            .insert(filename(label, "witness"), serde_json::to_string(&witness)?)
            .is_none(),
        "duplicate source batch witness",
    )?;
    read(
        &context.receipt,
        &context.evidence,
        label,
        &package.path,
        artifact,
        cases,
        smoke,
    )
}

pub(super) fn read(
    receipt: &Receipt,
    root: &Path,
    label: &str,
    project: &Path,
    artifact: &str,
    cases: &[Case],
    smoke: &Case,
) -> Result<Validated, DevError> {
    let witness: Witness = serde_json::from_str(
        receipt
            .observations
            .get(&filename(label, "witness"))
            .ok_or_else(|| DevError::corrupt("source batch witness missing"))?,
    )?;
    require(
        witness.schema == "lkjscript-offline-source-batch-1",
        "source batch witness schema differs",
    )?;
    for (proof, suffix) in [
        (&witness.input, "input.jsonl"),
        (&witness.output, "result.jsonl"),
        (&witness.head, "head"),
    ] {
        require(
            proof.path == filename(label, suffix)
                && evidence::proof(&root.join(&proof.path), proof.path.clone())? == *proof,
            "source batch retained file changed",
        )?;
    }
    require(
        witness.authority_before == witness.authority_after
            && witness.authority_before.files > 0
            && witness.authority_before.bytes > 0
            && evidence::hex_identity(&witness.authority_before.inventory_sha256, "", 64)
            && digest_file(&root.join(&witness.head.path), 1024 * 1024)?
                == witness.authority_before.head_sha256,
        "source batch changed accepted authority",
    )?;
    require(
        process::read_bounded(&root.join(&witness.input.path), MAXIMUM_BATCH_BYTES)?
            == requests(cases)?,
        "source batch literal requests omitted, reordered or substituted",
    )?;
    require(
        witness.smoke < witness.command,
        "source batch ordinary boundary ordering changed",
    )?;
    let executable = Path::new(&receipt.isolated_root)
        .join("lkjscript")
        .display()
        .to_string();
    let prefix = vec![
        executable,
        "--project".to_owned(),
        project.display().to_string(),
        "run".to_owned(),
        smoke.target.clone(),
    ];
    let mut expected_smoke = prefix.clone();
    expected_smoke.extend(["--arguments".into(), smoke.arguments.clone()]);
    let smoke_records = command_output(receipt, root, witness.smoke, expected_smoke)?;
    validate_success(&smoke_records, &smoke.target)?;
    require(
        smoke.expected.as_ref().ok()
            == Some(&decode_result(
                field(&smoke_records, "execution", "value")?.as_bytes(),
            )?),
        "source batch ordinary smoke full result differs",
    )?;
    for counter in ["production-instructions", "reference-expressions"] {
        require(
            field(&smoke_records, "execution", counter)?
                .parse::<u64>()
                .is_ok_and(|count| count > 0),
            "source batch ordinary smoke did no independent tier work",
        )?;
    }
    let mut expected_command = prefix;
    expected_command.extend([
        "--batch-input-file".into(),
        Path::new(&receipt.isolated_root)
            .join(&witness.input.path)
            .display()
            .to_string(),
        "--batch-result-file".into(),
        Path::new(&receipt.isolated_root)
            .join(&witness.output.path)
            .display()
            .to_string(),
    ]);
    let summary = command_output(receipt, root, witness.command, expected_command)?;
    let source = receipt
        .producer_inventories
        .get(witness.inventory)
        .ok_or_else(|| DevError::corrupt("source batch inventory missing"))?;
    let inventory = receipt
        .inventories
        .get(witness.inventory)
        .ok_or_else(|| DevError::corrupt("source batch transport inventory missing"))?;
    verify_producer_inventory(source, inventory)?;
    let transport = receipt
        .transport_digests
        .get(witness.inventory)
        .ok_or_else(|| DevError::corrupt("source batch transport digest missing"))?;
    let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
        &process::read_bounded(&root.join(artifact), MAXIMUM_CONTAINER_BYTES)?,
        &process::read_bounded(
            &root.join(format!("transport-{}.lkjp", witness.inventory + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?,
        transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        bound["package"] == source.package && bound["revision"] == source.semantic_revision,
        "source batch retained artifact differs from accepted export",
    )?;
    let bundle = bound["bundle"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("source batch bundle missing"))?;
    let binding = Binding::new(&smoke_records)?;
    binding.require(&summary)?;
    require(
        field(&summary, "result", "status")? == "success"
            && field(&summary, "result", "command")? == "run.batch"
            && field(&summary, "batch", "mode")? == "source-differential"
            && field(&summary, "batch", "source-preparations")? == "1"
            && field(&summary, "batch", "package-revision")?
                == bound["package_revision"].as_str().unwrap_or("")
            && field(&summary, "authority", "package")? == source.package
            && field(&summary, "authority", "revision")? == source.semantic_revision
            && field(&summary, "artifact", "bundle")? == bundle
            && field(&summary, "batch", "schema")?
                == lkjscript::platform::canonical_schema::CANONICAL_SCHEMA_HASH_HEX
            && field(&summary, "batch", "validator")?
                == lkjscript::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
        "source batch summary source or preparation binding differs",
    )?;
    let bytes = process::read_bounded(&root.join(&witness.output.path), MAXIMUM_BATCH_BYTES)?;
    let passed = cases.iter().filter(|case| case.expected.is_ok()).count();
    require(
        field(&summary, "batch", "rows")? == cases.len().to_string()
            && field(&summary, "batch", "passed")? == passed.to_string()
            && field(&summary, "batch", "failed")? == (cases.len() - passed).to_string()
            && field(&summary, "batch", "result-file")?
                == Path::new(&receipt.isolated_root)
                    .join(&witness.output.path)
                    .display()
                    .to_string()
            && field(&summary, "output", "path")?
                == Path::new(&receipt.isolated_root)
                    .join(&witness.output.path)
                    .display()
                    .to_string()
            && field(&summary, "output", "bytes")? == bytes.len().to_string()
            && field(&summary, "output", "visibility")? == "created"
            && field(&summary, "output", "durability")? == "synchronized"
            && field(&summary, "output", "stage-cleanup")? == "removed",
        "source batch summary row inventory differs",
    )?;
    let rows = rows(&bytes, cases, &binding)?;
    Ok(Validated {
        rows,
        command: witness.command,
        smoke: witness.smoke,
        smoke_records,
        package: source.package.clone(),
        revision: source.semantic_revision.clone(),
        bundle: bundle.to_owned(),
        inventory: witness.inventory,
    })
}

fn command_output(
    receipt: &Receipt,
    root: &Path,
    index: usize,
    expected: Vec<String>,
) -> Result<Vec<CompactRecord>, DevError> {
    let command = receipt
        .commands
        .get(index)
        .ok_or_else(|| DevError::corrupt("source batch actual command missing"))?;
    require(
        command.command == expected
            && command.cwd == receipt.isolated_root
            && command.expects_success
            && command.observation.status == process::ProcessStatus::Passed
            && command.observation.exit_code == Some(0)
            && command.observation.signal.is_none(),
        "source batch actual candidate command differs",
    )?;
    parse_records(
        "source-batch-process",
        &process::read_bounded(
            &root.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|error| DevError::corrupt(format!("source batch process output: {error:?}")))
}

struct Binding(BTreeMap<(&'static str, &'static str), String>);
impl Binding {
    fn new(records: &[CompactRecord]) -> Result<Self, DevError> {
        let mut values = BTreeMap::new();
        for (operation, keys) in [
            (
                "authority",
                &["repository", "package", "revision", "state"][..],
            ),
            (
                "artifact",
                &["manifest", "bundle", "bytes", "packages", "closure-objects"][..],
            ),
        ] {
            for key in keys {
                values.insert((operation, *key), field(records, operation, key)?);
            }
        }
        Ok(Self(values))
    }
    fn require(&self, records: &[CompactRecord]) -> Result<(), DevError> {
        for ((operation, key), expected) in &self.0 {
            require(
                field(records, operation, key)? == *expected,
                "source batch row selected a foreign source or artifact",
            )?;
        }
        Ok(())
    }
}

fn validate_success(records: &[CompactRecord], target: &str) -> Result<(), DevError> {
    require(
        field(records, "result", "status")? == "success"
            && field(records, "result", "command")? == "run"
            && field(records, "execution", "target")? == target
            && field(records, "execution", "differential")? == "equal",
        "source batch row result, exact target or differential differs",
    )
}

fn rows(bytes: &[u8], cases: &[Case], binding: &Binding) -> Result<Vec<Row>, DevError> {
    require(
        bytes.last() == Some(&b'\n'),
        "source batch complete row framing missing",
    )?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| DevError::corrupt("source batch output is not UTF-8"))?;
    let responses = text
        .lines()
        .map(serde_json::from_str::<Response>)
        .collect::<Result<Vec<_>, _>>()?;
    require(
        responses.len() == cases.len(),
        "source batch complete row inventory differs",
    )?;
    responses
        .into_iter()
        .zip(cases)
        .map(|(row, case)| {
            require(
                row.id == case.id,
                "source batch row omitted, duplicated or reordered",
            )?;
            let records =
                parse_records("source-batch-row", row.stdout.as_bytes()).map_err(|error| {
                    DevError::corrupt(format!("source batch row output: {error:?}"))
                })?;
            let result = match &case.expected {
                Ok(expected) => {
                    require(row.exit_code == 0, "source batch success row failed")?;
                    validate_success(&records, &case.target)?;
                    binding.require(&records)?;
                    observations(&records)?;
                    require(
                        field(&records, "execution", "value-location")? == "batch-row",
                        "source batch complete value location differs",
                    )?;
                    require(
                        records
                            .iter()
                            .filter(|record| record.operation == "execution")
                            .flat_map(|record| &record.fields)
                            .all(|field| !matches!(field.name.as_str(), "value" | "result-file")),
                        "source batch row invented inline value or result file",
                    )?;
                    let value = decode_result(
                        row.result_json
                            .as_str()
                            .ok_or_else(|| {
                                DevError::corrupt("source batch complete result missing")
                            })?
                            .as_bytes(),
                    )?;
                    require(
                        &value == expected,
                        "source batch full independent expected result differs",
                    )?;
                    Some(value)
                }
                Err(code) => {
                    require(
                        row.exit_code == 2
                            && row.result_json.is_null()
                            && field(&records, "result", "status")? == "failure"
                            && field(&records, "result", "command")? == "run"
                            && field(&records, "diagnostic", "class")? == "semantic"
                            && field(&records, "diagnostic", "code")? == *code
                            && records.iter().all(|record| record.operation != "execution"),
                        "source batch expected trap differs",
                    )?;
                    failure_observations(&records)?;
                    None
                }
            };
            Ok(Row {
                records,
                result,
                result_json: row.result_json.as_str().map(str::to_owned),
            })
        })
        .collect()
}

fn decode_result(bytes: &[u8]) -> Result<Value, DevError> {
    lkjscript::platform::json::decode_application(
        bytes,
        lkjscript::platform::json::JsonLimits {
            maximum_bytes: MAXIMUM_BATCH_BYTES as usize,
            maximum_string_bytes: MAXIMUM_BATCH_BYTES as usize,
            ..Default::default()
        },
    )
    .map_err(|error| DevError::corrupt(error.to_string()))
}

fn observation(value: &Value, reference: bool) -> Result<(), DevError> {
    let fields = if reference {
        &[
            "live_call_frames_after",
            "live_control_frames_after",
            "live_local_scopes_after",
            "live_type_scopes_after",
            "live_effect_scopes_after",
            "live_allowances_after",
            "live_transactions_after",
            "live_handles_after",
        ][..]
    } else {
        &[
            "live_call_frames_after",
            "live_locals_after",
            "live_type_bindings_after",
            "live_operands_after",
            "live_transactions_after",
            "live_handles_after",
        ][..]
    };
    require(
        fields.iter().all(|key| value[*key] == 0) && value["capability_calls"] == 0,
        "source batch VM/reference resources not released",
    )
}

fn executor(value: &Value) -> Result<(), DevError> {
    require(
        value["dispatch_open"] == false
            && value["active_dispatches"] == 0
            && value["remaining_workers"] == 0,
        "source batch executor resources not joined",
    )
}

fn observations(records: &[CompactRecord]) -> Result<(), DevError> {
    for (name, reference, counter) in [
        ("production-observation", false, "instructions"),
        ("reference-observation", true, "expressions"),
    ] {
        let value = decode_result(field(records, "execution", name)?.as_bytes())?;
        observation(&value, reference)?;
        let count = value[counter]
            .as_u64()
            .ok_or_else(|| DevError::corrupt("source batch tier counter is not unsigned"))?;
        require(count > 0, "source batch tier did no independent work")?;
        require(
            field(
                records,
                "execution",
                if reference {
                    "reference-expressions"
                } else {
                    "production-instructions"
                },
            )? == count.to_string(),
            "source batch tier counter differs from actual observation",
        )?;
    }
    executor(&decode_result(
        field(records, "execution", "executor-observation")?.as_bytes(),
    )?)
}

fn failure_observations(records: &[CompactRecord]) -> Result<(), DevError> {
    let notes: Vec<String> = serde_json::from_str(&field(records, "diagnostic", "notes")?)?;
    let mut names = std::collections::BTreeSet::new();
    for note in notes {
        if let Some(json) = note.strip_prefix("production-observation: ") {
            require(
                names.insert("production-observation"),
                "source batch duplicated failure observation",
            )?;
            let actual = decode_result(json.as_bytes())?;
            observation(&actual, false)?;
            require(
                actual["instructions"]
                    .as_u64()
                    .is_some_and(|count| count > 0),
                "source batch trap did not invoke the actual VM",
            )?;
        } else if let Some(json) = note.strip_prefix("executor-observation: ") {
            require(
                names.insert("executor-observation"),
                "source batch duplicated executor observation",
            )?;
            executor(&decode_result(json.as_bytes())?)?;
        } else {
            return Err(DevError::corrupt(
                "source batch trap retained unexpected or uninvoked tier notes",
            ));
        }
    }
    require(
        names
            == std::collections::BTreeSet::from(["production-observation", "executor-observation"]),
        "source batch trap omitted actual VM/executor cleanup",
    )
}
