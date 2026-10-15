//! Actual source batches retain the recursive pure rows on each side of the resource probe.
use super::*;

const ARTIFACT: &str = "recursive-received-exact.lkja";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Boundaries {
    export_command: usize,
    resource_command_boundary: usize,
    resource_runner: usize,
}

fn schema_cases() -> Vec<source_batch::Case> {
    let mut cases: Vec<_> = super::super::recursive_schemas::values()
        .into_iter()
        .map(|(name, value)| source_batch::Case::success(name, name, json!([value]), value))
        .collect();
    cases.push(source_batch::Case::success(
        "flip-next",
        "flip-next",
        json!([{"case":"stop"}]),
        json!({"case":"next","value":{"case":"stop"}}),
    ));
    cases
}

fn value_cases() -> Vec<source_batch::Case> {
    let original = small();
    let mut cases = Vec::new();
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
        cases.push(source_batch::Case::success(
            name,
            target,
            json!([original]),
            expected,
        ));
    }
    for (label, values) in [
        ("text", vec![json!("a"), json!("bc")]),
        ("owned", vec![json!({"text":"one"}), json!({"text":"two"})]),
    ] {
        let input = branch(vec![
            leaf(values[0].clone()),
            branch(vec![leaf(values[1].clone()), branch(vec![])]),
        ]);
        let target = format!("{label}-map");
        cases.push(source_batch::Case::success(
            &target,
            &target,
            json!([input]),
            input.clone(),
        ));
        cases.push(source_batch::Case::success(
            &format!("{label}-leaves"),
            &format!("{label}-flatten"),
            json!([input]),
            json!(values),
        ));
    }
    let empty = branch(vec![]);
    cases.push(source_batch::Case::success(
        "empty-trap",
        "trapped",
        json!([empty]),
        empty,
    ));
    let reversed = branch(vec![leaf(json!(4)), leaf(json!(2)), leaf(json!(1))]);
    for (id, input, code) in [
        (
            "division-before-overflow",
            original.clone(),
            "normalized_integer_division",
        ),
        (
            "overflow-before-division",
            reversed,
            "normalized_integer_overflow",
        ),
    ] {
        cases.push(source_batch::Case {
            id: id.to_owned(),
            target: "trapped".to_owned(),
            arguments: json!([input]).to_string(),
            expected: Err(code.to_owned()),
        });
    }
    // The original two traps remain ordered. A fresh healthy invocation in the same
    // prepared process must recover before the separately retained large resource calls.
    cases.push(source_batch::Case::success(
        "post-trap-recovery",
        "sum",
        json!([original]),
        json!(7),
    ));
    cases
}

fn cases(schemas: bool) -> Result<Vec<source_batch::Case>, DevError> {
    let cases = if schemas {
        schema_cases()
    } else {
        value_cases()
    };
    require(
        cases.len() == if schemas { 7 } else { 17 },
        "recursive complete batch inventory differs",
    )?;
    Ok(cases)
}

fn label(schemas: bool) -> &'static str {
    if schemas {
        "recursive-schemas"
    } else {
        "recursive-values"
    }
}

fn retain(
    receipt: &mut RecursiveReceipt,
    cases: &[source_batch::Case],
    rows: Vec<source_batch::Row>,
) -> Result<(), DevError> {
    require(
        rows.len() == cases.len(),
        "recursive complete retained row inventory differs",
    )?;
    let mut traps = Vec::new();
    for (case, row) in cases.iter().zip(rows) {
        if let Some(value) = row.result {
            receipt.results.insert(case.id.clone(), value);
        } else {
            traps.push(field(&row.records, "diagnostic", "code")?);
        }
    }
    if !traps.is_empty() {
        receipt
            .results
            .insert("trap-order".to_owned(), json!(traps));
    }
    Ok(())
}

pub(super) fn execute(
    context: &mut Context,
    package: &Package,
    schemas: bool,
) -> Result<(), DevError> {
    let cases = cases(schemas)?;
    let smoke = cases
        .first()
        .ok_or_else(|| DevError::corrupt("recursive ordinary smoke missing"))?;
    let observed =
        source_batch::execute(context, package, label(schemas), ARTIFACT, &cases, smoke)?;
    retain(&mut context.receipt.recursive, &cases, observed.rows)
}

pub(super) fn boundary(context: &mut Context, export_command: usize) -> Result<(), DevError> {
    let boundary = Boundaries {
        export_command,
        resource_command_boundary: context.receipt.commands.len(),
        resource_runner: context.receipt.runners.len(),
    };
    require(
        context
            .receipt
            .observations
            .insert(
                "recursive-pure-boundaries".to_owned(),
                serde_json::to_string(&boundary)?,
            )
            .is_none(),
        "duplicate recursive pure boundary witness",
    )
}

pub(super) fn read(receipt: &Receipt, root: &Path) -> Result<(), DevError> {
    let boundary: Boundaries = serde_json::from_str(
        receipt
            .observations
            .get("recursive-pure-boundaries")
            .ok_or_else(|| DevError::corrupt("recursive pure boundary witness missing"))?,
    )?;
    let project = Path::new(&receipt.isolated_root).join("recursive-receiver");
    let mut observed = Vec::new();
    for schemas in [true, false] {
        let cases = cases(schemas)?;
        let smoke = cases
            .first()
            .ok_or_else(|| DevError::corrupt("recursive ordinary smoke missing"))?;
        let batch = source_batch::read(
            receipt,
            root,
            label(schemas),
            &project,
            ARTIFACT,
            &cases,
            smoke,
        )?;
        require(
            batch.package == receipt.recursive.receiver_package,
            "recursive batch selected a foreign receiver",
        )?;
        let mut expected = RecursiveReceipt::default();
        retain(&mut expected, &cases, batch.rows)?;
        for (key, value) in expected.results {
            require(
                receipt.recursive.results.get(&key) == Some(&value),
                "recursive retained result differs from its complete actual batch row",
            )?;
        }
        observed.push((
            batch.inventory,
            batch.revision,
            batch.bundle,
            batch.smoke,
            batch.command,
        ));
    }
    let [before, after] = observed.as_slice() else {
        return Err(DevError::corrupt("recursive source batch phases missing"));
    };
    require(
        before.0 == after.0
            && before.1 == after.1
            && before.2 == after.2
            && boundary.export_command < before.3
            && before.3.checked_add(1) == Some(before.4)
            && before.4.checked_add(1) == Some(boundary.resource_command_boundary)
            && boundary.resource_command_boundary == after.3
            && after.3.checked_add(1) == Some(after.4),
        "recursive batches changed source or crossed their retained resource probe",
    )?;
    let phase_witness = |schemas| -> Result<Value, DevError> {
        Ok(serde_json::from_str(
            receipt
                .observations
                .get(&format!("source-batch-{}.witness", label(schemas)))
                .ok_or_else(|| DevError::corrupt("recursive source authority witness missing"))?,
        )?)
    };
    let before_authority = phase_witness(true)?;
    let after_authority = phase_witness(false)?;
    require(
        before_authority
            .get("authority_after")
            .is_some_and(|authority| Some(authority) == after_authority.get("authority_before"))
            && process::read_bounded(
                &root.join("source-batch-recursive-schemas.head"),
                1024 * 1024,
            )? == process::read_bounded(
                &root.join("source-batch-recursive-values.head"),
                1024 * 1024,
            )?,
        "recursive resource probe changed exact accepted HEAD or source authority",
    )?;
    let runner = receipt
        .runners
        .get(boundary.resource_runner)
        .ok_or_else(|| DevError::corrupt("recursive resource probe observation missing"))?;
    require(
        runner.as_semantic().is_some() && runner.returned_success(),
        "recursive resource probe did not complete between batches",
    )?;
    verify_observation_files(runner, &receipt.files, "recursive-resources")?;
    let export = receipt
        .commands
        .get(boundary.export_command)
        .ok_or_else(|| DevError::corrupt("recursive receiver source export missing"))?;
    require(
        export.expects_success
            && export.cwd == receipt.isolated_root
            && export.observation.returned_success()
            && export.observation.reported_code() == Some(0)
            && export
                .observation
                .as_process()
                .is_none_or(|process| process.signal.is_none())
            && export.command
                == [
                    receipt.host_path.clone(),
                    "--project".to_owned(),
                    project.display().to_string(),
                    "package".to_owned(),
                    "current".to_owned(),
                    "export".to_owned(),
                    "--kind".to_owned(),
                    "transport".to_owned(),
                    "--output".to_owned(),
                    Path::new(&receipt.isolated_root)
                        .join(format!("transport-{}.lkjp", boundary.export_command))
                        .display()
                        .to_string(),
                ],
        "recursive receiver was not exported by the actual copied candidate",
    )?;
    let exported = parse_records(
        "recursive-receiver-export",
        &process::read_bounded(
            &root.join(&export.observation.stdout().path),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
    .map_err(|error| DevError::corrupt(format!("recursive receiver export output: {error:?}")))?;
    require(
        field(&exported, "result", "status")? == "success"
            && field(&exported, "result", "command")? == "package.current.export"
            && field(&exported, "package", "id")? == receipt.recursive.receiver_package
            && field(&exported, "package", "revision")? == before.1
            && receipt.transport_digests.get(before.0)
                == Some(&field(&exported, "package", "transport")?),
        "recursive receiver export differs from its independently admitted batch source",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an authentic current full offline receipt and its exact candidate/verifier"]
    fn retained_recursive_batches_reject_omission_retargeting_and_probe_reordering() {
        let input =
            |name| PathBuf::from(std::env::var_os(name).expect("explicit actual offline original"));
        let source = input("LKJSCRIPT_OFFLINE_READER_RECEIPT");
        let verifier = input("LKJSCRIPT_OFFLINE_READER_VERIFIER");
        let receipt = super::super::super::read_semantic_receipt(&source, &verifier)
            .expect("complete actual current original passes its full reader");
        let root = source.parent().expect("actual evidence root");
        read(&receipt, root).expect("actual two recursive batch phases");
        let reject = |fault: Receipt| assert!(read(&fault, root).is_err());
        for name in [
            "source-batch-recursive-schemas.witness",
            "source-batch-recursive-values.witness",
            "recursive-pure-boundaries",
        ] {
            let mut fault = receipt.clone();
            fault
                .observations
                .remove(name)
                .expect("actual mandatory witness");
            reject(fault);
        }
        for name in [
            "node-value",
            "flip-next",
            "text-map",
            "owned-map",
            "post-trap-recovery",
            "trap-order",
        ] {
            let mut fault = receipt.clone();
            fault
                .recursive
                .results
                .remove(name)
                .expect("actual complete result");
            reject(fault);
        }
        for name in [
            "export_command",
            "resource_command_boundary",
            "resource_runner",
        ] {
            let mut fault = receipt.clone();
            let mut boundary: Value =
                serde_json::from_str(&fault.observations["recursive-pure-boundaries"]).unwrap();
            boundary[name] = json!(boundary[name].as_u64().unwrap() + 1);
            fault
                .observations
                .insert("recursive-pure-boundaries".to_owned(), boundary.to_string());
            reject(fault);
        }
        for name in [
            "source-batch-recursive-schemas.witness",
            "source-batch-recursive-values.witness",
        ] {
            let mut fault = receipt.clone();
            let mut witness: Value = serde_json::from_str(&fault.observations[name]).unwrap();
            witness["command"] = witness["smoke"].clone();
            fault
                .observations
                .insert(name.to_owned(), witness.to_string());
            reject(fault);
        }
        let mut fault = receipt.clone();
        fault
            .recursive
            .results
            .insert("post-trap-recovery".to_owned(), json!(8));
        reject(fault);
        let mut fault = receipt.clone();
        fault.recursive.results.insert(
            "trap-order".to_owned(),
            json!(["normalized_integer_overflow", "normalized_integer_division"]),
        );
        reject(fault);
        read(&receipt, root).expect("immutable actual original survives all in-memory mutations");
    }
}
