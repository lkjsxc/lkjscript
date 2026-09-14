//! Strict authentic Graph 14 read/run/edit through the existing public package workflow.
use super::*;
include!("requirements_predecessor_files.rs");
const REVISION: &str = "rev_2a50524afd4436be15e69c545fabf0f2eca056bb5b0f4e28459dfcf62ef816c4";
const PACKAGE: &str = "pkg_5ebe35b4968513b3ce06d3f7093d3963";
const LOGICAL: &str =
    "package_revision_9c46d3e276b3df89d29b792993283108017feabfc030d70f50a43ed05c8fb691";
const TRANSPORT: &str =
    "package_transport_51f58676d0a4bc944c78c97370d945fe2a7fc575709cf46b34dcac7c4aae2d1e";
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
    let root = context.root.join("requirement-predecessor");
    for (name, bytes) in FILES {
        let path = root.join(name);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| DevError::corrupt("predecessor file parent"))?,
        )?;
        fs::write(path, bytes)?;
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
    let values = expected()?;
    let mut commands = Vec::new();
    for (artifact, target) in [
        ("predecessor.lkja", "encode-integer"),
        ("predecessor.lkja", "encode-text"),
        ("rebuilt.lkja", "encode-integer"),
        ("rebuilt.lkja", "encode-text"),
        ("imported.lkja", "encode-integer"),
    ] {
        let path = root.join(format!("{artifact}-{target}.json"));
        fs::write(
            &path,
            evidence::encode_json(
                &serde_json::json!({"artifact":artifact,"target":target,"listen":null,"http":null,"session":null,"worker":null,"streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]}),
            )?,
        )?;
        commands.push(context.receipt.commands.len());
        let output = context.cli(
            None,
            &["run", "--deployment", &path.display().to_string()],
            true,
        )?;
        require(
            serde_json::from_str::<serde_json::Value>(&field(&output, "execution", "value")?)?
                == values[target],
            "authentic predecessor scalar bytes differ",
        )?;
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
    let commands: Vec<usize> = serde_json::from_value(value["commands"].clone())?;
    require(
        commands.len() == 5 && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "predecessor original/rebuilt/transport executions missing",
    )?;
    let values = expected()?;
    for (index, (artifact, target)) in commands.iter().zip([
        ("predecessor.lkja", "encode-integer"),
        ("predecessor.lkja", "encode-text"),
        ("rebuilt.lkja", "encode-integer"),
        ("rebuilt.lkja", "encode-text"),
        ("imported.lkja", "encode-integer"),
    ]) {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("predecessor command missing"))?;
        let path = Path::new(&receipt.isolated_root)
            .join("requirement-predecessor")
            .join(format!("{artifact}-{target}.json"));
        require(
            command.command
                == [
                    receipt.pinned_runtime_path.clone(),
                    "run".into(),
                    "--deployment".into(),
                    path.display().to_string(),
                ]
                && command.expects_success,
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
        require(
            serde_json::from_str::<serde_json::Value>(&field(&output, "execution", "value")?)?
                == values[target],
            "predecessor frozen typed-data bytes changed",
        )?;
    }
    Ok(commands)
}
