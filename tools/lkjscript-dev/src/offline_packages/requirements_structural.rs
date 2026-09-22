//! Public structural-library planning and inspection evidence; never writes semantic storage.
use super::*;

fn native_fixture(name: &str) -> Option<String> {
    (name == "producer").then(|| {
        format!(
            "{}{}",
            include_str!("requirements.producer.native.lkjc"),
            include_str!("requirements.resource-library.native.lkjc")
        )
    })
}

// The flat fixtures are an independently retained authoring oracle. Both notations are planned
// against one base and the flat token authorizes the structural request; no live effect is replayed.
pub(super) fn apply(
    context: &mut Context,
    package: &mut Package,
    name: &str,
    prelude: &str,
    flat: &str,
    structural: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    let header = format!(
        "request base={} idempotency=structural-{name}\n{prelude}",
        package.revision
    );
    let mut plans = Vec::new();
    let mut commands = Vec::new();
    let native = native_fixture(name);
    let mut notations = vec![("flat", flat), ("structural", structural)];
    if let Some(native) = &native {
        notations.push(("native", native));
    }
    for (notation, literal) in &notations {
        let input = context
            .evidence
            .join(format!("requirement-{name}-{notation}.lkjc"));
        let plan = context
            .evidence
            .join(format!("requirement-{name}-{notation}.lkjplan"));
        fs::write(&input, format!("{header}{literal}"))?;
        commands.push(context.receipt.commands.len());
        let output = context.cli(
            Some(&package.path),
            &[
                "change",
                "plan",
                "--input-file",
                &input.display().to_string(),
                "--output",
                &plan.display().to_string(),
            ],
            true,
        )?;
        let decoded = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(&plan)?))
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            decoded.token == field(&output, "plan", "token")?,
            "structural library review file disagrees with its public token",
        )?;
        plans.push((decoded, process::read_bounded(&plan, MAXIMUM_OUTPUT_BYTES)?));
    }
    require(
        plans.iter().all(|plan| plan == &plans[0]),
        &format!(
            "flat, structural or native {name} changed canonical intent or reviewed candidate"
        ),
    )?;
    let input = context.evidence.join(format!(
        "requirement-{name}-{}.lkjc",
        if native.is_some() {
            "native"
        } else {
            "structural"
        }
    ));
    commands.push(context.receipt.commands.len());
    let applied = context.cli(
        Some(&package.path),
        &[
            "change",
            "apply",
            "--input-file",
            &input.display().to_string(),
            "--plan",
            &plans[0].0.token,
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
    let mut evidence: Value = context
        .receipt
        .observations
        .get("requirement_structural")
        .map(|value| serde_json::from_str(value))
        .transpose()?
        .unwrap_or_else(|| json!({}));
    evidence[name] = json!({"commands":commands,"revision":package.revision});
    context
        .receipt
        .observations
        .insert("requirement_structural".into(), evidence.to_string());
    Ok(applied)
}

pub(super) fn inspect_factory(
    context: &mut Context,
    producer: &Package,
) -> Result<Vec<CompactRecord>, DevError> {
    context.cli(
        Some(&producer.path),
        &[
            "inspect",
            "owner",
            "pure_function",
            &producer.symbols["$factory"],
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )
}

fn signature_identities(
    records: &[CompactRecord],
    expected: usize,
) -> Result<BTreeMap<String, String>, DevError> {
    require(
        field(records, "page", "complete")? == "true",
        "structural function inspection is incomplete",
    )?;
    let mut owners = BTreeMap::new();
    for record in records.iter().filter(|record| {
        matches!(
            record.operation.as_str(),
            "definition.function"
                | "definition.parameter"
                | "definition.type-parameter"
                | "definition.effect-parameter"
                | "definition.requirement-parameter"
        )
    }) {
        let key = format!("{}:{}", record.operation, record_field(record, "name")?);
        require(
            owners.insert(key, record_field(record, "id")?).is_none(),
            "duplicate function signature owner in public inspection",
        )?;
    }
    require(
        owners.len() == expected
            && owners
                .values()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == expected,
        "function inspection omitted its declaration, value parameters, or T/E/R owners",
    )?;
    Ok(owners)
}

fn body_owners(records: &[CompactRecord]) -> Result<Vec<String>, DevError> {
    let owners = records
        .iter()
        .filter(|record| {
            matches!(
                record.operation.as_str(),
                "definition.expression" | "definition.binding"
            )
        })
        .map(|record| record_field(record, "id"))
        .collect::<Result<Vec<_>, _>>()?;
    require(
        owners
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == owners.len(),
        "structural body inspection duplicates an expression or binding identity",
    )?;
    Ok(owners)
}

pub(super) fn validate_body_changes(
    before: [&[CompactRecord]; 2],
    after: [&[CompactRecord]; 2],
    supplier_plan: &[CompactRecord],
) -> Result<(), DevError> {
    let mut former_bodies = Vec::new();
    for (index, signature, old_count, new_count) in [(0, 7, 5, 8), (1, 8, 5, 13)] {
        let old = body_owners(before[index])?;
        let new = body_owners(after[index])?;
        require(
            signature_identities(before[index], signature)?
                == signature_identities(after[index], signature)?
                && field(before[index], "definition.function", "body")?
                    != field(after[index], "definition.function", "body")?
                && old.len() == old_count
                && new.len() == new_count
                && old.iter().all(|owner| !new.contains(owner)),
            "structural replacement changed retained signature identities or reused old body owners",
        )?;
        former_bodies.extend(old);
    }
    let retired = supplier_plan
        .iter()
        .filter(|record| record.operation == "logical-plan.retirement")
        .map(|record| {
            require(
                record_field(record, "before-present")? == "false"
                    && record_field(record, "after-present")? == "true",
                "structural replacement did not create its body retirements",
            )?;
            record_field(record, "owner")
        })
        .collect::<Result<Vec<_>, _>>()?;
    require(
        retired.len() == former_bodies.len()
            && former_bodies.iter().all(|owner| retired.contains(owner)),
        "structural replacement omitted retirement of a former body owner",
    )
}

fn structural_output(root: &Path, index: usize) -> Result<Vec<CompactRecord>, DevError> {
    parse_records(
        "requirement-structural-output",
        &process::read_bounded(
            &root.join(format!("command-{index:04}.stdout")),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("structural library public output"))
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<(), DevError> {
    let value: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_structural")
            .ok_or_else(|| DevError::corrupt("structural library evidence missing"))?,
    )?;
    let packages: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_packages")
            .ok_or_else(|| DevError::corrupt("structural library package binding missing"))?,
    )?;
    let original_root = Path::new(&receipt.isolated_root);
    let original_evidence = Path::new(&receipt.evidence_root);
    let copied = original_root.join("lkjscript").display().to_string();
    for (name, package, flat, structural) in [
        (
            "producer",
            "requirement-producer",
            format!(
                "{}{}",
                include_str!("requirements.producer.lkjc"),
                include_str!("requirements.resource-library.lkjc")
            ),
            format!(
                "{}{}",
                include_str!("requirements.producer.structural.lkjc"),
                include_str!("requirements.resource-library.structural.lkjc")
            ),
        ),
        (
            "consumer",
            "requirement-consumer",
            format!(
                "{}{}{}{}",
                include_str!("requirements.consumer.lkjc"),
                include_str!("requirements.outcomes.consumer.flat.lkjc"),
                include_str!("requirements.resource-consumer.lkjc"),
                include_str!("requirements.unencodable.lkjc")
            ),
            format!(
                "{}{}{}{}",
                include_str!("requirements.consumer.structural.lkjc"),
                include_str!("requirements.outcomes.consumer.structural.lkjc"),
                include_str!("requirements.resource-consumer.structural.lkjc"),
                include_str!("requirements.unencodable.lkjc")
            ),
        ),
        (
            "supplier",
            "requirement-producer",
            format!(
                "{}{}",
                include_str!("requirements.stronger.lkjc"),
                include_str!("requirements.update-body.flat.lkjc")
            ),
            format!(
                "{}{}",
                include_str!("requirements.stronger.structural.lkjc"),
                include_str!("requirements.update-body.structural.lkjc")
            ),
        ),
    ] {
        let indices: Vec<usize> = serde_json::from_value(value[name]["commands"].clone())?;
        let native = native_fixture(name);
        let mut notations = vec![("flat", &flat), ("structural", &structural)];
        if let Some(native) = &native {
            notations.push(("native", native));
        }
        require(
            indices.len() == notations.len() + 1
                && indices.windows(2).all(|pair| pair[0] < pair[1]),
            "structural library plans/apply missing or reordered",
        )?;
        let mut prefix = None;
        let mut plans = Vec::new();
        for ((notation, literal), index) in notations.iter().copied().zip(&indices) {
            let input_name = format!("requirement-{name}-{notation}.lkjc");
            let plan_name = format!("requirement-{name}-{notation}.lkjplan");
            let input = String::from_utf8(process::read_bounded(
                &root.join(&input_name),
                MAXIMUM_OUTPUT_BYTES,
            )?)
            .map_err(|_| DevError::corrupt("structural library input is not UTF-8"))?;
            let observed_prefix = input
                .strip_suffix(literal.as_str())
                .ok_or_else(|| DevError::corrupt("independent literal library request changed"))?;
            require(
                observed_prefix.starts_with("request base=rev_")
                    && observed_prefix.contains(&format!(" idempotency=structural-{name}\n")),
                "structural parity request omitted its original reviewed header",
            )?;
            if let Some(prefix) = &prefix {
                require(
                    prefix == observed_prefix,
                    "flat/structural requests changed their base, controls, or selection prelude",
                )?;
            } else {
                prefix = Some(observed_prefix.to_owned());
            }
            let command = receipt
                .commands
                .get(*index)
                .ok_or_else(|| DevError::corrupt("structural plan command absent"))?;
            require(
                command.expects_success
                    && command.command
                        == [
                            copied.clone(),
                            "--project".into(),
                            original_root.join(package).display().to_string(),
                            "change".into(),
                            "plan".into(),
                            "--input-file".into(),
                            original_evidence.join(&input_name).display().to_string(),
                            "--output".into(),
                            original_evidence.join(&plan_name).display().to_string(),
                        ],
                "structural library plan changed executable, project, request, or review output",
            )?;
            let bytes = process::read_bounded(&root.join(&plan_name), MAXIMUM_OUTPUT_BYTES)?;
            let decoded = decode_logical_change_plan(std::io::Cursor::new(&bytes))
                .map_err(|error| DevError::corrupt(error.to_string()))?;
            require(
                decoded.token == field(&structural_output(root, *index)?, "plan", "token")?,
                "structural library plan token is not bound to the strict reviewed bytes",
            )?;
            plans.push((decoded, bytes));
        }
        require(
            plans.iter().all(|plan| plan == &plans[0]),
            "independent flat/structural/native request commitments or reviewed candidates differ",
        )?;
        let applied = receipt
            .commands
            .get(indices[notations.len()])
            .ok_or_else(|| DevError::corrupt("structural library apply absent"))?;
        require(
            applied.expects_success
                && applied.command
                    == [
                        copied.clone(),
                        "--project".into(),
                        original_root.join(package).display().to_string(),
                        "change".into(),
                        "apply".into(),
                        "--input-file".into(),
                        original_evidence
                            .join(format!(
                                "requirement-{name}-{}.lkjc",
                                if native.is_some() {
                                    "native"
                                } else {
                                    "structural"
                                }
                            ))
                            .display()
                            .to_string(),
                        "--plan".into(),
                        plans[0].0.token.clone(),
                    ],
            "structural apply did not consume the equivalent flat plan token",
        )?;
        let output = structural_output(root, indices[notations.len()])?;
        require(
            field(&output, "plan", "token")? == plans[0].0.token
                && value[name]["revision"] == field(&output, "revision", "result")?,
            "structural library apply changed reviewed identity or result revision",
        )?;
    }
    let mut definitions = Vec::new();
    for (key, kind, phase, symbol, name) in [
        (
            "factory_before",
            "pure_function",
            "producer",
            "$factory",
            "make-cell-updater",
        ),
        (
            "factory_after",
            "pure_function",
            "supplier",
            "$factory",
            "make-cell-updater",
        ),
        (
            "update_before",
            "task_function",
            "producer",
            "$update",
            "try-update",
        ),
        (
            "update_after",
            "task_function",
            "supplier",
            "$update",
            "try-update",
        ),
    ] {
        let index = value[key]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| DevError::corrupt("structural function inspection index absent"))?;
        let definition = structural_output(root, index)?;
        let created_index = value["producer"]["commands"][3]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| DevError::corrupt("structural producer apply index absent"))?;
        let creation = structural_output(root, created_index)?;
        let identity = creation
            .iter()
            .find(|record| {
                record.operation == "identity"
                    && record
                        .fields
                        .iter()
                        .any(|field| field.name == "symbol" && field.value == symbol)
            })
            .ok_or_else(|| DevError::corrupt("structural function creation identity absent"))?;
        let supplier_plan_index = value["supplier"]["commands"][0]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| DevError::corrupt("structural supplier plan index absent"))?;
        let supplier_apply_index = value["supplier"]["commands"][2]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| DevError::corrupt("structural supplier apply index absent"))?;
        require(
            field(&definition, "definition.function", "name")? == name
                && field(&definition, "definition.function", "id")?
                    == record_field(identity, "id")?
                && value[phase]["revision"] == field(&definition, "definition.header", "revision")?
                && value[phase]["revision"] == field(&definition, "revision", "observed")?
                && packages["producer"] == field(&definition, "definition.header", "package")?
                && if phase == "producer" {
                    created_index < index && index < supplier_plan_index
                } else {
                    supplier_apply_index < index
                },
            "structural inspection changed function identity, accepted revision, package, or edit order",
        )?;
        let command = receipt
            .commands
            .get(index)
            .ok_or_else(|| DevError::corrupt("structural function inspection absent"))?;
        require(
            command.expects_success
                && command.command
                    == [
                        copied.clone(),
                        "--project".into(),
                        original_root
                            .join("requirement-producer")
                            .display()
                            .to_string(),
                        "inspect".into(),
                        "owner".into(),
                        kind.into(),
                        field(&definition, "definition.function", "id")?,
                        "--detail".into(),
                        "definition".into(),
                        "--limit".into(),
                        "1000".into(),
                        "--bytes".into(),
                        "1048576".into(),
                    ],
            "structural body edit lacks its public function inspection",
        )?;
        definitions.push(definition);
    }
    let plan = parse_records(
        "requirement-supplier-plan",
        &process::read_bounded(
            &root.join("requirement-supplier-structural.lkjplan"),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|_| DevError::corrupt("structural supplier retirement records"))?;
    validate_body_changes(
        [&definitions[0], &definitions[2]],
        [&definitions[1], &definitions[3]],
        &plan,
    )
}
