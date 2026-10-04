//! Authentic v0.1.36 transaction source, original refusal and current-envelope execution.
use super::*;
include!("requirements_transaction_predecessor_files.rs");

const REVISION: &str = "rev_ece991392cca803d0b92145eecef1824d70c7e7af35d18f425da6da0bd596cb5";
const PACKAGE: &str = "pkg_12239e96293964646c643895dd684890";
const LOGICAL: &str =
    "package_revision_5b022b398e5c95fd56552b1a45dea764feb261b26f63fc65748a8fe98d4d6cf9";
const TRANSPORT: &str =
    "package_transport_3abb212b71ff3e56bdba07c0977bebffb1b8e336cc8092e982ab2398bb81a74d";
const MATERIAL: &str = "requirement-transaction-predecessor";
const CURRENT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/owned-predecessor-compiler25/transactions.lkja"
));
const ARTIFACTS: [&str; 4] = [
    "predecessor.lkja",
    "current.lkja",
    "rebuilt.lkja",
    "imported.lkja",
];

fn material_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{MATERIAL}--{}", name.replace('/', "--")))
}

fn admitted_artifact(root: &Path, artifact: &str) -> Result<String, DevError> {
    let bytes = process::read_bounded(&material_path(root, artifact), MAXIMUM_CONTAINER_BYTES)?;
    if artifact == "current.lkja" {
        require(
            bytes == CURRENT,
            "transaction current fixture bytes changed",
        )?;
    }
    let identity = lkjscript::platform::contributor::strict_artifact_identity_probe(&bytes)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    // A public rebuild may project a new interface/package revision while
    // accepted source remains fixed. Only the mechanical control keeps the
    // exact original transport; current_result checks rebuilt source identity.
    if artifact == "current.lkja" {
        let source = process::read_bounded(
            &material_path(root, "predecessor.lkjp"),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        let bound = lkjscript::platform::contributor::strict_artifact_source_probe(
            &bytes, &source, TRANSPORT,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            bound["bundle"] == identity
                && bound["source_transport"] == TRANSPORT
                && bound["package"] == PACKAGE
                && bound["revision"] == REVISION
                && bound["package_revision"] == LOGICAL,
            "transaction current artifact lost original source transport binding",
        )?;
    }
    Ok(identity)
}

fn original_refusal(output: &[CompactRecord]) -> Result<(), DevError> {
    require(
        field(output, "result", "status")? == "failure"
            && field(output, "result", "command")? == "run"
            && field(output, "diagnostic", "class")? == "source"
            && field(output, "diagnostic", "code")? == "compiler_unit_contract"
            && output
                .iter()
                .filter(|record| record.operation == "diagnostic")
                .count()
                == 1
            && !output.iter().any(|record| record.operation == "execution"),
        "original transaction artifact did not stop at its compiler unit contract",
    )
}

fn current_result(
    output: &[CompactRecord],
    identity: &str,
    expected: &Value,
    original_source: bool,
) -> Result<(), DevError> {
    if original_source {
        require(
            field(output, "execution", "package")? == PACKAGE
                && field(output, "execution", "revision")? == REVISION,
            "rebuilt transaction changed accepted source identity",
        )?;
    }
    require(
        field(output, "result", "status")? == "success"
            && field(output, "result", "command")? == "run"
            && field(output, "execution", "artifact")? == identity
            && serde_json::from_str::<Value>(&field(output, "execution", "value")?)? == *expected
            && !output.iter().any(|record| record.operation == "diagnostic"),
        "predecessor UpdateAttempt or legacy completion behavior changed",
    )
}

fn observe_refused(rows: &[Value]) -> Result<(), DevError> {
    store_observations(rows)?;
    require(
        rows.len() == 6 && rows.iter().all(|row| row == &rows[0]),
        "original transaction refusal changed its initial stores or omitted snapshots",
    )?;
    for store in 0..2 {
        for key in CELL_KEYS {
            expect_cell(&rows[0], store, key, None)?;
        }
    }
    Ok(())
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
    fs::write(root.join("current.lkja"), CURRENT)?;
    fs::write(material_path(&context.evidence, "current.lkja"), CURRENT)?;
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
    for artifact in ARTIFACTS {
        let original = artifact == "predecessor.lkja";
        let bundle = root.join(artifact.trim_end_matches(".lkja"));
        fs::create_dir(&bundle)?;
        fs::copy(root.join(artifact), bundle.join(artifact))?;
        if artifact != "predecessor.lkja" {
            fs::copy(
                root.join(artifact),
                material_path(&context.evidence, artifact),
            )?;
        }
        // Historical identity is owned by exact retained bytes/provenance and refusal,
        // never by loading an old artifact under the current contract.
        let identity = if original {
            None
        } else {
            Some(admitted_artifact(&context.evidence, artifact)?)
        };
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
            let index = context.receipt.commands.len();
            commands.push(index);
            let output = context.cli(
                None,
                &[
                    "run",
                    "--deployment",
                    &descriptor.display().to_string(),
                    "--arguments",
                    arguments,
                ],
                !original,
            )?;
            if let Some(identity) = &identity {
                current_result(&output, identity, &value, artifact != "imported.lkja")?;
            } else {
                original_refusal(&output)?;
                require(
                    context.receipt.commands[index].observation.exit_code == Some(2),
                    "transaction original refusal exit differs",
                )?;
            }
            states.push(observe(&bundle)?);
        }
        if original {
            observe_refused(&states)?;
        } else {
            observe_expected(&states)?;
        }
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
        commands.len() == 20 && commands.windows(2).all(|w| w[0] < w[1]),
        "authentic transaction original refusal/current/rebuilt/imported calls omitted",
    )?;
    for (artifact, indices) in ARTIFACTS
        .into_iter()
        .zip(commands.as_chunks::<5>().0.iter())
    {
        let original = artifact == "predecessor.lkja";
        let admitted = if original {
            None
        } else {
            Some(admitted_artifact(root, artifact)?)
        };
        let states: Vec<Value> = serde_json::from_slice(&process::read_bounded(
            &material_path(root, &format!("{artifact}.states.json")),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        if original {
            observe_refused(&states)?;
        } else {
            observe_expected(&states)?;
        }
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
                command.expects_success != original
                    && command.observation.exit_code == Some(if original { 2 } else { 0 })
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
            if let Some(admitted) = &admitted {
                current_result(&output, admitted, &expected, artifact != "imported.lkja")?;
            } else {
                original_refusal(&output)?;
            }
        }
    }
    Ok(commands)
}

#[cfg(test)]
mod current_control_tests {
    use super::*;

    #[test]
    fn rebuilt_transaction_result_requires_original_semantic_revision() {
        let text = format!(
            "result status=success command=run\nexecution artifact=selected value=0 package={PACKAGE} revision={REVISION}\n"
        );
        let value = json!(0);
        current_result(
            &parse_records("valid", text.as_bytes()).unwrap(),
            "selected",
            &value,
            true,
        )
        .unwrap();
        let changed = text.replace(REVISION, "different-revision");
        assert!(
            current_result(
                &parse_records("changed", changed.as_bytes()).unwrap(),
                "selected",
                &value,
                true
            )
            .is_err()
        );
        let changed = text.replace(PACKAGE, "different-package");
        assert!(
            current_result(
                &parse_records("changed", changed.as_bytes()).unwrap(),
                "selected",
                &value,
                true
            )
            .is_err()
        );
        assert!(
            current_result(
                &parse_records("wrong-artifact", text.as_bytes()).unwrap(),
                "different-artifact",
                &value,
                false
            )
            .is_err()
        );
    }

    #[test]
    fn transaction_current_control_requires_exact_bytes_and_original_transport() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let source = FILES
            .iter()
            .find(|(name, _)| *name == "predecessor.lkjp")
            .unwrap()
            .1;
        fs::write(material_path(root, "predecessor.lkjp"), source).unwrap();
        fs::write(material_path(root, "current.lkja"), CURRENT).unwrap();
        admitted_artifact(root, "current.lkja").unwrap();
        fs::write(
            material_path(root, "current.lkja"),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/fixtures/owned-predecessor-compiler25/requirements.lkja"
            )),
        )
        .unwrap();
        assert!(admitted_artifact(root, "current.lkja").is_err());
        fs::remove_file(material_path(root, "current.lkja")).unwrap();
        assert!(admitted_artifact(root, "current.lkja").is_err());
        fs::write(material_path(root, "current.lkja"), CURRENT).unwrap();
        fs::remove_file(material_path(root, "predecessor.lkjp")).unwrap();
        assert!(admitted_artifact(root, "current.lkja").is_err());
        let refusal = b"result status=failure command=run\ndiagnostic class=source code=compiler_unit_contract message=unsupported\n";
        original_refusal(&parse_records("refusal", refusal).unwrap()).unwrap();
        let later = b"result status=failure command=run\ndiagnostic class=capability code=secret_missing message=missing\n";
        assert!(original_refusal(&parse_records("later", later).unwrap()).is_err());
    }
}
