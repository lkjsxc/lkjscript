//! Authentic v0.1.36 transaction source, transport and artifact compatibility.
use super::*;
include!("requirements_transaction_predecessor_files.rs");

const REVISION: &str = "rev_ece991392cca803d0b92145eecef1824d70c7e7af35d18f425da6da0bd596cb5";
const PACKAGE: &str = "pkg_12239e96293964646c643895dd684890";
const LOGICAL: &str =
    "package_revision_5b022b398e5c95fd56552b1a45dea764feb261b26f63fc65748a8fe98d4d6cf9";
const TRANSPORT: &str =
    "package_transport_3abb212b71ff3e56bdba07c0977bebffb1b8e336cc8092e982ab2398bb81a74d";
const MATERIAL: &str = "requirement-transaction-predecessor";

fn material_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{MATERIAL}--{}", name.replace('/', "--")))
}

fn expected() -> [(&'static str, &'static str, Value); 5] {
    [
        (
            "number-direct",
            "[\"direct\"]",
            json!({"candidate":13,"primary_condition_matched":true}),
        ),
        (
            "number-bound",
            "[\"bound\"]",
            json!({"candidate":13,"primary_condition_matched":true}),
        ),
        (
            "text-direct",
            "[\"direct\"]",
            json!({"candidate":"a!","primary_condition_matched":true}),
        ),
        ("seed-auxiliary", "[]", json!(true)),
        (
            "false-attempt",
            "[\"direct\"]",
            json!({"candidate":16,"primary_condition_matched":true}),
        ),
    ]
}

fn preserve_packs(root: &Path) -> Result<(), DevError> {
    for (name, bytes) in FILES {
        if name.starts_with("project/packs/") {
            require(
                fs::read(root.join(name))? == *bytes,
                "authentic transaction predecessor pack changed",
            )?;
        }
    }
    Ok(())
}

fn observe_expected(rows: &[Value]) -> Result<(), DevError> {
    store_observations(rows)?;
    require(
        rows.len() == 6,
        "predecessor transaction state observations omitted",
    )?;
    expect_cell(&rows[1], 0, "direct", Some(json!(13)))?;
    expect_cell(&rows[2], 0, "bound", Some(json!(13)))?;
    expect_cell(&rows[3], 1, "direct", Some(json!("a!")))?;
    expect_cell(&rows[4], 0, "aux", Some(json!(41)))?;
    require(
        rows[4] == rows[5],
        "legacy failed condition published data or HEAD",
    )?;
    expect_cell(&rows[5], 0, "direct", Some(json!(13)))?;
    Ok(())
}

pub(super) fn workflow(context: &mut Context) -> Result<(), DevError> {
    let root = context.root.join(MATERIAL);
    for (name, bytes) in FILES {
        for path in [root.join(name), material_path(&context.evidence, name)] {
            fs::create_dir_all(
                path.parent()
                    .ok_or_else(|| DevError::corrupt("predecessor parent"))?,
            )?;
            fs::write(path, bytes)?;
        }
    }
    let mut old = Package {
        path: root.join("project"),
        id: PACKAGE.into(),
        revision: REVISION.into(),
        logical: LOGICAL.into(),
        transport: TRANSPORT.into(),
        container: root.join("predecessor.lkjp"),
        symbols: BTreeMap::new(),
    };
    let before = crate::authority::observe_graph_authority(&old.path)?;
    context.cli(Some(&old.path), &["status"], true)?;
    context.cli(Some(&old.path), &["check"], true)?;
    context.cli(
        Some(&old.path),
        &[
            "build",
            "--output",
            &root.join("rebuilt.lkja").display().to_string(),
        ],
        true,
    )?;
    require(
        crate::authority::observe_graph_authority(&old.path)?.head_sha256 == before.head_sha256,
        "old transaction check/build changed accepted HEAD",
    )?;
    preserve_packs(&root)?;

    let mut importer = context.new_package("requirement-transaction-predecessor-importer")?;
    context.stage(&importer, &old)?;
    context.apply(
        &mut importer,
        &format!(
            "{}reference.package as=$old package={PACKAGE} package-revision={LOGICAL}\n{}",
            binding("add", &old),
            include_str!("requirements.transaction-predecessor-consumer.lkjc")
        ),
    )?;
    context.cli(Some(&importer.path), &["check"], true)?;
    context.cli(
        Some(&importer.path),
        &[
            "build",
            "--output",
            &root.join("imported.lkja").display().to_string(),
        ],
        true,
    )?;
    fs::remove_dir_all(&importer.path)?;

    let mut commands = Vec::new();
    for artifact in ["predecessor.lkja", "rebuilt.lkja", "imported.lkja"] {
        let bundle = root.join(artifact.trim_end_matches(".lkja"));
        fs::create_dir(&bundle)?;
        fs::copy(root.join(artifact), bundle.join(artifact))?;
        if artifact != "predecessor.lkja" {
            fs::copy(
                root.join(artifact),
                material_path(&context.evidence, artifact),
            )?;
        }
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
        let mut states = vec![observe(&bundle)?];
        for (target, arguments, value) in expected() {
            let descriptor = deployment(&bundle, target, artifact)?;
            fs::copy(
                &descriptor,
                material_path(
                    &context.evidence,
                    &format!("{artifact}-{target}.deployment.json"),
                ),
            )?;
            commands.push(context.receipt.commands.len());
            let output = context.cli(
                None,
                &[
                    "run",
                    "--deployment",
                    &descriptor.display().to_string(),
                    "--arguments",
                    arguments,
                ],
                true,
            )?;
            require(
                serde_json::from_str::<Value>(&field(&output, "execution", "value")?)? == value,
                "authentic transaction predecessor result differs",
            )?;
            states.push(observe(&bundle)?);
        }
        observe_expected(&states)?;
        fs::write(
            material_path(&context.evidence, &format!("{artifact}.states.json")),
            evidence::encode_json(&states)?,
        )?;
    }
    let edit = context.apply(&mut old,
        "reference.owner as=$module package=local class=module name=cell-consumer\nrename.owner owner=$module name=cell-consumer-edited\n")?;
    require(
        old.revision != REVISION,
        "ordinary old transaction edit retained revision",
    )?;
    context.cli(Some(&old.path), &["check"], true)?;
    preserve_packs(&root)?;
    context.receipt.observations.insert("requirement_transaction_predecessor".into(),
        json!({"before":REVISION,"after":old.revision,"original_authority":before.inventory_sha256,
            "original_packs_preserved":true,"receipt":field(&edit,"receipt","digest")?,"commands":commands}).to_string());
    Ok(())
}

pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let value: Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_transaction_predecessor")
            .ok_or_else(|| DevError::corrupt("authentic v0.1.36 transaction evidence missing"))?,
    )?;
    require(
        value["before"] == REVISION
            && value["after"] != REVISION
            && value["original_packs_preserved"] == true
            && value["original_authority"]
                .as_str()
                .is_some_and(|s| s.len() == 64)
            && value["receipt"]
                .as_str()
                .is_some_and(|s| s.starts_with("receipt_object_")),
        "authentic transaction predecessor authority or edit binding differs",
    )?;
    for (name, bytes) in FILES {
        require(
            process::read_bounded(&material_path(root, name), MAXIMUM_CONTAINER_BYTES)? == *bytes,
            "authentic transaction predecessor material changed",
        )?;
    }
    let commands: Vec<usize> = serde_json::from_value(value["commands"].clone())?;
    require(
        commands.len() == 15 && commands.windows(2).all(|w| w[0] < w[1]),
        "authentic transaction original/rebuilt/imported executions omitted",
    )?;
    for (artifact, indices) in ["predecessor.lkja", "rebuilt.lkja", "imported.lkja"]
        .into_iter()
        .zip(commands.as_chunks::<5>().0.iter())
    {
        let artifact_bytes =
            process::read_bounded(&material_path(root, artifact), MAXIMUM_CONTAINER_BYTES)?;
        let admitted =
            lkjscript::platform::contributor::strict_artifact_identity_probe(&artifact_bytes)
                .map_err(|error| DevError::corrupt(error.to_string()))?;
        let states: Vec<Value> = serde_json::from_slice(&process::read_bounded(
            &material_path(root, &format!("{artifact}.states.json")),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        observe_expected(&states)?;
        for (index, (target, arguments, expected)) in indices.iter().zip(expected()) {
            require(
                serde_json::from_slice::<Value>(&process::read_bounded(
                    &material_path(root, &format!("{artifact}-{target}.deployment.json")),
                    MAXIMUM_OUTPUT_BYTES,
                )?)? == deployment_value(target, artifact),
                "authentic predecessor descriptor changed exact target or grants",
            )?;
            let command = receipt
                .commands
                .get(*index)
                .ok_or_else(|| DevError::corrupt("predecessor execution absent"))?;
            let descriptor = Path::new(&receipt.isolated_root)
                .join(MATERIAL)
                .join(artifact.trim_end_matches(".lkja"))
                .join(format!("{artifact}-{target}.deployment.json"));
            require(
                command.expects_success
                    && command.command
                        == [
                            receipt.pinned_runtime_path.clone(),
                            "run".into(),
                            "--deployment".into(),
                            descriptor.display().to_string(),
                            "--arguments".into(),
                            arguments.into(),
                        ],
                "authentic transaction invocation changed exact runtime, artifact or arguments",
            )?;
            let output = parse_records(
                "transaction-predecessor-output",
                &process::read_bounded(
                    &root.join(format!("command-{index:04}.stdout")),
                    MAXIMUM_OUTPUT_BYTES,
                )?,
            )
            .map_err(|_| DevError::corrupt("transaction predecessor execution output"))?;
            require(
                serde_json::from_str::<Value>(&field(&output, "execution", "value")?)? == expected
                    && field(&output, "execution", "artifact")? == admitted,
                "predecessor UpdateAttempt or legacy completion behavior changed",
            )?;
        }
    }
    Ok(commands)
}
