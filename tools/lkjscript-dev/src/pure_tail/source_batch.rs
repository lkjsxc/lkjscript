//! One real source preparation, with independently checked invocation evidence for every row.

use super::*;

const MAXIMUM_BATCH_BYTES: u64 = 64 * 1024 * 1024;
const REQUEST_FILE: &str = "source-batch.input.jsonl";
const RESULT_FILE: &str = "source-batch.result.jsonl";
const HEAD_FILE: &str = "source-batch.head";
const SMOKE_ARGUMENTS: &str = "[[1,2,4],3,5]";
const SMOKE_EXPECTED: &str = "[8,11,17]";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Evidence {
    command_index: usize,
    smoke_command_index: usize,
    authority_before: authority::AuthorityObservation,
    authority_after: authority::AuthorityObservation,
    head: evidence::FileProof,
    request: evidence::FileProof,
    result: evidence::FileProof,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    id: String,
    exit_code: u8,
    stdout: String,
    result_json: Option<String>,
}

struct Case {
    target: &'static str,
    arguments: String,
    expected: Option<String>,
    annotation: Annotation,
}

#[derive(Clone, Copy)]
enum Annotation {
    None,
    MapSample { n: u64, sample: u64 },
    Forward { n: i64, k: i64 },
    SumSample { n: i64, sample: u64 },
    SumPeak { n: i64 },
}

impl Case {
    fn success(target: &'static str, arguments: &str, expected: &str) -> Self {
        Self {
            target,
            arguments: arguments.to_owned(),
            expected: Some(expected.to_owned()),
            annotation: Annotation::None,
        }
    }
}

// These remain the original complete input/expected matrix, in its original order.
// Neither an observed candidate value nor the reference evaluator supplies an expectation.
fn cases() -> Result<Vec<Case>, DevError> {
    let mut cases = Vec::new();
    for (target, arguments, expected) in [
        ("map", "[[],3,5]", "[]"),
        ("map", "[[1],3,5]", "[8]"),
        ("map", "[[1,2,4],3,5]", "[8,11,17]"),
        ("map", "[[1,2,4],1,0]", "[1,2,4]"),
        (
            "map-aliases",
            "[[1,2,4]]",
            r#"{"left":[8,11,17,99],"mapped":[8,11,17],"original":[1,2,4],"right":[8,11,17,-7]}"#,
        ),
        ("map-types", "[[42,1,42]]", r#"["true","false","true"]"#),
        ("map-composed", "[[42,1,42]]", r#"["true","false","true"]"#),
    ] {
        cases.push(Case::success(target, arguments, expected));
    }
    for n in [256_u64, 1024, 4096, 8192] {
        for sample in 0..4 {
            let input = (0..n).collect::<Vec<_>>();
            let expected = (0..n).map(|x| 3 * x + 5).collect::<Vec<_>>();
            if n == 8192 {
                require(
                    expected.iter().sum::<u64>() == 100691968,
                    "independent map sum",
                )?;
            }
            let mut case = Case::success(
                "map",
                &serde_json::to_string(&serde_json::json!([input, 3, 5]))?,
                &serde_json::to_string(&expected)?,
            );
            case.annotation = Annotation::MapSample { n, sample };
            cases.push(case);
        }
    }
    cases.push(Case {
        target: "map",
        arguments: "[[1,9223372036854775807,4],3,5]".to_owned(),
        expected: None,
        annotation: Annotation::None,
    });
    cases.push(Case::success(
        "configured",
        "[3,5,-2,1,[1,2,4]]",
        r#"{"first":36,"second":-11}"#,
    ));
    for (target, arguments, expected) in [
        (
            "constant-text",
            "[\"runtime text\",[1,2,4]]",
            "[\"runtime text\",\"runtime text\",\"runtime text\"]",
        ),
        ("constant-text", "[\"runtime text\",[]]", "[]"),
        (
            "constant-lists",
            "[[\"first\",\"second\"],[1,2]]",
            "[[\"first\",\"second\"],[\"first\",\"second\"]]",
        ),
        (
            "generic-pair",
            "[{\"scale\":3,\"bias\":5},{\"scale\":-2,\"bias\":4},[1,2,4]]",
            "{\"again\":[8,11,17],\"first\":[8,11,17],\"second\":[2,0,-4]}",
        ),
        ("binding-empty", "[]", "9"),
        ("binding-partial", "[]", "9"),
        ("binding-repeated", "[]", "9"),
        ("binding-complete", "[]", "9"),
        ("binding-factory", "[]", "12"),
        ("compose-forward", "[4,3,5]", "17"),
        ("compose-reverse", "[4,3,5]", "27"),
        ("compose-types", "[42]", "\"true\""),
        ("compose-types", "[1]", "\"false\""),
        ("binding-thunk-created", "[]", "42"),
        ("binding-capture-once", "[]", "9"),
        ("binding-constant", "[]", "9"),
    ] {
        cases.push(Case::success(target, arguments, expected));
    }
    for target in [
        "forward",
        "forward-generic",
        "bound-forward",
        "nominal-forward",
        "nominal-bound",
    ] {
        for n in [1_i64, 256, 4096] {
            for k in [1_i64, 64, 1024] {
                let items = serde_json::json!((1..=n).collect::<Vec<_>>());
                let input = if target.starts_with("nominal-") {
                    serde_json::json!({"revision":7,"items":items})
                } else {
                    items
                };
                let mut case = Case::success(
                    target,
                    &serde_json::to_string(&serde_json::json!([k, input]))?,
                    &n.to_string(),
                );
                case.annotation = Annotation::Forward { n, k };
                cases.push(case);
            }
        }
    }
    for n in [256_i64, 1024, 4096] {
        for sample in 0..4 {
            let mut case = Case::success(
                "sum",
                &serde_json::to_string(&vec![(1..=n).collect::<Vec<_>>()])?,
                &(n * (n + 1) / 2).to_string(),
            );
            case.annotation = Annotation::SumSample { n, sample };
            cases.push(case);
        }
    }
    for n in [0_i64, 1, 256, 1024, 4096, 8192] {
        let mut case = Case::success(
            "sum",
            &serde_json::to_string(&vec![(1..=n).collect::<Vec<_>>()])?,
            &(n * (n + 1) / 2).to_string(),
        );
        case.annotation = Annotation::SumPeak { n };
        cases.push(case);
    }
    for (target, arguments, expected) in [
        ("count", "[8192]", "0"),
        ("even", "[8192]", "true"),
        ("even", "[8191]", "false"),
        ("generic-i64", "[8192,-17,true]", "-17"),
        ("generic-bool", "[8192,true,-17]", "true"),
        ("ordered", "[[1,2,4,8]]", "5"),
        ("ordered", "[[8,4,2,1]]", "-5"),
    ] {
        cases.push(Case::success(target, arguments, expected));
    }
    require(
        cases.len() == 111,
        "pure-tail source matrix must retain all 111 rows",
    )?;
    Ok(cases)
}

fn request_bytes(cases: &[Case]) -> Result<Vec<u8>, DevError> {
    let mut bytes = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        // No execution/deadline overrides: every invocation retains ordinary default limits.
        serde_json::to_writer(
            &mut bytes,
            &serde_json::json!({
                "id":index.to_string(), "target":case.target, "arguments_json":case.arguments,
            }),
        )?;
        bytes.push(b'\n');
    }
    require(
        bytes.len() as u64 <= MAXIMUM_BATCH_BYTES,
        "source batch input exceeds ordinary bound",
    )?;
    Ok(bytes)
}

pub(super) fn execute(context: &mut Context, package: &Package) -> Result<(), DevError> {
    let authority = authority::observe_graph_authority(&package.path)?;
    fs::write(
        context.output.join(HEAD_FILE),
        process::read_bounded(&package.path.join("HEAD"), 1024 * 1024)?,
    )?;
    let smoke_command_index = context.receipt.commands.len();
    // Separately prepare the ordinary public source call after supplier removal.
    context.run(package, "map", SMOKE_ARGUMENTS, SMOKE_EXPECTED)?;
    let cases = cases()?;
    let requests = request_bytes(&cases)?;
    let input = context.output.join(REQUEST_FILE);
    let output = context.output.join(RESULT_FILE);
    fs::write(&input, &requests)?;
    let command_index = context.receipt.commands.len();
    context.cli(
        Some(&package.path),
        &[
            "run",
            "map",
            "--batch-input-file",
            &input.display().to_string(),
            "--batch-result-file",
            &output.display().to_string(),
        ],
        true,
    )?;
    let authority_after = authority::observe_graph_authority(&package.path)?;
    require(
        authority == authority_after
            && process::read_bounded(&input, MAXIMUM_BATCH_BYTES)? == requests,
        "source batch changed accepted authority or exact input bytes",
    )?;
    context.bound()?;
    let evidence = Evidence {
        command_index,
        smoke_command_index,
        authority_before: authority,
        authority_after,
        head: evidence::proof(&context.output.join(HEAD_FILE), HEAD_FILE.to_owned())?,
        request: evidence::proof(&input, REQUEST_FILE.to_owned())?,
        result: evidence::proof(&output, RESULT_FILE.to_owned())?,
    };
    let outcomes = inspect(&context.receipt, &context.output, &evidence)?;
    context.receipt.outcomes.extend(outcomes);
    context.receipt.source_batch = Some(evidence);
    Ok(())
}

pub(super) fn read(receipt: &Receipt, root: &Path) -> Result<(), DevError> {
    let evidence = receipt
        .source_batch
        .as_ref()
        .ok_or_else(|| DevError::corrupt("pure-tail source batch evidence missing"))?;
    let expected = inspect(receipt, root, evidence)?;
    for (key, value) in &expected {
        require(
            receipt.outcomes.get(key) == Some(value),
            "source batch outcome differs from actual row evidence",
        )?;
    }
    require(
        receipt
            .outcomes
            .keys()
            .filter(|key| key.starts_with("batch-"))
            .count()
            == 110,
        "source batch successful outcome inventory is not exact",
    )
}

fn records(bytes: &[u8]) -> Result<Vec<CompactRecord>, DevError> {
    parse_records("pure-tail source batch", bytes)
        .map_err(|errors| DevError::corrupt(format!("source batch projection: {errors:?}")))
}

fn command_records(
    receipt: &Receipt,
    root: &Path,
    index: usize,
) -> Result<Vec<CompactRecord>, DevError> {
    let command = receipt
        .commands
        .get(index)
        .ok_or_else(|| DevError::corrupt("source batch command index absent"))?;
    require(
        command.expected_success
            && command.observation.as_semantic().is_some()
            && command.observation.returned_success()
            && command.observation.reported_code() == Some(0)
            && command.observation.stdout().path == format!("command-{index:04}.stdout"),
        "source batch boundary is not a successful linked public operation",
    )?;
    command.observation.require_outcome(true)?;
    command.observation.require_arguments(&command.command)?;
    records(&process::read_bounded(
        &root.join(&command.observation.stdout().path),
        MAXIMUM_OUTPUT,
    )?)
}

fn inspect(
    receipt: &Receipt,
    root: &Path,
    evidence: &Evidence,
) -> Result<BTreeMap<String, serde_json::Value>, DevError> {
    let isolated = Path::new(&receipt.isolated_root);
    let project = isolated.join("consumer").display().to_string();
    require(
        evidence.smoke_command_index.checked_add(1) == Some(evidence.command_index),
        "source batch must immediately follow its ordinary boundary smoke",
    )?;
    let smoke = command_records(receipt, root, evidence.smoke_command_index)?;
    let summary = command_records(receipt, root, evidence.command_index)?;
    require(
        receipt.commands[evidence.smoke_command_index].command
            == [
                "--project".to_owned(),
                project.clone(),
                "run".to_owned(),
                "map".to_owned(),
                "--arguments".to_owned(),
                SMOKE_ARGUMENTS.to_owned(),
            ]
            && receipt.commands[evidence.command_index].command
                == [
                    "--project".to_owned(),
                    project,
                    "run".to_owned(),
                    "map".to_owned(),
                    "--batch-input-file".to_owned(),
                    root.join(REQUEST_FILE).display().to_string(),
                    "--batch-result-file".to_owned(),
                    root.join(RESULT_FILE).display().to_string(),
                ],
        "source batch or ordinary smoke used foreign public operation arguments",
    )?;
    require(
        field(&smoke, "result", "status")? == "success"
            && field(&smoke, "result", "command")? == "run"
            && field(&smoke, "execution", "target")? == "map"
            && field(&smoke, "execution", "value")? == SMOKE_EXPECTED
            && field(&smoke, "execution", "differential")? == "equal"
            && receipt
                .outcomes
                .get(&format!("run-{}-map", evidence.smoke_command_index + 1))
                == Some(&execution_outcome(
                    &smoke,
                    SMOKE_EXPECTED,
                    &evidence.authority_before,
                )?),
        "ordinary source boundary smoke differs from its independent result or recorded work",
    )?;
    require(
        evidence.authority_before == evidence.authority_after
            && evidence.head.path == HEAD_FILE
            && evidence.head.kind == evidence::FileKind::File
            && evidence::proof(&root.join(HEAD_FILE), HEAD_FILE.to_owned())? == evidence.head
            && digest(&root.join(HEAD_FILE))? == evidence.authority_before.head_sha256
            && evidence.request.path == REQUEST_FILE
            && evidence.result.path == RESULT_FILE
            && evidence.request.kind == evidence::FileKind::File
            && evidence.result.kind == evidence::FileKind::File
            && evidence::proof(&root.join(REQUEST_FILE), REQUEST_FILE.to_owned())?
                == evidence.request
            && evidence::proof(&root.join(RESULT_FILE), RESULT_FILE.to_owned())? == evidence.result,
        "source batch input/output file bindings changed",
    )?;
    let cases = cases()?;
    require(
        process::read_bounded(&root.join(REQUEST_FILE), MAXIMUM_BATCH_BYTES)?
            == request_bytes(&cases)?,
        "source batch request differs from the complete fixed independent input matrix",
    )?;
    let bytes = process::read_bounded(&root.join(RESULT_FILE), MAXIMUM_BATCH_BYTES)?;
    let rows = decode_responses(&bytes)?;
    // Bind the shared preparation to the already required clean public build, admitted afresh.
    let artifact_bytes = process::read_bounded(&root.join("clean.lkja"), MAXIMUM_FILES_BYTES)?;
    let bundle = lkjscript::platform::contributor::strict_artifact_identity_probe(&artifact_bytes)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        field(&summary, "artifact", "bundle")? == bundle
            && field(&summary, "artifact", "bytes")? == artifact_bytes.len().to_string(),
        "source batch preparation differs from the independently admitted clean artifact",
    )?;
    require(
        evidence.authority_before.files > 0
            && evidence.authority_before.bytes > 0
            && evidence::hex_identity(&evidence.authority_before.head_sha256, "", 64)
            && evidence::hex_identity(&evidence.authority_before.inventory_sha256, "", 64),
        "source batch accepted authority inventory missing",
    )?;
    require(
        field(&summary, "result", "status")? == "success"
            && field(&summary, "result", "command")? == "run.batch"
            && field(&summary, "batch", "mode")? == "source-differential"
            && field(&summary, "batch", "source-preparations")? == "1"
            && field(&summary, "batch", "rows")? == "111"
            && field(&summary, "batch", "passed")? == "110"
            && field(&summary, "batch", "failed")? == "1"
            && field(&summary, "batch", "schema")?
                == lkjscript::platform::canonical_schema::SCHEMA_EPOCH.to_string()
            && field(&summary, "batch", "validator")?
                == lkjscript::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            && evidence::hex_identity(
                &field(&summary, "batch", "package-revision")?,
                "package_revision_",
                64,
            )
            && field(&summary, "batch", "result-file")?
                == root.join(RESULT_FILE).display().to_string()
            && field(&summary, "output", "path")? == root.join(RESULT_FILE).display().to_string()
            && field(&summary, "output", "bytes")? == bytes.len().to_string()
            && field(&summary, "output", "visibility")? == "created"
            && field(&summary, "output", "durability")? == "synchronized"
            && field(&summary, "output", "stage-cleanup")? == "removed",
        "source batch aggregate is incomplete or uses foreign source/validator identity",
    )?;
    require_binding(&smoke, &summary)?;
    inspect_rows(&cases, &rows, &smoke, evidence)
}

fn decode_responses(bytes: &[u8]) -> Result<Vec<Response>, DevError> {
    require(
        bytes.last() == Some(&b'\n'),
        "source batch final row is incomplete",
    )?;
    let mut rows = Vec::new();
    for line in bytes[..bytes.len() - 1].split(|byte| *byte == b'\n') {
        require(
            rows.len() < 111 && line.len() as u64 <= MAXIMUM_OUTPUT,
            "source batch row inventory exceeded bounds",
        )?;
        let row: Response = serde_json::from_slice(line)?;
        require(
            serde_json::to_vec(&row)? == line,
            "source batch row is noncanonical or has duplicate fields",
        )?;
        rows.push(row);
    }
    require(
        rows.len() == 111,
        "source batch complete row inventory missing",
    )?;
    Ok(rows)
}

fn require_binding(expected: &[CompactRecord], actual: &[CompactRecord]) -> Result<(), DevError> {
    for (operation, names) in [
        (
            "authority",
            &["repository", "package", "revision", "state"][..],
        ),
        (
            "artifact",
            &["manifest", "bundle", "bytes", "packages", "closure-objects"][..],
        ),
        (
            "native-package-plan",
            &[
                "artifact-sha256",
                "input-sha256",
                "outcome-sha256",
                "order-sha256",
                "packages",
                "edges",
            ][..],
        ),
    ] {
        for name in names {
            require(
                field(expected, operation, name)? == field(actual, operation, name)?,
                "source batch row changed exact source/artifact binding",
            )?;
        }
    }
    Ok(())
}

fn inspect_rows(
    cases: &[Case],
    rows: &[Response],
    smoke: &[CompactRecord],
    evidence: &Evidence,
) -> Result<BTreeMap<String, serde_json::Value>, DevError> {
    require(
        rows.len() == cases.len(),
        "source batch complete row inventory missing",
    )?;
    let mut outcomes = BTreeMap::new();
    let mut matrices: BTreeMap<&str, Vec<MatrixCell>> = BTreeMap::new();
    let mut peak = None;
    for (index, (case, row)) in cases.iter().zip(rows).enumerate() {
        require(
            row.id == index.to_string(),
            "source batch row is missing, reordered or duplicated",
        )?;
        let records = records(row.stdout.as_bytes())?;
        let Some(expected) = &case.expected else {
            require(
                row.exit_code == 2
                    && row.result_json.is_none()
                    && field(&records, "result", "status")? == "failure"
                    && field(&records, "result", "command")? == "run"
                    && field(&records, "diagnostic", "class")? == "semantic"
                    && field(&records, "diagnostic", "code")? == "normalized_integer_overflow"
                    && !records.iter().any(|record| record.operation == "execution"),
                "mapping overflow installed a partial result or changed its refusal",
            )?;
            require_failure_cleanup(&records)?;
            continue;
        };
        require(
            row.exit_code == 0
                && row.result_json.as_ref() == Some(expected)
                && field(&records, "result", "status")? == "success"
                && field(&records, "result", "command")? == "run"
                && field(&records, "execution", "target")? == case.target
                && field(&records, "execution", "differential")? == "equal"
                && field(&records, "execution", "value-location")? == "batch-row"
                && !records.iter().any(|record| {
                    record
                        .fields
                        .iter()
                        .any(|field| matches!(field.name.as_str(), "value" | "result-file"))
                }),
            "source batch target or complete independent result differs",
        )?;
        require_binding(smoke, &records)?;
        require_success_cleanup(&records)?;
        let mut outcome = execution_outcome(&records, expected, &evidence.authority_before)?;
        match case.annotation {
            Annotation::None => {}
            Annotation::MapSample { n, sample } => {
                outcome["items"] = serde_json::json!(n);
                outcome["sample"] = serde_json::json!(sample);
                outcome["warmup"] = serde_json::json!(sample == 0);
                for tier in ["production", "reference"] {
                    let work: ValueWork =
                        serde_json::from_value(outcome[format!("{tier}_value_work")].clone())?;
                    verify_mapping_collection_items(&outcome, tier, n)?;
                    require(
                        work.internal_guard_descendant_visits == 0
                            && work.lists.full_materializations == 0
                            && work.lists.materialized_elements == 0
                            && work.lists.element_handle_allocations == n
                            && work.lists.element_handle_copies <= 31 * n
                            && work.lists.element_slots_reserved == 32 * n
                            && work.lists.branch_slots_reserved <= 128 * (n / 32)
                            && work.lists.node_visits <= 12 * n,
                        "persistent mapping exceeded derived physical work bounds",
                    )?;
                }
            }
            Annotation::Forward { n, k } => {
                matrices.entry(case.target).or_default().push(MatrixCell {
                    n,
                    k,
                    production: serde_json::from_value(outcome["production_value_work"].clone())?,
                    reference: serde_json::from_value(outcome["reference_value_work"].clone())?,
                    preparation: serde_json::from_value(outcome["preparation"].clone())?,
                });
            }
            Annotation::SumSample { n, sample } => {
                outcome["items"] = serde_json::json!(n);
                outcome["sample"] = serde_json::json!(sample);
                outcome["warmup"] = serde_json::json!(sample == 0);
            }
            Annotation::SumPeak { n } if n >= 256 => {
                let observed = (
                    integer_field(&records, "production-peak-call-frames")?,
                    integer_field(&records, "reference-peak-call-frames")?,
                );
                require(
                    peak.is_none_or(|previous| previous == observed),
                    "live frame peak grows with tail-chain length",
                )?;
                peak = Some(observed);
            }
            Annotation::SumPeak { .. } => {}
        }
        outcomes.insert(
            format!("batch-{}-{index}-{}", evidence.command_index, case.target),
            outcome,
        );
    }
    for (target, matrix) in matrices {
        verify_matrix(&matrix)?;
        outcomes.insert(
            format!("checked-value-{target}-matrix"),
            serde_json::to_value(matrix)?,
        );
    }
    outcomes.insert(
        "persistent-map".to_owned(),
        serde_json::json!({
            "classification":"fresh passed", "large_length":8192, "large_sum":100691968,
            "configured":[8,11,17], "aliases_unchanged":true, "producer_removed":true,
            "generic_heterogeneous_composition":true, "overflow_no_partial_result":true,
            "samples_per_size":4, "sizes":[256,1024,4096,8192],
        }),
    );
    Ok(outcomes)
}

fn observation(records: &[CompactRecord], name: &str) -> Result<serde_json::Value, DevError> {
    strict_json(&field(records, "execution", name)?)
}

fn strict_json(json: &str) -> Result<serde_json::Value, DevError> {
    lkjscript::platform::json::decode_strict_with_integer_policy(
        json.as_bytes(),
        lkjscript::platform::json::JsonLimits::default(),
        lkjscript::platform::json::JsonIntegerPolicy::SignedOrUnsigned64,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))
}

fn require_quiescent(value: &serde_json::Value, reference: bool) -> Result<(), DevError> {
    let names: &[&str] = if reference {
        &[
            "live_call_frames_after",
            "live_control_frames_after",
            "live_local_scopes_after",
            "live_type_scopes_after",
            "live_effect_scopes_after",
            "live_allowances_after",
            "live_transactions_after",
            "live_handles_after",
        ]
    } else {
        &[
            "live_call_frames_after",
            "live_locals_after",
            "live_type_bindings_after",
            "live_operands_after",
            "live_transactions_after",
            "live_handles_after",
        ]
    };
    require(
        value.is_object()
            && value["capability_calls"] == 0
            && names.iter().all(|name| value[name] == 0),
        "source batch invocation retained resources or invoked a capability",
    )
}

fn require_executor(value: &serde_json::Value) -> Result<(), DevError> {
    require(
        value["dispatch_open"] == false
            && value["active_dispatches"] == 0
            && value["remaining_workers"] == 0,
        "source batch invocation executor was not joined",
    )
}

fn require_success_cleanup(records: &[CompactRecord]) -> Result<(), DevError> {
    for (tier, reference, counter) in [
        ("production", false, "instructions"),
        ("reference", true, "expressions"),
    ] {
        let value = observation(records, &format!("{tier}-observation"))?;
        require_quiescent(&value, reference)?;
        require(
            value[counter].as_u64().is_some_and(|work| work > 0)
                && value[counter] == integer_field(records, &format!("{tier}-{counter}"))?
                && value["maximum_call_depth"]
                    == integer_field(records, &format!("{tier}-peak-call-frames"))?
                && value["tail_transfers"]
                    == integer_field(records, &format!("{tier}-tail-transfers"))?
                && value["type_derivation_steps"]
                    == integer_field(records, &format!("{tier}-type-derivation-steps"))?
                && value["type_metadata_bytes"]
                    == integer_field(records, &format!("{tier}-type-metadata-bytes"))?
                && value["allocated_bytes"]
                    == integer_field(records, &format!("{tier}-allocated-bytes"))?
                && value["allocation_charges"]
                    == integer_field(records, &format!("{tier}-allocation-charges"))?
                && value["collection_items"]
                    == integer_field(records, &format!("{tier}-collection-items"))?,
            "source batch compact counters differ from actual evaluator observations",
        )?;
        require_subset(
            &serde_json::to_value(value_work(records, tier)?)?,
            &value["value_work"],
        )?;
    }
    require_executor(&observation(records, "executor-observation")?)
}

fn require_subset(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
) -> Result<(), DevError> {
    if let Some(fields) = expected.as_object() {
        require(
            actual.is_object(),
            "source batch actual work observation missing",
        )?;
        for (name, value) in fields {
            require_subset(value, &actual[name])?;
        }
        Ok(())
    } else {
        require(
            expected == actual,
            "source batch compact work differs from actual evaluator observation",
        )
    }
}

fn require_failure_cleanup(records: &[CompactRecord]) -> Result<(), DevError> {
    let notes: Vec<String> = serde_json::from_str(&field(records, "diagnostic", "notes")?)?;
    let mut seen = BTreeSet::new();
    for note in notes {
        if let Some(json) = note.strip_prefix("production-observation: ") {
            require(seen.insert("production"), "duplicate failed VM observation")?;
            let value = strict_json(json)?;
            require_quiescent(&value, false)?;
            require(
                value["instructions"].as_u64().is_some_and(|work| work > 0),
                "mapping overflow did not execute the actual VM",
            )?;
        } else if let Some(json) = note.strip_prefix("executor-observation: ") {
            require(
                seen.insert("executor"),
                "duplicate failed executor observation",
            )?;
            require_executor(&strict_json(json)?)?;
        } else {
            return Err(DevError::corrupt(
                "mapping overflow retained an unexpected or uninvoked evaluator observation",
            ));
        }
    }
    require(
        seen == BTreeSet::from(["production", "executor"]),
        "mapping overflow omitted actual VM/executor cleanup",
    )
}

#[cfg(test)]
#[path = "source_batch_tests.rs"]
mod tests;
