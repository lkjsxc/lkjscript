use lkjscript::application::{ApplicationValue, HostOutcomeClass};
use lkjscript::instance::{
    HostAdapterKind, INSTANCE_CONTRACT_VERSION, InstanceTransitionStatus, MAXIMUM_BLOB_BYTES,
    QueryResultDigest, immutable_blob_digest,
};
use lkjscript::schema::MAXIMUM_BYTE_STRING_BYTES;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[path = "lkjwork/bindings.rs"]
mod bindings;
#[path = "lkjwork/project.rs"]
mod project;
#[path = "lkjwork/render.rs"]
mod render;

use bindings::Bindings;
use project::{FakeAttachmentOutcomes, Project};

const PRODUCT_CONTRACT_VERSION: u16 = 1;
const PRODUCT_EXPORT_VERSION: u16 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_INFRASTRUCTURE: u8 = 3;
const EXIT_DOMAIN_CONFLICT: u8 = 10;
const MAXIMUM_ATTACHMENT_BYTES: u64 = if MAXIMUM_BYTE_STRING_BYTES < MAXIMUM_BLOB_BYTES {
    MAXIMUM_BYTE_STRING_BYTES as u64
} else {
    MAXIMUM_BLOB_BYTES as u64
};
const MAXIMUM_SESSION_LINE_BYTES: usize = 1024 * 1024;
const MAXIMUM_SESSION_ARGUMENTS: usize = 128;

const HELP: &str = "lkjwork — a local durable work ledger for humans and coding agents

Usage:
  lkjwork [--json] [--project PATH] [--base-revision N] [--idempotency-key KEY]
      [--known-result-digest DIGEST]
      COMMAND [OPTIONS]

Core commands:
  init [PATH] --name TEXT [--deterministic-fake]
      Create a private project. Example: lkjwork init ./work --name product-next
  add TITLE [--description TEXT] [--priority I64] [--label TEXT ...]
      [--depends TASK ...] [--actor TEXT] [--idempotency-key KEY]
      Create a planned task. Example: lkjwork add 'Add pure queries' --priority 10
  edit TASK [--title TEXT] [--description TEXT] [--priority I64]
      Patch task fields. Example: lkjwork edit #1 --priority 20
  show TASK
      Show one task. Example: lkjwork show #1
  why TASK
      Explain exact actionability facts. Example: lkjwork why #1
  list [--after N] [--limit N] [--phase PHASE] [--label TEXT]
      [--readiness any|ready|blocked] [--archive default|archived|all]
      [--order id|priority]
      List a deterministic task page. Example: lkjwork list --phase planned --order priority
  next [--limit N]
      Ask the application for actionable work. Example: lkjwork next --limit 3
  summary
      Show project counts. Example: lkjwork summary
  context [--maximum-tasks N] [--maximum-notes N]
      [--maximum-dependencies N] [--maximum-text-bytes N]
      Request bounded coding-agent context. Example: lkjwork context --maximum-tasks 5
  export [--after N] [--limit N]
      Emit one deterministic semantic page. Example: lkjwork --json export --limit 100
  history [TASK] [--after N] [--limit N]
      Inspect bounded semantic activity. Example: lkjwork history #1 --limit 20
  rename TEXT [--idempotency-key KEY]
      Rename the project. Example: lkjwork rename product-next
  start|stop|finish|cancel|reopen TASK
      Apply an exact lifecycle transition. Example: lkjwork start #1
  priority TASK I64
      Set task priority. Example: lkjwork priority #1 20
  hold TASK --reason TEXT | release TASK
      Control the explicit manual hold. Example: lkjwork hold #1 --reason 'waiting on review'
  depend TASK --on TASK | undepend TASK --on TASK
      Change one prerequisite edge. Example: lkjwork depend #2 --on #1
  label TASK add TEXT | label TASK remove TEXT
      Change one exact label. Example: lkjwork label #1 add runtime
  note TASK add TEXT [--actor TEXT]
      Append an immutable note. Example: lkjwork note #1 add 'verified' --actor agent
  attach TASK FILE [--name TEXT] [--actor TEXT] [--idempotency-key KEY]
      [--fake-put CLASS] [--fake-inspect CLASS]
      Publish immutable evidence. Example: lkjwork attach #1 ./report.txt --actor agent
  archive|unarchive TASK
      Change terminal-task visibility. Example: lkjwork archive #1
  doctor [--deep]
      Validate project authority. Example: lkjwork doctor --deep
  backup --to PATH
      Publish and validate a no-replace exact backup. Example: lkjwork backup --to ../work.backup
  restore BACKUP --to PATH
      Restore semantic state into a new exact instance and rebind its deployment grant.
  session
      Serve bounded line-delimited machine requests in this foreground process.
  version
      Report product and embedded application identities.

Stored text is escaped in human output. --json emits exactly one versioned JSON value.";

#[derive(Clone)]
struct GlobalOptions {
    json: bool,
    project: Option<PathBuf>,
    base_revision: Option<u64>,
    idempotency_key: Option<String>,
    known_result_digest: Option<String>,
    arguments: Vec<String>,
    session_project: Option<Project>,
}

#[derive(Clone, Debug)]
struct Output {
    operation: String,
    instance: Option<String>,
    revision: Option<u64>,
    result: Value,
    human: String,
    exit: u8,
}

fn main() -> ExitCode {
    let options = match parse_globals(std::env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(message) => return write_error(false, EXIT_USAGE, "usage", &message),
    };
    let bindings = match Bindings::load() {
        Ok(bindings) => bindings,
        Err(message) => {
            return write_error(
                options.json,
                EXIT_INFRASTRUCTURE,
                "embedded_application",
                &message,
            );
        }
    };
    if options.arguments.as_slice() == ["session"] {
        return run_session(&bindings);
    }
    match execute(&options, &bindings) {
        Ok(output) => write_output(options.json, output),
        Err(error) => write_error(options.json, error.exit, error.code, &error.message),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionRequest {
    contract_version: u16,
    request_id: u64,
    project: Option<String>,
    base_revision: Option<u64>,
    idempotency_key: Option<String>,
    known_result_digest: Option<String>,
    arguments: Vec<String>,
}

fn run_session(bindings: &Bindings) -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut request_ids = BTreeSet::new();
    let mut cached_project: Option<(PathBuf, Project)> = None;
    loop {
        let frame = match read_session_line(&mut reader) {
            Ok(Some(frame)) => frame,
            Ok(None) => break,
            Err(error) => {
                eprintln!("lkjwork session input failed: {error}");
                return ExitCode::from(EXIT_INFRASTRUCTURE);
            }
        };
        let mut stop = false;
        let response = match frame {
            SessionLine::Oversized => session_error(
                None,
                "request_too_large",
                "session request exceeds byte policy",
            ),
            SessionLine::Value(bytes) => match lkjscript::instance::strict_json::<SessionRequest>(
                &bytes,
                "lkjwork session request",
            ) {
                Err(error) => session_error(None, "malformed_request", &error.to_string()),
                Ok(request)
                    if request.contract_version != PRODUCT_CONTRACT_VERSION
                        || request.request_id == 0
                        || request.arguments.is_empty()
                        || request.arguments.len() > MAXIMUM_SESSION_ARGUMENTS =>
                {
                    session_error(
                        Some(request.request_id),
                        "malformed_request",
                        "session request version, ID, operation, or argument count is invalid",
                    )
                }
                Ok(request) if !request_ids.insert(request.request_id) => session_error(
                    Some(request.request_id),
                    "duplicate_request_id",
                    "session request ID was already consumed",
                ),
                Ok(request) if request.arguments.as_slice() == ["shutdown"] => {
                    stop = true;
                    json!({
                        "contract_version": PRODUCT_CONTRACT_VERSION,
                        "request_id": request.request_id,
                        "response": {"operation": "shutdown", "result": {"shutdown": true}},
                    })
                }
                Ok(request)
                    if request
                        .arguments
                        .first()
                        .is_some_and(|value| value == "session") =>
                {
                    session_error(
                        Some(request.request_id),
                        "malformed_request",
                        "a session cannot recursively start another session",
                    )
                }
                Ok(request) => execute_session_request(request, bindings, &mut cached_project),
            },
        };
        let encoded = match serde_json::to_vec(&response) {
            Ok(encoded) if encoded.len() <= MAXIMUM_SESSION_LINE_BYTES => encoded,
            Ok(_) => serde_json::to_vec(&session_error(
                response.get("request_id").and_then(Value::as_u64),
                "response_too_large",
                "session response exceeds byte policy",
            ))
            .unwrap_or_default(),
            Err(error) => {
                eprintln!("lkjwork session output encoding failed: {error}");
                return ExitCode::from(EXIT_INFRASTRUCTURE);
            }
        };
        if writer
            .write_all(&encoded)
            .and_then(|_| writer.write_all(b"\n"))
            .and_then(|_| writer.flush())
            .is_err()
        {
            return ExitCode::from(EXIT_INFRASTRUCTURE);
        }
        if stop {
            break;
        }
    }
    ExitCode::SUCCESS
}

fn execute_session_request(
    request: SessionRequest,
    bindings: &Bindings,
    cached_project: &mut Option<(PathBuf, Project)>,
) -> Value {
    let request_id = request.request_id;
    let command = request.arguments.first().map(String::as_str).unwrap_or("");
    if command == "restore" {
        *cached_project = None;
    }
    let project_path = request.project.as_deref().map(PathBuf::from);
    let session_project = if session_requires_project(command) {
        let Some(path) = project_path.as_deref() else {
            return json!({
                "contract_version": PRODUCT_CONTRACT_VERSION,
                "request_id": request_id,
                "error": {
                    "code": "project_required",
                    "message": "session project operations require an explicit project path"
                },
                "exit": EXIT_USAGE,
            });
        };
        let key = match Project::session_cache_key(path) {
            Ok(key) => key,
            Err(message) => {
                return json!({
                    "contract_version": PRODUCT_CONTRACT_VERSION,
                    "request_id": request_id,
                    "error": {"code": "project_open", "message": message},
                    "exit": EXIT_INFRASTRUCTURE,
                });
            }
        };
        if cached_project
            .as_ref()
            .is_some_and(|(cached_key, _)| cached_key != &key)
        {
            *cached_project = None;
        }
        if cached_project.is_none() {
            let project = match Project::discover_session(Some(&key), bindings) {
                Ok(project) => project,
                Err(message) => {
                    return json!({
                        "contract_version": PRODUCT_CONTRACT_VERSION,
                        "request_id": request_id,
                        "error": {"code": "project_open", "message": message},
                        "exit": EXIT_INFRASTRUCTURE,
                    });
                }
            };
            *cached_project = Some((key, project));
        }
        let Some(project) = cached_project.as_ref().map(|(_, project)| project.clone()) else {
            return json!({
                "contract_version": PRODUCT_CONTRACT_VERSION,
                "request_id": request_id,
                "error": {
                    "code": "project_open",
                    "message": "session project cache was unavailable after exact open"
                },
                "exit": EXIT_INFRASTRUCTURE,
            });
        };
        if let Err(message) = project.revalidate_locator(bindings) {
            *cached_project = None;
            return json!({
                "contract_version": PRODUCT_CONTRACT_VERSION,
                "request_id": request_id,
                "error": {"code": "project_open", "message": message},
                "exit": EXIT_INFRASTRUCTURE,
            });
        }
        Some(project)
    } else {
        None
    };
    let options = GlobalOptions {
        json: true,
        project: project_path,
        base_revision: request.base_revision,
        idempotency_key: request.idempotency_key,
        known_result_digest: request.known_result_digest,
        arguments: request.arguments,
        session_project,
    };
    match execute(&options, bindings) {
        Ok(output) => json!({
            "contract_version": PRODUCT_CONTRACT_VERSION,
            "request_id": request_id,
            "response": output_value(&output),
            "exit": output.exit,
        }),
        Err(error) => json!({
            "contract_version": PRODUCT_CONTRACT_VERSION,
            "request_id": request_id,
            "error": {"code": error.code, "message": error.message},
            "exit": error.exit,
        }),
    }
}

fn session_requires_project(command: &str) -> bool {
    matches!(
        command,
        "add"
            | "edit"
            | "rename"
            | "start"
            | "stop"
            | "finish"
            | "cancel"
            | "reopen"
            | "priority"
            | "hold"
            | "release"
            | "depend"
            | "undepend"
            | "label"
            | "note"
            | "attach"
            | "archive"
            | "unarchive"
            | "show"
            | "why"
            | "list"
            | "next"
            | "summary"
            | "context"
            | "export"
            | "history"
            | "backup"
            | "doctor"
    )
}

enum SessionLine {
    Value(Vec<u8>),
    Oversized,
}

fn read_session_line(reader: &mut impl BufRead) -> std::io::Result<Option<SessionLine>> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if bytes.is_empty() && !oversized {
                return Ok(None);
            }
            break;
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(available.len());
        if !oversized {
            if bytes.len().saturating_add(take) > MAXIMUM_SESSION_LINE_BYTES {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..take]);
            }
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            break;
        }
    }
    if oversized {
        Ok(Some(SessionLine::Oversized))
    } else {
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        Ok(Some(SessionLine::Value(bytes)))
    }
}

fn session_error(request_id: Option<u64>, code: &str, message: &str) -> Value {
    json!({
        "contract_version": PRODUCT_CONTRACT_VERSION,
        "request_id": request_id,
        "error": {"code": code, "message": message},
    })
}

fn execute(options: &GlobalOptions, bindings: &Bindings) -> Result<Output, ProductError> {
    let Some(command) = options.arguments.first().map(String::as_str) else {
        return Ok(help_output());
    };
    let arguments = &options.arguments[1..];
    match command {
        "help" | "--help" if arguments.is_empty() => Ok(help_output()),
        "version" if arguments.is_empty() => Ok(Output {
            operation: "version".to_owned(),
            instance: None,
            revision: None,
            result: json!({
                "product_contract": PRODUCT_CONTRACT_VERSION,
                "instance_contract": INSTANCE_CONTRACT_VERSION,
                "application_digest": bindings.application_digest(),
                "release": bindings.release(),
            }),
            human: format!(
                "lkjwork contract {}\napplication {}\nrelease {}",
                PRODUCT_CONTRACT_VERSION,
                bindings.application_digest(),
                bindings.release(),
            ),
            exit: 0,
        }),
        "init" => initialize(arguments, bindings),
        "add" => add(options, arguments, bindings),
        "edit" => edit(options, arguments, bindings),
        "rename" => rename(options, arguments, bindings),
        "start" => id_mutation(options, arguments, bindings, "start", "start_task"),
        "stop" => id_mutation(options, arguments, bindings, "stop", "stop_task"),
        "finish" => id_mutation(options, arguments, bindings, "finish", "complete_task"),
        "cancel" => id_mutation(options, arguments, bindings, "cancel", "cancel_task"),
        "reopen" => id_mutation(options, arguments, bindings, "reopen", "reopen_task"),
        "priority" => priority(options, arguments, bindings),
        "hold" => hold(options, arguments, bindings),
        "release" => id_mutation(options, arguments, bindings, "release", "release_task"),
        "depend" => dependency(options, arguments, bindings, true),
        "undepend" => dependency(options, arguments, bindings, false),
        "label" => label(options, arguments, bindings),
        "note" => note(options, arguments, bindings),
        "attach" => attach(options, arguments, bindings),
        "archive" => id_mutation(options, arguments, bindings, "archive", "archive_task"),
        "unarchive" => id_mutation(options, arguments, bindings, "unarchive", "unarchive_task"),
        "show" => show(options, arguments, bindings),
        "why" => why(options, arguments, bindings),
        "list" => list(options, arguments, bindings, "list_tasks", "list"),
        "next" => next(options, arguments, bindings),
        "summary" if arguments.is_empty() => {
            simple_query(options, bindings, "project_summary", None, "summary")
        }
        "context" => context(options, arguments, bindings),
        "export" => list(options, arguments, bindings, "export_page", "export"),
        "history" => history(options, arguments, bindings),
        "backup" => backup(options, arguments, bindings),
        "restore" => restore(arguments, bindings),
        "doctor" => doctor(options, arguments, bindings),
        _ => Err(ProductError {
            exit: EXIT_USAGE,
            code: "usage",
            message: format!("unknown command or invalid arguments\n\n{HELP}"),
        }),
    }
}

fn initialize(arguments: &[String], bindings: &Bindings) -> CommandResult {
    let mut destination = PathBuf::from(".");
    let mut name = None;
    let mut adapter = HostAdapterKind::Production;
    let mut index = 0;
    if arguments
        .first()
        .is_some_and(|value| !value.starts_with('-'))
    {
        destination = PathBuf::from(&arguments[0]);
        index = 1;
    }
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--name" if name.is_none() => {
                index += 1;
                name = arguments.get(index).cloned();
            }
            "--deterministic-fake" => adapter = HostAdapterKind::DeterministicFake,
            _ => return usage_error("init requires [PATH] --name TEXT [--deterministic-fake]"),
        }
        index += 1;
    }
    let name = name.ok_or_else(|| usage_tuple("init requires --name TEXT"))?;
    if name.is_empty() {
        return usage_error("project name must not be empty");
    }
    bindings.text(&name)?;
    let (project, receipt) = Project::initialize(&destination, &name, bindings, adapter)
        .map_err(|message| infrastructure("project_initialization", message))?;
    Ok(Output {
        operation: "init".to_owned(),
        instance: Some(project.instance.to_string()),
        revision: Some(receipt.revision),
        result: json!({
            "project": project.root,
            "instance": project.instance,
            "application": receipt.application,
            "revision": receipt.revision,
            "state_digest": receipt.state_digest,
            "published": receipt.published,
        }),
        human: format!(
            "Initialized {} at {} (revision {}).\nNext: cd {} && lkjwork add 'First task'",
            render::terminal_text(&name),
            project.root.display(),
            receipt.revision,
            project.root.display(),
        ),
        exit: 0,
    })
}

fn add(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let Some(title) = arguments
        .first()
        .filter(|value| !value.starts_with('-'))
        .cloned()
    else {
        return usage_error("add requires TITLE");
    };
    let mut description = String::new();
    let mut priority = 0_i64;
    let mut labels = Vec::new();
    let mut dependencies = Vec::new();
    let mut actor = "user".to_owned();
    let mut event_key = None;
    let mut index = 1;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| usage_tuple(&format!("{option} requires a value")))?;
        match option {
            "--description" => description = value.clone(),
            "--priority" => priority = parse_i64(value, "priority")?,
            "--label" => labels.push(value.clone()),
            "--depends" => dependencies.push(parse_task_id(value)?),
            "--actor" => actor = value.clone(),
            "--idempotency-key" => event_key = Some(value.clone()),
            _ => return usage_error("invalid add option"),
        }
        index += 1;
    }
    let label_values = labels
        .iter()
        .map(|value| bindings.text(value))
        .collect::<Result<Vec<_>, _>>()?;
    let dependency_values = dependencies
        .into_iter()
        .map(|value| bindings.integer(value))
        .collect();
    let payload = bindings.product(
        "create_task_input",
        vec![
            ("title", bindings.text(&title)?),
            ("description", bindings.text(&description)?),
            ("priority", bindings.integer(priority)),
            ("labels", bindings.sequence("text_sequence", label_values)?),
            (
                "dependencies",
                bindings.sequence("id_sequence", dependency_values)?,
            ),
            ("actor", bindings.text(&actor)?),
        ],
    )?;
    mutate(
        options,
        bindings,
        "add",
        "create_task",
        Some(payload),
        event_key,
    )
}

fn rename(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let Some(name) = arguments.first() else {
        return usage_error("rename requires TEXT");
    };
    let mut event_key = None;
    match &arguments[1..] {
        [] => {}
        [flag, value] if flag == "--idempotency-key" => event_key = Some(value.clone()),
        _ => return usage_error("rename accepts only TEXT [--idempotency-key KEY]"),
    }
    mutate(
        options,
        bindings,
        "rename",
        "rename_project",
        Some(bindings.text(name)?),
        event_key,
    )
}

fn edit(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let Some(task) = arguments.first() else {
        return usage_error("edit requires TASK and at least one patch option");
    };
    let task = parse_task_id(task)?;
    let mut title = None;
    let mut description = None;
    let mut priority = None;
    let mut index = 1;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| usage_tuple(&format!("{option} requires a value")))?;
        match option {
            "--title" if title.is_none() => title = Some(value.clone()),
            "--description" if description.is_none() => description = Some(value.clone()),
            "--priority" if priority.is_none() => priority = Some(parse_i64(value, "priority")?),
            _ => {
                return usage_error(
                    "edit accepts --title, --description, and --priority once each",
                );
            }
        }
        index += 1;
    }
    if title.is_none() && description.is_none() && priority.is_none() {
        return usage_error("edit requires at least one patch option");
    }
    let payload = bindings.product(
        "edit_task_input",
        vec![
            ("task", bindings.integer(task)),
            ("set_title", bindings.boolean(title.is_some())),
            ("title", bindings.text(title.as_deref().unwrap_or(""))?),
            ("set_description", bindings.boolean(description.is_some())),
            (
                "description",
                bindings.text(description.as_deref().unwrap_or(""))?,
            ),
            ("set_priority", bindings.boolean(priority.is_some())),
            ("priority", bindings.integer(priority.unwrap_or(0))),
        ],
    )?;
    mutate(options, bindings, "edit", "edit_task", Some(payload), None)
}

fn id_mutation(
    options: &GlobalOptions,
    arguments: &[String],
    bindings: &Bindings,
    operation: &str,
    variant: &str,
) -> CommandResult {
    if arguments.len() != 1 {
        return usage_error(&format!("{operation} requires exactly one TASK"));
    }
    mutate(
        options,
        bindings,
        operation,
        variant,
        Some(bindings.integer(parse_task_id(&arguments[0])?)),
        None,
    )
}

fn priority(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    if arguments.len() != 2 {
        return usage_error("priority requires TASK I64");
    }
    let task = parse_task_id(&arguments[0])?;
    let value = parse_i64(&arguments[1], "priority")?;
    let payload = bindings.product(
        "priority_input",
        vec![
            ("task", bindings.integer(task)),
            ("priority", bindings.integer(value)),
        ],
    )?;
    mutate(
        options,
        bindings,
        "priority",
        "set_priority",
        Some(payload),
        None,
    )
}

fn hold(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let [task, flag, reason] = arguments else {
        return usage_error("hold requires TASK --reason TEXT");
    };
    if flag != "--reason" {
        return usage_error("hold requires TASK --reason TEXT");
    }
    let payload = bindings.product(
        "hold_input",
        vec![
            ("task", bindings.integer(parse_task_id(task)?)),
            ("reason", bindings.text(reason)?),
        ],
    )?;
    mutate(options, bindings, "hold", "hold_task", Some(payload), None)
}

fn dependency(
    options: &GlobalOptions,
    arguments: &[String],
    bindings: &Bindings,
    add: bool,
) -> CommandResult {
    let [task, flag, prerequisite] = arguments else {
        return usage_error("depend/undepend requires TASK --on PREREQUISITE");
    };
    if flag != "--on" {
        return usage_error("depend/undepend requires TASK --on PREREQUISITE");
    }
    let payload = bindings.product(
        "dependency_input",
        vec![
            ("task", bindings.integer(parse_task_id(task)?)),
            (
                "prerequisite",
                bindings.integer(parse_task_id(prerequisite)?),
            ),
        ],
    )?;
    mutate(
        options,
        bindings,
        if add { "depend" } else { "undepend" },
        if add {
            "add_dependency"
        } else {
            "remove_dependency"
        },
        Some(payload),
        None,
    )
}

fn label(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let [task, action, label] = arguments else {
        return usage_error("label requires TASK add|remove TEXT");
    };
    let variant = match action.as_str() {
        "add" => "add_label",
        "remove" => "remove_label",
        _ => return usage_error("label action must be add or remove"),
    };
    let payload = bindings.product(
        "label_input",
        vec![
            ("task", bindings.integer(parse_task_id(task)?)),
            ("label", bindings.text(label)?),
        ],
    )?;
    mutate(options, bindings, "label", variant, Some(payload), None)
}

fn note(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let [task, action, body, rest @ ..] = arguments else {
        return usage_error("note requires TASK add TEXT [--actor TEXT]");
    };
    if action != "add" {
        return usage_error("note supports only the add action");
    }
    let actor = match rest {
        [] => "user",
        [flag, actor] if flag == "--actor" => actor.as_str(),
        _ => return usage_error("note accepts only [--actor TEXT] after the body"),
    };
    let payload = bindings.product(
        "note_input",
        vec![
            ("task", bindings.integer(parse_task_id(task)?)),
            ("actor", bindings.text(actor)?),
            ("body", bindings.text(body)?),
        ],
    )?;
    mutate(options, bindings, "note", "add_note", Some(payload), None)
}

fn attach(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let [task, file, rest @ ..] = arguments else {
        return usage_error("attach requires TASK FILE [--name TEXT] [--actor TEXT]");
    };
    let task = parse_task_id(task)?;
    let path = PathBuf::from(file);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|error| infrastructure("attachment_source", error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(infrastructure(
            "attachment_source",
            "attachment source must be a non-symlink regular file".to_owned(),
        ));
    }
    if metadata.len() == 0 || metadata.len() > MAXIMUM_ATTACHMENT_BYTES {
        return Err(ProductError {
            exit: EXIT_USAGE,
            code: "input_limit",
            message: format!(
                "attachment source is {} bytes; the exact supported range is 1..={MAXIMUM_ATTACHMENT_BYTES}",
                metadata.len()
            ),
        });
    }
    let default_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            infrastructure(
                "attachment_source",
                "attachment file name is not UTF-8".to_owned(),
            )
        })?
        .to_owned();
    let mut name = default_name;
    let mut actor = "user".to_owned();
    let mut event_key = None;
    let mut fake_put = None;
    let mut fake_inspect = None;
    let mut index = 0;
    while index < rest.len() {
        let option = rest[index].as_str();
        index += 1;
        let value = rest
            .get(index)
            .ok_or_else(|| usage_tuple(&format!("{option} requires a value")))?;
        match option {
            "--name" => name = value.clone(),
            "--actor" => actor = value.clone(),
            "--idempotency-key" => event_key = Some(value.clone()),
            "--fake-put" => {
                fake_put = Some(match value.as_str() {
                    "succeeded" => HostOutcomeClass::Succeeded,
                    "already-present" => HostOutcomeClass::AlreadyPresent,
                    "failed" => HostOutcomeClass::KnownFailureBeforeVisibility,
                    "unknown" => HostOutcomeClass::OutcomeUnknown,
                    _ => {
                        return usage_error(
                            "--fake-put must be succeeded, already-present, failed, or unknown",
                        );
                    }
                });
            }
            "--fake-inspect" => {
                fake_inspect = Some(match value.as_str() {
                    "present" => HostOutcomeClass::ReconciliationPresent,
                    "absent" => HostOutcomeClass::ReconciliationAbsent,
                    "indeterminate" => HostOutcomeClass::ReconciliationIndeterminate,
                    _ => {
                        return usage_error(
                            "--fake-inspect must be present, absent, or indeterminate",
                        );
                    }
                });
            }
            _ => return usage_error("invalid attach option"),
        }
        index += 1;
    }
    let mut content = Vec::with_capacity(metadata.len() as usize);
    File::open(&path)
        .and_then(|file| {
            file.take(MAXIMUM_ATTACHMENT_BYTES + 1)
                .read_to_end(&mut content)
        })
        .map_err(|error| infrastructure("attachment_source", error.to_string()))?;
    if content.len() as u64 != metadata.len() || content.len() as u64 > MAXIMUM_ATTACHMENT_BYTES {
        return Err(infrastructure(
            "attachment_source",
            "attachment source changed while it was being read".to_owned(),
        ));
    }
    let payload = bindings.product(
        "attachment_input",
        vec![
            ("task", bindings.integer(task)),
            ("name", bindings.text(&name)?),
            ("actor", bindings.text(&actor)?),
            ("content", bindings.bytes(&content)?),
        ],
    )?;
    let project = open_project(options, bindings)?;
    let fake_outcomes = match (project.adapter, fake_put, fake_inspect) {
        (HostAdapterKind::Production, None, None) => None,
        (HostAdapterKind::Production, _, _) => {
            return usage_error("fake outcomes require a deterministic-fake project");
        }
        (HostAdapterKind::DeterministicFake, None, _) => {
            return usage_error("a deterministic-fake project requires --fake-put CLASS");
        }
        (HostAdapterKind::DeterministicFake, Some(HostOutcomeClass::OutcomeUnknown), None) => {
            return usage_error("an unknown fake put requires --fake-inspect CLASS");
        }
        (HostAdapterKind::DeterministicFake, Some(put), inspect) => Some(FakeAttachmentOutcomes {
            put,
            inspect: inspect.unwrap_or(HostOutcomeClass::ReconciliationIndeterminate),
        }),
    };
    let event = bindings.sum("mutation_event", "request_attachment", Some(payload))?;
    let (receipt, host_receipts, event_key) = project
        .attach(
            event,
            event_key.or_else(|| options.idempotency_key.clone()),
            options.base_revision,
            fake_outcomes,
        )
        .map_err(|message| infrastructure("attachment", message))?;
    let result = render::mutation_result(bindings, &receipt.response)
        .map_err(|message| infrastructure("application_result", message))?;
    let hosts = host_receipts
        .iter()
        .map(|host| {
            json!({
                "command": host.command,
                "operation": host.operation,
                "class": host.class,
                "evidence": hex_bytes(host.evidence.as_slice()),
                "replayed": host.replayed,
            })
        })
        .collect::<Vec<_>>();
    let exit = if result.get("kind").and_then(Value::as_str) == Some("conflict") {
        EXIT_DOMAIN_CONFLICT
    } else {
        0
    };
    Ok(Output {
        operation: "attach".to_owned(),
        instance: Some(project.instance.to_string()),
        revision: Some(receipt.next_revision),
        human: render::human_mutation(&result, receipt.next_revision, receipt.published),
        result: json!({
            "publication": format!("{:?}", receipt.status).to_ascii_lowercase(),
            "published": receipt.published,
            "replayed": receipt.replayed,
            "idempotency_key": event_key,
            "state_digest": receipt.state_digest,
            "host": hosts,
            "value": result,
        }),
        exit,
    })
}

fn mutate(
    options: &GlobalOptions,
    bindings: &Bindings,
    operation: &str,
    variant: &str,
    payload: Option<ApplicationValue>,
    event_key: Option<String>,
) -> CommandResult {
    let project = open_project(options, bindings)?;
    let event = bindings.sum("mutation_event", variant, payload)?;
    let (receipt, event_key) = project
        .mutate(
            event,
            event_key.or_else(|| options.idempotency_key.clone()),
            options.base_revision,
        )
        .map_err(|message| infrastructure("mutation", message))?;
    let result = render::mutation_result(bindings, &receipt.response)
        .map_err(|message| infrastructure("application_result", message))?;
    let exit = if receipt.status == InstanceTransitionStatus::Declined
        || result.get("kind").and_then(Value::as_str) == Some("conflict")
    {
        EXIT_DOMAIN_CONFLICT
    } else {
        0
    };
    Ok(Output {
        operation: operation.to_owned(),
        instance: Some(project.instance.to_string()),
        revision: Some(receipt.next_revision),
        human: render::human_mutation(&result, receipt.next_revision, receipt.published),
        result: json!({
            "publication": format!("{:?}", receipt.status).to_ascii_lowercase(),
            "published": receipt.published,
            "replayed": receipt.replayed,
            "idempotency_key": event_key,
            "state_digest": receipt.state_digest,
            "value": result,
        }),
        exit,
    })
}

fn show(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    if arguments.len() != 1 {
        return usage_error("show requires exactly one TASK");
    }
    simple_query(
        options,
        bindings,
        "get_task",
        Some(bindings.integer(parse_task_id(&arguments[0])?)),
        "show",
    )
}

fn why(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    if arguments.len() != 1 {
        return usage_error("why requires exactly one TASK");
    }
    simple_query(
        options,
        bindings,
        "why",
        Some(bindings.integer(parse_task_id(&arguments[0])?)),
        "why",
    )
}

fn list(
    options: &GlobalOptions,
    arguments: &[String],
    bindings: &Bindings,
    variant: &str,
    operation: &str,
) -> CommandResult {
    let mut after = 0_i64;
    let mut limit = 20_i64;
    let mut phase = "any";
    let mut label = None;
    let mut readiness = "any";
    let mut archive = "default";
    let mut order = "id";
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| usage_tuple(&format!("{flag} requires a value")))?;
        match flag {
            "--after" => after = parse_nonnegative(value, "after")?,
            "--limit" => limit = parse_positive(value, "limit")?,
            "--phase" if variant == "list_tasks" => {
                phase = match value.as_str() {
                    "any" | "planned" | "active" | "done" | "cancelled" => value,
                    _ => {
                        return usage_error(
                            "phase must be any, planned, active, done, or cancelled",
                        );
                    }
                };
            }
            "--label" if variant == "list_tasks" && label.is_none() => label = Some(value.clone()),
            "--readiness" if variant == "list_tasks" => {
                readiness = match value.as_str() {
                    "any" | "ready" | "blocked" => value,
                    _ => return usage_error("readiness must be any, ready, or blocked"),
                };
            }
            "--archive" if variant == "list_tasks" => {
                archive = match value.as_str() {
                    "default" | "archived" | "all" => value,
                    _ => return usage_error("archive must be default, archived, or all"),
                };
            }
            "--order" if variant == "list_tasks" => {
                order = match value.as_str() {
                    "id" | "priority" => value,
                    _ => return usage_error("order must be id or priority"),
                };
            }
            _ => return usage_error("invalid list/export option"),
        }
        index += 1;
    }
    let page = if variant == "list_tasks" {
        let label_filter = match label {
            Some(value) => bindings.sum("label_filter", "exact", Some(bindings.text(&value)?))?,
            None => bindings.sum("label_filter", "any", None)?,
        };
        bindings.product(
            "list_request",
            vec![
                ("after", bindings.integer(after)),
                ("limit", bindings.integer(limit)),
                ("phase", bindings.sum("phase_filter", phase, None)?),
                (
                    "readiness",
                    bindings.sum("readiness_filter", readiness, None)?,
                ),
                ("label", label_filter),
                ("archive", bindings.sum("archive_filter", archive, None)?),
                ("order", bindings.sum("task_order", order, None)?),
            ],
        )?
    } else {
        bindings.product(
            "page_request",
            vec![
                ("after", bindings.integer(after)),
                ("limit", bindings.integer(limit)),
            ],
        )?
    };
    let mut output = simple_query(options, bindings, variant, Some(page), operation)?;
    if operation == "export" {
        output.result["export_version"] = json!(PRODUCT_EXPORT_VERSION);
    }
    Ok(output)
}

fn next(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let limit = match arguments {
        [] => 1,
        [flag, value] if flag == "--limit" => parse_positive(value, "limit")?,
        _ => return usage_error("next accepts only [--limit N]"),
    };
    simple_query(
        options,
        bindings,
        "next_tasks",
        Some(bindings.integer(limit)),
        "next",
    )
}

fn context(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let mut maximum_tasks = 5_i64;
    let mut maximum_notes = 10_i64;
    let mut maximum_dependencies = 20_i64;
    let mut maximum_text_bytes = 32_768_i64;
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| usage_tuple(&format!("{flag} requires a value")))?;
        let parsed = parse_positive(value, flag)?;
        match flag {
            "--maximum-tasks" => maximum_tasks = parsed,
            "--maximum-notes" => maximum_notes = parsed,
            "--maximum-dependencies" => maximum_dependencies = parsed,
            "--maximum-text-bytes" => maximum_text_bytes = parsed,
            _ => return usage_error("invalid context bound"),
        }
        index += 1;
    }
    let payload = bindings.product(
        "context_request",
        vec![
            ("maximum_tasks", bindings.integer(maximum_tasks)),
            ("maximum_notes", bindings.integer(maximum_notes)),
            (
                "maximum_dependencies",
                bindings.integer(maximum_dependencies),
            ),
            ("maximum_text_bytes", bindings.integer(maximum_text_bytes)),
        ],
    )?;
    simple_query(options, bindings, "agent_context", Some(payload), "context")
}

fn history(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let mut task = None;
    let mut after = 0_i64;
    let mut limit = 20_i64;
    let mut index = 0;
    if arguments
        .first()
        .is_some_and(|value| !value.starts_with('-'))
    {
        task = Some(parse_task_id(&arguments[0])?);
        index = 1;
    }
    while index < arguments.len() {
        let option = arguments[index].as_str();
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| usage_tuple(&format!("{option} requires a value")))?;
        match option {
            "--after" => after = parse_nonnegative(value, "after")?,
            "--limit" => limit = parse_positive(value, "limit")?,
            _ => return usage_error("history accepts [TASK] [--after N] [--limit N]"),
        }
        index += 1;
    }
    let filter = match task {
        Some(task) => bindings.sum("activity_filter", "task", Some(bindings.integer(task)))?,
        None => bindings.sum("activity_filter", "all", None)?,
    };
    let request = bindings.product(
        "activity_request",
        vec![
            ("after", bindings.integer(after)),
            ("limit", bindings.integer(limit)),
            ("filter", filter),
        ],
    )?;
    simple_query(
        options,
        bindings,
        "recent_activity",
        Some(request),
        "history",
    )
}

fn simple_query(
    options: &GlobalOptions,
    bindings: &Bindings,
    variant: &str,
    payload: Option<ApplicationValue>,
    operation: &str,
) -> CommandResult {
    let project = open_project(options, bindings)?;
    let query = bindings.sum("query", variant, payload)?;
    let receipt = project
        .query(query)
        .map_err(|message| infrastructure("query", message))?;
    let known_result_digest = options
        .known_result_digest
        .as_deref()
        .map(|value| {
            value
                .parse::<QueryResultDigest>()
                .map_err(|_| usage_tuple("--known-result-digest must be canonical lowercase hex"))
        })
        .transpose()?;
    if known_result_digest == Some(receipt.result_digest) {
        return Ok(Output {
            operation: operation.to_owned(),
            instance: Some(project.instance.to_string()),
            revision: Some(receipt.selected_revision),
            human: format!(
                "Unchanged at revision {} (result {}).",
                receipt.selected_revision, receipt.result_digest
            ),
            result: json!({
                "published": false,
                "unchanged": true,
                "state_digest": receipt.state_digest,
                "result_digest": receipt.result_digest,
            }),
            exit: 0,
        });
    }
    let result = render::query_result(bindings, &receipt.result)
        .map_err(|message| infrastructure("application_result", message))?;
    let result = if operation == "why" {
        render::why_result(&result)
            .map_err(|message| infrastructure("application_result", message))?
    } else {
        result
    };
    let exit = if matches!(
        result.get("kind").and_then(Value::as_str),
        Some("not_found" | "error")
    ) {
        EXIT_DOMAIN_CONFLICT
    } else {
        0
    };
    Ok(Output {
        operation: operation.to_owned(),
        instance: Some(project.instance.to_string()),
        revision: Some(receipt.selected_revision),
        human: render::human_query(&result, receipt.selected_revision),
        result: json!({
            "published": receipt.published,
            "state_digest": receipt.state_digest,
            "result_digest": receipt.result_digest,
            "value": result,
        }),
        exit,
    })
}

fn doctor(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let deep = match arguments {
        [] => false,
        [flag] if flag == "--deep" => true,
        _ => return usage_error("doctor accepts only [--deep]"),
    };
    let project = open_project(options, bindings)?;
    let inspection = if deep {
        project.inspect_deep()
    } else {
        project.inspect()
    }
    .map_err(|message| infrastructure("instance_validation", message))?;
    let blobs = project.blobs_directory();
    let metadata = std::fs::symlink_metadata(&blobs)
        .map_err(|error| infrastructure("blob_validation", error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(infrastructure(
            "blob_validation",
            "blob namespace is not a regular directory".to_owned(),
        ));
    }
    let closure = validate_attachment_closure(&project, bindings)
        .map_err(|message| infrastructure("blob_validation", message))?;
    Ok(Output {
        operation: "doctor".to_owned(),
        instance: Some(project.instance.to_string()),
        revision: Some(inspection.revision),
        result: json!({
            "valid": true,
            "scope": if deep { "complete_replay" } else { "ordinary_open" },
            "application": inspection.application,
            "locator_application": project.application,
            "revision": inspection.revision,
            "record_digest": inspection.record_digest,
            "state_digest": inspection.state_digest,
            "history_records": inspection.history_records,
            "history_bytes": inspection.history_bytes,
            "checkpoint_revision": inspection.checkpoint_revision,
            "normal_replay_records": inspection.normal_replay_records,
            "current_state_cache": inspection.current_state_cache,
            "deep_audited": inspection.deep_audited,
            "attachment_objects": closure.objects,
            "attachment_bytes": closure.bytes,
            "referenced_attachments": closure.referenced,
            "orphan_attachments": closure.orphans,
        }),
        human: format!(
            "Project is valid through revision {} ({}; {} records, {} bytes; {} attachment objects).",
            inspection.revision,
            if deep {
                "complete replay"
            } else {
                "ordinary open"
            },
            inspection.history_records,
            inspection.history_bytes,
            closure.objects,
        ),
        exit: 0,
    })
}

fn backup(options: &GlobalOptions, arguments: &[String], bindings: &Bindings) -> CommandResult {
    let [flag, destination] = arguments else {
        return usage_error("backup requires --to PATH");
    };
    if flag != "--to" {
        return usage_error("backup requires --to PATH");
    }
    let project = open_project(options, bindings)?;
    validate_attachment_closure(&project, bindings)
        .map_err(|message| infrastructure("backup_source_validation", message))?;
    let (copied, receipt) = project
        .backup(&PathBuf::from(destination), bindings)
        .map_err(|message| infrastructure("backup", message))?;
    let closure = validate_attachment_closure(&copied, bindings)
        .map_err(|message| infrastructure("backup_validation", message))?;
    Ok(Output {
        operation: "backup".to_owned(),
        instance: Some(copied.instance.to_string()),
        revision: Some(receipt.revision),
        result: json!({
            "destination": copied.root,
            "revision": receipt.revision,
            "state_digest": receipt.state_digest,
            "files": receipt.files,
            "bytes": receipt.bytes,
            "attachment_objects": closure.objects,
            "attachment_bytes": closure.bytes,
            "referenced_attachments": closure.referenced,
            "validated": true,
        }),
        human: format!(
            "Backed up revision {} to {} ({} files, {} bytes; validated).",
            receipt.revision,
            copied.root.display(),
            receipt.files,
            receipt.bytes,
        ),
        exit: 0,
    })
}

fn restore(arguments: &[String], bindings: &Bindings) -> CommandResult {
    let [source, flag, destination] = arguments else {
        return usage_error("restore requires BACKUP --to PATH");
    };
    if flag != "--to" {
        return usage_error("restore requires BACKUP --to PATH");
    }
    let source = Project::discover(Some(&PathBuf::from(source)), bindings)
        .map_err(|message| infrastructure("backup_open", message))?;
    validate_attachment_closure(&source, bindings)
        .map_err(|message| infrastructure("backup_validation", message))?;
    let source_instance = source.instance;
    let source_revision = source
        .inspect_deep()
        .map_err(|message| infrastructure("backup_validation", message))?
        .revision;
    let (restored, receipt) = Project::restore_from(&source, &PathBuf::from(destination), bindings)
        .map_err(|message| infrastructure("restore", message))?;
    let closure = validate_attachment_closure(&restored, bindings)
        .map_err(|message| infrastructure("restore_validation", message))?;
    Ok(Output {
        operation: "restore".to_owned(),
        instance: Some(restored.instance.to_string()),
        revision: Some(receipt.revision),
        result: json!({
            "source_instance": source_instance,
            "source_revision": source_revision,
            "instance": restored.instance,
            "revision": receipt.revision,
            "state_digest": receipt.state_digest,
            "destination": restored.root,
            "referenced_attachments": closure.referenced,
            "validated": true,
        }),
        human: format!(
            "Restored source revision {} into new instance {} at {} (revision 0; validated).",
            source_revision,
            restored.instance,
            restored.root.display(),
        ),
        exit: 0,
    })
}

#[derive(Clone, Copy, Debug)]
struct AttachmentClosure {
    objects: u64,
    bytes: u64,
    referenced: u64,
    orphans: u64,
}

fn validate_attachment_closure(
    project: &Project,
    bindings: &Bindings,
) -> Result<AttachmentClosure, String> {
    let mut expected = BTreeSet::new();
    let mut after = 0_i64;
    loop {
        let request = bindings.product(
            "page_request",
            vec![
                ("after", bindings.integer(after)),
                ("limit", bindings.integer(100)),
            ],
        )?;
        let query = bindings.sum("query", "export_page", Some(request))?;
        let receipt = project.query(query)?;
        let result = render::query_result(bindings, &receipt.result)?;
        let tasks = result
            .get("tasks")
            .and_then(Value::as_array)
            .ok_or_else(|| "export query did not return a task page".to_owned())?;
        for task in tasks {
            for attachment in task
                .get("attachments")
                .and_then(Value::as_array)
                .ok_or_else(|| "export task omitted attachments".to_owned())?
            {
                expected.insert(
                    attachment
                        .get("digest")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "export attachment omitted its digest".to_owned())?
                        .to_owned(),
                );
            }
        }
        let total = result.get("total").and_then(Value::as_i64).unwrap_or(0);
        let next = result
            .get("next_after")
            .and_then(Value::as_i64)
            .ok_or_else(|| "export page omitted its cursor".to_owned())?;
        if next >= total {
            break;
        }
        if next <= after {
            return Err("export page cursor did not advance".to_owned());
        }
        after = next;
    }
    let blobs = project.blobs_directory();
    let mut actual = BTreeSet::new();
    let mut objects = 0_u64;
    let mut bytes_total = 0_u64;
    let mut entries = std::fs::read_dir(&blobs)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "blob namespace contains nonregular object {}",
                path.display()
            ));
        }
        if metadata.len() > MAXIMUM_BLOB_BYTES as u64 {
            return Err(format!(
                "blob object {} exceeds the global byte policy",
                path.display()
            ));
        }
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| "blob object name is not UTF-8".to_owned())?;
        let named = file_name
            .strip_suffix(".lkjb")
            .ok_or_else(|| format!("blob namespace contains foreign object {file_name}"))?;
        let content = std::fs::read(&path).map_err(|error| error.to_string())?;
        let digest = immutable_blob_digest(&content).to_string();
        if named != digest {
            return Err(format!(
                "blob object {file_name} does not match its content digest"
            ));
        }
        if !actual.insert(digest) {
            return Err("blob namespace contains duplicate content identity".to_owned());
        }
        objects = objects
            .checked_add(1)
            .ok_or_else(|| "blob object count overflows".to_owned())?;
        bytes_total = bytes_total
            .checked_add(content.len() as u64)
            .ok_or_else(|| "blob byte count overflows".to_owned())?;
    }
    for digest in &expected {
        if !actual.contains(digest) {
            return Err(format!("referenced attachment object {digest} is missing"));
        }
    }
    Ok(AttachmentClosure {
        objects,
        bytes: bytes_total,
        referenced: expected.len() as u64,
        orphans: actual.len().saturating_sub(expected.len()) as u64,
    })
}

fn open_project(options: &GlobalOptions, bindings: &Bindings) -> Result<Project, ErrorTuple> {
    if let Some(project) = &options.session_project {
        return Ok(project.clone());
    }
    Project::discover(options.project.as_deref(), bindings)
        .map_err(|message| infrastructure("project_open", message))
}

fn parse_globals(arguments: Vec<String>) -> Result<GlobalOptions, String> {
    let mut json = false;
    let mut project = None;
    let mut base_revision = None;
    let mut idempotency_key = None;
    let mut known_result_digest = None;
    let mut retained = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--json" if !json => json = true,
            "--project" if project.is_none() => {
                index += 1;
                project = Some(PathBuf::from(
                    arguments
                        .get(index)
                        .ok_or_else(|| "--project requires a path".to_owned())?,
                ));
            }
            "--base-revision" if base_revision.is_none() => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| "--base-revision requires a value".to_owned())?;
                base_revision = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| "--base-revision must be a canonical u64".to_owned())?,
                );
            }
            "--idempotency-key" if idempotency_key.is_none() => {
                index += 1;
                idempotency_key = Some(
                    arguments
                        .get(index)
                        .ok_or_else(|| "--idempotency-key requires a value".to_owned())?
                        .clone(),
                );
            }
            "--known-result-digest" if known_result_digest.is_none() => {
                index += 1;
                known_result_digest = Some(
                    arguments
                        .get(index)
                        .ok_or_else(|| "--known-result-digest requires a value".to_owned())?
                        .clone(),
                );
            }
            value => retained.push(value.to_owned()),
        }
        index += 1;
    }
    Ok(GlobalOptions {
        json,
        project,
        base_revision,
        idempotency_key,
        known_result_digest,
        arguments: retained,
        session_project: None,
    })
}

fn parse_task_id(value: &str) -> Result<i64, ErrorTuple> {
    let value = value.strip_prefix('#').unwrap_or(value);
    parse_positive(value, "task ID")
}

fn parse_positive(value: &str, label: &str) -> Result<i64, ErrorTuple> {
    let parsed = parse_i64(value, label)?;
    if parsed <= 0 {
        return Err(usage_tuple(&format!("{label} must be positive")));
    }
    Ok(parsed)
}

fn parse_nonnegative(value: &str, label: &str) -> Result<i64, ErrorTuple> {
    let parsed = parse_i64(value, label)?;
    if parsed < 0 {
        return Err(usage_tuple(&format!("{label} must be nonnegative")));
    }
    Ok(parsed)
}

fn parse_i64(value: &str, label: &str) -> Result<i64, ErrorTuple> {
    value
        .parse::<i64>()
        .map_err(|_| usage_tuple(&format!("{label} must be a canonical i64")))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn help_output() -> Output {
    Output {
        operation: "help".to_owned(),
        instance: None,
        revision: None,
        result: json!({"help": HELP}),
        human: HELP.to_owned(),
        exit: 0,
    }
}

fn write_output(machine: bool, output: Output) -> ExitCode {
    if machine {
        let envelope = output_value(&output);
        match serde_json::to_string(&envelope) {
            Ok(encoded) => println!("{encoded}"),
            Err(error) => {
                return write_error(true, EXIT_INFRASTRUCTURE, "output", &error.to_string());
            }
        }
    } else {
        println!("{}", output.human);
    }
    ExitCode::from(output.exit)
}

fn output_value(output: &Output) -> Value {
    json!({
        "contract_version": PRODUCT_CONTRACT_VERSION,
        "operation": output.operation,
        "instance": output.instance,
        "revision": output.revision,
        "result": output.result,
    })
}

fn write_error(machine: bool, exit: u8, code: &str, message: &str) -> ExitCode {
    if machine {
        let value = json!({
            "contract_version": PRODUCT_CONTRACT_VERSION,
            "error": {"code": code, "message": message},
        });
        match serde_json::to_string(&value) {
            Ok(encoded) => println!("{encoded}"),
            Err(error) => eprintln!("cannot encode lkjwork error: {error}"),
        }
    } else {
        eprintln!("lkjwork: {message}");
    }
    ExitCode::from(exit)
}

#[derive(Clone, Debug)]
struct ProductError {
    exit: u8,
    code: &'static str,
    message: String,
}

type ErrorTuple = ProductError;
type CommandResult = Result<Output, ProductError>;

fn usage_tuple(message: &str) -> ErrorTuple {
    ProductError {
        exit: EXIT_USAGE,
        code: "usage",
        message: message.to_owned(),
    }
}

fn usage_error(message: &str) -> CommandResult {
    Err(usage_tuple(message))
}

fn infrastructure(code: &'static str, message: String) -> ErrorTuple {
    ProductError {
        exit: EXIT_INFRASTRUCTURE,
        code,
        message,
    }
}

impl From<String> for ProductError {
    fn from(message: String) -> Self {
        if message.starts_with("product input requests ") {
            ProductError {
                exit: EXIT_USAGE,
                code: "input_limit",
                message,
            }
        } else {
            infrastructure("application_input", message)
        }
    }
}
