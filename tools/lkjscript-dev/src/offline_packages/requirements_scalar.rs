//! Fresh current scalar source, immutable transport and detached execution controls.
use super::*;
use base64::Engine as _;

const MATERIAL: &str = "requirement-scalars";
const PRODUCER: &str = include_str!("requirements.scalar.lkjc");
const CONSUMER: &str = include_str!("requirements.scalar-consumer.lkjc");
const EDIT: &str = "reference.owner as=$module package=local class=module name=current-scalars\nrename.owner owner=$module name=current-scalars-edited\n";
const CASES: [(&str, &str); 4] = [
    ("producer", "encode-integer"),
    ("producer", "encode-text"),
    ("consumer", "encode-integer"),
    ("consumer", "encode-text"),
];

fn descriptor(artifact: &str, target: &str) -> Value {
    json!({"artifact":format!("{artifact}.lkja"),"target":target,"listen":null,"http":null,"session":null,"worker":null,"streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]})
}

fn source(context: &mut Context, package: &mut Package, label: &str) -> Result<(), DevError> {
    package.container = context.root.join(format!("{MATERIAL}-{label}.lkjp"));
    let exported = context.cli(
        Some(&package.path),
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &package.container.display().to_string(),
        ],
        true,
    )?;
    package.logical = field(&exported, "package", "package-revision")?;
    package.transport = field(&exported, "package", "transport")?;
    fs::copy(
        &package.container,
        context.evidence.join(format!("{MATERIAL}-{label}.lkjp")),
    )?;
    Ok(())
}

fn binding_probe(root: &Path, label: &str, transport: &str) -> Result<Value, DevError> {
    let artifact = process::read_bounded(
        &root.join(format!("{MATERIAL}-{label}.lkja")),
        MAXIMUM_CONTAINER_BYTES,
    )?;
    let source = process::read_bounded(
        &root.join(format!("{MATERIAL}-{label}.lkjp")),
        MAXIMUM_CONTAINER_BYTES,
    )?;
    let identity = lkjscript::platform::contributor::strict_artifact_identity_probe(&artifact)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
        &artifact, &source, transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        bound["bundle"] == identity && bound["source_transport"] == transport,
        "current scalar artifact is not bound to its complete source transport",
    )?;
    Ok(bound)
}

fn result(output: &[CompactRecord], bound: &Value, target: &str) -> Result<(), DevError> {
    let value = match target {
        "encode-integer" => json!(13),
        "encode-text" => json!("a!"),
        _ => return Err(DevError::corrupt("unknown current scalar oracle")),
    };
    let expected =
        json!({"$bytes":base64::engine::general_purpose::STANDARD.encode(typed_bytes(&value)?)});
    require(
        field(output, "result", "status")? == "success"
            && field(output, "result", "command")? == "run"
            && field(output, "execution", "artifact")?
                == bound["bundle"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("scalar artifact identity"))?
            && field(output, "execution", "package")?
                == bound["package"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("scalar source package"))?
            && field(output, "execution", "revision")?
                == bound["revision"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("scalar source revision"))?
            && serde_json::from_str::<Value>(&field(output, "execution", "value")?)? == expected
            && !output.iter().any(|record| record.operation == "diagnostic"),
        "current scalar result changed independently specified bytes or source identity",
    )?;
    let cleanup: Value = serde_json::from_str(&field(output, "execution", "cleanup")?)?;
    require(
        cleanup["admission_stopped"] == true
            && cleanup["remaining_tasks"] == 0
            && cleanup["cleanup_failures"] == json!([]),
        "current scalar execution did not quiesce",
    )
}

fn author(
    context: &mut Context,
    package: &mut Package,
    label: &str,
    body: &str,
) -> Result<Value, DevError> {
    let first = context.command_index();
    let base = package.revision.clone();
    let accepted = context.apply(package, body)?;
    let input = format!("request-{first}.lkjc");
    fs::copy(
        context.evidence.join(&input),
        context.evidence.join(format!("{MATERIAL}-{label}.request")),
    )?;
    Ok(
        json!({"first":first,"base":base,"input":input,"body":body,"receipt":field(&accepted,"receipt","digest")?}),
    )
}

fn packs(project: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, DevError> {
    fs::read_dir(project.join("packs"))?
        .map(|entry| {
            let path = entry?.path();
            Ok((
                path.clone(),
                process::read_bounded(&path, MAXIMUM_CONTAINER_BYTES)?,
            ))
        })
        .collect()
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    let mut producer = context.new_package("requirement-scalar-producer")?;
    context.stage(&producer, standard)?;
    let producer_author = author(
        context,
        &mut producer,
        "producer",
        &format!("{}{PRODUCER}", binding("add", standard)),
    )?;
    source(context, &mut producer, "producer")?;
    let before = crate::authority::observe_graph_authority(&producer.path)?;
    let original_packs = packs(&producer.path)?;
    context.cli(Some(&producer.path), &["status"], true)?;
    context.cli(Some(&producer.path), &["check"], true)?;
    let bundle = context.root.join(MATERIAL);
    fs::create_dir(&bundle)?;
    context.cli(
        Some(&producer.path),
        &[
            "build",
            "--output",
            &bundle.join("producer.lkja").display().to_string(),
        ],
        true,
    )?;
    require(
        crate::authority::observe_graph_authority(&producer.path)?.head_sha256
            == before.head_sha256,
        "current scalar reads, validation or build changed accepted source",
    )?;
    for (path, bytes) in &original_packs {
        require(
            process::read_bounded(path, MAXIMUM_CONTAINER_BYTES)? == *bytes,
            "current scalar read/check/build rewrote an immutable source pack",
        )?;
    }

    let mut consumer = context.new_package("requirement-scalar-consumer")?;
    context.stage(&consumer, &producer)?;
    let consumer_author = author(
        context,
        &mut consumer,
        "consumer",
        &format!(
            "{}reference.package as=$scalars package={} package-revision={}\n{CONSUMER}",
            binding("add", &producer),
            producer.id,
            producer.logical
        ),
    )?;
    source(context, &mut consumer, "consumer")?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.cli(
        Some(&consumer.path),
        &[
            "build",
            "--output",
            &bundle.join("consumer.lkja").display().to_string(),
        ],
        true,
    )?;
    for label in ["producer", "consumer"] {
        fs::copy(
            bundle.join(format!("{label}.lkja")),
            context.evidence.join(format!("{MATERIAL}-{label}.lkja")),
        )?;
    }
    let producer_binding = binding_probe(&context.evidence, "producer", &producer.transport)?;
    let consumer_binding = binding_probe(&context.evidence, "consumer", &consumer.transport)?;
    for (package, bound) in [
        (&producer, &producer_binding),
        (&consumer, &consumer_binding),
    ] {
        require(
            bound["package"] == package.id
                && bound["revision"] == package.revision
                && bound["package_revision"] == package.logical,
            "public current scalar build changed the accepted semantic revision",
        )?;
    }
    let old_revision = producer.revision.clone();
    let before_edit_packs = packs(&producer.path)?;
    let edit = author(context, &mut producer, "edit", EDIT)?;
    require(
        producer.revision != old_revision,
        "current scalar ordinary edit retained its revision",
    )?;
    // Existing immutable packs may remain in the edited project, but their bytes cannot change.
    for (path, bytes) in before_edit_packs {
        require(
            process::read_bounded(&path, MAXIMUM_CONTAINER_BYTES)? == bytes,
            "current scalar edit rewrote accepted source pack bytes",
        )?;
    }
    context.cli(Some(&producer.path), &["check"], true)?;
    fs::remove_dir_all(&producer.path)?;
    fs::remove_dir_all(&consumer.path)?;
    let mut commands = Vec::new();
    for (label, target) in CASES {
        let name = format!("{label}-{target}.deployment.json");
        let path = bundle.join(&name);
        let bytes = evidence::encode_json(&descriptor(label, target))?;
        fs::write(&path, &bytes)?;
        fs::write(context.evidence.join(format!("{MATERIAL}-{name}")), bytes)?;
        commands.push(context.command_index());
        let output = context.cli(
            None,
            &[
                "run",
                "--deployment",
                &path.display().to_string(),
                "--arguments",
                "[]",
            ],
            true,
        )?;
        result(
            &output,
            if label == "producer" {
                &producer_binding
            } else {
                &consumer_binding
            },
            target,
        )?;
    }
    context.receipt.observations.insert(MATERIAL.into(), json!({"standard":{"id":standard.id,"revision":standard.revision,"logical":standard.logical,"transport":standard.transport},"producer_author":producer_author,"consumer_author":consumer_author,"producer_transport":producer.transport,"consumer_transport":consumer.transport,"producer_binding":producer_binding,"consumer_binding":consumer_binding,"after":producer.revision,"edit_receipt":edit["receipt"],"edit_author":edit,"original_packs_preserved":true,"sources_removed":true,"commands":commands}).to_string());
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let value: Value = serde_json::from_str(
        receipt
            .observations
            .get(MATERIAL)
            .ok_or_else(|| DevError::corrupt("current scalar evidence missing"))?,
    )?;
    require(
        value["sources_removed"] == true
            && value["original_packs_preserved"] == true
            && value["after"] != value["producer_binding"]["revision"]
            && value["edit_author"]["base"] == value["producer_binding"]["revision"]
            && value["after"]
                .as_str()
                .is_some_and(|revision| revision.starts_with("rev_") && revision.len() == 68)
            && value["edit_receipt"]
                .as_str()
                .is_some_and(|digest| digest.starts_with("receipt_object_"))
            && value["edit_receipt"] == value["edit_author"]["receipt"],
        "current scalar source preservation, edit or detached execution evidence omitted",
    )?;
    for label in ["producer", "consumer", "edit"] {
        if label != "edit" {
            let transport = value[format!("{label}_transport")]
                .as_str()
                .ok_or_else(|| DevError::corrupt("current scalar transport absent"))?;
            require(
                binding_probe(root, label, transport)? == value[format!("{label}_binding")],
                "current scalar source or artifact substituted",
            )?;
        }
        let authored = &value[format!("{label}_author")];
        let first = authored["first"]
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or_else(|| DevError::corrupt("current scalar author command absent"))?;
        let applied_index = first
            .checked_add(1)
            .ok_or_else(|| DevError::corrupt("current scalar author command overflow"))?;
        let base = authored["base"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("current scalar author base absent"))?;
        let selected = if label != "consumer" {
            &value["standard"]
        } else {
            &value["producer_binding"]
        };
        let id = if label != "consumer" {
            &selected["id"]
        } else {
            &selected["package"]
        };
        let logical = if label != "consumer" {
            &selected["logical"]
        } else {
            &selected["package_revision"]
        };
        let prelude = format!(
            "add.dependency package={} semantic-revision={} package-revision={}\n",
            id.as_str()
                .ok_or_else(|| DevError::corrupt("scalar dependency package"))?,
            selected["revision"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("scalar dependency semantic revision"))?,
            logical
                .as_str()
                .ok_or_else(|| DevError::corrupt("scalar dependency revision"))?
        );
        let body = if label == "edit" {
            EDIT.to_owned()
        } else if label == "producer" {
            format!("{prelude}{PRODUCER}")
        } else {
            format!(
                "{prelude}reference.package as=$scalars package={} package-revision={}\n{CONSUMER}",
                id.as_str()
                    .ok_or_else(|| DevError::corrupt("scalar producer package"))?,
                logical
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("scalar producer revision"))?
            )
        };
        let expected = format!("request base={base} idempotency=offline-{first}\n{body}");
        require(
            authored["body"] == body
                && authored["input"] == format!("request-{first}.lkjc")
                && process::read_bounded(
                    &root.join(format!("{MATERIAL}-{label}.request")),
                    MAXIMUM_OUTPUT_BYTES,
                )? == expected.as_bytes()
                && process::read_bounded(
                    &root.join(format!("request-{first}.lkjc")),
                    MAXIMUM_OUTPUT_BYTES,
                )? == expected.as_bytes(),
            "current scalar authoring inputs differ from the independent literal",
        )?;
        let applied = receipt
            .commands
            .get(applied_index)
            .ok_or_else(|| DevError::corrupt("current scalar apply command absent"))?;
        let planned = receipt
            .commands
            .get(first)
            .ok_or_else(|| DevError::corrupt("current scalar plan command absent"))?;
        let plan_output = parse_records(
            "current-scalar-plan",
            &process::read_bounded(
                &root.join(format!("command-{first:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("current scalar plan output malformed"))?;
        let token = field(&plan_output, "plan", "token")?;
        let review = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(
            root.join(format!("review-{first}.lkjplan")),
        )?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        let executable = receipt.host_path.clone();
        let project = Path::new(&receipt.isolated_root)
            .join(format!(
                "requirement-scalar-{}",
                if label == "edit" { "producer" } else { label }
            ))
            .display()
            .to_string();
        let input = Path::new(&receipt.evidence_root)
            .join(format!("request-{first}.lkjc"))
            .display()
            .to_string();
        require(
            planned.expects_success
                && planned.observation.reported_code() == Some(0)
                && review.token == token
                && planned.command
                    == [
                        executable.clone(),
                        "--project".into(),
                        project.clone(),
                        "change".into(),
                        "plan".into(),
                        "--input-file".into(),
                        input.clone(),
                        "--output".into(),
                        Path::new(&receipt.evidence_root)
                            .join(format!("review-{first}.lkjplan"))
                            .display()
                            .to_string(),
                    ]
                && applied.command
                    == [
                        executable,
                        "--project".into(),
                        project,
                        "change".into(),
                        "apply".into(),
                        "--input-file".into(),
                        input,
                        "--plan".into(),
                        token,
                    ],
            "current scalar authoring did not use the exact copied candidate, input and reviewed plan",
        )?;
        let output = parse_records(
            "current-scalar-author",
            &process::read_bounded(
                &root.join(format!("command-{applied_index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("current scalar apply output malformed"))?;
        let expected_revision = if label == "edit" {
            &value["after"]
        } else {
            &value[format!("{label}_binding")]["revision"]
        };
        require(
            applied.expects_success
                && applied.observation.reported_code() == Some(0)
                && field(&output, "result", "status")? == "accepted"
                && field(&output, "revision", "base")? == base
                && field(&output, "receipt", "digest")?
                    == authored["receipt"]
                        .as_str()
                        .ok_or_else(|| DevError::corrupt("scalar accepted receipt"))?
                && field(&output, "revision", "result")?
                    == expected_revision
                        .as_str()
                        .ok_or_else(|| DevError::corrupt("scalar accepted revision"))?,
            "current scalar author acceptance differs from the exact transported source",
        )?;
    }
    let commands: Vec<usize> = serde_json::from_value(value["commands"].clone())?;
    require(
        commands.len() == CASES.len() && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "current scalar exact command inventory omitted or reordered",
    )?;
    for (index, (label, target)) in commands.iter().zip(CASES) {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("current scalar command absent"))?;
        let name = format!("{label}-{target}.deployment.json");
        require(
            serde_json::from_slice::<Value>(&process::read_bounded(
                &root.join(format!("{MATERIAL}-{name}")),
                MAXIMUM_OUTPUT_BYTES,
            )?)? == descriptor(label, target)
                && command.expects_success
                && command.observation.reported_code() == Some(0)
                && command.command
                    == [
                        receipt.host_path.clone(),
                        "run".into(),
                        "--deployment".into(),
                        Path::new(&receipt.isolated_root)
                            .join(MATERIAL)
                            .join(name)
                            .display()
                            .to_string(),
                        "--arguments".into(),
                        "[]".into(),
                    ],
            "current scalar target, runtime or argument binding changed",
        )?;
        let output = parse_records(
            "current-scalars",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("current scalar output malformed"))?;
        result(&output, &value[format!("{label}_binding")], target)?;
    }
    Ok(commands)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "independent scalar receipt fault controls"
)]
mod tests {
    use super::*;

    #[test]
    fn scalar_result_rejects_substituted_source_artifact_bytes_and_cleanup() {
        let bound = json!({"bundle":"selected-artifact","package":"selected-package","revision":"selected-revision"});
        let bytes =
            base64::engine::general_purpose::STANDARD.encode(typed_bytes(&json!(13)).unwrap());
        let complete = json!({"admission_stopped":true,"remaining_tasks":0,"cleanup_failures":[]});
        let make = |artifact: &str, package: &str, revision: &str, bytes: &str, cleanup: &Value| {
            let value = json!({"$bytes":bytes}).to_string();
            let cleanup = cleanup.to_string();
            let result = lkjscript::platform::control::render_record(
                "result",
                &[("status", "success"), ("command", "run")],
            )
            .unwrap();
            let execution = lkjscript::platform::control::render_record(
                "execution",
                &[
                    ("artifact", artifact),
                    ("package", package),
                    ("revision", revision),
                    ("value", &value),
                    ("cleanup", &cleanup),
                ],
            )
            .unwrap();
            format!("{result}{execution}")
        };
        let text = make(
            "selected-artifact",
            "selected-package",
            "selected-revision",
            &bytes,
            &complete,
        );
        let read = |text: &str| parse_records("scalar-result-control", text.as_bytes()).unwrap();
        result(&read(&text), &bound, "encode-integer").unwrap();
        for changed in [
            text.replace("selected-artifact", "substituted-artifact"),
            text.replace("selected-package", "substituted-package"),
            text.replace("selected-revision", "substituted-revision"),
            make(
                "selected-artifact",
                "selected-package",
                "selected-revision",
                "AA==",
                &complete,
            ),
            make(
                "selected-artifact",
                "selected-package",
                "selected-revision",
                &bytes,
                &json!({"admission_stopped":true,"remaining_tasks":1,"cleanup_failures":[]}),
            ),
            make(
                "selected-artifact",
                "selected-package",
                "selected-revision",
                &bytes,
                &json!({"admission_stopped":false,"remaining_tasks":0,"cleanup_failures":[]}),
            ),
            format!("{text}diagnostic code=unexpected\n"),
        ] {
            assert!(result(&read(&changed), &bound, "encode-integer").is_err());
        }
        assert!(result(&read(&text), &bound, "encode-text").is_err());
        result(&read(&text), &bound, "encode-integer").unwrap();
    }
}
