//! Copied-binary stateful HTTP authoring acceptance.

use crate::authority::{self, AuthorityObservation};
use crate::error::DevError;
use crate::evidence::{self, FileProof};
use crate::http_probe;
use crate::postgres::{self, LocalPostgresTools};
use crate::process::{self, ProcessControl, ProcessObservation, ProcessSpec, ProcessStatus};
use crate::service::POSTGRES_IMAGE;
use crate::stateful_http_program::{ProjectReferences, StandardReferences, build_program_request};
use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan, parse_records};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::net::{SocketAddr, TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAXIMUM_OUTPUT_BYTES: u64 = 32 * 1024 * 1024;
const MAXIMUM_CANDIDATE_BINARY_BYTES: u64 = 256 * 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const RUNNER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_TIMEOUT: Duration = Duration::from_secs(35);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Debug)]
struct Options {
    binary: PathBuf,
    postgres_root: Option<PathBuf>,
    machine: bool,
}

#[derive(Debug)]
struct Context {
    binary: PathBuf,
    evidence: PathBuf,
    ordinal: u64,
    commands: Vec<CommandEvidence>,
}

#[derive(Debug)]
struct Output {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum StatefulStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    class: String,
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct PlanObservation {
    bytes: u64,
    records: u64,
    allocations: u64,
    owners: u64,
    types: u64,
    dependencies: u64,
    retirements: u64,
    relations_removed: u64,
    relations_added: u64,
    structural_owners: u64,
    semantic_owners: u64,
    tests: u64,
    reasons: u64,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpObservation {
    name: String,
    method: String,
    path: String,
    status: u16,
    request_bytes: u64,
    response_bytes: u64,
    response_sha256: String,
    elapsed_nanoseconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct LiveObservation {
    postgres_image: String,
    postgres_port: u16,
    routes_checked: u64,
    created_identity: String,
    persistence_after_restart: bool,
    startup_failures_without_ready: u64,
    invalid_secret_no_ready: bool,
    migration_divergence_safe_failure: bool,
    statement_failure_rolled_back: bool,
    runner_restarts: u64,
    shutdown_cleanup_failures: u64,
    container_cleanup_complete: bool,
    authority_before: AuthorityObservation,
    authority_after: AuthorityObservation,
    authority_unchanged: bool,
    requests: Vec<HttpObservation>,
}

#[derive(Clone, Debug)]
enum PostgresVerifier {
    Docker {
        container: String,
    },
    Local {
        tools: LocalPostgresTools,
        port: u16,
    },
}

impl PostgresVerifier {
    fn execute(
        &self,
        context: &mut Context,
        cwd: &Path,
        database: &str,
        name: &str,
        statement: &str,
    ) -> Result<(), DevError> {
        let arguments = vec![
            "-U".to_owned(),
            "postgres".to_owned(),
            "-d".to_owned(),
            database.to_owned(),
            "-v".to_owned(),
            "ON_ERROR_STOP=1".to_owned(),
            "-Atc".to_owned(),
            statement.to_owned(),
        ];
        let (command, environment) = match self {
            Self::Docker { container } => {
                let mut command = vec![
                    "docker".to_owned(),
                    "exec".to_owned(),
                    container.clone(),
                    "psql".to_owned(),
                ];
                command.extend(arguments);
                (command, process::environment())
            }
            Self::Local { tools, port } => (
                tools.client_command("psql", *port, &arguments),
                tools.environment(),
            ),
        };
        let recorded_command = redact_sql_command(&command);
        context.success_external_recorded(name, &command, recorded_command, cwd, environment)?;
        Ok(())
    }
}

fn redact_sql_command(command: &[String]) -> Vec<String> {
    let mut recorded = command.to_vec();
    if let Some(statement) = recorded.last_mut() {
        *statement = "<redacted-sql>".to_owned();
    }
    recorded
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthoringResult {
    workflow: &'static str,
    project: String,
    initial_revision: String,
    accepted_revision: String,
    request_records: usize,
    request_bytes: usize,
    migration_checksum: String,
    plan_token: String,
    plan: PlanObservation,
    idempotent_reconciliation: bool,
    discovery_commands: u64,
    registry_digest: String,
    builtin_package_revision: String,
    artifact: String,
    artifact_bytes: u64,
    incremental_sha256: String,
    clean_sha256: String,
    deterministic: bool,
    live: LiveObservation,
    evidence: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct StatefulReceipt {
    contract_version: u32,
    status: StatefulStatus,
    workflow: &'static str,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    source_binary: FileProof,
    copied_binary: FileProof,
    isolated_root_outside_checkout: bool,
    temporary_root_cleanup_complete: bool,
    secret_environment_names: Vec<String>,
    raw_secret_values_retained: bool,
    commands: Vec<CommandEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<AuthoringResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_options(arguments)?;
    let repository = repository_root()?;
    let binary = resolve_binary(&repository, &options.binary)?;
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let evidence_parent = repository.join(".artifacts/lkjscript-dev/stateful-http");
    fs::create_dir_all(&evidence_parent)?;
    let evidence_path =
        evidence_parent.join(format!("{}-{}", unix_nanoseconds()?, std::process::id()));
    fs::create_dir(&evidence_path)?;
    let temporary = tempfile::Builder::new()
        .prefix("lkjscript-stateful-http-")
        .tempdir()
        .map_err(|error| DevError::infrastructure(format!("create isolated root: {error}")))?;
    let isolated = temporary.path().canonicalize()?;
    if isolated.starts_with(&repository) {
        return Err(DevError::infrastructure(
            "stateful HTTP isolated root is inside the checkout",
        ));
    }
    let copied = isolated.join("lkjscript");
    copy_binary(&binary, &copied)?;
    let source_binary = evidence::proof(&binary, binary.display().to_string())?;
    let copied_binary = evidence::proof(&copied, "isolated/lkjscript".to_owned())?;
    let mut context = Context {
        binary: copied,
        evidence: evidence_path.clone(),
        ordinal: 0,
        commands: Vec::new(),
    };
    let workflow = run_authoring(&mut context, &isolated, options.postgres_root.as_deref());
    let cleanup = temporary.close().map_err(|error| {
        DevError::infrastructure(format!("remove isolated stateful HTTP root: {error}"))
    });
    let cleanup_complete = cleanup.is_ok();
    let (status, result, failure) = match (workflow, cleanup) {
        (Ok(result), Ok(())) => (StatefulStatus::Passed, Some(result), None),
        (Err(error), _) | (Ok(_), Err(error)) => {
            let status = if error.kind() == "unavailable" {
                StatefulStatus::Unavailable
            } else {
                StatefulStatus::Failed
            };
            let failure = Failure {
                class: error.kind().to_owned(),
                code: format!("stateful_http_{}", error.kind()),
                message: error.message().to_owned(),
            };
            (status, None, Some(failure))
        }
    };
    let elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let receipt_value = StatefulReceipt {
        contract_version: 1,
        status,
        workflow: "stateful-http-public-authoring",
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds,
        source_binary,
        copied_binary,
        isolated_root_outside_checkout: true,
        temporary_root_cleanup_complete: cleanup_complete,
        secret_environment_names: vec![
            "BBS_DATABASE_URL".to_owned(),
            "POSTGRES_PASSWORD".to_owned(),
        ],
        raw_secret_values_retained: false,
        commands: context.commands,
        result,
        failure,
    };
    let receipt = evidence_path.join("authoring.json");
    let published = evidence::publish_json(&receipt, &receipt_value)?;
    if options.machine {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": matches!(status, StatefulStatus::Passed),
                "status": status,
                "workflow": "stateful-http",
                "receipt": receipt,
                "receipt_digest": published.digest,
            }))?
        );
    } else {
        println!("stateful HTTP public authoring {status:?}");
        println!("receipt: {}", receipt.display());
        println!("receipt digest: {}", published.digest);
    }
    Ok(match status {
        StatefulStatus::Passed => 0,
        StatefulStatus::Failed => 1,
        StatefulStatus::Unavailable => 2,
    })
}

fn run_authoring(
    context: &mut Context,
    isolated: &Path,
    postgres_root: Option<&Path>,
) -> Result<AuthoringResult, DevError> {
    let capabilities = records(
        &context
            .success("capabilities", &["capabilities"], isolated)?
            .bytes,
    )?;
    require_record_field(
        &capabilities,
        "registry",
        "graph",
        "lkjscript-meaning-graph-5",
    )?;
    let registry_digest =
        required_field(required_record(&capabilities, "registry")?, "digest")?.to_owned();
    for section in ["change", "deployment", "expression", "type"] {
        let present = capabilities
            .iter()
            .any(|record| record.operation == "section" && field(record, "name") == Some(section));
        if !present {
            return Err(DevError::corrupt(format!(
                "capabilities omitted required '{section}' section"
            )));
        }
    }
    verify_discovery(context, isolated)?;
    let builtin = records(
        &context
            .success(
                "builtin-summary",
                &["package", "builtin", "inspect"],
                isolated,
            )?
            .bytes,
    )?;
    let builtin_package_revision =
        required_field(required_record(&builtin, "package")?, "package-revision")?.to_owned();
    let standard = discover_standard(context, isolated)?;
    let discovery_commands = context.ordinal;
    let project = isolated.join("application");
    let created = records(
        &context
            .success(
                "new-http",
                &[
                    "new",
                    path_text(&project)?,
                    "--template",
                    "http",
                    "--name",
                    "bbs",
                ],
                isolated,
            )?
            .bytes,
    )?;
    let initial_revision = required_field(required_record(&created, "revision")?, "id")?.to_owned();
    let package = required_field(required_record(&created, "package")?, "id")?.to_owned();
    let project_references = discover_project(
        context,
        isolated,
        &project,
        package,
        initial_revision.clone(),
    )?;
    let request = build_program_request(&standard, &project_references)?;
    let request_path = isolated.join("bbs-change.lkjc");
    evidence::publish(&request_path, &request.bytes)?;
    evidence::publish(&context.evidence.join("bbs-change.lkjc"), &request.bytes)?;
    let plan_path = isolated.join("bbs-change.logical-plan");
    let planned = records(
        &context
            .success(
                "change-plan",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "plan",
                    "--input-file",
                    path_text(&request_path)?,
                    "--output",
                    path_text(&plan_path)?,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let plan_token = required_field(required_record(&planned, "plan")?, "token")?.to_owned();
    let plan_file = File::open(&plan_path)?;
    let decoded_plan =
        decode_logical_change_plan(BufReader::new(plan_file)).map_err(|diagnostic| {
            DevError::corrupt(format!(
                "strict logical-plan decode failed: {}",
                diagnostic.code
            ))
        })?;
    if decoded_plan.token != plan_token {
        return Err(DevError::corrupt(
            "strictly decoded logical-plan token disagrees with plan output",
        ));
    }
    let plan_output = required_record(&planned, "plan-output")?;
    let advertised_plan_bytes = required_field(plan_output, "bytes")?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("plan output byte count is not u64"))?;
    let advertised_plan_records = required_field(plan_output, "records")?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("plan output record count is not u64"))?;
    if decoded_plan.bytes != advertised_plan_bytes
        || decoded_plan.records != advertised_plan_records
    {
        return Err(DevError::corrupt(
            "strict logical-plan decode disagrees with advertised meters",
        ));
    }
    let plan_bytes = fs::read(&plan_path)?;
    evidence::publish(
        &context.evidence.join("bbs-change.logical-plan"),
        &plan_bytes,
    )?;
    let plan = PlanObservation {
        bytes: decoded_plan.bytes,
        records: decoded_plan.records,
        allocations: decoded_plan.counts.allocations,
        owners: decoded_plan.counts.owners,
        types: decoded_plan.counts.types,
        dependencies: decoded_plan.counts.dependencies,
        retirements: decoded_plan.counts.retirements,
        relations_removed: decoded_plan.counts.relations_removed,
        relations_added: decoded_plan.counts.relations_added,
        structural_owners: decoded_plan.counts.structural_owners,
        semantic_owners: decoded_plan.counts.semantic_owners,
        tests: decoded_plan.counts.tests,
        reasons: decoded_plan.counts.reasons,
    };
    let replay_plan_path = isolated.join("bbs-change-replay.logical-plan");
    let replay_planned = records(
        &context
            .success(
                "change-plan-repeat",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "plan",
                    "--input-file",
                    path_text(&request_path)?,
                    "--output",
                    path_text(&replay_plan_path)?,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let replay_token =
        required_field(required_record(&replay_planned, "plan")?, "token")?.to_owned();
    let replay_plan_bytes = fs::read(&replay_plan_path)?;
    evidence::publish(
        &context.evidence.join("bbs-change-replay.logical-plan"),
        &replay_plan_bytes,
    )?;
    if replay_token != plan_token || replay_plan_bytes != plan_bytes {
        let first_difference = plan_bytes
            .split(|byte| *byte == b'\n')
            .zip(replay_plan_bytes.split(|byte| *byte == b'\n'))
            .position(|(left, right)| left != right)
            .unwrap_or(usize::MAX);
        return Err(DevError::corrupt(format!(
            "repeated pre-publication plan changed at record {first_difference}"
        )));
    }
    let applied = records(
        &context
            .success(
                "change-apply",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "apply",
                    "--input-file",
                    path_text(&request_path)?,
                    "--plan",
                    &plan_token,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let accepted_revision =
        required_field(required_record(&applied, "revision")?, "result")?.to_owned();
    let reconciled = records(
        &context
            .success(
                "change-apply-reconcile",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "apply",
                    "--input-file",
                    path_text(&request_path)?,
                    "--plan",
                    &plan_token,
                ],
                isolated,
            )?
            .bytes,
    )?;
    require_record_field(&reconciled, "result", "status", "already-accepted")?;
    require_record_field(&reconciled, "revision", "result", &accepted_revision)?;
    context.success(
        "check",
        &["--project", path_text(&project)?, "check"],
        isolated,
    )?;
    let artifact = project.join("generated/bbs.lkja");
    context.success(
        "build-incremental",
        &[
            "--project",
            path_text(&project)?,
            "build",
            "--output",
            path_text(&artifact)?,
        ],
        isolated,
    )?;
    let first = fs::read(&artifact)?;
    let derived = project.join("derived");
    if derived.exists() {
        fs::remove_dir_all(&derived)?;
    }
    let clean_artifact = project.join("generated/bbs-clean.lkja");
    context.success(
        "build-clean",
        &[
            "--project",
            path_text(&project)?,
            "build",
            "--output",
            path_text(&clean_artifact)?,
        ],
        isolated,
    )?;
    let clean = fs::read(&clean_artifact)?;
    if first != clean {
        return Err(DevError::corrupt(
            "clean and incremental stateful HTTP artifacts differ",
        ));
    }
    let live = run_live(
        context,
        isolated,
        &project,
        &artifact,
        postgres_root,
        &request.migration_checksum,
    )?;
    Ok(AuthoringResult {
        workflow: "stateful-http-public-authoring",
        project: project.display().to_string(),
        initial_revision,
        accepted_revision,
        request_records: request.records,
        request_bytes: request.bytes.len(),
        migration_checksum: request.migration_checksum,
        plan_token,
        plan,
        idempotent_reconciliation: true,
        discovery_commands,
        registry_digest,
        builtin_package_revision,
        artifact: artifact.display().to_string(),
        artifact_bytes: first.len() as u64,
        incremental_sha256: sha256(&first),
        clean_sha256: sha256(&clean),
        deterministic: true,
        live,
        evidence: context.evidence.display().to_string(),
    })
}

fn run_live(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    artifact: &Path,
    postgres_root: Option<&Path>,
    migration_checksum: &str,
) -> Result<LiveObservation, DevError> {
    if let Some(postgres_root) = postgres_root {
        return run_live_local(
            context,
            isolated,
            project,
            artifact,
            postgres_root,
            migration_checksum,
        );
    }
    let authority_before = authority::observe_graph_authority(project)?;
    context.required_external(
        "docker-image-inspect",
        &["docker", "image", "inspect", POSTGRES_IMAGE],
        isolated,
        process::environment(),
    )?;
    let password = random_hex(24)?;
    let container = format!(
        "lkjscript-stateful-http-{}-{}",
        std::process::id(),
        unix_nanoseconds()?
    );
    let mut docker_environment = process::environment();
    docker_environment.insert("POSTGRES_PASSWORD".to_owned(), password.clone());
    let started = context.required_external(
        "postgres-start",
        &[
            "docker",
            "run",
            "--rm",
            "--name",
            &container,
            "-e",
            "POSTGRES_PASSWORD",
            "-e",
            "POSTGRES_DB=bbs",
            "-p",
            "127.0.0.1::5432",
            "-d",
            POSTGRES_IMAGE,
        ],
        isolated,
        docker_environment,
    );
    started?;
    let workflow = (|| {
        let port_output = context.success_external(
            "postgres-port",
            &["docker", "port", &container, "5432/tcp"],
            isolated,
            process::environment(),
        )?;
        let postgres_port = parse_host_port(&port_output.bytes)?;
        wait_for_postgres(context, isolated, &container, postgres_port)?;
        let database_url =
            format!("postgresql://postgres:{password}@127.0.0.1:{postgres_port}/bbs");
        exercise_live_service(
            context,
            isolated,
            project,
            artifact,
            authority_before.clone(),
            database_url,
            postgres_port,
            POSTGRES_IMAGE,
            &PostgresVerifier::Docker {
                container: container.clone(),
            },
            "bbs",
            migration_checksum,
        )
    })();
    let cleanup = context.success_external(
        "postgres-stop",
        &["docker", "stop", "--time", "5", &container],
        isolated,
        process::environment(),
    );
    match (workflow, cleanup) {
        (Ok(result), Ok(_)) => Ok(result),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn run_live_local(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    artifact: &Path,
    requested_root: &Path,
    migration_checksum: &str,
) -> Result<LiveObservation, DevError> {
    let tools = LocalPostgresTools::resolve(requested_root)?;
    let environment = tools.environment();
    let version = context.success_external(
        "postgres-local-version",
        &tools.version_command(),
        isolated,
        environment.clone(),
    )?;
    let version_text = tools.validate_version(&version.bytes)?;
    let authority_before = authority::observe_graph_authority(project)?;
    let data = isolated.join("postgres-data");
    let socket = isolated.join("postgres-socket");
    fs::create_dir(&socket)?;
    context.success_external(
        "postgres-local-init",
        &tools.initdb_command(&data),
        isolated,
        environment.clone(),
    )?;
    let postgres_port = free_port()?;
    let log = isolated.join("postgres.log");
    context.success_external(
        "postgres-local-start",
        &tools.start_command(&data, &log, &socket, postgres_port, 16),
        isolated,
        environment.clone(),
    )?;
    let workflow = exercise_live_service(
        context,
        isolated,
        project,
        artifact,
        authority_before,
        format!("postgresql://postgres@127.0.0.1:{postgres_port}/postgres"),
        postgres_port,
        &version_text,
        &PostgresVerifier::Local {
            tools: tools.clone(),
            port: postgres_port,
        },
        "postgres",
        migration_checksum,
    );
    let cleanup = context.success_external(
        "postgres-local-stop",
        &tools.stop_command(&data),
        isolated,
        environment,
    );
    match (workflow, cleanup) {
        (Ok(result), Ok(_)) => Ok(result),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

#[allow(clippy::too_many_arguments)]
fn exercise_live_service(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    artifact: &Path,
    authority_before: AuthorityObservation,
    database_url: String,
    postgres_port: u16,
    postgres_identity: &str,
    verifier: &PostgresVerifier,
    verifier_database: &str,
    migration_checksum: &str,
) -> Result<LiveObservation, DevError> {
    let descriptor = project.join("bbs.deployment.json");
    write_descriptor(&descriptor, artifact)?;
    context.failure(
        "serve-missing-database-secret",
        &["serve", "--deployment", path_text(&descriptor)?],
        isolated,
        BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
    )?;
    context.failure(
        "serve-invalid-database-secret",
        &["serve", "--deployment", path_text(&descriptor)?],
        isolated,
        BTreeMap::from([
            ("LANG".to_owned(), "C".to_owned()),
            (
                "BBS_DATABASE_URL".to_owned(),
                "not-a-database-url".to_owned(),
            ),
        ]),
    )?;

    let mut runner_environment = BTreeMap::from([("LANG".to_owned(), "C".to_owned())]);
    runner_environment.insert("BBS_DATABASE_URL".to_owned(), database_url);
    let (mut runner, address) = ActiveRunner::start(
        context,
        "service-first",
        &descriptor,
        isolated,
        runner_environment.clone(),
    )?;
    let mut requests = Vec::new();
    let first_result = exercise_before_restart(address, &mut requests);
    let statement_failure = exercise_statement_failure(
        context,
        isolated,
        verifier,
        verifier_database,
        address,
        &mut requests,
    );
    let first_stop = runner.stop();
    let created_identity = first_result?;
    statement_failure?;
    first_stop?;

    verifier.execute(
        context,
        isolated,
        verifier_database,
        "postgres-diverge-migration-checksum",
        "UPDATE lkjscript_schema_migrations SET checksum = '0000000000000000000000000000000000000000000000000000000000000000' WHERE migration_id = 1",
    )?;
    let (mut divergent_runner, divergent_address) = ActiveRunner::start(
        context,
        "service-migration-divergence",
        &descriptor,
        isolated,
        runner_environment.clone(),
    )?;
    let divergent = request(
        &mut requests,
        divergent_address,
        "migration-checksum-divergence",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&divergent, 500, None)?;
    divergent_runner.stop()?;
    verifier.execute(
        context,
        isolated,
        verifier_database,
        "postgres-restore-migration-checksum",
        &format!(
            "UPDATE lkjscript_schema_migrations SET checksum = '{migration_checksum}' WHERE migration_id = 1"
        ),
    )?;

    let (mut runner, restarted_address) = ActiveRunner::start(
        context,
        "service-restart",
        &descriptor,
        isolated,
        runner_environment,
    )?;
    let restarted_result =
        exercise_after_restart(restarted_address, &created_identity, &mut requests);
    let restarted_stop = runner.stop();
    restarted_result?;
    restarted_stop?;

    let authority_after = authority::observe_graph_authority(project)?;
    let authority_unchanged = authority_before == authority_after;
    if !authority_unchanged {
        return Err(DevError::corrupt(
            "stateful HTTP runtime changed accepted graph authority",
        ));
    }
    Ok(LiveObservation {
        postgres_image: postgres_identity.to_owned(),
        postgres_port,
        routes_checked: requests.len() as u64,
        created_identity,
        persistence_after_restart: true,
        startup_failures_without_ready: 2,
        invalid_secret_no_ready: true,
        migration_divergence_safe_failure: true,
        statement_failure_rolled_back: true,
        runner_restarts: 2,
        shutdown_cleanup_failures: 0,
        container_cleanup_complete: true,
        authority_before,
        authority_after,
        authority_unchanged,
        requests,
    })
}

fn write_descriptor(path: &Path, artifact: &Path) -> Result<(), DevError> {
    let artifact = artifact
        .strip_prefix(path.parent().ok_or_else(|| {
            DevError::infrastructure("stateful descriptor has no parent directory")
        })?)
        .map_err(|_| DevError::infrastructure("stateful artifact escaped its project"))?;
    let descriptor = json!({
        "contract_version": 1,
        "artifact": path_text(artifact)?,
        "target": "serve",
        "listen": "127.0.0.1:0",
        "runtime": {
            "contract_version": 1,
            "maximum_concurrent_tasks": 8,
            "maximum_queued_tasks": 32,
            "request_deadline_milliseconds": 30000,
            "shutdown_grace_milliseconds": 30000,
            "cancellation_grace_milliseconds": 5000
        },
        "execution": {
            "instruction_fuel": 10000000,
            "maximum_call_depth": 4096,
            "maximum_value_stack": 1000000
        },
        "http": {
            "contract_version": 1,
            "maximum_request_body_bytes": 65536,
            "maximum_response_body_bytes": 1048576,
            "maximum_header_bytes": 32768,
            "maximum_headers": 128
        },
        "worker": null,
        "streams": {
            "maximum_chunk_bytes": 65536,
            "maximum_buffered_chunks": 8,
            "maximum_total_bytes": 1048576,
            "maximum_live_streams": 64
        },
        "configuration": {},
        "secrets": [
            {"name": "database-url", "variable": "BBS_DATABASE_URL"}
        ],
        "grants": [
            {
                "requirement": "streams",
                "sharing_domain": "bbs-streams",
                "authority_revision": "1111111111111111111111111111111111111111111111111111111111111111",
                "adapter": {"kind": "byte_stream"}
            },
            {
                "requirement": "database",
                "sharing_domain": "bbs-database",
                "authority_revision": "2222222222222222222222222222222222222222222222222222222222222222",
                "adapter": {
                    "kind": "postgres",
                    "connection_secret": "database-url",
                    "maximum_connections": 4,
                    "maximum_wait_milliseconds": 5000,
                    "statement_timeout_milliseconds": 15000
                }
            },
            {
                "requirement": "identifiers",
                "sharing_domain": "bbs-identifiers",
                "authority_revision": "3333333333333333333333333333333333333333333333333333333333333333",
                "adapter": {"kind": "identifier"}
            },
            {
                "requirement": "clock",
                "sharing_domain": "bbs-clock",
                "authority_revision": "4444444444444444444444444444444444444444444444444444444444444444",
                "adapter": {"kind": "wall_clock"}
            }
        ]
    });
    let mut bytes = serde_json::to_vec_pretty(&descriptor)?;
    bytes.push(b'\n');
    evidence::publish(path, &bytes)?;
    Ok(())
}

fn verify_discovery(context: &mut Context, cwd: &Path) -> Result<(), DevError> {
    let change = records(
        &context
            .success(
                "discover-change-grammar",
                &["capabilities", "--section", "change"],
                cwd,
            )?
            .bytes,
    )?;
    for name in ["add.requirement", "set.function-contract"] {
        require_named_record(&change, "change.operation", "name", name)?;
    }
    for name in ["pure", "task"] {
        require_named_record(&change, "change.function-effect", "name", name)?;
    }

    let types = records(
        &context
            .success(
                "discover-type-grammar",
                &["capabilities", "--section", "type"],
                cwd,
            )?
            .bytes,
    )?;
    require_named_record(&types, "type.form", "name", "structural-record")?;

    let expressions = records(
        &context
            .success(
                "discover-expression-grammar",
                &["capabilities", "--section", "expression"],
                cwd,
            )?
            .bytes,
    )?;
    for name in [
        "let",
        "record",
        "field",
        "list",
        "variant",
        "match",
        "capability-call",
        "transaction",
    ] {
        require_named_record(&expressions, "expression.form", "name", name)?;
    }

    let deployment = records(
        &context
            .success(
                "discover-deployment-schema",
                &["capabilities", "--section", "deployment"],
                cwd,
            )?
            .bytes,
    )?;
    require_named_record(&deployment, "deployment.adapter", "kind", "postgres")?;
    for path in [
        "adapter.postgres.connection_secret",
        "adapter.postgres.maximum_connections",
        "adapter.postgres.maximum_wait_milliseconds",
        "adapter.postgres.statement_timeout_milliseconds",
    ] {
        require_named_record(&deployment, "deployment.adapter-field", "path", path)?;
    }

    let generated = cwd.join("discovered-contracts");
    let result = records(
        &context
            .success(
                "generate-stateful-walkthrough",
                &["capabilities", "--generate-docs", path_text(&generated)?],
                cwd,
            )?
            .bytes,
    )?;
    if result
        .iter()
        .filter(|record| record.operation == "file")
        .count()
        != 7
    {
        return Err(DevError::corrupt(
            "generated contract discovery did not publish seven owned documents",
        ));
    }
    let walkthrough = fs::read(generated.join("stateful-http-authoring.md"))?;
    for required in [
        b"walkthrough.request".as_slice(),
        b"walkthrough.body".as_slice(),
        b"walkthrough.json".as_slice(),
        b"walkthrough.database".as_slice(),
        b"walkthrough.response".as_slice(),
        b"walkthrough.grant".as_slice(),
    ] {
        if !walkthrough
            .windows(required.len())
            .any(|window| window == required)
        {
            return Err(DevError::corrupt(
                "generated stateful HTTP walkthrough is incomplete",
            ));
        }
    }
    Ok(())
}

fn require_named_record(
    records: &[CompactRecord],
    operation: &str,
    field_name: &str,
    expected: &str,
) -> Result<(), DevError> {
    if records
        .iter()
        .any(|record| record.operation == operation && field(record, field_name) == Some(expected))
    {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "discovery omitted {operation}.{field_name}='{expected}'"
        )))
    }
}

fn discover_standard(context: &mut Context, cwd: &Path) -> Result<StandardReferences, DevError> {
    let mut references = StandardReferences {
        declarations: BTreeMap::new(),
        interfaces: BTreeMap::new(),
        operations: BTreeMap::new(),
        cases: BTreeMap::new(),
    };
    for name in [
        "SqlType",
        "SqlValue",
        "add",
        "bool-and",
        "bool-not",
        "bytes-equal",
        "bytes-from-text",
        "i64-equal",
        "json-decode-or",
        "json-encode",
        "less",
        "less-equal",
        "list-get",
        "list-length",
        "query-get-or",
        "sql-row-get",
        "sql-rows-get",
        "sql-rows-length",
        "text-empty",
        "text-equal",
        "text-length",
    ] {
        let owner = builtin_owner(context, cwd, None, name)?;
        references.declarations.insert(name.to_owned(), owner);
    }
    for name in ["ByteStream", "Database", "Identifier", "WallClock"] {
        let owner = builtin_owner(context, cwd, Some("interface"), name)?;
        references.interfaces.insert(name.to_owned(), owner.clone());
        let identity = owner
            .split('/')
            .next_back()
            .ok_or_else(|| DevError::corrupt("standard interface reference is invalid"))?;
        let detail = records(
            &context
                .success(
                    &format!("builtin-inspect-{name}"),
                    &[
                        "package",
                        "builtin",
                        "inspect",
                        "owner",
                        "interface",
                        identity,
                    ],
                    cwd,
                )?
                .bytes,
        )?;
        for record in detail.iter().filter(|record| record.operation == "owner") {
            if field(record, "kind") == Some("operation") {
                let operation = required_field(record, "name")?;
                let reference = required_field(record, "reference")?;
                references
                    .operations
                    .insert(format!("{name}.{operation}"), reference.to_owned());
            }
        }
    }
    for name in ["SqlType", "SqlValue"] {
        let owner = references
            .declarations
            .get(name)
            .ok_or_else(|| DevError::corrupt("discovered standard declaration vanished"))?
            .split('/')
            .next_back()
            .ok_or_else(|| DevError::corrupt("standard declaration reference is invalid"))?
            .to_owned();
        let detail = records(
            &context
                .success(
                    &format!("builtin-inspect-{name}"),
                    &["package", "builtin", "inspect", "owner", "variant", &owner],
                    cwd,
                )?
                .bytes,
        )?;
        for record in detail.iter().filter(|record| record.operation == "owner") {
            if field(record, "kind") == Some("case") {
                references.cases.insert(
                    format!("{name}.{}", required_field(record, "name")?),
                    required_field(record, "reference")?.to_owned(),
                );
            }
        }
    }
    Ok(references)
}

fn builtin_owner(
    context: &mut Context,
    cwd: &Path,
    kind: Option<&str>,
    name: &str,
) -> Result<String, DevError> {
    let mut arguments = vec!["package", "builtin", "query", "owners"];
    if let Some(kind) = kind {
        arguments.extend(["--kind", kind]);
    }
    arguments.extend(["--name", name]);
    let output = records(
        &context
            .success(&format!("builtin-query-{name}"), &arguments, cwd)?
            .bytes,
    )?;
    let owners = output
        .iter()
        .filter(|record| record.operation == "owner")
        .collect::<Vec<_>>();
    if owners.len() != 1 {
        return Err(DevError::corrupt(format!(
            "built-in discovery found {} owners named '{name}'",
            owners.len()
        )));
    }
    Ok(required_field(owners[0], "reference")?.to_owned())
}

fn discover_project(
    context: &mut Context,
    cwd: &Path,
    project: &Path,
    package: String,
    base_revision: String,
) -> Result<ProjectReferences, DevError> {
    let owners = records(
        &context
            .success(
                "project-owners",
                &[
                    "--project",
                    path_text(project)?,
                    "query",
                    "owners",
                    "--limit",
                    "100",
                ],
                cwd,
            )?
            .bytes,
    )?;
    let find = |kind: &str, name: &str| -> Result<String, DevError> {
        let matches = owners
            .iter()
            .filter(|record| {
                record.operation == "owner"
                    && field(record, "kind") == Some(kind)
                    && field(record, "name") == Some(name)
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(DevError::corrupt(format!(
                "starter project has {} {kind} owners named '{name}'",
                matches.len()
            )));
        }
        Ok(required_field(matches[0], "id")?.to_owned())
    };
    Ok(ProjectReferences {
        base_revision,
        package,
        component: find("component", "application")?,
        handler: find("task_function", "handle")?,
        request_parameter: find("parameter", "request")?,
        streams_requirement: find("requirement", "streams")?,
    })
}

struct ActiveRunner {
    name: String,
    control: ProcessControl,
    receiver: Receiver<ProcessObservation>,
    thread: Option<thread::JoinHandle<()>>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl ActiveRunner {
    fn start(
        context: &mut Context,
        name: &str,
        descriptor: &Path,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(Self, SocketAddr), DevError> {
        let stdout_path = context.evidence.join(format!("runner-{name}.stdout.log"));
        let stderr_path = context.evidence.join(format!("runner-{name}.stderr.log"));
        let command = vec![
            context.binary.display().to_string(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor.display().to_string(),
        ];
        let specification = ProcessSpec {
            command,
            cwd: cwd.to_path_buf(),
            environment,
            timeout: RUNNER_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: None,
        };
        let root = repository_root()?;
        let control = ProcessControl::default();
        let child_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("stateful-http-{name}"))
            .spawn(move || {
                let observation = process::run_controlled(&specification, &root, &child_control);
                let _ = sender.send(observation);
            })
            .map_err(|error| DevError::infrastructure(format!("start runner thread: {error}")))?;
        let mut runner = Self {
            name: name.to_owned(),
            control,
            receiver,
            thread: Some(thread),
            stdout_path,
            stderr_path,
        };
        match runner.wait_ready() {
            Ok(address) => Ok((runner, address)),
            Err(error) => {
                let _ = runner.kill();
                Err(error)
            }
        }
    }

    fn wait_ready(&mut self) -> Result<SocketAddr, DevError> {
        let started = Instant::now();
        loop {
            if let Some(line) = first_line(&self.stdout_path)? {
                let value: Value = serde_json::from_slice(&line)?;
                if value.get("event").and_then(Value::as_str) != Some("ready")
                    || value.get("ok").and_then(Value::as_bool) != Some(true)
                {
                    return Err(DevError::corrupt(format!(
                        "stateful runner '{}' emitted a non-ready first event",
                        self.name
                    )));
                }
                return value
                    .get("local_address")
                    .and_then(Value::as_str)
                    .ok_or_else(|| DevError::corrupt("ready event omitted local_address"))?
                    .parse()
                    .map_err(|error| {
                        DevError::corrupt(format!("ready address is invalid: {error}"))
                    });
            }
            match self.receiver.try_recv() {
                Ok(observation) => {
                    return Err(DevError::corrupt(format!(
                        "stateful runner '{}' exited {:?} before readiness; inspect {}",
                        self.name,
                        observation.status,
                        self.stdout_path.display()
                    )));
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Err(DevError::infrastructure(
                        "stateful runner observation channel disconnected",
                    ));
                }
            }
            if started.elapsed() >= READY_TIMEOUT {
                return Err(DevError::corrupt(format!(
                    "stateful runner '{}' omitted readiness",
                    self.name
                )));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn stop(&mut self) -> Result<(), DevError> {
        self.control.interrupt();
        let observation = match self.receiver.recv_timeout(STOP_TIMEOUT) {
            Ok(observation) => observation,
            Err(_) => {
                self.control.kill();
                self.receiver
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|error| {
                        DevError::infrastructure(format!("stop stateful runner: {error}"))
                    })?
            }
        };
        self.join()?;
        if observation.status != ProcessStatus::Passed {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' stopped as {:?}",
                self.name, observation.status
            )));
        }
        let stdout = process::read_bounded(&self.stdout_path, MAXIMUM_OUTPUT_BYTES)?;
        if !stdout.split(|byte| *byte == b'\n').any(|line| {
            serde_json::from_slice::<Value>(line)
                .ok()
                .and_then(|value| {
                    value
                        .get("event")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .as_deref()
                == Some("stopped")
        }) {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' omitted its stopped receipt",
                self.name
            )));
        }
        let stderr = process::read_bounded(&self.stderr_path, MAXIMUM_OUTPUT_BYTES)?;
        if !stderr.is_empty() {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' wrote to stderr",
                self.name
            )));
        }
        Ok(())
    }

    fn kill(&mut self) -> Result<(), DevError> {
        self.control.kill();
        let _ = self.receiver.recv_timeout(Duration::from_secs(5));
        self.join()
    }

    fn join(&mut self) -> Result<(), DevError> {
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| DevError::infrastructure("stateful runner thread panicked"))?;
        }
        Ok(())
    }
}

impl Drop for ActiveRunner {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.control.kill();
            let _ = self.receiver.recv_timeout(Duration::from_secs(5));
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

fn exercise_before_restart(
    address: SocketAddr,
    observations: &mut Vec<HttpObservation>,
) -> Result<String, DevError> {
    let root = request(observations, address, "root", "GET", "/", b"", &[])?;
    require_http(&root, 200, Some("text/html; charset=utf-8"))?;
    if !root.body.starts_with(b"<!doctype html>") {
        return Err(DevError::corrupt(
            "BBS root did not return graph-owned HTML",
        ));
    }
    let list = request(
        observations,
        address,
        "list-empty",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&list, 200, Some("application/json"))?;
    require_json_array_len(&list.body, 0)?;

    for (name, body) in [
        ("malformed-json", b"{".as_slice()),
        (
            "trailing-json",
            b"{\"author\":\"agent\",\"body\":\"first\"} trailing".as_slice(),
        ),
        (
            "duplicate-json-field",
            b"{\"author\":\"agent\",\"author\":\"other\",\"body\":\"first\"}".as_slice(),
        ),
    ] {
        let response = request(
            observations,
            address,
            name,
            "POST",
            "/api/posts",
            body,
            &[("Content-Type", "application/json")],
        )?;
        require_http(&response, 400, Some("application/json"))?;
    }
    let missing_header = request(
        observations,
        address,
        "missing-content-type",
        "POST",
        "/api/posts",
        b"{\"author\":\"agent\",\"body\":\"first\"}",
        &[],
    )?;
    require_http(&missing_header, 400, Some("application/json"))?;

    let created = request(
        observations,
        address,
        "create",
        "POST",
        "/api/posts",
        b"{\"author\":\"agent\",\"body\":\"first\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&created, 201, Some("application/json"))?;
    let created_value: Value = serde_json::from_slice(&created.body)?;
    let identity = created_value
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| value.len() == 36)
        .ok_or_else(|| DevError::corrupt("created BBS post omitted its semantic identity"))?
        .to_owned();
    if created_value.get("author").and_then(Value::as_str) != Some("agent")
        || created_value.get("body").and_then(Value::as_str) != Some("first")
    {
        return Err(DevError::corrupt(
            "created BBS post disagrees with its request",
        ));
    }
    let list = request(
        observations,
        address,
        "list-created",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_json_array_len(&list.body, 1)?;

    for (name, path) in [
        ("update-missing-id", "/api/posts".to_owned()),
        ("update-malformed-id", "/api/posts?id=bad".to_owned()),
        (
            "update-duplicate-id",
            format!("/api/posts?id={identity}&id={identity}"),
        ),
    ] {
        let response = request(
            observations,
            address,
            name,
            "PUT",
            &path,
            b"{\"author\":\"agent\",\"body\":\"updated\"}",
            &[("Content-Type", "application/json")],
        )?;
        require_http(&response, 400, Some("application/json"))?;
    }
    let absent = request(
        observations,
        address,
        "update-absent",
        "PUT",
        "/api/posts?id=00000000-0000-4000-8000-000000000000",
        b"{\"author\":\"agent\",\"body\":\"updated\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&absent, 404, Some("application/json"))?;
    let updated = request(
        observations,
        address,
        "update",
        "PUT",
        &format!("/api/posts?id={identity}"),
        b"{\"author\":\"agent-two\",\"body\":\"updated\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&updated, 200, Some("application/json"))?;
    let updated_value: Value = serde_json::from_slice(&updated.body)?;
    if updated_value.get("body").and_then(Value::as_str) != Some("updated") {
        return Err(DevError::corrupt(
            "BBS update did not return the updated post",
        ));
    }
    Ok(identity)
}

fn exercise_statement_failure(
    context: &mut Context,
    isolated: &Path,
    verifier: &PostgresVerifier,
    database: &str,
    address: SocketAddr,
    observations: &mut Vec<HttpObservation>,
) -> Result<(), DevError> {
    verifier.execute(
        context,
        isolated,
        database,
        "postgres-add-failure-constraint",
        "ALTER TABLE bbs_posts ADD CONSTRAINT bbs_statement_failure CHECK (false) NOT VALID",
    )?;
    let failed = request(
        observations,
        address,
        "statement-failure",
        "POST",
        "/api/posts",
        br#"{"author":"agent","body":"statement-failure"}"#,
        &[("Content-Type", "application/json")],
    )?;
    let failure_result = require_http(&failed, 500, None).and_then(|()| {
        let list = request(
            observations,
            address,
            "list-after-statement-failure",
            "GET",
            "/api/posts",
            b"",
            &[],
        )?;
        require_json_array_len(&list.body, 1)
    });
    let cleanup = verifier.execute(
        context,
        isolated,
        database,
        "postgres-remove-failure-constraint",
        "ALTER TABLE bbs_posts DROP CONSTRAINT bbs_statement_failure",
    );
    match (failure_result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
    }
}

fn exercise_after_restart(
    address: SocketAddr,
    identity: &str,
    observations: &mut Vec<HttpObservation>,
) -> Result<(), DevError> {
    let list = request(
        observations,
        address,
        "list-after-restart",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&list, 200, Some("application/json"))?;
    let value: Value = serde_json::from_slice(&list.body)?;
    let post = value
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| DevError::corrupt("BBS post did not persist across restart"))?;
    if post.get("id").and_then(Value::as_str) != Some(identity)
        || post.get("body").and_then(Value::as_str) != Some("updated")
    {
        return Err(DevError::corrupt(
            "BBS persisted value disagrees after restart",
        ));
    }
    let unsupported = request(
        observations,
        address,
        "unsupported-method",
        "PATCH",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&unsupported, 405, Some("application/json"))?;
    let unknown = request(
        observations,
        address,
        "unknown-route",
        "GET",
        "/unknown",
        b"",
        &[],
    )?;
    require_http(&unknown, 404, Some("application/json"))?;
    let deleted = request(
        observations,
        address,
        "delete",
        "DELETE",
        &format!("/api/posts?id={identity}"),
        b"",
        &[],
    )?;
    require_http(&deleted, 204, None)?;
    if !deleted.body.is_empty() {
        return Err(DevError::corrupt("204 BBS response contained a body"));
    }
    let missing = request(
        observations,
        address,
        "delete-absent",
        "DELETE",
        &format!("/api/posts?id={identity}"),
        b"",
        &[],
    )?;
    require_http(&missing, 404, Some("application/json"))?;
    let final_list = request(
        observations,
        address,
        "list-after-delete",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_json_array_len(&final_list.body, 0)
}

fn request(
    observations: &mut Vec<HttpObservation>,
    address: SocketAddr,
    name: &str,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<http_probe::HttpResponse, DevError> {
    let response = http_probe::request(address, method, path, body, headers)?;
    observations.push(HttpObservation {
        name: name.to_owned(),
        method: method.to_owned(),
        path: path.split('?').next().unwrap_or(path).to_owned(),
        status: response.status,
        request_bytes: body.len() as u64,
        response_bytes: response.body.len() as u64,
        response_sha256: sha256(&response.body),
        elapsed_nanoseconds: response.elapsed_nanoseconds,
    });
    Ok(response)
}

fn require_http(
    response: &http_probe::HttpResponse,
    status: u16,
    content_type: Option<&str>,
) -> Result<(), DevError> {
    if response.status != status {
        return Err(DevError::corrupt(format!(
            "BBS response status was {}, expected {status}; failure code was {:?}",
            response.status,
            response.headers.get("x-lkjscript-failure-code")
        )));
    }
    if let Some(expected) = content_type
        && response.headers.get("content-type").map(String::as_str) != Some(expected)
    {
        return Err(DevError::corrupt(format!(
            "BBS response content type was {:?}, expected '{expected}'",
            response.headers.get("content-type")
        )));
    }
    Ok(())
}

fn require_json_array_len(bytes: &[u8], expected: usize) -> Result<(), DevError> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value.as_array().map(Vec::len) == Some(expected) {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "BBS JSON list does not contain {expected} items"
        )))
    }
}

fn first_line(path: &Path) -> Result<Option<Vec<u8>>, DevError> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|end| bytes[..end].to_vec())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn parse_host_port(bytes: &[u8]) -> Result<u16, DevError> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|value| value.trim().rsplit_once(':'))
        .and_then(|(_, port)| port.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .ok_or_else(|| DevError::corrupt("Docker did not report a valid PostgreSQL host port"))
}

fn free_port() -> Result<u16, DevError> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(DevError::from)
}

fn wait_for_postgres(
    context: &mut Context,
    cwd: &Path,
    container: &str,
    port: u16,
) -> Result<(), DevError> {
    let address: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|error| DevError::infrastructure(format!("PostgreSQL address: {error}")))?;
    let started = Instant::now();
    let mut attempt = 0_u64;
    while started.elapsed() < READY_TIMEOUT {
        let name = format!("postgres-ready-{attempt:03}");
        let result = context.observe_external(
            &name,
            &[
                "docker",
                "exec",
                container,
                "pg_isready",
                "-U",
                "postgres",
                "-d",
                "bbs",
            ],
            cwd,
            process::environment(),
        )?;
        if result.0.status == ProcessStatus::Passed
            && TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok()
        {
            return Ok(());
        }
        attempt = attempt.saturating_add(1);
        thread::sleep(Duration::from_millis(250));
    }
    Err(DevError::corrupt("PostgreSQL did not become ready"))
}

fn random_hex(bytes: usize) -> Result<String, DevError> {
    let mut value = vec![0_u8; bytes];
    File::open("/dev/urandom")?.read_exact(&mut value)?;
    Ok(value.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl Context {
    fn success<S: AsRef<str>>(
        &mut self,
        name: &str,
        arguments: &[S],
        cwd: &Path,
    ) -> Result<Output, DevError> {
        let mut command = vec![self.binary.display().to_string()];
        command.extend(arguments.iter().map(|value| value.as_ref().to_owned()));
        let (observation, output) = self.observe_command(
            name,
            command,
            cwd,
            BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
        )?;
        self.require_status(name, observation, ProcessStatus::Passed)?;
        Ok(output)
    }

    fn failure<S: AsRef<str>>(
        &mut self,
        name: &str,
        arguments: &[S],
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<Output, DevError> {
        let mut command = vec![self.binary.display().to_string()];
        command.extend(arguments.iter().map(|value| value.as_ref().to_owned()));
        let (observation, output) = self.observe_command(name, command, cwd, environment)?;
        if observation.status == ProcessStatus::Passed {
            return Err(DevError::corrupt(format!(
                "stateful HTTP negative step '{name}' unexpectedly succeeded"
            )));
        }
        if output
            .bytes
            .windows(b"\"event\":\"ready\"".len())
            .any(|value| value == b"\"event\":\"ready\"")
        {
            return Err(DevError::corrupt(format!(
                "stateful HTTP negative step '{name}' emitted readiness"
            )));
        }
        Ok(output)
    }

    fn success_external<S: AsRef<str>>(
        &mut self,
        name: &str,
        command: &[S],
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<Output, DevError> {
        let (observation, output) = self.observe_external(name, command, cwd, environment)?;
        self.require_status(name, observation, ProcessStatus::Passed)?;
        Ok(output)
    }

    fn success_external_recorded<S: AsRef<str>>(
        &mut self,
        name: &str,
        command: &[S],
        recorded_command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<Output, DevError> {
        let execution_command = command
            .iter()
            .map(|value| value.as_ref().to_owned())
            .collect();
        let (observation, output) = self.observe_command_with_record(
            name,
            execution_command,
            recorded_command,
            cwd,
            environment,
        )?;
        self.require_status(name, observation, ProcessStatus::Passed)?;
        Ok(output)
    }

    fn required_external<S: AsRef<str>>(
        &mut self,
        name: &str,
        command: &[S],
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<Output, DevError> {
        let (observation, output) = self.observe_external(name, command, cwd, environment)?;
        if observation.status != ProcessStatus::Passed {
            return Err(DevError::unavailable(format!(
                "required stateful HTTP prerequisite '{name}' is unavailable ({:?})",
                observation.status
            )));
        }
        Ok(output)
    }

    fn observe_external<S: AsRef<str>>(
        &mut self,
        name: &str,
        command: &[S],
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(ProcessObservation, Output), DevError> {
        self.observe_command(
            name,
            command
                .iter()
                .map(|value| value.as_ref().to_owned())
                .collect(),
            cwd,
            environment,
        )
    }

    fn observe_command(
        &mut self,
        name: &str,
        command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(ProcessObservation, Output), DevError> {
        let recorded_command = command.clone();
        self.observe_command_with_record(name, command, recorded_command, cwd, environment)
    }

    fn observe_command_with_record(
        &mut self,
        name: &str,
        command: Vec<String>,
        recorded_command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(ProcessObservation, Output), DevError> {
        let ordinal = self.ordinal;
        self.ordinal = self.ordinal.saturating_add(1);
        let safe = name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let stdout_path = self
            .evidence
            .join(format!("{ordinal:03}-{safe}.stdout.log"));
        let stderr_path = self
            .evidence
            .join(format!("{ordinal:03}-{safe}.stderr.log"));
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment,
            timeout: COMMAND_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path,
            unavailable_exit_code: None,
        };
        let observation = process::run(&specification, &repository_root()?);
        let output = Output {
            bytes: process::read_bounded(&stdout_path, MAXIMUM_OUTPUT_BYTES)?,
        };
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command: recorded_command,
            process: observation.clone(),
        });
        Ok((observation, output))
    }

    fn require_status(
        &self,
        name: &str,
        observation: ProcessObservation,
        expected: ProcessStatus,
    ) -> Result<(), DevError> {
        if observation.status == ProcessStatus::Unavailable {
            return Err(DevError::unavailable(format!(
                "stateful HTTP step '{name}' is unavailable; inspect {}",
                self.evidence.display()
            )));
        }
        if observation.status != expected {
            return Err(DevError::corrupt(format!(
                "stateful HTTP step '{name}' ended as {:?}; inspect {}",
                observation.status,
                self.evidence.display()
            )));
        }
        Ok(())
    }
}

fn records(bytes: &[u8]) -> Result<Vec<CompactRecord>, DevError> {
    parse_records("stateful-http", bytes).map_err(|diagnostics| {
        DevError::corrupt(format!(
            "stateful HTTP command output is not compact records: {}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ))
    })
}

fn required_record<'a>(
    records: &'a [CompactRecord],
    operation: &str,
) -> Result<&'a CompactRecord, DevError> {
    records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| DevError::corrupt(format!("compact output omitted '{operation}'")))
}

fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

fn required_field<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, DevError> {
    field(record, name).ok_or_else(|| {
        DevError::corrupt(format!(
            "compact '{}' record omitted '{name}'",
            record.operation
        ))
    })
}

fn require_record_field(
    records: &[CompactRecord],
    operation: &str,
    name: &str,
    expected: &str,
) -> Result<(), DevError> {
    let actual = required_field(required_record(records, operation)?, name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "compact {operation}.{name} was '{actual}', expected '{expected}'"
        )))
    }
}

fn path_text(path: &Path) -> Result<&str, DevError> {
    path.to_str()
        .ok_or_else(|| DevError::usage("stateful HTTP path is not UTF-8"))
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut binary = None;
    let mut postgres_root = None;
    let mut machine = false;
    let mut arguments = arguments;
    while let Some(argument) = crate::next_utf8(&mut arguments, "stateful HTTP option")? {
        match argument.as_str() {
            "--binary" => {
                let value = crate::next_utf8(&mut arguments, "stateful HTTP binary")?
                    .ok_or_else(|| DevError::usage("--binary requires a path"))?;
                if binary.replace(PathBuf::from(value)).is_some() {
                    return Err(DevError::usage("duplicate --binary option"));
                }
            }
            "--postgres-root" => {
                let value = crate::next_utf8(&mut arguments, "stateful HTTP PostgreSQL root")?
                    .ok_or_else(|| DevError::usage("--postgres-root requires a path"))?;
                if postgres_root.replace(PathBuf::from(value)).is_some() {
                    return Err(DevError::usage("duplicate --postgres-root option"));
                }
            }
            "--machine" if !machine => machine = true,
            "--machine" => return Err(DevError::usage("duplicate --machine option")),
            other => return Err(DevError::usage(format!("unknown option '{other}'"))),
        }
    }
    Ok(Options {
        binary: binary.unwrap_or_else(|| PathBuf::from("target/release/lkjscript")),
        postgres_root: postgres_root.or_else(postgres::configured_root),
        machine,
    })
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| DevError::infrastructure("lkjscript-dev escaped its workspace"))?
        .canonicalize()
        .map_err(DevError::from)
}

fn resolve_binary(repository: &Path, path: &Path) -> Result<PathBuf, DevError> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository.join(path)
    };
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(
            "stateful HTTP binary must be a regular file",
        ));
    }
    if metadata.len() > MAXIMUM_CANDIDATE_BINARY_BYTES {
        return Err(DevError::usage(format!(
            "stateful HTTP binary exceeds {MAXIMUM_CANDIDATE_BINARY_BYTES} bytes"
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage("stateful HTTP binary is not executable"));
    }
    path.canonicalize().map_err(DevError::from)
}

fn copy_binary(source: &Path, destination: &Path) -> Result<(), DevError> {
    let metadata = fs::metadata(source)?;
    fs::copy(source, destination)?;
    fs::set_permissions(destination, metadata.permissions())?;
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_closed() {
        let options =
            parse_options([OsString::from("--machine")].into_iter()).expect("stateful options");
        assert!(options.machine);
        assert_eq!(options.postgres_root, postgres::configured_root());
        assert!(parse_options([OsString::from("--unknown")].into_iter()).is_err());
    }

    #[test]
    fn postgres_oracle_statements_are_redacted_from_evidence() {
        let command = vec![
            "psql".to_owned(),
            "-Atc".to_owned(),
            "SELECT sensitive_control_value".to_owned(),
        ];
        assert_eq!(
            redact_sql_command(&command),
            ["psql", "-Atc", "<redacted-sql>"]
        );
        assert_eq!(command[2], "SELECT sensitive_control_value");
    }

    #[cfg(unix)]
    #[test]
    fn candidate_binary_is_regular_executable_and_bounded() {
        let temporary = tempfile::tempdir().expect("temporary stateful binary fixtures");
        let candidate = temporary.path().join("candidate");
        let file = File::create(&candidate).expect("create candidate");
        file.set_len(MAXIMUM_CANDIDATE_BINARY_BYTES.saturating_add(1))
            .expect("extend oversized candidate");
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))
            .expect("make candidate executable");
        assert!(resolve_binary(temporary.path(), &candidate).is_err());

        file.set_len(1).expect("shrink bounded candidate");
        assert!(resolve_binary(temporary.path(), &candidate).is_ok());
        let link = temporary.path().join("candidate-link");
        std::os::unix::fs::symlink(&candidate, &link).expect("create candidate link");
        assert!(resolve_binary(temporary.path(), &link).is_err());
    }
}
