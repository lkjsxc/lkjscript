//! Literal public author/edit/transport/recovery witness. This driver observes IDs, never binds
//! members by enumerating a host identity table. Only package/base/plan anchors are substituted.

use super::*;
use serde_json::{Value, json};

fn library_selection(package: &Package) -> String {
    format!(
        "reference.package as=$library package={} package-revision={}\n",
        package.id, package.logical
    )
}

fn observe(
    context: &mut Context,
    package: &Package,
    kind: &str,
    name: &str,
) -> Result<String, DevError> {
    let records = context.cli(
        Some(&package.path),
        &["query", "owners", "--kind", kind],
        true,
    )?;
    let owners = records
        .iter()
        .filter(|record| {
            record.operation == "owner"
                && record_field(record, "name").is_ok_and(|observed| observed == name)
        })
        .collect::<Vec<_>>();
    require(
        owners.len() == 1,
        "named witness observer expected exactly one independently named owner",
    )?;
    record_field(owners[0], "id")
}

fn run(
    context: &mut Context,
    cwd: &Path,
    descriptor: &Path,
    arguments: &str,
    expected: Value,
) -> Result<(), DevError> {
    let command = context.receipt.commands.len();
    let records = context.cli_copied_at(
        cwd,
        &[
            "run",
            "--deployment",
            &descriptor.display().to_string(),
            "--arguments",
            arguments,
        ],
    )?;
    let mut commands: Vec<usize> = context
        .receipt
        .observations
        .get("named_reference_commands")
        .map(|value| serde_json::from_str(value))
        .transpose()?
        .unwrap_or_default();
    commands.push(command);
    context.receipt.observations.insert(
        "named_reference_commands".into(),
        serde_json::to_string(&commands)?,
    );
    let value: Value = serde_json::from_str(&field(&records, "execution", "value")?)?;
    require(
        value == expected,
        "named library command differs from its independent expected value",
    )
}

pub(super) fn validate(receipt: &Receipt, evidence_root: &Path) -> Result<Vec<usize>, DevError> {
    let commands: Vec<usize> = serde_json::from_str(
        receipt
            .observations
            .get("named_reference_commands")
            .ok_or_else(|| DevError::corrupt("named command inventory absent"))?,
    )?;
    require(
        commands.len() == 5 && commands.windows(2).all(|pair| pair[0] < pair[1]),
        "named command inventory is incomplete or duplicated",
    )?;
    for (index, (descriptor, arguments, expected)) in commands.iter().zip([
        ("main", "[41]", json!(42)),
        ("text", "[\"lkj\"]", json!("lkj")),
        ("repaired", "[41]", json!(42)),
        ("main", "[41]", json!(42)),
        ("text", "[\"lkj\"]", json!("lkj")),
    ]) {
        let command = receipt
            .commands
            .get(*index)
            .ok_or_else(|| DevError::corrupt("named command index is absent"))?;
        let root = Path::new(&receipt.isolated_root);
        require(
            command.command
                == vec![
                    root.join("lkjscript").display().to_string(),
                    "run".into(),
                    "--deployment".into(),
                    root.join("named-bundle")
                        .join(format!("{descriptor}.deployment.json"))
                        .display()
                        .to_string(),
                    "--arguments".into(),
                    arguments.into(),
                ]
                && command.expects_success,
            "named witness did not use the copied product and exact bundle arguments",
        )?;
        let records = parse_records(
            "named-execution",
            &process::read_bounded(
                &evidence_root.join(&command.observation.stdout.path),
                MAXIMUM_OUTPUT_BYTES,
            )?,
        )
        .map_err(|errors| DevError::corrupt(format!("named command output: {errors:?}")))?;
        require(
            serde_json::from_str::<Value>(&field(&records, "execution", "value")?)? == expected,
            "named execution output differs from its independent expected value",
        )?;
    }
    Ok(commands)
}

pub(super) fn workflow(context: &mut Context, standard: &Package) -> Result<(), DevError> {
    context.cli(None, &["capabilities", "--section", "change"], true)?;
    context.cli(
        None,
        &[
            "package", "builtin", "query", "owners", "--kind", "external", "--name", "add",
        ],
        true,
    )?;
    let mut library = context.new_package("named-cells")?;
    context.apply(&mut library, include_str!("named.producer.lkjc"))?;
    context.cli(Some(&library.path), &["check"], true)?;
    context.export(&mut library)?;
    let first_library_revision = library.logical.clone();
    let original_field = observe(context, &library, "field", "value")?;
    let mut consumer = context.new_package("named-consumer")?;
    context.apply(
        &mut consumer,
        "create.module as=$created-module name=application\n",
    )?;
    context.stage(&consumer, standard)?;
    context.stage(&consumer, &library)?;
    context.apply(
        &mut consumer,
        &format!(
            "{}{}{}{}",
            binding("add", standard),
            binding("add", &library),
            library_selection(&library),
            include_str!("named.consumer.lkjc")
        ),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    let function = observe(context, &consumer, "pure_function", "main")?;
    let parameters = context.cli(
        Some(&consumer.path),
        &["query", "owners", "--kind", "parameter"],
        true,
    )?;
    let initial_parameters = parameters
        .iter()
        .filter(|record| record.operation == "owner")
        .map(|record| record_field(record, "id"))
        .collect::<Result<Vec<_>, _>>()?;
    context.cli(
        Some(&consumer.path),
        &["inspect", "owner", "pure_function", &function],
        true,
    )?;
    let bundle = context.root.join("named-bundle");
    let cwd = context.root.join("named-unrelated");
    fs::create_dir(&bundle)?;
    fs::create_dir(&cwd)?;
    let artifact = bundle.join("original.lkja");
    context.cli(
        Some(&consumer.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    let original_artifact = digest_file(&artifact, MAXIMUM_CONTAINER_BYTES)?;
    fs::copy(&artifact, context.evidence.join("named-original.lkja"))?;
    let mut descriptors = Vec::new();
    for target in ["main", "text"] {
        let descriptor = bundle.join(format!("{target}.deployment.json"));
        fs::write(
            &descriptor,
            evidence::encode_json(
                &json!({"artifact":"original.lkja","target":target,"listen":null,"http":null,"session":null,"worker":null,"streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]}),
            )?,
        )?;
        fs::copy(
            &descriptor,
            context
                .evidence
                .join(format!("named-{target}.deployment.json")),
        )?;
        descriptors.push(descriptor);
    }
    context.apply(&mut consumer, include_str!("named.edit.lkjc"))?;
    require(
        observe(context, &consumer, "pure_function", "calculate")? == function,
        "same-request rename changed the selected existing function identity",
    )?;
    let parameters = context.cli(
        Some(&consumer.path),
        &["query", "owners", "--kind", "parameter"],
        true,
    )?;
    let after_parameters = parameters
        .iter()
        .filter(|record| record.operation == "owner")
        .map(|record| record_field(record, "id"))
        .collect::<Result<Vec<_>, _>>()?;
    require(
        initial_parameters == after_parameters,
        "named parameter edit changed parameter identities",
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    // Remove both authoring paths before using the transported artifact. Keep one owned producer
    // copy only for the explicitly selected later edit; it is unavailable at its original path.
    let producer_recovery = context.root.join("named-producer-recovery");
    fs::rename(&library.path, &producer_recovery)?;
    let consumer_recovery = context.root.join("named-consumer-recovery");
    fs::rename(&consumer.path, &consumer_recovery)?;
    run(context, &cwd, &descriptors[0], "[41]", json!(42))?;
    run(context, &cwd, &descriptors[1], "[\"lkj\"]", json!("lkj"))?;
    fs::rename(producer_recovery, &library.path)?;
    fs::rename(consumer_recovery, &consumer.path)?;
    context.apply(&mut library, include_str!("named.rename.lkjc"))?;
    require(
        observe(context, &library, "field", "contents")? == original_field,
        "producer field rename did not preserve identity",
    )?;
    context.export(&mut library)?;
    require(
        library.logical != first_library_revision,
        "exported member rename did not change the exact interface revision",
    )?;
    context.stage(&consumer, &library)?;
    let replacement = format!(
        "{}{}",
        binding("replace", &library),
        library_selection(&library)
    );
    let bad = context.evidence.join("named-old-member.lkjc");
    fs::write(
        &bad,
        format!(
            "request base={}\n{replacement}reference.owner as=$cell package=$library class=declaration name=cell\nreference.owner as=$old package=$library class=field name=value parent=$cell\n",
            consumer.revision
        ),
    )?;
    context.reject(
        &consumer,
        &["change", "plan", "--input-file", &bad.display().to_string()],
        "change_reference_not_exposed",
    )?;
    context.apply(
        &mut consumer,
        &format!("{replacement}{}", include_str!("named.repair.lkjc")),
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    let repaired = bundle.join("repaired.lkja");
    context.cli(
        Some(&consumer.path),
        &["build", "--output", &repaired.display().to_string()],
        true,
    )?;
    let repaired_descriptor = bundle.join("repaired.deployment.json");
    fs::write(
        &repaired_descriptor,
        evidence::encode_json(
            &json!({"artifact":"repaired.lkja","target":"main","listen":null,"http":null,"session":null,"worker":null,"streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]}),
        )?,
    )?;
    fs::remove_dir_all(&library.path)?;
    fs::remove_dir_all(&consumer.path)?;
    fs::remove_file(&library.container)?;
    run(context, &cwd, &repaired_descriptor, "[41]", json!(42))?;
    run(context, &cwd, &descriptors[0], "[41]", json!(42))?;
    run(context, &cwd, &descriptors[1], "[\"lkj\"]", json!("lkj"))?;
    require(
        digest_file(&artifact, MAXIMUM_CONTAINER_BYTES)? == original_artifact,
        "new dependency selection changed the frozen old artifact",
    )?;
    context.receipt.observations.insert("named_references".into(), "literal-prelude;I64=42;Text=lkj;parameter-and-rename-continuity;replacement-repaired;old-artifact-preserved".into());
    context.receipt.observations.insert("named_reference_identities".into(), serde_json::to_string(&json!({"producer":library.id,"before":first_library_revision,"after":library.logical,"field":original_field,"consumer":consumer.id,"function":function,"parameters":initial_parameters,"old_artifact":original_artifact,"repaired_artifact":digest_file(&repaired,MAXIMUM_CONTAINER_BYTES)?,"member_ids_in_authored_inputs":0}))?);
    Ok(())
}
