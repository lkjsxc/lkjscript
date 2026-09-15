use super::*;
use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan};

const TIMEOUT: Duration = Duration::from_secs(120);
const MAXIMUM_REQUEST_BYTES: u64 = 64 * 1024;
const MAXIMUM_REVIEW_BYTES: u64 = 1024 * 1024;
const COMMANDS: [&str; 19] = [
    "capabilities",
    "change-capabilities",
    "new",
    "status",
    "create-plan",
    "create-apply",
    "find-module",
    "find-function",
    "find-parameter",
    "definition-created",
    "run-created",
    "replace-plan",
    "replace-apply",
    "definition-replaced",
    "run-replaced",
    "check",
    "build",
    "run",
    "status-final",
];

// Literal product authoring. Only the accepted base is supplied by the surrounding request.
const CREATE_BODY: &str = r#"reference.package as=$standard source=builtin
reference.owner as=$add package=$standard class=declaration name=add
create.module as=$module name=structural
expression.block as=$adjust-body
  (let
    (binding value (type i64) (local $input))
    (in (call $add (local value) (i64 1))))
expression.end
create.function as=$adjust module=$module name=adjust visibility=private result=i64 effect=pure body=$adjust-body
add.parameter as=$input function=$adjust name=input type=i64
expression.block as=$entry-body
  (call $adjust (i64 41))
expression.end
create.function as=$entry module=$module name=entry visibility=private result=i64 effect=pure body=$entry-body
type.function as=@Entry result=i64
create.component as=$component module=$module name=application visibility=private
add.port as=$port component=$component name=main type=@Entry function=$entry
create.target as=$target name=structural component=$component port=$port runner=command
"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    name: String,
    command: Vec<String>,
    process: Option<process::ProcessObservation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Review {
    request: FileBinding,
    logical_plan: FileBinding,
    base: String,
    candidate: String,
    token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    body: String,
    function: Vec<(String, String)>,
    parameter: Vec<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct StructuralAuthoring {
    creation: Review,
    replacement: Review,
    module: String,
    function: String,
    parameter: String,
    before: Definition,
    after: Definition,
    created_value: i64,
    replaced_value: i64,
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
    structural: Option<StructuralAuthoring>,
    artifact: Option<FileBinding>,
    value: Option<String>,
    pub(super) started_unix_nanoseconds: u128,
    pub(super) completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    cleanup_nanoseconds: u64,
    pub(super) cleanup_complete: bool,
    failure: Option<String>,
}

#[derive(Default)]
struct Progress {
    initial_revision: Option<String>,
    revision: Option<String>,
    creation: Option<Review>,
    replacement: Option<Review>,
    module: Option<String>,
    function: Option<String>,
    parameter: Option<String>,
    before: Option<Definition>,
    after: Option<Definition>,
    created_value: Option<i64>,
    replaced_value: Option<i64>,
    artifact: Option<FileBinding>,
    value: Option<String>,
}

fn required<'a, T>(value: &'a Option<T>, name: &str) -> Result<&'a T, DevError> {
    value
        .as_ref()
        .ok_or_else(|| DevError::corrupt(format!("route observation '{name}' is missing")))
}

impl Progress {
    fn request(&self, name: &str) -> Result<String, DevError> {
        match name {
            "create" => Ok(format!(
                "request base={} idempotency=installed-structural-create\n{CREATE_BODY}",
                required(&self.initial_revision, "initial revision")?
            )),
            "replace" => Ok(format!(
                r#"request base={} idempotency=installed-structural-replace
reference.package as=$standard source=builtin
reference.owner as=$add package=$standard class=declaration name=add
expression.block as=$replacement
  (let
    (binding value (type i64) (local (exact {})))
    (binding value (type i64) (call $add (local value) (i64 1)))
    (in (call $add (local value) (i64 1))))
expression.end
replace.body function={} body=$replacement
"#,
                required(&self.creation, "creation review")?.candidate,
                required(&self.parameter, "discovered parameter")?,
                required(&self.function, "discovered function")?
            )),
            _ => Err(DevError::corrupt("unknown route request")),
        }
    }

    fn command(
        &self,
        options: &PairOptions,
        route: Route,
        name: &str,
    ) -> Result<Vec<String>, DevError> {
        let root = route.root(options);
        let project = root.join("runtime/project").display().to_string();
        let artifact = root.join("artifact.lkja").display().to_string();
        let request_name = if name.starts_with("create-") {
            "create"
        } else {
            "replace"
        };
        let input = root
            .join(format!("{request_name}.lkjc"))
            .display()
            .to_string();
        let plan = root
            .join(format!("{request_name}.logical-plan"))
            .display()
            .to_string();
        let arguments = match name {
            "capabilities" => vec!["capabilities"],
            "change-capabilities" => vec!["capabilities", "--section", "change"],
            "new" => vec!["new", &project, "--template", "command", "--name", "hello"],
            "status" | "status-final" => vec!["status"],
            "create-plan" | "replace-plan" => {
                vec!["change", "plan", "--input-file", &input, "--output", &plan]
            }
            "create-apply" => vec![
                "change",
                "apply",
                "--input-file",
                &input,
                "--plan",
                &required(&self.creation, "creation review")?.token,
            ],
            "replace-apply" => vec![
                "change",
                "apply",
                "--input-file",
                &input,
                "--plan",
                &required(&self.replacement, "replacement review")?.token,
            ],
            "find-module" => vec!["query", "find", "module", "structural"],
            "find-function" => vec![
                "query",
                "find",
                "declaration",
                "adjust",
                "--parent",
                required(&self.module, "discovered module")?,
            ],
            "find-parameter" => vec![
                "query",
                "find",
                "parameter",
                "input",
                "--parent",
                required(&self.function, "discovered function")?,
            ],
            "definition-created" | "definition-replaced" => vec![
                "inspect",
                "owner",
                "pure_function",
                required(&self.function, "discovered function")?,
                "--detail",
                "definition",
                "--limit",
                "100",
                "--bytes",
                "65536",
            ],
            "run-created" | "run-replaced" => vec!["run", "structural"],
            "check" => vec!["check"],
            "build" => vec!["build", "--output", &artifact],
            "run" => vec!["run", "main"],
            _ => return Err(DevError::corrupt("unknown lifecycle operation")),
        };
        let mut command = vec![
            installation::candidate(options, route)
                .display()
                .to_string(),
        ];
        if !matches!(name, "capabilities" | "change-capabilities" | "new") {
            command.extend(["--project".to_owned(), project.clone()]);
        }
        command.extend(arguments.into_iter().map(str::to_owned));
        Ok(command)
    }

    fn observe(
        &mut self,
        root: &Path,
        name: &str,
        records: &[CompactRecord],
        manifest: &ReleaseManifest,
    ) -> Result<(), DevError> {
        if matches!(
            name,
            "run-created" | "run-replaced" | "check" | "build" | "run"
        ) {
            require(
                value(records, "authority", "revision")?
                    == required(&self.revision, "current revision")?,
                "route execution or build used a different accepted revision",
            )?;
        }
        match name {
            "capabilities" => capabilities(records, manifest)?,
            "change-capabilities" => {
                capabilities(records, manifest)?;
                structural_capabilities(records)?;
            }
            "new" => {
                let revision = value(records, "revision", "id")?.to_owned();
                self.initial_revision = Some(revision.clone());
                self.revision = Some(revision);
            }
            "status" | "status-final" => require(
                value(records, "revision", "id")? == required(&self.revision, "current revision")?,
                "route graph revision changed",
            )?,
            "create-plan" | "replace-plan" => {
                let kind = if name == "create-plan" {
                    "create"
                } else {
                    "replace"
                };
                let request = root.join(format!("{kind}.lkjc"));
                require(
                    process::read_bounded(&request, MAXIMUM_REQUEST_BYTES)?
                        == self.request(kind)?.as_bytes(),
                    "route retained literal request differs from public observations",
                )?;
                let plan = root.join(format!("{kind}.logical-plan"));
                let bytes = process::read_bounded(&plan, MAXIMUM_REVIEW_BYTES)?;
                let decoded = decode_logical_change_plan(std::io::Cursor::new(bytes))
                    .map_err(|error| DevError::corrupt(error.to_string()))?;
                let base = value(records, "revision", "base")?.to_owned();
                let candidate = value(records, "revision", "result")?.to_owned();
                let token = value(records, "plan", "token")?.to_owned();
                require(
                    base == *required(&self.revision, "current revision")?
                        && candidate != base
                        && decoded.token == token
                        && value(records, "plan-output", "path")? == plan.display().to_string(),
                    "route review does not bind its current base, candidate and retained plan",
                )?;
                let review = Review {
                    request: binding(&request)?,
                    logical_plan: binding(&plan)?,
                    base,
                    candidate,
                    token,
                };
                if name == "create-plan" {
                    self.creation = Some(review);
                } else {
                    self.replacement = Some(review);
                }
            }
            "create-apply" | "replace-apply" => {
                let review = required(
                    if name == "create-apply" {
                        &self.creation
                    } else {
                        &self.replacement
                    },
                    "applied review",
                )?;
                require(
                    value(records, "revision", "base")? == review.base
                        && value(records, "revision", "result")? == review.candidate
                        && value(records, "plan", "token")? == review.token
                        && required(&self.revision, "current revision")? == &review.base,
                    "route apply differs from its reviewed request and candidate",
                )?;
                self.revision = Some(review.candidate.clone());
            }
            "find-module" | "find-function" | "find-parameter" => {
                require(
                    value(records, "revision", "observed")?
                        == required(&self.revision, "current revision")?
                        && value(records, "summary", "match")? == "true",
                    "route named discovery did not match at its accepted revision",
                )?;
                let (expected_name, expected_kind, parent, prefix) = match name {
                    "find-module" => ("structural", "module", None, "mod_"),
                    "find-function" => (
                        "adjust",
                        "pure_function",
                        Some(required(&self.module, "module")?),
                        "decl_",
                    ),
                    _ => (
                        "input",
                        "parameter",
                        Some(required(&self.function, "function")?),
                        "param_",
                    ),
                };
                let id = value(records, "owner", "id")?.to_owned();
                require(
                    value(records, "owner", "name")? == expected_name
                        && value(records, "owner", "kind")? == expected_kind
                        && id.strip_prefix(prefix).is_some_and(|suffix| {
                            suffix.len() == 32
                                && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
                        }),
                    "route discovered a different typed owner",
                )?;
                if let Some(parent) = parent {
                    require(
                        value(records, "owner", "parent")? == parent,
                        "route discovered a foreign owner parent",
                    )?;
                }
                match name {
                    "find-module" => self.module = Some(id),
                    "find-function" => self.function = Some(id),
                    _ => self.parameter = Some(id),
                }
            }
            "definition-created" | "definition-replaced" => {
                let after = name == "definition-replaced";
                let definition = definition(
                    records,
                    required(&self.revision, "current revision")?,
                    required(&self.module, "module")?,
                    required(&self.function, "function")?,
                    required(&self.parameter, "parameter")?,
                    if after { 8 } else { 5 },
                    if after { 2 } else { 1 },
                )?;
                if after {
                    let before = required(&self.before, "original definition")?;
                    require(
                        definition.function == before.function
                            && definition.parameter == before.parameter
                            && definition.body != before.body,
                        "structural replacement lost unchanged owners or retained its old body",
                    )?;
                    self.after = Some(definition);
                } else {
                    self.before = Some(definition);
                }
            }
            "run-created" | "run-replaced" => {
                let expected = if name == "run-created" { 42 } else { 43 };
                let observed: i64 = serde_json::from_str(value(records, "execution", "value")?)?;
                require(
                    observed == expected
                        && value(records, "execution", "target")? == "structural"
                        && value(records, "execution", "differential")? == "equal",
                    "structural route did not produce its independently expected I64 result",
                )?;
                if name == "run-created" {
                    self.created_value = Some(observed);
                } else {
                    self.replaced_value = Some(observed);
                }
            }
            "check" => checked(records)?,
            "build" => {
                let artifact = binding(&root.join("artifact.lkja"))?;
                require(
                    value(records, "output", "visibility")? == "created"
                        && artifact.file.byte_length > 0
                        && artifact.file.byte_length <= 128 * 1024 * 1024,
                    "route artifact absent or exceeds bound",
                )?;
                self.artifact = Some(artifact);
            }
            "run" => {
                hello(records)?;
                self.value = Some(value(records, "execution", "value")?.to_owned());
            }
            _ => return Err(DevError::corrupt("unknown lifecycle observation")),
        }
        Ok(())
    }

    fn structural(&self) -> Result<StructuralAuthoring, DevError> {
        Ok(StructuralAuthoring {
            creation: required(&self.creation, "creation review")?.clone(),
            replacement: required(&self.replacement, "replacement review")?.clone(),
            module: required(&self.module, "module")?.clone(),
            function: required(&self.function, "function")?.clone(),
            parameter: required(&self.parameter, "parameter")?.clone(),
            before: required(&self.before, "original definition")?.clone(),
            after: required(&self.after, "replacement definition")?.clone(),
            created_value: *required(&self.created_value, "created result")?,
            replaced_value: *required(&self.replaced_value, "replacement result")?,
        })
    }
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
        commands: COMMANDS
            .into_iter()
            .map(|name| Command {
                name: name.to_owned(),
                command: Vec::new(),
                process: None,
            })
            .collect(),
        revision: None,
        structural: None,
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
        let mut progress = Progress::default();
        let mut head = None;
        for (index, name) in COMMANDS.iter().enumerate() {
            require(!control.cancelled(), "route lifecycle cancelled")?;
            if matches!(*name, "create-plan" | "replace-plan") {
                let kind = if *name == "create-plan" {
                    "create"
                } else {
                    "replace"
                };
                archive::write_new(
                    &root.join(format!("{kind}.lkjc")),
                    progress.request(kind)?.as_bytes(),
                    0o644,
                )?;
            }
            receipt.commands[index].command = progress.command(options, route, name)?;
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
            progress.observe(&root, name, &records, manifest)?;
            receipt.revision = progress.initial_revision.clone();
            receipt.artifact = progress.artifact.clone();
            receipt.value = progress.value.clone();
            if matches!(*name, "new" | "create-apply" | "replace-apply") {
                let next = process::read_bounded(&runtime.join("project/HEAD"), 4096)?;
                require(
                    head.as_ref() != Some(&next),
                    "route accepted mutation did not advance HEAD",
                )?;
                head = Some(next);
            } else if let Some(head) = &head {
                require(
                    process::read_bounded(&runtime.join("project/HEAD"), 4096)? == *head,
                    "route read-only lifecycle changed accepted HEAD",
                )?;
            }
            evidence::publish_json(&path, &receipt)?;
        }
        receipt.structural = Some(progress.structural()?);
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
            && receipt.completed_unix_nanoseconds >= receipt.started_unix_nanoseconds
            && receipt.commands.len() == COMMANDS.len(),
        "route lifecycle is foreign, incomplete or unclean",
    )?;
    let mut progress = Progress::default();
    for (index, (observed, name)) in receipt.commands.iter().zip(COMMANDS).enumerate() {
        require(
            observed.name == name && observed.command == progress.command(options, route, name)?,
            "route command was skipped, duplicated or redirected",
        )?;
        let records = read_command(&root, index, observed)?;
        progress.observe(&root, name, &records, &admitted.manifest)?;
    }
    require(
        receipt.revision == progress.initial_revision
            && receipt.artifact == progress.artifact
            && receipt.value == progress.value
            && receipt.structural.as_ref() == Some(&progress.structural()?),
        "route structural authoring or result evidence differs from its original observations",
    )
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
            && p.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && p.stderr.bytes == Some(0),
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
    let (expected_command, expected_status) = match command.name.as_str() {
        "change-capabilities" => ("capabilities.section", "success"),
        "status-final" => ("status", "success"),
        "create-plan" | "replace-plan" => ("change.plan", "prepared"),
        "create-apply" | "replace-apply" => ("change.apply", "accepted"),
        "find-module" | "find-function" | "find-parameter" => ("query.find", "success"),
        "definition-created" | "definition-replaced" => ("inspect.owner.definition", "success"),
        "run-created" | "run-replaced" => ("run", "success"),
        name => (name, "success"),
    };
    require(
        value(&records, "result", "status")? == expected_status
            && value(&records, "result", "command")? == expected_command,
        "route output command or disposition differs",
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
    field_value(record, field)
}

fn field_value<'a>(record: &'a CompactRecord, field: &str) -> Result<&'a str, DevError> {
    let mut fields = record.fields.iter().filter(|f| f.name == field);
    let value = fields
        .next()
        .ok_or_else(|| DevError::corrupt(format!("missing route {}.{field}", record.operation)))?;
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

fn structural_capabilities(records: &[CompactRecord]) -> Result<(), DevError> {
    require(
        value(records, "change", "expression-notations")? == "flat|block"
            && value(records, "change.expression-block", "header")? == "expression.block as=$ROOT"
            && value(records, "change.expression-block", "end")? == "expression.end",
        "installed route does not advertise structural blocks",
    )?;
    let forms = records
        .iter()
        .filter(|record| record.operation == "change.expression-syntax")
        .map(|record| field_value(record, "name"))
        .collect::<Result<Vec<_>, _>>()?;
    require(
        forms
            == [
                "unit",
                "bool",
                "i64",
                "text",
                "static-text",
                "local",
                "constant",
                "if",
                "sequence",
                "call",
                "function-value",
                "invoke",
                "bind",
                "let",
                "record",
                "variant",
                "field",
                "list",
                "map",
                "match",
                "capability-call",
                "transaction",
            ],
        "installed route structural form inventory differs",
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "independent expected owner and body-shape observations"
)]
fn definition(
    records: &[CompactRecord],
    revision: &str,
    module: &str,
    function: &str,
    parameter: &str,
    expressions: usize,
    bindings: usize,
) -> Result<Definition, DevError> {
    require(
        value(records, "revision", "observed")? == revision
            && value(records, "page", "complete")? == "true"
            && value(records, "definition.function", "id")? == function
            && value(records, "definition.function", "module")?
                == format!(
                    "{}/{module}",
                    value(records, "definition.header", "package")?
                )
            && value(records, "definition.function", "name")? == "adjust"
            && value(records, "definition.function", "parameters")? == "1"
            && value(records, "definition.parameter", "id")? == parameter
            && value(records, "definition.parameter", "parent")? == function
            && value(records, "definition.parameter", "name")? == "input"
            && value(records, "definition.parameter", "index")? == "0"
            && value(records, "definition.parameter", "use")? == "unrestricted"
            && records
                .iter()
                .filter(|r| r.operation == "definition.expression")
                .count()
                == expressions
            && records
                .iter()
                .filter(|r| r.operation == "definition.binding")
                .count()
                == bindings,
        "route definition omitted or changed its expected owners and structural body",
    )?;
    let fields = |operation: &str, excluded: &str| {
        records
            .iter()
            .filter(|record| record.operation == operation)
            .flat_map(|record| record.fields.iter())
            .filter(|field| field.name != excluded)
            .map(|field| (field.name.clone(), field.value.clone()))
            .collect::<Vec<_>>()
    };
    Ok(Definition {
        body: value(records, "definition.function", "body")?.to_owned(),
        function: fields("definition.function", "body"),
        parameter: fields("definition.parameter", ""),
    })
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
