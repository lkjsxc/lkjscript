use super::*;
use lkjscript::platform::control::CompactRecord;

const TIMEOUT: Duration = Duration::from_secs(120);
const COMMANDS: [&str; 7] = [
    "capabilities",
    "new",
    "status",
    "check",
    "build",
    "run",
    "status-final",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    name: String,
    command: Vec<String>,
    process: Option<process::ProcessObservation>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lifecycle {
    route: Route,
    pub(super) status: Status,
    disposition: Disposition,
    candidate: FileBinding,
    pub(super) installation: Option<installation::Installation>,
    runtime: String,
    pub(super) commands: Vec<Command>,
    revision: Option<String>,
    artifact: Option<FileBinding>,
    value: Option<String>,
    pub(super) started_unix_nanoseconds: u128,
    pub(super) completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    cleanup_nanoseconds: u64,
    pub(super) cleanup_complete: bool,
    failure: Option<String>,
}
fn commands(options: &PairOptions, route: Route) -> Vec<Command> {
    let candidate = installation::candidate(options, route)
        .display()
        .to_string();
    let project = route
        .root(options)
        .join("runtime/project")
        .display()
        .to_string();
    let artifact = route
        .root(options)
        .join("artifact.lkja")
        .display()
        .to_string();
    [
        vec!["capabilities".to_owned()],
        vec![
            "new".to_owned(),
            project.clone(),
            "--template".to_owned(),
            "command".to_owned(),
            "--name".to_owned(),
            "hello".to_owned(),
        ],
        vec!["--project".to_owned(), project.clone(), "status".to_owned()],
        vec!["--project".to_owned(), project.clone(), "check".to_owned()],
        vec![
            "--project".to_owned(),
            project.clone(),
            "build".to_owned(),
            "--output".to_owned(),
            artifact,
        ],
        vec![
            "--project".to_owned(),
            project.clone(),
            "run".to_owned(),
            "main".to_owned(),
        ],
        vec!["--project".to_owned(), project, "status".to_owned()],
    ]
    .into_iter()
    .zip(COMMANDS)
    .map(|(args, name)| Command {
        name: name.to_owned(),
        command: std::iter::once(candidate.clone()).chain(args).collect(),
        process: None,
    })
    .collect()
}

pub(super) fn run(
    options: &PairOptions,
    route: Route,
    admitted: &archive::VerifiedArchive,
    control: &process::ProcessControl,
) -> Result<Lifecycle, DevError> {
    let manifest = &admitted.manifest;
    let root = route.root(options);
    let runtime = root.join("runtime");
    fs::create_dir(&runtime)?;
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700))?;
    let started = Instant::now();
    let mut receipt = Lifecycle {
        route,
        status: Status::NotRun,
        disposition: Disposition::FreshExecution,
        candidate: binding(&route.extraction(options).join("lkjscript"))?,
        installation: None,
        runtime: runtime.display().to_string(),
        commands: commands(options, route),
        revision: None,
        artifact: None,
        value: None,
        started_unix_nanoseconds: super::super::super::unix_nanoseconds()?,
        completed_unix_nanoseconds: 0,
        elapsed_nanoseconds: 0,
        cleanup_nanoseconds: 0,
        cleanup_complete: false,
        failure: None,
    };
    let path = root.join("lifecycle.json");
    evidence::publish_json(&path, &receipt)?;
    let result = (|| {
        receipt.installation = Some(installation::run(options, route, admitted, control)?);
        evidence::publish_json(&path, &receipt)?;
        let mut head = None;
        for (index, name) in COMMANDS.iter().enumerate() {
            require(!control.cancelled(), "route lifecycle cancelled")?;
            let mut environment = super::environment()?.candidate_environment;
            environment.insert("TMPDIR".to_owned(), runtime.display().to_string());
            let spec = process::ProcessSpec {
                command: receipt.commands[index].command.clone(),
                cwd: runtime.clone(),
                environment,
                timeout: TIMEOUT,
                maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
                maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
                stdout_path: root.join(format!("{index:02}-{name}.stdout.log")),
                stderr_path: root.join(format!("{index:02}-{name}.stderr.log")),
                unavailable_exit_code: None,
            };
            #[cfg(test)]
            let spec =
                super::tests::process_hook(&format!("{}-{name}", route.name()), spec, control);
            receipt.commands[index].process =
                Some(process::run_supervised(&spec, &root, Some(control)));
            evidence::publish_json(&path, &receipt)?;
            let records = read_command(&root, index, &receipt.commands[index])?;
            match *name {
                "capabilities" => capabilities(&records, manifest)?,
                "new" => {
                    require(
                        value(&records, "result", "status")? == "success",
                        "route new failed",
                    )?;
                    receipt.revision = Some(value(&records, "revision", "id")?.to_owned());
                    head = Some(process::read_bounded(&runtime.join("project/HEAD"), 4096)?);
                }
                "status" | "status-final" => require(
                    Some(value(&records, "revision", "id")?) == receipt.revision.as_deref(),
                    "route graph revision changed",
                )?,
                "check" => checked(&records)?,
                "build" => {
                    require(
                        value(&records, "output", "visibility")? == "created",
                        "route artifact was not created",
                    )?;
                    let artifact = binding(&root.join("artifact.lkja"))?;
                    require(
                        artifact.file.byte_length > 0
                            && artifact.file.byte_length <= 128 * 1024 * 1024,
                        "route artifact absent or exceeds bound",
                    )?;
                    receipt.artifact = Some(artifact);
                }
                "run" => {
                    hello(&records)?;
                    receipt.value = Some(value(&records, "execution", "value")?.to_owned());
                }
                _ => return Err(DevError::corrupt("unknown lifecycle operation")),
            }
            if let Some(head) = &head {
                require(
                    process::read_bounded(&runtime.join("project/HEAD"), 4096)? == *head,
                    "route read-only lifecycle changed accepted HEAD",
                )?;
            }
        }
        require(!control.cancelled(), "route lifecycle cancelled")
    })();
    let cleanup_started = Instant::now();
    let cleanup =
        directory(&runtime).and_then(|()| fs::remove_dir_all(&runtime).map_err(DevError::from));
    receipt.cleanup_nanoseconds = elapsed(cleanup_started)?;
    receipt.cleanup_complete = cleanup.is_ok() && !runtime.try_exists()?;
    receipt.completed_unix_nanoseconds = super::super::super::unix_nanoseconds()?;
    receipt.elapsed_nanoseconds = elapsed(started)?;
    match result.and(cleanup) {
        Ok(()) => receipt.status = Status::FreshPassed,
        Err(error) => {
            receipt.status = Status::Failed;
            receipt.failure = Some(error.to_string());
        }
    }
    evidence::publish_json(&path, &receipt)?;
    Ok(receipt)
}

pub(super) fn validate(
    options: &PairOptions,
    route: Route,
    admitted: &archive::VerifiedArchive,
    receipt: &Lifecycle,
) -> Result<(), DevError> {
    let manifest = &admitted.manifest;
    let root = route.root(options);
    let path = root.join("lifecycle.json");
    installation::validate(
        options,
        route,
        admitted,
        receipt
            .installation
            .as_ref()
            .ok_or_else(|| DevError::corrupt("installation observation missing"))?,
    )?;
    regular(&path)?;
    require(
        process::read_bounded(&path, MAXIMUM_RECEIPT_BYTES)? == evidence::encode_json(receipt)?,
        "route lifecycle source receipt changed",
    )?;
    require(
        receipt.route == route
            && receipt.status == Status::FreshPassed
            && receipt.disposition == Disposition::FreshExecution
            && receipt.cleanup_complete
            && receipt.failure.is_none()
            && receipt.runtime == root.join("runtime").display().to_string()
            && !root.join("runtime").try_exists()?
            && receipt.candidate == binding(&route.extraction(options).join("lkjscript"))?
            && receipt.started_unix_nanoseconds > 0
            && receipt.completed_unix_nanoseconds >= receipt.started_unix_nanoseconds,
        "route lifecycle is foreign, incomplete or unclean",
    )?;
    let expected = commands(options, route);
    require(
        receipt.commands.len() == expected.len(),
        "route command omitted or extra",
    )?;
    let mut revision = None;
    for (index, (observed, expected)) in receipt.commands.iter().zip(expected).enumerate() {
        require(
            observed.name == expected.name && observed.command == expected.command,
            "route command was skipped, duplicated or redirected",
        )?;
        let records = read_command(&root, index, observed)?;
        match observed.name.as_str() {
            "capabilities" => capabilities(&records, manifest)?,
            "new" => {
                revision = Some(value(&records, "revision", "id")?.to_owned());
                require(
                    revision == receipt.revision,
                    "route revision evidence changed",
                )?;
            }
            "status" | "status-final" => require(
                Some(value(&records, "revision", "id")?) == revision.as_deref(),
                "route accepted graph is unhealthy or changed",
            )?,
            "check" => checked(&records)?,
            "build" => {
                let artifact = binding(&root.join("artifact.lkja"))?;
                require(
                    value(&records, "output", "visibility")? == "created"
                        && artifact.file.byte_length > 0
                        && Some(artifact) == receipt.artifact,
                    "route artifact observation changed",
                )?;
            }
            "run" => {
                hello(&records)?;
                require(
                    receipt.value.as_deref() == Some(value(&records, "execution", "value")?),
                    "route value observation changed",
                )?;
            }
            _ => return Err(DevError::corrupt("unknown lifecycle operation")),
        }
    }
    Ok(())
}
fn read_command(
    root: &Path,
    index: usize,
    command: &Command,
) -> Result<Vec<CompactRecord>, DevError> {
    let p = command
        .process
        .as_ref()
        .ok_or_else(|| DevError::corrupt("route process result missing"))?;
    require(
        p.status == process::ProcessStatus::Passed
            && p.exit_code == Some(0)
            && p.signal.is_none()
            && p.reason.is_none()
            && !p.stdout_limit_exhausted
            && !p.stderr_limit_exhausted
            && p.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && p.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES,
        "route process failed, cancelled, exhausted or unclean",
    )?;
    for (proof, suffix) in [(&p.stdout, "stdout.log"), (&p.stderr, "stderr.log")] {
        let name = format!("{index:02}-{}.{suffix}", command.name);
        regular(&root.join(&name))?;
        require(
            proof.path == name && *proof == evidence::proof(&root.join(&name), name)?,
            "route output evidence changed",
        )?;
    }
    let records = super::super::super::admission::compact(
        &command.name,
        &process::read_bounded(&root.join(&p.stdout.path), MAXIMUM_OUTPUT_BYTES)?,
    )?;
    require(
        value(&records, "result", "status")? == "success",
        "route compact result failed",
    )?;
    let expected_command = if command.name == "status-final" {
        "status"
    } else {
        &command.name
    };
    require(
        value(&records, "result", "command")? == expected_command,
        "route output command differs",
    )?;
    Ok(records)
}
pub(super) fn value<'a>(
    records: &'a [CompactRecord],
    operation: &str,
    field: &str,
) -> Result<&'a str, DevError> {
    let mut matching = records.iter().filter(|r| r.operation == operation);
    let record = matching
        .next()
        .ok_or_else(|| DevError::corrupt(format!("missing route {operation} record")))?;
    require(matching.next().is_none(), "duplicate route output record")?;
    let mut fields = record.fields.iter().filter(|f| f.name == field);
    let value = fields
        .next()
        .ok_or_else(|| DevError::corrupt(format!("missing route {operation}.{field}")))?;
    require(fields.next().is_none(), "duplicate route output field")?;
    Ok(&value.value)
}
fn capabilities(records: &[CompactRecord], manifest: &ReleaseManifest) -> Result<(), DevError> {
    require(
        value(records, "product", "name")? == "lkjscript"
            && value(records, "product", "version")? == manifest.product.version
            && value(records, "capabilities", "digest")? == manifest.executable.capabilities_digest,
        "actual route capabilities differ from manifest",
    )
}
fn checked(records: &[CompactRecord]) -> Result<(), DevError> {
    require(
        value(records, "tests", "failed")? == "0"
            && value(records, "tests", "differential")? == "equal"
            && value(records, "tests", "passed")?
                .parse::<u64>()
                .is_ok_and(|n| n > 0),
        "route graph checks did not pass",
    )
}
fn hello(records: &[CompactRecord]) -> Result<(), DevError> {
    let text: String = serde_json::from_str(value(records, "execution", "value")?)?;
    require(
        text == "hello"
            && value(records, "execution", "target")? == "main"
            && value(records, "execution", "differential")? == "equal",
        "route command did not produce independently expected typed text hello",
    )
}
