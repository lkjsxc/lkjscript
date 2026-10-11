//! Authentic Graph 14 read/edit, original-format refusal and current-envelope execution.
use super::*;
include!("requirements_predecessor_files.rs");
const REVISION: &str = "rev_2a50524afd4436be15e69c545fabf0f2eca056bb5b0f4e28459dfcf62ef816c4";
const PACKAGE: &str = "pkg_5ebe35b4968513b3ce06d3f7093d3963";
const LOGICAL: &str =
    "package_revision_9c46d3e276b3df89d29b792993283108017feabfc030d70f50a43ed05c8fb691";
const TRANSPORT: &str =
    "package_transport_51f58676d0a4bc944c78c97370d945fe2a7fc575709cf46b34dcac7c4aae2d1e";
const MATERIAL: &str = "requirement-predecessor";
const CURRENT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/owned-predecessor-compiler31/requirements.lkja"
));
const CASES: [(&str, &str); 7] = [
    ("predecessor.lkja", "encode-integer"),
    ("predecessor.lkja", "encode-text"),
    ("current.lkja", "encode-integer"),
    ("current.lkja", "encode-text"),
    ("rebuilt.lkja", "encode-integer"),
    ("rebuilt.lkja", "encode-text"),
    ("imported.lkja", "encode-integer"),
];

fn material_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{MATERIAL}--{}", name.replace('/', "--")))
}

fn descriptor(artifact: &str, target: &str) -> serde_json::Value {
    serde_json::json!({"artifact":artifact,"target":target,"listen":null,"http":null,"session":null,"worker":null,"streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]})
}

fn admitted_artifact(root: &Path, artifact: &str) -> Result<String, DevError> {
    let bytes = process::read_bounded(&material_path(root, artifact), MAXIMUM_CONTAINER_BYTES)?;
    if artifact == "current.lkja" {
        require(bytes == CURRENT, "scalar current fixture bytes changed")?;
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
            "scalar current artifact lost original source transport binding",
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
        "original scalar artifact did not stop at its compiler unit contract",
    )
}

fn current_result(
    output: &[CompactRecord],
    identity: &str,
    expected: &serde_json::Value,
    original_source: bool,
) -> Result<(), DevError> {
    if original_source {
        require(
            field(output, "execution", "package")? == PACKAGE
                && field(output, "execution", "revision")? == REVISION,
            "rebuilt scalar changed accepted source identity",
        )?;
    }
    require(
        field(output, "result", "status")? == "success"
            && field(output, "result", "command")? == "run"
            && field(output, "execution", "artifact")? == identity
            && serde_json::from_str::<serde_json::Value>(&field(output, "execution", "value")?)?
                == *expected
            && !output.iter().any(|record| record.operation == "diagnostic"),
        "authentic predecessor scalar execution identity or bytes differ",
    )
}

fn expected() -> Result<serde_json::Value, DevError> {
    let (_, bytes) = FILES
        .iter()
        .find(|(name, _)| *name == "expected.json")
        .ok_or_else(|| DevError::corrupt("frozen predecessor scalar expectations absent"))?;
    Ok(serde_json::from_slice(bytes)?)
}
fn preserved(root: &Path) -> Result<(), DevError> {
    for (name, bytes) in FILES {
        if name.starts_with("project/packs/") {
            require(
                fs::read(root.join(name))? == *bytes,
                "predecessor canonical object pack bytes changed",
            )?;
        }
    }
    Ok(())
}
pub(super) fn workflow(context: &mut Context) -> Result<(), DevError> {
    let root = context.root.join(MATERIAL);
    for (name, bytes) in FILES {
        let path = root.join(name);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| DevError::corrupt("predecessor file parent"))?,
        )?;
        fs::write(path, bytes)?;
        fs::write(material_path(&context.evidence, name), bytes)?;
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
        "predecessor read/check/build changed accepted HEAD",
    )?;
    preserved(&root)?;
    let mut importer = context.new_package("requirement-predecessor-importer")?;
    context.stage(&importer, &old)?;
    context.apply(
        &mut importer,
        &format!(
            "{}reference.package as=$old package={PACKAGE} package-revision={LOGICAL}\n{}",
            binding("add", &old),
            include_str!("requirements.predecessor-consumer.lkjc")
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
    for artifact in ["rebuilt.lkja", "imported.lkja"] {
        fs::copy(
            root.join(artifact),
            material_path(&context.evidence, artifact),
        )?;
    }
    let values = expected()?;
    let mut commands = Vec::new();
    for (artifact, target) in CASES {
        let original = artifact == "predecessor.lkja";
        let identity = if original {
            None
        } else {
            Some(admitted_artifact(&context.evidence, artifact)?)
        };
        let name = format!("{artifact}-{target}.json");
        let path = root.join(&name);
        let bytes = evidence::encode_json(&descriptor(artifact, target))?;
        fs::write(&path, &bytes)?;
        fs::write(material_path(&context.evidence, &name), bytes)?;
        let index = context.receipt.commands.len();
        commands.push(index);
        let output = context.cli(
            None,
            &["run", "--deployment", &path.display().to_string()],
            !original,
        )?;
        if let Some(identity) = identity {
            current_result(
                &output,
                &identity,
                &values[target],
                artifact != "imported.lkja",
            )?;
        } else {
            original_refusal(&output)?;
            require(
                context.receipt.commands[index].observation.exit_code == Some(2),
                "scalar original refusal exit differs",
            )?;
        }
    }
    let result = context.apply(&mut old,"reference.owner as=$module package=local class=module name=scalar-compatibility\nrename.owner owner=$module name=scalar-compatibility-edited\n")?;
    require(
        old.revision != REVISION,
        "ordinary predecessor edit retained the original revision",
    )?;
    context.cli(Some(&old.path), &["check"], true)?;
    preserved(&root)?;
    context.receipt.observations.insert("requirement_predecessor".into(),serde_json::json!({"before":REVISION,"after":old.revision,"original_authority":before.inventory_sha256,"original_packs_preserved":true,"receipt":field(&result,"receipt","digest")?,"commands":commands}).to_string());
    Ok(())
}
pub(super) fn validate(receipt: &Receipt, root: &Path) -> Result<Vec<usize>, DevError> {
    let value: serde_json::Value = serde_json::from_str(
        receipt
            .observations
            .get("requirement_predecessor")
            .ok_or_else(|| {
                DevError::corrupt("authentic requirement predecessor evidence missing")
            })?,
    )?;
    require(
        value["before"] == REVISION
            && value["after"] != REVISION
            && value["original_packs_preserved"] == true
            && value["original_authority"]
                .as_str()
                .is_some_and(|v| v.len() == 64)
            && value["receipt"]
                .as_str()
                .is_some_and(|v| v.starts_with("receipt_object_")),
        "predecessor authority/ordinary edit binding differs",
    )?;
    for (name, bytes) in FILES {
        require(
            process::read_bounded(&material_path(root, name), MAXIMUM_CONTAINER_BYTES)? == *bytes,
            "authentic scalar predecessor material changed",
        )?;
    }
    let commands: Vec<usize> = serde_json::from_value(value["commands"].clone())?;
    require(
        commands.len() == CASES.len() && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "predecessor original refusal/current/rebuilt/transport calls missing",
    )?;
    let values = expected()?;
    for (index, (artifact, target)) in commands.iter().zip(CASES) {
        let original = artifact == "predecessor.lkja";
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("predecessor command missing"))?;
        let path = Path::new(&receipt.isolated_root)
            .join(MATERIAL)
            .join(format!("{artifact}-{target}.json"));
        require(
            serde_json::from_slice::<serde_json::Value>(&process::read_bounded(
                &material_path(root, &format!("{artifact}-{target}.json")),
                MAXIMUM_OUTPUT_BYTES,
            )?)? == descriptor(artifact, target),
            "scalar predecessor exact descriptor changed",
        )?;
        require(
            command.command
                == [
                    receipt.pinned_runtime_path.clone(),
                    "run".into(),
                    "--deployment".into(),
                    path.display().to_string(),
                ]
                && command.expects_success != original
                && command.observation.exit_code == Some(if original { 2 } else { 0 }),
            "predecessor artifact/target changed",
        )?;
        let output = parse_records(
            "predecessor-output",
            &process::read_bounded(
                &root.join(format!("command-{index:04}.stdout")),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|_| DevError::corrupt("predecessor output"))?;
        if original {
            original_refusal(&output)?;
        } else {
            current_result(
                &output,
                &admitted_artifact(root, artifact)?,
                &values[target],
                artifact != "imported.lkja",
            )?;
        }
    }
    Ok(commands)
}

#[cfg(test)]
mod current_control_tests {
    use super::*;

    #[test]
    fn rebuilt_scalar_result_requires_original_semantic_revision() {
        let text = format!(
            "result status=success command=run\nexecution artifact=selected value=0 package={PACKAGE} revision={REVISION}\n"
        );
        let value = serde_json::json!(0);
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
    fn scalar_current_control_requires_exact_bytes_and_original_transport() {
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
        let historical = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/owned-predecessor-compiler30/requirements.lkja"
        ));
        let error = lkjscript::platform::contributor::strict_artifact_identity_probe(historical)
            .unwrap_err();
        assert!(error.to_string().contains("artifact_bundle_contract"));

        fs::write(
            material_path(root, "current.lkja"),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/fixtures/owned-predecessor-compiler31/transactions.lkja"
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
