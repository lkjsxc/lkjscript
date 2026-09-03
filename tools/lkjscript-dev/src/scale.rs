#[path = "scale/model.rs"]
mod model;
#[path = "scale/runner.rs"]
mod runner;

use crate::error::DevError;
use crate::evidence::{self, PublishedEvidence, VerificationDigest};
use lkjscript::platform::contributor::{
    SemanticInventory, catalog_inventory, compact_change_default_maximum_operations,
    semantic_inventory,
};
use lkjscript::platform::control::{
    MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_COMPACT_RECORDS, parse_records, render_record,
};
use model::{
    AdmissionClassification, ArtifactEvidence, BatchEvidence, BuildEvidence, CandidateIdentity,
    CapabilityIdentity, CapabilitySection, CheckEvidence, CleanupEvidence, CleanupStatus,
    CompilationEvidence, FailureEvidence, FileAreaEvidence, HostObservation, Lifecycle,
    LogicalEvidence, ObservationEvidence, OperationIdentity, OracleEvidence, PublicReadEvidence,
    ReceiptStatus, RenameEvidence, SCALE_CONTRACT_VERSION, SCALE_SCHEMA, ScaleReceipt,
    ScenarioEvidence, SemanticCounts, SourceIdentity, ToolchainIdentity,
};
use runner::{Invocation, Runner, bool_field, directory_bytes, record, required_field, u64_field};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAXIMUM_ITEMS: u64 = 1_000_000;
const DEFAULT_ITEMS: u64 = 10_000;
const DEFAULT_BATCH: u64 = 1_000;
const DEFAULT_MAXIMUM_WALL_SECONDS: u64 = 7_200;
const MAXIMUM_WALL_SECONDS: u64 = 86_400;
const DEFAULT_MAXIMUM_RUN_BYTES: u64 = 64 * 1_073_741_824;
const MINIMUM_RUN_BYTES: u64 = 16 * 1_048_576;
const MAXIMUM_RUN_BYTES: u64 = 1_099_511_627_776;
const READ_LIMIT: &str = "16";
const READ_BYTES: &str = "131072";
const REQUIRED_OPERATIONS: [&str; 7] = [
    "new", "status", "inspect", "query", "change", "check", "build",
];
const HELP: &str = "usage: lkjscript-dev scale <independent-modules|small-functions|wide-module|deep-chain|wide-fanout> [--items N] [--batch N] [--modules N] [--lifecycle full|capacity] [--binary PATH] [--evidence-root ABSENT_ABSOLUTE_PATH] [--retain ABSENT_ABSOLUTE_PATH] [--maximum-wall-seconds N] [--maximum-run-bytes N] [--minimum-available-memory-bytes N] [--minimum-available-disk-bytes N] [--machine]\n\nfull performs reviewed construction, one reviewed rename, current bounded reads, check, a forced clean build, an exact-current build, typed semantic and catalog-oracle comparison, and cleanup. capacity performs reviewed construction, current bounded reads, typed semantic and catalog-oracle comparison, and cleanup without rename/check/build. --batch counts topology items; the current compact public change budget admits at most 1000 items per batch. Receipt schema lkjscript-semantic-scale-receipt contract 3.";
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

    const fn name(self) -> &'static str {
        match self {
            Self::IndependentModules => "independent-modules",
            Self::SmallFunctions => "small-functions",
            Self::WideModule => "wide-module",
            Self::DeepChain => "deep-chain",
            Self::WideFanout => "wide-fanout",
        }
    }

    const fn semantic_shape(self) -> &'static str {
        match self {
            Self::IndependentModules => "many_independent_modules",
            Self::SmallFunctions => "many_small_pure_functions_distributed_across_modules",
            Self::WideModule => "one_module_with_many_small_pure_functions",
            Self::DeepChain => "one_module_with_a_direct_call_chain",
            Self::WideFanout => "one_module_with_many_direct_callers_of_one_root",
        }
    }

    fn maximum_batch(self) -> u64 {
        let public_operations = compact_change_default_maximum_operations();
        let record_bound = match self {
            Self::IndependentModules => (MAXIMUM_COMPACT_RECORDS - 1) as u64,
            Self::SmallFunctions | Self::WideModule | Self::DeepChain | Self::WideFanout => {
                ((MAXIMUM_COMPACT_RECORDS - 1) / 2) as u64
            }
        };
        public_operations.min(record_bound)
    }
}

#[derive(Clone, Debug)]
struct Options {
    topology: Topology,
    items: u64,
    batch: u64,
    modules: Option<u64>,
    lifecycle: Lifecycle,
    binary: PathBuf,
    evidence_root: Option<PathBuf>,
    retain: Option<PathBuf>,
    maximum_wall_seconds: u64,
    maximum_run_bytes: u64,
    minimum_available_memory_bytes: u64,
    minimum_available_disk_bytes: u64,
    machine: bool,
}

#[derive(Clone, Debug)]
struct RequestDocument {
    bytes: Vec<u8>,
    records: u64,
}

#[derive(Default)]
struct RequestBuilder {
    bytes: Vec<u8>,
    records: u64,
}

impl RequestBuilder {
    fn request(base: &str, idempotency: &str, intent: &str) -> Result<Self, DevError> {
        let mut builder = Self::default();
        builder.push(
            "request",
            &[
                ("base", base),
                ("idempotency", idempotency),
                ("intent", intent),
            ],
        )?;
        Ok(builder)
    }

    fn push(&mut self, operation: &str, fields: &[(&str, &str)]) -> Result<(), DevError> {
        let rendered = render_record(operation, fields).map_err(diagnostic_error)?;
        let required = self
            .bytes
            .len()
            .checked_add(rendered.len())
            .ok_or_else(|| DevError::infrastructure("scale request byte count overflow"))?;
        if required > MAXIMUM_COMPACT_INPUT_BYTES {
            return Err(DevError::usage(format!(
                "scale batch requires {required} compact bytes, exceeding the current {MAXIMUM_COMPACT_INPUT_BYTES}-byte input bound"
            )));
        }
        self.bytes.extend_from_slice(rendered.as_bytes());
        self.records = self
            .records
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("scale request record count overflow"))?;
        Ok(())
    }

    fn finish(self) -> Result<RequestDocument, DevError> {
        let parsed = parse_records("<scale-request>", &self.bytes).map_err(|diagnostics| {
            let code = diagnostics
                .first()
                .map_or("unknown", |diagnostic| diagnostic.code.as_str());
            DevError::infrastructure(format!(
                "generated scale request failed compact validation: {code}"
            ))
        })?;
        if parsed.len() as u64 != self.records {
            return Err(DevError::infrastructure(
                "generated scale request record count changed during validation",
            ));
        }
        Ok(RequestDocument {
            bytes: self.bytes,
            records: self.records,
        })
    }
}

struct ApplyOutcome {
    allocated: BTreeMap<String, String>,
    base_revision: String,
    result_revision: String,
}

struct ChangeContext<'a> {
    repository: &'a Path,
    evidence_root: &'a Path,
    project: &'a Path,
    binary: &'a Path,
    runner: &'a mut Runner,
    logical: &'a mut LogicalEvidence,
    revision: String,
    request_ordinal: u64,
}

impl ChangeContext<'_> {
    fn apply(
        &mut self,
        kind: &str,
        start: u64,
        end: u64,
        logical_items: u64,
        document: RequestDocument,
        construction: bool,
    ) -> Result<ApplyOutcome, DevError> {
        self.runner.admit_resources()?;
        let requests = self.evidence_root.join("requests");
        fs::create_dir_all(&requests).map_err(|error| {
            DevError::infrastructure(format!(
                "create scale request directory '{}': {error}",
                requests.display()
            ))
        })?;
        let request_path = requests.join(format!(
            "{:06}-{}-{start}-{end}.lkjc",
            self.request_ordinal, kind
        ));
        self.request_ordinal = self
            .request_ordinal
            .checked_add(1)
            .ok_or_else(|| DevError::infrastructure("scale request ordinal overflow"))?;
        let published = evidence::publish(&request_path, &document.bytes)?;
        let plan = self.runner.invoke(
            &format!("{kind}-plan"),
            self.binary,
            project_arguments(
                self.project,
                &[
                    "change",
                    "plan",
                    "--input-file",
                    &request_path.to_string_lossy(),
                ],
            ),
            "change.plan",
            "prepared",
        )?;
        let plan_revision = record(&plan.records, "revision")?;
        let plan_base = required_field(plan_revision, "base")?.to_owned();
        let plan_result = required_field(plan_revision, "result")?.to_owned();
        if plan_base != self.revision {
            return Err(DevError::corrupt(format!(
                "reviewed batch base {plan_base} differs from current {}",
                self.revision
            )));
        }
        let token = required_field(record(&plan.records, "plan")?, "token")?.to_owned();
        let planned_identities = identities(&plan.records)?;
        let compiler_units = u64_field(record(&plan.records, "validation")?, "compiler-units")?;
        let apply = self.runner.invoke(
            &format!("{kind}-apply"),
            self.binary,
            project_arguments(
                self.project,
                &[
                    "change",
                    "apply",
                    "--input-file",
                    &request_path.to_string_lossy(),
                    "--plan",
                    &token,
                ],
            ),
            "change.apply",
            "accepted",
        )?;
        let applied_revision = record(&apply.records, "revision")?;
        let apply_base = required_field(applied_revision, "base")?;
        let apply_result = required_field(applied_revision, "result")?;
        if apply_base != plan_base || apply_result != plan_result {
            return Err(DevError::corrupt(
                "reviewed plan and accepted apply revisions disagree",
            ));
        }
        let applied_identities = identities(&apply.records)?;
        if planned_identities != applied_identities {
            return Err(DevError::corrupt(
                "reviewed plan and accepted apply allocations disagree",
            ));
        }
        let identity_bytes = serde_json::to_vec(&applied_identities).map_err(|error| {
            DevError::infrastructure(format!("encode scale allocation evidence: {error}"))
        })?;
        self.logical.plan_batches = self.logical.plan_batches.saturating_add(1);
        self.logical.apply_batches = self.logical.apply_batches.saturating_add(1);
        if construction {
            self.logical.construction_batches = self.logical.construction_batches.saturating_add(1);
        }
        self.logical.accepted_revisions.push(plan_result.clone());
        self.logical.batches.push(BatchEvidence {
            kind: kind.to_owned(),
            start,
            end,
            logical_items,
            request_records: document.records,
            request_bytes: published.bytes,
            request_digest: published.digest,
            request_path: evidence::relative(self.repository, &request_path),
            plan_command: plan.ordinal,
            apply_command: apply.ordinal,
            base_revision: plan_base.clone(),
            result_revision: plan_result.clone(),
            plan_token: token,
            allocated_identities: applied_identities.len() as u64,
            identity_digest: VerificationDigest::of(&identity_bytes),
            compiler_units,
        });
        self.revision.clone_from(&plan_result);
        Ok(ApplyOutcome {
            allocated: applied_identities,
            base_revision: plan_base,
            result_revision: plan_result,
        })
    }
}

struct GeneratedSelectors {
    first_module: String,
    first_module_name: String,
    context_owner: String,
}

enum RunCompletion {
    Completed,
    NotRun(String),
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let arguments = arguments.collect::<Vec<_>>();
    if arguments.len() == 1
        && arguments[0]
            .to_str()
            .is_some_and(|value| matches!(value, "help" | "--help" | "-h"))
    {
        println!("{HELP}");
        return Ok(0);
    }
    let options = match parse_options(arguments.clone().into_iter()) {
        Ok(options) => options,
        Err(error) => return publish_invalid_options_receipt(&arguments, error),
    };
    let repository = repository_root()?;
    let evidence_root = prepare_evidence_root(&repository, options.evidence_root.as_deref())?;
    let project = options
        .retain
        .clone()
        .unwrap_or_else(|| evidence_root.join("project"));
    validate_project_destination(&project)?;
    let scenario = scenario(&options)?;
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let host = observe_host(&repository);
    let mut runner = Runner::new(
        &repository,
        &evidence_root,
        started,
        Duration::from_secs(options.maximum_wall_seconds),
        options.maximum_run_bytes,
    );
    let mut source = None;
    let mut toolchain = None;
    let mut candidate = None;
    let mut capabilities = None;
    let mut logical = LogicalEvidence::default();
    let mut limitations = vec![
        "wall time, CPU, RSS, filesystem, and byte totals are observations of this exact host and input, not service-level objectives".to_owned(),
        "child CPU and peak RSS come from process sampling; in-process request generation and typed-oracle CPU are not separately available".to_owned(),
        "the typed oracle reconstructs the accepted GraphRepository revision and does not call the production compact-result formatter".to_owned(),
    ];
    if options.lifecycle == Lifecycle::Capacity {
        limitations.push(
            "capacity lifecycle omits rename, check, and build; it is not full compiler/artifact admission"
                .to_owned(),
        );
    }
    let mut not_run_reason = None;
    let mut run_result = (|| -> Result<RunCompletion, DevError> {
        source = Some(source_identity(&repository)?);
        toolchain = Some(toolchain_identity(&repository)?);
        let copied = copy_candidate(&repository, &evidence_root, &options.binary)?;
        let binary = PathBuf::from(&copied.executed_path);
        candidate = Some(copied);
        let capability_invocation = runner.invoke(
            "capabilities",
            &binary,
            vec!["capabilities".to_owned()],
            "capabilities",
            "success",
        )?;
        let operation_invocation = runner.invoke(
            "capabilities-operations",
            &binary,
            vec![
                "capabilities".to_owned(),
                "--section".to_owned(),
                "operations".to_owned(),
            ],
            "capabilities.section",
            "success",
        )?;
        let discovered = capability_identity(&capability_invocation, &operation_invocation)?;
        require_current_operations(&discovered)?;
        capabilities = Some(discovered);
        if let Some(reason) = preflight_reason(&options, &host) {
            if options.lifecycle == Lifecycle::Capacity {
                not_run_reason = Some(reason.clone());
                return Ok(RunCompletion::NotRun(reason));
            }
            return Err(DevError::unavailable(reason));
        }
        run_scale(
            &options,
            &repository,
            &evidence_root,
            &project,
            &binary,
            &mut runner,
            &mut logical,
        )?;
        Ok(RunCompletion::Completed)
    })();

    if !logical.complete && project.join("HEAD").is_file() {
        let _ = complete_partial_oracle(&project, &scenario.requested, &mut logical);
    }
    let repository_area = repository_area(&project).ok();
    let derived_area = area_if_present(&project.join("derived")).ok().flatten();
    let artifact_area = area_if_present(&evidence_root.join("artifacts"))
        .ok()
        .flatten();
    let total_run_bytes = directory_bytes(&evidence_root).ok();
    let cleanup = cleanup_project(&project, options.retain.is_some());
    if matches!(run_result, Ok(RunCompletion::Completed))
        && elapsed_reaches_wall_limit(
            started.elapsed(),
            Duration::from_secs(options.maximum_wall_seconds),
        )
    {
        run_result = Err(DevError::unavailable(
            "scale maximum wall time was reached during terminal oracle or cleanup work",
        ));
    }

    let (mut status, mut admission, mut failure) = match run_result {
        Ok(RunCompletion::Completed) => (
            ReceiptStatus::Passed,
            AdmissionClassification::Completed,
            None,
        ),
        Ok(RunCompletion::NotRun(reason)) => (
            ReceiptStatus::Passed,
            AdmissionClassification::NotRunWithReason,
            Some(FailureEvidence {
                class: "environment".to_owned(),
                message: reason,
            }),
        ),
        Err(error) if error.kind() == "unavailable" => (
            if options.lifecycle == Lifecycle::Capacity {
                ReceiptStatus::Passed
            } else {
                ReceiptStatus::Failed
            },
            AdmissionClassification::EnvironmentLimit,
            Some(FailureEvidence {
                class: error.kind().to_owned(),
                message: error.message().to_owned(),
            }),
        ),
        Err(error) => (
            ReceiptStatus::Failed,
            AdmissionClassification::Failed,
            Some(FailureEvidence {
                class: error.kind().to_owned(),
                message: error.message().to_owned(),
            }),
        ),
    };
    if cleanup.status == CleanupStatus::Failed {
        status = ReceiptStatus::Failed;
        admission = AdmissionClassification::Failed;
        failure = Some(FailureEvidence {
            class: "infrastructure".to_owned(),
            message: format!("scale project cleanup failed: {}", cleanup.detail),
        });
    }
    if admission != AdmissionClassification::Completed {
        logical.complete = false;
    }
    if admission == AdmissionClassification::NotRunWithReason && not_run_reason.is_none() {
        status = ReceiptStatus::Failed;
        admission = AdmissionClassification::Failed;
        failure = Some(FailureEvidence {
            class: "corrupt".to_owned(),
            message: "not-run classification omitted its exact reason".to_owned(),
        });
    }
    let commands = runner.into_commands();
    let child_cpu_nanoseconds =
        sum_optional(commands.iter().map(|item| item.process.cpu_nanoseconds));
    let maximum_child_peak_rss_kib = commands
        .iter()
        .filter_map(|item| item.process.peak_rss_kib)
        .max();
    let receipt = ScaleReceipt {
        schema: SCALE_SCHEMA.to_owned(),
        contract_version: SCALE_CONTRACT_VERSION,
        status,
        admission,
        source,
        toolchain,
        candidate,
        capabilities,
        scenario,
        logical,
        commands,
        observations: ObservationEvidence {
            started_unix_nanoseconds: started_wall,
            completed_unix_nanoseconds: unix_nanoseconds()?,
            elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
            host,
            child_cpu_nanoseconds,
            maximum_child_peak_rss_kib,
            harness_peak_rss_kib: process_peak_rss_kib(),
            repository: repository_area,
            derived: derived_area,
            artifacts: artifact_area,
            total_run_bytes,
        },
        cleanup,
        limitations,
        failure,
    };
    let published = evidence::publish_json(&evidence_root.join("receipt.json"), &receipt)?;
    print_summary(
        &repository,
        options.machine,
        options.topology.name(),
        options.lifecycle.name(),
        &receipt,
        &published,
    )?;
    Ok(if receipt.status == ReceiptStatus::Passed {
        0
    } else {
        1
    })
}

fn elapsed_reaches_wall_limit(elapsed: Duration, maximum: Duration) -> bool {
    elapsed >= maximum
}

fn publish_invalid_options_receipt(
    arguments: &[OsString],
    error: DevError,
) -> Result<u8, DevError> {
    let repository = repository_root()?;
    let configured_root = argument_value(arguments, "--evidence-root").map(PathBuf::from);
    let (evidence_root, used_fallback_root) =
        match prepare_evidence_root(&repository, configured_root.as_deref()) {
            Ok(path) => (path, false),
            Err(_) if configured_root.is_some() => {
                (prepare_evidence_root(&repository, None)?, true)
            }
            Err(root_error) => return Err(root_error),
        };
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let topology = arguments.first().map_or_else(
        || "missing".to_owned(),
        |value| value.to_string_lossy().into_owned(),
    );
    let lifecycle = match argument_value(arguments, "--lifecycle") {
        Some("capacity") => Lifecycle::Capacity,
        _ => Lifecycle::Full,
    };
    let items = argument_u64(arguments, "--items").unwrap_or(DEFAULT_ITEMS);
    let batch = argument_u64(arguments, "--batch").unwrap_or(DEFAULT_BATCH);
    let modules = argument_u64(arguments, "--modules");
    let project = argument_value(arguments, "--retain")
        .map(PathBuf::from)
        .unwrap_or_else(|| evidence_root.join("project"));
    let mut limitations = vec![
        "option validation failed before candidate execution; no project authority was created"
            .to_owned(),
        "invalid UTF-8 option bytes, when present, are represented lossily in the failed receipt"
            .to_owned(),
    ];
    if used_fallback_root {
        limitations.push(
            "the configured evidence root was invalid, so the failed receipt used an automatically allocated artifact root"
                .to_owned(),
        );
    }
    let receipt = ScaleReceipt {
        schema: SCALE_SCHEMA.to_owned(),
        contract_version: SCALE_CONTRACT_VERSION,
        status: ReceiptStatus::Failed,
        admission: AdmissionClassification::Failed,
        source: source_identity(&repository).ok(),
        toolchain: toolchain_identity(&repository).ok(),
        candidate: None,
        capabilities: None,
        scenario: ScenarioEvidence {
            topology: topology.clone(),
            semantic_shape: "invalid_options".to_owned(),
            lifecycle,
            requested_items: items,
            batch_size: batch,
            requested_modules: modules,
            requested: SemanticCounts::default(),
            maximum_wall_seconds: argument_u64(arguments, "--maximum-wall-seconds")
                .unwrap_or(DEFAULT_MAXIMUM_WALL_SECONDS),
            maximum_run_bytes: argument_u64(arguments, "--maximum-run-bytes")
                .unwrap_or(DEFAULT_MAXIMUM_RUN_BYTES),
            minimum_available_memory_bytes: argument_u64(
                arguments,
                "--minimum-available-memory-bytes",
            )
            .unwrap_or(0),
            minimum_available_disk_bytes: argument_u64(arguments, "--minimum-available-disk-bytes")
                .unwrap_or(0),
        },
        logical: LogicalEvidence::default(),
        commands: Vec::new(),
        observations: ObservationEvidence {
            started_unix_nanoseconds: started_wall,
            completed_unix_nanoseconds: unix_nanoseconds()?,
            elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
            host: observe_host(&repository),
            child_cpu_nanoseconds: None,
            maximum_child_peak_rss_kib: None,
            harness_peak_rss_kib: process_peak_rss_kib(),
            repository: None,
            derived: None,
            artifacts: None,
            total_run_bytes: directory_bytes(&evidence_root).ok(),
        },
        cleanup: CleanupEvidence {
            status: CleanupStatus::NotCreated,
            project_path: project.to_string_lossy().into_owned(),
            detail: "option validation stopped before project creation".to_owned(),
        },
        limitations,
        failure: Some(FailureEvidence {
            class: error.kind().to_owned(),
            message: error.message().to_owned(),
        }),
    };
    let published = evidence::publish_json(&evidence_root.join("receipt.json"), &receipt)?;
    print_summary(
        &repository,
        arguments
            .iter()
            .any(|argument| argument.to_str() == Some("--machine")),
        &topology,
        lifecycle.name(),
        &receipt,
        &published,
    )?;
    Ok(1)
}

fn argument_value<'a>(arguments: &'a [OsString], option: &str) -> Option<&'a str> {
    arguments.windows(2).find_map(|pair| {
        (pair[0].to_str() == Some(option))
            .then(|| pair[1].to_str())
            .flatten()
    })
}

fn argument_u64(arguments: &[OsString], option: &str) -> Option<u64> {
    argument_value(arguments, option)?.parse().ok()
}

fn run_scale(
    options: &Options,
    repository: &Path,
    evidence_root: &Path,
    project: &Path,
    binary: &Path,
    runner: &mut Runner,
    logical: &mut LogicalEvidence,
) -> Result<(), DevError> {
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
        "new",
        "success",
    )?;
    let created_revision = required_field(record(&created.records, "revision")?, "id")?.to_owned();
    logical.starting_revision = Some(created_revision.clone());
    logical.repository =
        Some(required_field(record(&created.records, "repository")?, "id")?.to_owned());
    logical.package = Some(required_field(record(&created.records, "package")?, "id")?.to_owned());
    logical.semantic_state =
        Some(required_field(record(&created.records, "state")?, "digest")?.to_owned());

    let mut changes = ChangeContext {
        repository,
        evidence_root,
        project,
        binary,
        runner,
        logical,
        revision: created_revision,
        request_ordinal: 0,
    };
    let mut selectors = construct_topology(options, &mut changes)?;
    if options.lifecycle == Lifecycle::Full {
        let before = selectors.first_module_name.clone();
        let after = format!("{before}_renamed");
        let mut request = RequestBuilder::request(
            &changes.revision,
            "scale-rename",
            "scale local rename admission",
        )?;
        request.push(
            "rename.owner",
            &[("owner", &selectors.first_module), ("name", &after)],
        )?;
        let outcome = changes.apply("rename", 0, 1, 1, request.finish()?, false)?;
        changes.logical.rename = Some(RenameEvidence {
            owner: selectors.first_module.clone(),
            before,
            after: after.clone(),
            base_revision: outcome.base_revision,
            result_revision: outcome.result_revision,
        });
        selectors.first_module_name = after;
    }
    let final_revision = changes.revision.clone();
    changes.logical.final_revision = Some(final_revision.clone());
    changes.logical.public_reads = Some(public_reads(
        changes.project,
        changes.binary,
        changes.runner,
        &selectors,
        &final_revision,
        &scenario(options)?.requested,
    )?);

    if options.lifecycle == Lifecycle::Full {
        changes.logical.check = Some(run_check(
            changes.project,
            changes.binary,
            changes.runner,
            &final_revision,
        )?);
        changes.logical.cache_reset_before_clean_build =
            Some(reset_compiler_cache(changes.project)?);
        let artifacts = changes.evidence_root.join("artifacts");
        fs::create_dir_all(&artifacts).map_err(|error| {
            DevError::infrastructure(format!(
                "create scale artifact directory '{}': {error}",
                artifacts.display()
            ))
        })?;
        let clean = run_build(
            "clean",
            &artifacts.join("clean.lkja"),
            changes.project,
            changes.binary,
            changes.runner,
            repository,
            &final_revision,
        )?;
        if clean.compilation.cache != "clean" {
            return Err(DevError::corrupt(
                "forced clean scale build did not report a clean compilation",
            ));
        }
        let exact = run_build(
            "exact-current",
            &artifacts.join("exact-current.lkja"),
            changes.project,
            changes.binary,
            changes.runner,
            repository,
            &final_revision,
        )?;
        if exact.compilation.cache != "exact-current" {
            return Err(DevError::corrupt(
                "second scale build did not report exact-current compilation",
            ));
        }
        let equal = clean.artifact == exact.artifact
            && clean.sha256 == exact.sha256
            && files_equal(
                &artifacts.join("clean.lkja"),
                &artifacts.join("exact-current.lkja"),
            )?;
        if !equal {
            return Err(DevError::corrupt(
                "clean and exact-current scale artifacts disagree",
            ));
        }
        changes.logical.clean_exact_artifacts_equal = Some(true);
        changes.logical.builds = vec![clean, exact];
    }
    attach_oracle(
        changes.project,
        &scenario(options)?.requested,
        changes.logical,
    )?;
    changes.logical.shape_digest = Some(shape_digest(options, changes.logical)?);
    changes.runner.admit_resources()?;
    changes.logical.complete = true;
    Ok(())
}

fn construct_topology(
    options: &Options,
    changes: &mut ChangeContext<'_>,
) -> Result<GeneratedSelectors, DevError> {
    match options.topology {
        Topology::IndependentModules => construct_independent_modules(options, changes),
        Topology::SmallFunctions => construct_small_functions(options, changes),
        Topology::WideModule => construct_function_graph(options, changes, FunctionShape::Unit),
        Topology::DeepChain => construct_function_graph(options, changes, FunctionShape::Chain),
        Topology::WideFanout => construct_function_graph(options, changes, FunctionShape::Fanout),
    }
}

fn construct_independent_modules(
    options: &Options,
    changes: &mut ChangeContext<'_>,
) -> Result<GeneratedSelectors, DevError> {
    let mut first_module = None;
    for (start, end) in batches(options.items, options.batch) {
        let document = module_request(&changes.revision, start, end, "independent")?;
        let outcome = changes.apply(
            "independent-modules",
            start,
            end,
            end - start,
            document,
            true,
        )?;
        if start == 0 {
            first_module = outcome.allocated.get("$m0000000").cloned();
        }
    }
    let first_module = first_module.ok_or_else(|| {
        DevError::corrupt("independent-module construction omitted its first allocation")
    })?;
    Ok(GeneratedSelectors {
        first_module: first_module.clone(),
        first_module_name: module_name(0),
        context_owner: first_module,
    })
}

fn construct_small_functions(
    options: &Options,
    changes: &mut ChangeContext<'_>,
) -> Result<GeneratedSelectors, DevError> {
    let modules = options.modules.ok_or_else(|| {
        DevError::infrastructure("small-functions module count was not normalized")
    })?;
    let mut module_ids = Vec::with_capacity(modules as usize);
    for (start, end) in batches(
        modules,
        Topology::IndependentModules
            .maximum_batch()
            .min(options.batch),
    ) {
        let document = module_request(&changes.revision, start, end, "small-functions-modules")?;
        let outcome = changes.apply(
            "small-functions-modules",
            start,
            end,
            end - start,
            document,
            true,
        )?;
        for index in start..end {
            let symbol = module_symbol(index);
            module_ids.push(outcome.allocated.get(&symbol).cloned().ok_or_else(|| {
                DevError::corrupt(format!("module batch omitted allocation {symbol}"))
            })?);
        }
    }
    let mut first_function = None;
    for (start, end) in batches(options.items, options.batch) {
        let document = function_request(
            &changes.revision,
            start,
            end,
            &module_ids,
            FunctionShape::Unit,
            None,
        )?;
        let outcome = changes.apply("small-functions", start, end, end - start, document, true)?;
        if start == 0 {
            first_function = outcome.allocated.get("$f0000000").cloned();
        }
    }
    Ok(GeneratedSelectors {
        first_module: module_ids[0].clone(),
        first_module_name: module_name(0),
        context_owner: first_function.ok_or_else(|| {
            DevError::corrupt("small-function construction omitted its first function")
        })?,
    })
}

#[derive(Clone, Copy)]
enum FunctionShape {
    Unit,
    Chain,
    Fanout,
}

fn construct_function_graph(
    options: &Options,
    changes: &mut ChangeContext<'_>,
    shape: FunctionShape,
) -> Result<GeneratedSelectors, DevError> {
    let module = changes.apply(
        "function-module",
        0,
        1,
        1,
        module_request(&changes.revision, 0, 1, "function-module")?,
        true,
    )?;
    let module = module
        .allocated
        .get("$m0000000")
        .cloned()
        .ok_or_else(|| DevError::corrupt("function topology omitted its module allocation"))?;
    let module_ids = [module.clone()];
    let mut first_function = None;
    let mut previous_function = None;
    for (start, end) in batches(options.items, options.batch) {
        let external_target = match shape {
            FunctionShape::Unit => None,
            FunctionShape::Chain => previous_function.as_deref(),
            FunctionShape::Fanout => first_function.as_deref(),
        };
        let document = function_request(
            &changes.revision,
            start,
            end,
            &module_ids,
            shape,
            external_target,
        )?;
        let outcome = changes.apply(
            options.topology.name(),
            start,
            end,
            end - start,
            document,
            true,
        )?;
        if start == 0 {
            first_function = outcome.allocated.get("$f0000000").cloned();
        }
        previous_function = outcome.allocated.get(&function_symbol(end - 1)).cloned();
    }
    Ok(GeneratedSelectors {
        first_module: module,
        first_module_name: module_name(0),
        context_owner: first_function.ok_or_else(|| {
            DevError::corrupt("function topology omitted its first function allocation")
        })?,
    })
}

fn module_request(
    revision: &str,
    start: u64,
    end: u64,
    intent: &str,
) -> Result<RequestDocument, DevError> {
    let mut request = RequestBuilder::request(
        revision,
        &format!("scale-{intent}-{start}-{end}"),
        &format!("scale {intent} {start} through {end}"),
    )?;
    for index in start..end {
        let symbol = module_symbol(index);
        let name = module_name(index);
        request.push("create.module", &[("as", &symbol), ("name", &name)])?;
    }
    request.finish()
}

fn function_request(
    revision: &str,
    start: u64,
    end: u64,
    modules: &[String],
    shape: FunctionShape,
    external_target: Option<&str>,
) -> Result<RequestDocument, DevError> {
    let mut request = RequestBuilder::request(
        revision,
        &format!("scale-functions-{start}-{end}"),
        &format!("scale functions {start} through {end}"),
    )?;
    for index in start..end {
        let body = body_symbol(index);
        let function = function_symbol(index);
        let module = &modules[index as usize % modules.len()];
        match shape {
            FunctionShape::Unit => request.push("expression.unit", &[("as", &body)])?,
            FunctionShape::Chain if index == 0 => {
                request.push("expression.unit", &[("as", &body)])?;
            }
            FunctionShape::Chain => {
                let target = if index == start {
                    external_target.ok_or_else(|| {
                        DevError::corrupt("deep-chain batch omitted its previous function")
                    })?
                } else {
                    // Local symbols are request-wide and may be referenced before the owner is
                    // lowered; allocation is canonicalized before semantic validation.
                    &function_symbol(index - 1)
                };
                request.push("expression.call", &[("as", &body), ("function", target)])?;
            }
            FunctionShape::Fanout if index == 0 => {
                request.push("expression.unit", &[("as", &body)])?;
            }
            FunctionShape::Fanout => {
                let target = external_target.unwrap_or("$f0000000");
                request.push("expression.call", &[("as", &body), ("function", target)])?;
            }
        }
        let name = function_name(index);
        request.push(
            "create.function",
            &[
                ("as", &function),
                ("module", module),
                ("name", &name),
                ("visibility", "private"),
                ("result", "unit"),
                ("effect", "pure"),
                ("body", &body),
            ],
        )?;
    }
    request.finish()
}

fn module_symbol(index: u64) -> String {
    format!("$m{index:07}")
}

fn module_name(index: u64) -> String {
    format!("m{index:07}")
}

fn function_symbol(index: u64) -> String {
    format!("$f{index:07}")
}

fn body_symbol(index: u64) -> String {
    format!("$b{index:07}")
}

fn function_name(index: u64) -> String {
    format!("f{index:07}")
}

fn batches(total: u64, batch: u64) -> impl Iterator<Item = (u64, u64)> {
    (0..total).step_by(batch as usize).map(move |start| {
        let end = start.saturating_add(batch).min(total);
        (start, end)
    })
}

fn public_reads(
    project: &Path,
    binary: &Path,
    runner: &mut Runner,
    selectors: &GeneratedSelectors,
    revision: &str,
    requested: &SemanticCounts,
) -> Result<PublicReadEvidence, DevError> {
    let status = runner.invoke(
        "status",
        binary,
        project_arguments(project, &["status"]),
        "status",
        "success",
    )?;
    let status_revision = required_field(record(&status.records, "revision")?, "id")?;
    let status_owners = u64_field(record(&status.records, "summary")?, "owners")?;
    require_revision("status", revision, status_revision)?;
    if status_owners != requested.owners {
        return Err(DevError::corrupt(format!(
            "status reports {status_owners} owners; expected {}",
            requested.owners
        )));
    }

    let inspected = runner.invoke(
        "inspect-owner",
        binary,
        project_arguments(
            project,
            &["inspect", "owner", "module", &selectors.first_module],
        ),
        "inspect.owner",
        "success",
    )?;
    require_observed_revision("inspect", revision, &inspected.records)?;
    let inspected_owner = record(&inspected.records, "owner")?;
    let inspected_id = required_field(inspected_owner, "id")?.to_owned();
    let inspected_kind = required_field(inspected_owner, "kind")?.to_owned();
    if inspected_id != selectors.first_module || inspected_kind != "module" {
        return Err(DevError::corrupt(
            "exact owner inspection returned a different owner",
        ));
    }

    let owners = runner.invoke(
        "query-owners",
        binary,
        project_arguments(
            project,
            &[
                "query", "owners", "--limit", READ_LIMIT, "--bytes", READ_BYTES,
            ],
        ),
        "query.owners",
        "success",
    )?;
    require_observed_revision("owner query", revision, &owners.records)?;
    let owner_summary = record(&owners.records, "summary")?;
    let owner_query_returned = u64_field(owner_summary, "returned")?;
    let owner_query_visited = u64_field(owner_summary, "visited")?;
    let owner_query_truncated = bool_field(owner_summary, "truncated")?;
    if owner_query_returned == 0 || owner_query_returned > READ_LIMIT.parse().unwrap_or(16) {
        return Err(DevError::corrupt(
            "bounded owner query returned an invalid item count",
        ));
    }

    let found = runner.invoke(
        "query-find-module",
        binary,
        project_arguments(
            project,
            &["query", "find", "module", &selectors.first_module_name],
        ),
        "query.find",
        "success",
    )?;
    require_observed_revision("name query", revision, &found.records)?;
    let found_owner = required_field(record(&found.records, "owner")?, "id")?.to_owned();
    if found_owner != selectors.first_module {
        return Err(DevError::corrupt(
            "exact public name query returned a different module",
        ));
    }

    let context = runner.invoke(
        "query-context",
        binary,
        project_arguments(
            project,
            &[
                "query",
                "context",
                &selectors.context_owner,
                "--direction",
                "both",
                "--depth",
                "1",
                "--limit",
                READ_LIMIT,
                "--bytes",
                READ_BYTES,
            ],
        ),
        "query.context",
        "success",
    )?;
    require_observed_revision("context query", revision, &context.records)?;
    let context_summary = record(&context.records, "summary")?;
    let context_returned = u64_field(context_summary, "returned")?;
    if context_returned == 0 {
        return Err(DevError::corrupt(
            "bounded context query returned no observation",
        ));
    }
    Ok(PublicReadEvidence {
        revision: revision.to_owned(),
        status_owners,
        inspected_owner: inspected_id,
        inspected_kind,
        owner_query_returned,
        owner_query_visited,
        owner_query_truncated,
        name_query_owner: found_owner,
        context_owner: selectors.context_owner.clone(),
        context_returned,
        context_total_owners: u64_field(context_summary, "total-owners")?,
        context_total_relations: u64_field(context_summary, "total-relations")?,
        context_truncated: bool_field(context_summary, "truncated")?,
    })
}

fn run_check(
    project: &Path,
    binary: &Path,
    runner: &mut Runner,
    revision: &str,
) -> Result<CheckEvidence, DevError> {
    let checked = runner.invoke(
        "check",
        binary,
        project_arguments(project, &["check"]),
        "check",
        "success",
    )?;
    let authority = record(&checked.records, "authority")?;
    require_revision("check", revision, required_field(authority, "revision")?)?;
    let tests = record(&checked.records, "tests")?;
    let tests_failed = u64_field(tests, "failed")?;
    if tests_failed != 0 || required_field(tests, "differential")? != "equal" {
        return Err(DevError::corrupt("scale check did not pass exactly"));
    }
    Ok(CheckEvidence {
        command: checked.ordinal,
        revision: revision.to_owned(),
        compilation: compilation_evidence(record(&checked.records, "compilation")?)?,
        artifact: artifact_evidence(record(&checked.records, "artifact")?)?,
        tests_passed: u64_field(tests, "passed")?,
        tests_failed,
        differential: required_field(tests, "differential")?.to_owned(),
    })
}

fn run_build(
    mode: &str,
    output: &Path,
    project: &Path,
    binary: &Path,
    runner: &mut Runner,
    repository: &Path,
    revision: &str,
) -> Result<BuildEvidence, DevError> {
    if fs::symlink_metadata(output).is_ok() {
        return Err(DevError::infrastructure(format!(
            "scale build output '{}' already exists",
            output.display()
        )));
    }
    let built = runner.invoke(
        &format!("build-{mode}"),
        binary,
        project_arguments(project, &["build", "--output", &output.to_string_lossy()]),
        "build",
        "success",
    )?;
    require_revision(
        "build",
        revision,
        required_field(record(&built.records, "authority")?, "revision")?,
    )?;
    let output_record = record(&built.records, "output")?;
    let output_bytes = u64_field(output_record, "bytes")?;
    let proof = evidence::proof(output, evidence::relative(repository, output))?;
    if proof.bytes != Some(output_bytes) {
        return Err(DevError::corrupt(
            "build response and artifact file byte counts disagree",
        ));
    }
    Ok(BuildEvidence {
        mode: mode.to_owned(),
        command: built.ordinal,
        revision: revision.to_owned(),
        compilation: compilation_evidence(record(&built.records, "compilation")?)?,
        artifact: artifact_evidence(record(&built.records, "artifact")?)?,
        output: proof,
        sha256: sha256_file(output)?,
    })
}

fn compilation_evidence(
    value: &lkjscript::platform::control::CompactRecord,
) -> Result<CompilationEvidence, DevError> {
    Ok(CompilationEvidence {
        cache: required_field(value, "cache")?.to_owned(),
        manifest: required_field(value, "manifest")?.to_owned(),
        compiled: u64_field(value, "compiled")?,
        reused: u64_field(value, "reused")?,
        removed: u64_field(value, "removed")?,
    })
}

fn artifact_evidence(
    value: &lkjscript::platform::control::CompactRecord,
) -> Result<ArtifactEvidence, DevError> {
    Ok(ArtifactEvidence {
        manifest: required_field(value, "manifest")?.to_owned(),
        bundle: required_field(value, "bundle")?.to_owned(),
        bytes: u64_field(value, "bytes")?,
        packages: u64_field(value, "packages")?,
        closure_objects: u64_field(value, "closure-objects")?,
        compiler_units: u64_field(value, "compiler-units")?,
    })
}

fn reset_compiler_cache(project: &Path) -> Result<bool, DevError> {
    let cache = project.join("derived/compiler");
    let metadata = match fs::symlink_metadata(&cache) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "inspect compiler cache '{}': {error}",
                cache.display()
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::infrastructure(format!(
            "compiler cache '{}' is not a regular directory",
            cache.display()
        )));
    }
    fs::remove_dir_all(&cache).map_err(|error| {
        DevError::infrastructure(format!(
            "remove compiler cache '{}': {error}",
            cache.display()
        ))
    })?;
    Ok(true)
}

fn attach_oracle(
    project: &Path,
    requested: &SemanticCounts,
    logical: &mut LogicalEvidence,
) -> Result<(), DevError> {
    let inventory = semantic_inventory(project).map_err(diagnostic_error)?;
    let catalog = catalog_inventory(project).map_err(diagnostic_error)?;
    let observed = SemanticCounts {
        owners: inventory.owners,
        modules: inventory.modules,
        functions: inventory.functions,
        relations: inventory.relations,
    };
    let revision_equal = logical.final_revision.as_deref() == Some(inventory.revision.as_str());
    let public_equal = logical.public_reads.as_ref().is_some_and(|public| {
        public.revision == inventory.revision && public.status_owners == inventory.owners
    });
    let requested_equal = &observed == requested;
    logical.oracle = Some(oracle_evidence(inventory));
    let catalog_equal = catalog.identity == "lkjscript-object-catalog-2"
        && catalog.contract_version == 2
        && catalog.state == "loaded"
        && catalog.segments <= catalog.maximum_live_segments
        && catalog.maximum_lookup_segments == catalog.segments
        && catalog.history.full_rebuilds == 0
        && catalog.history.full_footer_scan_runs == 0
        && catalog.history.pack_footers_scanned == 0
        && catalog.work.full_rebuilds == 0
        && catalog.work.full_footer_scan_runs == 0
        && catalog.work.pack_footers_scanned == 0
        && catalog.footer_oracle_equal
        && catalog.footer_oracle_entries == catalog.entries
        && catalog.footer_oracle_packs == catalog.packs
        && catalog.footer_oracle_commitment == catalog.commitment
        && catalog.leftovers.is_empty();
    logical.catalog = Some(catalog);
    logical.observed = Some(observed);
    logical.public_oracle_equal = Some(revision_equal && public_equal && requested_equal);
    if !revision_equal || !public_equal || !requested_equal || !catalog_equal {
        return Err(DevError::corrupt(
            "public scale observations, typed semantic oracle, and independent catalog oracle disagree",
        ));
    }
    Ok(())
}

fn complete_partial_oracle(
    project: &Path,
    requested: &SemanticCounts,
    logical: &mut LogicalEvidence,
) -> Result<(), DevError> {
    let inventory = semantic_inventory(project).map_err(diagnostic_error)?;
    logical.catalog = catalog_inventory(project).ok();
    logical.final_revision = Some(inventory.revision.clone());
    logical.observed = Some(SemanticCounts {
        owners: inventory.owners,
        modules: inventory.modules,
        functions: inventory.functions,
        relations: inventory.relations,
    });
    logical.public_oracle_equal = Some(false);
    logical.oracle = Some(oracle_evidence(inventory));
    let _ = requested;
    Ok(())
}

fn oracle_evidence(inventory: SemanticInventory) -> OracleEvidence {
    OracleEvidence {
        revision: inventory.revision,
        owners: inventory.owners,
        modules: inventory.modules,
        functions: inventory.functions,
        relations: inventory.relations,
        types: inventory.types,
        dependencies: inventory.dependencies,
        retirements: inventory.retirements,
        owner_kinds: inventory.owner_kinds,
        owner_identity_digest: VerificationDigest::of(inventory.owner_identity_digest.as_bytes()),
        relation_digest: VerificationDigest::of(inventory.relation_digest.as_bytes()),
        validation_owner_records: inventory.validation_owner_records,
        validation_type_objects: inventory.validation_type_objects,
        validation_expression_records: inventory.validation_expression_records,
        validation_relation_edges: inventory.validation_relation_edges,
        validation_work: inventory.validation_work,
        map_pages_read: inventory.map_pages_read,
        map_bytes_read: inventory.map_bytes_read,
        store_objects_read: inventory.store_objects_read,
        store_bytes_read: inventory.store_bytes_read,
    }
}

fn shape_digest(
    options: &Options,
    logical: &LogicalEvidence,
) -> Result<VerificationDigest, DevError> {
    #[derive(Serialize)]
    struct CheckShape<'a> {
        cache: &'a str,
        compiled: u64,
        reused: u64,
        removed: u64,
        artifact_bytes: u64,
        packages: u64,
        closure_objects: u64,
        compiler_units: u64,
        tests_passed: u64,
        tests_failed: u64,
        differential: &'a str,
    }

    #[derive(Serialize)]
    struct Shape<'a> {
        topology: &'a str,
        items: u64,
        modules: Option<u64>,
        lifecycle: &'a str,
        construction_batches: u64,
        plan_batches: u64,
        apply_batches: u64,
        batch_shapes: Vec<(&'a str, u64, u64, u64, u64, u64)>,
        rename: bool,
        public_counts: Option<(u64, u64, u64, bool, u64, u64, bool)>,
        check: Option<CheckShape<'a>>,
        artifacts_equal: Option<bool>,
        observed: &'a Option<SemanticCounts>,
        public_oracle_equal: Option<bool>,
    }
    let shape = Shape {
        topology: options.topology.name(),
        items: options.items,
        modules: options.modules,
        lifecycle: options.lifecycle.name(),
        construction_batches: logical.construction_batches,
        plan_batches: logical.plan_batches,
        apply_batches: logical.apply_batches,
        batch_shapes: logical
            .batches
            .iter()
            .map(|batch| {
                (
                    batch.kind.as_str(),
                    batch.start,
                    batch.end,
                    batch.logical_items,
                    batch.request_records,
                    batch.compiler_units,
                )
            })
            .collect(),
        rename: logical.rename.is_some(),
        public_counts: logical.public_reads.as_ref().map(|public| {
            (
                public.status_owners,
                public.owner_query_returned,
                public.owner_query_visited,
                public.owner_query_truncated,
                public.context_total_owners,
                public.context_total_relations,
                public.context_truncated,
            )
        }),
        check: logical.check.as_ref().map(|check| CheckShape {
            cache: &check.compilation.cache,
            compiled: check.compilation.compiled,
            reused: check.compilation.reused,
            removed: check.compilation.removed,
            artifact_bytes: check.artifact.bytes,
            packages: check.artifact.packages,
            closure_objects: check.artifact.closure_objects,
            compiler_units: check.artifact.compiler_units,
            tests_passed: check.tests_passed,
            tests_failed: check.tests_failed,
            differential: &check.differential,
        }),
        artifacts_equal: logical.clean_exact_artifacts_equal,
        observed: &logical.observed,
        public_oracle_equal: logical.public_oracle_equal,
    };
    let bytes = serde_json::to_vec(&shape).map_err(|error| {
        DevError::infrastructure(format!("encode deterministic scale shape: {error}"))
    })?;
    Ok(VerificationDigest::of(&bytes))
}

fn capability_identity(
    invocation: &Invocation,
    operation_invocation: &Invocation,
) -> Result<CapabilityIdentity, DevError> {
    let product = record(&invocation.records, "product")?;
    let capabilities = record(&invocation.records, "capabilities")?;
    let mut sections = BTreeMap::new();
    for section in invocation
        .records
        .iter()
        .filter(|value| value.operation == "section")
    {
        let name = required_field(section, "name")?.to_owned();
        let previous = sections.insert(
            name.clone(),
            CapabilitySection {
                digest: required_field(section, "digest")?.to_owned(),
                records: u64_field(section, "records")?,
                bytes: u64_field(section, "bytes")?,
            },
        );
        if previous.is_some() {
            return Err(DevError::corrupt(format!(
                "capabilities repeated section '{name}'"
            )));
        }
    }
    let operations = operation_invocation
        .records
        .iter()
        .filter(|value| value.operation == "operation")
        .map(|value| -> Result<OperationIdentity, DevError> {
            Ok(OperationIdentity {
                name: required_field(value, "name")?.to_owned(),
                usage: required_field(value, "usage")?.to_owned(),
                request_model: required_field(value, "request-model")?.to_owned(),
                response_model: required_field(value, "response-model")?.to_owned(),
                authority_effect: required_field(value, "authority-effect")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let operation_product = record(&operation_invocation.records, "product")?;
    let operation_capabilities = record(&operation_invocation.records, "capabilities")?;
    if required_field(operation_product, "name")? != required_field(product, "name")?
        || required_field(operation_product, "version")? != required_field(product, "version")?
        || required_field(operation_capabilities, "digest")?
            != required_field(capabilities, "digest")?
    {
        return Err(DevError::corrupt(
            "capabilities operation contracts disagree with the discovered product identity",
        ));
    }
    if sections.is_empty() || operations.is_empty() {
        return Err(DevError::corrupt(
            "capabilities omitted its finite sections or operations",
        ));
    }
    Ok(CapabilityIdentity {
        product_name: required_field(product, "name")?.to_owned(),
        product_version: required_field(product, "version")?.to_owned(),
        digest: required_field(capabilities, "digest")?.to_owned(),
        sections,
        operations,
    })
}

fn require_current_operations(capabilities: &CapabilityIdentity) -> Result<(), DevError> {
    let operations = capabilities
        .operations
        .iter()
        .map(|operation| operation.name.as_str())
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_OPERATIONS {
        if !operations.contains(required) {
            return Err(DevError::corrupt(format!(
                "candidate capabilities omit required current operation '{required}'"
            )));
        }
    }
    for predecessor in ["doctor", "backup"] {
        if operations.contains(predecessor) {
            return Err(DevError::corrupt(format!(
                "candidate exposes removed top-level operation '{predecessor}'"
            )));
        }
    }
    Ok(())
}

fn scenario(options: &Options) -> Result<ScenarioEvidence, DevError> {
    let functions = match options.topology {
        Topology::IndependentModules => 0,
        _ => options.items,
    };
    let modules = match options.topology {
        Topology::IndependentModules => options.items,
        Topology::SmallFunctions => options.modules.ok_or_else(|| {
            DevError::infrastructure("small-functions modules were not normalized")
        })?,
        _ => 1,
    };
    let expression_owners = functions;
    let owners = modules
        .checked_add(functions)
        .and_then(|value| value.checked_add(expression_owners))
        .ok_or_else(|| DevError::usage("requested scale owner count overflows"))?;
    let base_relations = functions
        .checked_mul(2)
        .ok_or_else(|| DevError::usage("requested scale relation count overflows"))?;
    let relations = match options.topology {
        Topology::DeepChain | Topology::WideFanout => base_relations
            .checked_add(functions.saturating_sub(1))
            .ok_or_else(|| DevError::usage("requested scale relation count overflows"))?,
        _ => base_relations,
    };
    Ok(ScenarioEvidence {
        topology: options.topology.name().to_owned(),
        semantic_shape: options.topology.semantic_shape().to_owned(),
        lifecycle: options.lifecycle,
        requested_items: options.items,
        batch_size: options.batch,
        requested_modules: options.modules,
        requested: SemanticCounts {
            owners,
            modules,
            functions,
            relations,
        },
        maximum_wall_seconds: options.maximum_wall_seconds,
        maximum_run_bytes: options.maximum_run_bytes,
        minimum_available_memory_bytes: options.minimum_available_memory_bytes,
        minimum_available_disk_bytes: options.minimum_available_disk_bytes,
    })
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut arguments = arguments;
    let topology = crate::next_utf8(&mut arguments, "scale topology")?
        .ok_or_else(|| DevError::usage("scale requires a topology"))?;
    let topology = Topology::parse(&topology)?;
    let mut items = None;
    let mut batch = None;
    let mut modules = None;
    let mut lifecycle = None;
    let mut binary = None;
    let mut evidence_root = None;
    let mut retain = None;
    let mut maximum_wall_seconds = None;
    let mut maximum_run_bytes = None;
    let mut minimum_available_memory_bytes = None;
    let mut minimum_available_disk_bytes = None;
    let mut machine = false;
    while let Some(argument) = crate::next_utf8(&mut arguments, "scale option")? {
        match argument.as_str() {
            "--items" if items.is_none() => {
                items = Some(parse_u64(
                    &required_value(&mut arguments, "--items")?,
                    "--items",
                )?);
            }
            "--batch" if batch.is_none() => {
                batch = Some(parse_u64(
                    &required_value(&mut arguments, "--batch")?,
                    "--batch",
                )?);
            }
            "--modules" if modules.is_none() => {
                modules = Some(parse_u64(
                    &required_value(&mut arguments, "--modules")?,
                    "--modules",
                )?);
            }
            "--lifecycle" if lifecycle.is_none() => {
                lifecycle = Some(
                    match required_value(&mut arguments, "--lifecycle")?.as_str() {
                        "full" => Lifecycle::Full,
                        "capacity" => Lifecycle::Capacity,
                        value => {
                            return Err(DevError::usage(format!(
                                "unknown scale lifecycle '{value}'"
                            )));
                        }
                    },
                );
            }
            "--binary" if binary.is_none() => {
                binary = Some(PathBuf::from(required_value(&mut arguments, "--binary")?));
            }
            "--evidence-root" if evidence_root.is_none() => {
                evidence_root = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--evidence-root",
                )?));
            }
            "--retain" if retain.is_none() => {
                retain = Some(PathBuf::from(required_value(&mut arguments, "--retain")?));
            }
            "--maximum-wall-seconds" if maximum_wall_seconds.is_none() => {
                maximum_wall_seconds = Some(parse_u64(
                    &required_value(&mut arguments, "--maximum-wall-seconds")?,
                    "--maximum-wall-seconds",
                )?);
            }
            "--maximum-run-bytes" if maximum_run_bytes.is_none() => {
                maximum_run_bytes = Some(parse_u64(
                    &required_value(&mut arguments, "--maximum-run-bytes")?,
                    "--maximum-run-bytes",
                )?);
            }
            "--minimum-available-memory-bytes" if minimum_available_memory_bytes.is_none() => {
                minimum_available_memory_bytes = Some(parse_u64(
                    &required_value(&mut arguments, "--minimum-available-memory-bytes")?,
                    "--minimum-available-memory-bytes",
                )?);
            }
            "--minimum-available-disk-bytes" if minimum_available_disk_bytes.is_none() => {
                minimum_available_disk_bytes = Some(parse_u64(
                    &required_value(&mut arguments, "--minimum-available-disk-bytes")?,
                    "--minimum-available-disk-bytes",
                )?);
            }
            "--machine" if !machine => machine = true,
            _ => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate scale option '{argument}'"
                )));
            }
        }
    }
    let items = items.unwrap_or(DEFAULT_ITEMS);
    if !(1..=MAXIMUM_ITEMS).contains(&items) {
        return Err(DevError::usage(format!(
            "--items must be 1 through {MAXIMUM_ITEMS}"
        )));
    }
    let maximum_batch = topology.maximum_batch();
    let batch = batch.unwrap_or(DEFAULT_BATCH.min(maximum_batch));
    if batch == 0 || batch > maximum_batch {
        return Err(DevError::usage(format!(
            "--batch for {} must be 1 through {maximum_batch}",
            topology.name()
        )));
    }
    let modules = match topology {
        Topology::SmallFunctions => {
            let modules = modules.unwrap_or(items.min(100));
            if modules == 0 || modules > items {
                return Err(DevError::usage(
                    "--modules for small-functions must be 1 through --items",
                ));
            }
            Some(modules)
        }
        _ if modules.is_some() => {
            return Err(DevError::usage(
                "--modules is valid only for small-functions",
            ));
        }
        _ => None,
    };
    let maximum_wall_seconds = maximum_wall_seconds.unwrap_or(DEFAULT_MAXIMUM_WALL_SECONDS);
    if maximum_wall_seconds == 0 || maximum_wall_seconds > MAXIMUM_WALL_SECONDS {
        return Err(DevError::usage(format!(
            "--maximum-wall-seconds must be 1 through {MAXIMUM_WALL_SECONDS}"
        )));
    }
    let maximum_run_bytes = maximum_run_bytes.unwrap_or(DEFAULT_MAXIMUM_RUN_BYTES);
    if !(MINIMUM_RUN_BYTES..=MAXIMUM_RUN_BYTES).contains(&maximum_run_bytes) {
        return Err(DevError::usage(format!(
            "--maximum-run-bytes must be {MINIMUM_RUN_BYTES} through {MAXIMUM_RUN_BYTES}"
        )));
    }
    Ok(Options {
        topology,
        items,
        batch,
        modules,
        lifecycle: lifecycle.unwrap_or(Lifecycle::Full),
        binary: binary.unwrap_or_else(|| PathBuf::from("target/debug/lkjscript")),
        evidence_root,
        retain,
        maximum_wall_seconds,
        maximum_run_bytes,
        minimum_available_memory_bytes: minimum_available_memory_bytes.unwrap_or(0),
        minimum_available_disk_bytes: minimum_available_disk_bytes.unwrap_or(0),
        machine,
    })
}

fn parse_u64(value: &str, option: &str) -> Result<u64, DevError> {
    value
        .parse()
        .map_err(|_| DevError::usage(format!("{option} requires an unsigned integer")))
}

fn required_value(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, DevError> {
    crate::next_utf8(arguments, option)?
        .ok_or_else(|| DevError::usage(format!("{option} requires a value")))
}

fn prepare_evidence_root(
    repository: &Path,
    configured: Option<&Path>,
) -> Result<PathBuf, DevError> {
    let artifact_root = repository.join(".artifacts");
    fs::create_dir_all(&artifact_root).map_err(|error| {
        DevError::infrastructure(format!(
            "create artifact root '{}': {error}",
            artifact_root.display()
        ))
    })?;
    let artifact_root = artifact_root.canonicalize().map_err(|error| {
        DevError::infrastructure(format!(
            "resolve artifact root '{}': {error}",
            artifact_root.display()
        ))
    })?;
    let path = match configured {
        Some(path) => path.to_path_buf(),
        None => {
            let parent = artifact_root.join("lkjscript-dev/scale");
            fs::create_dir_all(&parent).map_err(|error| {
                DevError::infrastructure(format!(
                    "create default scale root '{}': {error}",
                    parent.display()
                ))
            })?;
            parent.join(format!(
                "{}-{}-{}",
                unix_nanoseconds()?,
                std::process::id(),
                RUN_ORDINAL.fetch_add(1, Ordering::Relaxed)
            ))
        }
    };
    if !path.is_absolute() || has_parent_component(&path) {
        return Err(DevError::usage(
            "--evidence-root must be an absolute normalized path",
        ));
    }
    if fs::symlink_metadata(&path).is_ok() {
        return Err(DevError::usage(format!(
            "scale evidence root '{}' must be absent",
            path.display()
        )));
    }
    let parent = path
        .parent()
        .ok_or_else(|| DevError::usage("scale evidence root has no parent"))?;
    let canonical_parent = parent.canonicalize().map_err(|error| {
        DevError::usage(format!(
            "resolve scale evidence parent '{}': {error}",
            parent.display()
        ))
    })?;
    if !canonical_parent.starts_with(&artifact_root) {
        return Err(DevError::usage(format!(
            "scale evidence root '{}' must be below '{}',",
            path.display(),
            artifact_root.display()
        )));
    }
    fs::create_dir(&path).map_err(|error| {
        DevError::infrastructure(format!(
            "create scale evidence root '{}': {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn validate_project_destination(path: &Path) -> Result<(), DevError> {
    if !path.is_absolute() || has_parent_component(path) || path.file_name().is_none() {
        return Err(DevError::usage(
            "scale project destination must be an absolute normalized child path",
        ));
    }
    if fs::symlink_metadata(path).is_ok() {
        return Err(DevError::usage(format!(
            "scale project destination '{}' must be absent",
            path.display()
        )));
    }
    let parent = path
        .parent()
        .ok_or_else(|| DevError::usage("scale project destination has no parent"))?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        DevError::usage(format!(
            "inspect scale project parent '{}': {error}",
            parent.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::usage(format!(
            "scale project parent '{}' must be a regular directory",
            parent.display()
        )));
    }
    Ok(())
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| component == Component::ParentDir)
}

fn copy_candidate(
    repository: &Path,
    evidence_root: &Path,
    configured: &Path,
) -> Result<CandidateIdentity, DevError> {
    let source = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        repository.join(configured)
    };
    let metadata = fs::symlink_metadata(&source).map_err(|error| {
        DevError::usage(format!(
            "inspect scale candidate '{}': {error}",
            source.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(format!(
            "scale candidate '{}' must be a regular non-symlink file",
            source.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage(format!(
            "scale candidate '{}' is not executable",
            source.display()
        )));
    }
    let destination_directory = evidence_root.join("candidate");
    fs::create_dir(&destination_directory).map_err(|error| {
        DevError::infrastructure(format!(
            "create candidate directory '{}': {error}",
            destination_directory.display()
        ))
    })?;
    let destination = destination_directory.join("lkjscript");
    let mut input = File::open(&source).map_err(|error| {
        DevError::infrastructure(format!(
            "open scale candidate '{}': {error}",
            source.display()
        ))
    })?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o755);
    let mut output = options.open(&destination).map_err(|error| {
        DevError::infrastructure(format!(
            "create copied scale candidate '{}': {error}",
            destination.display()
        ))
    })?;
    let copied = std::io::copy(&mut input, &mut output).map_err(|error| {
        DevError::infrastructure(format!(
            "copy scale candidate to '{}': {error}",
            destination.display()
        ))
    })?;
    output.sync_all().map_err(|error| {
        DevError::infrastructure(format!(
            "synchronize copied scale candidate '{}': {error}",
            destination.display()
        ))
    })?;
    if copied != metadata.len() {
        return Err(DevError::infrastructure(
            "scale candidate changed while it was copied",
        ));
    }
    let verification = evidence::proof(&destination, evidence::relative(repository, &destination))?;
    Ok(CandidateIdentity {
        configured_path: configured.to_string_lossy().into_owned(),
        executed_path: destination.to_string_lossy().into_owned(),
        bytes: copied,
        sha256: sha256_file(&destination)?,
        verification,
    })
}

fn source_identity(repository: &Path) -> Result<SourceIdentity, DevError> {
    let status = git_bytes(
        repository,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    let upstream = optional_git_output(
        repository,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    );
    let (ahead, behind) = match upstream.as_deref() {
        Some(upstream) => {
            let counts = git_output(
                repository,
                &[
                    "rev-list",
                    "--left-right",
                    "--count",
                    &format!("HEAD...{upstream}"),
                ],
            )?;
            let mut fields = counts.split_whitespace();
            let ahead = fields.next().and_then(|value| value.parse().ok());
            let behind = fields.next().and_then(|value| value.parse().ok());
            if fields.next().is_some() || ahead.is_none() || behind.is_none() {
                return Err(DevError::infrastructure(
                    "git divergence output was not two unsigned counts",
                ));
            }
            (ahead, behind)
        }
        None => (None, None),
    };
    Ok(SourceIdentity {
        branch: git_output(repository, &["rev-parse", "--abbrev-ref", "HEAD"])?,
        commit: git_output(repository, &["rev-parse", "HEAD"])?,
        tree: git_output(repository, &["rev-parse", "HEAD^{tree}"])?,
        upstream,
        ahead,
        behind,
        worktree_clean: status.is_empty(),
        worktree_status_bytes: status.len() as u64,
        worktree_status_digest: VerificationDigest::of(&status),
    })
}

fn toolchain_identity(repository: &Path) -> Result<ToolchainIdentity, DevError> {
    Ok(ToolchainIdentity {
        rustc: strict_command_output(repository, "rustc", &["--version"])?,
        cargo: strict_command_output(repository, "cargo", &["--version"])?,
        channel_file: evidence::proof(
            &repository.join("rust-toolchain.toml"),
            "rust-toolchain.toml".to_owned(),
        )?,
    })
}

fn preflight_reason(options: &Options, host: &HostObservation) -> Option<String> {
    if options.minimum_available_memory_bytes > 0 {
        match host.memory_available_bytes {
            Some(bytes) if bytes < options.minimum_available_memory_bytes => {
                return Some(format!(
                    "available memory {bytes} bytes is below the requested {}-byte preflight floor",
                    options.minimum_available_memory_bytes
                ));
            }
            None => {
                return Some(
                    "available memory could not be observed for the requested preflight floor"
                        .to_owned(),
                );
            }
            _ => {}
        }
    }
    if options.minimum_available_disk_bytes > 0 {
        match host.disk_available_bytes {
            Some(bytes) if bytes < options.minimum_available_disk_bytes => {
                return Some(format!(
                    "available filesystem space {bytes} bytes is below the requested {}-byte preflight floor",
                    options.minimum_available_disk_bytes
                ));
            }
            None => {
                return Some(
                    "available filesystem space could not be observed for the requested preflight floor"
                        .to_owned(),
                );
            }
            _ => {}
        }
    }
    None
}

fn observe_host(path: &Path) -> HostObservation {
    let kernel = command_output("uname", &["-srvo"]);
    let logical_cpus = std::thread::available_parallelism()
        .ok()
        .map(|value| value.get() as u64);
    let memory = fs::read_to_string("/proc/meminfo").ok();
    let memory_total_bytes = memory
        .as_deref()
        .and_then(|value| memory_bytes(value, "MemTotal:"));
    let memory_available_bytes = memory
        .as_deref()
        .and_then(|value| memory_bytes(value, "MemAvailable:"));
    let disk = command_output(
        "df",
        &["-B1", "--output=fstype,size,avail", &path.to_string_lossy()],
    );
    let disk_fields = disk
        .as_deref()
        .and_then(|value| value.lines().nth(1))
        .map(|line| line.split_whitespace().collect::<Vec<_>>());
    let filesystem = disk_fields
        .as_ref()
        .and_then(|fields| fields.first())
        .map(|value| (*value).to_owned());
    let disk_total_bytes = disk_fields
        .as_ref()
        .and_then(|fields| fields.get(1))
        .and_then(|value| value.parse().ok());
    let disk_available_bytes = disk_fields
        .as_ref()
        .and_then(|fields| fields.get(2))
        .and_then(|value| value.parse().ok());
    let mut unavailable_dimensions = Vec::new();
    for (name, unavailable) in [
        ("kernel", kernel.is_none()),
        ("logical_cpus", logical_cpus.is_none()),
        ("memory_total_bytes", memory_total_bytes.is_none()),
        ("memory_available_bytes", memory_available_bytes.is_none()),
        ("filesystem", filesystem.is_none()),
        ("disk_total_bytes", disk_total_bytes.is_none()),
        ("disk_available_bytes", disk_available_bytes.is_none()),
    ] {
        if unavailable {
            unavailable_dimensions.push(name.to_owned());
        }
    }
    HostObservation {
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        kernel,
        logical_cpus,
        memory_total_bytes,
        memory_available_bytes,
        filesystem,
        disk_total_bytes,
        disk_available_bytes,
        unavailable_dimensions,
    }
}

fn memory_bytes(contents: &str, label: &str) -> Option<u64> {
    contents.lines().find_map(|line| {
        let value = line.strip_prefix(label)?.split_whitespace().next()?;
        value.parse::<u64>().ok()?.checked_mul(1_024)
    })
}

fn repository_area(project: &Path) -> Result<FileAreaEvidence, DevError> {
    let mut area = FileAreaEvidence::default();
    let mut entries = fs::read_dir(project)
        .map_err(|error| {
            DevError::infrastructure(format!(
                "read scale project '{}': {error}",
                project.display()
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| DevError::infrastructure(format!("read scale project entry: {error}")))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        if entry.file_name() == "derived" {
            continue;
        }
        add_area(&entry.path(), &mut area)?;
    }
    Ok(area)
}

fn area_if_present(path: &Path) -> Result<Option<FileAreaEvidence>, DevError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            let mut area = FileAreaEvidence::default();
            add_area(path, &mut area)?;
            Ok(Some(area))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(DevError::infrastructure(format!(
            "inspect scale area '{}': {error}",
            path.display()
        ))),
    }
}

fn add_area(path: &Path, area: &mut FileAreaEvidence) -> Result<(), DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect scale area '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(DevError::infrastructure(format!(
            "scale area '{}' contains a symlink",
            path.display()
        )));
    }
    if metadata.is_file() {
        area.files = area.files.saturating_add(1);
        area.bytes = area.bytes.saturating_add(metadata.len());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(DevError::infrastructure(format!(
            "scale area '{}' contains an unsupported entry",
            path.display()
        )));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| {
            DevError::infrastructure(format!(
                "read scale area directory '{}': {error}",
                path.display()
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| DevError::infrastructure(format!("read scale area entry: {error}")))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        add_area(&entry.path(), area)?;
    }
    Ok(())
}

fn cleanup_project(project: &Path, retain: bool) -> CleanupEvidence {
    let label = project.to_string_lossy().into_owned();
    let metadata = match fs::symlink_metadata(project) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return CleanupEvidence {
                status: CleanupStatus::NotCreated,
                project_path: label,
                detail: "project destination was never created".to_owned(),
            };
        }
        Err(error) => {
            return CleanupEvidence {
                status: CleanupStatus::Failed,
                project_path: label,
                detail: format!("inspect project before cleanup: {error}"),
            };
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return CleanupEvidence {
            status: CleanupStatus::Failed,
            project_path: label,
            detail: "project destination became a non-directory or symlink".to_owned(),
        };
    }
    if retain {
        return CleanupEvidence {
            status: CleanupStatus::Retained,
            project_path: label,
            detail: "project retained by explicit --retain request".to_owned(),
        };
    }
    match fs::remove_dir_all(project) {
        Ok(()) => CleanupEvidence {
            status: CleanupStatus::Removed,
            project_path: label,
            detail: "campaign-owned temporary project removed".to_owned(),
        },
        Err(error) => CleanupEvidence {
            status: CleanupStatus::Failed,
            project_path: label,
            detail: format!("remove campaign-owned temporary project: {error}"),
        },
    }
}

fn identities(
    records: &[lkjscript::platform::control::CompactRecord],
) -> Result<BTreeMap<String, String>, DevError> {
    let mut identities = BTreeMap::new();
    for value in records.iter().filter(|value| value.operation == "identity") {
        let symbol = required_field(value, "symbol")?.to_owned();
        let id = required_field(value, "id")?.to_owned();
        if identities.insert(symbol.clone(), id).is_some() {
            return Err(DevError::corrupt(format!(
                "compact response repeated allocation '{symbol}'"
            )));
        }
    }
    Ok(identities)
}

fn project_arguments(project: &Path, suffix: &[&str]) -> Vec<String> {
    let mut arguments = vec![
        "--project".to_owned(),
        project.to_string_lossy().into_owned(),
    ];
    arguments.extend(suffix.iter().map(|value| (*value).to_owned()));
    arguments
}

fn require_observed_revision(
    operation: &str,
    expected: &str,
    records: &[lkjscript::platform::control::CompactRecord],
) -> Result<(), DevError> {
    require_revision(
        operation,
        expected,
        required_field(record(records, "revision")?, "observed")?,
    )
}

fn require_revision(operation: &str, expected: &str, observed: &str) -> Result<(), DevError> {
    if expected != observed {
        return Err(DevError::corrupt(format!(
            "{operation} observed revision {observed}; expected {expected}"
        )));
    }
    Ok(())
}

fn files_equal(left: &Path, right: &Path) -> Result<bool, DevError> {
    let left_metadata = fs::symlink_metadata(left).map_err(|error| {
        DevError::infrastructure(format!("inspect artifact '{}': {error}", left.display()))
    })?;
    let right_metadata = fs::symlink_metadata(right).map_err(|error| {
        DevError::infrastructure(format!("inspect artifact '{}': {error}", right.display()))
    })?;
    if left_metadata.file_type().is_symlink()
        || right_metadata.file_type().is_symlink()
        || !left_metadata.is_file()
        || !right_metadata.is_file()
        || left_metadata.len() != right_metadata.len()
    {
        return Ok(false);
    }
    let mut left_file = File::open(left).map_err(|error| {
        DevError::infrastructure(format!("open artifact '{}': {error}", left.display()))
    })?;
    let mut right_file = File::open(right).map_err(|error| {
        DevError::infrastructure(format!("open artifact '{}': {error}", right.display()))
    })?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_read = left_file.read(&mut left_buffer).map_err(|error| {
            DevError::infrastructure(format!("read artifact '{}': {error}", left.display()))
        })?;
        let right_read = right_file.read(&mut right_buffer).map_err(|error| {
            DevError::infrastructure(format!("read artifact '{}': {error}", right.display()))
        })?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn sha256_file(path: &Path) -> Result<String, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect SHA-256 input '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "SHA-256 input '{}' is not a regular file",
            path.display()
        )));
    }
    let mut input = File::open(path).map_err(|error| {
        DevError::infrastructure(format!("open SHA-256 input '{}': {error}", path.display()))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut observed = 0_u64;
    loop {
        let read = input.read(&mut buffer).map_err(|error| {
            DevError::infrastructure(format!("read SHA-256 input '{}': {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        observed = observed.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
    }
    if observed != metadata.len() {
        return Err(DevError::infrastructure(format!(
            "SHA-256 input '{}' changed while read",
            path.display()
        )));
    }
    Ok(lower_hex(&hasher.finalize()))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn sum_optional(values: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() || values.iter().any(Option::is_none) {
        return None;
    }
    Some(
        values
            .into_iter()
            .flatten()
            .fold(0_u64, u64::saturating_add),
    )
}

fn process_peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmHWM:")?.split_whitespace().next()?;
        value.parse().ok()
    })
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| DevError::infrastructure("resolve repository root"))
}

fn git_output(repository: &Path, arguments: &[&str]) -> Result<String, DevError> {
    let bytes = git_bytes(repository, arguments)?;
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|_| DevError::infrastructure("git output is not UTF-8"))
}

fn git_bytes(repository: &Path, arguments: &[&str]) -> Result<Vec<u8>, DevError> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .output()
        .map_err(|error| DevError::infrastructure(format!("run git: {error}")))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(DevError::infrastructure(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}

fn optional_git_output(repository: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn strict_command_output(
    repository: &Path,
    program: &str,
    arguments: &[&str],
) -> Result<String, DevError> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(repository)
        .output()
        .map_err(|error| DevError::infrastructure(format!("run {program}: {error}")))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(DevError::infrastructure(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| DevError::infrastructure(format!("{program} output is not UTF-8")))
}

fn command_output(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    if !output.status.success() || !output.stderr.is_empty() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn diagnostic_error(diagnostic: lkjscript::platform::diagnostic::Diagnostic) -> DevError {
    let message = format!("{}: {}", diagnostic.code, diagnostic.message);
    match diagnostic.class {
        lkjscript::platform::diagnostic::DiagnosticClass::Infrastructure => {
            DevError::infrastructure(message)
        }
        lkjscript::platform::diagnostic::DiagnosticClass::Resource => {
            DevError::unavailable(message)
        }
        _ => DevError::corrupt(message),
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

fn print_summary(
    repository: &Path,
    machine: bool,
    topology: &str,
    lifecycle: &str,
    receipt: &ScaleReceipt,
    published: &PublishedEvidence,
) -> Result<(), DevError> {
    #[derive(Serialize)]
    struct Summary<'a> {
        schema: &'static str,
        contract_version: u32,
        status: ReceiptStatus,
        admission: AdmissionClassification,
        topology: &'a str,
        lifecycle: &'a str,
        receipt: String,
        receipt_bytes: u64,
        receipt_digest: &'a VerificationDigest,
    }
    let summary = Summary {
        schema: SCALE_SCHEMA,
        contract_version: receipt.contract_version,
        status: receipt.status,
        admission: receipt.admission,
        topology,
        lifecycle,
        receipt: evidence::relative(repository, &published.path),
        receipt_bytes: published.bytes,
        receipt_digest: &published.digest,
    };
    if machine {
        println!(
            "{}",
            serde_json::to_string(&summary).map_err(|error| {
                DevError::infrastructure(format!("encode scale summary: {error}"))
            })?
        );
    } else {
        println!(
            "scale {:?}: topology={} lifecycle={} admission={:?} receipt={} digest={}",
            summary.status,
            summary.topology,
            summary.lifecycle,
            summary.admission,
            summary.receipt,
            summary.receipt_digest
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::model::ScaleReceipt;

    fn options(values: &[&str]) -> Options {
        parse_options(values.iter().copied().map(OsString::from)).expect("valid scale options")
    }

    #[test]
    fn topology_grammar_and_current_batch_bounds_are_closed() {
        for topology in [
            "independent-modules",
            "small-functions",
            "wide-module",
            "deep-chain",
            "wide-fanout",
        ] {
            assert_eq!(options(&[topology, "--items", "1"]).items, 1);
        }
        assert!(parse_options(["full"].into_iter().map(OsString::from)).is_err());
        assert!(
            parse_options(
                ["independent-modules", "--batch", "1001"]
                    .into_iter()
                    .map(OsString::from)
            )
            .is_err()
        );
        assert!(
            parse_options(
                ["small-functions", "--batch", "1001"]
                    .into_iter()
                    .map(OsString::from)
            )
            .is_err()
        );
        assert!(HELP.contains("contract 3"));
    }

    #[test]
    fn invalid_options_publish_one_failed_contract_three_receipt() {
        let repository = repository_root().expect("repository root");
        let parent = repository.join(".artifacts/lkjscript-dev/scale-option-tests");
        fs::create_dir_all(&parent).expect("option-test parent");
        let evidence_root = parent.join(format!(
            "{}-{}",
            std::process::id(),
            unix_nanoseconds().expect("test time")
        ));
        let code = command(
            [
                OsString::from("small-functions"),
                OsString::from("--items"),
                OsString::from("0"),
                OsString::from("--evidence-root"),
                evidence_root.clone().into_os_string(),
                OsString::from("--machine"),
            ]
            .into_iter(),
        )
        .expect("invalid options publish a receipt");
        assert_eq!(code, 1);
        let bytes = fs::read(evidence_root.join("receipt.json")).expect("failed receipt");
        let receipt: ScaleReceipt = serde_json::from_slice(&bytes).expect("contract-3 receipt");
        assert_eq!(receipt.status, ReceiptStatus::Failed);
        assert_eq!(receipt.admission, AdmissionClassification::Failed);
        assert_eq!(receipt.commands.len(), 0);
        assert_eq!(receipt.cleanup.status, CleanupStatus::NotCreated);
        assert_eq!(
            receipt
                .failure
                .as_ref()
                .map(|failure| failure.class.as_str()),
            Some("usage")
        );
        fs::remove_dir_all(&evidence_root).expect("remove option-test evidence");
    }

    #[test]
    fn scenario_counts_bind_every_retained_topology() {
        let independent = scenario(&options(&["independent-modules", "--items", "4"]))
            .expect("independent scenario");
        assert_eq!(independent.requested.owners, 4);
        assert_eq!(independent.requested.relations, 0);
        let small = scenario(&options(&[
            "small-functions",
            "--items",
            "5",
            "--modules",
            "2",
        ]))
        .expect("small-functions scenario");
        assert_eq!(small.requested.owners, 12);
        assert_eq!(small.requested.modules, 2);
        assert_eq!(small.requested.functions, 5);
        assert_eq!(small.requested.relations, 10);
        let wide =
            scenario(&options(&["wide-module", "--items", "5"])).expect("wide-module scenario");
        assert_eq!(wide.requested.owners, 11);
        assert_eq!(wide.requested.relations, 10);
        for topology in ["deep-chain", "wide-fanout"] {
            let graph = scenario(&options(&[topology, "--items", "5"]))
                .expect("relational function scenario");
            assert_eq!(graph.requested.owners, 11);
            assert_eq!(graph.requested.relations, 14);
        }
    }

    #[test]
    fn compact_requests_are_deterministic_and_within_public_record_bounds() {
        let revision = format!("rev_{}", "0".repeat(64));
        let modules_a = module_request(&revision, 0, 20, "fixture").expect("module request");
        let modules_b = module_request(&revision, 0, 20, "fixture").expect("module repeat");
        assert_eq!(modules_a.bytes, modules_b.bytes);
        assert_eq!(modules_a.records, 21);
        let functions = function_request(
            &revision,
            0,
            20,
            &[format!("mod_{}", "1".repeat(32))],
            FunctionShape::Unit,
            None,
        )
        .expect("function request");
        assert_eq!(functions.records, 41);
        let parsed = parse_records("fixture", &functions.bytes).expect("current compact request");
        assert_eq!(parsed.len() as u64, functions.records);
        assert_eq!(parsed[0].operation, "request");
    }

    #[test]
    fn logical_shape_digest_excludes_fresh_authority_identities() {
        let mut left = LogicalEvidence {
            starting_revision: Some("rev_left_start".to_owned()),
            final_revision: Some("rev_left_final".to_owned()),
            repository: Some("repo_left".to_owned()),
            package: Some("pkg_left".to_owned()),
            semantic_state: Some("state_left".to_owned()),
            construction_batches: 2,
            plan_batches: 2,
            apply_batches: 2,
            observed: Some(SemanticCounts {
                owners: 3,
                modules: 1,
                functions: 1,
                relations: 2,
            }),
            public_oracle_equal: Some(true),
            ..LogicalEvidence::default()
        };
        let mut right = left.clone();
        right.starting_revision = Some("rev_right_start".to_owned());
        right.final_revision = Some("rev_right_final".to_owned());
        right.repository = Some("repo_right".to_owned());
        right.package = Some("pkg_right".to_owned());
        right.semantic_state = Some("state_right".to_owned());
        left.accepted_revisions.push("rev_left_batch".to_owned());
        right.accepted_revisions.push("rev_right_batch".to_owned());
        assert_eq!(
            shape_digest(&options(&["small-functions", "--items", "1"]), &left)
                .expect("left shape"),
            shape_digest(&options(&["small-functions", "--items", "1"]), &right)
                .expect("right shape")
        );
    }

    #[test]
    fn receipt_contract_is_three_and_rejects_unknown_fields() {
        assert_eq!(SCALE_CONTRACT_VERSION, 3);
        assert_eq!(SCALE_SCHEMA, "lkjscript-semantic-scale-receipt");
        assert!(
            serde_json::from_str::<ScaleReceipt>(r#"{"unexpected":true}"#).is_err(),
            "strict contract-3 decoding must reject foreign receipt fields"
        );
    }

    #[test]
    fn terminal_wall_limit_includes_the_exact_boundary() {
        assert!(!super::elapsed_reaches_wall_limit(
            Duration::from_nanos(999),
            Duration::from_nanos(1_000),
        ));
        assert!(super::elapsed_reaches_wall_limit(
            Duration::from_nanos(1_000),
            Duration::from_nanos(1_000),
        ));
        assert!(super::elapsed_reaches_wall_limit(
            Duration::from_nanos(1_001),
            Duration::from_nanos(1_000),
        ));
    }

    #[test]
    fn predecessor_scale_vocabulary_is_absent_from_the_production_module() {
        let source = include_str!("scale.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production scale module");
        for forbidden in [
            ["PUBLIC_CHANGE_", "CONTRACT_VERSION"].concat(),
            ["Cli", "Envelope"].concat(),
            ["\"--", "commit\""].concat(),
            ["\"inspect\", \"", "project\""].concat(),
            ["\"--", "exact\""].concat(),
            ["\"doctor\", \"--", "deep\""].concat(),
            ["&[\"backup\", \"--", "output\""].concat(),
        ] {
            assert!(
                !production.contains(&forbidden),
                "removed scale vocabulary returned: {forbidden}"
            );
        }
    }
}
