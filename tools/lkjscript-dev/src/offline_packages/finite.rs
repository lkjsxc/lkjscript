//! Designed finite-callable composition and authentic predecessor repair through public commands.
use super::*;
use serde_json::{Value, json};

mod predecessor;

pub(super) fn upgrade(context: &mut Context) -> Result<(), DevError> {
    predecessor::workflow(context)
}

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

fn reject(context: &mut Context, package: &Package, named: bool) -> Result<(), DevError> {
    let before = crate::authority::observe_graph_authority(&package.path)?;
    let body = if named {
        "expression.function-value as=$call function=$bad\ntype.argument parent=$call index=0 type=@grown\nexpression.i64 as=$seven value=7\nexpression.sequence as=$bad-body\nexpression.argument parent=$bad-body index=0 expression=$call\nexpression.argument parent=$bad-body index=1 expression=$seven\n"
    } else {
        "expression.call as=$call function=$bad\ntype.argument parent=$call index=0 type=@grown\nexpression.i64 as=$seven value=7\nexpression.bool as=$stop value=true\nexpression.if as=$bad-body condition=$stop when-true=$seven when-false=$call\n"
    };
    let request = format!(
        "request base={} idempotency=finite-invalid-{named}\nreference.owner as=$module package=local class=module name=alternation\ntype.parameter as=@T parameter=$T\ntype.list as=@grown item=@T\n{body}create.function as=$bad module=$module name=expanding visibility=private result=i64 effect=pure body=$bad-body\nadd.type-parameter as=$T declaration=$bad name=T\n",
        package.revision
    );
    let path = context
        .evidence
        .join(format!("finite-invalid-{named}.lkjc"));
    fs::write(&path, request)?;
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

const HOSTILE: &[(&str, &[u8])] = &[
    (
        "direct",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding-direct.lkja"
        )),
    ),
    (
        "named",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding-named.lkja"
        )),
    ),
];

fn reject_artifacts(context: &mut Context, bundle: &Path) -> Result<(), DevError> {
    let mut commands = Vec::new();
    for (label, bytes) in HOSTILE {
        let artifact = format!("expanding-{label}.lkja");
        fs::write(bundle.join(&artifact), bytes)?;
        fs::write(context.evidence.join(format!("finite-{artifact}")), bytes)?;
        let descriptor = bundle.join(format!("expanding-{label}.deployment.json"));
        fs::write(
            &descriptor,
            evidence::encode_json(&json!({
                "artifact":artifact,"target":"main","listen":null,"http":null,"session":null,"worker":null,
                "streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]
            }))?,
        )?;
        fs::copy(
            &descriptor,
            context
                .evidence
                .join(format!("finite-expanding-{label}.deployment.json")),
        )?;
        commands.push(context.receipt.commands.len());
        let output = context.cli(
            None,
            &[
                "run",
                "--deployment",
                &descriptor.display().to_string(),
                "--arguments",
                "[]",
            ],
            false,
        )?;
        require(
            field(&output, "diagnostic", "code")? == "kernel_callable_expansion"
                && field(&output, "diagnostic", "class")? == "semantic"
                && output.iter().all(|record| record.operation != "execution"),
            "coherent hostile artifact escaped semantic admission before execution",
        )?;
    }
    context.receipt.observations.insert(
        "finite_artifact_rejections".into(),
        serde_json::to_string(&commands)?,
    );
    Ok(())
}

fn validate_artifact_rejections(receipt: &Receipt, root: &Path) -> Result<(), DevError> {
    let commands: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("finite_artifact_rejections")
            .ok_or_else(|| DevError::corrupt("finite artifact rejection inventory missing"))?,
    )?;
    require(
        commands.len() == HOSTILE.len() && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "finite hostile applications omitted or duplicated",
    )?;
    for (index, (label, bytes)) in commands.iter().zip(HOSTILE) {
        require(
            process::read_bounded(
                &root.join(format!("finite-expanding-{label}.lkja")),
                MAXIMUM_OUTPUT_BYTES,
            )? == *bytes,
            "finite artifact negative does not bind the complete rehashed fixture",
        )?;
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("finite rejection command missing"))?;
        let descriptor = Path::new(&receipt.isolated_root)
            .join("finite-bundle")
            .join(format!("expanding-{label}.deployment.json"));
        require(
            !command.expects_success
                && command.command
                    == [
                        receipt.pinned_runtime_path.clone(),
                        "run".into(),
                        "--deployment".into(),
                        descriptor.display().to_string(),
                        "--arguments".into(),
                        "[]".into(),
                    ],
            "finite artifact negative used different runtime or invocation",
        )?;
        let output = parse_records(
            "finite-artifact",
            &process::read_bounded(
                &root.join(&command.observation.stdout.path),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|e| DevError::corrupt(format!("finite artifact output: {e:?}")))?;
        require(
            field(&output, "diagnostic", "class")? == "semantic"
                && field(&output, "diagnostic", "code")? == "kernel_callable_expansion"
                && output.iter().all(|record| record.operation != "execution"),
            "finite artifact failed outside semantic application admission",
        )?;
    }
    Ok(())
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
    context.export(&mut library)?;
    let mut consumer = context.new_package("finite-consumer")?;
    context.stage(&consumer, standard)?;
    context.stage(&consumer, &library)?;
    let supplier = context.evidence.join("finite-expanding-supplier.lkjp");
    fs::write(
        &supplier,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/finite-callable-predecessor/expanding.lkjp"
        )),
    )?;
    let before = crate::authority::observe_graph_authority(&consumer.path)?;
    let rejected = context.cli(
        Some(&consumer.path),
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            "package_transport_eadf507fa02900fdc122d45b325cab3d123d329af444fd4542501e55d3a45421",
            "--input-file",
            &supplier.display().to_string(),
        ],
        false,
    )?;
    require(
        field(&rejected, "diagnostic", "code")? == "kernel_callable_expansion"
            || field(&rejected, "diagnostic", "message")?.contains("kernel_callable_expansion"),
        "complete supplier body escaped current callable admission",
    )?;
    require(
        crate::authority::observe_graph_authority(&consumer.path)? == before,
        "invalid supplier changed readiness or accepted authority",
    )?;
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
    reject_artifacts(context, &bundle)?;
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
        let index = context.receipt.commands.len();
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
    predecessor::workflow(context)?;
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    validate_artifact_rejections(receipt, root)?;
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
    for (index, (label, _, target, group, arguments, expected)) in commands.iter().zip(CASES) {
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
                    receipt.pinned_runtime_path.clone(),
                    "run".into(),
                    "--deployment".into(),
                    descriptor.display().to_string(),
                    "--arguments".into(),
                    (*arguments).into(),
                ],
            "finite call used different invocation or executable",
        )?;
        let output = process::read_bounded(
            &root.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?;
        let records = parse_records(label, &output)
            .map_err(|e| DevError::corrupt(format!("finite output: {e:?}")))?;
        if let Some(expected) = expected {
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
    predecessor::validate(receipt, root)
}

fn neutral(context: &mut Context, consumer: &Package) -> Result<(), DevError> {
    let verifier = std::env::current_exe()?;
    let spec = process::ProcessSpec {
        command: vec![
            verifier.display().to_string(),
            "effect-probe".into(),
            consumer.path.display().to_string(),
            "--case".into(),
            "finite-callable".into(),
        ],
        cwd: context.root.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(180),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context.evidence.join("finite-neutral.stdout"),
        stderr_path: context.evidence.join("finite-neutral.stderr"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&spec, &context.evidence);
    context.receipt.observations.insert(
        "finite_neutral_runner".into(),
        context.receipt.runners.len().to_string(),
    );
    context.receipt.runners.push(observation.clone());
    require(
        observation.status == process::ProcessStatus::Passed,
        "finite neutral reference/production probe failed",
    )
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
        index + 1 == receipt.runners.len()
            && runner.status == process::ProcessStatus::Passed
            && runner.exit_code == Some(0)
            && runner.signal.is_none(),
        "finite neutral runner did not complete",
    )?;
    verify_observation_files(runner, &receipt.files, "finite-neutral")?;
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

pub(super) fn read_focused(path: &Path, candidate: &Path, verifier: &Path) -> Result<(), DevError> {
    let bytes = process::read_bounded(path, 64 * 1024 * 1024)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("finite receipt root"))?;
    require(
        matches!(
            receipt.schema.as_str(),
            "lkjscript-offline-finite-callable-1"
                | "lkjscript-offline-validator-upgrade-1"
                | "lkjscript-offline-requirement-parameters-2"
        ) && receipt.status == "fresh passed"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && !Path::new(&receipt.isolated_root).exists(),
        "finite child classification or cleanup missing",
    )?;
    require(
        evidence::encode_json(&receipt)? == bytes
            && receipt.candidate_sha256 == digest_file(candidate, MAXIMUM_EXECUTABLE_BYTES)?
            && receipt.copied_candidate_sha256 == receipt.candidate_sha256
            && receipt.verifier_sha256 == digest_file(verifier, MAXIMUM_EXECUTABLE_BYTES)?,
        "finite child bytes or executables differ",
    )?;
    verify_file_inventory(&receipt, root)?;
    require(
        receipt.environment_names == ["LANG"]
            && Path::new(&receipt.isolated_root).is_absolute()
            && Path::new(&receipt.pinned_runtime_path).is_absolute()
            && Path::new(&receipt.pinned_runtime_path) == candidate.canonicalize()?
            && Path::new(&receipt.evidence_root) == root.canonicalize()?,
        "finite execution paths or environment binding differ",
    )?;
    let pinned = if receipt.schema == "lkjscript-offline-validator-upgrade-1" {
        predecessor::validate(&receipt, root)?
    } else if receipt.schema == "lkjscript-offline-requirement-parameters-2" {
        super::requirements::validate(&receipt, root)?
    } else {
        validate(&receipt, root)?
    };
    for (index, command) in receipt.commands.iter().enumerate() {
        require(
            command.cwd == receipt.isolated_root
                && command.observation.signal.is_none()
                && command
                    .observation
                    .exit_code
                    .is_some_and(|code| (code == 0) == command.expects_success),
            "finite child command status or environment differs",
        )?;
        let installed =
            pinned.contains(&index) || command.command.get(1).is_some_and(|c| c == "run");
        require(
            command.command.first()
                == Some(&if installed {
                    receipt.pinned_runtime_path.clone()
                } else {
                    Path::new(&receipt.isolated_root)
                        .join("lkjscript")
                        .display()
                        .to_string()
                }),
            "finite child executable differs",
        )?;
        verify_observation_files(
            &command.observation,
            &receipt.files,
            &format!("command-{index:04}"),
        )?;
    }
    Ok(())
}
