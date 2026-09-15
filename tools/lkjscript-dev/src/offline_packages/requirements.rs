//! Literal public requirement-parametric library with independent ordered-store observations.
use super::*;
#[path = "requirements_conflict.rs"]
mod conflict;
#[path = "requirements_structural.rs"]
mod structural;
#[path = "requirements_transaction_predecessor.rs"]
mod transaction_predecessor;
use lkjscript::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use serde_json::{Value, json};

const CELL_KEYS: [&str; 9] = [
    "direct",
    "bound",
    "aux",
    "trap-aux",
    "multi",
    "success-aux",
    "early-aux",
    "late-aux",
    "independent",
];

pub(super) fn focused(context: &mut Context) -> Result<(), DevError> {
    let builtin = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id")?,
        revision: field(&builtin, "package", "revision")?,
        logical: field(&builtin, "package", "package-revision")?,
        transport: field(&builtin, "package", "transport")?,
        container: context.root.join("requirements-standard.lkjp"),
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

fn selection(package: &Package) -> String {
    format!(
        "reference.package as=$library package={} package-revision={}\n",
        package.id, package.logical
    )
}

fn reject_request(
    context: &mut Context,
    package: &Package,
    name: &str,
    body: &str,
) -> Result<(), DevError> {
    let request = context
        .evidence
        .join(format!("requirement-reject-{name}.lkjc"));
    fs::write(
        &request,
        format!(
            "request base={} idempotency=reject-{name}\n{body}",
            package.revision
        ),
    )?;
    let before = crate::authority::observe_graph_authority(&package.path)?;
    let index = context.receipt.commands.len();
    let output = context.cli(
        Some(&package.path),
        &[
            "change",
            "plan",
            "--input-file",
            &request.display().to_string(),
        ],
        false,
    )?;
    let code = field(&output, "diagnostic", "code")?;
    require(
        code.contains("requirement")
            || matches!(
                code.as_str(),
                "kernel_type_capability_operation" | "kernel_type_argument"
            ),
        &format!("requirement {name} rejected for unrelated reason: {code}"),
    )?;
    require(
        crate::authority::observe_graph_authority(&package.path)? == before,
        "rejected requirement edit changed accepted authority",
    )?;
    context.receipt.observations.insert(
        format!("requirement_reject_{name}"),
        json!({"command":index,"code":code,"authority":before.inventory_sha256}).to_string(),
    );
    Ok(())
}

fn deployment_value(target: &str, artifact: &str) -> Value {
    json!({
        "artifact":artifact,"target":target,"listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),
        "configuration":{"suffix":{"kind":"text","value":"!"}},"secrets":[],
        "grants":[
            {"requirement":"counters","sharing_domain":"requirement-numbers","authority_revision":"a1".repeat(32),"adapter":{"kind":"data","root":"numbers","namespace":"number-cells","limits":DataLimits::default()}},
            {"requirement":"settings-store","sharing_domain":"requirement-texts","authority_revision":"a2".repeat(32),"adapter":{"kind":"data","root":"texts","namespace":"text-cells","limits":DataLimits::default()}},
            {"requirement":"suffix-config","sharing_domain":"requirement-configuration","authority_revision":"a3".repeat(32),"adapter":{"kind":"configuration"}}
        ]
    })
}

fn deployment(bundle: &Path, target: &str, artifact: &str) -> Result<PathBuf, DevError> {
    let descriptor = bundle.join(format!("{artifact}-{target}.deployment.json"));
    fs::write(
        &descriptor,
        evidence::encode_json(&deployment_value(target, artifact))?,
    )?;
    Ok(descriptor)
}

fn preflight(context: &mut Context, bundle: &Path) -> Result<(), DevError> {
    let descriptor = deployment(bundle, "unencodable-cell", "consumer.lkja")?;
    let mut value: Value = serde_json::from_slice(&fs::read(&descriptor)?)?;
    value["secrets"] =
        json!([{"name":"unread","variable":"LKJSCRIPT_REQUIREMENT_UNAVAILABLE_SECRET"}]);
    fs::write(&descriptor, evidence::encode_json(&value)?)?;
    fs::copy(
        &descriptor,
        context
            .evidence
            .join("requirement-unencodable.deployment.json"),
    )?;
    let index = context.receipt.commands.len();
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
        field(&output, "diagnostic", "code")? == "normalized_json_type"
            && !bundle.join("numbers").exists()
            && !bundle.join("texts").exists(),
        "unencodable closed update result reached secrets or operational state",
    )?;
    context.receipt.observations.insert(
        "requirement_unencodable".into(),
        json!({"command":index,"stores_absent":true,"before_secret":true}).to_string(),
    );
    Ok(())
}

// Independently implement only the two specified scalar wire examples, without evaluator,
// compiler, graph types, or production typed-data codec. The retained full bytes are the oracle.
fn typed_bytes(value: &Value) -> Result<Vec<u8>, DevError> {
    fn hash(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
        let mut h = blake3::Hasher::new_derive_key(domain);
        h.update(&(bytes.len() as u64).to_be_bytes());
        h.update(bytes);
        *h.finalize().as_bytes()
    }
    let (layout, type_identity, payload) = if let Some(number) = value.as_i64() {
        (
            2,
            "4872f33f8c53c3dbf43ad114824e6c18f4537f45cb609d7d2a82c1d479e23062",
            number.to_be_bytes().to_vec(),
        )
    } else if let Some(text) = value.as_str() {
        let mut payload = u32::try_from(text.len())
            .map_err(|_| DevError::corrupt("cell oracle text length"))?
            .to_be_bytes()
            .to_vec();
        payload.extend_from_slice(text.as_bytes());
        (
            4,
            "d8907b140f33643f5e02a15377e67645703815b56e0440c1ccd68f838e2829e3",
            payload,
        )
    } else {
        return Err(DevError::corrupt(
            "independent cell oracle accepts I64 or Text only",
        ));
    };
    let mut bytes = b"LKJDVAL1".to_vec();
    bytes.extend_from_slice(&1u16.to_be_bytes());
    // Frozen TypeObject 10 scalar identities are independently retained predecessor inputs.
    let mut description = (0..32)
        .map(|index| {
            u8::from_str_radix(&type_identity[index * 2..index * 2 + 2], 16)
                .map_err(|_| DevError::corrupt("fixed scalar identity"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    description.push(layout);
    bytes.extend_from_slice(&hash("lkjscript.data.typed-layout.v1", &description));
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&hash("lkjscript.data.typed-value-envelope.v1", &bytes));
    Ok(bytes)
}

fn observe(bundle: &Path) -> Result<Value, DevError> {
    let mut observations = Vec::new();
    for (directory, namespace) in [("numbers", "number-cells"), ("texts", "text-cells")] {
        let store = DataStore::open(&bundle.join(directory), namespace, DataLimits::default())
            .map_err(|e| DevError::corrupt(e.to_string()))?;
        let transaction = store
            .begin()
            .map_err(|e| DevError::corrupt(e.to_string()))?;
        let mut cells = BTreeMap::new();
        for key in CELL_KEYS {
            let key_value = DataKey::new(vec![DataKeyPart::Text(key.to_owned())], store.limits())
                .map_err(|e| DevError::corrupt(e.to_string()))?;
            let entry = transaction
                .get("typed-cell", &key_value)
                .map_err(|e| DevError::corrupt(e.to_string()))?;
            cells.insert(
                key,
                entry.map(|entry| json!({"bytes":entry.value,"revision":entry.revision})),
            );
        }
        observations.push(json!({"store":directory,"namespace":namespace,"revision":transaction.base_revision(),"cells":cells}));
        // Read-only snapshot; dropping it releases the observer's transaction. Never commit.
    }
    Ok(json!(observations))
}

fn expect_cell(
    observation: &Value,
    store: usize,
    key: &str,
    value: Option<Value>,
) -> Result<(), DevError> {
    let observed = observation[store]["cells"]
        .get(key)
        .ok_or_else(|| DevError::corrupt("independent cell observation omitted"))?;
    if let Some(value) = value {
        require(
            observed["bytes"] == json!(typed_bytes(&value)?),
            &format!("independent typed bytes disagree for store {store} key {key}: {observed}"),
        )
    } else {
        require(
            observed.is_null(),
            &format!("unexpected publication at store {store} key {key}"),
        )
    }
}

fn revision_identity(value: &Value) -> bool {
    value
        .as_str()
        .and_then(|text| text.strip_prefix("data_revision_"))
        .is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn byte_array(value: &Value, length: Option<usize>) -> bool {
    value.as_array().is_some_and(|bytes| {
        length.is_none_or(|length| bytes.len() == length)
            && bytes
                .iter()
                .all(|byte| byte.as_u64().is_some_and(|byte| byte <= 255))
    })
}

fn retained_cell(value: &Value) -> bool {
    value.is_null()
        || (value.as_object().is_some_and(|fields| fields.len() == 2)
            && byte_array(&value["bytes"], None)
            && byte_array(&value["revision"], Some(32)))
}

fn store_observations(rows: &[Value]) -> Result<(), DevError> {
    for row in rows {
        require(
            row.as_array().is_some_and(|stores| stores.len() == 2),
            "independent store pair omitted",
        )?;
        for (index, (store, namespace)) in [("numbers", "number-cells"), ("texts", "text-cells")]
            .into_iter()
            .enumerate()
        {
            let value = &row[index];
            require(
                value["store"] == store
                    && value["namespace"] == namespace
                    && revision_identity(&value["revision"])
                    && value["cells"].as_object().is_some_and(|cells| {
                        cells.len() == CELL_KEYS.len()
                            && CELL_KEYS
                                .iter()
                                .all(|key| cells.get(*key).is_some_and(retained_cell))
                    }),
                "independent store revision, complete cells, or entry bytes omitted",
            )?;
        }
    }
    Ok(())
}

fn completed_states(rows: &[Value]) -> Result<(), DevError> {
    require(
        rows.len() == 23,
        "completed transaction state evidence omitted",
    )?;
    expect_cell(&rows[14], 0, "multi", Some(json!(13)))?;
    expect_cell(&rows[14], 0, "success-aux", Some(json!(7)))?;
    require(
        rows[13][1] == rows[14][1],
        "multi-key update changed another store",
    )?;
    for row in &rows[15..=18] {
        require(
            row == &rows[14],
            "failed or no-write completion changed data or HEAD",
        )?;
    }
    expect_cell(&rows[18], 0, "early-aux", None)?;
    expect_cell(&rows[18], 0, "late-aux", None)?;
    expect_cell(&rows[18], 0, "direct", Some(json!(22)))?;
    require(
        rows[18][0] == rows[19][0],
        "outer condition failure published its store",
    )?;
    expect_cell(&rows[18], 1, "independent", None)?;
    expect_cell(&rows[19], 1, "independent", Some(json!("survived")))?;
    require(
        rows[18][1]["revision"] != rows[19][1]["revision"],
        "independent inner store completion did not advance its own HEAD",
    )?;
    for row in &rows[20..=22] {
        require(
            row == &rows[19],
            "empty, nominal or read-only completion changed data or HEAD",
        )?;
    }
    Ok(())
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    context.cli(None, &["capabilities", "--section", "change"], true)?;
    let mut producer = context.new_package("requirement-producer")?;
    context.stage(&producer, standard)?;
    let accepted = structural::apply(
        context,
        &mut producer,
        "producer",
        &binding("add", standard),
        &format!(
            "{}{}",
            include_str!("requirements.producer.lkjc"),
            include_str!("requirements.resource-library.lkjc")
        ),
        &format!(
            "{}{}",
            include_str!("requirements.producer.structural.lkjc"),
            include_str!("requirements.resource-library.structural.lkjc")
        ),
    )?;
    let original_request = context
        .receipt
        .commands
        .last()
        .and_then(|command| {
            command
                .command
                .windows(2)
                .find(|pair| pair[0] == "--input-file")
                .map(|pair| pair[1].clone())
        })
        .ok_or_else(|| DevError::corrupt("public requirement request path missing"))?;
    let before_retry = crate::authority::observe_graph_authority(&producer.path)?;
    let retried = context.cli(
        Some(&producer.path),
        &[
            "change",
            "apply",
            "--input-file",
            &original_request,
            "--plan",
            &field(&accepted, "plan", "token")?,
        ],
        true,
    )?;
    require(
        field(&accepted, "receipt", "digest")? == field(&retried, "receipt", "digest")?
            && field(&accepted, "receipt", "revision-record")?
                == field(&retried, "receipt", "revision-record")?
            && crate::authority::observe_graph_authority(&producer.path)? == before_retry,
        "accepted requirement retry changed the original receipt, revision record, or authority",
    )?;
    context.receipt.observations.insert(
        "requirement_retry".into(),
        field(&retried, "receipt", "digest")?,
    );
    context.cli(Some(&producer.path), &["check"], true)?;
    for (name, body) in [
        ("arity", include_str!("requirements.invalid-arity.lkjc")),
        ("scope", include_str!("requirements.invalid-scope.lkjc")),
        (
            "minimum-operation",
            include_str!("requirements.invalid-operation.lkjc"),
        ),
        (
            "forwarding",
            include_str!("requirements.invalid-forward.lkjc"),
        ),
    ] {
        reject_request(context, &producer, name, body)?;
    }
    context.cli(
        Some(&producer.path),
        &[
            "query",
            "owners",
            "--kind",
            "requirement_parameter",
            "--limit",
            "10",
        ],
        true,
    )?;
    let definition_before = context.receipt.commands.len();
    let original_update = context.cli(
        Some(&producer.path),
        &[
            "inspect",
            "owner",
            "task_function",
            &producer.symbols["$update"],
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    let factory_before = context.receipt.commands.len();
    let original_factory = structural::inspect_factory(context, &producer)?;
    context.export(&mut producer)?;
    let mut consumer = context.new_package("requirement-consumer")?;
    context.stage(&consumer, standard)?;
    context.stage(&consumer, &producer)?;
    structural::apply(
        context,
        &mut consumer,
        "consumer",
        &format!(
            "{}{}{}",
            binding("add", standard),
            binding("add", &producer),
            selection(&producer),
        ),
        &format!(
            "{}{}{}{}",
            include_str!("requirements.consumer.lkjc"),
            include_str!("requirements.outcomes.consumer.flat.lkjc"),
            include_str!("requirements.resource-consumer.lkjc"),
            include_str!("requirements.unencodable.lkjc")
        ),
        &format!(
            "{}{}{}{}",
            include_str!("requirements.consumer.structural.lkjc"),
            include_str!("requirements.outcomes.consumer.structural.lkjc"),
            include_str!("requirements.resource-consumer.structural.lkjc"),
            include_str!("requirements.unencodable.lkjc")
        ),
    )?;
    reject_request(
        context,
        &consumer,
        "interface",
        &format!(
            "{}{}",
            selection(&producer),
            include_str!("requirements.invalid-interface.lkjc")
        ),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    reject_request(
        context,
        &consumer,
        "callback-row",
        &format!(
            "{}{}",
            selection(&producer),
            include_str!("requirements.invalid-callback.lkjc")
        ),
    )?;
    let bundle = context.root.join("requirement-bundle");
    fs::create_dir(&bundle)?;
    context.cli(
        Some(&consumer.path),
        &[
            "build",
            "--output",
            &bundle.join("consumer.lkja").display().to_string(),
        ],
        true,
    )?;
    fs::copy(
        bundle.join("consumer.lkja"),
        context.evidence.join("requirement-consumer.lkja"),
    )?;
    let original = producer.logical.clone();
    structural::apply(
        context,
        &mut producer,
        "supplier",
        "",
        &format!(
            "{}{}",
            include_str!("requirements.stronger.lkjc"),
            include_str!("requirements.update-body.flat.lkjc")
        ),
        &format!(
            "{}{}",
            include_str!("requirements.stronger.structural.lkjc"),
            include_str!("requirements.update-body.structural.lkjc")
        ),
    )?;
    let factory_after = context.receipt.commands.len();
    let edited_factory = structural::inspect_factory(context, &producer)?;
    context.export(&mut producer)?;
    let definition_after = context.receipt.commands.len();
    let edited_update = context.cli(
        Some(&producer.path),
        &[
            "inspect",
            "owner",
            "task_function",
            &producer.symbols["$update"],
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    let supplier_plan = parse_records(
        "requirement-supplier-review",
        &process::read_bounded(
            &context
                .evidence
                .join("requirement-supplier-structural.lkjplan"),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("structural supplier review records"))?;
    structural::validate_body_changes(
        [&original_factory, &original_update],
        [&edited_factory, &edited_update],
        &supplier_plan,
    )?;
    let mut structural: Value =
        serde_json::from_str(&context.receipt.observations["requirement_structural"])?;
    structural["factory_before"] = json!(factory_before);
    structural["factory_after"] = json!(factory_after);
    structural["update_before"] = json!(definition_before);
    structural["update_after"] = json!(definition_after);
    context
        .receipt
        .observations
        .insert("requirement_structural".into(), structural.to_string());
    require(
        producer.logical != original,
        "changed minimum constraint retained an old package interface identity",
    )?;
    context.stage(&consumer, &producer)?;
    let rejected = context
        .evidence
        .join("requirement-insufficient-replacement.lkjc");
    fs::write(
        &rejected,
        format!(
            "request base={} idempotency=insufficient-replacement\n{}",
            consumer.revision,
            binding("replace", &producer)
        ),
    )?;
    let before = crate::authority::observe_graph_authority(&consumer.path)?;
    let replacement_rejection = context.receipt.commands.len();
    let result = context.cli(
        Some(&consumer.path),
        &[
            "change",
            "plan",
            "--input-file",
            &rejected.display().to_string(),
        ],
        false,
    )?;
    require(
        field(&result, "diagnostic", "code")?.contains("requirement"),
        "stronger constraint failed for an unrelated reason",
    )?;
    require(
        crate::authority::observe_graph_authority(&consumer.path)? == before,
        "insufficient dependency replacement changed accepted consumer meaning",
    )?;
    context.apply(
        &mut consumer,
        &format!(
            "{}{}",
            binding("replace", &producer),
            include_str!("requirements.repair.lkjc")
        ),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    context.cli(
        Some(&consumer.path),
        &[
            "build",
            "--output",
            &bundle.join("repaired.lkja").display().to_string(),
        ],
        true,
    )?;
    fs::copy(
        bundle.join("repaired.lkja"),
        context.evidence.join("requirement-repaired.lkja"),
    )?;
    context.receipt.observations.insert("requirement_packages".into(), json!({"producer":producer.id,"before":original,"after":producer.logical,"consumer":consumer.id,"repaired_revision":consumer.revision,"definition_before":definition_before,"definition_after":definition_after,"replacement_rejection":replacement_rejection,"unchanged_authority":before.inventory_sha256}).to_string());
    fs::remove_dir_all(&producer.path)?;
    let recovery = context.root.join("requirement-recovery");
    fs::rename(&consumer.path, &recovery)?;
    preflight(context, &bundle)?;
    for directory in ["numbers", "texts"] {
        context.cli(
            None,
            &[
                "data",
                "initialize",
                "--root",
                &bundle.join(directory).display().to_string(),
            ],
            true,
        )?;
    }
    let mut observations = vec![observe(&bundle)?];
    fs::write(
        context.evidence.join("requirement-store-observations.json"),
        evidence::encode_json(&observations)?,
    )?;
    let mut commands = Vec::new();
    for (artifact, target, arguments, expected) in cases() {
        let descriptor = deployment(&bundle, target, artifact)?;
        fs::copy(
            &descriptor,
            context
                .evidence
                .join(format!("requirement-{artifact}-{target}.deployment.json")),
        )?;
        commands.push(context.receipt.commands.len());
        let result = context.cli(
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
            let actual: Value = serde_json::from_str(&field(&result, "execution", "value")?)?;
            require(
                actual == expected,
                &format!("requirement {target}: actual {actual}, expected {expected}"),
            )?;
        } else {
            require(
                field(&result, "diagnostic", "code")?.contains("division"),
                "trap lost its original division failure",
            )?;
        }
        observations.push(observe(&bundle)?);
        fs::write(
            context.evidence.join("requirement-store-observations.json"),
            evidence::encode_json(&observations)?,
        )?;
    }
    for (index, store, key, value) in [
        (1, 0, "direct", json!(13)),
        (2, 0, "direct", json!(16)),
        (3, 0, "bound", json!(13)),
        (4, 0, "bound", json!(16)),
        (5, 1, "direct", json!("a!")),
        (6, 1, "direct", json!("a!!")),
        (7, 1, "bound", json!("a!")),
        (8, 1, "bound", json!("a!!")),
        (9, 0, "aux", json!(41)),
    ] {
        expect_cell(&observations[index], store, key, Some(value))?;
    }
    require(
        observations[9] == observations[10] && observations[10] == observations[11],
        "false expectation or trap published some of the lexical transaction",
    )?;
    expect_cell(&observations[11], 0, "direct", Some(json!(16)))?;
    expect_cell(&observations[11], 0, "trap-aux", None)?;
    expect_cell(&observations[11], 1, "aux", None)?;
    expect_cell(&observations[12], 0, "direct", Some(json!(22)))?;
    expect_cell(&observations[13], 1, "bound", Some(json!("a!!!!")))?;
    completed_states(&observations)?;
    // Reopening the owned recovery copy is a normal public recovery check, after execution no
    // longer has either authoring path. It must preserve the final accepted revision.
    context.cli(Some(&recovery), &["check"], true)?;
    fs::write(
        context.evidence.join("requirement-store-observations.json"),
        evidence::encode_json(&observations)?,
    )?;
    context.receipt.observations.insert(
        "requirement_commands".into(),
        serde_json::to_string(&commands)?,
    );
    context
        .receipt
        .observations
        .insert("requirement_sources_removed".into(), "true".into());
    super::requirements_predecessor::workflow(context)?;
    transaction_predecessor::workflow(context)?;
    conflict::workflow(context, &bundle)?;
    super::requirements_queue::workflow(context, &bundle)?;
    Ok(())
}

fn cases() -> [(&'static str, &'static str, &'static str, Option<Value>); 22] {
    [
        (
            "consumer.lkja",
            "number-direct",
            "[\"direct\"]",
            Some(json!({"case":"Committed","value":13})),
        ),
        (
            "consumer.lkja",
            "number-direct",
            "[\"direct\"]",
            Some(json!({"case":"Committed","value":16})),
        ),
        (
            "consumer.lkja",
            "number-bound",
            "[\"bound\"]",
            Some(json!({"case":"Committed","value":13})),
        ),
        (
            "consumer.lkja",
            "number-bound",
            "[\"bound\"]",
            Some(json!({"case":"Committed","value":16})),
        ),
        (
            "consumer.lkja",
            "text-direct",
            "[\"direct\"]",
            Some(json!({"case":"Committed","value":"a!"})),
        ),
        (
            "consumer.lkja",
            "text-direct",
            "[\"direct\"]",
            Some(json!({"case":"Committed","value":"a!!"})),
        ),
        (
            "consumer.lkja",
            "text-bound",
            "[\"bound\"]",
            Some(json!({"case":"Committed","value":"a!"})),
        ),
        (
            "consumer.lkja",
            "text-bound",
            "[\"bound\"]",
            Some(json!({"case":"Committed","value":"a!!"})),
        ),
        ("consumer.lkja", "seed-auxiliary", "[]", Some(json!(true))),
        (
            "consumer.lkja",
            "false-attempt",
            "[\"direct\"]",
            Some(json!({"case":"Aborted","value":{"case":"ConditionFailed"}})),
        ),
        ("consumer.lkja", "trapped-attempt", "[\"direct\"]", None),
        (
            "repaired.lkja",
            "number-direct",
            "[\"direct\"]",
            Some(json!({"case":"Committed","value":22})),
        ),
        (
            "repaired.lkja",
            "text-bound",
            "[\"bound\"]",
            Some(json!({"case":"Committed","value":"a!!!!"})),
        ),
        (
            "consumer.lkja",
            "multi-key-update",
            "[\"multi\"]",
            Some(json!({"case":"Committed","value":13})),
        ),
        (
            "consumer.lkja",
            "staged-false-attempt",
            "[\"direct\"]",
            Some(json!({"case":"Aborted","value":{"case":"ConditionFailed"}})),
        ),
        (
            "consumer.lkja",
            "no-write-false",
            "[]",
            Some(json!({"case":"Committed","value":false})),
        ),
        (
            "consumer.lkja",
            "condition-only",
            "[]",
            Some(json!({"case":"Aborted","value":{"case":"ConditionFailed"}})),
        ),
        (
            "consumer.lkja",
            "match-completion",
            "[\"direct\"]",
            Some(json!("condition-failed")),
        ),
        (
            "consumer.lkja",
            "other-store-survives",
            "[\"direct\"]",
            Some(json!({"case":"Aborted","value":{"case":"ConditionFailed"}})),
        ),
        (
            "consumer.lkja",
            "application-payload",
            "[]",
            Some(json!({"case":"Committed","value":{"amount":42,"note":"snapshot"}})),
        ),
        (
            "consumer.lkja",
            "empty-outcome",
            "[]",
            Some(json!({"case":"Committed","value":null})),
        ),
        (
            "consumer.lkja",
            "read-only-outcome",
            "[]",
            Some(json!({"case":"Committed","value":22})),
        ),
    ]
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    structural::validate(receipt, root)?;
    let mut expected_preflight = deployment_value("unencodable-cell", "consumer.lkja");
    expected_preflight["secrets"] =
        json!([{"name":"unread","variable":"LKJSCRIPT_REQUIREMENT_UNAVAILABLE_SECRET"}]);
    require(
        serde_json::from_slice::<Value>(&process::read_bounded(
            &root.join("requirement-unencodable.deployment.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)? == expected_preflight,
        "unencodable outcome preflight descriptor changed exact target or grants",
    )?;
    let preflight: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_unencodable")
            .ok_or_else(|| DevError::corrupt("requirement unencodable preflight missing"))?,
    )?;
    let preflight_index = preflight["command"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| DevError::corrupt("requirement preflight command index"))?;
    let preflight_command = receipt
        .commands
        .get(preflight_index)
        .ok_or_else(|| DevError::corrupt("requirement preflight command missing"))?;
    let preflight_output = parse_records(
        "requirement-preflight",
        &process::read_bounded(
            &root.join(format!("command-{preflight_index:04}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("requirement preflight output"))?;
    let preflight_descriptor = Path::new(&receipt.isolated_root)
        .join("requirement-bundle/consumer.lkja-unencodable-cell.deployment.json");
    require(
        preflight_command.command
            == [
                receipt.pinned_runtime_path.clone(),
                "run".into(),
                "--deployment".into(),
                preflight_descriptor.display().to_string(),
                "--arguments".into(),
                "[]".into(),
            ]
            && !preflight_command.expects_success
            && preflight["stores_absent"] == true
            && preflight["before_secret"] == true
            && field(&preflight_output, "diagnostic", "code")? == "normalized_json_type",
        "requirement closed output preflight differs",
    )?;
    let commands: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("requirement_commands")
            .ok_or_else(|| DevError::corrupt("requirement command evidence missing"))?,
    )?;
    require(
        commands.len() == cases().len() && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "requirement invocations omitted, duplicated, or reordered",
    )?;
    require(
        receipt
            .observations
            .get("requirement_sources_removed")
            .is_some_and(|v| v == "true")
            && receipt
                .observations
                .get("requirement_retry")
                .is_some_and(|v| v.starts_with("receipt_object_")),
        "requirement source removal or accepted retry missing",
    )?;
    let packages: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_packages")
            .ok_or_else(|| DevError::corrupt("requirement maintenance evidence missing"))?,
    )?;
    require(
        packages["before"] != packages["after"] && packages["producer"] != packages["consumer"],
        "requirement supplier transition or independent consumer identity missing",
    )?;
    for (name, minimum) in [("definition_before", "3"), ("definition_after", "4")] {
        let index = packages[name]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| DevError::corrupt("requirement constraint inspection index missing"))?;
        let command = receipt
            .commands
            .get(index)
            .ok_or_else(|| DevError::corrupt("requirement inspection command missing"))?;
        require(
            command.expects_success
                && command
                    .command
                    .windows(3)
                    .any(|w| w == ["inspect", "owner", "task_function"]),
            "requirement constraint inspection omitted",
        )?;
        let output = parse_records(
            "requirement-constraint",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("requirement constraint inspection output"))?;
        require(
            field(
                &output,
                "definition.requirement-parameter",
                "minimum-operations",
            )? == minimum,
            "inspected minimum constraint did not change from three operations to four",
        )?;
    }
    let index = packages["replacement_rejection"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| DevError::corrupt("requirement replacement rejection missing"))?;
    let command = receipt
        .commands
        .get(index)
        .ok_or_else(|| DevError::corrupt("requirement replacement command missing"))?;
    let output = parse_records(
        "requirement-replacement",
        &process::read_bounded(
            &root.join(format!("command-{index:04}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("requirement replacement output"))?;
    require(
        !command.expects_success
            && packages["unchanged_authority"]
                .as_str()
                .is_some_and(|s| s.len() == 64)
            && field(&output, "diagnostic", "code")?.contains("requirement"),
        "insufficient replacement or unchanged authority missing",
    )?;
    for name in [
        "arity",
        "scope",
        "minimum-operation",
        "forwarding",
        "interface",
        "callback-row",
    ] {
        let rejection: Value = serde_json::from_str(
            receipt
                .observations
                .get(&format!("requirement_reject_{name}"))
                .ok_or_else(|| DevError::corrupt("requirement semantic rejection missing"))?,
        )?;
        let index = rejection["command"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| DevError::corrupt("requirement rejection index"))?;
        let command = receipt
            .commands
            .get(index)
            .ok_or_else(|| DevError::corrupt("requirement rejection command missing"))?;
        require(
            !command.expects_success
                && rejection["authority"]
                    .as_str()
                    .is_some_and(|s| s.len() == 64),
            "requirement rejection omitted unchanged authority",
        )?;
        let output = parse_records(
            "requirement-rejection",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("requirement rejection output"))?;
        require(
            rejection["code"] == field(&output, "diagnostic", "code")?,
            "requirement rejection diagnostic differs",
        )?;
    }
    let observations: Value = serde_json::from_slice(&process::read_bounded(
        &root.join("requirement-store-observations.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    let rows = observations
        .as_array()
        .ok_or_else(|| DevError::corrupt("requirement store observations missing"))?;
    store_observations(rows)?;
    require(
        rows.len() == cases().len() + 1
            && rows
                .iter()
                .all(|row| row.as_array().is_some_and(|stores| stores.len() == 2)),
        "requirement independent store observations omitted",
    )?;
    for row in rows {
        require(
            row[0]["store"] == "numbers"
                && row[0]["namespace"] == "number-cells"
                && row[1]["store"] == "texts"
                && row[1]["namespace"] == "text-cells",
            "requirement store identity order differs",
        )?;
    }
    for store in [0, 1] {
        for key in CELL_KEYS {
            expect_cell(&rows[0], store, key, None)?;
        }
    }
    for (index, store, key, value) in [
        (1, 0, "direct", json!(13)),
        (2, 0, "direct", json!(16)),
        (3, 0, "bound", json!(13)),
        (4, 0, "bound", json!(16)),
        (5, 1, "direct", json!("a!")),
        (6, 1, "direct", json!("a!!")),
        (7, 1, "bound", json!("a!")),
        (8, 1, "bound", json!("a!!")),
        (9, 0, "aux", json!(41)),
        (12, 0, "direct", json!(22)),
        (13, 1, "bound", json!("a!!!!")),
    ] {
        expect_cell(&rows[index], store, key, Some(value))?;
    }
    require(
        rows[9] == rows[10]
            && rows[10] == rows[11]
            && rows[0][1] == rows[4][1]
            && rows[4][0] == rows[8][0],
        "requirement transaction rollback or distinct store observations differ",
    )?;
    expect_cell(&rows[11], 0, "trap-aux", None)?;
    expect_cell(&rows[11], 0, "direct", Some(json!(16)))?;
    completed_states(rows)?;
    let mut artifact_identities = BTreeMap::new();
    for (artifact, material) in [
        ("consumer.lkja", "requirement-consumer.lkja"),
        ("repaired.lkja", "requirement-repaired.lkja"),
    ] {
        let loaded = lkjscript::platform::contributor::strict_artifact_identity_probe(
            &process::read_bounded(&root.join(material), MAXIMUM_CONTAINER_BYTES)?,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        artifact_identities.insert(artifact, loaded);
    }
    for (index, (artifact, target, arguments, expected)) in commands.iter().zip(cases()) {
        require(
            serde_json::from_slice::<Value>(&process::read_bounded(
                &root.join(format!("requirement-{artifact}-{target}.deployment.json")),
                MAXIMUM_OUTPUT_BYTES,
            )?)? == deployment_value(target, artifact),
            "requirement descriptor changed exact target or grants",
        )?;
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("requirement invocation index missing"))?;
        let descriptor = Path::new(&receipt.isolated_root)
            .join("requirement-bundle")
            .join(format!("{artifact}-{target}.deployment.json"));
        require(
            command.command
                == [
                    receipt.pinned_runtime_path.clone(),
                    "run".into(),
                    "--deployment".into(),
                    descriptor.display().to_string(),
                    "--arguments".into(),
                    arguments.into(),
                ]
                && command.expects_success == expected.is_some(),
            "requirement invocation changed executable, target, arguments, or selected artifact",
        )?;
        let output = parse_records(
            "requirement-output",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("requirement invocation output"))?;
        if let Some(expected) = expected {
            let value: Value = serde_json::from_str(&field(&output, "execution", "value")?)?;
            let work: Value =
                serde_json::from_str(&field(&output, "execution", "production-observation")?)?;
            let cleanup: Value = serde_json::from_str(&field(&output, "execution", "cleanup")?)?;
            let expected_calls = match target {
                "false-attempt" | "match-completion" => 5,
                "seed-auxiliary" | "condition-only" | "read-only-outcome" => 2,
                "multi-key-update" => 4,
                "staged-false-attempt" | "other-store-survives" => 6,
                "no-write-false" | "application-payload" | "empty-outcome" => 1,
                name if name.starts_with("text-") => {
                    if artifact == "repaired.lkja" {
                        5
                    } else {
                        4
                    }
                }
                _ => 3,
            };
            require(
                value == expected
                    && field(&output, "execution", "artifact")? == artifact_identities[artifact]
                    && work["capability_calls"] == expected_calls
                    && cleanup["remaining_tasks"] == 0
                    && cleanup["cleanup_failures"] == json!([]),
                "requirement result, operation count, or cleanup differs",
            )?;
            for key in [
                "live_call_frames_after",
                "live_handles_after",
                "live_locals_after",
                "live_operands_after",
                "live_transactions_after",
                "live_type_bindings_after",
            ] {
                require(
                    work[key] == 0,
                    "requirement invocation retained owned execution state",
                )?;
            }
            if artifact == "repaired.lkja" {
                require(
                    packages["repaired_revision"] == field(&output, "execution", "revision")?,
                    "repaired requirement invocation used the predecessor source",
                )?;
            }
        } else {
            require(
                field(&output, "diagnostic", "code")? == "normalized_integer_division"
                    && field(&output, "diagnostic", "notes")?.contains("remaining-owned-tasks=0"),
                "requirement trap or joined cleanup missing",
            )?;
        }
    }
    let mut commands = commands;
    commands.push(preflight_index);
    commands.extend(super::requirements_predecessor::validate(receipt, root)?);
    commands.extend(transaction_predecessor::validate(receipt, root)?);
    commands.extend(conflict::validate(receipt, root)?);
    commands.extend(super::requirements_queue::validate(receipt, root)?);
    Ok(commands)
}
