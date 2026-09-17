use super::*;
use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan};

const TIMEOUT: Duration = Duration::from_secs(120);
const MAXIMUM_REQUEST_BYTES: u64 = 64 * 1024;
const MAXIMUM_REVIEW_BYTES: u64 = 1024 * 1024;
const COMMANDS: [&str; 30] = [
    "capabilities",
    "change-capabilities",
    "runners-capabilities",
    "new",
    "status",
    "create-plan",
    "create-apply",
    "find-module",
    "find-function",
    "find-parameter",
    "find-numerical-function",
    "find-numerical-parameter",
    "definition-created",
    "numerical-definition-created",
    "run-created",
    "numerical-run-created",
    "numerical-run-negative-zero",
    "replace-plan",
    "replace-apply",
    "definition-replaced",
    "numerical-definition-replaced",
    "run-replaced",
    "rejected-apply",
    "run-after-rejection",
    "numerical-run-replaced",
    "check",
    "build",
    "run",
    "status-final",
    "standalone-structural",
];

// Literal product authoring. Only the accepted base is supplied by the surrounding request.
const CREATE_BODY: &str = r#"reference.package as=$standard source=builtin
reference.owner as=$add package=$standard class=declaration name=add
reference.owner as=$f64-add package=$standard class=declaration name=f64-add
reference.owner as=$f64-to-text package=$standard class=declaration name=f64-to-text
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
expression.block as=$calibrate-body
  (call $f64-add (local $sample) (f64 0.5))
expression.end
create.function as=$calibrate module=$module name=calibrate visibility=private result=f64 effect=pure body=$calibrate-body
add.parameter as=$sample function=$calibrate name=sample type=f64
type.function as=@Numerical result=f64
type.argument parent=@Numerical index=0 type=f64
add.port as=$numerical-port component=$component name=numerical type=@Numerical function=$calibrate
create.target as=$numerical-target name=numerical component=$component port=$numerical-port runner=command
expression.block as=$format-body
  (call $f64-to-text (local $number))
expression.end
create.function as=$format module=$module name=format-number visibility=private result=text effect=pure body=$format-body
add.parameter as=$number function=$format name=number type=f64
type.function as=@NumberText result=text
type.argument parent=@NumberText index=0 type=f64
add.port as=$format-port component=$component name=number-text type=@NumberText function=$format
create.target as=$format-target name=number-text component=$component port=$format-port runner=command
"#;

const NUMERICAL_INPUT: &str = "[1.25e0]";
const NEGATIVE_ZERO_INPUT: &str = "[-0.0]";

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct NumericalAuthoring {
    function: String,
    parameter: String,
    before: Definition,
    after: Definition,
    decimal_input: FileBinding,
    negative_zero_input: FileBinding,
    created_bits: u64,
    replaced_bits: u64,
    negative_zero_text: String,
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
    numerical: Option<NumericalAuthoring>,
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
    numerical_function: Option<String>,
    numerical_parameter: Option<String>,
    numerical_before: Option<Definition>,
    numerical_after: Option<Definition>,
    decimal_input: Option<FileBinding>,
    negative_zero_input: Option<FileBinding>,
    numerical_created_bits: Option<u64>,
    numerical_replaced_bits: Option<u64>,
    negative_zero_text: Option<String>,
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
reference.owner as=$f64-add package=$standard class=declaration name=f64-add
expression.block as=$replacement
  (let
    (binding value (type i64) (local (exact {})))
    (binding value (type i64) (call $add (local value) (i64 1)))
    (in (call $add (local value) (i64 1))))
expression.end
replace.body function={} body=$replacement
expression.block as=$numerical-replacement
  (call $f64-add
    (call $f64-add (local (exact {})) (f64 0.5))
    (f64 1.0))
expression.end
replace.body function={} body=$numerical-replacement
"#,
                required(&self.creation, "creation review")?.candidate,
                required(&self.parameter, "discovered parameter")?,
                required(&self.function, "discovered function")?,
                required(&self.numerical_parameter, "discovered numerical parameter")?,
                required(&self.numerical_function, "discovered numerical function")?
            )),
            "rejected" => Ok(self.request("replace")?.replace("(f64 1.0)", "(f64 2.0)")),
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
        let request_name = if name == "rejected-apply" {
            "rejected"
        } else if name.starts_with("create-") {
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
        let deployment = root
            .join("standalone.deployment.json")
            .display()
            .to_string();
        let arguments = match name {
            "capabilities" => vec!["capabilities"],
            "change-capabilities" => vec!["capabilities", "--section", "change"],
            "runners-capabilities" => vec!["capabilities", "--section", "runners"],
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
            "replace-apply" | "rejected-apply" => vec![
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
            "find-numerical-function" => vec![
                "query",
                "find",
                "declaration",
                "calibrate",
                "--parent",
                required(&self.module, "discovered module")?,
            ],
            "find-numerical-parameter" => vec![
                "query",
                "find",
                "parameter",
                "sample",
                "--parent",
                required(&self.numerical_function, "discovered numerical function")?,
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
            "numerical-definition-created" | "numerical-definition-replaced" => vec![
                "inspect",
                "owner",
                "pure_function",
                required(&self.numerical_function, "discovered numerical function")?,
                "--detail",
                "definition",
                "--limit",
                "100",
                "--bytes",
                "65536",
            ],
            "run-created" | "run-replaced" | "run-after-rejection" => vec!["run", "structural"],
            "numerical-run-created" | "numerical-run-replaced" => {
                vec!["run", "numerical", "--arguments", NUMERICAL_INPUT]
            }
            "numerical-run-negative-zero" => {
                vec!["run", "number-text", "--arguments", NEGATIVE_ZERO_INPUT]
            }
            "check" => vec!["check"],
            "build" => vec!["build", "--output", &artifact],
            "run" => vec!["run", "main"],
            "standalone-structural" => vec!["run", "--deployment", &deployment],
            _ => return Err(DevError::corrupt("unknown lifecycle operation")),
        };
        let mut command = vec![
            installation::candidate(options, route)
                .display()
                .to_string(),
        ];
        if !matches!(
            name,
            "capabilities"
                | "change-capabilities"
                | "runners-capabilities"
                | "new"
                | "standalone-structural"
        ) {
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
            "run-created"
                | "run-replaced"
                | "run-after-rejection"
                | "numerical-run-created"
                | "numerical-run-replaced"
                | "numerical-run-negative-zero"
                | "check"
                | "build"
                | "run"
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
            "runners-capabilities" => {
                capabilities(records, manifest)?;
                numerical_capabilities(records)?;
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
            "find-module"
            | "find-function"
            | "find-parameter"
            | "find-numerical-function"
            | "find-numerical-parameter" => {
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
                    "find-numerical-function" => (
                        "calibrate",
                        "pure_function",
                        Some(required(&self.module, "module")?),
                        "decl_",
                    ),
                    "find-numerical-parameter" => (
                        "sample",
                        "parameter",
                        Some(required(&self.numerical_function, "numerical function")?),
                        "param_",
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
                    "find-numerical-function" => self.numerical_function = Some(id),
                    "find-numerical-parameter" => self.numerical_parameter = Some(id),
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
                    "adjust",
                    "input",
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
            "numerical-definition-created" | "numerical-definition-replaced" => {
                let after = name == "numerical-definition-replaced";
                let definition = definition(
                    records,
                    required(&self.revision, "current revision")?,
                    required(&self.module, "module")?,
                    required(&self.numerical_function, "numerical function")?,
                    required(&self.numerical_parameter, "numerical parameter")?,
                    "calibrate",
                    "sample",
                    if after { 5 } else { 3 },
                    0,
                )?;
                if after {
                    let before = required(&self.numerical_before, "original numerical definition")?;
                    require(
                        definition.function == before.function
                            && definition.parameter == before.parameter
                            && definition.body != before.body,
                        "numerical replacement lost unchanged owners or retained its old body",
                    )?;
                    self.numerical_after = Some(definition);
                } else {
                    self.numerical_before = Some(definition);
                }
            }
            "run-created" | "run-replaced" | "run-after-rejection" => {
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
            "rejected-apply" => {
                require(
                    process::read_bounded(&root.join("rejected.lkjc"), MAXIMUM_REQUEST_BYTES)?
                        == self.request("rejected")?.as_bytes(),
                    "rejected edit literal changed",
                )?;
                require(
                    value(records, "result", "status")? == "failure"
                        && value(records, "diagnostic", "code")?
                            == "change_request_commitment_mismatch",
                    "mismatched reviewed edit was not specifically rejected",
                )?;
            }
            "standalone-structural" => {
                require(
                    !root.join("runtime/project").exists(),
                    "standalone execution retained the authoring project",
                )?;
                require(
                    value(records, "execution", "value")? == "43"
                        && value(records, "execution", "execution-mode")? == "production"
                        && value(records, "execution", "verification")? == "not-performed",
                    "standalone installed artifact produced an unexpected result or execution mode",
                )?;
                validate_descriptor(root)?;
            }
            "numerical-run-created" | "numerical-run-replaced" => {
                let input = literal_input(root, "numerical-input.json", NUMERICAL_INPUT)?;
                // Independently fixed dyadic results: 1.25 + 0.5 = 1.75; then add 1.0 = 2.75.
                let expected = if name == "numerical-run-created" {
                    0x3ffc_0000_0000_0000
                } else {
                    0x4006_0000_0000_0000
                };
                let observed: f64 = serde_json::from_str(value(records, "execution", "value")?)?;
                require(
                    observed.to_bits() == expected
                        && value(records, "execution", "target")? == "numerical"
                        && value(records, "execution", "differential")? == "equal",
                    "numerical route did not produce its independently expected F64 bits",
                )?;
                self.decimal_input = Some(input);
                if name == "numerical-run-created" {
                    self.numerical_created_bits = Some(observed.to_bits());
                } else {
                    self.numerical_replaced_bits = Some(observed.to_bits());
                }
            }
            "numerical-run-negative-zero" => {
                self.negative_zero_input = Some(literal_input(
                    root,
                    "numerical-negative-zero.json",
                    NEGATIVE_ZERO_INPUT,
                )?);
                let observed: String = serde_json::from_str(value(records, "execution", "value")?)?;
                require(
                    observed == "-0.0"
                        && value(records, "execution", "target")? == "number-text"
                        && value(records, "execution", "differential")? == "equal",
                    "numerical route lost the external negative zero sign or standard formatting",
                )?;
                self.negative_zero_text = Some(observed);
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

    fn numerical(&self) -> Result<NumericalAuthoring, DevError> {
        Ok(NumericalAuthoring {
            function: required(&self.numerical_function, "numerical function")?.clone(),
            parameter: required(&self.numerical_parameter, "numerical parameter")?.clone(),
            before: required(&self.numerical_before, "original numerical definition")?.clone(),
            after: required(&self.numerical_after, "replacement numerical definition")?.clone(),
            decimal_input: required(&self.decimal_input, "decimal input")?.clone(),
            negative_zero_input: required(&self.negative_zero_input, "negative zero input")?
                .clone(),
            created_bits: *required(&self.numerical_created_bits, "created numerical result")?,
            replaced_bits: *required(&self.numerical_replaced_bits, "replaced numerical result")?,
            negative_zero_text: required(&self.negative_zero_text, "negative zero formatting")?
                .clone(),
        })
    }
}

fn validate_descriptor(root: &Path) -> Result<(), DevError> {
    let mut original: serde_json::Value = serde_json::from_slice(&process::read_bounded(
        &root.join("starter-command.deployment.json"),
        MAXIMUM_REQUEST_BYTES,
    )?)?;
    require(
        original["artifact"] == "generated/application.lkja"
            && original["target"] == "main"
            && original["grants"] == serde_json::json!([])
            && original["secrets"] == serde_json::json!([]),
        "standalone source descriptor differs from the discovered empty-grant starter",
    )?;
    original["artifact"] = serde_json::Value::String("artifact.lkja".to_owned());
    original["target"] = serde_json::Value::String("structural".to_owned());
    require(
        process::read_bounded(
            &root.join("standalone.deployment.json"),
            MAXIMUM_REQUEST_BYTES,
        )? == evidence::encode_json(&original)?,
        "standalone descriptor differs from the retained ordinary operation settings",
    )
}

fn literal_input(root: &Path, name: &str, expected: &str) -> Result<FileBinding, DevError> {
    let path = root.join(name);
    require(
        process::read_bounded(&path, MAXIMUM_REQUEST_BYTES)? == expected.as_bytes(),
        "route retained numerical input differs from its literal command arguments",
    )?;
    binding(&path)
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
        numerical: None,
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
            if *name == "rejected-apply" {
                archive::write_new(
                    &root.join("rejected.lkjc"),
                    progress.request("rejected")?.as_bytes(),
                    0o644,
                )?;
            }
            if *name == "standalone-structural" {
                let original = process::read_bounded(
                    &runtime.join("project/command.deployment.json"),
                    MAXIMUM_REQUEST_BYTES,
                )?;
                archive::write_new(
                    &root.join("starter-command.deployment.json"),
                    &original,
                    0o644,
                )?;
                let mut descriptor: serde_json::Value = serde_json::from_slice(&original)?;
                descriptor["artifact"] = serde_json::Value::String("artifact.lkja".to_owned());
                descriptor["target"] = serde_json::Value::String("structural".to_owned());
                archive::write_new(
                    &root.join("standalone.deployment.json"),
                    &evidence::encode_json(&descriptor)?,
                    0o644,
                )?;
                fs::remove_dir_all(runtime.join("project"))?;
            }
            if *name == "numerical-run-created" {
                archive::write_new(
                    &root.join("numerical-input.json"),
                    NUMERICAL_INPUT.as_bytes(),
                    0o644,
                )?;
            }
            if *name == "numerical-run-negative-zero" {
                archive::write_new(
                    &root.join("numerical-negative-zero.json"),
                    NEGATIVE_ZERO_INPUT.as_bytes(),
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
            } else if let Some(head) = &head
                && *name != "standalone-structural"
            {
                require(
                    process::read_bounded(&runtime.join("project/HEAD"), 4096)? == *head,
                    "route read-only lifecycle changed accepted HEAD",
                )?;
            }
            evidence::publish_json(&path, &receipt)?;
        }
        receipt.structural = Some(progress.structural()?);
        receipt.numerical = Some(progress.numerical()?);
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
            && receipt.structural.as_ref() == Some(&progress.structural()?)
            && receipt.numerical.as_ref() == Some(&progress.numerical()?),
        "route structural or numerical authoring and result evidence differs from its originals",
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
        (if command.name == "rejected-apply" {
            p.status == process::ProcessStatus::Failed
                && p.exit_code.is_some_and(|value| value != 0)
                && p.reason.as_deref() == Some("nonzero_exit")
        } else {
            p.status == process::ProcessStatus::Passed
                && p.exit_code == Some(0)
                && p.reason.is_none()
        }) && p.signal.is_none()
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
        "change-capabilities" | "runners-capabilities" => ("capabilities.section", "success"),
        "status-final" => ("status", "success"),
        "create-plan" | "replace-plan" => ("change.plan", "prepared"),
        "create-apply" | "replace-apply" => ("change.apply", "accepted"),
        "rejected-apply" => ("change", "failure"),
        "find-module"
        | "find-function"
        | "find-parameter"
        | "find-numerical-function"
        | "find-numerical-parameter" => ("query.find", "success"),
        "definition-created"
        | "definition-replaced"
        | "numerical-definition-created"
        | "numerical-definition-replaced" => ("inspect.owner.definition", "success"),
        "run-created"
        | "run-replaced"
        | "run-after-rejection"
        | "standalone-structural"
        | "numerical-run-created"
        | "numerical-run-replaced"
        | "numerical-run-negative-zero" => ("run", "success"),
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
                "f64",
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
                "transaction-outcome",
            ],
        "installed route structural form inventory differs",
    )
}

fn numerical_capabilities(records: &[CompactRecord]) -> Result<(), DevError> {
    require(
        value(records, "execution.f64", "type")? == "f64"
            && value(records, "execution.f64", "nan-bits")? == "0x7ff8000000000000"
            && value(records, "execution.f64", "intrinsic-prefix")? == "core.f64."
            && value(records, "execution.f64", "value-equality")?
                == "recursive-ieee-nan-unequal-zeros-equal"
            && value(records, "execution.f64", "observation-equality")?
                == "normalized-bits-nan-reflexive-zeros-distinct"
            && value(records, "execution.f64-conversion", "format")?
                == "core.f64.to-text:F64-to-Text"
            && value(records, "execution.f64-transport", "json-input")?
                == "finite-integer-fraction-exponent-with-correct-rounding-and-signed-zero"
            && value(records, "execution.f64-transport", "json-output")?
                == "finite-number-or-normalized_json_nonfinite",
        "installed route does not advertise the ordinary binary64 contract",
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
    function_name: &str,
    parameter_name: &str,
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
            && value(records, "definition.function", "name")? == function_name
            && value(records, "definition.function", "parameters")? == "1"
            && value(records, "definition.parameter", "id")? == parameter
            && value(records, "definition.parameter", "parent")? == function
            && value(records, "definition.parameter", "name")? == parameter_name
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

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
