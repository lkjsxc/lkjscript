//! Finite recursive library and independent consumers authored solely by the copied product.
use super::*;
use crate::pure_tail_program::Request;
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecursiveReceipt {
    pub library_package: String,
    pub library_revision: String,
    pub changed_library_revision: String,
    pub consumer_package: String,
    pub receiver_package: String,
    pub producers_removed: bool,
    pub results: BTreeMap<String, Value>,
    pub scale_leaves: Vec<i64>,
    pub scale_shape: Vec<i64>,
    pub scale_sum: i64,
    pub mapped_scale_sum: i64,
    pub changed_sum: i64,
    pub inspected_recursive_type: bool,
    pub session: crate::service::nominal::Observation,
    pub schemas: BTreeMap<String, String>,
    pub persistence: Vec<Value>,
    pub persisted_bytes_sha256: String,
    pub resources: Value,
    pub transaction_cancellation: Value,
}

fn leaf(value: Value) -> Value {
    json!({"case":"leaf","value":value})
}
fn branch(children: Vec<Value>) -> Value {
    json!({"case":"branch","value":children})
}
pub(super) fn small() -> Value {
    branch(vec![
        leaf(json!(1)),
        branch(vec![leaf(json!(2)), leaf(json!(4))]),
        branch(vec![]),
    ])
}

// The expected shape is generated independently from the graph's index-recursive algorithms.
pub(super) fn map_expected(value: &Value, scale: i64, bias: i64) -> Value {
    if value["case"] == "leaf" {
        leaf(json!(scale * value["value"].as_i64().unwrap_or(0) + bias))
    } else {
        branch(
            value["value"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|child| map_expected(child, scale, bias))
                .collect(),
        )
    }
}

fn balanced() -> Value {
    let mut level: Vec<_> = (1..=4096).map(|value| leaf(json!(value))).collect();
    while level.len() > 1 {
        level = level.chunks(2).map(|pair| branch(pair.to_vec())).collect();
    }
    level.remove(0)
}

fn ordered(value: &Value) -> Result<Vec<i64>, DevError> {
    let mut leaves = Vec::new();
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value["case"].as_str() {
            Some("leaf") => leaves.push(
                value["value"]
                    .as_i64()
                    .ok_or_else(|| DevError::corrupt("expected integer leaf"))?,
            ),
            Some("branch") => pending.extend(
                value["value"]
                    .as_array()
                    .ok_or_else(|| DevError::corrupt("expected ordered branch"))?
                    .iter()
                    .rev(),
            ),
            _ => return Err(DevError::corrupt("expected complete recursive tree shape")),
        }
    }
    Ok(leaves)
}

fn shape(value: &Value) -> Vec<i64> {
    let mut pending = vec![value];
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        if let Some(children) = node["value"].as_array() {
            result.push(children.len() as i64 + 1);
            pending.extend(children.iter().rev());
        } else {
            result.push(0);
        }
    }
    result
}

pub(super) fn validate(receipt: &RecursiveReceipt) -> Result<(), DevError> {
    let cancel = &receipt.transaction_cancellation;
    require(
        cancel["source_bound"] == true
            && cancel["effects_replayed"] == false
            && cancel["cleanup_complete"] == true
            && cancel["failure"]["code"] == "execution_cancelled"
            && cancel["observation"]["capability_calls"] == 3
            && cancel["observation"]["maximum_live_transactions"] == 1
            && cancel["observation"]["live_transactions_after"] == 0,
        "recursive staged cancellation and cleanup evidence omitted",
    )?;
    require(
        receipt.resources["source_bound"] == true
            && receipt.resources["effects_replayed"] == false
            && receipt.resources["cleanup_complete"] == true
            && receipt.resources["variant_instances"]
                .as_u64()
                .is_some_and(|n| n > 1)
            && receipt.resources["canonical_preparation"]["variant_instances"]
                == receipt.resources["variant_instances"]
            && receipt.resources["canonical_preparation"]["record_instances"]
                == receipt.resources["record_instances"]
            && receipt.resources["canonical_preparation"]["type_derivation_steps"]
                .as_u64()
                .is_some_and(|n| n > 0)
            && receipt.resources["instance_edges"]
                .as_u64()
                .is_some_and(|n| n > 0),
        "recursive preparation/resource evidence missing",
    )?;
    let observations = receipt.resources["observations"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("recursive resource cases missing"))?;
    require(
        observations.len() == 12,
        "recursive small/large independent evaluator and cancellation cases omitted",
    )?;
    for count in [32, 4096] {
        for tier in ["production", "canonical-reference"] {
            for target in ["scale-sum", "scale-mapped-sum"] {
                require(
                    observations.iter().any(|v| {
                        v["leaves"] == count
                            && v["tier"] == tier
                            && v["target"] == target
                            && v["observation_sink_disabled_nanoseconds"]
                                .as_u64()
                                .is_some_and(|n| n > 0)
                            && v["expected"]
                                == if target == "scale-sum" {
                                    json!(count * (count + 1) / 2)
                                } else {
                                    json!(3 * count * (count + 1) / 2 + 5 * count)
                                }
                            && v["observation"][if tier == "production" {
                                "instructions"
                            } else {
                                "expressions"
                            }]
                            .as_u64()
                            .is_some_and(|n| n > 0)
                    }),
                    "recursive resource matrix lost execution work",
                )?;
            }
            require(
                observations.iter().any(|v| {
                    v["leaves"] == count
                        && v["tier"] == tier
                        && v["failure"]["code"] == "execution_cancelled"
                }),
                "recursive cancellation classification missing",
            )?;
        }
    }
    require(
        receipt.persistence.len() == 4 && receipt.persisted_bytes_sha256.len() == 64,
        "recursive persistence rounds or independent byte identity missing",
    )?;
    for (index, round) in receipt.persistence.iter().enumerate() {
        require(
            round["cleanup_complete"] == true,
            "recursive service retained processes or tasks",
        )?;
        let requests = round["observations"]
            .as_array()
            .ok_or_else(|| DevError::corrupt("recursive HTTP observations missing"))?;
        require(
            requests.len()
                == if index == 0 {
                    3
                } else if index == 1 {
                    2
                } else {
                    1
                },
            "recursive HTTP observation count changed",
        )?;
        let expected = if index == 3 {
            json!("layout-mismatch")
        } else {
            small()
        };
        require(
            requests.last().is_some_and(|r| {
                r["value"] == expected && r["status"] == if index == 3 { 409 } else { 200 }
            }),
            "recursive restart, body edit or mismatch result omitted",
        )?;
        if index == 0 {
            require(
                requests[1]["status"] == 500 && requests[0]["value"] == small(),
                "recursive staged trap/rollback missing",
            )?;
        }
    }
    for label in [
        "node",
        "uninhabited",
        "expr",
        "group",
        "flip",
        "select-a",
        "select-b",
        "reset-a",
        "reset-b",
        "wrapper-a",
        "wrapper",
        "signature",
    ] {
        require(
            receipt
                .schemas
                .get(label)
                .is_some_and(|value| value.starts_with("decl_")),
            "finite public schema proof omitted",
        )?;
    }
    for (name, expected) in super::recursive_schemas::values() {
        require(
            receipt.results.get(name) == Some(&expected),
            "concrete finite public value observation omitted",
        )?;
    }
    require(
        receipt.results.get("flip-next") == Some(&json!({"case":"next","value":{"case":"stop"}})),
        "permuted concrete one-step construction missing",
    )?;
    for label in [
        "self",
        "mutual",
        "signature-expanding",
        "nominal-argument",
        "phantom",
    ] {
        require(
            receipt.schemas.get(label).map(String::as_str) == Some("kernel_type_nominal_expansion"),
            "expanding public schema witness omitted",
        )?;
    }
    for (label, code) in [
        ("member-edit-rejected", "kernel_type_match_exhaustive"),
        ("bound-edit-rejected", "kernel_type_constraint"),
    ] {
        require(
            receipt.schemas.get(label).map(String::as_str) == Some(code),
            "recursive member or bound edit invalidation evidence omitted",
        )?;
    }
    require(
        receipt.library_package.starts_with("pkg_")
            && receipt.consumer_package.starts_with("pkg_")
            && receipt.receiver_package.starts_with("pkg_")
            && receipt.library_package != receipt.consumer_package
            && receipt.consumer_package != receipt.receiver_package
            && receipt.library_revision.starts_with("rev_")
            && receipt.changed_library_revision.starts_with("rev_")
            && receipt.library_revision != receipt.changed_library_revision
            && receipt.producers_removed
            && receipt.inspected_recursive_type,
        "recursive source transport, exact revision, or producer-removal proof missing",
    )?;
    let original = small();
    for (name, expected) in [
        ("small-leaves", json!([1, 2, 4])),
        ("count", json!(3)),
        ("sum", json!(7)),
        ("ordered-fold", json!(124)),
        ("identity", original.clone()),
        ("mapped", map_expected(&original, 3, 5)),
        (
            "retained",
            json!({"changed":map_expected(&original,3,5),"original":original}),
        ),
        ("composed", map_expected(&original, 3, 11)),
        ("composition-map", map_expected(&original, 3, 11)),
        ("empty-trap", branch(vec![])),
        ("text-leaves", json!(["a", "bc"])),
        ("owned-leaves", json!([{"text":"one"},{"text":"two"}])),
        (
            "trap-order",
            json!(["normalized_integer_division", "normalized_integer_overflow"]),
        ),
    ] {
        require(
            receipt.results.get(name) == Some(&expected),
            &format!("recursive observation {name} omitted or changed"),
        )?;
    }
    let session_expected: Vec<_> = session_steps()
        .into_iter()
        .map(|(_, _, reply)| reply)
        .collect();
    require(
        receipt.session.messages == session_expected
            && receipt.session.accept_matches
            && receipt.session.close_code == 1000
            && receipt.session.cleanup_complete,
        "recursive public session omitted full retained state, traversal, reset, isolation or cleanup",
    )?;
    require(
        receipt.scale_leaves == (1..=4096).collect::<Vec<_>>()
            && receipt.scale_shape == shape(&balanced())
            && receipt.scale_sum == 8_390_656
            && receipt.mapped_scale_sum == 25_192_448
            && receipt.changed_sum == 8,
        "recursive scale order/sum or exact body-replacement result missing",
    )
}

fn session_steps() -> Vec<(usize, &'static str, Value)> {
    let empty = branch(vec![]);
    let a = branch(vec![empty.clone(), leaf(json!("a"))]);
    let z = branch(vec![empty.clone(), leaf(json!("z"))]);
    let abc = branch(vec![a.clone(), leaf(json!("bc"))]);
    let zbc = branch(vec![z.clone(), leaf(json!("bc"))]);
    vec![
        (0, "a", json!({"contents":"a","state":a})),
        (1, "z", json!({"contents":"z","state":z})),
        (0, "bc", json!({"contents":"abc","state":abc})),
        (0, "", json!({"contents":"","state":empty})),
        (1, "bc", json!({"contents":"zbc","state":zbc})),
    ]
}

fn sessions(
    context: &mut Context,
    standard: &mut Package,
    library: &Package,
    names: &BTreeMap<String, String>,
) -> Result<(), DevError> {
    let found = context.cli(
        None,
        &[
            "package",
            "builtin",
            "query",
            "owners",
            "--kind",
            "external",
            "--name",
            "text-equal",
        ],
        true,
    )?;
    standard
        .symbols
        .insert("text-equal".into(), field(&found, "owner", "reference")?);
    let mut package = context.new_package("recursive-session")?;
    context.stage(&package, standard)?;
    context.stage(&package, library)?;
    context.apply(
        &mut package,
        &format!(
            "{}{}{}",
            binding("add", standard),
            binding("add", library),
            super::nominal_session::session_program(&standard.symbols, Some(names))
        ),
    )?;
    context.cli(
        Some(&package.path),
        &[
            "inspect",
            "owner",
            "task_function",
            &package.symbols["$handler"],
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    context.cli(Some(&package.path), &["check"], true)?;
    context.export(&mut package)?;
    let standalone = context.root.join("recursive-session-standalone");
    fs::create_dir(&standalone)?;
    let artifact = standalone.join("session.lkja");
    context.cli(
        Some(&package.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    let mut descriptor: Value = serde_json::from_slice(&process::read_bounded(
        &context.evidence.join("nominal-session.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    descriptor["grants"][0]["sharing_domain"] = json!("recursive-session-streams");
    fs::write(
        standalone.join("session.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::copy(&artifact, context.evidence.join("recursive-session.lkja"))?;
    fs::write(
        context.evidence.join("recursive-session.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::remove_dir_all(&package.path)?;
    fs::remove_file(&package.container)?;
    let observed = crate::service::nominal::probe_script(
        &context.binary,
        &standalone,
        &context.evidence,
        "recursive-session",
        2,
        &session_steps(),
    );
    fs::remove_dir_all(&standalone)?;
    let (observation, process) = observed?;
    context.receipt.recursive.session = observation;
    context.receipt.runners.push(process);
    Ok(())
}

fn output(
    context: &mut Context,
    package: &Package,
    target: &str,
    tree: &Value,
    expected: &Value,
) -> Result<Value, DevError> {
    output_arguments(context, package, target, &json!([tree]), expected)
}

fn output_arguments(
    context: &mut Context,
    package: &Package,
    target: &str,
    arguments: &Value,
    expected: &Value,
) -> Result<Value, DevError> {
    let records = context.cli(
        Some(&package.path),
        &["run", target, "--arguments", &arguments.to_string()],
        true,
    )?;
    require(
        field(&records, "execution", "differential")? == "equal",
        "recursive evaluators disagree",
    )?;
    let actual: Value = serde_json::from_str(&field(&records, "execution", "value")?)?;
    require(
        &actual == expected,
        &format!("recursive {target} differs from independent full-value expectation"),
    )?;
    Ok(actual)
}

fn references(package: &Package, names: &[&str]) -> Result<BTreeMap<String, String>, DevError> {
    names
        .iter()
        .map(|name| Ok(((*name).into(), reference(package, &format!("${name}"))?)))
        .collect()
}

fn receiver(library: &Package, consumer: &Package) -> Result<String, DevError> {
    let mut r = Request::default();
    r.text.push_str(
        "create.component as=$component module=$module name=received visibility=private\n",
    );
    // Concrete schema identities are recovered from the imported library. Each public
    // receiver forwards one complete application without inheriting a producer layout index.
    let finite_names = references(
        library,
        &[
            "node",
            "expr",
            "flip",
            "flip-next",
            "select-a",
            "reset-a",
            "wrapper-a",
        ],
    )?;
    super::recursive_schemas::concrete(&mut r, &finite_names);
    r.text.push_str(&format!(
        "type.named as=@Owned declaration={}\n",
        reference(consumer, "$owned")?
    ));
    for (label, ty) in [("i64", "i64"), ("text", "text"), ("owned", "@Owned")] {
        r.text.push_str(&format!("type.application as=@{label}-tree declaration={}\ntype.argument parent=@{label}-tree index=0 type={ty}\ntype.list as=@{label}-leaves item={ty}\n", reference(library, "$tree")?));
        for (suffix, result) in [
            ("flatten", format!("@{label}-leaves")),
            ("map", format!("@{label}-tree")),
        ] {
            forward(&mut r, consumer, &format!("{label}-{suffix}"), &result)?;
        }
    }
    r.text.push_str("type.structural-record as=@Retained\ntype.field parent=@Retained index=0 name=changed type=@i64-tree\ntype.field parent=@Retained index=1 name=original type=@i64-tree\n");
    for (name, result) in [
        ("mapped", "@i64-tree"),
        ("composed", "@i64-tree"),
        ("composition-map", "@i64-tree"),
        ("ordered-fold", "i64"),
        ("sum", "i64"),
        ("count", "i64"),
        ("retained", "@Retained"),
        ("trapped", "@i64-tree"),
    ] {
        forward(&mut r, consumer, name, result)?;
    }
    for (name, result) in [
        ("scale-shape", "@i64-leaves"),
        ("scale-leaves", "@i64-leaves"),
        ("scale-sum", "i64"),
        ("scale-mapped-shape", "@i64-leaves"),
        ("scale-mapped-sum", "i64"),
    ] {
        let start = r.local(&format!("${name}_start"));
        let count = r.local(&format!("${name}_count"));
        let body = r.call(
            &reference(consumer, &format!("${name}"))?,
            &[],
            &[start, count],
        );
        r.function(name, result, &body, &[("start", "i64"), ("count", "i64")]);
        r.target(name, result, &["i64", "i64"]);
    }
    Ok(r.text)
}

fn forward(r: &mut Request, consumer: &Package, name: &str, result: &str) -> Result<(), DevError> {
    let argument = if name.starts_with("text-") {
        "@text-tree"
    } else if name.starts_with("owned-") {
        "@owned-tree"
    } else {
        "@i64-tree"
    };
    let input = r.local(&format!("${name}_tree"));
    let body = r.call(&reference(consumer, &format!("${name}"))?, &[], &[input]);
    r.function(name, result, &body, &[("tree", argument)]);
    r.target(name, result, &[argument]);
    Ok(())
}

pub(super) fn workflow(context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    super::recursive_schemas::negatives(context)?;
    standard.container = context.root.join("recursive-standard.lkjp");
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
    for (name, kind) in [
        ("list-length", "external"),
        ("list-get", "external"),
        ("list-append", "external"),
        ("i64-equal", "external"),
        ("divide", "external"),
        ("subtract", "external"),
        ("function-constant", "pure_function"),
    ] {
        let records = context.cli(
            None,
            &[
                "package", "builtin", "query", "owners", "--kind", kind, "--name", name,
            ],
            true,
        )?;
        standard
            .symbols
            .insert(name.into(), field(&records, "owner", "reference")?);
    }
    let mut library = context.new_package("recursive-library")?;
    context.stage(&library, standard)?;
    context.apply(
        &mut library,
        &format!(
            "{}{}{}",
            binding("add", standard),
            module(),
            super::recursive_program::library(&standard.symbols)
        ),
    )?;
    context.export(&mut library)?;
    for name in [
        "node",
        "uninhabited",
        "expr",
        "group",
        "flip",
        "select-a",
        "select-b",
        "reset-a",
        "reset-b",
        "wrapper-a",
        "wrapper",
        "signature",
    ] {
        context
            .receipt
            .recursive
            .schemas
            .insert(name.into(), library.symbols[&format!("${name}")].clone());
    }
    let library_a = library.clone();
    let names = references(
        &library,
        &[
            "tree",
            "leaf",
            "branch",
            "tree-map",
            "tree-fold",
            "tree-shape",
            "tree-flatten",
            "tree-sum",
            "snapshot",
            "node",
            "expr",
            "flip",
            "flip-next",
            "select-a",
            "reset-a",
            "wrapper-a",
        ],
    )?;
    sessions(context, standard, &library, &names)?;
    let persistence = super::recursive_data::start(context, standard, &library, &names)?;
    let mut consumer = context.new_package("recursive-consumer")?;
    context.stage(&consumer, standard)?;
    context.stage(&consumer, &library)?;
    context.apply(
        &mut consumer,
        &format!(
            "{}{}{}{}",
            binding("add", standard),
            binding("add", &library),
            module(),
            super::recursive_program::consumer(&standard.symbols, &names)
        ),
    )?;
    context.export(&mut consumer)?;
    let consumer_a = consumer.clone();
    let mut receiver_package = context.new_package("recursive-receiver")?;
    for dependency in [&*standard, &library, &consumer] {
        context.stage(&receiver_package, dependency)?;
    }
    context.apply(
        &mut receiver_package,
        &format!(
            "{}{}{}{}{}",
            binding("add", standard),
            binding("add", &library),
            binding("add", &consumer),
            module(),
            receiver(&library, &consumer)?
        ),
    )?;
    let inspected = context.cli(
        Some(&receiver_package.path),
        &[
            "package",
            "dependency",
            "inspect",
            "owner",
            "variant",
            &library.symbols["$tree"],
            "--package-revision",
            &library.logical,
        ],
        true,
    )?;
    require(
        inspected.iter().filter(|r| r.operation == "case").count() == 2
            && inspected.iter().any(|r| r.operation == "type-parameter"),
        "recursive projection omitted members or parameter",
    )?;
    context.receipt.recursive.inspected_recursive_type = true;
    for (label, changes, code) in [
        (
            "member",
            format!(
                "add.case as=$forbidden variant={} name=forbidden payload=secret\n",
                library.symbols["$tree"]
            ),
            "kernel_type_match_exhaustive",
        ),
        (
            "bound",
            format!(
                "set.type-parameter-constraint parameter={} constraint=capture-safe\n",
                library.symbols["$TreeT"]
            ),
            "kernel_type_constraint",
        ),
    ] {
        let request = context
            .evidence
            .join(format!("recursive-invalid-{label}-edit.lkjc"));
        fs::write(
            &request,
            format!("request base={}\n{changes}", library.revision),
        )?;
        context.reject(
            &library,
            &[
                "change",
                "plan",
                "--input-file",
                &request.display().to_string(),
            ],
            code,
        )?;
        context
            .receipt
            .recursive
            .schemas
            .insert(format!("{label}-edit-rejected"), code.into());
    }
    let mut replacement = Request::default();
    let input = replacement.local(&library.symbols["$tree-sum_tree"]);
    let callback = replacement.function_value(&standard.symbols["add"]);
    let zero = replacement.integer(0);
    let sum = replacement.call(
        &names["tree-fold"],
        &["i64", "i64"],
        &[input, callback, zero],
    );
    let one = replacement.integer(1);
    let body = replacement.call(&standard.symbols["add"], &[], &[sum, one]);
    replacement.text.push_str(&format!(
        "replace.body function={} body={body}\n",
        library.symbols["$tree-sum"]
    ));
    context.apply(&mut library, &replacement.text)?;
    context.export(&mut library)?;
    super::recursive_data::finish(context, persistence, &library)?;
    context.stage(&consumer, &library)?;
    context.apply(&mut consumer, &binding("replace", &library))?;
    context.export(&mut consumer)?;
    context.stage(&receiver_package, &library)?;
    context.stage(&receiver_package, &consumer)?;
    context.receipt.recursive.library_package = library.id.clone();
    context.receipt.recursive.library_revision = library_a.revision.clone();
    context.receipt.recursive.changed_library_revision = library.revision.clone();
    context.receipt.recursive.consumer_package = consumer.id.clone();
    context.receipt.recursive.receiver_package = receiver_package.id.clone();
    fs::remove_dir_all(&library.path)?;
    fs::remove_dir_all(&consumer.path)?;
    for container in [
        &library_a.container,
        &library.container,
        &consumer_a.container,
        &consumer.container,
    ] {
        fs::remove_file(container)?;
    }
    context.receipt.recursive.producers_removed = !library.path.exists() && !consumer.path.exists();
    context.cache_recovery(&receiver_package, "recursive-received")?;
    context.cli(Some(&receiver_package.path), &["check"], true)?;
    for (name, value) in super::recursive_schemas::values() {
        let actual = output(context, &receiver_package, name, &value, &value)?;
        context
            .receipt
            .recursive
            .results
            .insert(name.into(), actual);
    }
    let flip = output(
        context,
        &receiver_package,
        "flip-next",
        &json!({"case":"stop"}),
        &json!({"case":"next","value":{"case":"stop"}}),
    )?;
    context
        .receipt
        .recursive
        .results
        .insert("flip-next".into(), flip);
    let specification = process::ProcessSpec {
        command: vec![
            std::env::current_exe()?.display().to_string(),
            "recursive-probe".into(),
            receiver_package.path.display().to_string(),
        ],
        cwd: context.root.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context.evidence.join("recursive-resources.stdout"),
        stderr_path: context.evidence.join("recursive-resources.stderr"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&specification, &context.evidence);
    require(
        observation.status == process::ProcessStatus::Passed,
        "source-bound recursive resource probe failed",
    )?;
    context.receipt.recursive.resources = serde_json::from_slice(&process::read_bounded(
        &specification.stdout_path,
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    context.receipt.runners.push(observation);
    let original = small();
    for (name, target, expected) in [
        ("small-leaves", "i64-flatten", json!([1, 2, 4])),
        ("count", "count", json!(3)),
        ("sum", "sum", json!(7)),
        ("ordered-fold", "ordered-fold", json!(124)),
        ("identity", "i64-map", original.clone()),
        ("mapped", "mapped", map_expected(&original, 3, 5)),
        ("composed", "composed", map_expected(&original, 3, 11)),
        (
            "composition-map",
            "composition-map",
            map_expected(&original, 3, 11),
        ),
        (
            "retained",
            "retained",
            json!({"changed":map_expected(&original,3,5),"original":original}),
        ),
    ] {
        let value = output(context, &receiver_package, target, &original, &expected)?;
        context.receipt.recursive.results.insert(name.into(), value);
    }
    for (label, values) in [
        ("text", vec![json!("a"), json!("bc")]),
        ("owned", vec![json!({"text":"one"}), json!({"text":"two"})]),
    ] {
        let input = branch(vec![
            leaf(values[0].clone()),
            branch(vec![leaf(values[1].clone()), branch(vec![])]),
        ]);
        output(
            context,
            &receiver_package,
            &format!("{label}-map"),
            &input,
            &input,
        )?;
        let actual = output(
            context,
            &receiver_package,
            &format!("{label}-flatten"),
            &input,
            &json!(values),
        )?;
        context
            .receipt
            .recursive
            .results
            .insert(format!("{label}-leaves"), actual);
    }
    let empty = branch(vec![]);
    let actual = output(context, &receiver_package, "trapped", &empty, &empty)?;
    context
        .receipt
        .recursive
        .results
        .insert("empty-trap".into(), actual);
    let trapped = context.cli(
        Some(&receiver_package.path),
        &[
            "run",
            "trapped",
            "--arguments",
            &json!([original]).to_string(),
        ],
        false,
    )?;
    require(
        trapped.iter().all(|r| r.operation != "execution"),
        "trapped map published a successful partial tree",
    )?;
    require(
        field(&trapped, "diagnostic", "code")? == "normalized_integer_division",
        "map did not trap on the second leaf before the fourth",
    )?;
    let reversed = branch(vec![leaf(json!(4)), leaf(json!(2)), leaf(json!(1))]);
    let trapped = context.cli(
        Some(&receiver_package.path),
        &[
            "run",
            "trapped",
            "--arguments",
            &json!([reversed]).to_string(),
        ],
        false,
    )?;
    require(
        field(&trapped, "diagnostic", "code")? == "normalized_integer_overflow"
            && trapped.iter().all(|r| r.operation != "execution"),
        "reversed traversal did not expose the first overflow before division",
    )?;
    context.receipt.recursive.results.insert(
        "trap-order".into(),
        json!(["normalized_integer_division", "normalized_integer_overflow"]),
    );
    let large = balanced();
    let expected = ordered(&large)?;
    require(
        expected == (1..=4096).collect::<Vec<_>>(),
        "independently generated balanced input order changed",
    )?;
    let seeds = json!([1, 4096]);
    let expected_shape = shape(&large);
    fs::write(
        context.evidence.join("recursive-balanced-input.json"),
        evidence::encode_json(&large)?,
    )?;
    output_arguments(
        context,
        &receiver_package,
        "scale-shape",
        &seeds,
        &json!(expected_shape),
    )?;
    output_arguments(
        context,
        &receiver_package,
        "scale-leaves",
        &seeds,
        &json!(expected),
    )?;
    let sum: i64 = expected.iter().sum();
    output_arguments(context, &receiver_package, "scale-sum", &seeds, &json!(sum))?;
    let mapped = map_expected(&large, 3, 5);
    output_arguments(
        context,
        &receiver_package,
        "scale-mapped-shape",
        &seeds,
        &json!(expected_shape),
    )?;
    let mapped_sum: i64 = ordered(&mapped)?.iter().sum();
    output_arguments(
        context,
        &receiver_package,
        "scale-mapped-sum",
        &seeds,
        &json!(mapped_sum),
    )?;
    context.receipt.recursive.scale_leaves = expected;
    context.receipt.recursive.scale_shape = expected_shape;
    context.receipt.recursive.scale_sum = sum;
    context.receipt.recursive.mapped_scale_sum = mapped_sum;
    context.apply(
        &mut receiver_package,
        &format!(
            "{}{}",
            binding("replace", &library),
            binding("replace", &consumer)
        ),
    )?;
    output(context, &receiver_package, "sum", &original, &json!(8))?;
    context.receipt.recursive.changed_sum = 8;
    context.cache_recovery(&receiver_package, "recursive-body-replacement")?;
    fs::remove_dir_all(&receiver_package.path)?;
    validate(&context.receipt.recursive)
}
