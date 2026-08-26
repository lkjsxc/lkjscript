use crate::error::DevError;
use crate::evidence::{self, FileProof, PublishedEvidence, VerificationDigest};
use crate::process::{self, ProcessObservation, ProcessSpec, ProcessStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SCALE_CONTRACT_VERSION: u32 = 1;
const PUBLIC_CHANGE_CONTRACT_VERSION: u16 = 3;
const MAXIMUM_BATCH: usize = 10_000;
const MAXIMUM_ITEMS: usize = 1_000_000;
const MAXIMUM_STDOUT_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_STDERR_BYTES: u64 = 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3_600);
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Topology {
    IndependentModules,
    SmallFunctions,
    WideModule,
    DeepChain,
    WideFanout,
}

impl Topology {
    fn parse(value: &str) -> Result<Self, DevError> {
        match value {
            "independent-modules" => Ok(Self::IndependentModules),
            "small-functions" => Ok(Self::SmallFunctions),
            "wide-module" => Ok(Self::WideModule),
            "deep-chain" => Ok(Self::DeepChain),
            "wide-fanout" => Ok(Self::WideFanout),
            _ => Err(DevError::usage(format!("unknown scale topology '{value}'"))),
        }
    }

    fn cli_name(self) -> &'static str {
        match self {
            Self::IndependentModules => "independent-modules",
            Self::SmallFunctions => "small-functions",
            Self::WideModule => "wide-module",
            Self::DeepChain => "deep-chain",
            Self::WideFanout => "wide-fanout",
        }
    }

    fn semantic_shape(self) -> &'static str {
        match self {
            Self::IndependentModules => "many_independent_modules",
            Self::SmallFunctions => "many_small_pure_functions_distributed_across_modules",
            Self::WideModule => "one_module_with_many_small_pure_functions",
            Self::DeepChain => "one_module_with_a_deep_direct_call_chain",
            Self::WideFanout => "one_module_with_many_direct_callers_of_one_root",
        }
    }
}

#[derive(Clone, Debug)]
struct Options {
    topology: Topology,
    items: usize,
    batch: usize,
    modules: Option<usize>,
    binary: PathBuf,
    retain: Option<PathBuf>,
    machine: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ScaleStatus {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ScaleReceipt {
    contract_version: u32,
    status: ScaleStatus,
    topology: Topology,
    semantic_shape: String,
    parameters: ScaleParameters,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    project_retained: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary: Option<FileProof>,
    commands: Vec<CommandEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<ScaleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ScaleParameters {
    requested_items: usize,
    batch_size: usize,
    requested_modules: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    class: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_digest: Option<VerificationDigest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ScaleResult {
    final_revision: String,
    generated_modules: usize,
    generated_functions: usize,
    call_depth: usize,
    caller_fanout: usize,
    base_modules: usize,
    measured_local_modules: usize,
    total_modules: usize,
    creation: Timing,
    apply_batches: Vec<BatchMeasurement>,
    transaction_request_bytes: u64,
    local_create: LocalMutation,
    local_rename: LocalMutation,
    orient: OrientMeasurement,
    exact_find_cold_index: FindMeasurement,
    exact_find: FindMeasurement,
    exact_show: ShowMeasurement,
    deep_doctor: DoctorMeasurement,
    build: BuildMeasurement,
    backup: BackupMeasurement,
    canonical_store_bytes: u64,
    store_bytes_with_indexes: u64,
    store_inventory: BTreeMap<String, AreaMeasurement>,
    platform: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Timing {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BatchMeasurement {
    kind: String,
    start: usize,
    end: usize,
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalMutation {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    revision: String,
    validation: ValidationMeasurement,
    store_delta: BTreeMap<String, AreaDelta>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ValidationMeasurement {
    profile: String,
    graph_valid: bool,
    full_oracle_equal: bool,
    modules_checked: u64,
    declarations_checked: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OrientMeasurement {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    returned_items: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FindMeasurement {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    work: u64,
    matches: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ShowMeasurement {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DoctorMeasurement {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    valid: bool,
    deep: bool,
    modules_checked: u64,
    revisions_checked: u64,
    roots_checked: u64,
    receipts_checked: u64,
    rebuilt_indexes: u64,
    revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildMeasurement {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    artifact_bytes: u64,
    artifact_digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BackupMeasurement {
    elapsed_nanoseconds: u64,
    response_bytes: u64,
    backup_bytes: u64,
    backup_digest: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AreaMeasurement {
    files: u64,
    bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AreaDelta {
    files: i64,
    bytes: i64,
}

#[derive(Debug, Deserialize)]
struct CliEnvelope {
    ok: bool,
    status: String,
    result: Option<Value>,
    error: Option<Value>,
}

struct Invocation {
    result: Value,
    timing: Timing,
}

struct Runner {
    repository: PathBuf,
    evidence_directory: PathBuf,
    ordinal: usize,
    commands: Vec<CommandEvidence>,
}

impl Runner {
    fn new(repository: &Path, evidence_directory: &Path) -> Self {
        Self {
            repository: repository.to_path_buf(),
            evidence_directory: evidence_directory.to_path_buf(),
            ordinal: 0,
            commands: Vec::new(),
        }
    }

    fn invoke(
        &mut self,
        name: &str,
        binary: &Path,
        arguments: Vec<String>,
    ) -> Result<Invocation, DevError> {
        let ordinal = self.ordinal;
        self.ordinal = self
            .ordinal
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("scale command ordinal overflow"))?;
        let mut command = vec![binary.to_string_lossy().into_owned()];
        command.extend(arguments);
        let stdout_path = self
            .evidence_directory
            .join(format!("command-{ordinal:06}-{name}.stdout.log"));
        let stderr_path = self
            .evidence_directory
            .join(format!("command-{ordinal:06}-{name}.stderr.log"));
        let observation = process::run(
            &ProcessSpec {
                command: command.clone(),
                cwd: self.repository.clone(),
                environment: process::environment(),
                timeout: COMMAND_TIMEOUT,
                maximum_stdout_bytes: MAXIMUM_STDOUT_BYTES,
                maximum_stderr_bytes: MAXIMUM_STDERR_BYTES,
                stdout_path: stdout_path.clone(),
                stderr_path: stderr_path.clone(),
                unavailable_exit_code: None,
            },
            &self.repository,
        );
        let response = process::read_bounded(&stdout_path, MAXIMUM_STDOUT_BYTES);
        let response_digest = response
            .as_ref()
            .ok()
            .map(|bytes| VerificationDigest::of(bytes));
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            process: observation.clone(),
            response_digest,
        });
        if observation.status != ProcessStatus::Passed {
            return Err(DevError::infrastructure(format!(
                "public CLI command '{name}' ended as {:?}: {}",
                observation.status,
                observation.reason.as_deref().unwrap_or("unknown")
            )));
        }
        if observation.stderr.bytes.unwrap_or(0) != 0 {
            let excerpt = process::excerpt(&stderr_path, 512)
                .unwrap_or_else(|_| "stderr unavailable".to_owned());
            return Err(DevError::infrastructure(format!(
                "public CLI command '{name}' wrote stderr: {excerpt}"
            )));
        }
        let response = response?;
        let envelope: CliEnvelope = serde_json::from_slice(&response).map_err(|error| {
            DevError::corrupt(format!(
                "public CLI command '{name}' returned invalid JSON: {error}"
            ))
        })?;
        if !envelope.ok {
            let failure = envelope
                .error
                .as_ref()
                .map(compact_value)
                .unwrap_or_else(|| envelope.status.clone());
            return Err(DevError::infrastructure(format!(
                "public CLI command '{name}' failed as '{}': {failure}",
                envelope.status
            )));
        }
        let result = envelope.result.ok_or_else(|| {
            DevError::corrupt(format!("public CLI command '{name}' omitted result"))
        })?;
        Ok(Invocation {
            result,
            timing: Timing {
                elapsed_nanoseconds: observation.elapsed_nanoseconds,
                response_bytes: response.len() as u64,
            },
        })
    }
}

#[derive(Serialize)]
struct ChangeRequest {
    contract_version: u16,
    base_revision: String,
    idempotency_key: String,
    preconditions: Vec<Value>,
    changes: Vec<ScaleChange>,
    budget: ChangeBudget,
    intent: String,
}

#[derive(Serialize)]
struct ChangeBudget {
    maximum_operations: usize,
    maximum_work: u64,
    maximum_affected_owners: u64,
}

#[derive(Serialize)]
#[serde(tag = "change", rename_all = "snake_case")]
enum ScaleChange {
    CreateModule {
        #[serde(rename = "as")]
        symbol: String,
        name: String,
    },
    CreateFunction {
        #[serde(rename = "as")]
        symbol: String,
        module: String,
        name: String,
        result: UnitType,
        body: ScaleExpression,
        exported: bool,
    },
    RenameModule {
        module: String,
        new_name: String,
    },
}

#[derive(Serialize)]
struct UnitType {
    #[serde(rename = "type")]
    kind: &'static str,
}

impl UnitType {
    fn unit() -> Self {
        Self { kind: "unit" }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ScaleExpression {
    Unit { unit: bool },
    Call { call: String },
}

struct ApplyResult {
    revision: String,
    allocated: BTreeMap<String, String>,
    validation: ValidationMeasurement,
    timing: Timing,
}

struct Generation {
    generated_modules: usize,
    generated_functions: usize,
    call_depth: usize,
    caller_fanout: usize,
    batches: Vec<BatchMeasurement>,
    request_bytes: u64,
}

struct ChangeContext<'a> {
    runner: &'a mut Runner,
    binary: &'a Path,
    project: &'a Path,
    revision: String,
    request_ordinal: usize,
    request_bytes: u64,
    batches: Vec<BatchMeasurement>,
}

impl ChangeContext<'_> {
    fn apply(
        &mut self,
        kind: &str,
        start: usize,
        end: usize,
        changes: Vec<ScaleChange>,
        record_generation: bool,
    ) -> Result<ApplyResult, DevError> {
        let request = ChangeRequest {
            contract_version: PUBLIC_CHANGE_CONTRACT_VERSION,
            base_revision: self.revision.clone(),
            idempotency_key: format!("scale-{kind}-{start}-{end}-{}", self.request_ordinal),
            preconditions: Vec::new(),
            budget: ChangeBudget {
                maximum_operations: changes.len(),
                maximum_work: 10_000_000,
                maximum_affected_owners: 100_000,
            },
            changes,
            intent: format!("Deterministic public CLI scale fixture: {kind}"),
        };
        self.request_ordinal = self
            .request_ordinal
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("scale request ordinal overflow"))?;
        let encoded = serde_json::to_vec(&request).map_err(|error| {
            DevError::infrastructure(format!("encode scale change request: {error}"))
        })?;
        let request_path = self.project.join(format!(
            ".lkjscript-dev-scale-request-{:06}.json",
            self.request_ordinal
        ));
        evidence::publish(&request_path, &encoded)?;
        let invocation = self.runner.invoke(
            kind,
            self.binary,
            vec![
                "--project".to_owned(),
                self.project.to_string_lossy().into_owned(),
                "change".to_owned(),
                "--request-file".to_owned(),
                request_path.to_string_lossy().into_owned(),
                "--commit".to_owned(),
            ],
        );
        let removal = fs::remove_file(&request_path).map_err(|error| {
            DevError::infrastructure(format!(
                "remove scale request '{}': {error}",
                request_path.display()
            ))
        });
        let invocation = match (invocation, removal) {
            (Ok(invocation), Ok(())) => invocation,
            (Err(error), _) => return Err(error),
            (Ok(_), Err(error)) => return Err(error),
        };
        let revision = string_at(&invocation.result, &["published_revision"])?;
        let allocated = allocated_identities(&invocation.result)?;
        let validation = validation_at(&invocation.result)?;
        if record_generation {
            self.request_bytes = self
                .request_bytes
                .checked_add(encoded.len() as u64)
                .ok_or_else(|| DevError::infrastructure("scale request byte count overflow"))?;
            self.batches.push(BatchMeasurement {
                kind: kind.to_owned(),
                start,
                end,
                elapsed_nanoseconds: invocation.timing.elapsed_nanoseconds,
                response_bytes: invocation.timing.response_bytes,
                revision: revision.clone(),
            });
        }
        self.revision.clone_from(&revision);
        Ok(ApplyResult {
            revision,
            allocated,
            validation,
            timing: invocation.timing,
        })
    }
}

enum Project {
    Temporary(tempfile::TempDir),
    Retained(PathBuf),
}

impl Project {
    fn create(retain: Option<&Path>) -> Result<Self, DevError> {
        if let Some(path) = retain {
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                DevError::usage(format!(
                    "inspect retained project '{}': {error}",
                    path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(DevError::usage(format!(
                    "retained project '{}' must be a non-symlink directory",
                    path.display()
                )));
            }
            if fs::read_dir(path)
                .map_err(|error| {
                    DevError::usage(format!(
                        "read retained project '{}': {error}",
                        path.display()
                    ))
                })?
                .next()
                .is_some()
            {
                return Err(DevError::usage(format!(
                    "retained project '{}' is not empty",
                    path.display()
                )));
            }
            return Ok(Self::Retained(path.to_path_buf()));
        }
        tempfile::Builder::new()
            .prefix("lkjscript-scale-")
            .tempdir()
            .map(Self::Temporary)
            .map_err(|error| DevError::infrastructure(format!("create scale project: {error}")))
    }

    fn path(&self) -> &Path {
        match self {
            Self::Temporary(directory) => directory.path(),
            Self::Retained(path) => path,
        }
    }
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse(arguments)?;
    let repository = repository_root()?;
    let evidence_directory = new_evidence_directory(&repository)?;
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let mut runner = Runner::new(&repository, &evidence_directory);
    let mut binary_proof = None;
    let result = (|| {
        let binary = resolve_binary(&repository, &options.binary)?;
        binary_proof = Some(evidence::proof(
            &binary,
            binary.to_string_lossy().into_owned(),
        )?);
        let project = Project::create(options.retain.as_deref())?;
        run_scale(&options, &binary, project.path(), &mut runner)
    })();
    let (status, scale_result, failure) = match result {
        Ok(result) => (ScaleStatus::Passed, Some(result), None),
        Err(error) => (
            ScaleStatus::Failed,
            None,
            Some(Failure {
                class: error.kind().to_owned(),
                message: error.message().to_owned(),
            }),
        ),
    };
    let receipt = ScaleReceipt {
        contract_version: SCALE_CONTRACT_VERSION,
        status,
        topology: options.topology,
        semantic_shape: options.topology.semantic_shape().to_owned(),
        parameters: ScaleParameters {
            requested_items: options.items,
            batch_size: options.batch,
            requested_modules: options.modules,
        },
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        project_retained: options.retain.is_some(),
        binary: binary_proof,
        commands: runner.commands,
        result: scale_result,
        failure,
    };
    let path = evidence_directory.join("receipt.json");
    let published = evidence::publish_json(&path, &receipt)?;
    print_summary(&repository, &options, &receipt, &published)?;
    Ok(if status == ScaleStatus::Passed { 0 } else { 1 })
}

fn run_scale(
    options: &Options,
    binary: &Path,
    project: &Path,
    runner: &mut Runner,
) -> Result<ScaleResult, DevError> {
    let created = runner.invoke(
        "new",
        binary,
        vec![
            "new".to_owned(),
            project.to_string_lossy().into_owned(),
            "--template".to_owned(),
            "minimal".to_owned(),
            "--name".to_owned(),
            "scale".to_owned(),
        ],
    )?;
    let revision = string_at(&created.result, &["revision"])?;
    let mut context = ChangeContext {
        runner,
        binary,
        project,
        revision,
        request_ordinal: 0,
        request_bytes: 0,
        batches: Vec::new(),
    };
    let generation = generate(options, &mut context)?;

    let store = project.join(".lkjscript/meaning");
    let before_local_create = store_inventory(&store)?;
    let local_create = context.apply(
        "local-create",
        0,
        1,
        vec![ScaleChange::CreateModule {
            symbol: "$local-module".to_owned(),
            name: "scale.localedit".to_owned(),
        }],
        false,
    )?;
    let local_module = local_create
        .allocated
        .get("$local-module")
        .cloned()
        .ok_or_else(|| DevError::corrupt("local module identity is absent"))?;
    let after_local_create = store_inventory(&store)?;
    let local_rename = context.apply(
        "local-rename",
        0,
        1,
        vec![ScaleChange::RenameModule {
            module: local_module.clone(),
            new_name: "scale.renamedlocaledit".to_owned(),
        }],
        false,
    )?;
    let after_local_rename = store_inventory(&store)?;

    let orient = context.runner.invoke(
        "orient",
        binary,
        project_arguments(project, &["inspect", "project", "--limit", "10"]),
    )?;
    let total_modules = usize_at(&orient.result, &["module_count"])?;
    let expected_nonbase = generation
        .generated_modules
        .checked_add(1)
        .ok_or_else(|| DevError::infrastructure("generated module count overflow"))?;
    let base_modules = total_modules.checked_sub(expected_nonbase).ok_or_else(|| {
        DevError::corrupt("public project module count is smaller than generated topology")
    })?;
    let find_cold = context.runner.invoke(
        "find-cold",
        binary,
        project_arguments(
            project,
            &[
                "query",
                "find",
                "scale.renamedlocaledit",
                "--exact",
                "--limit",
                "10",
            ],
        ),
    )?;
    let find = context.runner.invoke(
        "find",
        binary,
        project_arguments(
            project,
            &[
                "query",
                "find",
                "scale.renamedlocaledit",
                "--exact",
                "--limit",
                "10",
            ],
        ),
    )?;
    let show = context.runner.invoke(
        "show",
        binary,
        project_arguments(project, &["inspect", "owner", &local_module]),
    )?;
    let doctor = context.runner.invoke(
        "doctor",
        binary,
        project_arguments(project, &["doctor", "--deep"]),
    )?;
    let artifact_path = project.join("scale.lkja");
    let build = context.runner.invoke(
        "build",
        binary,
        project_arguments(
            project,
            &["build", "--output", &artifact_path.to_string_lossy()],
        ),
    )?;
    let backup_path = project.join("scale.lkjb");
    let backup = context.runner.invoke(
        "backup",
        binary,
        project_arguments(
            project,
            &["backup", "--output", &backup_path.to_string_lossy()],
        ),
    )?;
    let final_inventory = store_inventory(&store)?;
    Ok(ScaleResult {
        final_revision: local_rename.revision.clone(),
        generated_modules: generation.generated_modules,
        generated_functions: generation.generated_functions,
        call_depth: generation.call_depth,
        caller_fanout: generation.caller_fanout,
        base_modules,
        measured_local_modules: 1,
        total_modules,
        creation: created.timing,
        apply_batches: generation.batches,
        transaction_request_bytes: generation.request_bytes,
        local_create: LocalMutation {
            elapsed_nanoseconds: local_create.timing.elapsed_nanoseconds,
            response_bytes: local_create.timing.response_bytes,
            revision: local_create.revision,
            validation: local_create.validation,
            store_delta: inventory_delta(&before_local_create, &after_local_create)?,
        },
        local_rename: LocalMutation {
            elapsed_nanoseconds: local_rename.timing.elapsed_nanoseconds,
            response_bytes: local_rename.timing.response_bytes,
            revision: local_rename.revision,
            validation: local_rename.validation,
            store_delta: inventory_delta(&after_local_create, &after_local_rename)?,
        },
        orient: OrientMeasurement {
            elapsed_nanoseconds: orient.timing.elapsed_nanoseconds,
            response_bytes: orient.timing.response_bytes,
            returned_items: array_len_at(&orient.result, &["modules"])?,
        },
        exact_find_cold_index: find_measurement(find_cold)?,
        exact_find: find_measurement(find)?,
        exact_show: ShowMeasurement {
            elapsed_nanoseconds: show.timing.elapsed_nanoseconds,
            response_bytes: show.timing.response_bytes,
            id: string_at(&show.result, &["id"])?,
        },
        deep_doctor: doctor_measurement(doctor)?,
        build: BuildMeasurement {
            elapsed_nanoseconds: build.timing.elapsed_nanoseconds,
            response_bytes: build.timing.response_bytes,
            artifact_bytes: regular_file_bytes(&artifact_path)?,
            artifact_digest: string_at(&build.result, &["receipt", "artifact_digest"])?,
        },
        backup: BackupMeasurement {
            elapsed_nanoseconds: backup.timing.elapsed_nanoseconds,
            response_bytes: backup.timing.response_bytes,
            backup_bytes: payload_bytes(&backup_path)?,
            backup_digest: string_at(&backup.result, &["receipt", "digest"])?,
        },
        canonical_store_bytes: byte_count(&store, false)?,
        store_bytes_with_indexes: byte_count(&store, true)?,
        store_inventory: final_inventory,
        platform: format!("{} {}", platform_name(), std::env::consts::ARCH),
    })
}

fn generate(options: &Options, context: &mut ChangeContext<'_>) -> Result<Generation, DevError> {
    let (generated_modules, generated_functions, call_depth, caller_fanout) = match options.topology
    {
        Topology::IndependentModules => {
            create_modules(
                context,
                options.items,
                options.batch,
                "scale.module",
                "modules",
            )?;
            (options.items, 0, 0, 0)
        }
        Topology::SmallFunctions => {
            let modules = options
                .modules
                .ok_or_else(|| DevError::usage("small-functions requires a module count"))?;
            let module_ids = create_modules(
                context,
                modules,
                options.batch,
                "scale.functions.module",
                "function-modules",
            )?;
            create_unit_functions(
                context,
                options.items,
                options.batch,
                &module_ids,
                "small-functions",
            )?;
            (modules, options.items, 1, 0)
        }
        Topology::WideModule => {
            let modules = create_modules(context, 1, options.batch, "scale.wide", "wide-module")?;
            create_unit_functions(
                context,
                options.items,
                options.batch,
                &modules,
                "wide-functions",
            )?;
            (1, options.items, 1, 0)
        }
        Topology::DeepChain => {
            let modules = create_modules(context, 1, options.batch, "scale.chain", "chain-module")?;
            create_chain(context, options.items, options.batch, &modules[0])?;
            (1, options.items, options.items, 0)
        }
        Topology::WideFanout => {
            let modules =
                create_modules(context, 1, options.batch, "scale.fanout", "fanout-module")?;
            create_fanout(context, options.items, options.batch, &modules[0])?;
            (1, options.items, 1, options.items.saturating_sub(1))
        }
    };
    Ok(Generation {
        generated_modules,
        generated_functions,
        call_depth,
        caller_fanout,
        batches: std::mem::take(&mut context.batches),
        request_bytes: context.request_bytes,
    })
}

fn create_modules(
    context: &mut ChangeContext<'_>,
    count: usize,
    batch: usize,
    prefix: &str,
    kind: &str,
) -> Result<Vec<String>, DevError> {
    let mut identities = Vec::with_capacity(count);
    for start in (0..count).step_by(batch) {
        let end = start.saturating_add(batch).min(count);
        let changes = (start..end)
            .map(|ordinal| ScaleChange::CreateModule {
                symbol: format!("$module-{ordinal}"),
                name: format!("{prefix}{ordinal:06}"),
            })
            .collect();
        let applied = context.apply(kind, start, end, changes, true)?;
        for ordinal in start..end {
            identities.push(required_allocated(
                &applied.allocated,
                &format!("$module-{ordinal}"),
            )?);
        }
    }
    Ok(identities)
}

fn create_unit_functions(
    context: &mut ChangeContext<'_>,
    count: usize,
    batch: usize,
    modules: &[String],
    kind: &str,
) -> Result<(), DevError> {
    if modules.is_empty() {
        return Err(DevError::infrastructure("function topology has no modules"));
    }
    for start in (0..count).step_by(batch) {
        let end = start.saturating_add(batch).min(count);
        let changes = (start..end)
            .map(|ordinal| ScaleChange::CreateFunction {
                symbol: format!("$function-{ordinal}"),
                module: modules[ordinal % modules.len()].clone(),
                name: format!("f{ordinal:06}"),
                result: UnitType::unit(),
                body: ScaleExpression::Unit { unit: true },
                exported: false,
            })
            .collect();
        context.apply(kind, start, end, changes, true)?;
    }
    Ok(())
}

fn create_chain(
    context: &mut ChangeContext<'_>,
    count: usize,
    batch: usize,
    module: &str,
) -> Result<(), DevError> {
    let mut previous: Option<String> = None;
    for start in (0..count).step_by(batch) {
        let end = start.saturating_add(batch).min(count);
        let mut changes = Vec::with_capacity(end - start);
        for ordinal in start..end {
            let body = if ordinal == 0 {
                ScaleExpression::Unit { unit: true }
            } else if ordinal == start {
                ScaleExpression::Call {
                    call: previous.clone().ok_or_else(|| {
                        DevError::infrastructure("deep chain lost its previous function")
                    })?,
                }
            } else {
                ScaleExpression::Call {
                    call: format!("$function-{}", ordinal - 1),
                }
            };
            changes.push(ScaleChange::CreateFunction {
                symbol: format!("$function-{ordinal}"),
                module: module.to_owned(),
                name: format!("f{ordinal:06}"),
                result: UnitType::unit(),
                body,
                exported: false,
            });
        }
        let applied = context.apply("deep-chain", start, end, changes, true)?;
        previous = Some(required_allocated(
            &applied.allocated,
            &format!("$function-{}", end - 1),
        )?);
    }
    Ok(())
}

fn create_fanout(
    context: &mut ChangeContext<'_>,
    count: usize,
    batch: usize,
    module: &str,
) -> Result<(), DevError> {
    let root = context.apply(
        "fanout-root",
        0,
        1,
        vec![ScaleChange::CreateFunction {
            symbol: "$function-0".to_owned(),
            module: module.to_owned(),
            name: "f000000".to_owned(),
            result: UnitType::unit(),
            body: ScaleExpression::Unit { unit: true },
            exported: false,
        }],
        true,
    )?;
    let root = required_allocated(&root.allocated, "$function-0")?;
    for start in (1..count).step_by(batch) {
        let end = start.saturating_add(batch).min(count);
        let changes = (start..end)
            .map(|ordinal| ScaleChange::CreateFunction {
                symbol: format!("$function-{ordinal}"),
                module: module.to_owned(),
                name: format!("f{ordinal:06}"),
                result: UnitType::unit(),
                body: ScaleExpression::Call { call: root.clone() },
                exported: false,
            })
            .collect();
        context.apply("wide-fanout", start, end, changes, true)?;
    }
    Ok(())
}

fn validation_at(value: &Value) -> Result<ValidationMeasurement, DevError> {
    Ok(ValidationMeasurement {
        profile: string_at(value, &["receipt", "validation", "profile"])?,
        graph_valid: bool_at(value, &["receipt", "validation", "graph_valid"])?,
        full_oracle_equal: bool_at(value, &["receipt", "validation", "full_oracle_equal"])?,
        modules_checked: u64_at(value, &["receipt", "validation", "modules_checked"])?,
        declarations_checked: u64_at(value, &["receipt", "validation", "declarations_checked"])?,
    })
}

fn find_measurement(invocation: Invocation) -> Result<FindMeasurement, DevError> {
    Ok(FindMeasurement {
        elapsed_nanoseconds: invocation.timing.elapsed_nanoseconds,
        response_bytes: invocation.timing.response_bytes,
        work: u64_at(&invocation.result, &["work"])?,
        matches: u64_at(&invocation.result, &["total_items"])?,
    })
}

fn doctor_measurement(invocation: Invocation) -> Result<DoctorMeasurement, DevError> {
    Ok(DoctorMeasurement {
        elapsed_nanoseconds: invocation.timing.elapsed_nanoseconds,
        response_bytes: invocation.timing.response_bytes,
        valid: bool_at(&invocation.result, &["valid"])?,
        deep: bool_at(&invocation.result, &["deep"])?,
        modules_checked: u64_at(&invocation.result, &["modules_checked"])?,
        revisions_checked: u64_at(&invocation.result, &["revisions_checked"])?,
        roots_checked: u64_at(&invocation.result, &["roots_checked"])?,
        receipts_checked: u64_at(&invocation.result, &["receipts_checked"])?,
        rebuilt_indexes: u64_at(&invocation.result, &["rebuilt_indexes"])?,
        revision: string_at(&invocation.result, &["revision"])?,
    })
}

fn allocated_identities(value: &Value) -> Result<BTreeMap<String, String>, DevError> {
    let object = at(value, &["allocated_identities"])?
        .as_object()
        .ok_or_else(|| DevError::corrupt("allocated_identities is not an object"))?;
    object
        .iter()
        .map(|(symbol, value)| string_at(value, &["id"]).map(|identity| (symbol.clone(), identity)))
        .collect()
}

fn required_allocated(
    allocated: &BTreeMap<String, String>,
    symbol: &str,
) -> Result<String, DevError> {
    allocated
        .get(symbol)
        .cloned()
        .ok_or_else(|| DevError::corrupt(format!("allocated identity '{symbol}' is absent")))
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value, DevError> {
    let mut current = value;
    for field in path {
        current = current.get(*field).ok_or_else(|| {
            DevError::corrupt(format!("public CLI result omitted '{}'", path.join(".")))
        })?;
    }
    Ok(current)
}

fn string_at(value: &Value, path: &[&str]) -> Result<String, DevError> {
    at(value, path)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| DevError::corrupt(format!("'{}' is not text", path.join("."))))
}

fn u64_at(value: &Value, path: &[&str]) -> Result<u64, DevError> {
    at(value, path)?.as_u64().ok_or_else(|| {
        DevError::corrupt(format!("'{}' is not an unsigned integer", path.join(".")))
    })
}

fn usize_at(value: &Value, path: &[&str]) -> Result<usize, DevError> {
    usize::try_from(u64_at(value, path)?)
        .map_err(|_| DevError::corrupt(format!("'{}' does not fit usize", path.join("."))))
}

fn bool_at(value: &Value, path: &[&str]) -> Result<bool, DevError> {
    at(value, path)?
        .as_bool()
        .ok_or_else(|| DevError::corrupt(format!("'{}' is not a boolean", path.join("."))))
}

fn array_len_at(value: &Value, path: &[&str]) -> Result<usize, DevError> {
    at(value, path)?
        .as_array()
        .map(Vec::len)
        .ok_or_else(|| DevError::corrupt(format!("'{}' is not an array", path.join("."))))
}

fn compact_value(value: &Value) -> String {
    let mut encoded = serde_json::to_string(value).unwrap_or_else(|_| "invalid_error".to_owned());
    encoded.truncate(2_048);
    encoded
}

fn project_arguments(project: &Path, command: &[&str]) -> Vec<String> {
    let mut arguments = vec![
        "--project".to_owned(),
        project.to_string_lossy().into_owned(),
    ];
    arguments.extend(command.iter().map(|value| (*value).to_owned()));
    arguments
}

fn store_inventory(root: &Path) -> Result<BTreeMap<String, AreaMeasurement>, DevError> {
    let mut inventory: BTreeMap<String, AreaMeasurement> = BTreeMap::new();
    visit_files(root, root, &mut |relative, metadata| {
        if relative.file_name().and_then(|value| value.to_str()) == Some("LOCK") {
            return Ok(());
        }
        let area = relative
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .ok_or_else(|| DevError::infrastructure("store path has no portable area"))?;
        let entry = inventory.entry(area.to_owned()).or_default();
        entry.files = entry
            .files
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("store file count overflow"))?;
        entry.bytes = entry
            .bytes
            .checked_add(metadata.len())
            .ok_or_else(|| DevError::infrastructure("store byte count overflow"))?;
        Ok(())
    })?;
    Ok(inventory)
}

fn inventory_delta(
    before: &BTreeMap<String, AreaMeasurement>,
    after: &BTreeMap<String, AreaMeasurement>,
) -> Result<BTreeMap<String, AreaDelta>, DevError> {
    let mut areas = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<Vec<_>>();
    areas.sort();
    areas.dedup();
    let mut delta = BTreeMap::new();
    for area in areas {
        let old = before.get(&area).cloned().unwrap_or_default();
        let new = after.get(&area).cloned().unwrap_or_default();
        if old == new {
            continue;
        }
        delta.insert(
            area,
            AreaDelta {
                files: signed_delta(new.files, old.files)?,
                bytes: signed_delta(new.bytes, old.bytes)?,
            },
        );
    }
    Ok(delta)
}

fn signed_delta(new: u64, old: u64) -> Result<i64, DevError> {
    let new = i64::try_from(new)
        .map_err(|_| DevError::infrastructure("scale observation exceeds i64"))?;
    let old = i64::try_from(old)
        .map_err(|_| DevError::infrastructure("scale observation exceeds i64"))?;
    new.checked_sub(old)
        .ok_or_else(|| DevError::infrastructure("scale delta overflow"))
}

fn byte_count(root: &Path, include_indexes: bool) -> Result<u64, DevError> {
    let mut total = 0_u64;
    visit_files(root, root, &mut |relative, metadata| {
        if relative.file_name().and_then(|value| value.to_str()) == Some("LOCK") {
            return Ok(());
        }
        let area = relative
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str());
        if !include_indexes && matches!(area, Some("indexes" | "drafts")) {
            return Ok(());
        }
        total = total
            .checked_add(metadata.len())
            .ok_or_else(|| DevError::infrastructure("store byte count overflow"))?;
        Ok(())
    })?;
    Ok(total)
}

fn payload_bytes(path: &Path) -> Result<u64, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect payload '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(DevError::infrastructure(format!(
            "payload '{}' is a symlink",
            path.display()
        )));
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(DevError::infrastructure(format!(
            "payload '{}' is not a file or directory",
            path.display()
        )));
    }
    let mut total = 0_u64;
    visit_files(path, path, &mut |_, metadata| {
        total = total
            .checked_add(metadata.len())
            .ok_or_else(|| DevError::infrastructure("payload byte count overflow"))?;
        Ok(())
    })?;
    Ok(total)
}

fn regular_file_bytes(path: &Path) -> Result<u64, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect output '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "output '{}' is not a regular file",
            path.display()
        )));
    }
    Ok(metadata.len())
}

fn visit_files(
    root: &Path,
    directory: &Path,
    visitor: &mut impl FnMut(&Path, &fs::Metadata) -> Result<(), DevError>,
) -> Result<(), DevError> {
    for entry in fs::read_dir(directory).map_err(|error| {
        DevError::infrastructure(format!("read directory '{}': {error}", directory.display()))
    })? {
        let entry = entry
            .map_err(|error| DevError::infrastructure(format!("read directory entry: {error}")))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            DevError::infrastructure(format!("inspect path '{}': {error}", path.display()))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(DevError::infrastructure(format!(
                "refusing scale inventory symlink '{}'",
                path.display()
            )));
        }
        if metadata.is_dir() {
            visit_files(root, &path, visitor)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| DevError::infrastructure("scale inventory path escaped its root"))?;
            visitor(relative, &metadata)?;
        }
    }
    Ok(())
}

fn resolve_binary(repository: &Path, configured: &Path) -> Result<PathBuf, DevError> {
    let candidate = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        repository.join(configured)
    };
    let binary = candidate.canonicalize().map_err(|error| {
        DevError::usage(format!("resolve binary '{}': {error}", candidate.display()))
    })?;
    let metadata = fs::metadata(&binary).map_err(|error| {
        DevError::usage(format!("inspect binary '{}': {error}", binary.display()))
    })?;
    if !metadata.is_file() {
        return Err(DevError::usage(format!(
            "binary '{}' is not a regular file",
            binary.display()
        )));
    }
    Ok(binary)
}

fn new_evidence_directory(repository: &Path) -> Result<PathBuf, DevError> {
    let root = repository.join(".artifacts/lkjscript-dev/scale");
    fs::create_dir_all(&root).map_err(|error| {
        DevError::infrastructure(format!(
            "create scale evidence root '{}': {error}",
            root.display()
        ))
    })?;
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let path = root.join(format!(
        "{}-{}-{ordinal}",
        unix_nanoseconds()?,
        std::process::id()
    ));
    fs::create_dir(&path).map_err(|error| {
        DevError::infrastructure(format!(
            "create scale evidence '{}': {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn parse(mut arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let topology = crate::next_utf8(&mut arguments, "scale topology")?
        .ok_or_else(|| DevError::usage("scale topology is required"))?;
    let topology = Topology::parse(&topology)?;
    let mut items = 10_000_usize;
    let mut batch = 10_000_usize;
    let mut modules = None;
    let mut binary = PathBuf::from("target/release/lkjscript");
    let mut retain = None;
    let mut machine = false;
    while let Some(argument) = crate::next_utf8(&mut arguments, "scale option")? {
        match argument.as_str() {
            "--items" => items = parse_usize(&mut arguments, "--items")?,
            "--batch" => batch = parse_usize(&mut arguments, "--batch")?,
            "--modules" => modules = Some(parse_usize(&mut arguments, "--modules")?),
            "--binary" => {
                binary = PathBuf::from(required_value(&mut arguments, "--binary")?);
            }
            "--retain" => {
                retain = Some(PathBuf::from(required_value(&mut arguments, "--retain")?));
            }
            "--machine" if !machine => machine = true,
            value => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate scale option '{value}'"
                )));
            }
        }
    }
    if items == 0 || items > MAXIMUM_ITEMS {
        return Err(DevError::usage(format!(
            "--items must be between 1 and {MAXIMUM_ITEMS}"
        )));
    }
    if batch == 0 || batch > MAXIMUM_BATCH {
        return Err(DevError::usage(format!(
            "--batch must be between 1 and {MAXIMUM_BATCH}"
        )));
    }
    match (topology, modules) {
        (Topology::SmallFunctions, None) => modules = Some(items.min(64)),
        (Topology::SmallFunctions, Some(0)) => {
            return Err(DevError::usage("--modules must be at least 1"));
        }
        (Topology::SmallFunctions, Some(value)) if value > MAXIMUM_ITEMS => {
            return Err(DevError::usage(format!(
                "--modules must not exceed {MAXIMUM_ITEMS}"
            )));
        }
        (Topology::SmallFunctions, Some(_)) => {}
        (_, Some(_)) => {
            return Err(DevError::usage("--modules applies only to small-functions"));
        }
        (_, None) => {}
    }
    Ok(Options {
        topology,
        items,
        batch,
        modules,
        binary,
        retain,
        machine,
    })
}

fn parse_usize(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<usize, DevError> {
    let value = required_value(arguments, option)?;
    value
        .parse()
        .map_err(|_| DevError::usage(format!("{option} must be an unsigned integer")))
}

fn required_value(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, DevError> {
    crate::next_utf8(arguments, option)?
        .ok_or_else(|| DevError::usage(format!("{option} requires a value")))
}

fn print_summary(
    repository: &Path,
    options: &Options,
    receipt: &ScaleReceipt,
    published: &PublishedEvidence,
) -> Result<(), DevError> {
    #[derive(Serialize)]
    struct Summary<'a> {
        contract_version: u32,
        status: ScaleStatus,
        topology: &'a str,
        requested_items: usize,
        elapsed_nanoseconds: u64,
        receipt: String,
        receipt_bytes: u64,
        receipt_digest: &'a VerificationDigest,
        failure: &'a Option<Failure>,
    }
    let summary = Summary {
        contract_version: receipt.contract_version,
        status: receipt.status,
        topology: options.topology.cli_name(),
        requested_items: options.items,
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        receipt: evidence::relative(repository, &published.path),
        receipt_bytes: published.bytes,
        receipt_digest: &published.digest,
        failure: &receipt.failure,
    };
    if options.machine {
        println!(
            "{}",
            serde_json::to_string(&summary).map_err(|error| {
                DevError::infrastructure(format!("encode compact scale summary: {error}"))
            })?
        );
    } else {
        println!(
            "scale {:?}: topology={} items={} elapsed={:.3}s receipt={} digest={}",
            receipt.status,
            options.topology.cli_name(),
            options.items,
            receipt.elapsed_nanoseconds as f64 / 1_000_000_000.0,
            summary.receipt,
            summary.receipt_digest,
        );
        if let Some(failure) = &receipt.failure {
            println!(
                "failure: class={} message={}",
                failure.class, failure.message
            );
        }
    }
    Ok(())
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| DevError::infrastructure("resolve repository root"))
}

fn platform_name() -> &'static str {
    match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        value => value,
    }
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_names_are_closed_and_truthful() {
        assert_eq!(
            Topology::parse("deep-chain").expect("deep chain"),
            Topology::DeepChain
        );
        assert!(Topology::parse("dependency-chain").is_err());
        assert!(Topology::DeepChain.semantic_shape().contains("call_chain"));
        assert!(Topology::WideFanout.semantic_shape().contains("callers"));
    }

    #[test]
    fn typed_requests_encode_unit_and_call_bodies_without_private_apis() {
        let request = ChangeRequest {
            contract_version: PUBLIC_CHANGE_CONTRACT_VERSION,
            base_revision: "rev_test".to_owned(),
            idempotency_key: "test".to_owned(),
            preconditions: Vec::new(),
            changes: vec![
                ScaleChange::CreateFunction {
                    symbol: "$root".to_owned(),
                    module: "mod_test".to_owned(),
                    name: "root".to_owned(),
                    result: UnitType::unit(),
                    body: ScaleExpression::Unit { unit: true },
                    exported: false,
                },
                ScaleChange::CreateFunction {
                    symbol: "$caller".to_owned(),
                    module: "mod_test".to_owned(),
                    name: "caller".to_owned(),
                    result: UnitType::unit(),
                    body: ScaleExpression::Call {
                        call: "$root".to_owned(),
                    },
                    exported: false,
                },
            ],
            budget: ChangeBudget {
                maximum_operations: 2,
                maximum_work: 10,
                maximum_affected_owners: 10,
            },
            intent: "test".to_owned(),
        };
        let value = serde_json::to_value(request).expect("typed public request");
        assert_eq!(value["changes"][0]["body"]["unit"], true);
        assert_eq!(value["changes"][1]["body"]["call"], "$root");
    }

    #[test]
    fn inventory_delta_preserves_independent_file_and_byte_units() {
        let before =
            BTreeMap::from([("objects".to_owned(), AreaMeasurement { files: 2, bytes: 9 })]);
        let after = BTreeMap::from([(
            "objects".to_owned(),
            AreaMeasurement {
                files: 3,
                bytes: 14,
            },
        )]);
        let delta = inventory_delta(&before, &after).expect("inventory delta");
        assert_eq!(delta["objects"].files, 1);
        assert_eq!(delta["objects"].bytes, 5);
    }

    #[test]
    fn retained_scale_receipt_is_strictly_typed() {
        let receipt = ScaleReceipt {
            contract_version: SCALE_CONTRACT_VERSION,
            status: ScaleStatus::Failed,
            topology: Topology::WideModule,
            semantic_shape: Topology::WideModule.semantic_shape().to_owned(),
            parameters: ScaleParameters {
                requested_items: 1,
                batch_size: 1,
                requested_modules: None,
            },
            started_unix_nanoseconds: 1,
            completed_unix_nanoseconds: 2,
            elapsed_nanoseconds: 1,
            project_retained: false,
            binary: None,
            commands: Vec::new(),
            result: None,
            failure: Some(Failure {
                class: "infrastructure".to_owned(),
                message: "fixture".to_owned(),
            }),
        };
        let mut value = serde_json::to_value(&receipt).expect("typed scale receipt");
        let decoded: ScaleReceipt =
            serde_json::from_value(value.clone()).expect("decode typed scale receipt");
        assert_eq!(decoded.status, ScaleStatus::Failed);
        value["unknown"] = Value::Bool(true);
        assert!(serde_json::from_value::<ScaleReceipt>(value).is_err());
    }
}
