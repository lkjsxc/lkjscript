//! One transported task library, exercised with consumer-owned configuration and data callbacks.
use super::*;
use lkjscript::platform::data::DataLimits;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn records_json(records: &[CompactRecord]) -> Value {
    json!(
        records
            .iter()
            .map(|r| json!({"operation":r.operation,
        "fields":r.fields.iter().map(|f| (&f.name, &f.value)).collect::<BTreeMap<_,_>>()}))
            .collect::<Vec<_>>()
    )
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectReceipt {
    pub library_package: String,
    pub library_revision: String,
    pub consumer_package: String,
    pub consumer_revision: String,
    pub producers_removed: bool,
    pub definitions: Vec<Value>,
    pub rounds: Vec<Value>,
    pub changed_library_revision: String,
    pub changed_consumer_revision: String,
    pub artifact_bindings: Vec<Value>,
    pub stale_artifact_rejection: String,
    pub authority_unchanged: bool,
    pub public_rejections: Vec<String>,
    pub resources: Value,
    pub cancellation: Value,
    pub iteration: Value,
    pub foreground: super::foreground::ForegroundReceipt,
}

fn transaction_cancellation(
    context: &mut Context,
    standalone: &Path,
    function: &str,
) -> Result<(), DevError> {
    #[cfg(not(test))]
    let verifier = std::env::current_exe()?;
    #[cfg(test)]
    let verifier = PathBuf::from(std::env::var_os("LKJSCRIPT_EFFECTS_VERIFIER").ok_or_else(
        || DevError::usage("effect iteration requires an explicitly frozen verifier executable"),
    )?);
    let specification = process::ProcessSpec {
        command: vec![
            verifier.display().to_string(),
            "effect-transaction-probe".into(),
            standalone
                .join("service.deployment.json")
                .display()
                .to_string(),
            function.into(),
        ],
        cwd: context.root.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context.evidence.join("effect-transaction.stdout"),
        stderr_path: context.evidence.join("effect-transaction.stderr"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&specification, &context.evidence);
    context.receipt.runners.push(observation.clone());
    require(
        observation.status == process::ProcessStatus::Passed,
        "source-bound effect transaction cancellation failed",
    )?;
    context.receipt.effects.cancellation = serde_json::from_slice(&process::read_bounded(
        &specification.stdout_path,
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    Ok(())
}

fn resources(context: &mut Context, consumer: &Package) -> Result<(), DevError> {
    #[cfg(not(test))]
    let verifier = std::env::current_exe()?;
    #[cfg(test)]
    let verifier = PathBuf::from(std::env::var_os("LKJSCRIPT_EFFECTS_VERIFIER").ok_or_else(
        || DevError::usage("effect iteration requires an explicitly frozen verifier executable"),
    )?);
    let specification = process::ProcessSpec {
        command: vec![
            verifier.display().to_string(),
            "effect-probe".into(),
            consumer.path.display().to_string(),
        ],
        cwd: context.root.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(180),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context.evidence.join("effect-resources.stdout"),
        stderr_path: context.evidence.join("effect-resources.stderr"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&specification, &context.evidence);
    context.receipt.runners.push(observation.clone());
    require(
        observation.status == process::ProcessStatus::Passed,
        "source-bound effect resource probe failed",
    )?;
    context.receipt.effects.resources = serde_json::from_slice(&process::read_bounded(
        &specification.stdout_path,
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    Ok(())
}

fn reject_bad_effect_requests(
    context: &mut Context,
    consumer: &Package,
    bindings: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> Result<(), DevError> {
    for (label, code) in [
        ("excess", "kernel_effect_argument_count"),
        ("omitted", "kernel_effect_argument_count"),
        ("wrong-row", "kernel_type_root"),
        ("empty-task-pure", "kernel_type_pure_task_call"),
    ] {
        let mut r = crate::pure_tail_program::Request::default();
        if label != "empty-task-pure" {
            r.text.push_str(&format!("type.application as=@Job declaration={}\ntype.argument parent=@Job index=0 type=i64\n", library["job"]));
            let requirement = if label == "wrong-row" {
                "$data"
            } else {
                "$config"
            };
            r.text.push_str(&format!("effect.row as=@ExpectedRow\neffect.requirement parent=@ExpectedRow index=0 requirement={}\ntype.task-function as=@Expected result=i64 effect=@ExpectedRow\ntype.argument parent=@Expected index=0 type=i64\ntype.argument parent=@Expected index=1 type=@Job\n", reference(consumer, requirement)?));
        }
        let (body, result) = if label == "empty-task-pure" {
            let zero = r.integer(0);
            r.text.push_str(&format!("create.function as=$empty module={} name=empty-task visibility=private result=i64 effect=task body={zero}\n", bindings["module"]));
            let callee = r.function_value("$empty");
            (r.invoke(&callee, &[]), "i64")
        } else if label == "omitted" {
            let callee = r.function_value(&library["task-map"]);
            r.types(&callee, &["@Job", "i64"]);
            (callee, "@Expected")
        } else {
            let callee = r.function_value(&reference(consumer, "$config-job")?);
            if label == "excess" {
                r.effects(&callee, &["@ExpectedRow"]);
            }
            (callee, "@Expected")
        };
        r.text.push_str(&format!("create.function as=$bad module={} name=bad-{label} visibility=private result={result} effect=pure body={body}\n", bindings["module"]));
        let request = context
            .evidence
            .join(format!("effect-rejection-{label}.lkjc"));
        fs::write(
            &request,
            format!("request base={}\n{}", consumer.revision, r.text),
        )?;
        context.reject(
            consumer,
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
            .effects
            .public_rejections
            .push(format!("{label}:{code}"));
        if label == "wrong-row" {
            let valid = r.text.replace(
                &reference(consumer, "$data")?,
                &reference(consumer, "$config")?,
            );
            let planned = context.cli(
                Some(&consumer.path),
                &[
                    "change",
                    "plan",
                    "--input",
                    &format!("request base={}\n{valid}", consumer.revision),
                ],
                true,
            )?;
            context.reject(
                consumer,
                &[
                    "change",
                    "apply",
                    "--input-file",
                    &request.display().to_string(),
                    "--plan",
                    &field(&planned, "plan", "token")?,
                ],
                "change_request_commitment_mismatch",
            )?;
            context
                .receipt
                .effects
                .public_rejections
                .push("wrong-row-apply:change_request_commitment_mismatch".to_owned());
        }
    }
    Ok(())
}

fn definition(
    context: &mut Context,
    package: &Package,
    kind: &str,
    symbol: &str,
) -> Result<Value, DevError> {
    let id = &package.symbols[symbol];
    let mut continuation: Option<String> = None;
    let mut identity = None;
    let mut first = None;
    let mut rows = Vec::new();
    let mut pages = 0;
    loop {
        require(
            pages < 64,
            "effect definition exceeded its bounded page inventory",
        )?;
        let mut arguments = vec![
            "inspect",
            "owner",
            kind,
            id,
            "--detail",
            "definition",
            "--limit",
            "32",
            "--bytes",
            "16384",
        ];
        if let Some(token) = &continuation {
            arguments.extend(["--continuation", token]);
        }
        let records = context.cli(Some(&package.path), &arguments, true)?;
        let digest = field(&records, "projection", "digest")?;
        let total = field(&records, "projection", "total-records")?;
        let revision = field(&records, "revision", "observed")?;
        require(
            revision == package.revision,
            "effect definition has a foreign revision",
        )?;
        let binding = (digest, total);
        require(
            identity.as_ref().is_none_or(|prior| prior == &binding),
            "effect definition changed between pages",
        )?;
        identity = Some(binding);
        require(
            field(&records, "page", "start")? == rows.len().to_string(),
            "effect definition page gap or overlap",
        )?;
        rows.extend(
            records
                .iter()
                .filter(|r| r.operation.starts_with("definition."))
                .cloned(),
        );
        require(
            field(&records, "page", "end")? == rows.len().to_string(),
            "effect definition page count differs",
        )?;
        pages += 1;
        if field(&records, "page", "complete")? == "true" {
            break;
        }
        let token = field(&records, "continuation", "token")?;
        if first.is_none() {
            first = Some(token.clone());
        }
        continuation = Some(token);
    }
    let (digest, total) = identity.ok_or_else(|| DevError::corrupt("effect definition missing"))?;
    require(
        total == rows.len().to_string(),
        "effect definition omitted terminal records",
    )?;
    require(
        rows.iter()
            .filter(|r| r.operation == "definition.effect-parameter")
            .count()
            == 1,
        "effect definition lost its exact effect parameter",
    )?;
    if kind == "task_function" {
        require(
            rows.iter()
                .any(|r| r.operation == "definition.effect-argument"),
            "task definition lost explicit application rows",
        )?;
    }
    Ok(
        json!({"function":id,"revision":package.revision,"digest":digest,"records":records_json(&rows),"pages":pages,"first_continuation":first}),
    )
}

fn reject_iteration_requests(
    context: &mut Context,
    consumer: &Package,
    bindings: &BTreeMap<String, String>,
    library: &BTreeMap<String, String>,
) -> Result<(), DevError> {
    for (label, code) in [
        ("iteration-result", "kernel_type_argument"),
        ("iteration-effect-missing", "kernel_effect_argument_count"),
        ("iteration-effect-wrong", "kernel_type_argument"),
        ("iteration-pure", "kernel_type_pure_task_call"),
    ] {
        let mut r = crate::pure_tail_program::Request::default();
        let output = if label == "iteration-result" {
            "bool"
        } else {
            "i64"
        };
        r.text.push_str(&format!("type.application as=@Transition declaration={}\ntype.argument parent=@Transition index=0 type=unit\ntype.argument parent=@Transition index=1 type={output}\neffect.row as=@Row\n",library["iteration-step"]));
        if label == "iteration-effect-wrong" {
            r.text.push_str(&format!(
                "effect.requirement parent=@Row index=0 requirement={}\n",
                reference(consumer, "$config")?
            ));
        }
        let value = if output == "bool" {
            r.expression("bool", "value=true")
        } else {
            r.integer(0)
        };
        let done = r.expression(
            "variant",
            &format!("case={} payload={value}", library["iteration-done"]),
        );
        r.types(&done, &["unit", output]);
        r.text.push_str(&format!("create.function as=$step module={} name=step-{label} visibility=private result=@Transition effect=task body={done}\nadd.parameter as=$state function=$step name=state type=unit\n",bindings["module"]));
        let initial = r.expression("unit", "");
        let step = r.function_value("$step");
        let call = r.call(&library["task-iterate"], &["unit", "i64"], &[initial, step]);
        if label != "iteration-effect-missing" {
            r.effects(&call, &["@Row"]);
        }
        let kind = if label == "iteration-pure" {
            "pure"
        } else {
            "task"
        };
        r.text.push_str(&format!("create.function as=$bad module={} name=bad-{label} visibility=private result=i64 effect={kind} body={call}\n",bindings["module"]));
        if label == "iteration-effect-wrong" {
            r.text.push_str(&format!(
                "effect.requirement parent=$bad index=0 requirement={}\n",
                reference(consumer, "$config")?
            ));
        }
        let request = context
            .evidence
            .join(format!("effect-rejection-{label}.lkjc"));
        fs::write(
            &request,
            format!("request base={}\n{}", consumer.revision, r.text),
        )?;
        context.reject(
            consumer,
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
            .effects
            .public_rejections
            .push(format!("{label}:{code}"));
        if label == "iteration-pure" {
            let valid = r.text.replace("effect=pure", "effect=task");
            let planned = context.cli(
                Some(&consumer.path),
                &[
                    "change",
                    "plan",
                    "--input",
                    &format!("request base={}\n{valid}", consumer.revision),
                ],
                true,
            )?;
            context.reject(
                consumer,
                &[
                    "change",
                    "apply",
                    "--input-file",
                    &request.display().to_string(),
                    "--plan",
                    &field(&planned, "plan", "token")?,
                ],
                "change_request_commitment_mismatch",
            )?;
            context
                .receipt
                .effects
                .public_rejections
                .push("iteration-pure-apply:change_request_commitment_mismatch".into());
        }
    }
    Ok(())
}

fn interface(
    context: &mut Context,
    consumer: &Package,
    library: &Package,
) -> Result<Value, DevError> {
    let mut continuation: Option<String> = None;
    let mut owners = BTreeSet::new();
    let mut pages = 0;
    let mut total = None;
    loop {
        require(
            pages < 64,
            "effect interface exceeded its finite page inventory",
        )?;
        let mut arguments = vec![
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &library.logical,
            "--limit",
            "5",
            "--bytes",
            "4096",
        ];
        if let Some(token) = &continuation {
            arguments.extend(["--continuation", token]);
        }
        let records = context.cli(Some(&consumer.path), &arguments, true)?;
        let remaining = field(&records, "summary", "matched")?
            .parse::<usize>()
            .map_err(|_| DevError::corrupt("effect interface count is invalid"))?;
        let expected = owners
            .len()
            .checked_add(remaining)
            .ok_or_else(|| DevError::corrupt("effect interface count overflow"))?;
        require(
            total.is_none_or(|total| total == expected),
            "effect interface count changed between pages",
        )?;
        total = Some(expected);
        for record in records.iter().filter(|r| r.operation == "owner") {
            require(
                owners.insert(record_field(record, "reference")?),
                "effect interface repeated an owner",
            )?;
        }
        pages += 1;
        if field(&records, "summary", "truncated")? == "false" {
            require(
                total == Some(owners.len()),
                "effect interface lost owners across pages",
            )?;
            break;
        }
        continuation = Some(field(&records, "continuation", "token")?);
    }
    for symbol in [
        "$task-map",
        "$task-fold-left",
        "$configure-task",
        "$configure-task-E",
        "$task-map-E",
        "$task-fold-left-E",
        "$task-iterate",
        "$iteration-step",
        "$iteration-State",
        "$iteration-Output",
        "$iteration-continue",
        "$iteration-done",
        "$make-iteration",
        "$make-iteration-E",
    ] {
        require(
            owners.contains(&reference(library, symbol)?),
            "transported public interface omitted effect-owned meaning",
        )?;
    }
    let detail = context.cli(
        Some(&consumer.path),
        &[
            "package",
            "dependency",
            "inspect",
            "owner",
            "pure_function",
            &library.symbols["$configure-task"],
            "--package-revision",
            &library.logical,
        ],
        true,
    )?;
    require(
        detail.iter().any(|r| r.operation == "effect-parameter"),
        "factory interface omitted its effect parameter",
    )?;
    require(
        detail
            .iter()
            .any(|r| r.operation == "type.effect-parameter"),
        "nested task signature omitted its symbolic row",
    )?;
    let family = context.cli(
        Some(&consumer.path),
        &[
            "package",
            "dependency",
            "inspect",
            "owner",
            "variant",
            &library.symbols["$iteration-step"],
            "--package-revision",
            &library.logical,
        ],
        true,
    )?;
    Ok(
        json!({"package_revision":library.logical,"pages":pages,"owners":owners,"factory":records_json(&detail),"iteration_family":records_json(&family)}),
    )
}

pub(super) fn duplicate_map_output(
    standard: &BTreeMap<String, String>,
    library: &Package,
) -> String {
    let mut r = crate::pure_tail_program::Request::default();
    r.text.push_str(&format!(
        "type.parameter as=@Output parameter={}\n",
        library.symbols["$task-map-step-Output"]
    ));
    let mapper = r.local(&library.symbols["$task-map-step_mapper"]);
    let input = r.local(&library.symbols["$task-map-step_input"]);
    let mapped = r.invoke(&mapper, &[input]);
    let prior = r.local(&library.symbols["$task-map-step_output"]);
    let once = r.local("$mapped");
    let once = r.call(&standard["list-append"], &["@Output"], &[prior, once]);
    let twice = r.local("$mapped");
    let twice = r.call(&standard["list-append"], &["@Output"], &[once, twice]);
    let body = r.expression("let", &format!("body={twice}"));
    r.text.push_str(&format!("expression.binding parent={body} index=0 as=$mapped name=mapped type=@Output value={mapped}\nreplace.body function={} body={body}\n",library.symbols["$task-map-step"]));
    // The revised exact dependency also changes the data-dependent witness: its pure factory
    // extends the configured threshold by one stride while preserving its public contract.
    r.text.push_str(&format!(
        "effect.row as=@FactoryRow\neffect.parameter parent=@FactoryRow index=0 parameter={}/{}\n",
        library.id, library.symbols["$make-iteration-E"]
    ));
    let function = r.function_value(&format!(
        "{}/{}",
        library.id, library.symbols["$iteration-advance"]
    ));
    r.effects(&function, &["@FactoryRow"]);
    let stride = r.local(&library.symbols["$make-iteration_stride"]);
    let threshold = r.local(&library.symbols["$make-iteration_threshold"]);
    let extra = r.local(&library.symbols["$make-iteration_stride"]);
    let threshold = r.call(&standard["add"], &[], &[threshold, extra]);
    let observe = r.local(&library.symbols["$make-iteration_observe"]);
    let body = r.bind(&function, &[stride, threshold, observe]);
    r.text.push_str(&format!(
        "replace.body function={} body={body}\n",
        library.symbols["$make-iteration"]
    ));
    r.text
}

pub(super) fn discover(context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    for name in [
        "add",
        "subtract",
        "multiply",
        "divide",
        "i64-equal",
        "less",
        "list-get",
        "list-length",
        "list-append",
        "list-map",
        "list-fold-left",
        "json-encode",
        "json-decode-or",
        "data-encode",
        "data-decode-or",
        "text-equal",
        "text-concat",
        "Configuration",
        "ByteStream",
        "DataStore",
        "DataKeyPart",
        "DataExpectation",
        "DataEntry",
    ] {
        let owners = context.cli(
            None,
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        )?;
        standard
            .symbols
            .insert(name.into(), field(&owners, "owner", "reference")?);
    }
    for (name, kind, member) in [
        ("Configuration", "interface", "operation"),
        ("ByteStream", "interface", "operation"),
        ("DataStore", "interface", "operation"),
        ("DataKeyPart", "variant", "case"),
        ("DataExpectation", "variant", "case"),
        ("DataEntry", "record", "field"),
    ] {
        let owner = standard.symbols[name]
            .rsplit('/')
            .next()
            .ok_or_else(|| DevError::corrupt("standard identity missing"))?
            .to_owned();
        let records = context.cli(
            None,
            &["package", "builtin", "inspect", "owner", kind, &owner],
            true,
        )?;
        for record in records.iter().filter(|r| r.operation == "owner") {
            if record_field(record, "kind")? == member {
                standard.symbols.insert(
                    format!("{name}.{}", record_field(record, "name")?),
                    record_field(record, "reference")?,
                );
            }
        }
    }
    Ok(())
}

pub(super) fn job(n: i64) -> Value {
    json!({"case":"item","value":{"value":n}})
}

fn request(
    address: std::net::SocketAddr,
    mode: &str,
    prefix: i64,
    values: Vec<Value>,
    expected: Option<Value>,
) -> Result<Value, DevError> {
    let input = json!({"mode":mode,"prefix":prefix,"values":values});
    let response =
        crate::http_probe::request(address, "GET", "/", &serde_json::to_vec(&input)?, &[])?;
    let status = if expected.is_some() { 200 } else { 500 };
    require(
        response.status == status,
        &format!(
            "effect library {mode} status {} differs from {status}: {:?}",
            response.status, response.headers
        ),
    )?;
    let value = if let Some(expected) = expected {
        let value: Value = serde_json::from_slice(&response.body)?;
        require(
            value == expected,
            &format!(
                "effect library {mode} complete result differs: expected {expected}, received {value}"
            ),
        )?;
        value
    } else {
        let expected_code = if mode == "nested" {
            "normalized_transaction_nested"
        } else {
            "normalized_integer_division"
        };
        require(
            response
                .headers
                .get("x-lkjscript-failure-code")
                .is_some_and(|code| code == expected_code),
            "failed task traversal has a different failure identity",
        )?;
        json!(String::from_utf8_lossy(&response.body))
    };
    Ok(
        json!({"request":input,"status":response.status,"result":value,"failure":response.headers.get("x-lkjscript-failure-code"),"elapsed_nanoseconds":response.elapsed_nanoseconds}),
    )
}

pub(super) fn validate(parent: &Receipt, root: &Path) -> Result<(), DevError> {
    let receipt = &parent.effects;
    require(
        receipt.library_package.starts_with("pkg_")
            && receipt.consumer_package.starts_with("pkg_")
            && receipt.library_package != receipt.consumer_package
            && receipt.producers_removed
            && receipt.authority_unchanged,
        "effect library isolation or unchanged authority proof is absent",
    )?;
    require(
        receipt.public_rejections
            == [
                "excess:kernel_effect_argument_count",
                "omitted:kernel_effect_argument_count",
                "wrong-row:kernel_type_root",
                "wrong-row-apply:change_request_commitment_mismatch",
                "empty-task-pure:kernel_type_pure_task_call",
                "iteration-result:kernel_type_argument",
                "iteration-effect-missing:kernel_effect_argument_count",
                "iteration-effect-wrong:kernel_type_argument",
                "iteration-pure:kernel_type_pure_task_call",
                "iteration-pure-apply:change_request_commitment_mismatch",
            ],
        "public effect argument, row, or empty-task negatives are absent",
    )?;
    for (package, revisions) in [
        (
            &receipt.library_package,
            [&receipt.library_revision, &receipt.changed_library_revision],
        ),
        (
            &receipt.consumer_package,
            [
                &receipt.consumer_revision,
                &receipt.changed_consumer_revision,
            ],
        ),
    ] {
        require(
            revisions[0].starts_with("rev_")
                && revisions[1].starts_with("rev_")
                && revisions[0] != revisions[1],
            "effect source replacement is absent",
        )?;
        for revision in revisions {
            require(
                parent
                    .producer_inventories
                    .iter()
                    .any(|p| &p.package == package && &p.semantic_revision == revision),
                "effect source is absent from independent transported inventory",
            )?;
        }
    }
    require(
        receipt.definitions.len() == 6,
        "complete effect definitions and interface observations are missing",
    )?;
    for definition in &receipt.definitions[..5] {
        require(
            definition["revision"] == receipt.library_revision
                && definition["pages"]
                    .as_u64()
                    .is_some_and(|n| n > 1 && n < 64),
            "effect definition pagination is not bound to the source",
        )?;
        let records = definition["records"]
            .as_array()
            .ok_or_else(|| DevError::corrupt("effect definition records absent"))?;
        require(
            records
                .iter()
                .filter(|r| r["operation"] == "definition.effect-parameter")
                .count()
                == 1,
            "effect parameter inspection is incomplete",
        )?;
    }
    let imported = &receipt.definitions[5];
    require(
        imported["pages"].as_u64().is_some_and(|n| n > 1)
            && imported["owners"].as_array().is_some_and(|v| v.len() == 49),
        "effect public interface pagination is incomplete",
    )?;
    require(
        imported["factory"].as_array().is_some_and(|records| {
            records
                .iter()
                .any(|r| r["operation"] == "type.effect-parameter")
        }),
        "nominal factory task row is absent from inspection",
    )?;
    let family = imported["iteration_family"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("ordinary iteration family inspection absent"))?;
    let parameters = family
        .iter()
        .filter(|r| r["operation"] == "type-parameter")
        .collect::<Vec<_>>();
    let cases = family
        .iter()
        .filter(|r| r["operation"] == "case")
        .collect::<Vec<_>>();
    require(
        parameters.len() == 2
            && parameters[0]["fields"]["index"] == "0"
            && parameters[0]["fields"]["name"] == "State"
            && parameters[1]["fields"]["index"] == "1"
            && parameters[1]["fields"]["name"] == "Output"
            && parameters
                .iter()
                .all(|r| r["fields"]["constraint"] == "none")
            && cases.len() == 2
            && cases
                .iter()
                .any(|r| r["fields"]["name"] == "continue" && r["fields"]["payload"] == "true")
            && cases
                .iter()
                .any(|r| r["fields"]["name"] == "done" && r["fields"]["payload"] == "true"),
        "iteration nominal parameters, cases or ordinary constraints changed",
    )?;
    require(
        receipt.rounds.len() == 4,
        "effect HTTP and recovery rounds are incomplete",
    )?;
    for (index, name) in [
        "effects-initial",
        "effects-restarted",
        "effects-successor",
        "effects-old-supported",
    ]
    .into_iter()
    .enumerate()
    {
        let observed: Value = serde_json::from_slice(&process::read_bounded(
            &root.join(format!("{name}.json")),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        require(
            observed == receipt.rounds[index],
            "effect HTTP observations changed in transfer",
        )?;
        let rows = observed["observations"]
            .as_array()
            .ok_or_else(|| DevError::corrupt("effect HTTP observations absent"))?;
        let failures = rows.iter().filter(|r| r["status"] == 500).count();
        require(
            observed["cleanup_complete"] == true
                && observed["runtime"]["runs"] == 1
                && observed["runtime"]["admitted_tasks"] == rows.len()
                && observed["runtime"]["completed_tasks"] == rows.len()
                && observed["runtime"]["failed_tasks"] == failures,
            "effect HTTP task counts or cleanup differ",
        )?;
    }
    let initial = receipt.rounds[0]["observations"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("initial effect values absent"))?;
    require(initial.len() == 25, "effect traversal cases missing")?;
    for (index, count) in [0_i64, 1, 31, 32, 33, 4097, 8192].into_iter().enumerate() {
        let values = (1..=count).map(job).collect::<Vec<_>>();
        let expected = (1..=count).map(|n| n * 3 + 12).collect::<Vec<_>>();
        for (offset, mode, output) in [
            (0, "map", json!(expected)),
            (1, "fold", json!(expected.iter().sum::<i64>())),
        ] {
            let observed = &initial[index * 2 + offset];
            require(
                observed["request"] == json!({"mode":mode,"prefix":7,"values":values})
                    && observed["result"] == output
                    && observed["status"] == 200
                    && observed["failure"].is_null(),
                "complete deterministic effect map/fold result differs",
            )?;
        }
    }
    for (index, mode, result) in [
        (14, "map", json!([21, 27, 15, 27])),
        (15, "transaction", json!([13, 11, 14, 11, 15])),
        (16, "read", json!(15)),
        (18, "read", json!(15)),
        (20, "read", json!(15)),
        (22, "read", json!(109)),
    ] {
        require(
            initial[index]["request"]["mode"] == mode
                && initial[index]["result"] == result
                && initial[index]["status"] == 200,
            "recursive values, ordered writes, or transaction visibility differ",
        )?;
    }
    for (index, mode, code) in [
        (17, "transaction", "normalized_integer_division"),
        (19, "nested", "normalized_transaction_nested"),
        (21, "write", "normalized_integer_division"),
    ] {
        require(
            initial[index]["request"]["mode"] == mode
                && initial[index]["status"] == 500
                && initial[index]["failure"] == code,
            "effect failure, nested transaction, or partial visibility case differs",
        )?;
    }
    require(
        initial[23]["request"]["mode"] == "results"
            && initial[23]["status"] == 200
            && initial[23]["result"]
                == json!([{"case":"ok","value":21},{"case":"error","value":15},{"case":"ok","value":24},{"case":"error","value":15},{"case":"ok","value":27}]),
        "ordinary returned error cases stopped task traversal",
    )?;
    require(
        initial[24]["request"]["mode"] == "nested-map"
            && initial[24]["status"] == 200
            && initial[24]["result"] == json!([21, 15, 24]),
        "nested task maps changed callback values or order",
    )?;
    require(
        receipt.rounds[1]["observations"][0]["result"] == 109
            && receipt.rounds[1]["observations"]
                .as_array()
                .is_some_and(|r| r.len() == 1),
        "committed callback data did not survive restart",
    )?;
    let successor = &receipt.rounds[2]["observations"];
    require(
        successor.as_array().is_some_and(|r| r.len() == 4)
            && successor[0]["result"] == json!([21, 21, 15, 15, 24, 24, 15, 15, 27, 27])
            && successor[1]["result"] == 102
            && successor[2]["result"] == 109
            && successor[3]["request"]["mode"] == "iterate"
            && successor[3]["result"] == json!({"position":4,"total":6})
            && successor[3]["status"] == 200,
        "exact dependency replacement result differs",
    )?;
    require(
        receipt.rounds[3]["observations"][0]["result"] == json!([21, 15, 24, 15, 27])
            && receipt.rounds[3]["observations"]
                .as_array()
                .is_some_and(|rows| rows.len() == 2)
            && receipt.rounds[3]["observations"][1]["request"]["mode"] == "iterate"
            && receipt.rounds[3]["observations"][1]["result"] == json!({"position":3,"total":3})
            && receipt.rounds[3]["observations"][1]["status"] == 200,
        "self-consistent old bundle recovery differs",
    )?;
    require(
        receipt.artifact_bindings.len() == 2
            && receipt.artifact_bindings[0]["bundle"] != receipt.artifact_bindings[1]["bundle"]
            && receipt.stale_artifact_rejection == "contributor_artifact_source",
        "effect artifacts did not retain distinct exact source bindings",
    )?;
    let mut sources = Vec::new();
    for (index, (name, revision)) in [
        ("effect-original.lkja", &receipt.consumer_revision),
        ("effect-successor.lkja", &receipt.changed_consumer_revision),
    ]
    .into_iter()
    .enumerate()
    {
        let binding = &receipt.artifact_bindings[index];
        require(
            binding["revision"] == *revision
                && binding["package"] == receipt.consumer_package
                && binding["packages"] == 3,
            "effect output has a foreign root or dependency inventory",
        )?;
        let transport = binding["source_transport"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("effect output source transport absent"))?;
        let position = parent
            .transport_digests
            .iter()
            .position(|p| p == transport)
            .ok_or_else(|| DevError::corrupt("effect output source transport missing"))?;
        let source = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", position + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let artifact = process::read_bounded(&root.join(name), MAXIMUM_CONTAINER_BYTES)?;
        let actual = lkjscript::platform::contributor::strict_artifact_source_probe(
            &artifact, &source, transport,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            actual == *binding,
            "effect output changed its independently admitted source binding",
        )?;
        sources.push((source, transport.to_owned()));
    }
    let old = process::read_bounded(&root.join("effect-original.lkja"), MAXIMUM_CONTAINER_BYTES)?;
    require(
        lkjscript::platform::contributor::strict_artifact_source_probe(
            &old,
            &sources[1].0,
            &sources[1].1,
        )
        .is_err_and(|error| error.code == "contributor_artifact_source"),
        "old output was accepted for a new effect source",
    )?;
    super::effects_resources::validate(parent, root)?;
    super::effects_iteration_evidence::validate(parent, root)?;
    super::foreground::validate(parent, root)?;
    Ok(())
}

pub(super) fn prepare_library(
    context: &mut Context,
    standard: &mut Package,
) -> Result<(Package, BTreeMap<String, String>), DevError> {
    discover(context, standard)?;
    // A previous offline workload may have removed its source copy; exporting is acquisition only.
    if !standard.container.exists() {
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
    }
    let mut library = context.new_package("effect-library")?;
    context.stage(&library, standard)?;
    context.apply(
        &mut library,
        &format!(
            "{}{}{}",
            binding("add", standard),
            module(),
            super::effects_program::library(&standard.symbols)
        ),
    )?;
    context.cli(Some(&library.path), &["check"], true)?;
    context.export(&mut library)?;
    for (kind, symbol) in [
        ("pure_function", "$configure-task"),
        ("task_function", "$task-map"),
        ("task_function", "$task-fold-left"),
        ("task_function", "$task-iterate"),
        ("pure_function", "$make-iteration"),
    ] {
        let observed = definition(context, &library, kind, symbol)?;
        context.receipt.effects.definitions.push(observed);
    }
    let names = [
        "task-fold-left",
        "task-map",
        "configure-task",
        "payload",
        "payload-value",
        "job",
        "job-item",
        "job-children",
        "task-iterate",
        "iteration-step",
        "iteration-continue",
        "iteration-done",
        "make-iteration",
        "iteration-state",
        "iteration-state-cursor",
        "iteration-state-sum",
        "iteration-output",
        "iteration-output-position",
        "iteration-output-total",
    ]
    .into_iter()
    .map(|name| Ok((name.to_owned(), reference(&library, &format!("${name}"))?)))
    .collect::<Result<BTreeMap<_, _>, DevError>>()?;
    Ok((library, names))
}

pub(super) fn workflow(context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    let (mut library, names) = prepare_library(context, standard)?;
    let mut foreground = super::foreground::prepare(context, standard, &library, &names)?;
    let path = context.root.join("effect-consumer");
    let created = context.cli(
        None,
        &[
            "new",
            &path.display().to_string(),
            "--template",
            "http",
            "--name",
            "effect-consumer",
        ],
        true,
    )?;
    let mut consumer = Package {
        path,
        id: field(&created, "package", "id")?,
        revision: field(&created, "revision", "id")?,
        logical: String::new(),
        transport: String::new(),
        container: PathBuf::new(),
        symbols: BTreeMap::new(),
    };
    context.stage(&consumer, &library)?;
    let module = context.cli(
        Some(&consumer.path),
        &["query", "find", "module", "application"],
        true,
    )?;
    let module = field(&module, "owner", "id")?;
    let function = context.cli(
        Some(&consumer.path),
        &[
            "query",
            "find",
            "declaration",
            "handle",
            "--parent",
            &module,
        ],
        true,
    )?;
    let function = field(&function, "owner", "id")?;
    let component = context.cli(
        Some(&consumer.path),
        &[
            "query",
            "find",
            "declaration",
            "application",
            "--parent",
            &module,
        ],
        true,
    )?;
    let component = field(&component, "owner", "id")?;
    let port = context.cli(
        Some(&consumer.path),
        &["query", "find", "port", "http", "--parent", &component],
        true,
    )?;
    let port = field(&port, "owner", "id")?;
    let definition = context.cli(
        Some(&consumer.path),
        &[
            "inspect",
            "owner",
            "task_function",
            &function,
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    let streams = definition
        .iter()
        .find(|r| {
            r.operation == "definition.reference"
                && record_field(r, "role").is_ok_and(|role| role == "function_requirement")
        })
        .ok_or_else(|| DevError::corrupt("HTTP streams requirement absent"))?;
    let bindings = BTreeMap::from([
        ("module".into(), module),
        ("function".into(), function),
        ("component".into(), component),
        ("port".into(), port),
        ("streams".into(), record_field(streams, "target")?),
        (
            "parameter".into(),
            field(&definition, "definition.parameter", "id")?,
        ),
        (
            "request-type".into(),
            field(&definition, "definition.parameter", "type")?,
        ),
    ]);
    context.apply(
        &mut consumer,
        &format!(
            "{}{}",
            binding("add", &library),
            super::effects_consumer::program(&standard.symbols, &bindings, &names)
        ),
    )?;
    let observed = interface(context, &consumer, &library)?;
    context.receipt.effects.definitions.push(observed);
    reject_bad_effect_requests(context, &consumer, &bindings, &names)?;
    reject_iteration_requests(context, &consumer, &bindings, &names)?;
    let library_recovery = context.root.join("effect-library-recovery");
    fs::rename(&library.path, &library_recovery)?;
    super::foreground::initial(context, &foreground)?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.export(&mut consumer)?;
    let standalone = context.root.join("effect-standalone");
    fs::create_dir(&standalone)?;
    context.cli(
        Some(&consumer.path),
        &[
            "build",
            "--output",
            &standalone.join("application.lkja").display().to_string(),
        ],
        true,
    )?;
    fs::copy(
        standalone.join("application.lkja"),
        context.evidence.join("effect-original.lkja"),
    )?;
    let original_bytes = process::read_bounded(
        &standalone.join("application.lkja"),
        MAXIMUM_CONTAINER_BYTES,
    )?;
    let original_source = process::read_bounded(&consumer.container, MAXIMUM_CONTAINER_BYTES)?;
    let original_transport = consumer.transport.clone();
    let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
        &original_bytes,
        &original_source,
        &original_transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    context.receipt.effects.artifact_bindings.push(bound);
    let mut descriptor: Value = serde_json::from_slice(&process::read_bounded(
        &consumer.path.join("service.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    descriptor["artifact"] = json!("application.lkja");
    descriptor["listen"] = json!("127.0.0.1:0");
    descriptor["http"]["maximum_request_body_bytes"] = json!(1_048_576);
    descriptor["configuration"] = json!({"scale":{"kind":"i64","value":3},"bias":{"kind":"i64","value":5},"stride":{"kind":"i64","value":1},"threshold":{"kind":"i64","value":3},"tick":{"kind":"i64","value":0}});
    let grants = descriptor["grants"]
        .as_array_mut()
        .ok_or_else(|| DevError::corrupt("HTTP grants missing"))?;
    grants.push(json!({"requirement":"config","sharing_domain":"effects-config","authority_revision":"66".repeat(32),"adapter":{"kind":"configuration"}}));
    grants.push(json!({"requirement":"data","sharing_domain":"effects-data","authority_revision":"77".repeat(32),"adapter":{"kind":"data","root":"data","namespace":"effects","limits":DataLimits{maximum_live_transactions:1,..Default::default()}}}));
    fs::write(
        standalone.join("service.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::write(
        context.evidence.join("effect.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &standalone.join("data").display().to_string(),
        ],
        true,
    )?;
    context.receipt.effects.library_package = library.id.clone();
    context.receipt.effects.library_revision = library.revision.clone();
    context.receipt.effects.consumer_package = consumer.id.clone();
    context.receipt.effects.consumer_revision = consumer.revision.clone();
    context.receipt.effects.producers_removed = !library.path.exists();
    super::effects_iteration_evidence::public(context, &standalone)?;
    resources(context, &consumer)?;
    let before_execution = crate::authority::observe_graph_authority(&consumer.path)?;
    let round = crate::stateful_http::recursive_round(
        &context.binary,
        &standalone,
        &context.evidence,
        "effects-initial",
        |address| {
            let mut observations = Vec::new();
            for count in [0, 1, 31, 32, 33, 4097, 8192] {
                let values = (1..=count).map(job).collect::<Vec<_>>();
                let expected = (1..=count).map(|n| 3 * n + 12).collect::<Vec<_>>();
                observations.push(request(
                    address,
                    "map",
                    7,
                    values.clone(),
                    Some(json!(expected)),
                )?);
                observations.push(request(
                    address,
                    "fold",
                    7,
                    values,
                    Some(json!(expected.iter().sum::<i64>())),
                )?);
            }
            let nested = vec![
                job(3),
                json!({"case":"children","value":[job(1),job(4)]}),
                job(1),
                job(5),
            ];
            observations.push(request(
                address,
                "map",
                7,
                nested,
                Some(json!([21, 27, 15, 27])),
            )?);
            let values = [3, 1, 4, 1, 5].into_iter().map(job).collect::<Vec<_>>();
            observations.push(request(
                address,
                "transaction",
                10,
                values,
                Some(json!([13, 11, 14, 11, 15])),
            )?);
            observations.push(request(address, "read", 0, vec![], Some(json!(15)))?);
            let fails = [9, -1, 8].into_iter().map(job).collect::<Vec<_>>();
            observations.push(request(address, "transaction", 100, fails.clone(), None)?);
            observations.push(request(address, "read", 0, vec![], Some(json!(15)))?);
            observations.push(request(address, "nested", 100, vec![job(9)], None)?);
            observations.push(request(address, "read", 0, vec![], Some(json!(15)))?);
            observations.push(request(address, "write", 100, fails, None)?);
            observations.push(request(address, "read", 0, vec![], Some(json!(109)))?);
            observations.push(request(address, "results", 7, [3,1,4,1,5].into_iter().map(job).collect(),
                Some(json!([{"case":"ok","value":21},{"case":"error","value":15},{"case":"ok","value":24},{"case":"error","value":15},{"case":"ok","value":27}])))?);
            observations.push(request(
                address,
                "nested-map",
                7,
                [3, 1, 4].into_iter().map(job).collect(),
                Some(json!([21, 15, 24])),
            )?);
            Ok(json!(observations))
        },
    )?;
    context.receipt.effects.rounds.push(round);
    transaction_cancellation(context, &standalone, &consumer.symbols["$transaction-jobs"])?;
    let restarted = crate::stateful_http::recursive_round(
        &context.binary,
        &standalone,
        &context.evidence,
        "effects-restarted",
        |address| {
            Ok(json!([request(
                address,
                "read",
                0,
                vec![],
                Some(json!(109))
            )?]))
        },
    )?;
    context.receipt.effects.rounds.push(restarted);
    require(
        crate::authority::observe_graph_authority(&consumer.path)? == before_execution,
        "task execution changed canonical authority",
    )?;
    context.receipt.effects.authority_unchanged = true;
    fs::rename(library_recovery, &library.path)?;

    let replacement = duplicate_map_output(&standard.symbols, &library);
    context.apply(&mut library, &replacement)?;
    context.cli(Some(&library.path), &["check"], true)?;
    context.export(&mut library)?;
    super::foreground::replace(context, &mut foreground, &library)?;
    context.stage(&consumer, &library)?;
    context.apply(&mut consumer, &binding("replace", &library))?;
    let second_recovery = context.root.join("effect-library-successor-recovery");
    fs::rename(&library.path, &second_recovery)?;
    super::foreground::recovery(context, &mut foreground)?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.export(&mut consumer)?;
    let successor = standalone.join("successor.lkja");
    context.cli(
        Some(&consumer.path),
        &["build", "--output", &successor.display().to_string()],
        true,
    )?;
    let source = process::read_bounded(&consumer.container, MAXIMUM_CONTAINER_BYTES)?;
    let successor_bytes = process::read_bounded(&successor, MAXIMUM_CONTAINER_BYTES)?;
    let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
        &successor_bytes,
        &source,
        &consumer.transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    context.receipt.effects.artifact_bindings.push(bound);
    let before_rejection = crate::authority::observe_graph_authority(&consumer.path)?;
    let rejected = lkjscript::platform::contributor::strict_artifact_source_probe(
        &original_bytes,
        &source,
        &consumer.transport,
    )
    .err()
    .ok_or_else(|| DevError::corrupt("old artifact certified a new source revision"))?;
    require(
        rejected.code == "contributor_artifact_source",
        "old artifact failed at an unrelated boundary",
    )?;
    require(
        crate::authority::observe_graph_authority(&consumer.path)? == before_rejection,
        "old artifact rejection changed authority",
    )?;
    context.receipt.effects.stale_artifact_rejection = rejected.code;
    context.receipt.effects.changed_library_revision = library.revision.clone();
    context.receipt.effects.changed_consumer_revision = consumer.revision.clone();
    fs::copy(&successor, context.evidence.join("effect-successor.lkja"))?;
    fs::copy(&successor, standalone.join("application.lkja"))?;
    let round = crate::stateful_http::recursive_round(
        &context.binary,
        &standalone,
        &context.evidence,
        "effects-successor",
        |address| {
            Ok(json!([
                request(
                    address,
                    "map",
                    7,
                    [3, 1, 4, 1, 5].into_iter().map(job).collect(),
                    Some(json!([21, 21, 15, 15, 24, 24, 15, 15, 27, 27]))
                )?,
                request(
                    address,
                    "fold",
                    7,
                    [3, 1, 4, 1, 5].into_iter().map(job).collect(),
                    Some(json!(102))
                )?,
                request(address, "read", 0, vec![], Some(json!(109)))?,
                request(
                    address,
                    "iterate",
                    0,
                    vec![],
                    Some(json!({"position":4,"total":6}))
                )?
            ]))
        },
    )?;
    context.receipt.effects.rounds.push(round);
    fs::write(standalone.join("original.lkja"), &original_bytes)?;
    let current_descriptor = process::read_bounded(
        &standalone.join("service.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?;
    descriptor["artifact"] = json!("original.lkja");
    fs::write(
        standalone.join("service.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    let rebound = lkjscript::platform::contributor::strict_artifact_source_probe(
        &original_bytes,
        &original_source,
        &original_transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        rebound == context.receipt.effects.artifact_bindings[0],
        "old supported bundle changed during recovery",
    )?;
    let round = crate::stateful_http::recursive_round(
        &context.binary,
        &standalone,
        &context.evidence,
        "effects-old-supported",
        |address| {
            Ok(json!([
                request(
                    address,
                    "map",
                    7,
                    [3, 1, 4, 1, 5].into_iter().map(job).collect(),
                    Some(json!([21, 15, 24, 15, 27]))
                )?,
                request(
                    address,
                    "iterate",
                    0,
                    vec![],
                    Some(json!({"position":3,"total":3}))
                )?
            ]))
        },
    );
    fs::write(
        standalone.join("service.deployment.json"),
        current_descriptor,
    )?;
    context.receipt.effects.rounds.push(round?);
    require(
        crate::authority::observe_graph_authority(&consumer.path)? == before_rejection,
        "successor or recovery execution changed authority",
    )?;
    Ok(())
}
