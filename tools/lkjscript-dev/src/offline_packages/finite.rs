//! Current finite-callable composition and complete public refusal controls.
use super::*;
use serde_json::{Value, json};

fn selection(library: &Package) -> String {
    format!(
        "reference.package as=$library package={} package-revision={}\n",
        library.id, library.logical
    )
}

pub(super) fn focused(context: &mut Context) -> Result<(), DevError> {
    let builtin = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id")?,
        revision: field(&builtin, "package", "revision")?,
        logical: field(&builtin, "package", "package-revision")?,
        transport: field(&builtin, "package", "transport")?,
        container: context.root.join("finite-standard.lkjp"),
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

fn rejected_request(revision: &str, named: bool) -> String {
    let body = if named {
        "expression.function-value as=$call function=$bad\ntype.argument parent=$call index=0 type=@grown\nexpression.i64 as=$seven value=7\nexpression.sequence as=$bad-body\nexpression.argument parent=$bad-body index=0 expression=$call\nexpression.argument parent=$bad-body index=1 expression=$seven\n"
    } else {
        "expression.call as=$call function=$bad\ntype.argument parent=$call index=0 type=@grown\nexpression.i64 as=$seven value=7\nexpression.bool as=$stop value=true\nexpression.if as=$bad-body condition=$stop when-true=$seven when-false=$call\n"
    };
    format!(
        "request base={} idempotency=finite-invalid-{named}\nreference.owner as=$module package=local class=module name=alternation\ntype.parameter as=@T parameter=$T\ntype.list as=@grown item=@T\n{body}create.function as=$bad module=$module name=expanding visibility=private result=i64 effect=pure body=$bad-body\nadd.type-parameter as=$T declaration=$bad name=T\n",
        revision
    )
}

fn reject(context: &mut Context, package: &Package, named: bool) -> Result<(), DevError> {
    let before = crate::authority::observe_graph_authority(&package.path)?;
    let path = context
        .evidence
        .join(format!("finite-invalid-{named}.lkjc"));
    fs::write(&path, rejected_request(&package.revision, named))?;
    let index = context.command_index();
    let result = context.cli(
        Some(&package.path),
        &[
            "change",
            "plan",
            "--input-file",
            &path.display().to_string(),
        ],
        false,
    )?;
    require(
        field(&result, "diagnostic", "code")? == "kernel_callable_expansion",
        "expanding scheme escaped semantic publication admission",
    )?;
    require(
        crate::authority::observe_graph_authority(&package.path)? == before,
        "rejected expanding request changed accepted authority or idempotency",
    )?;
    context.receipt.observations.insert(
        format!("finite_public_rejection_{named}"),
        json!({"command":index,"revision":package.revision,"authority":before.inventory_sha256})
            .to_string(),
    );
    Ok(())
}

fn validate_rejections(receipt: &Receipt, root: &Path) -> Result<(), DevError> {
    let library: Value = serde_json::from_str(
        receipt
            .observations
            .get("finite_library")
            .ok_or_else(|| DevError::corrupt("current finite source binding absent"))?,
    )?;
    let inventory_index = library["inventory"]
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .ok_or_else(|| DevError::corrupt("finite source inventory index"))?;
    let file_index = inventory_index
        .checked_add(1)
        .ok_or_else(|| DevError::corrupt("finite source inventory overflow"))?;
    let transport = library["transport"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("finite source transport absent"))?;
    let inventory = offline_package_inventory(
        &process::read_bounded(
            &root.join(format!("transport-{file_index}.lkjp")),
            MAXIMUM_CONTAINER_BYTES,
        )?,
        transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let producer = receipt
        .producer_inventories
        .get(inventory_index)
        .ok_or_else(|| DevError::corrupt("finite source producer inventory absent"))?;
    verify_producer_inventory(producer, &inventory)?;
    require(
        receipt.inventories.get(inventory_index) == Some(&inventory)
            && receipt
                .transport_digests
                .get(inventory_index)
                .is_some_and(|digest| digest == transport)
            && library["package"] == producer.package
            && library["revision"] == producer.semantic_revision,
        "finite refusal source differs from its independently admitted current export",
    )?;
    let mut commands = Vec::new();
    for named in [false, true] {
        let value: Value = serde_json::from_str(
            receipt
                .observations
                .get(&format!("finite_public_rejection_{named}"))
                .ok_or_else(|| DevError::corrupt("current finite rejection evidence absent"))?,
        )?;
        let index = value["command"]
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or_else(|| DevError::corrupt("finite rejection command index"))?;
        commands.push(index);
        let revision = value["revision"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("finite rejected source revision"))?;
        require(
            revision == producer.semantic_revision,
            "finite refusal changed its admitted source base",
        )?;
        let filename = format!("finite-invalid-{named}.lkjc");
        require(
            process::read_bounded(&root.join(&filename), MAXIMUM_OUTPUT_BYTES)?
                == rejected_request(revision, named).as_bytes()
                && value["authority"]
                    .as_str()
                    .is_some_and(|digest| digest.len() == 64),
            "finite refusal lost its exact untaken or unused source and unchanged authority",
        )?;
        let command = receipt
            .commands
            .get(index)
            .ok_or_else(|| DevError::corrupt("finite public rejection command missing"))?;
        require(
            !command.expects_success
                && command.observation.reported_code() == Some(2)
                && command.command
                    == [
                        receipt.host_path.clone(),
                        "--project".into(),
                        Path::new(&receipt.isolated_root)
                            .join("finite-library")
                            .display()
                            .to_string(),
                        "change".into(),
                        "plan".into(),
                        "--input-file".into(),
                        Path::new(&receipt.evidence_root)
                            .join(filename)
                            .display()
                            .to_string(),
                    ],
            "finite refusal did not use the exact copied candidate and authored request",
        )?;
        let records = parse_records(
            "finite-public-refusal",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("finite public refusal output malformed"))?;
        require(
            field(&records, "diagnostic", "code")? == "kernel_callable_expansion"
                && records.iter().all(|record| record.operation != "execution"),
            "finite public refusal changed its semantic failure or executed the invalid candidate",
        )?;
    }
    require(
        commands[0] < commands[1],
        "finite public refusal commands duplicated or reordered",
    )
}

fn deployment(
    bundle: &Path,
    artifact: &str,
    target: &str,
    group: &str,
) -> Result<PathBuf, DevError> {
    let descriptor = bundle.join(format!("{group}-{target}.deployment.json"));
    let value = json!({
        "artifact":artifact,"target":target,"listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),
        "configuration":{"left-prefix":{"kind":"text","value":"I"}},"secrets":[],
        "grants":[
            {"requirement":"config","sharing_domain":format!("finite-{group}-configuration"),"authority_revision":"93".repeat(32),"adapter":{"kind":"configuration"}},
            {"requirement":"data","sharing_domain":format!("finite-{group}-data"),"authority_revision":"94".repeat(32),"adapter":{"kind":"data","root":"data","namespace":format!("finite-{group}"),"limits":lkjscript::platform::data::DataLimits::default()}}
        ]
    });
    fs::write(&descriptor, evidence::encode_json(&value)?)?;
    Ok(descriptor)
}

type FiniteCase = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);
const CASES: &[FiniteCase] = &[
    (
        "pure-six",
        "original.lkja",
        "main",
        "pure",
        "[6]",
        Some("I7;Tx;I7;Tx;I7;Tx;"),
    ),
    (
        "pure-zero",
        "original.lkja",
        "main",
        "pure",
        "[0]",
        Some(""),
    ),
    ("reset", "original.lkja", "reset", "pure", "[3]", Some("7")),
    (
        "task-six",
        "original.lkja",
        "task",
        "six",
        "[6,\"x\"]",
        Some("I7;Tx;I7;Tx;I7;Tx;"),
    ),
    (
        "trace-six",
        "original.lkja",
        "trace",
        "six",
        "[]",
        Some("I7;Tx;I7;Tx;I7;Tx;"),
    ),
    (
        "task-zero",
        "original.lkja",
        "task",
        "zero",
        "[0,\"trap\"]",
        Some(""),
    ),
    (
        "trace-zero",
        "original.lkja",
        "trace",
        "zero",
        "[]",
        Some(""),
    ),
    (
        "task-failure",
        "original.lkja",
        "task",
        "failure",
        "[6,\"trap\"]",
        None,
    ),
    (
        "trace-failure",
        "original.lkja",
        "trace",
        "failure",
        "[]",
        Some("I7;"),
    ),
    (
        "edited",
        "replacement.lkja",
        "main",
        "later",
        "[6]",
        Some("edited:I7;Tx;I7;Tx;I7;Tx;"),
    ),
    (
        "replaced-reset",
        "replacement.lkja",
        "reset",
        "later",
        "[3]",
        Some("8"),
    ),
    (
        "later-task",
        "replacement.lkja",
        "task",
        "later",
        "[6,\"x\"]",
        Some("I7;Tx;I7;Tx;I7;Tx;"),
    ),
    (
        "later-trace",
        "replacement.lkja",
        "trace",
        "later",
        "[]",
        Some("I7;Tx;I7;Tx;I7;Tx;"),
    ),
];

fn artifact_identities(root: &Path) -> Result<BTreeMap<&'static str, String>, DevError> {
    ["original.lkja", "replacement.lkja"]
        .into_iter()
        .map(|artifact| {
            let bytes = process::read_bounded(
                &root.join(format!("finite-{artifact}")),
                MAXIMUM_CONTAINER_BYTES,
            )?;
            let identity = lkjscript::platform::contributor::strict_artifact_identity_probe(&bytes)
                .map_err(|error| DevError::corrupt(error.to_string()))?;
            Ok((artifact, identity))
        })
        .collect()
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    context.cli(None, &["capabilities", "--section", "change"], true)?;
    for name in ["i64-to-text", "text-concat", "i64-equal"] {
        context.cli(
            None,
            &[
                "package", "builtin", "query", "owners", "--kind", "external", "--name", name,
            ],
            true,
        )?;
    }
    let mut library = context.new_package("finite-library")?;
    context.stage(&library, standard)?;
    context.apply(
        &mut library,
        &format!(
            "{}{}",
            binding("add", standard),
            include_str!("finite.producer.lkjc")
        ),
    )?;
    context.cli(Some(&library.path), &["check"], true)?;
    reject(context, &library, false)?;
    reject(context, &library, true)?;
    let inventory_index = context.inventory_index();
    context.export(&mut library)?;
    context.receipt.observations.insert("finite_library".into(), json!({"inventory":inventory_index,"package":library.id,"revision":library.revision,"transport":library.transport}).to_string());
    let mut consumer = context.new_package("finite-consumer")?;
    context.stage(&consumer, standard)?;
    context.stage(&consumer, &library)?;
    context.apply(
        &mut consumer,
        &format!(
            "{}{}{}{}",
            binding("add", standard),
            binding("add", &library),
            selection(&library),
            include_str!("finite.consumer.lkjc")
        ),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    let bundle = context.root.join("finite-bundle");
    fs::create_dir(&bundle)?;
    context.cli(
        Some(&consumer.path),
        &[
            "build",
            "--output",
            &bundle.join("original.lkja").display().to_string(),
        ],
        true,
    )?;
    neutral(context, &consumer)?;
    context.apply(&mut library, "expression.i64 as=$eight value=8\nreplace.body function=alternation/finite-reset body=$eight\n")?;
    context.export(&mut library)?;
    fs::remove_dir_all(&library.path)?;
    // The consumer repairs/reviews its exact selection using retained transport alone.
    context.stage(&consumer, &library)?;
    context.apply(
        &mut consumer,
        &format!(
            "{}{}{}",
            binding("replace", &library),
            selection(&library),
            include_str!("finite.edit.lkjc")
        ),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.cli(
        Some(&consumer.path),
        &[
            "build",
            "--output",
            &bundle.join("replacement.lkja").display().to_string(),
        ],
        true,
    )?;
    for name in ["original.lkja", "replacement.lkja"] {
        fs::copy(
            bundle.join(name),
            context.evidence.join(format!("finite-{name}")),
        )?;
    }
    let identities = artifact_identities(&context.evidence)?;
    fs::remove_dir_all(&consumer.path)?;
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
    let mut commands = Vec::new();
    for (label, artifact, target, group, arguments, expected) in CASES {
        let descriptor = deployment(&bundle, artifact, target, group)?;
        fs::copy(
            &descriptor,
            context
                .evidence
                .join(format!("finite-{label}.deployment.json")),
        )?;
        let index = context.command_index();
        let output = context.cli(
            None,
            &[
                "run",
                "--deployment",
                &descriptor.display().to_string(),
                "--arguments",
                arguments,
            ],
            expected.is_some(),
        )?;
        if let Some(expected) = expected {
            require(
                field(&output, "execution", "artifact")? == identities[*artifact],
                "finite execution used another artifact than its strict current bundle",
            )?;
            let value: Value = serde_json::from_str(&field(&output, "execution", "value")?)?;
            let expected = if *target == "reset" {
                json!(
                    expected
                        .parse::<i64>()
                        .map_err(|_| DevError::corrupt("finite reset oracle"))?
                )
            } else {
                json!(expected)
            };
            require(
                value == expected,
                &format!("finite case {label}: actual {value}, expected {expected}"),
            )?;
        } else {
            require(
                field(&output, "diagnostic", "code")?.contains("division"),
                "selected callback trap did not retain division failure",
            )?;
        }
        commands.push(index);
    }
    context
        .receipt
        .observations
        .insert("finite_commands".into(), serde_json::to_string(&commands)?);
    context
        .receipt
        .observations
        .insert("finite_producers_removed".into(), "true".into());
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    validate_rejections(receipt, root)?;
    let identities = artifact_identities(root)?;
    let commands: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("finite_commands")
            .ok_or_else(|| DevError::corrupt("finite callable command inventory missing"))?,
    )?;
    require(
        commands.len() == CASES.len() && commands.windows(2).all(|w| w[0] < w[1]),
        "finite callable commands omitted or duplicated",
    )?;
    require(
        receipt
            .observations
            .get("finite_producers_removed")
            .is_some_and(|s| s == "true"),
        "finite supplier removal missing",
    )?;
    for (index, (label, artifact, target, group, arguments, expected)) in commands.iter().zip(CASES)
    {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("finite command index missing"))?;
        let descriptor = Path::new(&receipt.isolated_root)
            .join("finite-bundle")
            .join(format!("{group}-{target}.deployment.json"));
        require(
            command.command
                == [
                    receipt.host_path.clone(),
                    "run".into(),
                    "--deployment".into(),
                    descriptor.display().to_string(),
                    "--arguments".into(),
                    (*arguments).into(),
                ],
            "finite call used different invocation or executable",
        )?;
        let output = process::read_bounded(
            &root.join(&command.observation.stdout().path),
            MAXIMUM_OUTPUT_BYTES,
        )?;
        let records = parse_records(label, &output)
            .map_err(|e| DevError::corrupt(format!("finite output: {e:?}")))?;
        if let Some(expected) = expected {
            require(
                field(&records, "execution", "artifact")? == identities[*artifact],
                "finite receipt substituted the independently admitted current artifact",
            )?;
            let value: Value = serde_json::from_str(&field(&records, "execution", "value")?)?;
            require(
                value
                    == if *target == "reset" {
                        json!(
                            expected
                                .parse::<i64>()
                                .map_err(|_| DevError::corrupt("reset oracle"))?
                        )
                    } else {
                        json!(expected)
                    },
                "finite output or persisted observation order differs",
            )?;
            let observation: Value =
                serde_json::from_str(&field(&records, "execution", "production-observation")?)?;
            let calls = if *target == "trace" {
                1
            } else if *target == "task" && *arguments != "[0,\"trap\"]" {
                21
            } else {
                0
            };
            require(
                observation["capability_calls"] == calls,
                "finite invocation duplicated or omitted capability effects",
            )?;
            let cleanup: Value = serde_json::from_str(&field(&records, "execution", "cleanup")?)?;
            require(
                cleanup["admission_stopped"] == true
                    && cleanup["remaining_tasks"] == 0
                    && cleanup["cleanup_failures"] == json!([]),
                "finite invocation cleanup is incomplete",
            )?;
            if *label == "pure-six" {
                validate_neutral(receipt, root, &field(&records, "execution", "artifact")?)?;
            }
        } else {
            require(
                !command.expects_success
                    && field(&records, "diagnostic", "code")?.contains("division"),
                "finite trap classification missing",
            )?;
            require(
                field(&records, "diagnostic", "notes")?
                    .contains("remaining-owned-tasks=0 failures=0"),
                "finite trap lost joined cleanup evidence",
            )?;
        }
    }
    Ok(Vec::new())
}

fn neutral(context: &mut Context, consumer: &Package) -> Result<(), DevError> {
    let index = context.runner_index();
    let project = consumer.path.clone();
    #[allow(
        clippy::result_large_err,
        reason = "direct semantic boundary callbacks preserve complete typed diagnostics"
    )]
    let probe = || lkjscript::platform::contributor::finite_callable_execution_probe(&project);
    context.probe_json(
        "finite-neutral",
        "contributor.finite_callable_execution_probe",
        &[project.display().to_string()],
        probe,
    )?;
    context
        .receipt
        .observations
        .insert("finite_neutral_runner".into(), index.to_string());
    Ok(())
}

fn validate_neutral(receipt: &Receipt, root: &Path, artifact: &str) -> Result<(), DevError> {
    let index: usize = receipt
        .observations
        .get("finite_neutral_runner")
        .ok_or_else(|| DevError::corrupt("finite neutral runner absent"))?
        .parse()
        .map_err(|_| DevError::corrupt("finite runner index"))?;
    let runner = receipt
        .runners
        .get(index)
        .ok_or_else(|| DevError::corrupt("finite runner absent"))?;
    require(
        index.checked_add(1) == Some(receipt.runners.len())
            && runner.as_semantic().is_some()
            && runner.returned_success(),
        "finite neutral runner did not complete",
    )?;
    verify_observation_files(runner, &receipt.files, "finite-neutral")?;
    let call = runner
        .as_semantic()
        .ok_or_else(|| DevError::corrupt("finite neutral linked call missing"))?;
    require(
        call.operation == "contributor.finite_callable_execution_probe"
            && call.arguments
                == [Path::new(&receipt.isolated_root)
                    .join("finite-consumer")
                    .display()
                    .to_string()]
            && call.elapsed_nanoseconds <= Duration::from_secs(180).as_nanos() as u64,
        "finite neutral explicit fixture or time bound changed",
    )?;
    let value: Value = serde_json::from_slice(&process::read_bounded(
        &root.join("finite-neutral.stdout"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        value["artifact"] == artifact
            && value["live_effects_replayed"] == false
            && value["cleanup_complete"] == true,
        "finite neutral source or cleanup binding differs",
    )?;
    let rows = value["rows"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("finite neutral rows"))?;
    require(rows.len() == 10, "finite neutral cases missing")?;
    for (index, row) in rows.iter().enumerate() {
        let case = ["none", "empty", "trap", "cancelled", "quota"][index % 5];
        let count = [6, 0, 1, 2, 2][index % 5];
        let expected: Vec<_> = (0..count)
            .map(|i| if i % 2 == 0 { "I7;" } else { "Tx;" })
            .collect();
        let mut events = Vec::new();
        for i in 0..count {
            if i % 2 == 0 {
                events.push("configuration");
            }
            events.extend(["begin", "get", "put", "commit"]);
        }
        if case == "cancelled" {
            events.extend(["configuration", "begin", "get", "put", "rollback"]);
        }
        if case == "quota" {
            events.push("configuration");
        }
        let counters = row["observation"]
            .as_object()
            .ok_or_else(|| DevError::corrupt("finite neutral counters absent"))?;
        let live: Vec<_> = counters
            .iter()
            .filter(|(key, _)| key.starts_with("live_") && key.ends_with("_after"))
            .collect();
        require(
            row["tier"] == if index < 5 { "production" } else { "reference" }
                && row["case"] == case
                && row["commits"] == json!(expected)
                && row["events"] == json!(events)
                && live.len() == if index < 5 { 6 } else { 8 }
                && live.iter().all(|(_, value)| value.as_u64() == Some(0))
                && row["cleanup_complete"] == true,
            "finite neutral value/effect sequence differs",
        )?;
        let output = &row["output"];
        require(
            match case {
                "none" | "empty" => output["value"] == expected.concat(),
                "trap" => {
                    output["failure"]["code"]
                        == if index < 5 {
                            "normalized_integer_division"
                        } else {
                            "reference_integer_division"
                        }
                }
                "cancelled" => output["failure"]["class"] == "cancelled",
                "quota" => output["failure"]["class"] == "resource",
                _ => false,
            },
            "finite neutral failure classification differs",
        )?;
    }
    Ok(())
}

pub(super) fn read_focused(path: &Path, verifier: &Path) -> Result<(), DevError> {
    let bytes = process::read_bounded(path, 64 * 1024 * 1024)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("finite receipt root"))?;
    require(
        matches!(
            receipt.schema.as_str(),
            "lkjscript-offline-finite-callable-semantic-2"
                | "lkjscript-offline-requirement-parameters-semantic-2"
                | "lkjscript-offline-f64-semantic-2"
                | "lkjscript-offline-parameter-type-semantic-2"
                | "lkjscript-offline-persistent-maps-semantic-2"
        ) && receipt.status == "fresh passed"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && !Path::new(&receipt.isolated_root).exists(),
        "finite child classification or cleanup missing",
    )?;
    require(
        evidence::encode_json(&receipt)? == bytes
            && receipt.verifier_sha256 == digest_file(verifier, MAXIMUM_EXECUTABLE_BYTES)?,
        "finite child bytes or executables differ",
    )?;
    verify_file_inventory(&receipt, root)?;
    require(
        receipt
            .environment_names
            .windows(2)
            .all(|pair| pair[0] < pair[1])
            && Path::new(&receipt.isolated_root).is_absolute()
            && Path::new(&receipt.host_path).is_absolute()
            && Path::new(&receipt.host_path) == verifier.canonicalize()?
            && Path::new(&receipt.evidence_root) == root.canonicalize()?,
        "finite execution paths or environment binding differ",
    )?;
    let _ = if receipt.schema == "lkjscript-offline-requirement-parameters-semantic-2" {
        super::requirements::validate(&receipt, root)?
    } else if receipt.schema == "lkjscript-offline-f64-semantic-2" {
        require(
            receipt.inventories.len() == 3,
            "F64 focused source inventories incomplete",
        )?;
        super::f64::validate(&receipt, root, 3)?
    } else if receipt.schema == "lkjscript-offline-parameter-type-semantic-2" {
        require(
            receipt.inventories.len() == 4,
            "parameter-type focused source inventories incomplete",
        )?;
        super::parameter_type::validate(&receipt, root, 0..4)?;
        Vec::new()
    } else if receipt.schema == "lkjscript-offline-persistent-maps-semantic-2" {
        require(
            receipt.inventories.len() == 4,
            "map focused source inventories incomplete",
        )?;
        super::maps::validate(&receipt, root, 0..4)?;
        Vec::new()
    } else {
        validate(&receipt, root)?
    };
    for (index, command) in receipt.commands.iter().enumerate() {
        require(
            command.observation.as_semantic().is_some()
                && command.command.first() == Some(&receipt.host_path),
            "focused source fixture did not use the linked host",
        )?;
        command
            .observation
            .require_outcome(command.expects_success)?;
        command
            .observation
            .require_arguments(&command.command[1..])?;
        require(
            command.observation.elapsed_nanoseconds() <= MAXIMUM_LINKED_CALL_TIME.as_nanos() as u64,
            "focused linked operation exceeded its time bound",
        )?;
        let response = parse_records(
            "focused-semantic-response",
            &process::read_bounded(
                &root.join(command.observation.stdout().path.clone()),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("focused semantic response malformed"))?;
        command.observation.require_public_response(&response)?;
        verify_observation_files(
            &command.observation,
            &receipt.files,
            &format!("command-{index:04}"),
        )?;
    }
    Ok(())
}
