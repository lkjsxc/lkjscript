//! Literal graph-authored statistics, transferred execution and original numerical evidence.
//! The driver transports inputs and opaque checkpoints; all accumulation runs in the graph.
use super::*;
use lkjscript::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use serde_json::{Value, json};

const PRODUCER: &str = include_str!("f64.producer.lkjc");
const CONSUMER: &str = include_str!("f64.consumer.lkjc");
const DATA: &str = include_str!("f64.data.lkjc");
const EDIT: &str = include_str!("f64.edit.lkjc");
const SMALL: &str = "[0.5,1.5,2.5,3.5]";
const LARGE: &str = "[1099511627776.25,1099511627776.5,1099511627776.75,1099511627777.0]";
const FIRST: &str = "[1099511627776.25,1099511627776.5]";
const SECOND: &str = "[1099511627776.75,1099511627777.0]";
const MEASUREMENTS: &str = "[{\"label\":\"a\",\"value\":0.5},{\"label\":\"b\",\"value\":1.5},{\"label\":\"c\",\"value\":2.5},{\"label\":\"d\",\"value\":3.5}]";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    package: String,
    revision: String,
    logical: String,
    transport: String,
    inventory: usize,
}

impl Identity {
    fn of(package: &Package, inventory: usize) -> Self {
        Self {
            package: package.id.clone(),
            revision: package.revision.clone(),
            logical: package.logical.clone(),
            transport: package.transport.clone(),
            inventory,
        }
    }

    fn dependency(&self) -> String {
        format!(
            "add.dependency package={} semantic-revision={} package-revision={}\n",
            self.package, self.revision, self.logical
        )
    }

    fn selection(&self) -> String {
        format!(
            "reference.package as=$library package={} package-revision={}\n",
            self.package, self.logical
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authoring {
    name: String,
    base: String,
    result: String,
    plan_command: usize,
    apply_command: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    name: String,
    artifact: Option<String>,
    target: String,
    command: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericalEvidence {
    schema: String,
    standard: Identity,
    producer: Identity,
    before: Identity,
    after: Identity,
    authoring: Vec<Authoring>,
    inspect_before: usize,
    inspect_after: usize,
    producer_removed_before_run: bool,
    calls: Vec<Invocation>,
    store_observation_commands: Vec<usize>,
}

pub(super) fn focused(context: &mut Context) -> Result<(), DevError> {
    let builtin = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id")?,
        revision: field(&builtin, "package", "revision")?,
        logical: field(&builtin, "package", "package-revision")?,
        transport: field(&builtin, "package", "transport")?,
        container: context.root.join("f64-standard.lkjp"),
        symbols: BTreeMap::new(),
    };
    context.cli(
        None,
        &[
            "package",
            "builtin",
            "export",
            "--kind",
            "transport",
            "--output",
            &standard.container.display().to_string(),
        ],
        true,
    )?;
    workflow(context, &standard)
}

fn author(
    context: &mut Context,
    package: &mut Package,
    name: &str,
    prelude: &str,
    literal: &str,
) -> Result<Authoring, DevError> {
    let base = package.revision.clone();
    let request = context.root.join(format!("f64-{name}.lkjc"));
    let review = context.root.join(format!("f64-{name}.lkjplan"));
    fs::write(
        &request,
        format!("request base={base} idempotency=f64-{name}\n{prelude}{literal}"),
    )?;
    fs::copy(&request, context.evidence.join(format!("f64-{name}.lkjc")))?;
    let plan_command = context.receipt.commands.len();
    let planned = context.cli(
        Some(&package.path),
        &[
            "change",
            "plan",
            "--input-file",
            &request.display().to_string(),
            "--output",
            &review.display().to_string(),
        ],
        true,
    )?;
    let plan = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(&review)?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    fs::copy(
        &review,
        context.evidence.join(format!("f64-{name}.lkjplan")),
    )?;
    require(
        plan.token == field(&planned, "plan", "token")?,
        "F64 literal review token differs",
    )?;
    let apply_command = context.receipt.commands.len();
    let applied = context.cli(
        Some(&package.path),
        &[
            "change",
            "apply",
            "--input-file",
            &request.display().to_string(),
            "--plan",
            &plan.token,
        ],
        true,
    )?;
    package.revision = field(&applied, "revision", "result")?;
    for record in applied
        .iter()
        .filter(|record| record.operation == "identity")
    {
        package
            .symbols
            .insert(record_field(record, "symbol")?, record_field(record, "id")?);
    }
    Ok(Authoring {
        name: name.to_owned(),
        base,
        result: package.revision.clone(),
        plan_command,
        apply_command,
    })
}

fn inspect_calibration(context: &mut Context, package: &Package) -> Result<usize, DevError> {
    let command = context.receipt.commands.len();
    context.cli(
        Some(&package.path),
        &[
            "inspect",
            "owner",
            "pure_function",
            &package.symbols["$calibrate"],
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    Ok(command)
}

fn deployment_value(artifact: &str, target: &str) -> Value {
    let grants = if matches!(target, "save" | "load" | "after-commit") {
        json!([{"requirement":"statistics-store","sharing_domain":"f64-statistics","authority_revision":"f6".repeat(32),"adapter":{"kind":"data","root":"statistics-data","namespace":"numerical","limits":DataLimits::default()}}])
    } else {
        json!([])
    };
    json!({
        "artifact":format!("f64-{artifact}.lkja"),"target":target,"listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":grants
    })
}

#[allow(clippy::too_many_arguments)]
fn invoke(
    context: &mut Context,
    package: &Package,
    bundle: &Path,
    name: &str,
    artifact: Option<&str>,
    target: &str,
    arguments: &str,
    passes: bool,
) -> Result<(Invocation, Vec<CompactRecord>), DevError> {
    fs::write(
        context.evidence.join(format!("f64-input-{name}.json")),
        arguments,
    )?;
    let input_file = context.root.join(format!("f64-input-{name}.json"));
    let file_argument = input_file.display().to_string();
    let (selector, input) = if matches!(
        name,
        "project-decimals" | "scale" | "invalid-before-effect" | "invalid-f64-before-effect"
    ) {
        fs::write(&input_file, arguments)?;
        ("--arguments-file", file_argument.as_str())
    } else {
        ("--arguments", arguments)
    };
    let command = context.receipt.commands.len();
    let output = if let Some(artifact) = artifact {
        let filename = format!("f64-{artifact}-{target}.deployment.json");
        let descriptor = bundle.join(&filename);
        let bytes = evidence::encode_json(&deployment_value(artifact, target))?;
        if !descriptor.exists() {
            fs::write(&descriptor, &bytes)?;
            fs::write(context.evidence.join(filename), bytes)?;
        }
        context.cli(
            None,
            &[
                "run",
                "--deployment",
                &descriptor.display().to_string(),
                selector,
                input,
            ],
            passes,
        )?
    } else {
        context.cli(
            Some(&package.path),
            &["run", target, selector, input],
            passes,
        )?
    };
    Ok((
        Invocation {
            name: name.to_owned(),
            artifact: artifact.map(str::to_owned),
            target: target.to_owned(),
            command,
        },
        output,
    ))
}

// Only the already checked halfway accumulator (I64 count 2, exact dyadic mean
// 1099511627776.375 and m2=0.03125) and the opaque Bytes checkpoint use this parser.
// Arithmetic expectations, including square roots, compare the original JSON text below.
fn transport_value(records: &[CompactRecord]) -> Result<Value, DevError> {
    Ok(serde_json::from_str(&field(
        records,
        "execution",
        "value",
    )?)?)
}

fn build_artifact(
    context: &mut Context,
    package: &Package,
    bundle: &Path,
    name: &str,
) -> Result<(), DevError> {
    let filename = format!("f64-{name}.lkja");
    let output = bundle.join(&filename);
    context.cli(
        Some(&package.path),
        &["build", "--output", &output.display().to_string()],
        true,
    )?;
    fs::copy(&output, context.evidence.join(&filename))?;
    context.receipt.observations.insert(
        format!("artifact-f64-{name}"),
        digest_file(&output, MAXIMUM_CONTAINER_BYTES)?,
    );
    Ok(())
}

fn summary(count: i64, mean: f64, m2: f64, variance: f64, sqrt_bits: u64) -> Value {
    json!({"case":"Ready","value":{"count":count,"mean":mean,"m2":m2,"population-variance":variance,"standard-deviation":f64::from_bits(sqrt_bits)}})
}

fn small_summary(mean: f64) -> Value {
    summary(4, mean, 5.0, 1.25, 0x3ff1_e377_9b97_f4a8)
}

fn scalar_observation() -> Value {
    json!([
        "1.4142135623730951",
        "1.0",
        "0.0",
        "1e-323",
        "inf",
        "nan",
        "-inf",
        "-0.0"
    ])
}

fn large_summary() -> Value {
    summary(
        4,
        1_099_511_627_776.625,
        0.3125,
        0.078125,
        0x3fd1_e377_9b97_f4a8,
    )
}

fn scale_input() -> String {
    // Input construction only. The graph consumes all 4096 runtime measurements in its fold.
    let items = (0..4096)
        .map(|i| format!("{}{}", 1_000_000 + i / 4, [".0", ".25", ".5", ".75"][i % 4]))
        .collect::<Vec<_>>()
        .join(",");
    // Whitespace grows only the transport, preserving the independently fixed numeric workload.
    // This file exceeds the observed Linux single-argument capacity without expanding JSON limits.
    let mut input = format!("[[{items}]]");
    input.extend(std::iter::repeat_n(' ', 200_000 - input.len()));
    input
}

fn expect_value(records: &[CompactRecord], expected: &Value, name: &str) -> Result<(), DevError> {
    let actual = field(records, "execution", "value")?;
    // The expected values are independent dyadic constants and fixed binary64 sqrt bits.
    // Compare their canonical JSON directly: generic JSON float parsing must not become
    // a second rounding step or the arithmetic oracle.
    let expected = serde_json::to_string(expected)?;
    require(
        actual == expected,
        &format!("F64 {name}: actual {actual}, independently expected {expected}"),
    )
}

fn observe_store(bundle: &Path) -> Result<Value, DevError> {
    let store = DataStore::open(
        &bundle.join("statistics-data"),
        "numerical",
        DataLimits::default(),
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let transaction = store
        .begin()
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let mut result = serde_json::Map::new();
    for key in ["saved", "after-json", "invalid"] {
        let key_value = DataKey::new(vec![DataKeyPart::Text(key.into())], store.limits())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let value = transaction
            .get("statistics", &key_value)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        result.insert(
            key.to_owned(),
            match value {
                Some(value) => json!({"revision":value.revision,"bytes":value.value}),
                None => Value::Null,
            },
        );
    }
    Ok(Value::Object(result))
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    context.cli(None, &["capabilities", "--section", "change"], true)?;
    let standard_identity = Identity::of(standard, usize::MAX);
    let mut producer = context.new_package("f64-producer")?;
    context.stage(&producer, standard)?;
    let producer_authoring = author(
        context,
        &mut producer,
        "producer",
        &binding("add", standard),
        PRODUCER,
    )?;
    context.cli(Some(&producer.path), &["check"], true)?;
    context.export(&mut producer)?;
    let producer_identity = Identity::of(&producer, context.receipt.inventories.len() - 1);
    let mut consumer = context.new_package("f64-consumer")?;
    context.stage(&consumer, &producer)?;
    let prelude = format!(
        "{}{}{}",
        binding("add", standard),
        binding("add", &producer),
        producer_identity.selection()
    );
    let consumer_authoring = author(
        context,
        &mut consumer,
        "consumer",
        &prelude,
        &format!("{CONSUMER}{DATA}"),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.export(&mut consumer)?;
    let before = Identity::of(&consumer, context.receipt.inventories.len() - 1);
    let bundle = context.root.join("f64-bundle");
    fs::create_dir(&bundle)?;
    build_artifact(context, &consumer, &bundle, "before")?;
    let inspect_before = inspect_calibration(context, &consumer)?;
    fs::remove_dir_all(&producer.path)?;
    require(
        !producer.path.exists(),
        "numerical producer authoring path remains available",
    )?;
    let mut calls = Vec::new();
    let cases = [
        (
            "project-decimals",
            None,
            "direct",
            format!("[{SMALL}]"),
            small_summary(2.0),
        ),
        (
            "direct-small",
            Some("before"),
            "direct",
            format!("[{SMALL}]"),
            small_summary(2.0),
        ),
        (
            "direct-large",
            Some("before"),
            "direct",
            format!("[{LARGE}]"),
            large_summary(),
        ),
        (
            "scale",
            Some("before"),
            "direct",
            scale_input(),
            summary(
                4096,
                1_000_511.875,
                357_913_920.0,
                87_381.328125,
                0x4072_79a7_3c53_5f77,
            ),
        ),
        (
            "merge",
            Some("before"),
            "merged",
            format!("[{FIRST},{SECOND}]"),
            large_summary(),
        ),
        (
            "merge-left-empty",
            Some("before"),
            "merged",
            format!("[[],{LARGE}]"),
            large_summary(),
        ),
        (
            "merge-right-empty",
            Some("before"),
            "merged",
            format!("[{LARGE},[]]"),
            large_summary(),
        ),
        (
            "empty",
            Some("before"),
            "direct",
            "[[]]".into(),
            json!({"case":"Empty"}),
        ),
        (
            "calibration-before",
            Some("before"),
            "measurements",
            format!("[{MEASUREMENTS},0.25]"),
            small_summary(2.25),
        ),
        (
            "invalid-projection",
            Some("before"),
            "measurements",
            "[[{\"label\":\"invalid\",\"value\":1.0}],0.25]".into(),
            json!({"case":"InvalidSample"}),
        ),
        (
            "overflow",
            Some("before"),
            "direct",
            "[[1e308,-1e308]]".into(),
            json!({"case":"InvalidArithmetic"}),
        ),
        (
            "halfway-state",
            Some("before"),
            "state",
            format!("[{FIRST}]"),
            json!({"case":"Accumulated","value":{"count":2,"mean":1_099_511_627_776.375,"m2":0.03125}}),
        ),
    ];
    let mut halfway_state = None;
    for (name, artifact, target, arguments, expected) in cases {
        let (call, output) = invoke(
            context, &consumer, &bundle, name, artifact, target, &arguments, true,
        )?;
        expect_value(&output, &expected, name)?;
        if name == "halfway-state" {
            halfway_state = Some(transport_value(&output)?["value"].clone());
        }
        calls.push(call);
    }
    let (checkpoint_call, checkpoint_output) = invoke(
        context,
        &consumer,
        &bundle,
        "checkpoint",
        Some("before"),
        "checkpoint",
        &format!("[{FIRST}]"),
        true,
    )?;
    let checkpoint = transport_value(&checkpoint_output)?;
    require(
        checkpoint["case"] == "Saved"
            && checkpoint["value"]["$bytes"]
                .as_str()
                .is_some_and(|v| !v.is_empty()),
        "F64 checkpoint omitted typed opaque bytes",
    )?;
    fs::write(
        context.evidence.join("f64-checkpoint.json"),
        evidence::encode_json(&checkpoint["value"])?,
    )?;
    calls.push(checkpoint_call);
    // cli() joined the checkpoint process. The next invocation receives its unchanged opaque value.
    let resumed_arguments = format!("[{},{}]", checkpoint["value"], SECOND);
    let (resume_call, resumed) = invoke(
        context,
        &consumer,
        &bundle,
        "resume",
        Some("before"),
        "resume",
        &resumed_arguments,
        true,
    )?;
    expect_value(&resumed, &large_summary(), "resume")?;
    calls.push(resume_call);
    let (bad_checkpoint, bad_output) = invoke(
        context,
        &consumer,
        &bundle,
        "invalid-checkpoint",
        Some("before"),
        "resume",
        "[{\"$bytes\":\"\"},[]]",
        true,
    )?;
    expect_value(
        &bad_output,
        &json!({"case":"InvalidArithmetic"}),
        "invalid-checkpoint",
    )?;
    calls.push(bad_checkpoint);
    let edited = author(context, &mut consumer, "edit", "", EDIT)?;
    let inspect_after = inspect_calibration(context, &consumer)?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.export(&mut consumer)?;
    let after = Identity::of(&consumer, context.receipt.inventories.len() - 1);
    build_artifact(context, &consumer, &bundle, "after")?;
    for (name, artifact, expected) in [
        ("calibration-after", "after", small_summary(3.25)),
        ("old-artifact", "before", small_summary(2.25)),
    ] {
        let (call, output) = invoke(
            context,
            &consumer,
            &bundle,
            name,
            Some(artifact),
            "measurements",
            &format!("[{MEASUREMENTS},0.25]"),
            true,
        )?;
        expect_value(&output, &expected, name)?;
        calls.push(call);
    }
    let (scalar_call, scalar_output) = invoke(
        context,
        &consumer,
        &bundle,
        "scalar-observation",
        Some("after"),
        "scalar-observation",
        "[]",
        true,
    )?;
    expect_value(&scalar_output, &scalar_observation(), "scalar-observation")?;
    calls.push(scalar_call);
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &bundle.join("statistics-data").display().to_string(),
        ],
        true,
    )?;
    let state =
        halfway_state.ok_or_else(|| DevError::corrupt("original graph accumulator missing"))?;
    let mut observations = vec![observe_store(&bundle)?];
    let mut store_commands = Vec::new();
    for (name, target, arguments, expected) in [
        (
            "invalid-before-effect",
            "save",
            "[\"invalid\",{\"count\":1.0,\"mean\":1.25,\"m2\":0.0}]".into(),
            None,
        ),
        (
            "invalid-f64-before-effect",
            "save",
            "[\"invalid\",{\"count\":1,\"mean\":1e400,\"m2\":0.0}]".into(),
            None,
        ),
        (
            "data-save",
            "save",
            format!("[\"saved\",{state}]"),
            Some(json!({"case":"Committed","value":state})),
        ),
        (
            "data-load",
            "load",
            "[\"saved\"]".into(),
            Some(json!({"case":"Accumulated","value":state})),
        ),
        (
            "data-aborted",
            "save",
            format!("[\"saved\",{state}]"),
            Some(json!({"case":"Aborted","value":{"case":"ConditionFailed"}})),
        ),
        (
            "committed-json-failure",
            "after-commit",
            format!("[\"after-json\",{state}]"),
            None,
        ),
        (
            "load-after-json-failure",
            "load",
            "[\"after-json\"]".into(),
            Some(json!({"case":"Accumulated","value":state})),
        ),
    ] {
        let (call, output) = invoke(
            context,
            &consumer,
            &bundle,
            name,
            Some("after"),
            target,
            &arguments,
            expected.is_some(),
        )?;
        if let Some(expected) = expected {
            expect_value(&output, &expected, name)?;
        } else {
            require(
                field(&output, "diagnostic", "code")?.contains("json"),
                "F64 external rejection changed diagnostic boundary",
            )?;
        }
        store_commands.push(call.command);
        calls.push(call);
        observations.push(observe_store(&bundle)?);
    }
    fs::write(
        context.evidence.join("f64-data-observations.json"),
        evidence::encode_json(&observations)?,
    )?;
    validate_store_observations(&observations)?;
    let numerical = NumericalEvidence {
        schema: "lkjscript-numerical-library-2".into(),
        standard: standard_identity,
        producer: producer_identity,
        before,
        after,
        authoring: vec![producer_authoring, consumer_authoring, edited],
        inspect_before,
        inspect_after,
        producer_removed_before_run: true,
        calls,
        store_observation_commands: store_commands,
    };
    context
        .receipt
        .observations
        .insert("f64".into(), serde_json::to_string(&numerical)?);
    Ok(())
}

fn validate_store_observations(observations: &[Value]) -> Result<(), DevError> {
    require(
        observations.len() == 8,
        "F64 data observation sequence is incomplete",
    )?;
    let empty = json!({"saved":null,"after-json":null,"invalid":null});
    require(
        observations[0] == empty && observations[1] == empty && observations[2] == empty,
        "malformed F64 application arguments reached a write",
    )?;
    require(
        observations[3]["saved"]["bytes"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
            && observations[3]["after-json"].is_null()
            && observations[3]["invalid"].is_null(),
        "F64 committed nominal write missing",
    )?;
    require(
        observations[3] == observations[4] && observations[4] == observations[5],
        "load or aborted condition changed a completed F64 write",
    )?;
    require(
        observations[6]["after-json"]["bytes"] == observations[3]["saved"]["bytes"]
            && observations[6]["saved"] == observations[3]["saved"]
            && observations[6]["invalid"].is_null()
            && observations[6] == observations[7],
        "JSON failure erased a completed transaction or restart changed its bytes",
    )
}

fn output(root: &Path, index: usize) -> Result<Vec<CompactRecord>, DevError> {
    parse_records(
        "f64-original-output",
        &process::read_bounded(
            &root.join(format!("command-{index:04}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|error| DevError::corrupt(format!("F64 original output: {error:?}")))
}

fn command(receipt: &Receipt, index: usize) -> Result<&CommandEvidence, DevError> {
    receipt
        .commands
        .get(index)
        .ok_or_else(|| DevError::corrupt("F64 original command missing"))
}

fn validate_identity(receipt: &Receipt, root: &Path, identity: &Identity) -> Result<(), DevError> {
    let inventory = receipt
        .inventories
        .get(identity.inventory)
        .ok_or_else(|| DevError::corrupt("F64 package inventory missing"))?;
    let producer = receipt
        .producer_inventories
        .get(identity.inventory)
        .ok_or_else(|| DevError::corrupt("F64 producer inventory missing"))?;
    require(
        producer.package == identity.package
            && producer.semantic_revision == identity.revision
            && receipt.transport_digests.get(identity.inventory) == Some(&identity.transport),
        "F64 accepted source and transport identities differ",
    )?;
    require(
        inventory.packages.iter().any(|p| {
            p.package == identity.package
                && p.semantic_revision == identity.revision
                && p.package_revision == identity.logical
        }),
        "F64 exact package revision is absent from independent container inventory",
    )?;
    verify_producer_inventory(producer, inventory)?;
    let bytes = process::read_bounded(
        &root.join(format!("transport-{}.lkjp", identity.inventory + 1)),
        MAXIMUM_CONTAINER_BYTES,
    )?;
    require(
        offline_package_inventory(&bytes, &identity.transport)
            .map_err(|error| DevError::corrupt(error.to_string()))?
            == *inventory,
        "F64 original transport differs from independently read source inventory",
    )
}

fn validate_standard(
    receipt: &Receipt,
    root: &Path,
    numerical: &NumericalEvidence,
) -> Result<(), DevError> {
    let producer_plan = numerical
        .authoring
        .first()
        .ok_or_else(|| DevError::corrupt("F64 producer authoring missing"))?
        .plan_command;
    let copied = Path::new(&receipt.isolated_root)
        .join("lkjscript")
        .display()
        .to_string();
    let expected_command = [copied, "package".into(), "builtin".into(), "inspect".into()];
    let mut inspections = 0;
    for (index, command) in receipt.commands.iter().enumerate().take(producer_plan) {
        if command.command != expected_command {
            continue;
        }
        let original = output(root, index)?;
        require(
            command.expects_success
                && field(&original, "package", "id")? == numerical.standard.package
                && field(&original, "package", "revision")? == numerical.standard.revision
                && field(&original, "package", "package-revision")? == numerical.standard.logical
                && field(&original, "package", "transport")? == numerical.standard.transport,
            "F64 standard identity differs from original copied-product discovery",
        )?;
        inspections += 1;
    }
    require(inspections != 0, "F64 original standard discovery missing")?;
    for identity in [&numerical.producer, &numerical.before, &numerical.after] {
        let inventory = receipt
            .inventories
            .get(identity.inventory)
            .ok_or_else(|| DevError::corrupt("F64 standard source inventory missing"))?;
        let standard = inventory
            .packages
            .iter()
            .find(|package| package.package == numerical.standard.package)
            .ok_or_else(|| DevError::corrupt("F64 transported standard source missing"))?;
        require(
            standard.semantic_revision == numerical.standard.revision
                && standard.package_revision == numerical.standard.logical,
            "F64 discovered standard identity differs from admitted transported source",
        )?;
    }
    Ok(())
}

fn validate_artifacts(
    receipt: &Receipt,
    root: &Path,
    numerical: &NumericalEvidence,
) -> Result<BTreeMap<&'static str, String>, DevError> {
    let isolated = Path::new(&receipt.isolated_root);
    let mut bundles = BTreeMap::new();
    for (name, identity) in [("before", &numerical.before), ("after", &numerical.after)] {
        let path = root.join(format!("f64-{name}.lkja"));
        let digest = digest_file(&path, MAXIMUM_CONTAINER_BYTES)?;
        require(
            receipt.observations.get(&format!("artifact-f64-{name}")) == Some(&digest),
            "F64 original built artifact differs",
        )?;
        let artifact = process::read_bounded(&path, MAXIMUM_CONTAINER_BYTES)?;
        let source = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", identity.inventory + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
            &artifact,
            &source,
            &identity.transport,
        )
        .map_err(|error| {
            DevError::corrupt(format!("F64 original artifact/source binding: {error}"))
        })?;
        require(
            bound["package"] == identity.package
                && bound["revision"] == identity.revision
                && bound["package_revision"] == identity.logical
                && bound["source_transport"] == identity.transport,
            "F64 admitted artifact differs from the selected accepted source identity",
        )?;
        let bundle = bound["bundle"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("F64 admitted artifact bundle identity missing"))?
            .to_owned();
        let expected_command = [
            isolated.join("lkjscript").display().to_string(),
            "--project".into(),
            isolated.join("f64-consumer").display().to_string(),
            "build".into(),
            "--output".into(),
            isolated
                .join("f64-bundle")
                .join(format!("f64-{name}.lkja"))
                .display()
                .to_string(),
        ];
        let mut builds = 0;
        for (index, command) in receipt.commands.iter().enumerate() {
            if command.command != expected_command {
                continue;
            }
            let original = output(root, index)?;
            require(
                command.expects_success
                    && field(&original, "artifact", "bundle")? == bundle
                    && field(&original, "authority", "package")? == identity.package
                    && field(&original, "authority", "revision")? == identity.revision,
                "F64 original build output differs from admitted artifact/source identity",
            )?;
            builds += 1;
        }
        require(
            builds == 1,
            "F64 original artifact build missing or duplicated",
        )?;
        bundles.insert(name, bundle);
    }
    Ok(bundles)
}

fn validate_authoring(
    receipt: &Receipt,
    root: &Path,
    numerical: &NumericalEvidence,
) -> Result<(), DevError> {
    require(
        numerical.authoring.len() == 3,
        "F64 literal producer, consumer or edit authoring missing",
    )?;
    let isolated = Path::new(&receipt.isolated_root);
    let copied = isolated.join("lkjscript").display().to_string();
    for (index, name, project, prelude, literal, result) in [
        (
            0,
            "producer",
            "f64-producer",
            numerical.standard.dependency(),
            PRODUCER.to_owned(),
            numerical.producer.revision.as_str(),
        ),
        (
            1,
            "consumer",
            "f64-consumer",
            format!(
                "{}{}{}",
                numerical.standard.dependency(),
                numerical.producer.dependency(),
                numerical.producer.selection()
            ),
            format!("{CONSUMER}{DATA}"),
            numerical.before.revision.as_str(),
        ),
        (
            2,
            "edit",
            "f64-consumer",
            String::new(),
            EDIT.to_owned(),
            numerical.after.revision.as_str(),
        ),
    ] {
        let authored = &numerical.authoring[index];
        require(
            authored.name == name
                && authored.result == result
                && authored.apply_command == authored.plan_command + 1,
            "F64 authoring sequence or accepted revision differs",
        )?;
        let request_name = format!("f64-{name}.lkjc");
        let plan_name = format!("f64-{name}.lkjplan");
        require(
            process::read_bounded(&root.join(&request_name), MAXIMUM_OUTPUT_BYTES)?
                == format!(
                    "request base={} idempotency=f64-{name}\n{prelude}{literal}",
                    authored.base
                )
                .as_bytes(),
            "F64 retained request differs from the complete literal ordinary library, consumer or projection edit",
        )?;
        let plan_bytes = process::read_bounded(&root.join(&plan_name), MAXIMUM_OUTPUT_BYTES)?;
        let plan = decode_logical_change_plan(std::io::Cursor::new(&plan_bytes))
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let plan_command = command(receipt, authored.plan_command)?;
        let apply_command = command(receipt, authored.apply_command)?;
        require(
            plan_command.expects_success
                && plan_command.command
                    == [
                        copied.clone(),
                        "--project".into(),
                        isolated.join(project).display().to_string(),
                        "change".into(),
                        "plan".into(),
                        "--input-file".into(),
                        isolated.join(&request_name).display().to_string(),
                        "--output".into(),
                        isolated.join(&plan_name).display().to_string(),
                    ],
            "F64 planning did not consume the isolated original request and produce its retained logical plan",
        )?;
        require(
            apply_command.expects_success
                && apply_command.command
                    == [
                        copied.clone(),
                        "--project".into(),
                        isolated.join(project).display().to_string(),
                        "change".into(),
                        "apply".into(),
                        "--input-file".into(),
                        isolated.join(&request_name).display().to_string(),
                        "--plan".into(),
                        plan.token.clone(),
                    ],
            "F64 publication did not consume the exact reviewed request and plan",
        )?;
        for original in [
            output(root, authored.plan_command)?,
            output(root, authored.apply_command)?,
        ] {
            require(
                field(&original, "plan", "token")? == plan.token
                    && field(&original, "revision", "base")? == authored.base
                    && field(&original, "revision", "result")? == authored.result,
                "F64 original plan/apply result lost its review binding",
            )?;
        }
    }
    require(
        numerical.authoring[2].base == numerical.before.revision,
        "F64 callback edit did not start at the accepted consumer",
    )?;
    let before = output(root, numerical.inspect_before)?;
    let after = output(root, numerical.inspect_after)?;
    let creation = output(root, numerical.authoring[1].apply_command)?;
    let created_calibration = creation
        .iter()
        .find(|record| {
            record.operation == "identity"
                && record
                    .fields
                    .iter()
                    .any(|field| field.name == "symbol" && field.value == "$calibrate")
        })
        .ok_or_else(|| DevError::corrupt("F64 original calibration identity missing"))?;
    let calibration = record_field(created_calibration, "id")?;
    let repository = field(&creation, "project", "repository")?;
    let mut signatures = Vec::new();
    let mut headers = Vec::new();
    let mut bodies = Vec::new();
    for (index, records, revision) in [
        (
            numerical.inspect_before,
            &before,
            &numerical.before.revision,
        ),
        (numerical.inspect_after, &after, &numerical.after.revision),
    ] {
        require(
            field(records, "page", "complete")? == "true"
                && field(records, "definition.function", "name")? == "calibrate",
            "F64 calibration inspection incomplete or foreign",
        )?;
        let declaration = field(records, "definition.function", "id")?;
        require(
            declaration == calibration
                && field(records, "definition.header", "function")? == calibration
                && field(records, "definition.header", "repository")? == repository
                && field(records, "definition.header", "package")? == numerical.before.package
                && field(records, "definition.header", "revision")? == *revision
                && field(records, "revision", "observed")? == *revision
                && records
                    .iter()
                    .filter(|record| record.operation == "definition.header")
                    .count()
                    == 1
                && records
                    .iter()
                    .filter(|record| record.operation == "revision")
                    .count()
                    == 1,
            "F64 calibration inspection changed accepted revision, repository, package or function identity",
        )?;
        require(
            command(receipt, index)?.command
                == [
                    copied.clone(),
                    "--project".into(),
                    isolated.join("f64-consumer").display().to_string(),
                    "inspect".into(),
                    "owner".into(),
                    "pure_function".into(),
                    declaration,
                    "--detail".into(),
                    "definition".into(),
                    "--limit".into(),
                    "1000".into(),
                    "--bytes".into(),
                    "1048576".into(),
                ],
            "F64 calibration inspection selected another owner",
        )?;
        let mut signature = BTreeMap::new();
        for record in records.iter().filter(|record| {
            matches!(
                record.operation.as_str(),
                "definition.function" | "definition.parameter"
            )
        }) {
            let key = format!("{}:{}", record.operation, record_field(record, "name")?);
            let fields = record
                .fields
                .iter()
                .filter(|field| record.operation != "definition.function" || field.name != "body")
                .map(|field| (field.name.clone(), field.value.clone()))
                .collect::<Vec<_>>();
            require(
                signature.insert(key, fields).is_none(),
                "F64 calibration inspection duplicated a signature owner",
            )?;
        }
        require(
            signature.len() == 3,
            "F64 calibration function or either retained parameter absent",
        )?;
        signatures.push(signature);
        headers.push(
            records
                .iter()
                .find(|record| record.operation == "definition.header")
                .ok_or_else(|| DevError::corrupt("F64 calibration header missing"))?
                .fields
                .iter()
                .filter(|field| field.name != "revision")
                .map(|field| (field.name.clone(), field.value.clone()))
                .collect::<Vec<_>>(),
        );
        bodies.push(
            records
                .iter()
                .filter(|record| {
                    matches!(
                        record.operation.as_str(),
                        "definition.expression" | "definition.binding"
                    )
                })
                .map(|record| record_field(record, "id"))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    require(
        signatures[0] == signatures[1]
            && headers[0] == headers[1]
            && field(&before, "definition.function", "body")?
                != field(&after, "definition.function", "body")?
            && !bodies[0].is_empty()
            && bodies[0].iter().all(|old| !bodies[1].contains(old)),
        "F64 projection edit changed its complete retained signature or header, or reused the previous body",
    )?;
    let edited_plan = parse_records(
        "f64-edit-logical-plan",
        &process::read_bounded(&root.join("f64-edit.lkjplan"), MAXIMUM_OUTPUT_BYTES)?,
    )
    .map_err(|error| DevError::corrupt(format!("F64 edit logical records: {error:?}")))?;
    let retired = edited_plan
        .iter()
        .filter(|record| record.operation == "logical-plan.retirement")
        .map(|record| {
            require(
                record_field(record, "before-present")? == "false"
                    && record_field(record, "after-present")? == "true",
                "F64 body retirement did not preserve the prior owner",
            )?;
            record_field(record, "owner")
        })
        .collect::<Result<Vec<_>, _>>()?;
    require(
        retired.len() == bodies[0].len() && bodies[0].iter().all(|id| retired.contains(id)),
        "F64 reviewed replacement omitted previous body retirements",
    )
}

fn required_calls() -> Vec<(&'static str, Option<&'static str>, &'static str)> {
    vec![
        ("project-decimals", None, "direct"),
        ("direct-small", Some("before"), "direct"),
        ("direct-large", Some("before"), "direct"),
        ("scale", Some("before"), "direct"),
        ("merge", Some("before"), "merged"),
        ("merge-left-empty", Some("before"), "merged"),
        ("merge-right-empty", Some("before"), "merged"),
        ("empty", Some("before"), "direct"),
        ("calibration-before", Some("before"), "measurements"),
        ("invalid-projection", Some("before"), "measurements"),
        ("overflow", Some("before"), "direct"),
        ("halfway-state", Some("before"), "state"),
        ("checkpoint", Some("before"), "checkpoint"),
        ("resume", Some("before"), "resume"),
        ("invalid-checkpoint", Some("before"), "resume"),
        ("calibration-after", Some("after"), "measurements"),
        ("old-artifact", Some("before"), "measurements"),
        ("scalar-observation", Some("after"), "scalar-observation"),
        ("invalid-before-effect", Some("after"), "save"),
        ("invalid-f64-before-effect", Some("after"), "save"),
        ("data-save", Some("after"), "save"),
        ("data-load", Some("after"), "load"),
        ("data-aborted", Some("after"), "save"),
        ("committed-json-failure", Some("after"), "after-commit"),
        ("load-after-json-failure", Some("after"), "load"),
    ]
}

fn expected_arguments(name: &str, checkpoint: &Value, state: &Value) -> Result<String, DevError> {
    Ok(match name {
        "project-decimals" | "direct-small" => format!("[{SMALL}]"),
        "direct-large" => format!("[{LARGE}]"),
        "scale" => scale_input(),
        "merge" => format!("[{FIRST},{SECOND}]"),
        "merge-left-empty" => format!("[[],{LARGE}]"),
        "merge-right-empty" => format!("[{LARGE},[]]"),
        "empty" => "[[]]".into(),
        "calibration-before" | "calibration-after" | "old-artifact" => {
            format!("[{MEASUREMENTS},0.25]")
        }
        "invalid-projection" => "[[{\"label\":\"invalid\",\"value\":1.0}],0.25]".into(),
        "overflow" => "[[1e308,-1e308]]".into(),
        "halfway-state" | "checkpoint" => format!("[{FIRST}]"),
        "resume" => format!("[{checkpoint},{SECOND}]"),
        "invalid-checkpoint" => "[{\"$bytes\":\"\"},[]]".into(),
        "scalar-observation" => "[]".into(),
        "invalid-before-effect" => "[\"invalid\",{\"count\":1.0,\"mean\":1.25,\"m2\":0.0}]".into(),
        "invalid-f64-before-effect" => {
            "[\"invalid\",{\"count\":1,\"mean\":1e400,\"m2\":0.0}]".into()
        }
        "data-save" | "data-aborted" => format!("[\"saved\",{state}]"),
        "data-load" => "[\"saved\"]".into(),
        "committed-json-failure" => format!("[\"after-json\",{state}]"),
        "load-after-json-failure" => "[\"after-json\"]".into(),
        _ => return Err(DevError::corrupt("unrecognized numerical input")),
    })
}

fn expected_value(
    name: &str,
    checkpoint: &Value,
    state: &Value,
) -> Result<Option<Value>, DevError> {
    Ok(match name {
        "project-decimals" | "direct-small" => Some(small_summary(2.0)),
        "direct-large" | "merge" | "merge-left-empty" | "merge-right-empty" | "resume" => {
            Some(large_summary())
        }
        "scale" => Some(summary(
            4096,
            1_000_511.875,
            357_913_920.0,
            87_381.328125,
            0x4072_79a7_3c53_5f77,
        )),
        "empty" => Some(json!({"case":"Empty"})),
        "calibration-before" | "old-artifact" => Some(small_summary(2.25)),
        "calibration-after" => Some(small_summary(3.25)),
        "scalar-observation" => Some(scalar_observation()),
        "invalid-projection" => Some(json!({"case":"InvalidSample"})),
        "overflow" | "invalid-checkpoint" => Some(json!({"case":"InvalidArithmetic"})),
        "halfway-state" => Some(
            json!({"case":"Accumulated","value":{"count":2,"mean":1_099_511_627_776.375,"m2":0.03125}}),
        ),
        "checkpoint" => Some(json!({"case":"Saved","value":checkpoint})),
        "data-save" => Some(json!({"case":"Committed","value":state})),
        "data-load" | "load-after-json-failure" => {
            Some(json!({"case":"Accumulated","value":state}))
        }
        "data-aborted" => Some(json!({"case":"Aborted","value":{"case":"ConditionFailed"}})),
        "invalid-before-effect" | "invalid-f64-before-effect" | "committed-json-failure" => None,
        _ => return Err(DevError::corrupt("unrecognized numerical expected value")),
    })
}

pub(super) fn validate(
    receipt: &Receipt,
    root: &Path,
    inventory_end: usize,
) -> Result<Vec<usize>, DevError> {
    let numerical: NumericalEvidence = serde_json::from_str(
        receipt
            .observations
            .get("f64")
            .ok_or_else(|| DevError::corrupt("ordinary F64 library original evidence missing"))?,
    )?;
    require(
        numerical.schema == "lkjscript-numerical-library-2"
            && numerical.producer_removed_before_run
            && numerical.producer.package != numerical.before.package
            && numerical.before.package == numerical.after.package
            && numerical.before.revision != numerical.after.revision
            && numerical.before.logical != numerical.after.logical
            && numerical.standard.inventory == usize::MAX,
        "F64 independent library, edited consumer or producer removal missing",
    )?;
    let inventory_start = inventory_end
        .checked_sub(3)
        .ok_or_else(|| DevError::corrupt("F64 source inventory boundary is too small"))?;
    require(
        inventory_span_matches(
            [
                numerical.producer.inventory,
                numerical.before.inventory,
                numerical.after.inventory,
            ],
            inventory_start..inventory_end,
            receipt.inventories.len(),
        ),
        "F64 exact source inventories omitted or reordered",
    )?;
    for identity in [&numerical.producer, &numerical.before, &numerical.after] {
        validate_identity(receipt, root, identity)?;
    }
    validate_standard(receipt, root, &numerical)?;
    let before = &receipt.producer_inventories[numerical.before.inventory];
    let after = &receipt.producer_inventories[numerical.after.inventory];
    require(
        before.dependencies == after.dependencies
            && before.dependencies.contains(&(
                numerical.producer.package.clone(),
                numerical.producer.logical.clone(),
            ))
            && before.dependencies.contains(&(
                numerical.standard.package.clone(),
                numerical.standard.logical.clone(),
            )),
        "F64 callback edit changed a library, primitive implementation or standard dependency",
    )?;
    validate_authoring(receipt, root, &numerical)?;
    let artifact_bundles = validate_artifacts(receipt, root, &numerical)?;
    let required = required_calls();
    require(
        numerical.calls.len() == required.len()
            && numerical
                .calls
                .windows(2)
                .all(|pair| pair[0].command < pair[1].command),
        "F64 calls missing, duplicated, replayed or reordered",
    )?;
    let checkpoint: Value = serde_json::from_slice(&process::read_bounded(
        &root.join("f64-checkpoint.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        checkpoint.as_object().is_some_and(|o| o.len() == 1)
            && checkpoint["$bytes"].as_str().is_some_and(|s| !s.is_empty()),
        "F64 original opaque checkpoint missing",
    )?;
    let halfway = output(root, numerical.calls[11].command)?;
    expect_value(
        &halfway,
        &json!({"case":"Accumulated","value":{"count":2,"mean":1_099_511_627_776.375,"m2":0.03125}}),
        "halfway-state",
    )?;
    let state = transport_value(&halfway)?["value"].clone();
    require(
        state == json!({"count":2,"mean":1_099_511_627_776.375,"m2":0.03125}),
        "F64 data workflow does not transport the actual graph-produced halfway accumulator",
    )?;
    let isolated = Path::new(&receipt.isolated_root);
    let copied = isolated.join("lkjscript").display().to_string();
    let mut installed = Vec::new();
    for (call, (name, artifact, target)) in numerical.calls.iter().zip(required) {
        require(
            call.name == name && call.artifact.as_deref() == artifact && call.target == target,
            "F64 workload selected another case, artifact or target",
        )?;
        let arguments = expected_arguments(name, &checkpoint, &state)?;
        require(
            process::read_bounded(
                &root.join(format!("f64-input-{name}.json")),
                MAXIMUM_OUTPUT_BYTES,
            )? == arguments.as_bytes(),
            "F64 retained runtime input differs from its independent literal fixture or original checkpoint",
        )?;
        let original_command = command(receipt, call.command)?;
        let expected = expected_value(name, &checkpoint, &state)?;
        // The reader independently requires both file routes and the two pre-effect failures.
        let (selector, input) = match name {
            "project-decimals"
            | "scale"
            | "invalid-before-effect"
            | "invalid-f64-before-effect" => (
                "--arguments-file",
                isolated
                    .join(format!("f64-input-{name}.json"))
                    .display()
                    .to_string(),
            ),
            _ => ("--arguments", arguments),
        };
        let expected_command = if let Some(artifact) = artifact {
            let filename = format!("f64-{artifact}-{target}.deployment.json");
            require(
                serde_json::from_slice::<Value>(&process::read_bounded(
                    &root.join(&filename),
                    MAXIMUM_OUTPUT_BYTES,
                )?)? == deployment_value(artifact, target),
                "F64 deployment changed exact artifact, target or granted disposable authority",
            )?;
            installed.push(call.command);
            vec![
                receipt.pinned_runtime_path.clone(),
                "run".into(),
                "--deployment".into(),
                isolated
                    .join("f64-bundle")
                    .join(filename)
                    .display()
                    .to_string(),
                selector.into(),
                input,
            ]
        } else {
            vec![
                copied.clone(),
                "--project".into(),
                isolated.join("f64-consumer").display().to_string(),
                "run".into(),
                target.into(),
                selector.into(),
                input,
            ]
        };
        require(
            original_command.command == expected_command
                && original_command.expects_success == expected.is_some(),
            "F64 actual invocation differs from retained arguments or exact installed runtime",
        )?;
        let original_output = output(root, call.command)?;
        if let Some(expected) = expected {
            if let Some(artifact) = artifact {
                let identity = if artifact == "before" {
                    &numerical.before
                } else {
                    &numerical.after
                };
                require(
                    field(&original_output, "execution", "artifact")? == artifact_bundles[artifact]
                        && field(&original_output, "execution", "package")? == identity.package
                        && field(&original_output, "execution", "revision")? == identity.revision,
                    "F64 original execution differs from admitted artifact/source identity",
                )?;
            } else {
                require(
                    field(&original_output, "artifact", "bundle")? == artifact_bundles["before"]
                        && field(&original_output, "authority", "package")?
                            == numerical.before.package
                        && field(&original_output, "authority", "revision")?
                            == numerical.before.revision,
                    "F64 original project execution differs from admitted accepted source identity",
                )?;
            }
            expect_value(&original_output, &expected, name)?;
            if artifact.is_none() {
                require(
                    field(&original_output, "execution", "differential")? == "equal",
                    "F64 source execution lost independent evaluator agreement",
                )?;
            }
        } else {
            require(
                !original_output
                    .iter()
                    .any(|record| record.operation == "execution")
                    && field(&original_output, "diagnostic", "code")?.contains("json"),
                "F64 rejected JSON became a successful payload or lost its classified failure",
            )?;
            if name == "committed-json-failure" {
                require(
                    field(&original_output, "diagnostic", "code")? == "normalized_json_nonfinite",
                    "F64 completed-write output failure changed its nonfinite boundary",
                )?;
                let notes: Vec<String> =
                    serde_json::from_str(&field(&original_output, "diagnostic", "notes")?)?;
                require(notes.iter().any(|note| note == "earlier application effects may already be visible; automatic retry is not safe") && notes.iter().any(|note| note.contains("remaining-owned-tasks=0 failures=0")), "F64 output failure lost truthful prior-effect visibility or joined cleanup")?;
            }
        }
    }
    require(
        numerical.authoring[1].apply_command < numerical.inspect_before
            && numerical.inspect_before < numerical.calls[0].command
            && numerical.calls[14].command < numerical.authoring[2].plan_command
            && numerical.authoring[2].apply_command < numerical.inspect_after
            && numerical.inspect_after < numerical.calls[15].command,
        "F64 callback edit was not reviewed between the original and changed workload",
    )?;
    require(
        numerical.store_observation_commands
            == numerical.calls[18..]
                .iter()
                .map(|call| call.command)
                .collect::<Vec<_>>(),
        "F64 store observations are not bound to all original sequential invocations",
    )?;
    let observations: Vec<Value> = serde_json::from_slice(&process::read_bounded(
        &root.join("f64-data-observations.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    validate_store_observations(&observations)?;
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires retained focused F64 receipt and its original verifier"]
    fn argument_file_originals_reject_rehashed_substitution() {
        let source = PathBuf::from(
            std::env::var_os("LKJSCRIPT_F64_ARGUMENT_RECEIPT").expect("focused F64 receipt"),
        );
        let verifier = PathBuf::from(
            std::env::var_os("LKJSCRIPT_F64_ARGUMENT_VERIFIER").expect("original verifier"),
        );
        let mut receipt: Receipt = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        let candidate = PathBuf::from(&receipt.pinned_runtime_path);
        super::super::finite::read_focused(&source, &candidate, &verifier)
            .expect("unaltered original passes current reader");
        let owned = tempfile::tempdir().unwrap();
        for file in &receipt.files {
            assert_eq!(Path::new(&file.path).components().count(), 1);
            fs::copy(
                source.parent().unwrap().join(&file.path),
                owned.path().join(&file.path),
            )
            .unwrap();
        }
        receipt.evidence_root = owned.path().display().to_string();
        let path = owned.path().join("receipt.json");
        fs::write(&path, evidence::encode_json(&receipt).unwrap()).unwrap();
        super::super::finite::read_focused(&path, &candidate, &verifier)
            .expect("owned relocated fixture passes");
        let numerical: NumericalEvidence =
            serde_json::from_str(&receipt.observations["f64"]).unwrap();
        let index = numerical
            .calls
            .iter()
            .find(|call| call.name == "scale")
            .unwrap()
            .command;
        let name = "f64-input-scale.json";
        let input = fs::read_to_string(owned.path().join(name)).unwrap();
        assert_eq!(input.len(), 200_000);

        let mut fault = receipt.clone();
        let arguments = &mut fault.commands[index].command;
        let length = arguments.len();
        assert_eq!(arguments[length - 2], "--arguments-file");
        arguments[length - 2] = "--arguments".into();
        arguments[length - 1] = input.clone();
        fs::write(&path, evidence::encode_json(&fault).unwrap()).unwrap();
        let rejected = super::super::finite::read_focused(&path, &candidate, &verifier)
            .expect_err("inline substitution must not prove file delivery");
        assert!(
            rejected
                .to_string()
                .contains("F64 actual invocation differs")
        );

        fs::write(owned.path().join(name), input.trim_end()).unwrap();
        let mut fault = receipt.clone();
        *fault
            .files
            .iter_mut()
            .find(|file| file.path == name)
            .unwrap() = evidence::proof(&owned.path().join(name), name.to_owned()).unwrap();
        fs::write(&path, evidence::encode_json(&fault).unwrap()).unwrap();
        let rejected = super::super::finite::read_focused(&path, &candidate, &verifier)
            .expect_err("rehashed padding removal must not prove large file delivery");
        assert!(
            rejected
                .to_string()
                .contains("F64 retained runtime input differs")
        );

        fs::write(owned.path().join(name), input).unwrap();
        fs::write(&path, evidence::encode_json(&receipt).unwrap()).unwrap();
        super::super::finite::read_focused(&path, &candidate, &verifier)
            .expect("healthy restored fixture passes");
        super::super::finite::read_focused(&source, &candidate, &verifier)
            .expect("originals remain untouched");
    }
}
