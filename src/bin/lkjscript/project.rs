#![allow(clippy::result_large_err)]

use super::{
    CliOutcome, EXIT_ARTIFACT, EXIT_OUTPUT, EXIT_RESOURCE, EXIT_TRANSPORT, EXIT_USAGE_OR_JSON,
};
use lkjscript::application::{ApplicationInvocation, ApplicationRunReceipt};
use lkjscript::error::{ErrorCode, LkError};
use lkjscript::history::{HistoryPage, RestorationReceipt, RevisionRecordInspection};
use lkjscript::ids::Revision;
use lkjscript::machine::MAX_JSON_OUTPUT_BYTES;
use lkjscript::project::{
    MAXIMUM_PROJECT_CHANGE_BYTES, Project, ProjectBackupReceipt, ProjectChangeReceipt,
    ProjectChangeRequest, ProjectContextResult, ProjectDiffPage, ProjectInitReceipt,
    ProjectNodeInspection, ProjectOrientationResult, ProjectStatus, ProjectTargetReceipt,
    TargetInspection, decode_change,
};
use lkjscript::target::TargetSummary;
use lkjscript::transaction::TransactionMode;
use lkjscript::workbench::{
    ContextPacket, ContextPacketDigest, ContextPurpose, decode_context_packet, parse_edit_document,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Read;
use std::io::{BufRead, Write};
use std::path::{Component, Path, PathBuf};

const PROJECT_MACHINE_VERSION: u16 = 1;
const PROJECT_SESSION_VERSION: u16 = 1;
const MAXIMUM_PROJECT_SESSION_REQUESTS: usize = 65_536;

pub(super) enum ProjectCommand {
    Init {
        path: PathBuf,
        pretty: bool,
    },
    Orient {
        project: Option<PathBuf>,
        known_digest: Option<String>,
        pretty: bool,
    },
    Status {
        project: Option<PathBuf>,
        pretty: bool,
    },
    Inspect {
        project: Option<PathBuf>,
        selector: String,
        revision: Option<Revision>,
        expand: bool,
        pretty: bool,
    },
    Context {
        project: Option<PathBuf>,
        purpose: ContextPurpose,
        targets: Vec<String>,
        revision: Option<Revision>,
        from_revision: Option<Revision>,
        maximum_nodes: Option<u32>,
        known_digest: Option<ContextPacketDigest>,
        pretty: bool,
    },
    Change {
        project: Option<PathBuf>,
        mode: TransactionMode,
        input: ChangeInput,
        pretty: bool,
    },
    Log {
        project: Option<PathBuf>,
        before: Option<Revision>,
        limit: u32,
        pretty: bool,
    },
    Show {
        project: Option<PathBuf>,
        revision: Revision,
        pretty: bool,
    },
    Diff {
        project: Option<PathBuf>,
        from: Revision,
        to: Revision,
        offset: u64,
        limit: u32,
        pretty: bool,
    },
    Restore {
        project: Option<PathBuf>,
        source_revision: Revision,
        validate_only: bool,
        pretty: bool,
    },
    TargetList {
        project: Option<PathBuf>,
        revision: Option<Revision>,
        pretty: bool,
    },
    TargetShow {
        project: Option<PathBuf>,
        selector: String,
        revision: Option<Revision>,
        pretty: bool,
    },
    TargetBuild {
        project: Option<PathBuf>,
        selector: String,
        revision: Option<Revision>,
        output: PathBuf,
        pretty: bool,
    },
    TargetTest {
        project: Option<PathBuf>,
        selector: String,
        revision: Option<Revision>,
        pretty: bool,
    },
    TargetRun {
        project: Option<PathBuf>,
        selector: String,
        revision: Option<Revision>,
        pretty: bool,
    },
    Doctor {
        project: Option<PathBuf>,
        deep: bool,
        pretty: bool,
    },
    Backup {
        project: Option<PathBuf>,
        destination: PathBuf,
        pretty: bool,
    },
    Session {
        project: Option<PathBuf>,
    },
}

pub(super) enum ChangeInput {
    Json,
    Document { context: Option<PathBuf> },
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ProjectEnvelope {
    version: u16,
    result: ProjectResult,
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum ProjectResult {
    Initialized(ProjectInitReceipt),
    Orientation(ProjectOrientationResult),
    Status(ProjectStatus),
    Inspection(ProjectNodeInspection),
    Context(ProjectContextResult),
    Change(ProjectChangeReceipt),
    History(HistoryPage),
    Revision(RevisionRecordInspection),
    Diff(ProjectDiffPage),
    Restoration(RestorationReceipt),
    Targets(Vec<TargetSummary>),
    Target(TargetInspection),
    TargetReceipt(ProjectTargetReceipt),
    TargetRun(ApplicationRunReceipt),
    Backup(ProjectBackupReceipt),
    Error(LkError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSessionRequestEnvelope {
    version: u16,
    request_id: u64,
    request: ProjectSessionRequest,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum ProjectSessionRequest {
    Orient {
        #[serde(default)]
        known_digest: Option<String>,
    },
    Status,
    Inspect {
        selector: String,
        #[serde(default)]
        revision: Option<Revision>,
        #[serde(default)]
        summary: bool,
    },
    Context {
        purpose: ContextPurpose,
        #[serde(default)]
        targets: Vec<String>,
        #[serde(default)]
        revision: Option<Revision>,
        #[serde(default)]
        from_revision: Option<Revision>,
        #[serde(default)]
        maximum_nodes: Option<u32>,
        #[serde(default)]
        known_digest: Option<ContextPacketDigest>,
    },
    ChangeValidate(ProjectChangeRequest),
    ChangeApply(ProjectChangeRequest),
    DocumentValidate {
        document: String,
    },
    DocumentApply {
        document: String,
    },
    Log {
        #[serde(default)]
        before: Option<Revision>,
        limit: u32,
    },
    Show {
        revision: Revision,
    },
    Diff {
        from: Revision,
        to: Revision,
        #[serde(default)]
        offset: u64,
        limit: u32,
    },
    Restore {
        source_revision: Revision,
        #[serde(default)]
        validate_only: bool,
    },
    TargetList {
        #[serde(default)]
        revision: Option<Revision>,
    },
    TargetShow {
        selector: String,
        #[serde(default)]
        revision: Option<Revision>,
    },
    TargetBuild {
        selector: String,
        #[serde(default)]
        revision: Option<Revision>,
        output: PathBuf,
    },
    TargetTest {
        selector: String,
        #[serde(default)]
        revision: Option<Revision>,
    },
    TargetRun {
        selector: String,
        #[serde(default)]
        revision: Option<Revision>,
        invocation: ApplicationInvocation,
    },
    Doctor {
        #[serde(default)]
        deep: bool,
    },
    Backup {
        destination: PathBuf,
    },
    Shutdown,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ProjectSessionResponseEnvelope {
    version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_id: Option<u64>,
    result: ProjectResult,
}

enum SessionLine {
    End,
    Data(Vec<u8>),
    TooLarge,
}

struct ProjectSession {
    project: Project,
    request_ids: BTreeSet<u64>,
    context: Option<ContextPacket>,
}

pub(super) fn is_project_command(command: &str) -> bool {
    matches!(
        command,
        "init"
            | "orient"
            | "status"
            | "inspect"
            | "context"
            | "change"
            | "log"
            | "show"
            | "diff"
            | "restore"
            | "target"
            | "doctor"
            | "backup"
            | "session"
    )
}

pub(super) fn parse(
    command: &str,
    arguments: impl Iterator<Item = String>,
) -> Result<ProjectCommand, String> {
    let arguments = arguments.collect::<Vec<_>>();
    match command {
        "init" => parse_init(&arguments),
        "orient" => parse_orient(&arguments),
        "status" => parse_status(&arguments),
        "inspect" => parse_inspect(&arguments),
        "context" => parse_context(&arguments),
        "change" => parse_change(&arguments),
        "log" => parse_log(&arguments),
        "show" => parse_show(&arguments),
        "diff" => parse_diff(&arguments),
        "restore" => parse_restore(&arguments),
        "target" => parse_target(&arguments),
        "doctor" => parse_doctor(&arguments),
        "backup" => parse_backup(&arguments),
        "session" => parse_project_session(&arguments),
        _ => Err(project_usage("unknown project command")),
    }
}

pub(super) fn run(command: ProjectCommand) -> CliOutcome {
    let (result, pretty) = match run_inner(command) {
        Ok(value) => value,
        Err((error, pretty)) => return encode_error(error, pretty),
    };
    encode_result(result, pretty, 0, None)
}

pub(super) fn run_session(project_path: Option<&Path>) -> std::process::ExitCode {
    let project = match Project::open(project_path) {
        Ok(project) => project,
        Err(error) => {
            let response = encode_session_response(None, ProjectResult::Error(error));
            return write_session_response(response, true);
        }
    };
    let mut session = ProjectSession {
        project,
        request_ids: BTreeSet::new(),
        context: None,
    };
    let mut reader = std::io::BufReader::new(std::io::stdin().lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let line = match read_session_line(&mut reader, MAXIMUM_PROJECT_CHANGE_BYTES) {
            Ok(line) => line,
            Err(error) => {
                eprintln!("cannot read project session request: {error}");
                return std::process::ExitCode::from(EXIT_TRANSPORT);
            }
        };
        let (request_id, result, shutdown, fatal) = match line {
            SessionLine::End => return std::process::ExitCode::SUCCESS,
            SessionLine::TooLarge => (
                None,
                ProjectResult::Error(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "project session request exceeds input byte policy",
                )),
                false,
                false,
            ),
            SessionLine::Data(bytes) => match decode_session_request(&bytes) {
                Ok(envelope) => {
                    let request_id = envelope.request_id;
                    if request_id == 0 {
                        (
                            Some(request_id),
                            ProjectResult::Error(LkError::new(
                                ErrorCode::ProtocolMalformed,
                                "project session request ID must be nonzero",
                            )),
                            false,
                            false,
                        )
                    } else if session.request_ids.len() >= MAXIMUM_PROJECT_SESSION_REQUESTS {
                        (
                            Some(request_id),
                            ProjectResult::Error(LkError::new(
                                ErrorCode::PolicyExceeded,
                                "project session request-count policy is exhausted",
                            )),
                            false,
                            true,
                        )
                    } else if !session.request_ids.insert(request_id) {
                        (
                            Some(request_id),
                            ProjectResult::Error(LkError::new(
                                ErrorCode::ProtocolMalformed,
                                "project session request ID is duplicated",
                            )),
                            false,
                            false,
                        )
                    } else {
                        let shutdown = matches!(&envelope.request, ProjectSessionRequest::Shutdown);
                        match session.execute(envelope.request) {
                            Ok(result) => (Some(request_id), result, shutdown, false),
                            Err(error) => {
                                let fatal = matches!(
                                    error.code,
                                    ErrorCode::CommitOutcomeUnknown
                                        | ErrorCode::ArtifactPublicationOutcomeUnknown
                                );
                                (Some(request_id), ProjectResult::Error(error), false, fatal)
                            }
                        }
                    }
                }
                Err(error) => (None, ProjectResult::Error(error), false, false),
            },
        };
        let response = encode_session_response(request_id, result);
        if let Err(error) = stdout.write_all(&response).and_then(|()| stdout.flush()) {
            eprintln!("cannot write project session response: {error}");
            return std::process::ExitCode::from(EXIT_OUTPUT);
        }
        if shutdown {
            return std::process::ExitCode::SUCCESS;
        }
        if fatal {
            return std::process::ExitCode::from(EXIT_TRANSPORT);
        }
    }
}

fn run_inner(command: ProjectCommand) -> Result<(ProjectResult, bool), (LkError, bool)> {
    match command {
        ProjectCommand::Init { path, pretty } => Project::initialize(&path)
            .map(ProjectResult::Initialized)
            .map(|result| (result, pretty))
            .map_err(|error| (error, pretty)),
        ProjectCommand::Orient {
            project,
            known_digest,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project
                .orient(known_digest.as_deref())
                .map(ProjectResult::Orientation)
        }),
        ProjectCommand::Status { project, pretty } => {
            with_project(project.as_deref(), pretty, |project| {
                project.status().map(ProjectResult::Status)
            })
        }
        ProjectCommand::Inspect {
            project,
            selector,
            revision,
            expand,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project
                .inspect(&selector, revision, expand)
                .map(ProjectResult::Inspection)
        }),
        ProjectCommand::Context {
            project,
            purpose,
            targets,
            revision,
            from_revision,
            maximum_nodes,
            known_digest,
            pretty,
        } => with_project_mut(project.as_deref(), pretty, |project| {
            project
                .context(
                    purpose,
                    &targets,
                    revision,
                    from_revision,
                    maximum_nodes,
                    known_digest,
                )
                .map(ProjectResult::Context)
        }),
        ProjectCommand::Change {
            project,
            mode,
            input: input_mode,
            pretty,
        } => {
            let input_bytes = read_stdin(MAXIMUM_PROJECT_CHANGE_BYTES, "project change")
                .map_err(|error| (error, pretty))?;
            let request = match input_mode {
                ChangeInput::Json => {
                    decode_change(&input_bytes).map_err(|error| (error, pretty))?
                }
                ChangeInput::Document { context } => {
                    let packet = context
                        .as_deref()
                        .map(read_context)
                        .transpose()
                        .map_err(|error| (error, pretty))?;
                    let parsed = parse_edit_document(&input_bytes, mode, packet.as_ref())
                        .map_err(|error| (document_error(error), pretty))?;
                    lkjscript::project::ProjectChangeRequest {
                        version: lkjscript::project::PROJECT_CHANGE_VERSION,
                        workspace: parsed.request.transaction.workspace,
                        base_revision: parsed.request.transaction.base_revision,
                        idempotency_key: parsed.request.transaction.idempotency_key,
                        operations: parsed.request.transaction.operations,
                        response: parsed.request.response,
                    }
                }
            };
            with_project_mut(project.as_deref(), pretty, |project| {
                project
                    .apply_change(&request, mode)
                    .map(ProjectResult::Change)
            })
        }
        ProjectCommand::Log {
            project,
            before,
            limit,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project.history(before, limit).map(ProjectResult::History)
        }),
        ProjectCommand::Show {
            project,
            revision,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project
                .revision_record(revision)
                .map(ProjectResult::Revision)
        }),
        ProjectCommand::Diff {
            project,
            from,
            to,
            offset,
            limit,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project
                .diff(from, to, offset, limit)
                .map(ProjectResult::Diff)
        }),
        ProjectCommand::Restore {
            project,
            source_revision,
            validate_only,
            pretty,
        } => with_project_mut(project.as_deref(), pretty, |project| {
            project
                .restore(source_revision, validate_only)
                .map(ProjectResult::Restoration)
        }),
        ProjectCommand::TargetList {
            project,
            revision,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project.list_targets(revision).map(ProjectResult::Targets)
        }),
        ProjectCommand::TargetShow {
            project,
            selector,
            revision,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project
                .show_target(&selector, revision)
                .map(ProjectResult::Target)
        }),
        ProjectCommand::TargetBuild {
            project,
            selector,
            revision,
            output,
            pretty,
        } => {
            let output = canonical_output_path(&output).map_err(|error| (error, pretty))?;
            with_project(project.as_deref(), pretty, |project| {
                project
                    .build_target(&selector, revision, &output)
                    .map(ProjectResult::TargetReceipt)
            })
        }
        ProjectCommand::TargetTest {
            project,
            selector,
            revision,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project
                .test_target(&selector, revision)
                .map(ProjectResult::TargetReceipt)
        }),
        ProjectCommand::TargetRun {
            project,
            selector,
            revision,
            pretty,
        } => {
            let input = read_stdin(MAXIMUM_PROJECT_CHANGE_BYTES, "target invocation")
                .map_err(|error| (error, pretty))?;
            let invocation = decode_json::<ApplicationInvocation>(&input, "target invocation")
                .map_err(|error| (error, pretty))?;
            with_project(project.as_deref(), pretty, |project| {
                project
                    .run_target(&selector, revision, &invocation)
                    .map(ProjectResult::TargetRun)
            })
        }
        ProjectCommand::Doctor {
            project,
            deep,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project.doctor(deep).map(ProjectResult::Status)
        }),
        ProjectCommand::Backup {
            project,
            destination,
            pretty,
        } => with_project(project.as_deref(), pretty, |project| {
            project.backup(&destination).map(ProjectResult::Backup)
        }),
        ProjectCommand::Session { .. } => Err((
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "project session must own the process input and output streams",
            ),
            false,
        )),
    }
}

impl ProjectSession {
    fn execute(&mut self, request: ProjectSessionRequest) -> lkjscript::Result<ProjectResult> {
        match request {
            ProjectSessionRequest::Orient { known_digest } => self
                .project
                .orient(known_digest.as_deref())
                .map(ProjectResult::Orientation),
            ProjectSessionRequest::Status => self.project.status().map(ProjectResult::Status),
            ProjectSessionRequest::Inspect {
                selector,
                revision,
                summary,
            } => {
                let selector = self.selector(&selector, revision)?;
                self.project
                    .inspect(&selector, revision, !summary)
                    .map(ProjectResult::Inspection)
            }
            ProjectSessionRequest::Context {
                purpose,
                targets,
                revision,
                from_revision,
                maximum_nodes,
                known_digest,
            } => {
                let targets = targets
                    .iter()
                    .map(|selector| self.selector(selector, revision))
                    .collect::<lkjscript::Result<Vec<_>>>()?;
                let result = self.project.context(
                    purpose,
                    &targets,
                    revision,
                    from_revision,
                    maximum_nodes,
                    known_digest,
                )?;
                if let ProjectContextResult::Changed(packet) = &result {
                    self.context = Some((**packet).clone());
                }
                Ok(ProjectResult::Context(result))
            }
            ProjectSessionRequest::ChangeValidate(request) => self
                .project
                .apply_change(&request, TransactionMode::ValidateOnly)
                .map(ProjectResult::Change),
            ProjectSessionRequest::ChangeApply(request) => {
                let receipt = self
                    .project
                    .apply_change(&request, TransactionMode::Commit)?;
                Ok(ProjectResult::Change(receipt))
            }
            ProjectSessionRequest::DocumentValidate { document } => {
                let request = self.document_request(&document, TransactionMode::ValidateOnly)?;
                self.project
                    .apply_change(&request, TransactionMode::ValidateOnly)
                    .map(ProjectResult::Change)
            }
            ProjectSessionRequest::DocumentApply { document } => {
                let request = self.document_request(&document, TransactionMode::Commit)?;
                let receipt = self
                    .project
                    .apply_change(&request, TransactionMode::Commit)?;
                Ok(ProjectResult::Change(receipt))
            }
            ProjectSessionRequest::Log { before, limit } => self
                .project
                .history(before, limit)
                .map(ProjectResult::History),
            ProjectSessionRequest::Show { revision } => self
                .project
                .revision_record(revision)
                .map(ProjectResult::Revision),
            ProjectSessionRequest::Diff {
                from,
                to,
                offset,
                limit,
            } => self
                .project
                .diff(from, to, offset, limit)
                .map(ProjectResult::Diff),
            ProjectSessionRequest::Restore {
                source_revision,
                validate_only,
            } => self
                .project
                .restore(source_revision, validate_only)
                .map(ProjectResult::Restoration),
            ProjectSessionRequest::TargetList { revision } => self
                .project
                .list_targets(revision)
                .map(ProjectResult::Targets),
            ProjectSessionRequest::TargetShow { selector, revision } => {
                let selector = self.selector(&selector, revision)?;
                self.project
                    .show_target(&selector, revision)
                    .map(ProjectResult::Target)
            }
            ProjectSessionRequest::TargetBuild {
                selector,
                revision,
                output,
            } => {
                let selector = self.selector(&selector, revision)?;
                let output = canonical_output_path(&output)?;
                self.project
                    .build_target(&selector, revision, &output)
                    .map(ProjectResult::TargetReceipt)
            }
            ProjectSessionRequest::TargetTest { selector, revision } => {
                let selector = self.selector(&selector, revision)?;
                self.project
                    .test_target(&selector, revision)
                    .map(ProjectResult::TargetReceipt)
            }
            ProjectSessionRequest::TargetRun {
                selector,
                revision,
                invocation,
            } => {
                let selector = self.selector(&selector, revision)?;
                self.project
                    .run_target(&selector, revision, &invocation)
                    .map(ProjectResult::TargetRun)
            }
            ProjectSessionRequest::Doctor { deep } => {
                self.project.doctor(deep).map(ProjectResult::Status)
            }
            ProjectSessionRequest::Backup { destination } => {
                self.project.backup(&destination).map(ProjectResult::Backup)
            }
            ProjectSessionRequest::Shutdown => self.project.status().map(ProjectResult::Status),
        }
    }

    fn selector(&self, selector: &str, revision: Option<Revision>) -> lkjscript::Result<String> {
        let Some(alias) = selector.strip_prefix('@') else {
            return Ok(selector.to_owned());
        };
        let packet = self.context.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidQuery,
                "project session alias has no live context capsule",
            )
        })?;
        let selected_revision = revision.unwrap_or(self.project.current_revision()?);
        if packet.payload.workspace != self.project.workspace()
            || packet.payload.revision != selected_revision
        {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                "project session alias is stale for the selected revision",
            )
            .for_workspace(self.project.workspace())
            .at_revision(selected_revision));
        }
        packet
            .payload
            .aliases
            .iter()
            .find(|binding| binding.alias == alias)
            .map(|binding| binding.node.to_string())
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidQuery,
                    "project session alias is unknown in the live context capsule",
                )
            })
    }

    fn document_request(
        &self,
        document: &str,
        mode: TransactionMode,
    ) -> lkjscript::Result<ProjectChangeRequest> {
        let parsed = parse_edit_document(document.as_bytes(), mode, self.context.as_ref())
            .map_err(document_error)?;
        Ok(ProjectChangeRequest {
            version: lkjscript::project::PROJECT_CHANGE_VERSION,
            workspace: parsed.request.transaction.workspace,
            base_revision: parsed.request.transaction.base_revision,
            idempotency_key: parsed.request.transaction.idempotency_key,
            operations: parsed.request.transaction.operations,
            response: parsed.request.response,
        })
    }
}

fn with_project(
    path: Option<&Path>,
    pretty: bool,
    action: impl FnOnce(&Project) -> lkjscript::Result<ProjectResult>,
) -> Result<(ProjectResult, bool), (LkError, bool)> {
    let project = Project::open(path).map_err(|error| (error, pretty))?;
    action(&project)
        .map(|result| (result, pretty))
        .map_err(|error| (error, pretty))
}

fn with_project_mut(
    path: Option<&Path>,
    pretty: bool,
    action: impl FnOnce(&mut Project) -> lkjscript::Result<ProjectResult>,
) -> Result<(ProjectResult, bool), (LkError, bool)> {
    let mut project = Project::open(path).map_err(|error| (error, pretty))?;
    action(&mut project)
        .map(|result| (result, pretty))
        .map_err(|error| (error, pretty))
}

fn encode_error(error: LkError, pretty: bool) -> CliOutcome {
    let diagnostic = error.to_string();
    let exit = match error.code {
        ErrorCode::Io | ErrorCode::AuthorityBusy | ErrorCode::CommitOutcomeUnknown => {
            EXIT_TRANSPORT
        }
        ErrorCode::ArtifactCorrupt | ErrorCode::ArtifactPublicationOutcomeUnknown => EXIT_ARTIFACT,
        ErrorCode::PolicyExceeded
        | ErrorCode::ExecutionFuelExhausted
        | ErrorCode::ExecutionFrameExhausted
        | ErrorCode::ExecutionMemoryExhausted
        | ErrorCode::ManagedObjectPolicyExceeded
        | ErrorCode::ManagedVisibleBytePolicyExceeded
        | ErrorCode::RetainedBytePolicyExceeded
        | ErrorCode::ResultBytePolicyExceeded => EXIT_RESOURCE,
        _ => EXIT_USAGE_OR_JSON,
    };
    encode_result(ProjectResult::Error(error), pretty, exit, Some(diagnostic))
}

fn encode_result(
    result: ProjectResult,
    pretty: bool,
    exit: u8,
    diagnostic: Option<String>,
) -> CliOutcome {
    let envelope = ProjectEnvelope {
        version: PROJECT_MACHINE_VERSION,
        result,
    };
    let encoded = if pretty {
        serde_json::to_vec_pretty(&envelope)
    } else {
        serde_json::to_vec(&envelope)
    };
    match encoded {
        Ok(stdout) if stdout.len() <= MAX_JSON_OUTPUT_BYTES => CliOutcome {
            stdout,
            diagnostic,
            exit,
        },
        Ok(_) => CliOutcome {
            stdout: b"{\"version\":1,\"result\":{\"kind\":\"error\",\"data\":{\"code\":\"policy_exceeded\",\"related\":[],\"retryable\":false,\"message\":\"project response exceeds output byte policy\"}}}".to_vec(),
            diagnostic: Some("project response exceeds output byte policy".into()),
            exit: EXIT_OUTPUT,
        },
        Err(error) => CliOutcome {
            stdout: b"{\"version\":1,\"result\":{\"kind\":\"error\",\"data\":{\"code\":\"protocol_malformed\",\"related\":[],\"retryable\":false,\"message\":\"cannot encode project response\"}}}".to_vec(),
            diagnostic: Some(format!("cannot encode project response: {error}")),
            exit: EXIT_OUTPUT,
        },
    }
}

fn parse_init(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut path = None;
    let mut pretty = false;
    for argument in arguments {
        match argument.as_str() {
            "--pretty" | "--human" if !pretty => pretty = true,
            value if !value.starts_with('-') && path.is_none() => {
                path = Some(PathBuf::from(value));
            }
            _ => return Err(project_usage("init accepts one optional path")),
        }
    }
    Ok(ProjectCommand::Init {
        path: path.unwrap_or_else(|| PathBuf::from(".")),
        pretty,
    })
}

fn parse_orient(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    let known_digest = options.value("--known-digest")?.map(str::to_owned);
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Orient {
        project,
        known_digest,
        pretty,
    })
}

fn parse_status(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Status { project, pretty })
}

fn parse_inspect(arguments: &[String]) -> Result<ProjectCommand, String> {
    let (selector, rest) = positional(arguments, "inspect requires one selector")?;
    let mut options = Options::new(rest);
    let project = options.project()?;
    let revision = options.revision("--at")?;
    let expand = !options.flag("--summary")?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Inspect {
        project,
        selector: selector.to_owned(),
        revision,
        expand,
        pretty,
    })
}

fn parse_context(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    let purpose = options
        .value("--purpose")?
        .ok_or_else(|| project_usage("context requires --purpose"))?
        .parse::<ContextPurpose>()
        .map_err(project_usage)?;
    let targets = options.values("--target")?;
    let revision = options.revision("--at")?;
    let from_revision = options.revision("--from")?;
    let maximum_nodes = options.u32("--max-nodes")?;
    let known_digest = options
        .value("--known-digest")?
        .map(|value| {
            value
                .parse::<ContextPacketDigest>()
                .map_err(|error| project_usage(&error.to_string()))
        })
        .transpose()?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Context {
        project,
        purpose,
        targets,
        revision,
        from_revision,
        maximum_nodes,
        known_digest,
        pretty,
    })
}

fn parse_change(arguments: &[String]) -> Result<ProjectCommand, String> {
    let Some(action) = arguments.first() else {
        return Err(project_usage("change requires validate or apply"));
    };
    let mode = match action.as_str() {
        "validate" => TransactionMode::ValidateOnly,
        "apply" => TransactionMode::Commit,
        _ => return Err(project_usage("change requires validate or apply")),
    };
    let mut options = Options::new(&arguments[1..]);
    let project = options.project()?;
    let document = options.flag("--document")?;
    let context = options.value("--context")?.map(PathBuf::from);
    if context.is_some() && !document {
        return Err(project_usage("--context requires --document"));
    }
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Change {
        project,
        mode,
        input: if document {
            ChangeInput::Document { context }
        } else {
            ChangeInput::Json
        },
        pretty,
    })
}

fn parse_log(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    let before = options.revision("--before")?;
    let limit = options.u32("--limit")?.unwrap_or(32);
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Log {
        project,
        before,
        limit,
        pretty,
    })
}

fn parse_show(arguments: &[String]) -> Result<ProjectCommand, String> {
    let (revision, rest) = positional(arguments, "show requires one revision")?;
    let revision = parse_revision(revision)?;
    let mut options = Options::new(rest);
    let project = options.project()?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Show {
        project,
        revision,
        pretty,
    })
}

fn parse_diff(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    let from = options
        .revision("--from")?
        .ok_or_else(|| project_usage("diff requires --from"))?;
    let to = options
        .revision("--to")?
        .ok_or_else(|| project_usage("diff requires --to"))?;
    let offset = options.u64("--offset")?.unwrap_or(0);
    let limit = options.u32("--limit")?.unwrap_or(256);
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Diff {
        project,
        from,
        to,
        offset,
        limit,
        pretty,
    })
}

fn parse_restore(arguments: &[String]) -> Result<ProjectCommand, String> {
    let (source, rest) = positional(arguments, "restore requires one historical revision")?;
    let source_revision = parse_revision(source)?;
    let mut options = Options::new(rest);
    let project = options.project()?;
    let validate_only = options.flag("--validate")?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Restore {
        project,
        source_revision,
        validate_only,
        pretty,
    })
}

fn parse_target(arguments: &[String]) -> Result<ProjectCommand, String> {
    let Some(action) = arguments.first().map(String::as_str) else {
        return Err(project_usage(
            "target requires list, show, build, test, or run",
        ));
    };
    match action {
        "list" => {
            let mut options = Options::new(&arguments[1..]);
            let project = options.project()?;
            let revision = options.revision("--at")?;
            let pretty = options.pretty()?;
            options.finish()?;
            Ok(ProjectCommand::TargetList {
                project,
                revision,
                pretty,
            })
        }
        "show" | "build" | "test" | "run" => {
            let (selector, rest) = positional(
                &arguments[1..],
                "target action requires one target selector",
            )?;
            let mut options = Options::new(rest);
            let project = options.project()?;
            let revision = options.revision("--at")?;
            let output = if action == "build" {
                Some(
                    options
                        .value("--output")?
                        .map(PathBuf::from)
                        .ok_or_else(|| project_usage("target build requires --output"))?,
                )
            } else {
                None
            };
            let pretty = options.pretty()?;
            options.finish()?;
            match action {
                "show" => Ok(ProjectCommand::TargetShow {
                    project,
                    selector: selector.to_owned(),
                    revision,
                    pretty,
                }),
                "build" => Ok(ProjectCommand::TargetBuild {
                    project,
                    selector: selector.to_owned(),
                    revision,
                    output: output
                        .ok_or_else(|| project_usage("target build requires --output"))?,
                    pretty,
                }),
                "test" => Ok(ProjectCommand::TargetTest {
                    project,
                    selector: selector.to_owned(),
                    revision,
                    pretty,
                }),
                "run" => Ok(ProjectCommand::TargetRun {
                    project,
                    selector: selector.to_owned(),
                    revision,
                    pretty,
                }),
                _ => Err(project_usage("unknown target action")),
            }
        }
        _ => Err(project_usage("unknown target action")),
    }
}

fn parse_doctor(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    let deep = options.flag("--deep")?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Doctor {
        project,
        deep,
        pretty,
    })
}

fn parse_backup(arguments: &[String]) -> Result<ProjectCommand, String> {
    let (destination, rest) = positional(arguments, "backup requires one destination")?;
    let mut options = Options::new(rest);
    let project = options.project()?;
    let pretty = options.pretty()?;
    options.finish()?;
    Ok(ProjectCommand::Backup {
        project,
        destination: PathBuf::from(destination),
        pretty,
    })
}

fn parse_project_session(arguments: &[String]) -> Result<ProjectCommand, String> {
    let mut options = Options::new(arguments);
    let project = options.project()?;
    options.finish()?;
    Ok(ProjectCommand::Session { project })
}

struct Options<'a> {
    arguments: &'a [String],
    used: Vec<bool>,
}

impl<'a> Options<'a> {
    fn new(arguments: &'a [String]) -> Self {
        Self {
            arguments,
            used: vec![false; arguments.len()],
        }
    }

    fn project(&mut self) -> Result<Option<PathBuf>, String> {
        self.value("--project")
            .map(|value| value.map(PathBuf::from))
    }

    fn pretty(&mut self) -> Result<bool, String> {
        let pretty = self.flag("--pretty")?;
        let human = self.flag("--human")?;
        if pretty && human {
            return Err(project_usage(
                "--pretty and --human are aliases and cannot repeat",
            ));
        }
        Ok(pretty || human)
    }

    fn revision(&mut self, name: &str) -> Result<Option<Revision>, String> {
        self.value(name)?.map(parse_revision).transpose()
    }

    fn u32(&mut self, name: &str) -> Result<Option<u32>, String> {
        self.value(name)?
            .map(|value| parse_canonical_integer::<u32>(value, name))
            .transpose()
    }

    fn u64(&mut self, name: &str) -> Result<Option<u64>, String> {
        self.value(name)?
            .map(|value| parse_canonical_integer::<u64>(value, name))
            .transpose()
    }

    fn flag(&mut self, name: &str) -> Result<bool, String> {
        let matches = self
            .arguments
            .iter()
            .enumerate()
            .filter(|(index, value)| !self.used[*index] && value.as_str() == name)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            return Err(project_usage("duplicate option"));
        }
        if let Some(index) = matches.first().copied() {
            self.used[index] = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn value(&mut self, name: &str) -> Result<Option<&'a str>, String> {
        let matches = self
            .arguments
            .iter()
            .enumerate()
            .filter(|(index, value)| !self.used[*index] && value.as_str() == name)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            return Err(project_usage("duplicate option"));
        }
        let Some(index) = matches.first().copied() else {
            return Ok(None);
        };
        let value_index = index.saturating_add(1);
        if value_index >= self.arguments.len()
            || self.used[value_index]
            || self.arguments[value_index].starts_with("--")
        {
            return Err(project_usage("option is missing its value"));
        }
        self.used[index] = true;
        self.used[value_index] = true;
        Ok(Some(self.arguments[value_index].as_str()))
    }

    fn values(&mut self, name: &str) -> Result<Vec<String>, String> {
        let mut values = Vec::new();
        for index in 0..self.arguments.len() {
            if self.used[index] || self.arguments[index] != name {
                continue;
            }
            let value_index = index.saturating_add(1);
            if value_index >= self.arguments.len()
                || self.used[value_index]
                || self.arguments[value_index].starts_with("--")
            {
                return Err(project_usage("option is missing its value"));
            }
            self.used[index] = true;
            self.used[value_index] = true;
            values.push(self.arguments[value_index].clone());
        }
        Ok(values)
    }

    fn finish(&self) -> Result<(), String> {
        if self.used.iter().all(|used| *used) {
            Ok(())
        } else {
            Err(project_usage("unknown, duplicate, or positional argument"))
        }
    }
}

fn positional<'a>(
    arguments: &'a [String],
    message: &str,
) -> Result<(&'a str, &'a [String]), String> {
    let Some(first) = arguments.first() else {
        return Err(project_usage(message));
    };
    if first.starts_with('-') {
        return Err(project_usage(message));
    }
    Ok((first, &arguments[1..]))
}

fn parse_revision(value: &str) -> Result<Revision, String> {
    parse_canonical_integer::<u64>(value, "revision").map(Revision::new)
}

fn parse_canonical_integer<T>(value: &str, label: &str) -> Result<T, String>
where
    T: std::str::FromStr + ToString,
{
    let parsed = value
        .parse::<T>()
        .map_err(|_| project_usage(&format!("{label} must be a canonical integer")))?;
    if parsed.to_string() != value {
        return Err(project_usage(&format!(
            "{label} must be a canonical integer"
        )));
    }
    Ok(parsed)
}

fn read_stdin(maximum: usize, label: &str) -> Result<Vec<u8>, LkError> {
    let mut input = Vec::new();
    std::io::stdin()
        .lock()
        .take(maximum as u64 + 1)
        .read_to_end(&mut input)
        .map_err(|error| LkError::new(ErrorCode::Io, format!("cannot read {label}: {error}")))?;
    if input.len() > maximum {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds input byte policy"),
        ));
    }
    Ok(input)
}

fn read_session_line(reader: &mut impl BufRead, maximum: usize) -> std::io::Result<SessionLine> {
    let mut bytes = Vec::new();
    let mut too_large = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if bytes.is_empty() && !too_large {
                Ok(SessionLine::End)
            } else if too_large {
                Ok(SessionLine::TooLarge)
            } else {
                Ok(SessionLine::Data(bytes))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.unwrap_or(available.len());
        if !too_large {
            if bytes.len().saturating_add(count) > maximum {
                too_large = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..count]);
            }
        }
        let consumed = count + usize::from(newline.is_some());
        reader.consume(consumed);
        if newline.is_some() {
            return if too_large {
                Ok(SessionLine::TooLarge)
            } else {
                Ok(SessionLine::Data(bytes))
            };
        }
    }
}

fn decode_session_request(bytes: &[u8]) -> Result<ProjectSessionRequestEnvelope, LkError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope =
        ProjectSessionRequestEnvelope::deserialize(&mut deserializer).map_err(|error| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                format!("project session request is malformed: {error}"),
            )
        })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("project session request has trailing input: {error}"),
        )
    })?;
    if envelope.version != PROJECT_SESSION_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "project session request version is unsupported",
        ));
    }
    Ok(envelope)
}

fn encode_session_response(request_id: Option<u64>, result: ProjectResult) -> Vec<u8> {
    let response = ProjectSessionResponseEnvelope {
        version: PROJECT_SESSION_VERSION,
        request_id,
        result,
    };
    let mut bytes = match serde_json::to_vec(&response) {
        Ok(bytes) if bytes.len() <= MAX_JSON_OUTPUT_BYTES => bytes,
        _ => b"{\"version\":1,\"result\":{\"kind\":\"error\",\"data\":{\"code\":\"policy_exceeded\",\"related\":[],\"retryable\":false,\"message\":\"project session response exceeds output byte policy\"}}}".to_vec(),
    };
    bytes.push(b'\n');
    bytes
}

fn write_session_response(bytes: Vec<u8>, failure: bool) -> std::process::ExitCode {
    if let Err(error) = std::io::stdout().lock().write_all(&bytes) {
        eprintln!("cannot write project session response: {error}");
        return std::process::ExitCode::from(EXIT_OUTPUT);
    }
    if failure {
        std::process::ExitCode::from(EXIT_TRANSPORT)
    } else {
        std::process::ExitCode::SUCCESS
    }
}

fn document_error(error: lkjscript::workbench::DocumentError) -> LkError {
    LkError::new(
        match error.code {
            lkjscript::workbench::DocumentErrorCode::UnsupportedVersion
            | lkjscript::workbench::DocumentErrorCode::StaleSchema => ErrorCode::ProtocolVersion,
            lkjscript::workbench::DocumentErrorCode::PacketMismatch
            | lkjscript::workbench::DocumentErrorCode::PacketRequired
            | lkjscript::workbench::DocumentErrorCode::UnknownAlias
            | lkjscript::workbench::DocumentErrorCode::ScopeMismatch => ErrorCode::InvalidQuery,
            _ => ErrorCode::ProtocolMalformed,
        },
        format!(
            "editable semantic document {}: {}",
            error.code.machine_name(),
            error
        ),
    )
}

fn decode_json<T>(bytes: &[u8], label: &str) -> Result<T, LkError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} JSON is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} JSON has trailing input: {error}"),
        )
    })?;
    Ok(value)
}

fn read_context(path: &Path) -> Result<ContextPacket, LkError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot inspect context capsule: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::InvalidQuery,
            "context capsule path must name one regular non-symlink file",
        ));
    }
    if metadata.len() > lkjscript::workbench::MAX_CONTEXT_PACKET_BYTES as u64 {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "context capsule exceeds byte policy",
        ));
    }
    let bytes = std::fs::read(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot read context capsule: {error}"),
        )
    })?;
    if let Ok(packet) = decode_context_packet(&bytes) {
        return Ok(packet);
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SavedContextEnvelope {
        version: u16,
        result: SavedContextResult,
    }
    #[derive(Deserialize)]
    #[serde(
        tag = "kind",
        content = "data",
        rename_all = "snake_case",
        deny_unknown_fields
    )]
    enum SavedContextResult {
        Context(ProjectContextResult),
    }
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    let envelope = SavedContextEnvelope::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("saved context response is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("saved context response has trailing input: {error}"),
        )
    })?;
    if envelope.version != PROJECT_MACHINE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "saved context response version is unsupported",
        ));
    }
    match envelope.result {
        SavedContextResult::Context(ProjectContextResult::Changed(packet)) => Ok(*packet),
        SavedContextResult::Context(ProjectContextResult::Unchanged { .. }) => Err(LkError::new(
            ErrorCode::InvalidQuery,
            "an unchanged context receipt does not contain a reusable context capsule",
        )),
    }
}

fn canonical_output_path(path: &Path) -> Result<PathBuf, LkError> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(LkError::new(
            ErrorCode::Io,
            "target output path cannot contain parent traversal",
        ));
    }
    let mut output = if path.is_absolute() {
        PathBuf::from("/")
    } else {
        std::env::current_dir()?
    };
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(part) => output.push(part),
            Component::ParentDir | Component::Prefix(_) => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "target output path is not canonical",
                ));
            }
        }
    }
    Ok(output)
}

fn project_usage(reason: &str) -> String {
    format!(
        "{reason}; usage: lkjscript init [PROJECT] | orient|status [--project PROJECT] | inspect SELECTOR [--at N] | context --purpose PURPOSE [--target SELECTOR ...] [--known-digest DIGEST] | change validate|apply [--document [--context FILE]] | log [--before N] [--limit N] | show REVISION | diff --from N --to N | restore REVISION [--validate] | target list|show|build|test|run ... | doctor [--deep] | backup DESTINATION; all project commands accept --pretty"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_parser_rejects_duplicate_and_noncanonical_options() {
        assert!(parse("status", ["--pretty".into(), "--pretty".into()].into_iter()).is_err());
        assert!(parse("show", ["01".into()].into_iter()).is_err());
        assert!(parse("diff", ["--from".into(), "0".into()].into_iter()).is_err());
        assert!(matches!(
            parse("init", Vec::<String>::new().into_iter()),
            Ok(ProjectCommand::Init { .. })
        ));
    }
}
