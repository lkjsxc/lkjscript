//! Observer-only extension of the transported library workload, using the existing queue adapter.
use super::*;
use lkjscript::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use serde_json::{Value, json};

fn observe(bundle: &Path) -> Result<Value, DevError> {
    let store = DataStore::open(
        &bundle.join("queue"),
        "requirement-queue",
        DataLimits::default(),
    )
    .map_err(|e| DevError::corrupt(e.to_string()))?;
    let transaction = store
        .begin()
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    let key = DataKey::new(vec![DataKeyPart::Text("job".into())], store.limits())
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    let entry = transaction
        .get("__queue.jobs", &key)
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    match entry {
        None => Ok(Value::Null),
        Some(entry) => Ok(json!({"bytes":entry.value,"revision":entry.revision,
            "job":crate::service::observe_queue_record(&entry.value)?})),
    }
}

fn cases() -> [(&'static str, bool, u64); 3] {
    [
        ("queue-seed", true, 2),
        ("queue-finish", true, 3),
        ("queue-finish", false, 1),
    ]
}

pub(super) fn workflow(context: &mut Context, bundle: &Path) -> Result<(), DevError> {
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &bundle.join("queue").display().to_string(),
        ],
        true,
    )?;
    let mut rows = vec![observe(bundle)?];
    let mut commands = Vec::new();
    for (target, expected, _) in cases() {
        let descriptor = bundle.join(format!("consumer.lkja-{target}.deployment.json"));
        let value = json!({
            "artifact":"consumer.lkja","target":target,"listen":null,"http":null,"session":null,"worker":null,
            "streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],
            "grants":[{"requirement":"jobs","sharing_domain":"requirement-queue","authority_revision":"b1".repeat(32),
                "adapter":{"kind":"durable_queue_data","root":"queue","namespace":"requirement-queue",
                    "data_limits":DataLimits::default(),"limits":lkjscript::platform::queue::QueueLimits::default()}}]
        });
        fs::write(&descriptor, evidence::encode_json(&value)?)?;
        fs::copy(
            &descriptor,
            context
                .evidence
                .join(format!("requirement-{target}.deployment.json")),
        )?;
        commands.push(context.receipt.commands.len());
        let result = context.cli(
            None,
            &[
                "run",
                "--deployment",
                &descriptor.display().to_string(),
                "--arguments",
                "[]",
            ],
            true,
        )?;
        require(
            serde_json::from_str::<Value>(&field(&result, "execution", "value")?)? == expected,
            "transported queue library returned an unexpected transition result",
        )?;
        rows.push(observe(bundle)?);
        fs::write(
            context.evidence.join("requirement-queue-observations.json"),
            evidence::encode_json(&rows)?,
        )?;
    }
    validate_rows(&rows)?;
    context.receipt.observations.insert(
        "requirement_queue".into(),
        serde_json::to_string(&commands)?,
    );
    Ok(())
}

fn validate_rows(rows: &[Value]) -> Result<(), DevError> {
    require(
        rows.len() == 4 && rows[0].is_null(),
        "queue resource initial absence missing",
    )?;
    for row in &rows[1..] {
        let bytes: Vec<u8> = serde_json::from_value(row["bytes"].clone())?;
        require(
            row["job"] == crate::service::observe_queue_record(&bytes)?,
            "queue independent wire observation changed",
        )?;
    }
    require(
        rows[1]["job"]
            == json!({"job":"job","state":"ready","attempts":0,"attempt":null,
        "worker":null,"lease_until":null,"error":null,"result":null})
            && rows[2]["job"]
                == json!({"job":"job","state":"completed","attempts":1,"attempt":null,
        "worker":null,"lease_until":null,"error":null,"result":b"done".to_vec()})
            && rows[2] == rows[3],
        "queue resource acquisition, consumption, or absent retry changed persistence",
    )
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let commands: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("requirement_queue")
            .ok_or_else(|| {
                DevError::corrupt("transported requirement resource evidence missing")
            })?,
    )?;
    require(
        commands.len() == 3 && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "queue resource commands missing or reordered",
    )?;
    let rows: Vec<Value> = serde_json::from_slice(&process::read_bounded(
        &root.join("requirement-queue-observations.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    validate_rows(&rows)?;
    for (index, (target, expected, calls)) in commands.iter().zip(cases()) {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("queue resource command index"))?;
        let descriptor = Path::new(&receipt.isolated_root)
            .join("requirement-bundle")
            .join(format!("consumer.lkja-{target}.deployment.json"));
        require(
            command.command
                == [
                    receipt.pinned_runtime_path.clone(),
                    "run".into(),
                    "--deployment".into(),
                    descriptor.display().to_string(),
                    "--arguments".into(),
                    "[]".into(),
                ]
                && command.expects_success,
            "queue resource invocation changed executable, artifact, or arguments",
        )?;
        let output = parse_records(
            "queue-resource",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("queue resource output"))?;
        let work: Value =
            serde_json::from_str(&field(&output, "execution", "production-observation")?)?;
        let cleanup: Value = serde_json::from_str(&field(&output, "execution", "cleanup")?)?;
        require(
            serde_json::from_str::<Value>(&field(&output, "execution", "value")?)? == expected
                && work["capability_calls"] == calls
                && cleanup["remaining_tasks"] == 0
                && cleanup["cleanup_failures"] == json!([]),
            "queue resource result, calls, or joined cleanup differs",
        )?;
        for name in [
            "live_call_frames_after",
            "live_handles_after",
            "live_locals_after",
            "live_operands_after",
            "live_transactions_after",
            "live_type_bindings_after",
        ] {
            require(work[name] == 0, "queue library leaked owned runtime state")?;
        }
    }
    Ok(commands)
}
