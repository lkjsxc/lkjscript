//! Literal native libraries compose under one caller-owned transaction; the host only observes.
use super::*;

const MATERIAL: &str = "requirement-composition";
const CREDIT: &str = include_str!("requirements.composition.credit.lkjc");
const STOCK: &str = include_str!("requirements.composition.stock.lkjc");
const CONSUMER: &str = include_str!("requirements.composition.consumer.lkjc");
const CONTRACTS: [(&str, &str); 3] = [
    ("operation", "require-transaction"),
    ("task_function", "data-cell-update-in-transaction"),
    ("task_function", "data-cell-try-update"),
];

fn output(root: &Path, index: usize) -> Result<Vec<CompactRecord>, DevError> {
    parse_records(
        "composition-output",
        &process::read_bounded(
            &root.join(format!("command-{index:04}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("composition command output is malformed"))
}

fn apply_input(
    context: &mut Context,
    package: &mut Package,
    name: &str,
    source: &str,
) -> Result<Vec<usize>, DevError> {
    let input = context.evidence.join(format!("{MATERIAL}-{name}.lkjc"));
    let review = context.evidence.join(format!("{MATERIAL}-{name}.lkjplan"));
    fs::write(&input, source)?;
    let first = context.receipt.commands.len();
    let plan = context.cli(
        Some(&package.path),
        &[
            "change",
            "plan",
            "--input-file",
            &input.display().to_string(),
            "--output",
            &review.display().to_string(),
        ],
        true,
    )?;
    let token = field(&plan, "plan", "token")?;
    let decoded = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(&review)?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        decoded.token == token,
        "composition review/token binding differs",
    )?;
    let accepted = context.cli(
        Some(&package.path),
        &[
            "change",
            "apply",
            "--input-file",
            &input.display().to_string(),
            "--plan",
            &token,
        ],
        true,
    )?;
    package.revision = field(&accepted, "revision", "result")?;
    for record in accepted
        .iter()
        .filter(|record| record.operation == "identity")
    {
        package
            .symbols
            .insert(record_field(record, "symbol")?, record_field(record, "id")?);
    }
    Ok(vec![first, first + 1])
}

fn author(
    context: &mut Context,
    package: &mut Package,
    name: &str,
    prelude: &str,
    literal: &str,
) -> Result<Vec<usize>, DevError> {
    let source = format!(
        "request base={} idempotency={MATERIAL}-{name}\n{prelude}{literal}",
        package.revision
    );
    apply_input(context, package, name, &source)
}

fn descriptor(target: &str) -> Value {
    json!({
        "artifact":"consumer.lkja","target":target,"listen":null,"http":null,"session":null,"worker":null,
        "streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],
        "grants":[
            {"requirement":"shared","sharing_domain":"composition-shared","authority_revision":"b1".repeat(32),"adapter":{"kind":"data","root":"shared","namespace":"composition","limits":DataLimits::default()}},
            {"requirement":"other","sharing_domain":"composition-other","authority_revision":"b2".repeat(32),"adapter":{"kind":"data","root":"other","namespace":"composition","limits":DataLimits::default()}}
        ]
    })
}

fn invoke(
    context: &mut Context,
    bundle: &Path,
    target: &str,
    passes: bool,
) -> Result<usize, DevError> {
    let path = bundle.join(format!("{target}.deployment.json"));
    fs::write(&path, evidence::encode_json(&descriptor(target))?)?;
    fs::copy(
        &path,
        context.evidence.join(format!(
            "{}-{target}.deployment.json",
            bundle
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| DevError::corrupt("composition bundle name"))?
        )),
    )?;
    let index = context.receipt.commands.len();
    context.cli(
        None,
        &[
            "run",
            "--deployment",
            &path.display().to_string(),
            "--arguments",
            "[]",
        ],
        passes,
    )?;
    Ok(index)
}

fn state(bundle: &Path) -> Result<Value, DevError> {
    let mut stores = BTreeMap::new();
    for name in ["shared", "other"] {
        let root = bundle.join(name);
        let store = DataStore::open(&root, "composition", DataLimits::default())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let transaction = store
            .begin()
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let key = DataKey::new(vec![DataKeyPart::Text("account".into())], store.limits())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let mut entries = BTreeMap::new();
        for space in ["credit", "stock", "marker"] {
            let entry = transaction
                .get(space, &key)
                .map_err(|error| DevError::corrupt(error.to_string()))?;
            entries.insert(
                space,
                entry.map(|entry| json!({"bytes":entry.value,"revision":entry.revision})),
            );
        }
        stores.insert(name, json!({"head":fs::read(root.join("HEAD"))?,"revision":transaction.base_revision(),"verify":store.verify().map_err(|error| DevError::corrupt(error.to_string()))?,"entries":entries}));
    }
    Ok(json!(stores))
}

fn receipt_value(credit: i64, available: i64, reserved: i64) -> Value {
    json!({"credit":credit,"stock":{"available":available,"reserved":reserved}})
}

fn cases() -> Vec<(&'static str, &'static str, Option<Value>, Value)> {
    let initial = receipt_value(100, 8, 0);
    let abort = json!({"case":"Aborted","value":{"case":"ConditionFailed"}});
    vec![
        (
            "old",
            "compose",
            Some(json!({"case":"Committed","value":receipt_value(70, 6, 2)})),
            receipt_value(70, 6, 2),
        ),
        (
            "old",
            "repeated",
            Some(json!({"case":"Committed","value":60})),
            receipt_value(60, 8, 0),
        ),
        (
            "old",
            "failed-condition",
            Some(abort.clone()),
            initial.clone(),
        ),
        ("old", "trapped", None, initial.clone()),
        ("old", "independent", Some(abort), initial.clone()),
        ("old", "absent-owner", None, initial.clone()),
        ("old", "unrelated-owner", None, initial.clone()),
        ("old", "descriptor-after-owner", None, initial.clone()),
        (
            "old",
            "standalone",
            Some(json!({"case":"Committed","value":100})),
            initial.clone(),
        ),
        ("old", "nested-owner", None, initial),
        (
            "new",
            "compose",
            Some(json!({"case":"Committed","value":receipt_value(65, 6, 2)})),
            receipt_value(65, 6, 2),
        ),
    ]
}

fn expected_code(target: &str) -> &'static str {
    match target {
        "trapped" => "normalized_integer_division",
        "nested-owner" => "normalized_transaction_nested",
        _ => "normalized_data_transaction_required",
    }
}

// The nominal type layout is fixed by the independently authored initializer. The oracle
// preserves that exact 42-byte header, writes the two specified I64 payloads, and computes the
// documented envelope checksum; no helper result, typed codec, or graph interpreter is used.
fn stock_bytes(
    seed: &[u8],
    available: i64,
    reserved: i64,
    available_first: bool,
) -> Result<Vec<u8>, DevError> {
    require(
        seed.len() == 90 && &seed[..10] == b"LKJDVAL1\0\x01",
        "nominal stock envelope shape differs",
    )?;
    let mut bytes = seed[..42].to_vec();
    for value in if available_first {
        [available, reserved]
    } else {
        [reserved, available]
    } {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    let mut hash = blake3::Hasher::new_derive_key("lkjscript.data.typed-value-envelope.v1");
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(&bytes);
    bytes.extend_from_slice(hash.finalize().as_bytes());
    Ok(bytes)
}

fn states(
    before: &Value,
    after: &Value,
    expected: &Value,
    target: &str,
    available_first: bool,
) -> Result<(), DevError> {
    let seed: Vec<u8> =
        serde_json::from_value(before["shared"]["entries"]["stock"]["bytes"].clone())?;
    require(
        before["shared"]["entries"]["credit"]["bytes"] == json!(typed_bytes(&json!(100))?)
            && seed == stock_bytes(&seed, 8, 0, available_first)?,
        "composition initial values differ from fixed independent inputs",
    )?;
    require(
        after["shared"]["entries"]["credit"]["bytes"] == json!(typed_bytes(&expected["credit"])?)
            && after["shared"]["entries"]["stock"]["bytes"]
                == json!(stock_bytes(
                    &seed,
                    expected["stock"]["available"]
                        .as_i64()
                        .ok_or_else(|| DevError::corrupt("stock available expected"))?,
                    expected["stock"]["reserved"]
                        .as_i64()
                        .ok_or_else(|| DevError::corrupt("stock reserved expected"))?,
                    available_first
                )?),
        "composition durable bytes differ from independent fixed payloads",
    )?;
    let commits = matches!(target, "compose" | "repeated" | "standalone");
    if commits {
        require(
            before["shared"]["head"] != after["shared"]["head"]
                && after["shared"]["verify"]["revisions"].as_u64()
                    == before["shared"]["verify"]["revisions"]
                        .as_u64()
                        .and_then(|count| count.checked_add(1)),
            "composition must publish exactly one physical shared-store completion",
        )?;
    } else {
        require(
            before["shared"] == after["shared"],
            "composition failure changed shared store data or HEAD",
        )?;
    }
    if target == "independent" {
        require(
            after["other"]["entries"]["marker"]["bytes"] == json!(typed_bytes(&json!("survived"))?)
                && before["other"]["head"] != after["other"]["head"],
            "independent callback marker did not survive parent abort",
        )?;
    } else {
        require(
            before["other"] == after["other"],
            "composition callback unexpectedly changed independent store",
        )?;
    }
    Ok(())
}

fn stock_order(draft: &str, available: &str, reserved: &str) -> Result<bool, DevError> {
    let field = |identity: &str, name: &str| -> Result<usize, DevError> {
        let prefix = format!("(field edit {identity} {name}\n");
        require(
            draft.matches(&prefix).count() == 1,
            "nominal schema draft omitted its exact named field",
        )?;
        draft
            .find(&prefix)
            .ok_or_else(|| DevError::corrupt("nominal schema field absent"))
    };
    require(
        available != reserved,
        "nominal schema field identities are not distinct",
    )?;
    Ok(field(available, "available")? < field(reserved, "reserved")?)
}

fn edited_draft(source: &str) -> Result<String, DevError> {
    let add = source
        .lines()
        .find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("(reference"))
                .then(|| words.next())
                .flatten()
                .filter(|alias| alias.starts_with("ref_add_"))
        })
        .ok_or_else(|| DevError::corrupt("native credit draft omitted exact add reference"))?;
    let old = "(local value) (local delta)";
    require(
        source.matches(old).count() == 1,
        "native credit draft changed reviewed arithmetic boundary",
    )?;
    Ok(source.replacen(
        old,
        &format!("(local value) (call {add} (local delta) (i64 -5))"),
        1,
    ))
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    let discovery = context.receipt.commands.len();
    context.cli(None, &["capabilities", "--section", "change"], true)?;
    context.cli(None, &["package", "builtin", "inspect"], true)?;
    for (kind, name) in CONTRACTS {
        let found = context.cli(
            None,
            &[
                "package", "builtin", "query", "owners", "--kind", kind, "--name", name,
            ],
            true,
        )?;
        context.cli(
            None,
            &[
                "package",
                "builtin",
                "inspect",
                "owner",
                kind,
                &field(&found, "owner", "id")?,
            ],
            true,
        )?;
    }
    let mut authored = BTreeMap::new();
    let inventory_start = context.receipt.inventories.len();
    let mut credit = context.new_package("composition-credit")?;
    context.stage(&credit, standard)?;
    authored.insert(
        "credit",
        author(
            context,
            &mut credit,
            "credit",
            &binding("add", standard),
            CREDIT,
        )?,
    );
    context.cli(Some(&credit.path), &["check"], true)?;
    context.export(&mut credit)?;
    let old_credit = credit.clone();
    let mut stock = context.new_package("composition-stock")?;
    context.stage(&stock, standard)?;
    authored.insert(
        "stock",
        author(
            context,
            &mut stock,
            "stock",
            &binding("add", standard),
            STOCK,
        )?,
    );
    context.cli(Some(&stock.path), &["check"], true)?;
    context.export(&mut stock)?;
    let mut consumer = context.new_package("composition-consumer")?;
    for package in [standard, &credit, &stock] {
        context.stage(&consumer, package)?;
    }
    let prelude = format!(
        "{}{}{}reference.package as=$credit package={} package-revision={}\nreference.owner as=$credit-adjust package=$credit class=declaration name=adjust-credit\nreference.package as=$stock package={} package-revision={}\nreference.owner as=$stock-stage package=$stock class=declaration name=stage-stock\n",
        binding("add", standard),
        binding("add", &credit),
        binding("add", &stock),
        credit.id,
        credit.logical,
        stock.id,
        stock.logical
    );
    authored.insert(
        "consumer",
        author(context, &mut consumer, "consumer", &prelude, CONSUMER)?,
    );
    let schema_index = context.receipt.commands.len();
    let schema_path = context
        .evidence
        .join(format!("{MATERIAL}-stock-schema.lkjc"));
    context.cli(
        Some(&consumer.path),
        &[
            "change",
            "draft",
            "--owner",
            &consumer.symbols["$stock-type"],
            "--output",
            &schema_path.display().to_string(),
        ],
        true,
    )?;
    let available_first = stock_order(
        &fs::read_to_string(&schema_path)?,
        &consumer.symbols["$stock-available"],
        &consumer.symbols["$stock-reserved"],
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    let old_revision = consumer.revision.clone();
    let old_build = context.receipt.commands.len();
    context.build(&consumer, &format!("{MATERIAL}-old"))?;

    let draft_path = context.evidence.join(format!("{MATERIAL}-draft.lkjc"));
    let draft_index = context.receipt.commands.len();
    context.cli(
        Some(&credit.path),
        &[
            "change",
            "draft",
            "--owner",
            &credit.symbols["$credit-module"],
            "--output",
            &draft_path.display().to_string(),
        ],
        true,
    )?;
    let unchanged = context.cli(
        Some(&credit.path),
        &[
            "change",
            "plan",
            "--input-file",
            &draft_path.display().to_string(),
        ],
        true,
    )?;
    require(
        field(&unchanged, "result", "outcome")? == "unchanged",
        "native reconstructed credit draft changed meaning",
    )?;
    let draft = fs::read_to_string(&draft_path)?;
    authored.insert(
        "edited-credit",
        apply_input(
            context,
            &mut credit,
            "edited-credit",
            &edited_draft(&draft)?,
        )?,
    );
    context.cli(Some(&credit.path), &["check"], true)?;
    context.export(&mut credit)?;
    require(
        credit.id == old_credit.id && credit.logical != old_credit.logical,
        "native credit edit lost identity or behavior revision",
    )?;
    context.stage(&consumer, &credit)?;
    authored.insert(
        "dependency",
        author(
            context,
            &mut consumer,
            "dependency",
            &binding("replace", &credit),
            "",
        )?,
    );
    context.cli(Some(&consumer.path), &["check"], true)?;
    let new_build = context.receipt.commands.len();
    context.build(&consumer, &format!("{MATERIAL}-new"))?;
    for package in [&credit, &stock, &consumer] {
        fs::remove_dir_all(&package.path)?;
    }

    let mut observations = Vec::new();
    for (case, (artifact, target, expected, expected_state)) in cases().into_iter().enumerate() {
        let bundle = context.root.join(format!("{MATERIAL}-{case}"));
        fs::create_dir(&bundle)?;
        fs::copy(
            context.evidence.join(format!("{MATERIAL}-{artifact}.lkja")),
            bundle.join("consumer.lkja"),
        )?;
        for name in ["shared", "other"] {
            context.cli(
                None,
                &[
                    "data",
                    "initialize",
                    "--root",
                    &bundle.join(name).display().to_string(),
                ],
                true,
            )?;
        }
        let initialize = invoke(context, &bundle, "initialize", true)?;
        require(
            field(
                &output(&context.evidence, initialize)?,
                "execution",
                "value",
            )? == "true",
            "composition seed failed",
        )?;
        let before = state(&bundle)?;
        let invocation = invoke(context, &bundle, target, expected.is_some())?;
        let result = output(&context.evidence, invocation)?;
        let after = state(&bundle)?;
        observations.push(json!({"initialize":initialize,"invocation":invocation,"read":null,"recovery":null,"before":before,"after":after,"reopened":null}));
        fs::write(
            context.evidence.join(format!("{MATERIAL}-states.json")),
            evidence::encode_json(&observations)?,
        )?;
        if let Some(expected) = &expected {
            require(
                serde_json::from_str::<Value>(&field(&result, "execution", "value")?)? == *expected,
                "composition returned wrong fixed result",
            )?;
        } else {
            require(
                field(&result, "diagnostic", "code")? == expected_code(target),
                "composition failure diagnostic differs",
            )?;
        }
        states(&before, &after, &expected_state, target, available_first)?;
        let read = invoke(context, &bundle, "read", true)?;
        require(
            serde_json::from_str::<Value>(&field(
                &output(&context.evidence, read)?,
                "execution",
                "value",
            )?)? == expected_state,
            "separate-process composition reopen disagrees",
        )?;
        let reopened = state(&bundle)?;
        require(
            after == reopened,
            "read-only composition reopen changed durable state",
        )?;
        let recovery = if target == "trapped" {
            let index = invoke(context, &bundle, "compose", true)?;
            require(
                serde_json::from_str::<Value>(&field(
                    &output(&context.evidence, index)?,
                    "execution",
                    "value",
                )?)? == json!({"case":"Committed","value":receipt_value(70,6,2)}),
                "healthy invocation after callback trap failed",
            )?;
            Some(index)
        } else {
            None
        };
        observations[case] = json!({"initialize":initialize,"invocation":invocation,"read":read,"recovery":recovery,"before":before,"after":after,"reopened":reopened});
        fs::write(
            context.evidence.join(format!("{MATERIAL}-states.json")),
            evidence::encode_json(&observations)?,
        )?;
    }
    fs::write(
        context.evidence.join(format!("{MATERIAL}-states.json")),
        evidence::encode_json(&observations)?,
    )?;
    context.receipt.observations.insert("requirement_composition".into(), json!({"discovery":discovery,"authored":authored,"draft":draft_index,"schema":schema_index,"sources_removed":true,"credit":credit.id,"stock":stock.id,"consumer":consumer.id,"old_credit":old_credit.logical,"new_credit":credit.logical,"credit_revision":credit.revision,"old_revision":old_revision,"new_revision":consumer.revision,"old_build":old_build,"new_build":new_build,"inventory_start":inventory_start}).to_string());
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let observation: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_composition")
            .ok_or_else(|| DevError::corrupt("native composition evidence missing"))?,
    )?;
    require(
        observation["sources_removed"] == true
            && observation["credit"] != observation["stock"]
            && observation["consumer"] != observation["credit"]
            && observation["consumer"] != observation["stock"]
            && observation["old_credit"] != observation["new_credit"],
        "composition independent libraries or exact evolution absent",
    )?;
    let mut commands = Vec::new();
    let copied = Path::new(&receipt.isolated_root)
        .join("lkjscript")
        .display()
        .to_string();
    let consumer_apply: usize =
        serde_json::from_value(observation["authored"]["consumer"][1].clone())?;
    let created = output(root, consumer_apply)?;
    let identity = |symbol: &str| -> Result<String, DevError> {
        let record = created
            .iter()
            .find(|record| {
                record.operation == "identity"
                    && record
                        .fields
                        .iter()
                        .any(|field| field.name == "symbol" && field.value == symbol)
            })
            .ok_or_else(|| DevError::corrupt("nominal stock authored identity omitted"))?;
        record_field(record, "id")
    };
    let schema: usize = serde_json::from_value(observation["schema"].clone())?;
    let schema_path = format!("{MATERIAL}-stock-schema.lkjc");
    let schema_source = fs::read_to_string(root.join(&schema_path))?;
    let schema_command = receipt
        .commands
        .get(schema)
        .ok_or_else(|| DevError::corrupt("nominal stock schema discovery omitted"))?;
    require(
        schema_command.expects_success
            && schema_command.command
                == [
                    copied.clone(),
                    "--project".into(),
                    Path::new(&receipt.isolated_root)
                        .join("composition-consumer")
                        .display()
                        .to_string(),
                    "change".into(),
                    "draft".into(),
                    "--owner".into(),
                    identity("$stock-type")?,
                    "--output".into(),
                    Path::new(&receipt.evidence_root)
                        .join(&schema_path)
                        .display()
                        .to_string(),
                ]
            && schema_source.starts_with(&format!(
                "request base={} ",
                observation["old_revision"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("nominal stock source revision"))?
            )),
        "nominal stock schema came from another accepted source",
    )?;
    let available_first = stock_order(
        &schema_source,
        &identity("$stock-available")?,
        &identity("$stock-reserved")?,
    )?;
    let discovery: usize = serde_json::from_value(observation["discovery"].clone())?;
    for (offset, arguments) in [
        (0, ["capabilities", "--section", "change"]),
        (1, ["package", "builtin", "inspect"]),
    ] {
        let command = receipt
            .commands
            .get(discovery + offset)
            .ok_or_else(|| DevError::corrupt("composition discovery omitted"))?;
        require(
            command.expects_success
                && command.command
                    == std::iter::once(copied.clone())
                        .chain(arguments.map(str::to_owned))
                        .collect::<Vec<_>>(),
            "composition public discovery changed",
        )?;
    }
    for (position, (kind, name)) in CONTRACTS.into_iter().enumerate() {
        let query = discovery + 2 + position * 2;
        let found = output(root, query)?;
        let identity = field(&found, "owner", "id")?;
        let contract = output(root, query + 1)?;
        for (index, arguments) in [
            (
                query,
                vec![
                    "package", "builtin", "query", "owners", "--kind", kind, "--name", name,
                ],
            ),
            (
                query + 1,
                vec!["package", "builtin", "inspect", "owner", kind, &identity],
            ),
        ] {
            let command = receipt
                .commands
                .get(index)
                .ok_or_else(|| DevError::corrupt("standard cell discovery missing"))?;
            require(
                command.expects_success
                    && command.command
                        == std::iter::once(copied.clone())
                            .chain(arguments.into_iter().map(str::to_owned))
                            .collect::<Vec<_>>(),
                "standard cell discovery changed exact public selection",
            )?;
        }
        require(
            field(&found, "owner", "name")? == name && field(&contract, "owner", "id")? == identity,
            "standard cell discovery/contract binding differs",
        )?;
        if kind == "operation" {
            require(
                field(&contract, "operation", "idempotency")? == "idempotent"
                    && field(&contract, "operation", "external-visibility")? == "none"
                    && field(&contract, "operation", "parameters")? == "0"
                    && field(&contract, "type", "form")? == "unit",
                "standard participation guard contract differs",
            )?;
        } else {
            require(
                field(&contract, "type-parameter", "constraint")? == "none"
                    && field(&contract, "requirement-parameter", "minimum-operations")?
                        == if position == 1 { "3" } else { "4" },
                "standard cell generic/minimum contract differs",
            )?;
        }
    }
    let draft: usize = serde_json::from_value(observation["draft"].clone())?;
    require(
        field(&output(root, draft + 1)?, "result", "outcome")? == "unchanged",
        "composition native draft lacks its unchanged public review",
    )?;
    let inventory_start: usize = serde_json::from_value(observation["inventory_start"].clone())?;
    for (offset, package, logical) in [
        (0, "credit", "old_credit"),
        (1, "stock", ""),
        (2, "credit", "new_credit"),
    ] {
        let inventory = receipt
            .inventories
            .get(inventory_start + offset)
            .ok_or_else(|| DevError::corrupt("composition exported inventory missing"))?;
        require(
            inventory.packages.iter().any(|entry| {
                observation[package] == entry.package
                    && (logical.is_empty() || observation[logical] == entry.package_revision)
            }),
            "composition transported library identity changed",
        )?;
    }
    for (name, literal) in [
        ("credit", CREDIT),
        ("stock", STOCK),
        ("consumer", CONSUMER),
        ("edited-credit", ""),
        ("dependency", ""),
    ] {
        let input_name = format!("{MATERIAL}-{name}.lkjc");
        let input = fs::read_to_string(root.join(&input_name))?;
        require(
            input.starts_with("request base=rev_") && input.ends_with(literal),
            "composition literal native request changed",
        )?;
        if name == "edited-credit" {
            require(
                input
                    == edited_draft(&fs::read_to_string(
                        root.join(format!("{MATERIAL}-draft.lkjc")),
                    )?)?,
                "composition edit did not derive from the exact retained native draft",
            )?;
        } else if name == "dependency" {
            let expected = format!(
                "request base={} idempotency={MATERIAL}-dependency\nreplace.dependency package={} semantic-revision={} package-revision={}\n",
                observation["old_revision"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("composition old revision"))?,
                observation["credit"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("composition credit identity"))?,
                observation["credit_revision"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("composition credit revision"))?,
                observation["new_credit"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("composition new exact dependency"))?
            );
            require(
                input == expected,
                "composition exact dependency repair changed",
            )?;
        }
        let indices: Vec<usize> = serde_json::from_value(observation["authored"][name].clone())?;
        require(
            indices.len() == 2 && indices[0] < indices[1],
            "composition reviewed apply missing",
        )?;
        let plan = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(
            root.join(format!("{MATERIAL}-{name}.lkjplan")),
        )?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        for (position, index) in indices.iter().copied().enumerate() {
            let command = receipt
                .commands
                .get(index)
                .ok_or_else(|| DevError::corrupt("composition authored command absent"))?;
            let kind = if position == 0 { "plan" } else { "apply" };
            require(
                command.expects_success
                    && command.command[0]
                        == Path::new(&receipt.isolated_root)
                            .join("lkjscript")
                            .display()
                            .to_string()
                    && command
                        .command
                        .windows(2)
                        .any(|words| words == ["change", kind])
                    && command.command.windows(2).any(|words| {
                        words
                            == [
                                "--input-file",
                                &Path::new(&receipt.evidence_root)
                                    .join(&input_name)
                                    .display()
                                    .to_string(),
                            ]
                    })
                    && field(&output(root, index)?, "plan", "token")? == plan.token,
                "composition plan/apply differs from exact literal and reviewed token",
            )?;
        }
    }
    let states_file: Vec<Value> = serde_json::from_slice(&process::read_bounded(
        &root.join(format!("{MATERIAL}-states.json")),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        states_file.len() == cases().len(),
        "composition cases omitted",
    )?;
    let mut artifacts = BTreeMap::new();
    for name in ["old", "new"] {
        artifacts.insert(
            name,
            lkjscript::platform::contributor::strict_artifact_identity_probe(
                &process::read_bounded(
                    &root.join(format!("{MATERIAL}-{name}.lkja")),
                    MAXIMUM_CONTAINER_BYTES,
                )?,
            )
            .map_err(|error| DevError::corrupt(error.to_string()))?,
        );
        let index: usize = serde_json::from_value(observation[format!("{name}_build")].clone())?;
        let command = receipt
            .commands
            .get(index)
            .ok_or_else(|| DevError::corrupt("composition build command missing"))?;
        require(
            command.expects_success
                && command.command
                    == [
                        copied.clone(),
                        "--project".into(),
                        Path::new(&receipt.isolated_root)
                            .join("composition-consumer")
                            .display()
                            .to_string(),
                        "build".into(),
                        "--output".into(),
                        Path::new(&receipt.evidence_root)
                            .join(format!("{MATERIAL}-{name}.lkja"))
                            .display()
                            .to_string(),
                    ]
                && field(&output(root, index)?, "artifact", "bundle")? == artifacts[name],
            "composition artifact did not come from the retained consumer build",
        )?;
    }
    for (case, ((artifact, target, expected, expected_state), observed)) in
        cases().into_iter().zip(states_file).enumerate()
    {
        states(
            &observed["before"],
            &observed["after"],
            &expected_state,
            target,
            available_first,
        )?;
        require(
            observed["after"] == observed["reopened"],
            "composition reopened state differs",
        )?;
        let mut expected_commands = vec![
            ("initialize", "initialize", Some(json!(true))),
            ("invocation", target, expected),
            ("read", "read", Some(expected_state)),
        ];
        if target == "trapped" {
            expected_commands.push((
                "recovery",
                "compose",
                Some(json!({"case":"Committed","value":receipt_value(70,6,2)})),
            ));
        }
        for (key, operation, expected) in expected_commands {
            let index = observed[key]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| DevError::corrupt("composition invocation index absent"))?;
            let command = receipt
                .commands
                .get(index)
                .ok_or_else(|| DevError::corrupt("composition invocation missing"))?;
            let path = Path::new(&receipt.isolated_root)
                .join(format!("{MATERIAL}-{case}/{operation}.deployment.json"));
            require(
                serde_json::from_slice::<Value>(&process::read_bounded(
                    &root.join(format!("{MATERIAL}-{case}-{operation}.deployment.json")),
                    MAXIMUM_OUTPUT_BYTES,
                )?)? == descriptor(operation),
                "composition exact grants or descriptor changed",
            )?;
            require(
                command.expects_success == expected.is_some()
                    && command.command
                        == [
                            receipt.pinned_runtime_path.clone(),
                            "run".into(),
                            "--deployment".into(),
                            path.display().to_string(),
                            "--arguments".into(),
                            "[]".into(),
                        ],
                "composition invocation changed runtime, target, or inputs",
            )?;
            let result = output(root, index)?;
            if let Some(expected) = expected {
                require(
                    serde_json::from_str::<Value>(&field(&result, "execution", "value")?)?
                        == expected
                        && field(&result, "execution", "artifact")? == artifacts[artifact]
                        && observation[format!("{artifact}_revision")]
                            == field(&result, "execution", "revision")?,
                    "composition result or exact artifact differs",
                )?;
                let cleanup: Value =
                    serde_json::from_str(&field(&result, "execution", "cleanup")?)?;
                let work: Value =
                    serde_json::from_str(&field(&result, "execution", "production-observation")?)?;
                require(
                    cleanup["remaining_tasks"] == 0 && cleanup["cleanup_failures"] == json!([]),
                    "composition cleanup did not join",
                )?;
                for key in [
                    "live_call_frames_after",
                    "live_handles_after",
                    "live_locals_after",
                    "live_operands_after",
                    "live_transactions_after",
                    "live_type_bindings_after",
                ] {
                    require(work[key] == 0, "composition retained owned execution state")?;
                }
            } else {
                require(
                    field(&result, "diagnostic", "code")? == expected_code(operation),
                    "composition negative diagnostic differs",
                )?;
            }
            commands.push(index);
        }
    }
    Ok(commands)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "fixed independent adversarial store snapshots"
)]
mod tests {
    use super::*;

    fn initial() -> Value {
        let mut seed = b"LKJDVAL1\0\x01".to_vec();
        seed.resize(90, 0);
        let stock = stock_bytes(&seed, 8, 0, true).unwrap();
        json!({
            "shared":{"head":[1],"revision":"original","verify":{"revisions":2},"entries":{
                "credit":{"bytes":typed_bytes(&json!(100)).unwrap(),"revision":"credit"},
                "stock":{"bytes":stock,"revision":"stock"},"marker":null}},
            "other":{"head":[2],"revision":"original-other","verify":{"revisions":1},"entries":{
                "credit":null,"stock":null,"marker":null}}
        })
    }

    #[test]
    fn abort_reader_rejects_changed_head_tentative_bytes_and_hidden_callback() {
        let before = initial();
        states(
            &before,
            &before,
            &receipt_value(100, 8, 0),
            "failed-condition",
            true,
        )
        .unwrap();
        let mut head = before.clone();
        head["shared"]["head"] = json!([3]);
        assert!(
            states(
                &before,
                &head,
                &receipt_value(100, 8, 0),
                "failed-condition",
                true
            )
            .is_err()
        );
        let mut tentative = before.clone();
        tentative["shared"]["entries"]["credit"]["bytes"] = json!(typed_bytes(&json!(70)).unwrap());
        assert!(
            states(
                &before,
                &tentative,
                &receipt_value(100, 8, 0),
                "failed-condition",
                true
            )
            .is_err()
        );
        let mut callback = before.clone();
        callback["other"]["entries"]["marker"] =
            json!({"bytes":typed_bytes(&json!("survived")).unwrap()});
        assert!(
            states(
                &before,
                &callback,
                &receipt_value(100, 8, 0),
                "absent-owner",
                true
            )
            .is_err()
        );
        assert!(
            states(
                &before,
                &before,
                &receipt_value(100, 8, 0),
                "independent",
                true
            )
            .is_err()
        );
    }

    #[test]
    fn completed_reader_requires_one_completion_and_exact_nominal_payload() {
        let before = initial();
        let mut completed = before.clone();
        let seed: Vec<u8> =
            serde_json::from_value(before["shared"]["entries"]["stock"]["bytes"].clone()).unwrap();
        completed["shared"]["entries"]["credit"]["bytes"] = json!(typed_bytes(&json!(70)).unwrap());
        completed["shared"]["entries"]["stock"]["bytes"] =
            json!(stock_bytes(&seed, 6, 2, true).unwrap());
        completed["shared"]["head"] = json!([3]);
        completed["shared"]["verify"]["revisions"] = json!(3);
        states(
            &before,
            &completed,
            &receipt_value(70, 6, 2),
            "compose",
            true,
        )
        .unwrap();
        for revisions in [2, 4] {
            let mut forged = completed.clone();
            forged["shared"]["verify"]["revisions"] = json!(revisions);
            assert!(states(&before, &forged, &receipt_value(70, 6, 2), "compose", true).is_err());
        }
        completed["shared"]["entries"]["stock"]["bytes"] =
            json!(stock_bytes(&seed, 7, 1, true).unwrap());
        assert!(
            states(
                &before,
                &completed,
                &receipt_value(70, 6, 2),
                "compose",
                true
            )
            .is_err()
        );
    }
}
