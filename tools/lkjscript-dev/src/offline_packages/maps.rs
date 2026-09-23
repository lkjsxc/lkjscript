//! Ordinary transported aggregation, retained map versions and bounded project execution.
//! The host supplies literal requests and independent expected contents, never graph mutations.
use super::*;
use serde_json::{Value, json};

const PRODUCER: &str = include_str!("maps.producer.lkjc");
const PRODUCER_EDIT: &str = include_str!("maps.producer_edit.lkjc");
const CONSUMER: &str = include_str!("maps.consumer.lkjc");
const COUNT: &str = include_str!("maps.count.lkjc");
const SMALL: &str =
    "[[{\"key\":\"b\",\"amount\":3},{\"key\":\"a\",\"amount\":2},{\"key\":\"b\",\"amount\":4}]]";

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    commands: BTreeMap<String, usize>,
    sources: Vec<Source>,
    producer_removed_before_build: bool,
    consumer_removed_before_bundle_runs: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    package: String,
    revision: String,
    logical: String,
    inventory: usize,
}

struct Case {
    name: &'static str,
    phase: &'static str,
    target: &'static str,
    arguments: String,
    expected: Result<Value, &'static str>,
    artifact: bool,
}

fn selection(package: &Package) -> String {
    format!(
        "reference.package as=$library package={} package-revision={}\n",
        package.id, package.logical
    )
}

impl Witness {
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
            "duplicate persistent-map command label",
        )?;
        if project.is_none() && arguments.starts_with(&["run", "--deployment"]) && passes {
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
        let request = context.root.join(format!("maps-{name}.lkjc"));
        let review = context.root.join(format!("maps-{name}.lkjplan"));
        fs::write(
            &request,
            format!(
                "request base={} idempotency=maps-{name}\n{body}",
                package.revision
            ),
        )?;
        fs::copy(&request, context.evidence.join(format!("maps-{name}.lkjc")))?;
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
            "persistent-map retained review differs",
        )?;
        fs::copy(
            &review,
            context.evidence.join(format!("maps-{name}.lkjplan")),
        )?;
        let applied = self.cli(
            context,
            &format!("{name}-apply"),
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
}

pub(super) fn focused(context: &mut Context) -> Result<(), DevError> {
    let builtin = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id")?,
        revision: field(&builtin, "package", "revision")?,
        logical: field(&builtin, "package", "package-revision")?,
        transport: field(&builtin, "package", "transport")?,
        container: context.root.join("maps-standard.lkjp"),
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

fn descriptor(phase: &str, target: &str) -> Value {
    json!({"artifact":format!("maps-{phase}.lkja"),"target":target,
        "listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),
        "configuration":{},"secrets":[],"grants":[]})
}

fn cases() -> Result<Vec<Case>, DevError> {
    let mut cases = Vec::new();
    for (phase, a, b, increment) in [("before", 2, 7, 0), ("after", 3, 9, 1)] {
        let prefix = if phase == "before" { "before" } else { "after" };
        // Arithmetic is independently specified here; the program uses the transported fold.
        let events: Vec<_> = (0_i64..512)
            .map(|i| json!({"key":format!("group-{:02}", (i * 17) % 53),"amount":i % 11 - 5}))
            .collect();
        let mut expected = BTreeMap::<String, i64>::new();
        for i in 0_i64..512 {
            *expected
                .entry(format!("group-{:02}", (i * 17) % 53))
                .or_default() += i % 11 - 5 + increment;
        }
        // Fixed key bounds are independent of the standard window's index loop.
        let middle_entries = Value::Array(
            expected
                .iter()
                .filter(|(key, _)| key.as_str() >= "group-15" && key.as_str() < "group-25")
                .map(|(key, value)| json!({"key":key,"value":value}))
                .collect(),
        );
        let middle_arguments = serde_json::to_string(&json!([events, 15, 10]))?;
        let large = Value::Array(expected.into_iter().map(|(k, v)| json!([k, v])).collect());
        for (name, target, arguments, expected) in [
            (
                if prefix == "before" {
                    "before-empty"
                } else {
                    "after-empty"
                },
                "aggregate",
                "[[]]".to_owned(),
                json!([]),
            ),
            (
                if prefix == "before" {
                    "before-small"
                } else {
                    "after-small"
                },
                "aggregate",
                SMALL.to_owned(),
                json!([["a", a], ["b", b]]),
            ),
            (
                if prefix == "before" {
                    "before-snapshots"
                } else {
                    "after-snapshots"
                },
                "snapshots",
                SMALL.to_owned(),
                json!([
                    [["a", a], ["b", b]],
                    [["a", 10], ["b", b]],
                    [["a", 10]],
                    [["a", 10], ["b", b]],
                    [["a", 10]],
                    [["b", b]]
                ]),
            ),
            (
                if prefix == "before" {
                    "before-ordered"
                } else {
                    "after-ordered"
                },
                "ordered",
                SMALL.to_owned(),
                json!([{"key":"a","value":a},{"key":"b","value":b}]),
            ),
            (
                if prefix == "before" {
                    "before-operations"
                } else {
                    "after-operations"
                },
                "operations",
                SMALL.to_owned(),
                json!({"length":2,"lookup":b,"present":true,"missing":false,"existing-fallback":a,"missing-fallback":-99}),
            ),
            (
                if prefix == "before" {
                    "before-large"
                } else {
                    "after-large"
                },
                "aggregate",
                serde_json::to_string(&json!([events]))?,
                large,
            ),
        ] {
            cases.push(Case {
                name,
                phase,
                target,
                arguments,
                expected: Ok(expected),
                artifact: false,
            });
        }
        let small_items = json!([
            {"key":"b","amount":3},
            {"key":"a","amount":2},
            {"key":"b","amount":4}
        ]);
        for (name, target, arguments, expected) in [
            (
                if phase == "before" {
                    "before-page-first"
                } else {
                    "after-page-first"
                },
                "page",
                serde_json::to_string(&json!([small_items, 0, 1]))?,
                json!([{"key":"a","value":a}]),
            ),
            (
                if phase == "before" {
                    "before-page-tail"
                } else {
                    "after-page-tail"
                },
                "page",
                serde_json::to_string(&json!([small_items, 1, i64::MAX]))?,
                json!([{"key":"b","value":b}]),
            ),
            (
                if phase == "before" {
                    "before-page-past-end"
                } else {
                    "after-page-past-end"
                },
                "page",
                serde_json::to_string(&json!([small_items, i64::MAX, i64::MAX]))?,
                json!([]),
            ),
            (
                if phase == "before" {
                    "before-page-negative-start"
                } else {
                    "after-page-negative-start"
                },
                "page",
                serde_json::to_string(&json!([small_items, i64::MIN, 1]))?,
                json!([{"key":"a","value":a}]),
            ),
            (
                if phase == "before" {
                    "before-page-negative-count"
                } else {
                    "after-page-negative-count"
                },
                "page",
                serde_json::to_string(&json!([small_items, 0, i64::MIN]))?,
                json!([]),
            ),
            (
                if phase == "before" {
                    "before-page-empty"
                } else {
                    "after-page-empty"
                },
                "page",
                "[[],0,1]".to_owned(),
                json!([]),
            ),
            (
                if phase == "before" {
                    "before-page-middle"
                } else {
                    "after-page-middle"
                },
                "page",
                middle_arguments,
                middle_entries,
            ),
            (
                if phase == "before" {
                    "before-event-window"
                } else {
                    "after-event-window"
                },
                "event-window",
                serde_json::to_string(&json!([small_items, 1, 2]))?,
                json!([{"key":"a","amount":2},{"key":"b","amount":4}]),
            ),
        ] {
            cases.push(Case {
                name,
                phase,
                target,
                arguments,
                expected: Ok(expected),
                artifact: false,
            });
        }
        for (name, target, expected) in [
            (
                if phase == "before" {
                    "bundle-before-page"
                } else {
                    "bundle-after-page"
                },
                "page",
                json!([{"key":"b","value":b}]),
            ),
            (
                if phase == "before" {
                    "bundle-before-event-window"
                } else {
                    "bundle-after-event-window"
                },
                "event-window",
                json!([{"key":"a","amount":2},{"key":"b","amount":4}]),
            ),
        ] {
            cases.push(Case {
                name,
                phase,
                target,
                arguments: serde_json::to_string(&json!([small_items, 1, 2]))?,
                expected: Ok(expected),
                artifact: true,
            });
        }
        for (name, target, arguments, expected) in [
            (
                if prefix == "before" {
                    "before-missing"
                } else {
                    "after-missing"
                },
                "missing-lookup",
                SMALL.to_owned(),
                "normalized_map_key_absent",
            ),
            (
                if prefix == "before" {
                    "before-overflow"
                } else {
                    "after-overflow"
                },
                "aggregate",
                "[[{\"key\":\"a\",\"amount\":9223372036854775807},{\"key\":\"a\",\"amount\":1}]]"
                    .to_owned(),
                "normalized_integer_overflow",
            ),
        ] {
            cases.push(Case {
                name,
                phase,
                target,
                arguments,
                expected: Err(expected),
                artifact: false,
            });
        }
        for (name, target, expected) in [
            (
                if prefix == "before" {
                    "bundle-before-small"
                } else {
                    "bundle-after-small"
                },
                "aggregate",
                json!([["a", a], ["b", b]]),
            ),
            (
                if prefix == "before" {
                    "bundle-before-snapshots"
                } else {
                    "bundle-after-snapshots"
                },
                "snapshots",
                json!([
                    [["a", a], ["b", b]],
                    [["a", 10], ["b", b]],
                    [["a", 10]],
                    [["a", 10], ["b", b]],
                    [["a", 10]],
                    [["b", b]]
                ]),
            ),
        ] {
            cases.push(Case {
                name,
                phase,
                target,
                arguments: SMALL.to_owned(),
                expected: Ok(expected),
                artifact: true,
            });
        }
    }
    cases.push(Case {
        name: "count-4096",
        phase: "before",
        target: "count-keys",
        arguments: "[4096]".to_owned(),
        expected: Ok(json!(4096)),
        artifact: false,
    });
    cases.push(Case {
        name: "bundle-count-4096",
        phase: "before",
        target: "count-keys",
        arguments: "[4096]".to_owned(),
        expected: Ok(json!(4096)),
        artifact: true,
    });
    cases.push(Case {
        name: "old-pin-after-stage",
        phase: "staged",
        target: "aggregate",
        arguments: SMALL.to_owned(),
        expected: Ok(json!([["a", 2], ["b", 7]])),
        artifact: false,
    });
    Ok(cases)
}

fn run_case(
    witness: &mut Witness,
    context: &mut Context,
    consumer: &Package,
    case: &Case,
) -> Result<(), DevError> {
    fs::write(
        context
            .evidence
            .join(format!("maps-input-{}.json", case.name)),
        &case.arguments,
    )?;
    let output = if case.artifact {
        let name = format!("maps-{}-{}.deployment.json", case.phase, case.target);
        let path = context.root.join(&name);
        let bytes = evidence::encode_json(&descriptor(case.phase, case.target))?;
        fs::write(&path, &bytes)?;
        fs::write(context.evidence.join(name), bytes)?;
        witness.cli(
            context,
            case.name,
            None,
            &[
                "run",
                "--deployment",
                &path.display().to_string(),
                "--arguments",
                &case.arguments,
            ],
            true,
        )?
    } else {
        witness.cli(
            context,
            case.name,
            Some(&consumer.path),
            &["run", case.target, "--arguments", &case.arguments],
            case.expected.is_ok(),
        )?
    };
    validate_result(&output, case)
}

fn validate_result(output: &[CompactRecord], case: &Case) -> Result<(), DevError> {
    match &case.expected {
        Ok(expected) => {
            require(
                field(output, "execution", "value")? == serde_json::to_string(expected)?,
                &format!(
                    "{} differs from independently specified ordered map contents",
                    case.name
                ),
            )?;
            if case.artifact {
                require(
                    field(output, "execution", "execution-profile")? == "trusted-foreground",
                    "map bundle lost trusted foreground execution",
                )?;
                for limit in [
                    "instruction-limit",
                    "allocation-limit",
                    "collection-limit",
                    "capability-call-limit",
                    "deadline-milliseconds",
                ] {
                    require(
                        field(output, "execution", limit)? == "absent",
                        "map bundle invented a trusted execution quota or deadline",
                    )?;
                }
                let cleanup: Value = serde_json::from_str(&field(output, "execution", "cleanup")?)?;
                require(
                    cleanup["remaining_tasks"] == 0 && cleanup["cleanup_failures"] == json!([]),
                    "map bundle resources did not join",
                )?;
            } else {
                require(
                    field(output, "execution", "differential")? == "equal",
                    "map project evaluators disagree",
                )?;
                if case.name == "count-4096" {
                    for field_name in ["production-collection-items", "reference-collection-items"]
                    {
                        let items: u64 =
                            field(output, "execution", field_name)?
                                .parse()
                                .map_err(|_| {
                                    DevError::corrupt("map project collection observation absent")
                                })?;
                        require(
                            items < 1_000_000,
                            "4,096 map updates exceed the unchanged project collection budget",
                        )?;
                    }
                    // A growing map creates one entry per distinct key. The AVL height is
                    // below twice ceil(log2(n + 1)); allow three rotation nodes per edit.
                    // These observations supplement the carrier's independent pointer oracle.
                    for tier in ["production", "reference"] {
                        let count = |name: &str| -> Result<u64, DevError> {
                            field(output, "execution", &format!("{tier}-map-{name}"))?
                                .parse()
                                .map_err(|_| DevError::corrupt("map storage observation absent"))
                        };
                        require(
                            count("entry-handles-allocated")? == 4096
                                && (4096..=4096 * (2 * 13 + 3))
                                    .contains(&count("nodes-allocated")?)
                                && count("key-bytes-copied")? == 0,
                            "4,096-key public workload did not retain path-bounded map storage",
                        )?;
                    }
                }
            }
        }
        Err(code) => require(
            field(output, "diagnostic", "code")? == *code,
            "map failure changed its existing trap contract",
        )?,
    }
    Ok(())
}

fn build(
    witness: &mut Witness,
    context: &mut Context,
    package: &Package,
    phase: &str,
) -> Result<(), DevError> {
    let name = format!("maps-{phase}.lkja");
    let output = context.root.join(&name);
    witness.cli(
        context,
        &format!("{phase}-build"),
        Some(&package.path),
        &["build", "--output", &output.display().to_string()],
        true,
    )?;
    fs::copy(output, context.evidence.join(name))?;
    Ok(())
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    let mut witness = Witness::default();
    witness.cli(
        context,
        "discovery",
        None,
        &["capabilities", "--section", "change"],
        true,
    )?;
    let mut producer = context.new_package("maps-producer")?;
    context.stage(&producer, standard)?;
    witness.author(
        context,
        &mut producer,
        "producer",
        &format!("{}{PRODUCER}", binding("add", standard)),
    )?;
    witness.export(context, &mut producer)?;
    let old_producer = producer.clone();
    witness.author(context, &mut producer, "producer-edit", PRODUCER_EDIT)?;
    witness.export(context, &mut producer)?;
    require(
        old_producer.logical != producer.logical,
        "map library edit did not change its exact revision",
    )?;
    fs::remove_dir_all(&producer.path)?;
    witness.producer_removed_before_build = !producer.path.exists();
    witness.cli(
        context,
        "producer-absent",
        Some(&producer.path),
        &["status"],
        false,
    )?;
    let mut consumer = context.new_package("maps-consumer")?;
    context.stage(&consumer, standard)?;
    context.stage(&consumer, &old_producer)?;
    witness.author(
        context,
        &mut consumer,
        "consumer",
        &format!(
            "{}{}{}{CONSUMER}",
            binding("add", standard),
            binding("add", &old_producer),
            selection(&old_producer)
        ),
    )?;
    witness.author(context, &mut consumer, "count", COUNT)?;
    witness.export(context, &mut consumer)?;
    witness.cli(
        context,
        "before-check",
        Some(&consumer.path),
        &["check"],
        true,
    )?;
    build(&mut witness, context, &consumer, "before")?;
    let cases = cases()?;
    for case in cases
        .iter()
        .filter(|case| case.phase == "before" && !case.artifact)
    {
        run_case(&mut witness, context, &consumer, case)?;
    }
    context.stage(&consumer, &producer)?;
    for case in cases.iter().filter(|case| case.phase == "staged") {
        run_case(&mut witness, context, &consumer, case)?;
    }
    witness.author(
        context,
        &mut consumer,
        "consumer-replace",
        &binding("replace", &producer),
    )?;
    witness.export(context, &mut consumer)?;
    witness.cli(
        context,
        "after-check",
        Some(&consumer.path),
        &["check"],
        true,
    )?;
    build(&mut witness, context, &consumer, "after")?;
    for case in cases
        .iter()
        .filter(|case| case.phase == "after" && !case.artifact)
    {
        run_case(&mut witness, context, &consumer, case)?;
    }
    fs::remove_dir_all(&consumer.path)?;
    witness.consumer_removed_before_bundle_runs = !consumer.path.exists();
    witness.cli(
        context,
        "consumer-absent",
        Some(&consumer.path),
        &["status"],
        false,
    )?;
    for case in cases.iter().filter(|case| case.artifact) {
        run_case(&mut witness, context, &consumer, case)?;
    }
    context.receipt.observations.insert(
        "persistent_maps".to_owned(),
        serde_json::to_string(&witness)?,
    );
    Ok(())
}

fn output(
    receipt: &Receipt,
    root: &Path,
    witness: &Witness,
    label: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    let index = witness
        .commands
        .get(label)
        .ok_or_else(|| DevError::corrupt(format!("map witness omitted command {label}")))?;
    let command = receipt
        .commands
        .get(*index)
        .ok_or_else(|| DevError::corrupt("map command index absent"))?;
    parse_records(
        "map-output",
        &process::read_bounded(
            &root.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|errors| DevError::corrupt(format!("map output: {errors:?}")))
}

fn verify_command(
    receipt: &Receipt,
    witness: &Witness,
    label: &str,
    expected: Vec<String>,
    passes: bool,
) -> Result<(), DevError> {
    let command = witness
        .commands
        .get(label)
        .and_then(|index| receipt.commands.get(*index))
        .ok_or_else(|| DevError::corrupt("map command missing"))?;
    require(
        command.command == expected && command.expects_success == passes,
        "map command did not use exact copied-product inputs",
    )
}

pub(super) fn validate_project_source(
    records: &[CompactRecord],
    package: &str,
    revision: &str,
    bundle: &str,
) -> Result<(), DevError> {
    require(
        field(records, "authority", "package")? == package
            && field(records, "authority", "revision")? == revision
            && field(records, "artifact", "bundle")? == bundle,
        "map project output does not bind its exact accepted source and bundle",
    )
}

fn validate_authoring(receipt: &Receipt, root: &Path, witness: &Witness) -> Result<(), DevError> {
    let [old_library, new_library, old_consumer, new_consumer] = witness.sources.as_slice() else {
        return Err(DevError::corrupt("map authoring source sequence missing"));
    };
    let [standard_binding] = receipt.producer_inventories[old_library.inventory]
        .dependencies
        .as_slice()
    else {
        return Err(DevError::corrupt(
            "map library must have one ordinary standard dependency",
        ));
    };
    let standard = receipt.inventories[old_library.inventory]
        .packages
        .iter()
        .find(|p| p.package == standard_binding.0 && p.package_revision == standard_binding.1)
        .ok_or_else(|| DevError::corrupt("map standard source absent"))?;
    let dependency = |action: &str, package: &str, revision: &str, logical: &str| {
        format!(
            "{action}.dependency package={package} semantic-revision={revision} package-revision={logical}\n"
        )
    };
    let std = dependency(
        "add",
        &standard.package,
        &standard.semantic_revision,
        &standard.package_revision,
    );
    let bodies = [
        (
            "producer",
            "maps-producer",
            format!("{std}{PRODUCER}"),
            Some(&old_library.revision),
        ),
        (
            "producer-edit",
            "maps-producer",
            PRODUCER_EDIT.to_owned(),
            Some(&new_library.revision),
        ),
        (
            "consumer",
            "maps-consumer",
            format!(
                "{}{}reference.package as=$library package={} package-revision={}\n{CONSUMER}",
                std,
                dependency(
                    "add",
                    &old_library.package,
                    &old_library.revision,
                    &old_library.logical
                ),
                old_library.package,
                old_library.logical
            ),
            None,
        ),
        (
            "count",
            "maps-consumer",
            COUNT.to_owned(),
            Some(&old_consumer.revision),
        ),
        (
            "consumer-replace",
            "maps-consumer",
            dependency(
                "replace",
                &new_library.package,
                &new_library.revision,
                &new_library.logical,
            ),
            Some(&new_consumer.revision),
        ),
    ];
    let isolated = Path::new(&receipt.isolated_root);
    for (name, project, body, result) in bodies {
        let planned = output(receipt, root, witness, &format!("{name}-plan"))?;
        let applied = output(receipt, root, witness, &format!("{name}-apply"))?;
        let base = field(&planned, "revision", "base")?;
        let token = field(&planned, "plan", "token")?;
        let actual_result = field(&applied, "revision", "result")?;
        require(
            field(&applied, "revision", "base")? == base
                && field(&applied, "plan", "token")? == token
                && field(&planned, "revision", "result")? == actual_result
                && result.is_none_or(|expected| *expected == actual_result),
            "map accepted source lost its reviewed revision",
        )?;
        let previous = match name {
            "producer-edit" => Some(old_library.revision.clone()),
            "count" => Some(field(
                &output(receipt, root, witness, "consumer-apply")?,
                "revision",
                "result",
            )?),
            "consumer-replace" => Some(old_consumer.revision.clone()),
            _ => None,
        };
        require(
            previous.is_none_or(|expected| expected == base),
            "map edit did not start at its accepted predecessor",
        )?;
        let request = format!("maps-{name}.lkjc");
        let review = format!("maps-{name}.lkjplan");
        require(
            process::read_bounded(&root.join(&request), MAXIMUM_OUTPUT_BYTES)?
                == format!("request base={base} idempotency=maps-{name}\n{body}").as_bytes(),
            "map literal authoring request changed",
        )?;
        let plan = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(
            root.join(&review),
        )?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(plan.token == token, "map review token changed")?;
        for (action, tail) in [
            (
                "plan",
                vec![
                    "--output".to_owned(),
                    isolated.join(&review).display().to_string(),
                ],
            ),
            ("apply", vec!["--plan".to_owned(), token.clone()]),
        ] {
            let mut command = vec![
                isolated.join("lkjscript").display().to_string(),
                "--project".to_owned(),
                isolated.join(project).display().to_string(),
                "change".to_owned(),
                action.to_owned(),
                "--input-file".to_owned(),
                isolated.join(&request).display().to_string(),
            ];
            command.extend(tail);
            verify_command(receipt, witness, &format!("{name}-{action}"), command, true)?;
        }
    }
    Ok(())
}

pub(super) fn validate(
    receipt: &Receipt,
    root: &Path,
    inventory_span: std::ops::Range<usize>,
) -> Result<Vec<usize>, DevError> {
    let witness: Witness = serde_json::from_str(
        receipt
            .observations
            .get("persistent_maps")
            .ok_or_else(|| DevError::corrupt("persistent map witness absent"))?,
    )?;
    require(
        witness.sources.len() == 4
            && witness.producer_removed_before_build
            && witness.consumer_removed_before_bundle_runs,
        "map source removal or closure proof missing",
    )?;
    require(
        inventory_span.len() == 4
            && inventory_span.end == receipt.inventories.len()
            && inventory_span_matches(
                witness.sources.iter().map(|source| source.inventory),
                inventory_span,
                receipt.inventories.len(),
            ),
        "map exact source inventories omitted, reordered or overlapping",
    )?;
    for source in &witness.sources {
        let producer = receipt
            .producer_inventories
            .get(source.inventory)
            .ok_or_else(|| DevError::corrupt("map producer inventory absent"))?;
        let inventory = &receipt.inventories[source.inventory];
        let transport = receipt
            .transport_digests
            .get(source.inventory)
            .ok_or_else(|| DevError::corrupt("map transport identity absent"))?;
        verify_producer_inventory(producer, inventory)?;
        require(
            producer.package == source.package
                && producer.semantic_revision == source.revision
                && inventory.packages.iter().any(|p| {
                    p.package == source.package
                        && p.semantic_revision == source.revision
                        && p.package_revision == source.logical
                }),
            "map exact producer binding differs",
        )?;
        let bytes = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", source.inventory + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        require(
            offline_package_inventory(&bytes, transport)
                .map_err(|error| DevError::corrupt(error.to_string()))?
                == *inventory,
            "map retained transport source changed",
        )?;
    }
    let [old_library, new_library, old_consumer, new_consumer] = witness.sources.as_slice() else {
        return Err(DevError::corrupt("map source sequence incomplete"));
    };
    require(
        old_library.package == new_library.package
            && old_library.logical != new_library.logical
            && old_consumer.package == new_consumer.package
            && old_consumer.logical != new_consumer.logical,
        "map package identities or exact evolution differs",
    )?;
    for (consumer, library) in [(old_consumer, old_library), (new_consumer, new_library)] {
        require(
            receipt.producer_inventories[consumer.inventory]
                .dependencies
                .iter()
                .any(|(package, revision)| {
                    package == &library.package && revision == &library.logical
                }),
            "map consumer omitted its exact library dependency",
        )?;
    }
    validate_authoring(receipt, root, &witness)?;
    let isolated = Path::new(&receipt.isolated_root);
    for (label, directory) in [
        ("producer-absent", "maps-producer"),
        ("consumer-absent", "maps-consumer"),
    ] {
        verify_command(
            receipt,
            &witness,
            label,
            vec![
                isolated.join("lkjscript").display().to_string(),
                "--project".into(),
                isolated.join(directory).display().to_string(),
                "status".into(),
            ],
            false,
        )?;
        require(
            field(
                &output(receipt, root, &witness, label)?,
                "diagnostic",
                "code",
            )? == "project_path",
            "map source was still available to the copied executable",
        )?;
    }
    require(
        witness
            .commands
            .get("producer-absent")
            .zip(witness.commands.get("before-build"))
            .is_some_and(|(absent, build)| absent < build),
        "map consumer was built before producer removal",
    )?;
    let mut bundles = BTreeMap::new();
    for (phase, source) in [("before", old_consumer), ("after", new_consumer)] {
        let artifact = process::read_bounded(
            &root.join(format!("maps-{phase}.lkja")),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let container = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", source.inventory + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
            &artifact,
            &container,
            &receipt.transport_digests[source.inventory],
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        let built = output(receipt, root, &witness, &format!("{phase}-build"))?;
        let bundle = field(&built, "artifact", "bundle")?;
        require(
            bound["package"] == source.package
                && bound["revision"] == source.revision
                && bound["package_revision"] == source.logical
                && bound["bundle"] == bundle
                && field(&built, "authority", "revision")? == source.revision,
            "map retained bundle does not bind its accepted source",
        )?;
        bundles.insert(phase, bundle.clone());
        verify_command(
            receipt,
            &witness,
            &format!("{phase}-build"),
            vec![
                isolated.join("lkjscript").display().to_string(),
                "--project".into(),
                isolated.join("maps-consumer").display().to_string(),
                "build".into(),
                "--output".into(),
                isolated
                    .join(format!("maps-{phase}.lkja"))
                    .display()
                    .to_string(),
            ],
            true,
        )?;
        let checked = output(receipt, root, &witness, &format!("{phase}-check"))?;
        require(
            field(&checked, "tests", "failed")? == "0"
                && field(&checked, "tests", "differential")? == "equal",
            "map consumer check did not pass both evaluators",
        )?;
        validate_project_source(&checked, &source.package, &source.revision, &bundle)?;
    }
    let mut copied = Vec::new();
    for case in cases()? {
        require(
            process::read_bounded(
                &root.join(format!("maps-input-{}.json", case.name)),
                MAXIMUM_OUTPUT_BYTES,
            )? == case.arguments.as_bytes(),
            "map literal invocation input changed",
        )?;
        let mut command = vec![isolated.join("lkjscript").display().to_string()];
        if case.artifact {
            let name = format!("maps-{}-{}.deployment.json", case.phase, case.target);
            require(
                process::read_bounded(&root.join(&name), MAXIMUM_OUTPUT_BYTES)?
                    == evidence::encode_json(&descriptor(case.phase, case.target))?,
                "map standalone descriptor changed",
            )?;
            command.extend([
                "run".into(),
                "--deployment".into(),
                isolated.join(name).display().to_string(),
            ]);
        } else {
            command.extend([
                "--project".into(),
                isolated.join("maps-consumer").display().to_string(),
                "run".into(),
                case.target.into(),
            ]);
        }
        command.extend(["--arguments".into(), case.arguments.clone()]);
        verify_command(receipt, &witness, case.name, command, case.expected.is_ok())?;
        let records = output(receipt, root, &witness, case.name)?;
        validate_result(&records, &case)?;
        if case.artifact {
            require(
                witness.commands["consumer-absent"] < witness.commands[case.name],
                "map bundle executed before its source became unavailable",
            )?;
            require(
                bundles.get(case.phase) == Some(&field(&records, "execution", "artifact")?),
                "map execution selected a foreign bundle",
            )?;
            copied.push(witness.commands[case.name]);
        } else if case.expected.is_ok() {
            // Staging the new library preserves the old accepted consumer until
            // its reviewed replacement. Equal output alone cannot bind that source.
            let (source, phase) = if case.phase == "after" {
                (new_consumer, "after")
            } else {
                (old_consumer, "before")
            };
            let bundle = bundles
                .get(phase)
                .ok_or_else(|| DevError::corrupt("map project bundle binding absent"))?;
            validate_project_source(&records, &source.package, &source.revision, bundle)?;
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires retained focused persistent-map receipt and its original verifier"]
    fn window_originals_reject_omission_and_rehashed_substitution() {
        let source = PathBuf::from(
            std::env::var_os("LKJSCRIPT_MAP_WINDOW_RECEIPT").expect("focused map receipt"),
        );
        let verifier = PathBuf::from(
            std::env::var_os("LKJSCRIPT_MAP_WINDOW_VERIFIER").expect("original verifier"),
        );
        let mut receipt: Receipt = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        let candidate = PathBuf::from(&receipt.pinned_runtime_path);
        let admit = |path: &Path| super::super::finite::read_focused(path, &candidate, &verifier);
        admit(&source).expect("authentic originals pass");
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
        admit(&path).expect("owned relocated fixture passes");

        let mut witness: Witness =
            serde_json::from_str(&receipt.observations["persistent_maps"]).unwrap();
        let middle = witness.commands["before-page-middle"];
        witness.commands.remove("before-page-middle").unwrap();
        let mut fault = receipt.clone();
        fault.observations.insert(
            "persistent_maps".to_owned(),
            serde_json::to_string(&witness).unwrap(),
        );
        fs::write(&path, evidence::encode_json(&fault).unwrap()).unwrap();
        assert!(
            admit(&path)
                .expect_err("omitted page must reject")
                .to_string()
                .contains("map command missing")
        );

        let mut fault = receipt.clone();
        *fault.commands[middle].command.last_mut().unwrap() = "[[],0,0]".to_owned();
        fs::write(&path, evidence::encode_json(&fault).unwrap()).unwrap();
        assert!(
            admit(&path)
                .expect_err("substituted page invocation must reject")
                .to_string()
                .contains("map command did not use exact copied-product inputs")
        );

        for (name, from, to, expected) in [
            (
                "maps-input-before-page-middle.json",
                ",15,10]",
                ",16,10]",
                "map literal invocation input changed",
            ),
            (
                "maps-consumer.lkjc",
                "name=list-window",
                "name=list-map",
                "map literal authoring request changed",
            ),
        ] {
            let file = owned.path().join(name);
            let original = fs::read_to_string(&file).unwrap();
            let changed = original.replace(from, to);
            assert_ne!(changed, original);
            fs::write(&file, changed).unwrap();
            let mut fault = receipt.clone();
            *fault
                .files
                .iter_mut()
                .find(|file| file.path == name)
                .unwrap() = evidence::proof(&file, name.to_owned()).unwrap();
            fs::write(&path, evidence::encode_json(&fault).unwrap()).unwrap();
            assert!(
                admit(&path)
                    .expect_err("rehashed substitution must reject")
                    .to_string()
                    .contains(expected)
            );
            fs::write(file, original).unwrap();
        }

        let mut fault = receipt.clone();
        fault.schema = "lkjscript-offline-persistent-maps-1".to_owned();
        fs::write(&path, evidence::encode_json(&fault).unwrap()).unwrap();
        admit(&path).expect_err("predecessor schema cannot prove the new workload");
        fs::write(&path, evidence::encode_json(&receipt).unwrap()).unwrap();
        admit(&path).expect("restored fixture passes");
        admit(&source).expect("originals remain untouched");
    }
}
