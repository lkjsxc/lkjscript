//! Foreground cells in the existing transported effect-library acceptance child.
use super::*;
use lkjscript::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ForegroundReceipt {
    pub schema: String,
    pub consumers: Vec<ConsumerEvidence>,
    pub cells: Vec<Cell>,
    pub loops: Vec<LoopCell>,
    pub failures: Vec<FailureCell>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LoopCell {
    label: String,
    command: usize,
    n: i64,
    bounded: bool,
    output: Value,
    execution: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailureCell {
    label: String,
    command: usize,
    before: i64,
    after: i64,
    code: String,
    notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsumerEvidence {
    package: String,
    revision: String,
    artifact_sha256: String,
    changed_revision: Option<String>,
    changed_artifact_sha256: Option<String>,
    checkout: String,
    bundle: String,
    cwd: String,
    checkout_unavailable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cell {
    label: String,
    consumer: usize,
    command: usize,
    n: i64,
    position: i64,
    executions: i64,
    output: Value,
    persisted_report: Value,
    ordered_trace: Vec<i64>,
    execution: BTreeMap<String, String>,
}

pub(super) struct Consumer {
    package: Package,
    recovery: PathBuf,
    bundle: PathBuf,
    cwd: PathBuf,
    descriptor: Value,
}

pub(super) fn prepare(
    context: &mut Context,
    standard: &Package,
    library: &Package,
    names: &BTreeMap<String, String>,
) -> Result<Vec<Consumer>, DevError> {
    context.receipt.effects.foreground.schema = "lkjscript-foreground-1".into();
    let mut consumers = Vec::new();
    for index in 0..2 {
        let name = format!("foreground-{index}");
        let mut package = context.new_package(&name)?;
        context.stage(&package, standard)?;
        context.stage(&package, library)?;
        context.apply(
            &mut package,
            &format!(
                "{}{}{}{}",
                binding("add", standard),
                binding("add", library),
                module(),
                super::foreground_program::consumer(&standard.symbols, names)
            ),
        )?;
        context.cli(Some(&package.path), &["check"], true)?;
        context.export(&mut package)?;
        let bundle = context.root.join(format!("{name}-bundle"));
        let cwd = context.root.join(format!("unrelated-{index}"));
        fs::create_dir(&bundle)?;
        fs::create_dir(&cwd)?;
        let artifact = bundle.join("application.lkja");
        context.cli(
            Some(&package.path),
            &["build", "--output", &artifact.display().to_string()],
            true,
        )?;
        fs::copy(
            &artifact,
            context.evidence.join(format!("{name}-original.lkja")),
        )?;
        let descriptor = json!({
            "artifact":"application.lkja","target":"main","listen":null,"http":null,"session":null,"worker":null,
            "streams":lkjscript::platform::stream::StreamLimits::default(),
            "configuration":{"stride":{"kind":"i64","value":1},"offset":{"kind":"i64","value":index}},"secrets":[],
            "grants":[
                {"requirement":"config","sharing_domain":format!("foreground-{index}-config"),"authority_revision":"81".repeat(32),"adapter":{"kind":"configuration"}},
                {"requirement":"data","sharing_domain":format!("foreground-{index}-data"),"authority_revision":"82".repeat(32),"adapter":{"kind":"data","root":"data","namespace":"foreground","limits":DataLimits{maximum_live_transactions:1,..Default::default()}}}
            ]
        });
        fs::write(
            bundle.join("command.deployment.json"),
            evidence::encode_json(&descriptor)?,
        )?;
        fs::write(
            context.evidence.join(format!("{name}.deployment.json")),
            evidence::encode_json(&descriptor)?,
        )?;
        context.cli(
            None,
            &[
                "data",
                "initialize",
                "--root",
                &bundle.join("data").display().to_string(),
            ],
            true,
        )?;
        let recovery = context.root.join(format!("{name}-authoring-recovery"));
        fs::rename(&package.path, &recovery)?;
        context
            .receipt
            .effects
            .foreground
            .consumers
            .push(ConsumerEvidence {
                package: package.id.clone(),
                revision: package.revision.clone(),
                artifact_sha256: digest_file(&artifact, MAXIMUM_CONTAINER_BYTES)?,
                changed_revision: None,
                changed_artifact_sha256: None,
                checkout: package.path.display().to_string(),
                bundle: bundle.display().to_string(),
                cwd: cwd.display().to_string(),
                checkout_unavailable: !package.path.exists(),
            });
        consumers.push(Consumer {
            package,
            recovery,
            bundle,
            cwd,
            descriptor,
        });
    }
    Ok(consumers)
}

fn read(
    transaction: &lkjscript::platform::data::DataTransaction,
    name: &str,
    numbers: &[i64],
) -> Result<Value, DevError> {
    let mut parts = vec![DataKeyPart::Text(name.to_owned())];
    parts.extend(numbers.iter().copied().map(DataKeyPart::I64));
    let key = DataKey::new(parts, &DataLimits::default())
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    let entry = transaction
        .get("foreground", &key)
        .map_err(|e| DevError::corrupt(e.to_string()))?
        .ok_or_else(|| DevError::corrupt(format!("missing foreground data {name} {numbers:?}")))?;
    Ok(serde_json::from_slice(&entry.value)?)
}

fn expected(position: i64, executions: i64) -> Value {
    let sum = position * (position - 1) / 2;
    json!({"executions":executions,"result":{"position":position,"total":sum},"payload":{"case":"children","value":[{"case":"item","value":{"value":sum}}]}})
}

fn run(
    context: &mut Context,
    consumer: &Consumer,
    index: usize,
    label: &str,
    n: i64,
    position: i64,
    executions: i64,
) -> Result<(), DevError> {
    require(
        !consumer.package.path.exists(),
        "foreground authoring checkout still available",
    )?;
    let command = context.receipt.commands.len();
    let output = context.cli_at(
        &consumer.cwd,
        None,
        &[
            "run",
            "--deployment",
            &consumer
                .bundle
                .join("command.deployment.json")
                .display()
                .to_string(),
            "--arguments",
            &format!("[{n}]"),
        ],
        true,
    )?;
    let record = output
        .iter()
        .find(|r| r.operation == "execution")
        .ok_or_else(|| DevError::corrupt("foreground execution receipt absent"))?;
    let execution = record
        .fields
        .iter()
        .map(|f| (f.name.clone(), f.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let value: Value = serde_json::from_str(&record_field(record, "value")?)?;
    require(
        value == expected(position, executions),
        "foreground arithmetic, recursive payload or once-only counter differs",
    )?;
    let store = DataStore::open(
        &consumer.bundle.join("data"),
        "foreground",
        DataLimits::default(),
    )
    .map_err(|e| DevError::corrupt(e.to_string()))?;
    let transaction = store
        .begin()
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    let persisted_report = read(&transaction, "report", &[])?;
    require(
        persisted_report == value && read(&transaction, "executions", &[])? == executions,
        "foreground persisted counter/report differs",
    )?;
    let trace_count = (position + 1).min(258);
    require(
        read(&transaction, "tick", &[])? == trace_count,
        "foreground trace call count differs",
    )?;
    let ordered_trace = (0..trace_count)
        .map(|i| {
            read(&transaction, "trace", &[executions, i]).and_then(|v| {
                v.as_i64()
                    .ok_or_else(|| DevError::corrupt("non-integer foreground trace"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    require(
        ordered_trace == (0..trace_count).collect::<Vec<_>>(),
        "foreground callback order differs",
    )?;
    let cell = Cell {
        label: label.into(),
        consumer: index,
        command,
        n,
        position,
        executions,
        output: value,
        persisted_report,
        ordered_trace,
        execution,
    };
    validate_cell(&cell)?;
    context.receipt.effects.foreground.cells.push(cell);
    Ok(())
}

pub(super) fn initial(context: &mut Context, consumers: &[Consumer]) -> Result<(), DevError> {
    for (execution, n) in [0, 1, 257, 8193].into_iter().enumerate() {
        run(
            context,
            &consumers[0],
            0,
            &format!("initial-{n}"),
            n,
            n,
            execution as i64 + 1,
        )?;
    }
    run(context, &consumers[1], 1, "independent-root", 1, 2, 1)?;
    loops(context, &consumers[0])
}

// Observed calibration, retained under campaign 202609121842: N=0 used 44 production
// instructions, N=1000 used 32044. The smallest N above the former 10M cutoff is 312499.
// This scalar nominal state retains no growing list or capabilities; I64 sum remains exact.
const LONG_LOOP: i64 = 312_499;

fn loops(context: &mut Context, consumer: &Consumer) -> Result<(), DevError> {
    let mut descriptor = consumer.descriptor.clone();
    descriptor["target"] = json!("loop");
    descriptor["configuration"] = json!({});
    descriptor["grants"] = json!([]);
    let path = consumer.bundle.join("loop.deployment.json");
    for (label, n, bounded) in [
        ("calibration-zero", 0, false),
        ("calibration-thousand", 1000, false),
        ("past-ten-million", LONG_LOOP, false),
        ("bounded-equivalent", 1000, true),
    ] {
        if bounded {
            descriptor["execution"] = json!(lkjscript::platform::execution::RunPolicy::default());
        }
        fs::write(&path, evidence::encode_json(&descriptor)?)?;
        let command = context.receipt.commands.len();
        let records = context.cli_at(
            &consumer.cwd,
            None,
            &[
                "run",
                "--deployment",
                &path.display().to_string(),
                "--arguments",
                &format!("[{n}]"),
            ],
            true,
        )?;
        let record = records
            .iter()
            .find(|r| r.operation == "execution")
            .ok_or_else(|| DevError::corrupt("loop output missing"))?;
        let output = serde_json::from_str(&record_field(record, "value")?)?;
        let execution = record
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.value.clone()))
            .collect();
        let cell = LoopCell {
            label: label.into(),
            command,
            n,
            bounded,
            output,
            execution,
        };
        validate_loop(&cell)?;
        context.receipt.effects.foreground.loops.push(cell);
    }
    Ok(())
}

fn validate_loop(cell: &LoopCell) -> Result<(), DevError> {
    require(
        cell.output == json!({"position":cell.n,"total":cell.n*(cell.n-1)/2}),
        "empty-row foreground loop result differs",
    )?;
    let production: Value = serde_json::from_str(
        cell.execution
            .get("production-observation")
            .ok_or_else(|| DevError::corrupt("loop production observation absent"))?,
    )?;
    require(
        production["instructions"] == 44 + 32 * cell.n
            && production["capability_calls"] == 0
            && production["maximum_call_depth"] == 3
            && production["live_handles_after"] == 0
            && production["live_transactions_after"] == 0,
        "loop fixed-state baseline, absent quota or cleanup differs",
    )?;
    for (key, value) in [
        ("execution-mode", "production"),
        ("verification", "not-performed"),
        (
            "execution-profile",
            if cell.bounded {
                "bounded"
            } else {
                "trusted-foreground"
            },
        ),
        (
            "instruction-limit",
            if cell.bounded { "10000000" } else { "absent" },
        ),
        (
            "allocation-limit",
            if cell.bounded { "268435456" } else { "absent" },
        ),
        (
            "collection-limit",
            if cell.bounded { "1000000" } else { "absent" },
        ),
        (
            "capability-call-limit",
            if cell.bounded { "100000" } else { "absent" },
        ),
    ] {
        require(
            cell.execution.get(key).map(String::as_str) == Some(value),
            "loop effective policy differs",
        )?;
    }
    let cleanup: Value = serde_json::from_str(&cell.execution["cleanup"])?;
    require(
        cleanup["remaining_tasks"] == 0
            && cleanup["cleanup_failures"] == json!([])
            && cleanup["admission_stopped"] == true,
        "loop cleanup incomplete",
    )
}

pub(super) fn replace(
    context: &mut Context,
    consumers: &mut [Consumer],
    library: &Package,
) -> Result<(), DevError> {
    let consumer = &mut consumers[0];
    fs::rename(&consumer.recovery, &consumer.package.path)?;
    context.stage(&consumer.package, library)?;
    context.apply(&mut consumer.package, &binding("replace", library))?;
    context.cli(Some(&consumer.package.path), &["check"], true)?;
    context.export(&mut consumer.package)?;
    let artifact = consumer.bundle.join("successor.lkja");
    context.cli(
        Some(&consumer.package.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    fs::copy(
        &artifact,
        context.evidence.join("foreground-0-successor.lkja"),
    )?;
    fs::rename(&consumer.package.path, &consumer.recovery)?;
    let receipt = &mut context.receipt.effects.foreground.consumers[0];
    receipt.changed_revision = Some(consumer.package.revision.clone());
    receipt.changed_artifact_sha256 = Some(digest_file(&artifact, MAXIMUM_CONTAINER_BYTES)?);
    Ok(())
}

pub(super) fn recovery(context: &mut Context, consumers: &mut [Consumer]) -> Result<(), DevError> {
    let consumer = &mut consumers[0];
    consumer.descriptor["artifact"] = json!("successor.lkja");
    fs::write(
        consumer.bundle.join("command.deployment.json"),
        evidence::encode_json(&consumer.descriptor)?,
    )?;
    run(context, consumer, 0, "changed-callback", 257, 258, 5)?;
    consumer.descriptor["artifact"] = json!("application.lkja");
    fs::write(
        consumer.bundle.join("command.deployment.json"),
        evidence::encode_json(&consumer.descriptor)?,
    )?;
    run(context, consumer, 0, "retained-bundle", 257, 257, 6)?;
    lifecycle(context, &consumers[1])?;
    fs::write(
        context.evidence.join("foreground.json"),
        evidence::encode_json(&context.receipt.effects.foreground)?,
    )?;
    Ok(())
}

fn counter(consumer: &Consumer) -> Result<i64, DevError> {
    let store = DataStore::open(
        &consumer.bundle.join("data"),
        "foreground",
        DataLimits::default(),
    )
    .map_err(|e| DevError::corrupt(e.to_string()))?;
    let tx = store
        .begin()
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    read(&tx, "executions", &[])?
        .as_i64()
        .ok_or_else(|| DevError::corrupt("execution counter is not an integer"))
}

#[allow(clippy::too_many_arguments)] // One explicit cell binds outcome, persistence and signal policy.
fn failure(
    context: &mut Context,
    consumer: &Consumer,
    label: &str,
    descriptor: &Value,
    arguments: &str,
    expected_code: &str,
    expected_after: i64,
    invoked: bool,
    signal: Option<bool>,
) -> Result<(), DevError> {
    let path = consumer.bundle.join("failure.deployment.json");
    fs::write(&path, evidence::encode_json(descriptor)?)?;
    let before = counter(consumer)?;
    let command = context.receipt.commands.len();
    let arguments = [
        "run",
        "--deployment",
        &path.display().to_string(),
        "--arguments",
        arguments,
    ];
    let records = if let Some(terminate) = signal {
        let specification = process::ProcessSpec {
            command: std::iter::once(context.binary.display().to_string())
                .chain(arguments.iter().map(|s| (*s).to_owned()))
                .collect(),
            cwd: consumer.cwd.clone(),
            environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
            timeout: Duration::from_secs(40),
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: context
                .evidence
                .join(format!("command-{command:04}.stdout")),
            stderr_path: context
                .evidence
                .join(format!("command-{command:04}.stderr")),
            unavailable_exit_code: None,
        };
        let control = process::ProcessControl::default();
        let observed = std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let deadline = Instant::now() + Duration::from_secs(30);
                loop {
                    if counter(consumer).is_ok_and(|value| value == expected_after) {
                        if terminate {
                            control.terminate();
                        } else {
                            control.interrupt();
                        }
                        return Ok(());
                    }
                    if Instant::now() >= deadline {
                        control.kill();
                        return Err(DevError::corrupt(
                            "foreground commit was not visible before signal deadline",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            let observation = process::run_controlled(&specification, &context.evidence, &control);
            handle
                .join()
                .map_err(|_| DevError::infrastructure("foreground signal observer panicked"))??;
            Ok::<_, DevError>(observation)
        })?;
        require(
            observed.status == process::ProcessStatus::Failed
                && observed.signal.is_none()
                && observed.exit_code.is_some_and(|code| code != 0),
            "foreground graceful signal did not produce a joined diagnostic exit",
        )?;
        context.receipt.commands.push(CommandEvidence {
            command: specification.command,
            cwd: consumer.cwd.display().to_string(),
            expects_success: false,
            observation: observed,
        });
        parse_records(
            "foreground-signal",
            &process::read_bounded(&specification.stdout_path, MAXIMUM_OUTPUT_BYTES)?,
        )
        .map_err(|e| DevError::corrupt(format!("signal diagnostic: {e:?}")))?
    } else {
        context.cli_at(&consumer.cwd, None, &arguments, false)?
    };
    let code = field(&records, "diagnostic", "code")?;
    require(
        code == expected_code,
        &format!("foreground {label}: expected {expected_code}, got {code}"),
    )?;
    let diagnostic = records
        .iter()
        .find(|r| r.operation == "diagnostic")
        .ok_or_else(|| DevError::corrupt("failure diagnostic missing"))?;
    let notes = record_field(diagnostic, "notes")
        .ok()
        .map(|s| serde_json::from_str::<Vec<String>>(&s))
        .transpose()?
        .unwrap_or_default();
    if invoked {
        require(notes.iter().any(|note|note.contains("earlier application effects may already be visible")) && notes.iter().any(|note|note=="foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"),"foreground failure omitted possible visibility or joined cleanup")?;
    }
    let after = counter(consumer)?;
    require(
        after == expected_after,
        "foreground failure replayed or discarded a committed invocation",
    )?;
    context
        .receipt
        .effects
        .foreground
        .failures
        .push(FailureCell {
            label: label.into(),
            command,
            before,
            after,
            code,
            notes,
        });
    Ok(())
}

fn lifecycle(context: &mut Context, consumer: &Consumer) -> Result<(), DevError> {
    let mut descriptor = consumer.descriptor.clone();
    descriptor["secrets"] =
        json!([{"name":"unread","variable":"LKJSCRIPT_FOREGROUND_UNAVAILABLE_SECRET"}]);
    descriptor["grants"][1]["adapter"]["root"] = json!("uncreated-data");
    failure(
        context,
        consumer,
        "arity-before-secret",
        &descriptor,
        "[]",
        "normalized_runner_argument_count",
        1,
        false,
        None,
    )?;
    failure(
        context,
        consumer,
        "type-before-secret",
        &descriptor,
        "[\"wrong\"]",
        "normalized_json_type",
        1,
        false,
        None,
    )?;
    descriptor["target"] = json!("unencodable");
    failure(
        context,
        consumer,
        "unselected-output-before-secret",
        &descriptor,
        "[]",
        "normalized_json_type",
        1,
        false,
        None,
    )?;
    require(
        !consumer.bundle.join("uncreated-data").exists(),
        "rejected preflight created operational state",
    )?;
    let mut foreign = consumer.descriptor["grants"].clone();
    foreign.as_array_mut().ok_or_else(||DevError::corrupt("foreground grant inventory is not an array"))?.push(json!({"requirement":"foreign","sharing_domain":"foreign","authority_revision":"83".repeat(32),"adapter":{"kind":"configuration"}}));
    for (label, grant, code) in [
        ("missing-grant", json!([]), "deployment_grant_missing"),
        ("foreign-grant", foreign, "deployment_grant_foreign"),
    ] {
        let mut descriptor = consumer.descriptor.clone();
        descriptor["grants"] = grant;
        descriptor["secrets"] =
            json!([{"name":"unread","variable":"LKJSCRIPT_FOREGROUND_UNAVAILABLE_SECRET"}]);
        failure(
            context,
            consumer,
            label,
            &descriptor,
            "[0]",
            code,
            1,
            false,
            None,
        )?;
    }
    let mut descriptor = consumer.descriptor.clone();
    descriptor["grants"][1]["adapter"] = json!({"kind":"configuration"});
    descriptor["secrets"] =
        json!([{"name":"unread","variable":"LKJSCRIPT_FOREGROUND_UNAVAILABLE_SECRET"}]);
    failure(
        context,
        consumer,
        "wrong-interface",
        &descriptor,
        "[0]",
        "normalized_deployment_adapter_interface",
        1,
        false,
        None,
    )?;
    for target in ["count", "pure-requiring-component"] {
        let mut descriptor = consumer.descriptor.clone();
        descriptor["target"] = json!(target);
        descriptor["grants"] = json!([consumer.descriptor["grants"][1].clone()]);
        descriptor["secrets"] =
            json!([{"name":"unread","variable":"LKJSCRIPT_FOREGROUND_UNAVAILABLE_SECRET"}]);
        failure(
            context,
            consumer,
            &format!("whole-component-{target}"),
            &descriptor,
            "[]",
            "deployment_grant_missing",
            1,
            false,
            None,
        )?;
    }
    let mut descriptor = consumer.descriptor.clone();
    descriptor["target"] = json!("rollback");
    failure(
        context,
        consumer,
        "transaction-rollback",
        &descriptor,
        "[1]",
        "normalized_integer_division",
        1,
        true,
        None,
    )?;
    descriptor["target"] = json!("trap");
    failure(
        context,
        consumer,
        "committed-then-trap",
        &descriptor,
        "[1]",
        "normalized_integer_division",
        2,
        true,
        None,
    )?;
    descriptor["target"] = json!("main");
    descriptor["execution"] = json!(lkjscript::platform::RunPolicy {
        instruction_fuel: 100,
        ..Default::default()
    });
    failure(
        context,
        consumer,
        "explicit-fuel",
        &descriptor,
        "[1]",
        "normalized_instruction_steps",
        2,
        true,
        None,
    )?;
    descriptor
        .as_object_mut()
        .ok_or_else(|| DevError::corrupt("foreground descriptor is not an object"))?
        .remove("execution");
    descriptor["target"] = json!("cancel-after-commit");
    descriptor["runtime"] = json!(lkjscript::platform::runtime::ResidentLimits {
        request_deadline_milliseconds: 100,
        ..Default::default()
    });
    failure(
        context,
        consumer,
        "deadline-after-commit",
        &descriptor,
        "[0]",
        "execution_deadline",
        3,
        true,
        None,
    )?;
    descriptor
        .as_object_mut()
        .ok_or_else(|| DevError::corrupt("foreground descriptor is not an object"))?
        .remove("runtime");
    failure(
        context,
        consumer,
        "sigint-after-commit",
        &descriptor,
        "[0]",
        "execution_cancelled",
        4,
        true,
        Some(false),
    )?;
    failure(
        context,
        consumer,
        "sigterm-after-commit",
        &descriptor,
        "[0]",
        "execution_cancelled",
        5,
        true,
        Some(true),
    )?;
    descriptor["target"] = json!("oversized");
    failure(
        context,
        consumer,
        "late-oversized-result",
        &descriptor,
        "[]",
        "normalized_json_type",
        6,
        true,
        None,
    )?;
    broken_output(context, consumer)
}

fn broken_output(context: &mut Context, consumer: &Consumer) -> Result<(), DevError> {
    let command = context.receipt.commands.len();
    let spec = process::ProcessSpec {
        command: vec![
            context.binary.display().to_string(),
            "run".into(),
            "--deployment".into(),
            consumer
                .bundle
                .join("command.deployment.json")
                .display()
                .to_string(),
            "--arguments".into(),
            "[0]".into(),
        ],
        cwd: consumer.cwd.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(30),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context
            .evidence
            .join(format!("command-{command:04}.stdout")),
        stderr_path: context
            .evidence
            .join(format!("command-{command:04}.stderr")),
        unavailable_exit_code: None,
    };
    let before = counter(consumer)?;
    let observation = process::run_closed_stdout(&spec, &context.evidence);
    require(
        observation.status == process::ProcessStatus::Failed && observation.signal.is_none(),
        "broken output reported success or skipped graceful completion",
    )?;
    let error: Value = serde_json::from_slice(&process::read_bounded(
        &spec.stderr_path,
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let code = error["code"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("broken output diagnostic absent"))?
        .to_owned();
    let notes: Vec<String> = serde_json::from_value(error["notes"].clone())?;
    require(
        notes.iter().any(|s| {
            s.contains("invocation and cleanup completed; earlier effects may be visible")
        }),
        "broken output omitted possible visibility and cleanup",
    )?;
    let after = counter(consumer)?;
    require(
        before == 6 && after == 7,
        "broken output replayed the application",
    )?;
    context.receipt.commands.push(CommandEvidence {
        command: spec.command,
        cwd: consumer.cwd.display().to_string(),
        expects_success: false,
        observation,
    });
    context
        .receipt
        .effects
        .foreground
        .failures
        .push(FailureCell {
            label: "broken-output".into(),
            command,
            before,
            after,
            code,
            notes,
        });
    Ok(())
}

fn validate_cell(cell: &Cell) -> Result<(), DevError> {
    let f = &cell.execution;
    for (name, expected) in [
        ("execution-mode", "production"),
        ("verification", "not-performed"),
        ("execution-profile", "trusted-foreground"),
        ("instruction-limit", "absent"),
        ("allocation-limit", "absent"),
        ("collection-limit", "absent"),
        ("capability-call-limit", "absent"),
        ("deadline-milliseconds", "absent"),
    ] {
        require(
            f.get(name).map(String::as_str) == Some(expected),
            &format!("foreground field {name} differs"),
        )?;
    }
    let cleanup: Value = serde_json::from_str(
        f.get("cleanup")
            .ok_or_else(|| DevError::corrupt("foreground cleanup absent"))?,
    )?;
    require(
        cleanup["admission_stopped"] == true
            && cleanup["remaining_tasks"] == 0
            && cleanup["cleanup_failures"] == json!([]),
        "foreground shutdown was omitted or incomplete",
    )?;
    require(
        cell.output == expected(cell.position, cell.executions)
            && cell.persisted_report == cell.output
            && cell.ordered_trace == (0..(cell.position + 1).min(258)).collect::<Vec<_>>(),
        "foreground independently expected report/trace differs",
    )
}

pub(super) fn validate(parent: &Receipt, root: &Path) -> Result<(), DevError> {
    let receipt = &parent.effects.foreground;
    require(
        receipt.schema == "lkjscript-foreground-1"
            && receipt.consumers.len() == 2
            && receipt.cells.len() == 7,
        "foreground child missing required composition cells",
    )?;
    let failures = [
        (
            "arity-before-secret",
            "normalized_runner_argument_count",
            1,
            1,
        ),
        ("type-before-secret", "normalized_json_type", 1, 1),
        (
            "unselected-output-before-secret",
            "normalized_json_type",
            1,
            1,
        ),
        ("missing-grant", "deployment_grant_missing", 1, 1),
        ("foreign-grant", "deployment_grant_foreign", 1, 1),
        (
            "wrong-interface",
            "normalized_deployment_adapter_interface",
            1,
            1,
        ),
        ("whole-component-count", "deployment_grant_missing", 1, 1),
        (
            "whole-component-pure-requiring-component",
            "deployment_grant_missing",
            1,
            1,
        ),
        ("transaction-rollback", "normalized_integer_division", 1, 1),
        ("committed-then-trap", "normalized_integer_division", 1, 2),
        ("explicit-fuel", "normalized_instruction_steps", 2, 2),
        ("deadline-after-commit", "execution_deadline", 2, 3),
        ("sigint-after-commit", "execution_cancelled", 3, 4),
        ("sigterm-after-commit", "execution_cancelled", 4, 5),
        ("late-oversized-result", "normalized_json_type", 5, 6),
        ("broken-output", "cli_output", 6, 7),
    ];
    require(
        receipt.failures.len() == failures.len(),
        "foreground authority/lifecycle cells missing",
    )?;
    for (index, (cell, (label, code, before, after))) in
        receipt.failures.iter().zip(failures).enumerate()
    {
        require(
            (
                cell.label.as_str(),
                cell.code.as_str(),
                cell.before,
                cell.after,
            ) == (label, code, before, after),
            "foreground failure workload or persisted count substituted",
        )?;
        if index >= 8 && label != "broken-output" {
            require(cell.notes.iter().any(|note|note=="foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0") && cell.notes.iter().any(|note|note.contains("earlier application effects may already be visible")),"foreground failed invocation omitted visibility/cleanup")?;
        }
        let command = parent
            .commands
            .get(cell.command)
            .ok_or_else(|| DevError::corrupt("foreground failure process evidence absent"))?;
        require(
            !command.expects_success
                && command.observation.status == process::ProcessStatus::Failed
                && command.observation.signal.is_none(),
            "foreground rejection lacks a completed diagnostic exit",
        )?;
        if label == "broken-output" {
            let error: Value = serde_json::from_slice(&process::read_bounded(
                &root.join(format!("command-{:04}.stderr", cell.command)),
                MAXIMUM_OUTPUT_BYTES,
            )?)?;
            require(
                error["code"] == code
                    && error["notes"] == json!(cell.notes)
                    && cell.notes.iter().any(|note| {
                        note.contains(
                            "invocation and cleanup completed; earlier effects may be visible",
                        )
                    }),
                "broken output evidence substituted",
            )?;
            continue;
        }
        let records = parse_records(
            "foreground-failure",
            &process::read_bounded(
                &root.join(format!("command-{:04}.stdout", cell.command)),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|e| DevError::corrupt(format!("foreground failure records: {e:?}")))?;
        require(
            field(&records, "diagnostic", "code")? == code,
            "foreground diagnostic evidence substituted",
        )?;
    }
    require(receipt.loops.len() == 4, "foreground policy cells missing")?;
    for (cell, (label, n, bounded)) in receipt.loops.iter().zip([
        ("calibration-zero", 0, false),
        ("calibration-thousand", 1000, false),
        ("past-ten-million", LONG_LOOP, false),
        ("bounded-equivalent", 1000, true),
    ]) {
        require(
            (cell.label.as_str(), cell.n, cell.bounded) == (label, n, bounded),
            "foreground loop workload substituted",
        )?;
        validate_loop(cell)?;
        let records = parse_records(
            "loop-evidence",
            &process::read_bounded(
                &root.join(format!("command-{:04}.stdout", cell.command)),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|e| DevError::corrupt(format!("loop evidence {e:?}")))?;
        let record = records
            .iter()
            .find(|r| r.operation == "execution")
            .ok_or_else(|| DevError::corrupt("loop recorded output absent"))?;
        require(
            record
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.value.clone()))
                .collect::<BTreeMap<_, _>>()
                == cell.execution,
            "loop output evidence substituted",
        )?;
    }
    let retained: ForegroundReceipt = serde_json::from_slice(&process::read_bounded(
        &root.join("foreground.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        serde_json::to_value(&retained)? == serde_json::to_value(receipt)?,
        "foreground child evidence substituted",
    )?;
    require(
        receipt.consumers[0].package != receipt.consumers[1].package
            && receipt.consumers[0].cwd != receipt.consumers[1].cwd
            && receipt.consumers[0].bundle != receipt.consumers[1].bundle,
        "foreground consumers do not have independent identities and roots",
    )?;
    for (index, consumer) in receipt.consumers.iter().enumerate() {
        require(
            consumer.checkout_unavailable
                && digest_file(
                    &root.join(format!("foreground-{index}-original.lkja")),
                    MAXIMUM_CONTAINER_BYTES,
                )? == consumer.artifact_sha256,
            "foreground artifact or checkout binding differs",
        )?;
    }
    require(
        receipt.consumers[0]
            .changed_revision
            .as_ref()
            .is_some_and(|r| r != &receipt.consumers[0].revision)
            && receipt.consumers[0].changed_artifact_sha256.as_deref()
                == Some(&digest_file(
                    &root.join("foreground-0-successor.lkja"),
                    MAXIMUM_CONTAINER_BYTES,
                )?),
        "foreground reviewed dependency replacement absent",
    )?;
    for (cell, (label, index, n, position, executions)) in receipt.cells.iter().zip([
        ("initial-0", 0, 0, 0, 1),
        ("initial-1", 0, 1, 1, 2),
        ("initial-257", 0, 257, 257, 3),
        ("initial-8193", 0, 8193, 8193, 4),
        ("independent-root", 1, 1, 2, 1),
        ("changed-callback", 0, 257, 258, 5),
        ("retained-bundle", 0, 257, 257, 6),
    ]) {
        require(
            (
                cell.label.as_str(),
                cell.consumer,
                cell.n,
                cell.position,
                cell.executions,
            ) == (label, index, n, position, executions),
            "foreground cell workload substituted",
        )?;
        validate_cell(cell)?;
        let command = parent
            .commands
            .get(cell.command)
            .ok_or_else(|| DevError::corrupt("foreground command evidence absent"))?;
        require(
            command.expects_success
                && command.cwd == receipt.consumers[cell.consumer].cwd
                && command.command
                    == vec![
                        Path::new(&parent.isolated_root)
                            .join("lkjscript")
                            .display()
                            .to_string(),
                        "run".into(),
                        "--deployment".into(),
                        Path::new(&receipt.consumers[cell.consumer].bundle)
                            .join("command.deployment.json")
                            .display()
                            .to_string(),
                        "--arguments".into(),
                        format!("[{}]", cell.n),
                    ]
                && command.observation.status == process::ProcessStatus::Passed,
            "foreground process evidence differs",
        )?;
        let records = parse_records(
            "foreground-evidence",
            &process::read_bounded(
                &root.join(format!("command-{:04}.stdout", cell.command)),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|e| DevError::corrupt(format!("foreground output records: {e:?}")))?;
        let record = records
            .iter()
            .find(|r| r.operation == "execution")
            .ok_or_else(|| DevError::corrupt("foreground recorded output absent"))?;
        require(
            record
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.value.clone()))
                .collect::<BTreeMap<_, _>>()
                == cell.execution,
            "foreground output substituted",
        )?;
    }
    Ok(())
}
