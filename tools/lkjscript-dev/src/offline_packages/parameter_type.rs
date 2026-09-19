//! Public, identity-preserving evolution of an ordinary fold library and its exact consumer.
//! The driver supplies literal requests and integer arithmetic expectations; it never writes meaning.
use super::*;
use serde_json::{Value, json};

const PRODUCER: &str = include_str!("parameter_type.producer.lkjc");
const PRODUCER_EDIT: &str = include_str!("parameter_type.producer_edit.lkjc");
const CONSUMER: &str = include_str!("parameter_type.consumer.lkjc");
const CONSUMER_EDIT: &str = include_str!("parameter_type.consumer_edit.lkjc");

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evolution {
    commands: BTreeMap<String, usize>,
    sources: Vec<Source>,
    repositories_removed: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    package: String,
    revision: String,
    logical: String,
    inventory: usize,
}

fn selection(package: &Package) -> String {
    format!(
        "reference.package as=$library package={} package-revision={}\n",
        package.id, package.logical
    )
}

impl Evolution {
    fn cli(
        &mut self,
        context: &mut Context,
        label: &str,
        project: Option<&Path>,
        arguments: &[&str],
        passes: bool,
    ) -> Result<Vec<CompactRecord>, DevError> {
        require(
            self.commands
                .insert(label.to_owned(), context.receipt.commands.len())
                .is_none(),
            "duplicate parameter-type command label",
        )?;
        if project.is_none() && arguments.starts_with(&["run", "--deployment"]) && passes {
            // This authoring witness claims the copied executable outside the language checkout.
            context.cli_copied_at(&context.root.clone(), arguments)
        } else {
            context.cli(project, arguments, passes)
        }
    }

    fn author(
        &mut self,
        context: &mut Context,
        package: &mut Package,
        name: &str,
        body: &str,
    ) -> Result<(), DevError> {
        let request = context.root.join(format!("parameter-type-{name}.lkjc"));
        let review = context.root.join(format!("parameter-type-{name}.lkjplan"));
        fs::write(
            &request,
            format!(
                "request base={} idempotency=parameter-type-{name}\n{body}",
                package.revision
            ),
        )?;
        fs::copy(
            &request,
            context.evidence.join(format!("parameter-type-{name}.lkjc")),
        )?;
        let planned = self.cli(
            context,
            &format!("{name}-plan"),
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
        require(
            plan.token == field(&planned, "plan", "token")?,
            "parameter-type review differs",
        )?;
        fs::copy(
            &review,
            context
                .evidence
                .join(format!("parameter-type-{name}.lkjplan")),
        )?;
        let arguments = [
            "change",
            "apply",
            "--input-file",
            &request.display().to_string(),
            "--plan",
            &plan.token,
        ];
        let applied = self.cli(
            context,
            &format!("{name}-apply"),
            Some(&package.path),
            &arguments,
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
        if name == "producer-edit" {
            let retry = self.cli(
                context,
                "producer-retry",
                Some(&package.path),
                &arguments,
                true,
            )?;
            for (record, field_name) in [
                ("revision", "result"),
                ("receipt", "digest"),
                ("receipt", "revision-record"),
            ] {
                require(
                    field(&applied, record, field_name)? == field(&retry, record, field_name)?,
                    "accepted parameter edit retry changed its immutable result",
                )?;
            }
        }
        Ok(())
    }

    fn export(&mut self, context: &mut Context, package: &mut Package) -> Result<(), DevError> {
        context.export(package)?;
        self.sources.push(Source {
            package: package.id.clone(),
            revision: package.revision.clone(),
            logical: package.logical.clone(),
            inventory: context.receipt.inventories.len() - 1,
        });
        Ok(())
    }

    fn inspect(
        &mut self,
        context: &mut Context,
        package: &Package,
        label: &str,
        functions: &[&str],
    ) -> Result<(), DevError> {
        for function in functions {
            self.cli(
                context,
                &format!("{label}-{function}"),
                Some(&package.path),
                &[
                    "inspect",
                    "owner",
                    "pure_function",
                    &package.symbols[*function],
                    "--detail",
                    "definition",
                    "--limit",
                    "1000",
                    "--bytes",
                    "1048576",
                ],
                true,
            )?;
        }
        for kind in ["component", "port", "target"] {
            self.cli(
                context,
                &format!("{label}-{kind}"),
                Some(&package.path),
                &["query", "owners", "--kind", kind],
                true,
            )?;
        }
        Ok(())
    }

    fn reject(
        &mut self,
        context: &mut Context,
        package: &Package,
        name: &str,
        body: &str,
    ) -> Result<(), DevError> {
        let request = context.root.join(format!("parameter-type-{name}.lkjc"));
        fs::write(
            &request,
            format!("request base={}\n{body}", package.revision),
        )?;
        fs::copy(
            &request,
            context.evidence.join(format!("parameter-type-{name}.lkjc")),
        )?;
        let before = self.cli(
            context,
            &format!("{name}-before"),
            Some(&package.path),
            &["status"],
            true,
        )?;
        let function = package
            .symbols
            .get("$energy")
            .or_else(|| package.symbols.get("$command"))
            .ok_or_else(|| DevError::corrupt("parameter-type rejected fixture function missing"))?;
        let inspect = [
            "inspect",
            "owner",
            "pure_function",
            function.as_str(),
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ];
        let definition_before = self.cli(
            context,
            &format!("{name}-definition-before"),
            Some(&package.path),
            &inspect,
            true,
        )?;
        let authority = crate::authority::observe_graph_authority(&package.path)?;
        let result = self.cli(
            context,
            name,
            Some(&package.path),
            &[
                "change",
                "plan",
                "--input-file",
                &request.display().to_string(),
            ],
            false,
        )?;
        require(
            result
                .iter()
                .filter(|r| r.operation == "diagnostic")
                .any(|r| {
                    record_field(r, "code").is_ok_and(|code| {
                        code.starts_with("kernel_type_") || code.starts_with("kernel_port_")
                    })
                }),
            "incomplete parameter repair failed outside semantic type/port admission",
        )?;
        let after = self.cli(
            context,
            &format!("{name}-after"),
            Some(&package.path),
            &["status"],
            true,
        )?;
        let definition_after = self.cli(
            context,
            &format!("{name}-definition-after"),
            Some(&package.path),
            &inspect,
            true,
        )?;
        require(
            before == after
                && definition_before == definition_after
                && crate::authority::observe_graph_authority(&package.path)? == authority,
            "rejected parameter repair changed accepted meaning",
        )
    }

    fn imported_contract(
        &mut self,
        context: &mut Context,
        consumer: &Package,
        producer: &Package,
        name: &str,
    ) -> Result<(), DevError> {
        self.cli(
            context,
            name,
            Some(&consumer.path),
            &[
                "package",
                "dependency",
                "inspect",
                "owner",
                "pure_function",
                &producer.symbols["$energy"],
                "--package-revision",
                &producer.logical,
            ],
            true,
        )?;
        Ok(())
    }
}

pub(super) fn focused(context: &mut Context) -> Result<(), DevError> {
    let builtin = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id")?,
        revision: field(&builtin, "package", "revision")?,
        logical: field(&builtin, "package", "package-revision")?,
        transport: field(&builtin, "package", "transport")?,
        container: context.root.join("parameter-type-standard.lkjp"),
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

fn descriptor(artifact: &str) -> Value {
    json!({"artifact":format!("parameter-type-{artifact}.lkja"),"target":"energy",
        "listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),
        "configuration":{},"secrets":[],"grants":[]})
}

fn build(
    evolution: &mut Evolution,
    context: &mut Context,
    package: &Package,
    name: &str,
) -> Result<PathBuf, DevError> {
    let filename = format!("parameter-type-{name}.lkja");
    let artifact = context.root.join(&filename);
    evolution.cli(
        context,
        &format!("{name}-build"),
        Some(&package.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    fs::copy(&artifact, context.evidence.join(&filename))?;
    let filename = format!("parameter-type-{name}.deployment.json");
    let deployment = context.root.join(&filename);
    fs::write(&deployment, evidence::encode_json(&descriptor(name))?)?;
    fs::copy(&deployment, context.evidence.join(filename))?;
    Ok(deployment)
}

fn cases() -> Result<Vec<(&'static str, &'static str, String, u64)>, DevError> {
    let old_values: Vec<u64> = (0_u64..73).map(|i| (i * 5 + 2) % 11).collect();
    let old_expected = old_values.iter().map(|v| v * v).sum();
    let enabled = "[[{\"value\":3,\"enabled\":true},{\"value\":4,\"enabled\":true}]]";
    let disabled = "[[{\"value\":3,\"enabled\":true},{\"value\":4,\"enabled\":false}]]";
    let readings: Vec<Value> = (0_u64..4096)
        .map(|i| json!({"value":i % 8,"enabled":i % 3 != 0}))
        .collect();
    let expected = (0_u64..4096)
        .filter(|i| i % 3 != 0)
        .map(|i| (i % 8) * (i % 8))
        .sum();
    Ok(vec![
        ("before-empty", "before", "[[]]".into(), 0),
        ("before-3-4", "before", "[[3,4]]".into(), 25),
        (
            "before-varying",
            "before",
            serde_json::to_string(&vec![old_values])?,
            old_expected,
        ),
        ("after-enabled", "after", enabled.into(), 25),
        ("after-disabled", "after", disabled.into(), 9),
        (
            "after-all-disabled",
            "after",
            "[[{\"value\":3,\"enabled\":false},{\"value\":4,\"enabled\":false}]]".into(),
            0,
        ),
        ("after-empty", "after", "[[]]".into(), 0),
        (
            "after-4096",
            "after",
            serde_json::to_string(&vec![readings])?,
            expected,
        ),
        ("closed-after", "after", enabled.into(), 25),
        ("recovery-before", "before", "[[3,4]]".into(), 25),
    ])
}

fn run_case(
    evolution: &mut Evolution,
    context: &mut Context,
    name: &str,
    artifact: &str,
    arguments: &str,
    expected: u64,
) -> Result<(), DevError> {
    fs::write(
        context.evidence.join(format!("parameter-type-{name}.json")),
        arguments,
    )?;
    let deployment = context
        .root
        .join(format!("parameter-type-{artifact}.deployment.json"));
    let records = evolution.cli(
        context,
        name,
        None,
        &[
            "run",
            "--deployment",
            &deployment.display().to_string(),
            "--arguments",
            arguments,
        ],
        true,
    )?;
    require(
        field(&records, "execution", "value")? == format!("{expected}.0"),
        &format!("{name} disagrees with independent integer sum of squares"),
    )
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    let mut evolution = Evolution::default();
    evolution.cli(
        context,
        "discovery",
        None,
        &["capabilities", "--section", "change"],
        true,
    )?;
    let mut producer = context.new_package("parameter-type-producer")?;
    context.stage(&producer, standard)?;
    evolution.author(
        context,
        &mut producer,
        "producer",
        &format!("{}{PRODUCER}", binding("add", standard)),
    )?;
    evolution.inspect(context, &producer, "producer-before", &["$step", "$energy"])?;
    evolution.export(context, &mut producer)?;
    let old_producer = producer.clone();
    let mut consumer = context.new_package("parameter-type-consumer")?;
    context.stage(&consumer, &producer)?;
    evolution.author(
        context,
        &mut consumer,
        "consumer",
        &format!(
            "{}{}{CONSUMER}",
            binding("add", &producer),
            selection(&producer)
        ),
    )?;
    evolution.inspect(context, &consumer, "consumer-before", &["$command"])?;
    evolution.export(context, &mut consumer)?;
    evolution.imported_contract(context, &consumer, &old_producer, "old-contract")?;
    build(&mut evolution, context, &consumer, "before")?;
    for (name, artifact, arguments, expected) in cases()?.into_iter().take(3) {
        run_case(
            &mut evolution,
            context,
            name,
            artifact,
            &arguments,
            expected,
        )?;
    }
    let before = evolution.cli(
        context,
        "reference-before",
        Some(&consumer.path),
        &["run", "energy", "--arguments", "[[3,4]]"],
        true,
    )?;
    require(
        field(&before, "execution", "differential")? == "equal"
            && field(&before, "execution", "value")? == "25.0",
        "baseline reference disagreement",
    )?;
    let missing_body = PRODUCER_EDIT
        .split("expression.block as=$new-step")
        .next()
        .ok_or_else(|| DevError::corrupt("parameter edit body marker missing"))?;
    evolution.reject(context, &producer, "missing-body", missing_body)?;
    evolution.reject(
        context,
        &producer,
        "stale-producer-port",
        &PRODUCER_EDIT.replace("set.port-contract port=$port type=@Energy\n", ""),
    )?;
    evolution.author(context, &mut producer, "producer-edit", PRODUCER_EDIT)?;
    evolution.cli(
        context,
        "producer-check",
        Some(&producer.path),
        &["check"],
        true,
    )?;
    evolution.inspect(context, &producer, "producer-after", &["$step", "$energy"])?;
    evolution.export(context, &mut producer)?;
    require(
        producer.logical != old_producer.logical,
        "parameter edit did not change immutable package revision",
    )?;
    context.stage(&consumer, &producer)?;
    evolution.imported_contract(
        context,
        &consumer,
        &old_producer,
        "old-contract-after-producer",
    )?;
    let old_pin = evolution.cli(
        context,
        "old-pin-run",
        Some(&consumer.path),
        &["run", "energy", "--arguments", "[[3,4]]"],
        true,
    )?;
    require(
        field(&old_pin, "execution", "value")? == "25.0",
        "producer edit changed old consumer pin",
    )?;
    let replacement = binding("replace", &producer);
    evolution.reject(context, &consumer, "dependency-only", &replacement)?;
    let repaired = format!("{replacement}{}{CONSUMER_EDIT}", selection(&producer));
    evolution.reject(
        context,
        &consumer,
        "stale-consumer-port",
        &repaired.replace("set.port-contract port=$port type=@Energy\n", ""),
    )?;
    evolution.author(context, &mut consumer, "consumer-edit", &repaired)?;
    evolution.cli(
        context,
        "consumer-check",
        Some(&consumer.path),
        &["check"],
        true,
    )?;
    evolution.inspect(context, &consumer, "consumer-after", &["$command"])?;
    evolution.export(context, &mut consumer)?;
    evolution.imported_contract(context, &consumer, &producer, "new-contract")?;
    build(&mut evolution, context, &consumer, "after")?;
    // This public project route uses the disjoint canonical reference evaluator for pure logic.
    let arguments = "[[{\"value\":3,\"enabled\":true},{\"value\":4,\"enabled\":false}]]";
    let repaired_run = evolution.cli(
        context,
        "reference-after",
        Some(&consumer.path),
        &["run", "energy", "--arguments", arguments],
        true,
    )?;
    require(
        field(&repaired_run, "execution", "differential")? == "equal"
            && field(&repaired_run, "execution", "value")? == "9.0",
        "repaired reference disagreement",
    )?;
    for (name, artifact, arguments, expected) in cases()?.into_iter().skip(3).take(5) {
        run_case(
            &mut evolution,
            context,
            name,
            artifact,
            &arguments,
            expected,
        )?;
    }
    // Only owned disposable authoring repositories disappear. Both frozen bundles remain intact.
    fs::remove_dir_all(&producer.path)?;
    fs::remove_dir_all(&consumer.path)?;
    evolution.repositories_removed = !producer.path.exists() && !consumer.path.exists();
    for (name, artifact, arguments, expected) in cases()?.into_iter().skip(8) {
        run_case(
            &mut evolution,
            context,
            name,
            artifact,
            &arguments,
            expected,
        )?;
    }
    context
        .receipt
        .observations
        .insert("parameter_type".into(), serde_json::to_string(&evolution)?);
    Ok(())
}

fn output(
    receipt: &Receipt,
    root: &Path,
    evolution: &Evolution,
    name: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    let index = evolution
        .commands
        .get(name)
        .ok_or_else(|| DevError::corrupt(format!("missing parameter-type command {name}")))?;
    let command = receipt
        .commands
        .get(*index)
        .ok_or_else(|| DevError::corrupt("parameter-type command index missing"))?;
    parse_records(
        "parameter-type-output",
        &process::read_bounded(
            &root.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|errors| DevError::corrupt(format!("parameter-type output: {errors:?}")))
}

fn fields(record: &CompactRecord) -> BTreeMap<String, String> {
    record
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.value.clone()))
        .collect()
}

fn signature(records: &[CompactRecord]) -> Result<Vec<BTreeMap<String, String>>, DevError> {
    require(
        field(records, "page", "complete")? == "true",
        "incomplete parameter-type definition inspection",
    )?;
    Ok(records
        .iter()
        .filter(|r| {
            matches!(
                r.operation.as_str(),
                "definition.function" | "definition.parameter"
            )
        })
        .map(fields)
        .collect())
}

fn validate_signatures(
    receipt: &Receipt,
    root: &Path,
    evolution: &Evolution,
) -> Result<(), DevError> {
    for (project, functions) in [
        ("producer", &["$step", "$energy"][..]),
        ("consumer", &["$command"][..]),
    ] {
        for function in functions {
            let before = signature(&output(
                receipt,
                root,
                evolution,
                &format!("{project}-before-{function}"),
            )?)?;
            let after = signature(&output(
                receipt,
                root,
                evolution,
                &format!("{project}-after-{function}"),
            )?)?;
            require(
                before.len() == after.len()
                    && before.len() == if *function == "$step" { 3 } else { 2 },
                "parameter membership changed",
            )?;
            for (index, (before, after)) in before.iter().zip(&after).enumerate() {
                let changed_field = if index == 0 { "body" } else { "type" };
                for (key, value) in before {
                    if key != changed_field {
                        require(
                            after.get(key) == Some(value),
                            "parameter identity, parent, name, order, use, requirement or function contract changed",
                        )?;
                    }
                }
                if index != 0 {
                    let changed = before.get("name").is_some_and(|name| name != "acc");
                    require(
                        (before.get("type") != after.get("type")) == changed,
                        "parameter edit changed the wrong type",
                    )?;
                }
            }
        }
        for kind in ["component", "port", "target"] {
            let owners = |records: Vec<CompactRecord>| {
                records
                    .iter()
                    .filter(|r| r.operation == "owner")
                    .map(|r| record_field(r, "id"))
                    .collect::<Result<Vec<_>, _>>()
            };
            let before = owners(output(
                receipt,
                root,
                evolution,
                &format!("{project}-before-{kind}"),
            )?)?;
            let after = owners(output(
                receipt,
                root,
                evolution,
                &format!("{project}-after-{kind}"),
            )?)?;
            require(
                before.len() == 1 && before == after,
                "parameter edit replaced a component, port or target identity",
            )?;
        }
    }
    Ok(())
}

fn validate_authoring(
    receipt: &Receipt,
    root: &Path,
    evolution: &Evolution,
) -> Result<(), DevError> {
    let old_library = &evolution.sources[0];
    let new_library = &evolution.sources[2];
    let inventory = &receipt.inventories[old_library.inventory];
    let [standard_binding] = receipt.producer_inventories[old_library.inventory]
        .dependencies
        .as_slice()
    else {
        return Err(DevError::corrupt(
            "parameter-type producer must have its single ordinary standard dependency",
        ));
    };
    let standard = inventory
        .packages
        .iter()
        .find(|p| p.package == standard_binding.0 && p.package_revision == standard_binding.1)
        .ok_or_else(|| DevError::corrupt("parameter-type ordinary standard dependency missing"))?;
    let dependency = |operation: &str, package: &str, revision: &str, logical: &str| {
        format!(
            "{operation}.dependency package={package} semantic-revision={revision} package-revision={logical}\n"
        )
    };
    let select = |source: &Source| {
        format!(
            "reference.package as=$library package={} package-revision={}\n",
            source.package, source.logical
        )
    };
    let bodies = [
        format!(
            "{}{PRODUCER}",
            dependency(
                "add",
                &standard.package,
                &standard.semantic_revision,
                &standard.package_revision
            )
        ),
        format!(
            "{}{}{CONSUMER}",
            dependency(
                "add",
                &old_library.package,
                &old_library.revision,
                &old_library.logical
            ),
            select(old_library)
        ),
        PRODUCER_EDIT.to_owned(),
        format!(
            "{}{}{CONSUMER_EDIT}",
            dependency(
                "replace",
                &new_library.package,
                &new_library.revision,
                &new_library.logical
            ),
            select(new_library)
        ),
    ];
    let copied = Path::new(&receipt.isolated_root)
        .join("lkjscript")
        .display()
        .to_string();
    for (index, (name, body)) in ["producer", "consumer", "producer-edit", "consumer-edit"]
        .into_iter()
        .zip(bodies)
        .enumerate()
    {
        let planned = output(receipt, root, evolution, &format!("{name}-plan"))?;
        let applied = output(receipt, root, evolution, &format!("{name}-apply"))?;
        let base = field(&planned, "revision", "base")?;
        let token = field(&planned, "plan", "token")?;
        require(
            field(&applied, "plan", "token")? == token
                && field(&applied, "revision", "base")? == base
                && field(&planned, "revision", "result")? == evolution.sources[index].revision
                && field(&applied, "revision", "result")? == evolution.sources[index].revision,
            "parameter-type authoring lost its reviewed accepted revision",
        )?;
        if index >= 2 {
            require(
                base == evolution.sources[index - 2].revision,
                "parameter edit did not start from selected predecessor",
            )?;
        }
        let request = format!("parameter-type-{name}.lkjc");
        let review = format!("parameter-type-{name}.lkjplan");
        require(
            process::read_bounded(&root.join(&request), MAXIMUM_OUTPUT_BYTES)?
                == format!("request base={base} idempotency=parameter-type-{name}\n{body}")
                    .as_bytes(),
            "parameter-type retained input differs from the literal ordinary program",
        )?;
        let plan = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(
            root.join(&review),
        )?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            plan.token == token,
            "parameter-type retained review token differs",
        )?;
        let project = Path::new(&receipt.isolated_root)
            .join(if index % 2 == 0 {
                "parameter-type-producer"
            } else {
                "parameter-type-consumer"
            })
            .display()
            .to_string();
        for (action, tail) in [
            (
                "plan",
                vec![
                    "--output".to_owned(),
                    Path::new(&receipt.isolated_root)
                        .join(&review)
                        .display()
                        .to_string(),
                ],
            ),
            ("apply", vec!["--plan".to_owned(), token.clone()]),
        ] {
            let mut expected = vec![
                copied.clone(),
                "--project".into(),
                project.clone(),
                "change".into(),
                action.into(),
                "--input-file".into(),
                Path::new(&receipt.isolated_root)
                    .join(&request)
                    .display()
                    .to_string(),
            ];
            expected.extend(tail);
            let command = &receipt.commands[evolution.commands[&format!("{name}-{action}")]];
            require(
                command.command == expected && command.expects_success,
                "parameter-type authoring did not use exact copied-product input/review",
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate(
    receipt: &Receipt,
    root: &Path,
    inventory_span: std::ops::Range<usize>,
) -> Result<Vec<usize>, DevError> {
    let evolution: Evolution = serde_json::from_str(
        receipt
            .observations
            .get("parameter_type")
            .ok_or_else(|| DevError::corrupt("parameter-type witness absent"))?,
    )?;
    require(
        evolution.sources.len() == 4 && evolution.repositories_removed,
        "parameter-type closure/source evidence incomplete",
    )?;
    require(
        inventory_span.len() == 4
            && inventory_span_matches(
                evolution.sources.iter().map(|source| source.inventory),
                inventory_span,
                receipt.inventories.len(),
            ),
        "parameter-type exact source inventories omitted, reordered or overlapping",
    )?;
    for source in &evolution.sources {
        let producer = receipt
            .producer_inventories
            .get(source.inventory)
            .ok_or_else(|| DevError::corrupt("parameter-type source inventory absent"))?;
        let inventory = receipt
            .inventories
            .get(source.inventory)
            .ok_or_else(|| DevError::corrupt("parameter-type transport inventory absent"))?;
        let transport = receipt
            .transport_digests
            .get(source.inventory)
            .ok_or_else(|| DevError::corrupt("parameter-type transport identity absent"))?;
        verify_producer_inventory(producer, inventory)?;
        require(
            producer.package == source.package
                && producer.semantic_revision == source.revision
                && inventory.packages.iter().any(|p| {
                    p.package == source.package
                        && p.semantic_revision == source.revision
                        && p.package_revision == source.logical
                }),
            "parameter-type exact source binding differs",
        )?;
        let bytes = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", source.inventory + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        require(
            offline_package_inventory(&bytes, transport)
                .map_err(|e| DevError::corrupt(e.to_string()))?
                == *inventory,
            "parameter-type retained source bytes differ",
        )?;
    }
    let [old_library, old_consumer, new_library, new_consumer] = evolution.sources.as_slice()
    else {
        return Err(DevError::corrupt("parameter-type source sequence missing"));
    };
    require(
        old_library.package == new_library.package
            && old_library.logical != new_library.logical
            && old_consumer.package == new_consumer.package
            && old_consumer.logical != new_consumer.logical,
        "parameter-type package identities/revisions did not evolve",
    )?;
    for (consumer, library) in [(old_consumer, old_library), (new_consumer, new_library)] {
        require(
            receipt.producer_inventories[consumer.inventory]
                .dependencies
                .iter()
                .any(|(package, revision)| {
                    package == &library.package && revision == &library.logical
                }),
            "consumer did not select the expected exact library revision",
        )?;
    }
    validate_authoring(receipt, root, &evolution)?;
    validate_signatures(receipt, root, &evolution)?;
    let discovery = output(receipt, root, &evolution, "discovery")?;
    require(
        discovery.iter().any(|r| {
            r.operation == "change.operation"
                && record_field(r, "name").is_ok_and(|name| name == "set.parameter-type")
        }),
        "parameter-type operation absent from copied-product discovery",
    )?;
    let old = output(receipt, root, &evolution, "old-contract")?;
    let still_old = output(receipt, root, &evolution, "old-contract-after-producer")?;
    let new = output(receipt, root, &evolution, "new-contract")?;
    require(
        old == still_old,
        "staged new library changed old public pinned contract",
    )?;
    for (records, expected) in [(&old, "f64"), (&new, "named")] {
        let find_type = |path: &str| {
            records
                .iter()
                .find(|r| r.operation == "type" && record_field(r, "path").is_ok_and(|p| p == path))
        };
        require(
            find_type("parameter.items")
                .is_some_and(|r| record_field(r, "form").is_ok_and(|f| f == "list"))
                && find_type("parameter.items.item")
                    .is_some_and(|r| record_field(r, "form").is_ok_and(|f| f == expected)),
            "public library inspection lost the selected sample/reading contract",
        )?;
    }
    for (phase, records) in [("before", &old), ("after", &new)] {
        for (project, function, parameter, path) in [
            ("producer", "$step", "sample", "parameter.items.item"),
            ("producer", "$energy", "items", "parameter.items"),
            ("consumer", "$command", "items", "parameter.items"),
        ] {
            let ty = records
                .iter()
                .find(|r| r.operation == "type" && record_field(r, "path").is_ok_and(|p| p == path))
                .ok_or_else(|| DevError::corrupt("public parameter type missing"))?;
            let definition = output(
                receipt,
                root,
                &evolution,
                &format!("{project}-{phase}-{function}"),
            )?;
            let parameter = definition
                .iter()
                .find(|r| {
                    r.operation == "definition.parameter"
                        && record_field(r, "name").is_ok_and(|name| name == parameter)
                })
                .ok_or_else(|| DevError::corrupt("selected public definition parameter missing"))?;
            require(
                record_field(parameter, "type")? == record_field(ty, "digest")?,
                "local parameter contract differs from exact transported public type",
            )?;
        }
    }
    for name in [
        "missing-body",
        "stale-producer-port",
        "dependency-only",
        "stale-consumer-port",
    ] {
        require(
            output(receipt, root, &evolution, &format!("{name}-before"))?
                == output(receipt, root, &evolution, &format!("{name}-after"))?
                && output(
                    receipt,
                    root,
                    &evolution,
                    &format!("{name}-definition-before"),
                )? == output(
                    receipt,
                    root,
                    &evolution,
                    &format!("{name}-definition-after"),
                )?,
            "rejected incomplete repair changed public accepted HEAD",
        )?;
        let rejected = output(receipt, root, &evolution, name)?;
        require(
            rejected
                .iter()
                .filter(|r| r.operation == "diagnostic")
                .any(|r| {
                    record_field(r, "code").is_ok_and(|code| {
                        code.starts_with("kernel_type_") || code.starts_with("kernel_port_")
                    })
                }),
            "parameter-type rejection lost type/port admission diagnostic",
        )?;
    }
    let accepted = output(receipt, root, &evolution, "producer-edit-apply")?;
    let retried = output(receipt, root, &evolution, "producer-retry")?;
    for (record, key) in [
        ("revision", "result"),
        ("receipt", "digest"),
        ("receipt", "revision-record"),
    ] {
        require(
            field(&accepted, record, key)? == field(&retried, record, key)?,
            "parameter-type retry allocated a new accepted result",
        )?;
    }
    for (name, expected) in [
        ("reference-before", "25.0"),
        ("old-pin-run", "25.0"),
        ("reference-after", "9.0"),
    ] {
        let run = output(receipt, root, &evolution, name)?;
        require(
            field(&run, "execution", "value")? == expected
                && field(&run, "execution", "differential")? == "equal",
            "parameter-type production/reference equality absent",
        )?;
    }
    for name in ["producer-check", "consumer-check"] {
        let checked = output(receipt, root, &evolution, name)?;
        require(
            field(&checked, "tests", "failed")? == "0"
                && field(&checked, "tests", "differential")? == "equal",
            "repaired project check did not pass both evaluators",
        )?;
    }
    let mut bundles = BTreeMap::new();
    for (name, source) in [("before", old_consumer), ("after", new_consumer)] {
        let artifact = process::read_bounded(
            &root.join(format!("parameter-type-{name}.lkja")),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let transport = &receipt.transport_digests[source.inventory];
        let container = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", source.inventory + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
            &artifact, &container, transport,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        let bundle = bound["bundle"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("parameter-type artifact bundle missing"))?
            .to_owned();
        let built = output(receipt, root, &evolution, &format!("{name}-build"))?;
        require(
            bound["package"] == source.package
                && bound["revision"] == source.revision
                && bound["package_revision"] == source.logical
                && field(&built, "artifact", "bundle")? == bundle
                && field(&built, "authority", "revision")? == source.revision,
            "parameter-type artifact does not bind its retained accepted source",
        )?;
        bundles.insert(name, bundle);
    }
    let mut copied = Vec::new();
    for (name, artifact, arguments, expected) in cases()? {
        let index = evolution
            .commands
            .get(name)
            .ok_or_else(|| DevError::corrupt("parameter-type call missing"))?;
        let command = &receipt.commands[*index];
        require(
            command.command
                == vec![
                    Path::new(&receipt.isolated_root)
                        .join("lkjscript")
                        .display()
                        .to_string(),
                    "run".into(),
                    "--deployment".into(),
                    Path::new(&receipt.isolated_root)
                        .join(format!("parameter-type-{artifact}.deployment.json"))
                        .display()
                        .to_string(),
                    "--arguments".into(),
                    arguments.clone(),
                ]
                && command.expects_success,
            "parameter-type invocation did not use the exact standalone bundle/arguments",
        )?;
        require(
            process::read_bounded(
                &root.join(format!("parameter-type-{name}.json")),
                MAXIMUM_OUTPUT_BYTES,
            )? == arguments.as_bytes(),
            "parameter-type literal arguments changed",
        )?;
        let run = output(receipt, root, &evolution, name)?;
        require(
            field(&run, "execution", "value")? == format!("{expected}.0")
                && bundles.get(artifact) == Some(&field(&run, "execution", "artifact")?),
            "parameter-type independent integer result differs",
        )?;
        let cleanup: Value = serde_json::from_str(&field(&run, "execution", "cleanup")?)?;
        require(
            cleanup["remaining_tasks"] == 0 && cleanup["cleanup_failures"] == json!([]),
            "parameter-type foreground resources did not join",
        )?;
        copied.push(*index);
    }
    for name in ["before", "after"] {
        require(
            process::read_bounded(
                &root.join(format!("parameter-type-{name}.deployment.json")),
                MAXIMUM_OUTPUT_BYTES,
            )? == evidence::encode_json(&descriptor(name))?,
            "parameter-type recovery descriptor differs",
        )?;
        require(
            fs::metadata(root.join(format!("parameter-type-{name}.lkja")))?.len() > 0,
            "parameter-type retained artifact missing",
        )?;
    }
    Ok(copied)
}
