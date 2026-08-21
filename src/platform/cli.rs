//! Strict public command projection for the source-authored platform.

use super::artifact::{MAXIMUM_ARTIFACT_BYTES, load_artifact};
use super::authority::ProjectAuthority;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{PreparedProgram, ReferenceInterpreter, RunPolicy, Vm};
use super::json::{JsonLimits, decode_strict, decode_typed, encode_typed};
use super::package::RunnerKind;
use super::workspace::SourceWorkspace;
use serde::Serialize;
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const CLI_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CliSuccess {
    pub contract_version: u16,
    pub ok: bool,
    pub command: String,
    pub result: serde_json::Value,
}

pub fn execute(arguments: Vec<String>) -> Result<CliSuccess, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    let command = arguments.first().map(String::as_str).unwrap_or("help");
    match command {
        "help" => {
            exact_arguments(&arguments, 1, "help")?;
            success("help", json!({"usage": usage()}))
        }
        "project" => project_command(&arguments[1..], project.as_deref()),
        "module" => module_command(&arguments[1..], project.as_deref()),
        "component" => component_command(&arguments[1..], project.as_deref()),
        "package" => package_command(&arguments[1..], project.as_deref()),
        "artifact" => artifact_command(&arguments[1..]),
        "target" => target_command(&arguments[1..], project.as_deref()),
        other => Err(usage_error(format!("unknown command '{other}'"))),
    }
}

fn component_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 2 || arguments[0] != "inspect" {
        return Err(usage_error(
            "component inspect requires one local module.Component name",
        ));
    }
    let workspace = open_workspace(project)?;
    let (artifact_bytes, _) = workspace.build_artifact()?;
    let program = PreparedProgram::prepare(load_artifact(&artifact_bytes)?)?;
    let root = &program.artifact().root_package_id;
    let component = program
        .components()
        .values()
        .find(|component| {
            component.owner.package == *root
                && format!("{}.{}", component.owner.module, component.owner.declaration)
                    == arguments[1]
        })
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "component_missing",
                format!("package has no local component '{}'", arguments[1]),
            )
        })?;
    let requirements = component
        .requirements
        .values()
        .map(|requirement| {
            json!({
                "alias": requirement.alias,
                "interface": requirement.interface,
                "operations": requirement.operations,
                "limits": requirement.limits,
            })
        })
        .collect::<Vec<_>>();
    let ports = component
        .ports
        .values()
        .map(|port| {
            json!({
                "name": port.name,
                "function": port.function,
                "parameters": port.signature.parameters,
                "result": port.signature.result,
                "task_capabilities": port.signature.task_capabilities,
            })
        })
        .collect::<Vec<_>>();
    success(
        "component.inspect",
        json!({
            "owner": component.owner,
            "requirements": requirements,
            "ports": ports,
        }),
    )
}

fn project_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let subcommand = arguments.first().map(String::as_str).ok_or_else(|| {
        usage_error("project requires init, orient, status, validate, apply, history, doctor, restore, backup, or restore-backup")
    })?;
    match subcommand {
        "init" => {
            exact_arguments(arguments, 1, "project init")?;
            let root = project_or_current(project)?;
            let (_, receipt) = SourceWorkspace::initialize(&root)?;
            serialized("project.init", &receipt)
        }
        "orient" => {
            exact_arguments(arguments, 1, "project orient")?;
            let workspace = open_workspace(project)?;
            serialized("project.orient", &workspace.orient()?)
        }
        "status" => {
            exact_arguments(arguments, 1, "project status")?;
            let workspace = open_workspace(project)?;
            serialized("project.status", &workspace.status()?)
        }
        "validate" | "apply" => {
            let (revision, record) = exact_base(&arguments[1..])?;
            let workspace = open_workspace(project)?;
            let receipt = if subcommand == "validate" {
                workspace.validate(revision, &record)?
            } else {
                workspace.apply(revision, &record)?
            };
            serialized(&format!("project.{subcommand}"), &receipt)
        }
        "history" => {
            let limit = optional_usize(&arguments[1..], "--limit", 50)?;
            let before = optional_u64_value(&arguments[1..], "--before")?;
            reject_unknown_options(&arguments[1..], &["--limit", "--before"])?;
            let workspace = open_workspace(project)?;
            serialized(
                "project.history",
                &workspace.authority().history(before, limit)?,
            )
        }
        "doctor" => {
            let deep = match arguments.get(1).map(String::as_str) {
                None => false,
                Some("--deep") if arguments.len() == 2 => true,
                _ => return Err(usage_error("project doctor accepts only optional --deep")),
            };
            let workspace = open_workspace(project)?;
            serialized("project.doctor", &workspace.authority().doctor(deep)?)
        }
        "restore" => {
            if arguments.len() != 3 || arguments[1] != "--revision" {
                return Err(usage_error(
                    "project restore requires --revision <accepted-revision>",
                ));
            }
            let revision = parse_u64(&arguments[2], "revision")?;
            let workspace = open_workspace(project)?;
            workspace.authority().restore_working(revision)?;
            success(
                "project.restore",
                json!({"revision": revision, "published": false}),
            )
        }
        "backup" => {
            if arguments.len() != 3 || arguments[1] != "--output" {
                return Err(usage_error("project backup requires --output <directory>"));
            }
            let workspace = open_workspace(project)?;
            serialized(
                "project.backup",
                &workspace.authority().backup_to(Path::new(&arguments[2]))?,
            )
        }
        "restore-backup" => {
            if arguments.len() != 5 || arguments[1] != "--backup" || arguments[3] != "--output" {
                return Err(usage_error(
                    "project restore-backup requires --backup <directory> --output <new-project>",
                ));
            }
            serialized(
                "project.restore-backup",
                &ProjectAuthority::restore_backup(
                    Path::new(&arguments[2]),
                    Path::new(&arguments[4]),
                )?,
            )
        }
        other => Err(usage_error(format!("unknown project command '{other}'"))),
    }
}

fn module_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let subcommand = arguments
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage_error("module requires list or show"))?;
    let workspace = open_workspace(project)?;
    match subcommand {
        "list" => {
            exact_arguments(arguments, 1, "module list")?;
            let orientation = workspace.orient()?;
            success("module.list", json!({"modules": orientation.modules}))
        }
        "show" => {
            if arguments.len() != 2 {
                return Err(usage_error("module show requires one module name"));
            }
            let bytes = workspace.module_source(&arguments[1])?;
            let source = String::from_utf8(bytes).map_err(|_| {
                Diagnostic::new(
                    DiagnosticClass::Corrupt,
                    "module_source_utf8",
                    "accepted module source is not UTF-8",
                )
            })?;
            success(
                "module.show",
                json!({
                    "module": arguments[1],
                    "source": source,
                    "declarations": workspace.declaration_names(&arguments[1])?,
                }),
            )
        }
        other => Err(usage_error(format!("unknown module command '{other}'"))),
    }
}

fn package_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let subcommand = arguments
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage_error("package requires build or test"))?;
    let workspace = open_workspace(project)?;
    match subcommand {
        "build" => {
            let output = option_value(&arguments[1..], "--output")?
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace.root().join("target/application.lkja"));
            reject_unknown_options(&arguments[1..], &["--output"])?;
            let (bytes, receipt) = workspace.build_artifact()?;
            let publication = write_immutable_output(&output, &bytes)?;
            let mut value = serde_json::to_value(receipt).map_err(internal_json)?;
            value["output"] = json!(output.display().to_string());
            value["publication"] = json!(publication);
            success("package.build", value)
        }
        "test" => {
            exact_arguments(arguments, 1, "package test")?;
            let (bytes, _) = workspace.build_artifact()?;
            let program = PreparedProgram::prepare(load_artifact(&bytes)?)?;
            run_package_tests(&program)
        }
        other => Err(usage_error(format!("unknown package command '{other}'"))),
    }
}

fn run_package_tests(program: &PreparedProgram) -> Result<CliSuccess, Diagnostic> {
    let mut production_instructions = 0u64;
    let mut oracle_instructions = 0u64;
    let mut production_elapsed_nanoseconds = 0u64;
    let mut oracle_elapsed_nanoseconds = 0u64;
    for test in program.tests() {
        let production_started = Instant::now();
        let (actual, actual_observation) = Vm::new(program, RunPolicy::default())
            .invoke(&test.actual, Vec::new())
            .map_err(execution_diagnostic)?;
        let (expected, expected_observation) = Vm::new(program, RunPolicy::default())
            .invoke(&test.expected, Vec::new())
            .map_err(execution_diagnostic)?;
        production_elapsed_nanoseconds = production_elapsed_nanoseconds.saturating_add(
            u64::try_from(production_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        );
        let oracle_started = Instant::now();
        let (oracle_actual, oracle_actual_observation) =
            ReferenceInterpreter::new(program, RunPolicy::default())
                .invoke(&test.actual, Vec::new())
                .map_err(execution_diagnostic)?;
        let (oracle_expected, oracle_expected_observation) =
            ReferenceInterpreter::new(program, RunPolicy::default())
                .invoke(&test.expected, Vec::new())
                .map_err(execution_diagnostic)?;
        oracle_elapsed_nanoseconds = oracle_elapsed_nanoseconds
            .saturating_add(u64::try_from(oracle_started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        production_instructions = production_instructions
            .saturating_add(actual_observation.instructions)
            .saturating_add(expected_observation.instructions);
        oracle_instructions = oracle_instructions
            .saturating_add(oracle_actual_observation.instructions)
            .saturating_add(oracle_expected_observation.instructions);
        let actual = actual.canonical_json();
        let expected = expected.canonical_json();
        let oracle_actual = oracle_actual.canonical_json();
        let oracle_expected = oracle_expected.canonical_json();
        if actual != oracle_actual || expected != oracle_expected {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "package_test_differential",
                format!(
                    "production and oracle execution disagree for {}::{}::{}",
                    test.package.as_str(),
                    test.module,
                    test.name
                ),
            ));
        }
        if actual != expected || oracle_actual != oracle_expected {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "package_test_failed",
                format!(
                    "test {}::{}::{} did not equal its expected value",
                    test.package.as_str(),
                    test.module,
                    test.name
                ),
            ));
        }
    }
    success(
        "package.test",
        json!({
            "passed": program.tests().len(),
            "failed": 0,
            "production_tier": "bytecode_v1",
            "oracle_tier": "reference_ast_v1",
            "production_instructions": production_instructions,
            "oracle_instructions": oracle_instructions,
            "production_elapsed_nanoseconds": production_elapsed_nanoseconds,
            "oracle_elapsed_nanoseconds": oracle_elapsed_nanoseconds,
            "differential": "equal",
        }),
    )
}

fn artifact_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 2 || arguments[0] != "inspect" {
        return Err(usage_error("artifact inspect requires one artifact path"));
    }
    let bytes = read_bounded(
        Path::new(&arguments[1]),
        MAXIMUM_ARTIFACT_BYTES,
        "component artifact",
    )?;
    let artifact = load_artifact(&bytes)?;
    let program = PreparedProgram::prepare(artifact)?;
    let packages = program
        .artifact()
        .packages
        .values()
        .map(|package| {
            json!({
                "package_id": package.descriptor.package_id,
                "name": package.descriptor.name,
                "revision_digest": package.revision_digest,
                "package_artifact_digest": super::artifact::package_artifact_digest(package),
                "modules": package.modules.iter().map(|module| module.module.name.clone()).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let targets = program
        .targets()
        .values()
        .map(|target| {
            let requirements = program
                .components()
                .get(&target.component)
                .map(|component| component.requirements.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            json!({
                "name": target.name,
                "runner": target.runner,
                "component": target.component,
                "port": target.port.name,
                "parameters": target.port.signature.parameters,
                "result": target.port.signature.result,
                "requirements": requirements,
            })
        })
        .collect::<Vec<_>>();
    success(
        "artifact.inspect",
        json!({
            "artifact_digest": program.artifact().artifact_digest,
            "root_package_id": program.artifact().root_package_id,
            "root_revision_digest": program.artifact().root_revision_digest,
            "packages": packages,
            "targets": targets,
        }),
    )
}

fn target_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let Some(subcommand) = arguments.first().map(String::as_str) else {
        return Err(usage_error("target requires list or run"));
    };
    if subcommand == "list" {
        exact_arguments(arguments, 1, "target list")?;
        let workspace = open_workspace(project)?;
        return success(
            "target.list",
            json!({"targets": workspace.orient()?.targets}),
        );
    }
    if subcommand != "run" || arguments.len() < 2 {
        return Err(usage_error(
            "target run requires <target> and optional --arguments <JSON-array>",
        ));
    }
    let target_name = &arguments[1];
    let encoded_arguments =
        option_value(&arguments[2..], "--arguments")?.unwrap_or_else(|| "[]".to_owned());
    reject_unknown_options(&arguments[2..], &["--arguments"])?;
    let workspace = open_workspace(project)?;
    let (artifact_bytes, _) = workspace.build_artifact()?;
    let program = PreparedProgram::prepare(load_artifact(&artifact_bytes)?)?;
    let target = program.target(target_name)?;
    if !matches!(
        target.runner,
        RunnerKind::Command | RunnerKind::Batch | RunnerKind::Test
    ) {
        return Err(usage_error(format!(
            "target '{}' uses {:?} runner; use its topology-specific command",
            target.name, target.runner
        )));
    }
    let component = program
        .components()
        .get(&target.component)
        .ok_or_else(|| internal_error("target component disappeared"))?;
    if !component.requirements.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticClass::Capability,
            "target_grants_required",
            "effectful target requires an exact deployment descriptor",
        ));
    }
    let json_arguments = decode_strict(encoded_arguments.as_bytes(), JsonLimits::default())?;
    let items = json_arguments.as_array().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "target_arguments_array",
            "target arguments must be one JSON array",
        )
    })?;
    if items.len() != target.port.signature.parameters.len() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "target_argument_count",
            format!(
                "target expects {} arguments; {} were supplied",
                target.port.signature.parameters.len(),
                items.len()
            ),
        ));
    }
    let values = items
        .iter()
        .zip(&target.port.signature.parameters)
        .map(|(value, ty)| {
            let bytes = serde_json::to_vec(value).map_err(internal_json)?;
            decode_typed(
                &bytes,
                ty,
                &program.artifact().packages,
                JsonLimits::default(),
            )
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let (value, production) = Vm::new(&program, RunPolicy::default())
        .invoke(&target.port.function, values.clone())
        .map_err(execution_diagnostic)?;
    let (reference, oracle) = ReferenceInterpreter::new(&program, RunPolicy::default())
        .invoke(&target.port.function, values)
        .map_err(execution_diagnostic)?;
    if value.canonical_json() != reference.canonical_json() {
        return Err(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "target_differential",
            "production and reference execution disagree",
        ));
    }
    let result_bytes = encode_typed(
        &value,
        &target.port.signature.result,
        &program.artifact().packages,
        JsonLimits::default(),
    )?;
    let result = decode_strict(&result_bytes, JsonLimits::default())?;
    success(
        "target.run",
        json!({
            "target": target.name,
            "result": result,
            "production": production,
            "oracle": oracle,
            "differential": "equal",
        }),
    )
}

fn extract_global_project(
    arguments: Vec<String>,
) -> Result<(Vec<String>, Option<PathBuf>), Diagnostic> {
    let mut output = Vec::new();
    let mut project = None;
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error("--project requires a path"))?;
            if project.replace(PathBuf::from(value)).is_some() {
                return Err(usage_error("--project may be supplied only once"));
            }
            index += 2;
        } else {
            output.push(arguments[index].clone());
            index += 1;
        }
    }
    Ok((output, project))
}

fn exact_base(arguments: &[String]) -> Result<(u64, String), Diagnostic> {
    if arguments.len() != 4 || arguments[0] != "--revision" || arguments[2] != "--record" {
        return Err(usage_error(
            "validate/apply require --revision <u64> --record <digest>",
        ));
    }
    Ok((parse_u64(&arguments[1], "revision")?, arguments[3].clone()))
}

fn option_value(arguments: &[String], name: &str) -> Result<Option<String>, Diagnostic> {
    let mut found = None;
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == name {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error(format!("{name} requires a value")))?;
            if found.replace(value.clone()).is_some() {
                return Err(usage_error(format!("{name} may be supplied only once")));
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(found)
}

fn optional_usize(arguments: &[String], name: &str, default: usize) -> Result<usize, Diagnostic> {
    option_value(arguments, name)?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| usage_error(format!("{name} requires a non-negative integer")))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn optional_u64_value(arguments: &[String], name: &str) -> Result<Option<u64>, Diagnostic> {
    option_value(arguments, name)?
        .map(|value| parse_u64(&value, name))
        .transpose()
}

fn reject_unknown_options(arguments: &[String], known: &[&str]) -> Result<(), Diagnostic> {
    let mut index = 0;
    while index < arguments.len() {
        if !known.contains(&arguments[index].as_str()) {
            return Err(usage_error(format!(
                "unknown or positional argument '{}'",
                arguments[index]
            )));
        }
        if index + 1 >= arguments.len() {
            return Err(usage_error(format!(
                "{} requires a value",
                arguments[index]
            )));
        }
        index += 2;
    }
    Ok(())
}

fn exact_arguments(arguments: &[String], expected: usize, command: &str) -> Result<(), Diagnostic> {
    if arguments.len() != expected {
        return Err(usage_error(format!(
            "{command} received unexpected arguments"
        )));
    }
    Ok(())
}

fn open_workspace(project: Option<&Path>) -> Result<SourceWorkspace, Diagnostic> {
    SourceWorkspace::open(&project_or_current(project)?)
}

fn project_or_current(project: Option<&Path>) -> Result<PathBuf, Diagnostic> {
    match project {
        Some(project) => Ok(project.to_path_buf()),
        None => std::env::current_dir().map_err(|error| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "cli_current_directory",
                format!("cannot obtain current directory: {error}"),
            )
        }),
    }
}

fn write_immutable_output(path: &Path, bytes: &[u8]) -> Result<&'static str, Diagnostic> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            let existing = read_bounded(path, bytes.len(), "existing build output")?;
            if existing == bytes {
                return Ok("existing_equal");
            }
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "build_output_conflict",
                format!(
                    "build output '{}' already contains foreign bytes",
                    path.display()
                ),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("build_output_metadata", path, error)),
    }
    let parent = path.parent().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "build_output_parent",
            "build output has no parent directory",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| io_error("build_output_parent", parent, error))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error("build_output_create", path, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("build_output_write", path, error))?;
    Ok("published")
}

fn read_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("cli_read_metadata", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "cli_read_type",
            format!(
                "{label} '{}' is not a regular non-symlink file",
                path.display()
            ),
        ));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "cli_read_length",
            format!("{label} length cannot be represented"),
        )
    })?;
    if length > maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "cli_read_limit",
            format!("{label} exceeds {maximum} bytes"),
        ));
    }
    let file = File::open(path).map_err(|error| io_error("cli_read_open", path, error))?;
    let mut bytes = Vec::with_capacity(length);
    file.take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("cli_read", path, error))?;
    if bytes.len() != length || bytes.len() > maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "cli_read_changed",
            format!("{label} changed during the read"),
        ));
    }
    Ok(bytes)
}

fn parse_u64(value: &str, label: &str) -> Result<u64, Diagnostic> {
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return Err(usage_error(format!("{label} is not a canonical u64")));
    }
    value
        .parse::<u64>()
        .map_err(|_| usage_error(format!("{label} is not a canonical u64")))
}

fn serialized(command: &str, value: &impl Serialize) -> Result<CliSuccess, Diagnostic> {
    success(command, serde_json::to_value(value).map_err(internal_json)?)
}

fn success(command: &str, result: serde_json::Value) -> Result<CliSuccess, Diagnostic> {
    Ok(CliSuccess {
        contract_version: CLI_CONTRACT_VERSION,
        ok: true,
        command: command.to_owned(),
        result,
    })
}

fn execution_diagnostic(error: super::execution::ExecutionError) -> Diagnostic {
    let class = match error.class {
        super::execution::ExecutionFailureClass::Trap => DiagnosticClass::Semantic,
        super::execution::ExecutionFailureClass::Capability
        | super::execution::ExecutionFailureClass::PossibleVisibility => {
            DiagnosticClass::Capability
        }
        super::execution::ExecutionFailureClass::Resource => DiagnosticClass::Resource,
        super::execution::ExecutionFailureClass::Cancelled => DiagnosticClass::Cancelled,
        super::execution::ExecutionFailureClass::Infrastructure => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
}

fn usage_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "cli_usage", message)
}

fn internal_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, "cli_internal", message)
}

fn internal_json(error: serde_json::Error) -> Diagnostic {
    internal_error(format!("machine JSON projection failed: {error}"))
}

fn io_error(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn usage() -> &'static str {
    "lkjscript [--project PATH] <project|module|component|package|artifact|target> ...\n\
project init|orient|status|validate|apply|history|doctor|restore|backup|restore-backup\n\
module list|show; component inspect; package build|test; artifact inspect; target list|run\n\
lkjscript serve|worker --deployment DESCRIPTOR; deployment inspect DESCRIPTOR; hash stdin"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_options_and_noncanonical_numbers_reject() {
        assert!(execute(vec!["unknown".to_owned()]).is_err());
        assert!(parse_u64("01", "revision").is_err());
        assert!(reject_unknown_options(&["--wat".to_owned()], &["--limit"]).is_err());
    }
}
