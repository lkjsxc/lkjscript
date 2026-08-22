//! Strict, bounded public semantic command projection.

use super::artifact::{MAXIMUM_ARTIFACT_BYTES, load_artifact};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{PreparedProgram, ReferenceInterpreter, RunPolicy, Vm};
use super::json::{JsonLimits, decode_strict, decode_typed, encode_typed};
use super::meaning::{GRAPH_CONTRACT_IDENTITY, RelationRole};
use super::package::RunnerKind;
use super::repository::{MAXIMUM_BACKUP_BYTES, SemanticRepository};
use super::revision::{AffectedOwner, TransactionReceipt, ValidationFacts};
use super::semantic_diff::diff_revisions;
use super::semantic_digest::{ReceiptDigest, SemanticDiffDigest, TransactionDigest};
use super::semantic_draft::SemanticDraftStore;
use super::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, DraftId, ExpressionId,
    FieldId, ModuleId, OperationId, ParameterId, PortId, RepositoryId, RequirementId, RevisionId,
    TargetId,
};
use super::semantic_merge::{
    SEMANTIC_MERGE_CONTRACT_VERSION, SemanticMergeRequest, SemanticMergeResult,
    SemanticMergeStatus, merge_revisions,
};
use super::semantic_projection::{MAXIMUM_REVIEW_PROJECTION_BYTES, render_review_projection};
use super::semantic_query::{OwnerKind, QueryBudget, SemanticQueryIndex};
use super::semantic_transaction::{
    TransactionMode, TransactionRequest, TransactionResult, TransactionStatus, execute_transaction,
};
use super::workspace::{DEFAULT_ORIENTATION_ITEMS, SemanticWorkspace};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const CLI_CONTRACT_VERSION: u16 = 2;
pub const MAXIMUM_CLI_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_TRANSACTION_REQUEST_BYTES: usize = 16 * 1_048_576;
const MAXIMUM_INLINE_AFFECTED_OWNERS: usize = 64;

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CliSuccess {
    pub contract_version: u16,
    pub ok: bool,
    pub status: &'static str,
    pub command: String,
    pub result: serde_json::Value,
}

impl CliSuccess {
    pub fn process_exit_code(&self) -> u8 {
        match self.status {
            "stale_base" | "stale_head" => 7,
            "invalid_graph" => 8,
            "resource_exhausted" => 4,
            "precondition_failed" | "foreign_identity" | "conflicted" => 2,
            _ => 0,
        }
    }
}

pub fn execute(arguments: Vec<String>) -> Result<CliSuccess, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    let command = arguments.first().map(String::as_str).unwrap_or("help");
    match command {
        "help" => {
            exact_arguments(&arguments, 1, "help")?;
            semantic_help(None)
        }
        "semantic" => semantic_command(&arguments[1..], project.as_deref()),
        other => Err(usage_error(format!(
            "unknown command '{other}'; current development commands are under 'semantic'"
        ))),
    }
}

fn semantic_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let subcommand = arguments.first().map(String::as_str).unwrap_or("help");
    match subcommand {
        "help" => {
            if arguments.len() > 2 {
                return Err(usage_error("semantic help accepts at most one command"));
            }
            semantic_help(arguments.get(1).map(String::as_str))
        }
        "schema" => {
            exact_arguments(arguments, 1, "semantic schema")?;
            success(
                "semantic.schema",
                json!({
                    "graph_contract": GRAPH_CONTRACT_IDENTITY,
                    "cli_contract_version": CLI_CONTRACT_VERSION,
                    "schema_digest": command_schema_digest(),
                    "owner_kinds": owner_kind_names(),
                    "relation_roles": relation_role_names(),
                    "expansion_command": "lkjscript semantic help <command>",
                }),
            )
        }
        "id-allocate" => id_allocate_command(&arguments[1..]),
        "dependency-stage" => dependency_stage_command(&arguments[1..], project),
        "import" => import_command(&arguments[1..], project),
        "status" => {
            exact_arguments(arguments, 1, "semantic status")?;
            let workspace = open_workspace(project)?;
            serialized("semantic.status", &workspace.status()?)
        }
        "orient" => {
            let limit = optional_usize(&arguments[1..], "--limit", DEFAULT_ORIENTATION_ITEMS)?;
            ensure_options(&arguments[1..], &["--limit"], &[])?;
            let workspace = open_workspace(project)?;
            serialized("semantic.orient", &workspace.orient(limit)?)
        }
        "owners" => owners_command(&arguments[1..], project),
        "find" => find_command(&arguments[1..], project),
        "show" => show_command(&arguments[1..], project),
        "refs" => relation_command(
            "semantic.refs",
            &arguments[1..],
            project,
            true,
            true,
            BTreeSet::new(),
        ),
        "callers" => relation_command(
            "semantic.callers",
            &arguments[1..],
            project,
            true,
            false,
            BTreeSet::from([RelationRole::Call]),
        ),
        "callees" => relation_command(
            "semantic.callees",
            &arguments[1..],
            project,
            false,
            true,
            BTreeSet::from([RelationRole::Call]),
        ),
        "type-uses" => relation_command(
            "semantic.type_uses",
            &arguments[1..],
            project,
            true,
            true,
            BTreeSet::from([
                RelationRole::TypeUse,
                RelationRole::FieldUse,
                RelationRole::VariantConstruction,
                RelationRole::VariantPattern,
            ]),
        ),
        "capability-uses" => relation_command(
            "semantic.capability_uses",
            &arguments[1..],
            project,
            true,
            true,
            BTreeSet::from([
                RelationRole::CapabilityInterface,
                RelationRole::CapabilityOperation,
            ]),
        ),
        "context" => context_command(&arguments[1..], project, false),
        "impact" => context_command(&arguments[1..], project, true),
        "query" => query_command(&arguments[1..], project),
        "diff" => diff_command(&arguments[1..], project),
        "merge" => merge_command(&arguments[1..], project),
        "plan" => transaction_command(&arguments[1..], project, TransactionMode::Plan),
        "validate" => transaction_command(&arguments[1..], project, TransactionMode::Validate),
        "apply" => transaction_command(&arguments[1..], project, TransactionMode::Apply),
        "draft-create" => draft_create_command(&arguments[1..], project),
        "draft-status" => draft_status_command(&arguments[1..], project),
        "draft-drop" => draft_drop_command(&arguments[1..], project),
        "draft-rebase" => draft_rebase_command(&arguments[1..], project),
        "draft-publish" => draft_publish_command(&arguments[1..], project),
        "targets" => targets_command(&arguments[1..], project),
        "build" => build_command(&arguments[1..], project),
        "test" => {
            exact_arguments(arguments, 1, "semantic test")?;
            let workspace = open_workspace(project)?;
            run_package_tests(&workspace.prepare()?)
        }
        "run" => run_target_command(&arguments[1..], project),
        "artifact-inspect" => artifact_inspect_command(&arguments[1..]),
        "text-project" | "export-text" => {
            text_projection_command(subcommand, &arguments[1..], project)
        }
        "history" => history_command(&arguments[1..], project),
        "revision-show" => revision_show_command(&arguments[1..], project),
        "doctor" => doctor_command(&arguments[1..], project),
        "backup" | "export-bundle" => backup_command(subcommand, &arguments[1..], project),
        "restore" => restore_command(&arguments[1..], project),
        other => Err(usage_error(format!(
            "unknown semantic command '{other}'; use 'semantic help'"
        ))),
    }
}

fn semantic_help(command: Option<&str>) -> Result<CliSuccess, Diagnostic> {
    let commands = command_registry();
    if let Some(command) = command {
        let entry = commands
            .iter()
            .find(|entry| entry.name == command)
            .ok_or_else(|| usage_error(format!("unknown semantic command '{command}'")))?;
        return serialized("semantic.help", entry);
    }
    success(
        "semantic.help",
        json!({
            "usage": "lkjscript [--project PATH] semantic <command> [options]",
            "graph_contract": GRAPH_CONTRACT_IDENTITY,
            "schema_digest": command_schema_digest(),
            "commands": commands.iter().map(|entry| entry.name).collect::<Vec<_>>(),
            "details": "lkjscript semantic help <command>",
        }),
    )
}

#[derive(Serialize)]
struct CommandHelp {
    name: &'static str,
    purpose: &'static str,
    usage: &'static str,
    mutates_authority: bool,
}

fn command_registry() -> Vec<CommandHelp> {
    vec![
        CommandHelp {
            name: "status",
            purpose: "Show exact canonical authority and health.",
            usage: "semantic status",
            mutates_authority: false,
        },
        CommandHelp {
            name: "orient",
            purpose: "Return a compact package, module, dependency, and target map.",
            usage: "semantic orient [--limit N]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "schema",
            purpose: "Return command and graph contract identities.",
            usage: "semantic schema",
            mutates_authority: false,
        },
        CommandHelp {
            name: "id-allocate",
            purpose: "Allocate opaque stable IDs in one exact semantic domain.",
            usage: "semantic id-allocate DOMAIN [--count N]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "dependency-stage",
            purpose: "Validate and stage an immutable dependency artifact before a transaction.",
            usage: "semantic dependency-stage PATH",
            mutates_authority: false,
        },
        CommandHelp {
            name: "owners",
            purpose: "List bounded semantic owners.",
            usage: "semantic owners [--kind KIND] [--module ID] [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "find",
            purpose: "Find bounded owners by identity or name.",
            usage: "semantic find TEXT [--exact] [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "show",
            purpose: "Show one exact owner.",
            usage: "semantic show ID [--body] [--revision REV]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "refs",
            purpose: "Show incoming and outgoing semantic relations.",
            usage: "semantic refs ID [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "callers",
            purpose: "Show direct callers.",
            usage: "semantic callers ID [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "callees",
            purpose: "Show direct callees.",
            usage: "semantic callees ID [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "type-uses",
            purpose: "Show exact semantic type uses.",
            usage: "semantic type-uses ID [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "capability-uses",
            purpose: "Show capability requirements and operation uses.",
            usage: "semantic capability-uses ID [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "context",
            purpose: "Build a task-scoped semantic context bundle.",
            usage: "semantic context --seed ID... [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "impact",
            purpose: "Calculate conservative relation impact.",
            usage: "semantic impact --seed ID... [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "query",
            purpose: "Execute one closed declarative query request.",
            usage: "semantic query --request JSON",
            mutates_authority: false,
        },
        CommandHelp {
            name: "diff",
            purpose: "Compare exact revisions by stable semantic identity.",
            usage: "semantic diff --base REV --result REV [--offset N] [query budgets]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "merge",
            purpose: "Preview or publish one exact three-way semantic merge.",
            usage: "semantic merge --base REV --left REV --right REV [--apply] [--work N]",
            mutates_authority: true,
        },
        CommandHelp {
            name: "plan",
            purpose: "Plan one exact-base semantic transaction without publication.",
            usage: "semantic plan (--request JSON | --request-file PATH)",
            mutates_authority: false,
        },
        CommandHelp {
            name: "validate",
            purpose: "Validate one exact-base semantic transaction without publication.",
            usage: "semantic validate (--request JSON | --request-file PATH)",
            mutates_authority: false,
        },
        CommandHelp {
            name: "apply",
            purpose: "Atomically publish one exact-base semantic transaction.",
            usage: "semantic apply (--request JSON | --request-file PATH)",
            mutates_authority: true,
        },
        CommandHelp {
            name: "draft-create",
            purpose: "Create non-executable draft authority from one exact accepted base.",
            usage: "semantic draft-create [--base REV] [--intent TEXT]",
            mutates_authority: true,
        },
        CommandHelp {
            name: "draft-status",
            purpose: "Show one draft base, generation, operations, holes, and conflicts.",
            usage: "semantic draft-status DRAFT",
            mutates_authority: false,
        },
        CommandHelp {
            name: "draft-drop",
            purpose: "Discard one draft without changing accepted meaning.",
            usage: "semantic draft-drop DRAFT",
            mutates_authority: true,
        },
        CommandHelp {
            name: "draft-rebase",
            purpose: "Validate and move one draft onto the exact current revision.",
            usage: "semantic draft-rebase DRAFT --base REV",
            mutates_authority: true,
        },
        CommandHelp {
            name: "draft-publish",
            purpose: "Validate and publish one resolved draft, then remove it.",
            usage: "semantic draft-publish DRAFT [--idempotency-key KEY]",
            mutates_authority: true,
        },
        CommandHelp {
            name: "targets",
            purpose: "List bounded executable targets.",
            usage: "semantic targets [--limit N]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "build",
            purpose: "Build a deterministic graph-native artifact.",
            usage: "semantic build [--output PATH]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "test",
            purpose: "Compare bytecode and the semantic oracle for graph-owned tests.",
            usage: "semantic test",
            mutates_authority: false,
        },
        CommandHelp {
            name: "run",
            purpose: "Run a pure command, batch, or test target through both execution tiers.",
            usage: "semantic run TARGET [--arguments JSON]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "artifact-inspect",
            purpose: "Inspect a graph-native artifact without source parsing.",
            usage: "semantic artifact-inspect PATH",
            mutates_authority: false,
        },
        CommandHelp {
            name: "text-project",
            purpose: "Write a deterministic non-authoritative review projection.",
            usage: "semantic text-project [--revision REV] [--output PATH]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "export-text",
            purpose: "Export the deterministic non-authoritative review projection.",
            usage: "semantic export-text [--revision REV] [--output PATH]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "history",
            purpose: "Show bounded accepted semantic history.",
            usage: "semantic history [--before REV] [--limit N]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "revision-show",
            purpose: "Inspect one exact semantic revision record.",
            usage: "semantic revision-show REV",
            mutates_authority: false,
        },
        CommandHelp {
            name: "doctor",
            purpose: "Validate current or complete retained authority.",
            usage: "semantic doctor [--deep]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "backup",
            purpose: "Capture one exact reachable canonical authority bundle.",
            usage: "semantic backup [--output PATH]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "export-bundle",
            purpose: "Export one exact independently verifiable recovery bundle.",
            usage: "semantic export-bundle [--output PATH]",
            mutates_authority: false,
        },
        CommandHelp {
            name: "restore",
            purpose: "Verify and atomically restore a canonical authority bundle.",
            usage: "semantic restore --backup PATH (--output PROJECT | --project PROJECT)",
            mutates_authority: true,
        },
        CommandHelp {
            name: "import",
            purpose: "Create new graph authority from a current graph-native artifact.",
            usage: "semantic import --artifact PATH",
            mutates_authority: true,
        },
    ]
}

fn command_schema_digest() -> String {
    let bytes = serde_json::to_vec(&command_registry()).unwrap_or_default();
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-cli-registry.v1");
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    hasher.finalize().to_hex().to_string()
}

fn owner_kind_names() -> Vec<&'static str> {
    vec![
        "repository",
        "package",
        "module",
        "record",
        "variant",
        "interface",
        "external",
        "pure_function",
        "task_function",
        "constant",
        "component",
        "test",
        "field",
        "case",
        "operation",
        "parameter",
        "binding",
        "expression",
        "requirement",
        "port",
        "target",
        "documentation",
        "annotation",
    ]
}

fn relation_role_names() -> Vec<&'static str> {
    vec![
        "import",
        "export",
        "type_use",
        "value_reference",
        "call",
        "field_use",
        "variant_construction",
        "variant_pattern",
        "capability_interface",
        "capability_operation",
        "component_port_function",
        "target_component",
        "target_port",
        "test_dependency",
    ]
}

fn id_allocate_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    let domain = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("semantic id-allocate requires one identity domain"))?;
    ensure_options(&arguments[1..], &["--count"], &[])?;
    let count = optional_usize(&arguments[1..], "--count", 1)?;
    if count == 0 || count > 1_000 {
        return Err(usage_error(
            "identity allocation count must be 1 through 1000",
        ));
    }
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        let id = match domain.as_str() {
            "repository" => RepositoryId::generate()?.to_string(),
            "module" => ModuleId::generate()?.to_string(),
            "declaration" => DeclarationId::generate()?.to_string(),
            "field" => FieldId::generate()?.to_string(),
            "case" => CaseId::generate()?.to_string(),
            "operation" => OperationId::generate()?.to_string(),
            "parameter" => ParameterId::generate()?.to_string(),
            "binding" => BindingId::generate()?.to_string(),
            "expression" => ExpressionId::generate()?.to_string(),
            "requirement" => RequirementId::generate()?.to_string(),
            "port" => PortId::generate()?.to_string(),
            "target" => TargetId::generate()?.to_string(),
            "documentation" => DocumentationId::generate()?.to_string(),
            "annotation" => AnnotationId::generate()?.to_string(),
            _ => {
                return Err(usage_error(format!(
                    "unknown semantic identity domain '{domain}'"
                )));
            }
        };
        ids.push(id);
    }
    success(
        "semantic.id_allocate",
        json!({
            "domain": domain,
            "count": ids.len(),
            "ids": ids,
        }),
    )
}

fn import_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let artifact = option_value(arguments, "--artifact")?
        .ok_or_else(|| usage_error("semantic import requires --artifact PATH"))?;
    ensure_options(arguments, &["--artifact"], &[])?;
    let root = project_or_current(project)?;
    let (workspace, receipt) =
        SemanticWorkspace::initialize_from_artifact(&root, Path::new(&artifact))?;
    success(
        "semantic.import",
        json!({
            "receipt": receipt,
            "status": workspace.status()?,
        }),
    )
}

fn dependency_stage_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error(
            "semantic dependency-stage requires one graph artifact path",
        ));
    }
    let bytes = read_bounded(
        Path::new(&arguments[0]),
        MAXIMUM_ARTIFACT_BYTES + 50,
        "graph dependency artifact",
    )?;
    let artifact = load_artifact(&bytes)?;
    let workspace = open_workspace(project)?;
    let mut staged = 0usize;
    for digest in artifact.package_artifacts.values() {
        let package = artifact.package_object(*digest).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Corrupt,
                "semantic_dependency_object_missing",
                "validated artifact closure lost one exact package object",
            )
        })?;
        workspace
            .repository()
            .write_artifact_object(*digest, package)?;
        staged = staged.checked_add(1).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "semantic_dependency_count",
                "dependency package object count overflowed",
            )
        })?;
    }
    success(
        "semantic.dependency_stage",
        json!({
            "status": "staged",
            "package_id": artifact.root_package_id,
            "semantic_revision": artifact.root_revision,
            "artifact": artifact.root_package_artifact,
            "objects": staged,
            "current_revision": workspace.status()?.revision,
        }),
    )
}

fn owners_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &query_value_options(["--kind", "--module"]), &[])?;
    let kind = option_value(arguments, "--kind")?
        .map(|value| OwnerKind::parse(&value))
        .transpose()?;
    let module = option_value(arguments, "--module")?
        .map(|value| value.parse::<ModuleId>())
        .transpose()?;
    let workspace = open_workspace(project)?;
    let index = query_index(&workspace, arguments)?;
    let page = index.owners(
        kind,
        module,
        option_value(arguments, "--continue")?.as_deref(),
        query_budget(arguments)?,
    )?;
    serialized("semantic.owners", &page)
}

fn find_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let text = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("semantic find requires one search text"))?;
    ensure_options(&arguments[1..], &query_value_options([]), &["--exact"])?;
    let workspace = open_workspace(project)?;
    let exact = flag_present(&arguments[1..], "--exact")?;
    let continuation = option_value(&arguments[1..], "--continue")?;
    let budget = query_budget(&arguments[1..])?;
    let page = if exact {
        let revision = selected_revision(&workspace, &arguments[1..])?;
        SemanticQueryIndex::exact_find_revision(
            workspace.repository(),
            revision,
            text,
            continuation.as_deref(),
            budget,
        )?
    } else {
        query_index(&workspace, &arguments[1..])?.find(
            text,
            false,
            continuation.as_deref(),
            budget,
        )?
    };
    serialized("semantic.find", &page)
}

fn show_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("semantic show requires one exact owner ID"))?;
    ensure_options(&arguments[1..], &["--revision"], &["--body"])?;
    let workspace = open_workspace(project)?;
    serialized(
        "semantic.show",
        &SemanticQueryIndex::show_revision(
            workspace.repository(),
            selected_revision(&workspace, &arguments[1..])?,
            id,
            flag_present(&arguments[1..], "--body")?,
        )?,
    )
}

fn relation_command(
    command: &str,
    arguments: &[String],
    project: Option<&Path>,
    incoming: bool,
    outgoing: bool,
    roles: BTreeSet<RelationRole>,
) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error(format!("{command} requires one exact owner ID")))?;
    ensure_options(&arguments[1..], &query_value_options([]), &[])?;
    let workspace = open_workspace(project)?;
    let index = query_index(&workspace, &arguments[1..])?;
    let page = index.relations(
        id,
        incoming,
        outgoing,
        &roles,
        option_value(&arguments[1..], "--continue")?.as_deref(),
        query_budget(&arguments[1..])?,
    )?;
    serialized(command, &page)
}

fn context_command(
    arguments: &[String],
    project: Option<&Path>,
    impact: bool,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &query_value_options(["--seed"]), &[])?;
    let seeds = option_values(arguments, "--seed")?;
    let workspace = open_workspace(project)?;
    let index = query_index(&workspace, arguments)?;
    let continuation = option_value(arguments, "--continue")?;
    let page = if impact {
        index.impact(&seeds, continuation.as_deref(), query_budget(arguments)?)?
    } else {
        index.context(&seeds, continuation.as_deref(), query_budget(arguments)?)?
    };
    serialized(
        if impact {
            "semantic.impact"
        } else {
            "semantic.context"
        },
        &page,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosedQuery {
    contract_version: u16,
    #[serde(default)]
    revision: Option<RevisionId>,
    #[serde(default)]
    continuation: Option<String>,
    #[serde(default)]
    budget: QueryBudget,
    #[serde(flatten)]
    selection: ClosedSelection,
}

#[derive(Deserialize)]
#[serde(tag = "select", rename_all = "snake_case")]
enum ClosedSelection {
    Owners {
        #[serde(default)]
        kind: Option<OwnerKind>,
        #[serde(default)]
        module: Option<ModuleId>,
    },
    Find {
        text: String,
        #[serde(default)]
        exact: bool,
    },
    Relations {
        id: String,
        incoming: bool,
        outgoing: bool,
        #[serde(default)]
        roles: BTreeSet<RelationRole>,
    },
    Context {
        seeds: Vec<String>,
    },
    Impact {
        seeds: Vec<String>,
    },
}

fn query_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let request = option_value(arguments, "--request")?
        .ok_or_else(|| usage_error("semantic query requires --request JSON"))?;
    ensure_options(arguments, &["--request"], &[])?;
    let mut deserializer = serde_json::Deserializer::from_str(&request);
    let request = ClosedQuery::deserialize(&mut deserializer).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_query_request",
            format!("query request is not strict current JSON: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_query_trailing",
            format!("query request has trailing input: {error}"),
        )
    })?;
    if request.contract_version != super::semantic_query::QUERY_CONTRACT_VERSION {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_query_contract",
            "query request uses an unknown contract",
        ));
    }
    let workspace = open_workspace(project)?;
    let index = match request.revision {
        Some(revision) => SemanticQueryIndex::revision(workspace.repository(), revision)?,
        None => SemanticQueryIndex::current(workspace.repository())?,
    };
    let result = match request.selection {
        ClosedSelection::Owners { kind, module } => serde_json::to_value(index.owners(
            kind,
            module,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Find { text, exact } => serde_json::to_value(index.find(
            &text,
            exact,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Relations {
            id,
            incoming,
            outgoing,
            roles,
        } => serde_json::to_value(index.relations(
            &id,
            incoming,
            outgoing,
            &roles,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Context { seeds } => serde_json::to_value(index.context(
            &seeds,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Impact { seeds } => serde_json::to_value(index.impact(
            &seeds,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
    };
    success("semantic.query", result)
}

fn transaction_command(
    arguments: &[String],
    project: Option<&Path>,
    mode: TransactionMode,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--request", "--request-file"], &[])?;
    let inline = option_value(arguments, "--request")?;
    let file = option_value(arguments, "--request-file")?;
    let bytes = match (inline, file) {
        (Some(value), None) => {
            if value.len() > MAXIMUM_TRANSACTION_REQUEST_BYTES {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "semantic_transaction_request_limit",
                    format!(
                        "transaction request exceeds {MAXIMUM_TRANSACTION_REQUEST_BYTES} bytes"
                    ),
                ));
            }
            value.into_bytes()
        }
        (None, Some(path)) => read_bounded(
            Path::new(&path),
            MAXIMUM_TRANSACTION_REQUEST_BYTES,
            "semantic transaction request",
        )?,
        (Some(_), Some(_)) => {
            return Err(usage_error(
                "supply exactly one of --request or --request-file",
            ));
        }
        (None, None) => {
            return Err(usage_error(
                "semantic transaction requires --request JSON or --request-file PATH",
            ));
        }
    };
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    let request = TransactionRequest::deserialize(&mut deserializer).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_transaction_request",
            format!("transaction request is not strict current JSON: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_transaction_trailing",
            format!("transaction request has trailing input: {error}"),
        )
    })?;
    let workspace = open_workspace(project)?;
    if request.draft.is_some() {
        let drafts = SemanticDraftStore::new(workspace.repository());
        return match mode {
            TransactionMode::Apply => serialized("semantic.apply", &drafts.append(&request)?),
            TransactionMode::Plan | TransactionMode::Validate => transaction_result(
                if mode == TransactionMode::Plan {
                    "semantic.plan"
                } else {
                    "semantic.validate"
                },
                &drafts.evaluate(&request, mode)?,
            ),
        };
    }
    let result = execute_transaction(workspace.repository(), &request, mode)?;
    transaction_result(
        match mode {
            TransactionMode::Plan => "semantic.plan",
            TransactionMode::Validate => "semantic.validate",
            TransactionMode::Apply => "semantic.apply",
        },
        &result,
    )
}

fn draft_create_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--base", "--intent"], &[])?;
    let base = option_value(arguments, "--base")?
        .map(|value| value.parse::<RevisionId>())
        .transpose()?;
    let intent = option_value(arguments, "--intent")?;
    let workspace = open_workspace(project)?;
    serialized(
        "semantic.draft_create",
        &SemanticDraftStore::new(workspace.repository()).create(base, intent)?,
    )
}

fn draft_status_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error("semantic draft-status requires one draft ID"));
    }
    let id = arguments[0].parse::<DraftId>()?;
    let workspace = open_workspace(project)?;
    serialized(
        "semantic.draft_status",
        &SemanticDraftStore::new(workspace.repository()).status(id)?,
    )
}

fn draft_drop_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error("semantic draft-drop requires one draft ID"));
    }
    let id = arguments[0].parse::<DraftId>()?;
    let workspace = open_workspace(project)?;
    SemanticDraftStore::new(workspace.repository()).drop(id)?;
    success(
        "semantic.draft_drop",
        json!({
            "status": "draft_dropped",
            "draft": id,
            "revision": workspace.repository().current()?.head.revision,
        }),
    )
}

fn draft_rebase_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("semantic draft-rebase requires one draft ID"))?
        .parse::<DraftId>()?;
    ensure_options(&arguments[1..], &["--base"], &[])?;
    let base = option_value(&arguments[1..], "--base")?
        .ok_or_else(|| usage_error("semantic draft-rebase requires --base REV"))?
        .parse::<RevisionId>()?;
    let workspace = open_workspace(project)?;
    serialized(
        "semantic.draft_rebase",
        &SemanticDraftStore::new(workspace.repository()).rebase(id, base)?,
    )
}

fn draft_publish_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("semantic draft-publish requires one draft ID"))?
        .parse::<DraftId>()?;
    ensure_options(&arguments[1..], &["--idempotency-key"], &[])?;
    let idempotency_key = option_value(&arguments[1..], "--idempotency-key")?;
    let workspace = open_workspace(project)?;
    transaction_result(
        "semantic.draft_publish",
        &SemanticDraftStore::new(workspace.repository()).publish(id, idempotency_key)?,
    )
}

fn targets_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let limit = optional_usize(arguments, "--limit", DEFAULT_ORIENTATION_ITEMS)?;
    ensure_options(arguments, &["--limit"], &[])?;
    let workspace = open_workspace(project)?;
    let orientation = workspace.orient(limit)?;
    success(
        "semantic.targets",
        json!({
            "revision": orientation.revision,
            "returned_items": orientation.targets.len(),
            "total_items": orientation.target_count,
            "truncated": orientation.target_count > orientation.targets.len(),
            "items": orientation.targets,
        }),
    )
}

fn build_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--output"], &[])?;
    let workspace = open_workspace(project)?;
    let output = option_value(arguments, "--output")?
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().join("target/application.lkja"));
    let (bytes, receipt) = workspace.build_artifact()?;
    let publication = write_derived_output(
        &output,
        &bytes,
        MAXIMUM_ARTIFACT_BYTES + 50,
        "graph artifact",
    )?;
    success(
        "semantic.build",
        json!({
            "receipt": receipt,
            "output": output.display().to_string(),
            "publication": publication,
        }),
    )
}

fn run_package_tests(program: &PreparedProgram) -> Result<CliSuccess, Diagnostic> {
    let mut production_instructions = 0u64;
    let mut oracle_instructions = 0u64;
    let mut production_elapsed_nanoseconds = 0u64;
    let mut oracle_elapsed_nanoseconds = 0u64;
    for test in program.tests() {
        let production_started = Instant::now();
        let (actual, actual_observation) = Vm::new(program, RunPolicy::default())
            .invoke_test_expression(&test.actual, Vec::new())
            .map_err(execution_diagnostic)?;
        let (expected, expected_observation) = Vm::new(program, RunPolicy::default())
            .invoke_test_expression(&test.expected, Vec::new())
            .map_err(execution_diagnostic)?;
        production_elapsed_nanoseconds = production_elapsed_nanoseconds.saturating_add(
            u64::try_from(production_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        );
        let oracle_started = Instant::now();
        let (oracle_actual, oracle_actual_observation) =
            ReferenceInterpreter::new(program, RunPolicy::default())
                .invoke_test_expression(&test.actual, Vec::new())
                .map_err(execution_diagnostic)?;
        let (oracle_expected, oracle_expected_observation) =
            ReferenceInterpreter::new(program, RunPolicy::default())
                .invoke_test_expression(&test.expected, Vec::new())
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
                "semantic_test_differential",
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
                "semantic_test_failed",
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
        "semantic.test",
        json!({
            "revision": program.artifact().root_revision,
            "passed": program.tests().len(),
            "failed": 0,
            "production_tier": "bytecode_v1",
            "oracle_tier": "semantic_reference_v1",
            "production_instructions": production_instructions,
            "oracle_instructions": oracle_instructions,
            "production_elapsed_nanoseconds": production_elapsed_nanoseconds,
            "oracle_elapsed_nanoseconds": oracle_elapsed_nanoseconds,
            "differential": "equal",
        }),
    )
}

fn artifact_inspect_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error(
            "semantic artifact-inspect requires one artifact path",
        ));
    }
    let bytes = read_bounded(
        Path::new(&arguments[0]),
        MAXIMUM_ARTIFACT_BYTES + 50,
        "graph artifact",
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
                "semantic_revision": package.accepted_revision,
                "package_artifact": program.artifact().package_artifacts.get(&package.descriptor.package_id),
                "modules": package.modules.iter().map(|module| json!({
                    "id": module.module_id,
                    "name": module.module.name,
                })).collect::<Vec<_>>(),
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
        "semantic.artifact_inspect",
        json!({
            "artifact_digest": program.artifact().artifact_digest,
            "root_package_artifact": program.artifact().root_package_artifact,
            "root_package_id": program.artifact().root_package_id,
            "root_revision": program.artifact().root_revision,
            "packages": packages,
            "targets": targets,
        }),
    )
}

fn text_projection_command(
    command: &str,
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--revision", "--output"], &[])?;
    let workspace = open_workspace(project)?;
    let revision = option_value(arguments, "--revision")?
        .map(|value| value.parse())
        .transpose()?;
    let output = option_value(arguments, "--output")?
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().join("target/meaning-review.json"));
    let (bytes, receipt) = render_review_projection(workspace.repository(), revision)?;
    let publication = write_derived_output(
        &output,
        &bytes,
        MAXIMUM_REVIEW_PROJECTION_BYTES + 1,
        "semantic review projection",
    )?;
    success(
        &format!("semantic.{}", command.replace('-', "_")),
        json!({
            "receipt": receipt,
            "output": output.display().to_string(),
            "publication": publication,
            "importable": false,
            "recovery_command": "lkjscript semantic export-bundle --output target/meaning-backup.lkjb",
        }),
    )
}

fn run_target_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let target_name = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("semantic run requires one target name"))?;
    ensure_options(&arguments[1..], &["--arguments"], &[])?;
    let encoded_arguments =
        option_value(&arguments[1..], "--arguments")?.unwrap_or_else(|| "[]".to_owned());
    let workspace = open_workspace(project)?;
    let program = workspace.prepare()?;
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
        "semantic.run",
        json!({
            "revision": program.artifact().root_revision,
            "target": target.name,
            "result": result,
            "production": production,
            "oracle": oracle,
            "differential": "equal",
        }),
    )
}

fn history_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--limit", "--before"], &[])?;
    let limit = optional_usize(arguments, "--limit", 50)?;
    let before = option_value(arguments, "--before")?
        .map(|value| value.parse::<RevisionId>())
        .transpose()?;
    let workspace = open_workspace(project)?;
    let records = workspace.repository().history(before, limit)?;
    success(
        "semantic.history",
        json!({
            "revision": workspace.status()?.revision,
            "returned_items": records.len(),
            "items": records,
        }),
    )
}

fn diff_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(
        arguments,
        &query_value_options(["--base", "--result", "--offset"]),
        &[],
    )?;
    let base = option_value(arguments, "--base")?
        .ok_or_else(|| usage_error("semantic diff requires --base REV"))?
        .parse::<RevisionId>()?;
    let result = option_value(arguments, "--result")?
        .ok_or_else(|| usage_error("semantic diff requires --result REV"))?
        .parse::<RevisionId>()?;
    let offset = optional_usize(arguments, "--offset", 0)?;
    let workspace = open_workspace(project)?;
    serialized(
        "semantic.diff",
        &diff_revisions(
            workspace.repository(),
            base,
            result,
            offset,
            query_budget(arguments)?,
        )?,
    )
}

fn merge_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(
        arguments,
        &["--base", "--left", "--right", "--work", "--intent"],
        &["--apply"],
    )?;
    let revision = |name: &str| -> Result<RevisionId, Diagnostic> {
        option_value(arguments, name)?
            .ok_or_else(|| usage_error(format!("semantic merge requires {name} REV")))?
            .parse()
    };
    let request = SemanticMergeRequest {
        contract_version: SEMANTIC_MERGE_CONTRACT_VERSION,
        base_revision: revision("--base")?,
        left_revision: revision("--left")?,
        right_revision: revision("--right")?,
        maximum_work: optional_usize(
            arguments,
            "--work",
            super::repository::MAXIMUM_HISTORY_ITEMS,
        )?,
        intent: option_value(arguments, "--intent")?,
    };
    let workspace = open_workspace(project)?;
    merge_result(
        "semantic.merge",
        &merge_revisions(
            workspace.repository(),
            &request,
            flag_present(arguments, "--apply")?,
        )?,
    )
}

fn revision_show_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error(
            "semantic revision-show requires one exact revision ID",
        ));
    }
    let revision = arguments[0].parse::<RevisionId>()?;
    let workspace = open_workspace(project)?;
    let snapshot = workspace.repository().reconstruct_revision(revision)?;
    success(
        "semantic.revision_show",
        json!({
            "record": snapshot.record,
            "receipt": snapshot.receipt,
            "root": {
                "repository_id": snapshot.root.repository_id,
                "package_id": snapshot.root.package_id,
                "package_name": snapshot.root.package_name,
                "modules": snapshot.root.modules.len(),
                "dependencies": snapshot.root.dependencies.len(),
                "targets": snapshot.root.targets.len(),
                "tombstones": snapshot.root.tombstones.len(),
            },
        }),
    )
}

fn doctor_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &[], &["--deep"])?;
    let workspace = open_workspace(project)?;
    serialized(
        "semantic.doctor",
        &workspace
            .repository()
            .doctor(flag_present(arguments, "--deep")?)?,
    )
}

fn backup_command(
    command: &str,
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--output"], &[])?;
    let workspace = open_workspace(project)?;
    let output = option_value(arguments, "--output")?
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().join("target/meaning-backup.lkjb"));
    let (bytes, receipt) = workspace.repository().backup()?;
    let publication = write_derived_output(
        &output,
        &bytes,
        MAXIMUM_BACKUP_BYTES + 50,
        "semantic backup",
    )?;
    success(
        &format!("semantic.{}", command.replace('-', "_")),
        json!({
            "receipt": receipt,
            "output": output.display().to_string(),
            "publication": publication,
        }),
    )
}

fn restore_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--backup", "--output"], &[])?;
    let backup = option_value(arguments, "--backup")?
        .ok_or_else(|| usage_error("semantic restore requires --backup PATH"))?;
    let explicit_output = option_value(arguments, "--output")?.map(PathBuf::from);
    if explicit_output.is_some() && project.is_some() {
        return Err(usage_error(
            "semantic restore accepts either --output or global --project, not both",
        ));
    }
    let output = explicit_output
        .or_else(|| project.map(Path::to_path_buf))
        .ok_or_else(|| {
            usage_error("semantic restore requires --output PROJECT or --project PROJECT")
        })?;
    let output = fs::canonicalize(&output)
        .map_err(|error| io_error("semantic_restore_output", &output, error))?;
    let bytes = read_bounded(
        Path::new(&backup),
        MAXIMUM_BACKUP_BYTES + 50,
        "semantic backup",
    )?;
    let (repository, receipt) = SemanticRepository::restore_backup(&output, &bytes)?;
    success(
        "semantic.restore",
        json!({
            "receipt": receipt,
            "status": SemanticWorkspace::open(repository.project_root())?.status()?,
        }),
    )
}

fn query_index(
    workspace: &SemanticWorkspace,
    arguments: &[String],
) -> Result<SemanticQueryIndex, Diagnostic> {
    match option_value(arguments, "--revision")? {
        Some(value) => SemanticQueryIndex::revision(workspace.repository(), value.parse()?),
        None => SemanticQueryIndex::current(workspace.repository()),
    }
}

fn selected_revision(
    workspace: &SemanticWorkspace,
    arguments: &[String],
) -> Result<RevisionId, Diagnostic> {
    match option_value(arguments, "--revision")? {
        Some(value) => value.parse(),
        None => Ok(workspace.status()?.revision),
    }
}

fn query_budget(arguments: &[String]) -> Result<QueryBudget, Diagnostic> {
    QueryBudget {
        maximum_items: optional_usize(arguments, "--limit", QueryBudget::default().maximum_items)?,
        maximum_bytes: optional_usize(arguments, "--bytes", QueryBudget::default().maximum_bytes)?,
        maximum_work: optional_usize(arguments, "--work", QueryBudget::default().maximum_work)?,
        maximum_depth: optional_usize(arguments, "--depth", QueryBudget::default().maximum_depth)?,
        maximum_fanout: optional_usize(
            arguments,
            "--fanout",
            QueryBudget::default().maximum_fanout,
        )?,
    }
    .validate()
}

fn query_value_options<const N: usize>(extra: [&str; N]) -> Vec<&str> {
    let mut values = vec![
        "--revision",
        "--limit",
        "--bytes",
        "--work",
        "--depth",
        "--fanout",
        "--continue",
    ];
    values.extend(extra);
    values
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

fn option_value(arguments: &[String], name: &str) -> Result<Option<String>, Diagnostic> {
    let values = option_values(arguments, name)?;
    if values.len() > 1 {
        return Err(usage_error(format!("{name} may be supplied only once")));
    }
    Ok(values.into_iter().next())
}

fn option_values(arguments: &[String], name: &str) -> Result<Vec<String>, Diagnostic> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == name {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error(format!("{name} requires a value")))?;
            if value.starts_with("--") {
                return Err(usage_error(format!("{name} requires a value")));
            }
            values.push(value.clone());
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(values)
}

fn optional_usize(arguments: &[String], name: &str, default: usize) -> Result<usize, Diagnostic> {
    option_value(arguments, name)?
        .map(|value| parse_usize(&value, name))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn flag_present(arguments: &[String], name: &str) -> Result<bool, Diagnostic> {
    let count = arguments
        .iter()
        .filter(|value| value.as_str() == name)
        .count();
    if count > 1 {
        return Err(usage_error(format!("{name} may be supplied only once")));
    }
    Ok(count == 1)
}

fn ensure_options(arguments: &[String], valued: &[&str], flags: &[&str]) -> Result<(), Diagnostic> {
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        if valued.contains(&option) {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error(format!("{option} requires a value")))?;
            if value.starts_with("--") {
                return Err(usage_error(format!("{option} requires a value")));
            }
            index += 2;
        } else if flags.contains(&option) {
            index += 1;
        } else {
            return Err(usage_error(format!(
                "unknown option or argument '{option}'"
            )));
        }
    }
    Ok(())
}

fn exact_arguments(arguments: &[String], expected: usize, command: &str) -> Result<(), Diagnostic> {
    if arguments.len() != expected {
        return Err(usage_error(format!(
            "{command} received an unexpected argument"
        )));
    }
    Ok(())
}

fn open_workspace(project: Option<&Path>) -> Result<SemanticWorkspace, Diagnostic> {
    SemanticWorkspace::open(&project_or_current(project)?)
}

fn project_or_current(project: Option<&Path>) -> Result<PathBuf, Diagnostic> {
    let path = match project {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().map_err(|error| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "semantic_current_directory",
                format!("current directory is unavailable: {error}"),
            )
        })?,
    };
    fs::canonicalize(&path).map_err(|error| io_error("semantic_project_path", &path, error))
}

fn write_derived_output(
    path: &Path,
    bytes: &[u8],
    maximum_existing_bytes: usize,
    label: &str,
) -> Result<&'static str, Diagnostic> {
    let parent = path.parent().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_output_parent",
            "output path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error("semantic_output_directory", parent, error))?;
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "semantic_output_type",
                format!("output '{}' is not a regular file", path.display()),
            ));
        }
        if read_bounded(path, maximum_existing_bytes, label)? == bytes {
            return Ok("unchanged");
        }
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| usage_error("output file name is not portable UTF-8"))?;
    let temporary = parent.join(format!(".{file_name}.stage-{}", RepositoryId::generate()?));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| io_error("semantic_output_create", &temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| io_error("semantic_output_write", &temporary, error))?;
        file.sync_all()
            .map_err(|error| io_error("semantic_output_sync", &temporary, error))?;
        fs::rename(&temporary, path)
            .map_err(|error| io_error("semantic_output_publish", path, error))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("semantic_output_parent_sync", parent, error))?;
        Ok("published")
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("read_open", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "read_type",
            format!("{label} '{}' is not a regular file", path.display()),
        ));
    }
    let mut file = File::open(path).map_err(|error| io_error("read_open", path, error))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read_bytes", path, error))?;
    if bytes.len() > maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "read_limit",
            format!("{label} exceeds {maximum} bytes"),
        ));
    }
    Ok(bytes)
}

fn parse_usize(value: &str, label: &str) -> Result<usize, Diagnostic> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(usage_error(format!(
            "{label} must be a canonical unsigned integer"
        )));
    }
    value
        .parse::<usize>()
        .map_err(|_| usage_error(format!("{label} is outside the supported range")))
}

fn serialized(command: &str, value: &impl Serialize) -> Result<CliSuccess, Diagnostic> {
    success(command, serde_json::to_value(value).map_err(internal_json)?)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TransactionCliProjection<'a> {
    contract_version: u16,
    graph_contract: &'static str,
    repository_id: RepositoryId,
    requested_base: RevisionId,
    observed_current: RevisionId,
    status: TransactionStatus,
    transaction: TransactionDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_diff: Option<SemanticDiffDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    predicted_revision: Option<RevisionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_revision: Option<RevisionId>,
    affected_owner_count: usize,
    affected_owners: &'a [AffectedOwner],
    affected_owners_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt: Option<TransactionReceiptProjection<'a>>,
    diagnostics: &'a [Diagnostic],
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TransactionReceiptProjection<'a> {
    digest: ReceiptDigest,
    result: RevisionId,
    semantic_diff: SemanticDiffDigest,
    validation: &'a ValidationFacts,
    expansion: String,
}

fn transaction_result(command: &str, value: &TransactionResult) -> Result<CliSuccess, Diagnostic> {
    let (status, ok) = match value.status {
        TransactionStatus::Planned => ("planned", true),
        TransactionStatus::Validated => ("validated", true),
        TransactionStatus::AcceptedChange => ("accepted_change", true),
        TransactionStatus::Replayed => ("replayed", true),
        TransactionStatus::SemanticNoChange => ("semantic_no_change", true),
        TransactionStatus::StaleBase => ("stale_base", false),
        TransactionStatus::PreconditionFailed => ("precondition_failed", false),
        TransactionStatus::ForeignIdentity => ("foreign_identity", false),
        TransactionStatus::InvalidGraph => ("invalid_graph", false),
        TransactionStatus::ResourceExhausted => ("resource_exhausted", false),
    };
    let affected_limit = value
        .affected_owners
        .len()
        .min(MAXIMUM_INLINE_AFFECTED_OWNERS);
    let receipt = value
        .receipt
        .as_ref()
        .map(transaction_receipt_projection)
        .transpose()?;
    let projection = TransactionCliProjection {
        contract_version: value.contract_version,
        graph_contract: value.graph_contract,
        repository_id: value.repository_id,
        requested_base: value.requested_base,
        observed_current: value.observed_current,
        status: value.status,
        transaction: value.transaction,
        semantic_diff: value.semantic_diff,
        predicted_revision: value.predicted_revision,
        published_revision: value.published_revision,
        affected_owner_count: value.affected_owners.len(),
        affected_owners: &value.affected_owners[..affected_limit],
        affected_owners_truncated: affected_limit != value.affected_owners.len(),
        receipt,
        diagnostics: &value.diagnostics,
    };
    outcome(
        command,
        status,
        ok,
        serde_json::to_value(projection).map_err(internal_json)?,
    )
}

fn transaction_receipt_projection(
    receipt: &TransactionReceipt,
) -> Result<TransactionReceiptProjection<'_>, Diagnostic> {
    Ok(TransactionReceiptProjection {
        digest: receipt.digest()?,
        result: receipt.result,
        semantic_diff: receipt.semantic_diff,
        validation: &receipt.validation,
        expansion: format!("semantic revision-show {}", receipt.result),
    })
}

fn merge_result(command: &str, value: &SemanticMergeResult) -> Result<CliSuccess, Diagnostic> {
    let (status, ok) = match value.status {
        SemanticMergeStatus::Ready => ("ready", true),
        SemanticMergeStatus::Conflicted => ("conflicted", false),
        SemanticMergeStatus::AcceptedChange => ("accepted_change", true),
        SemanticMergeStatus::SemanticNoChange => ("semantic_no_change", true),
        SemanticMergeStatus::StaleHead => ("stale_head", false),
    };
    outcome(
        command,
        status,
        ok,
        serde_json::to_value(value).map_err(internal_json)?,
    )
}

fn success(command: &str, result: serde_json::Value) -> Result<CliSuccess, Diagnostic> {
    outcome(command, "success", true, result)
}

fn outcome(
    command: &str,
    status: &'static str,
    ok: bool,
    result: serde_json::Value,
) -> Result<CliSuccess, Diagnostic> {
    let response = CliSuccess {
        contract_version: CLI_CONTRACT_VERSION,
        ok,
        status,
        command: command.to_owned(),
        result,
    };
    let bytes = serde_json::to_vec(&response).map_err(internal_json)?;
    if bytes.len().saturating_add(1) > MAXIMUM_CLI_RESPONSE_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "semantic_output_budget",
            format!("command response exceeds the hard {MAXIMUM_CLI_RESPONSE_BYTES}-byte bound"),
        ));
    }
    Ok(response)
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
    Diagnostic::new(DiagnosticClass::Source, "semantic_cli_usage", message)
}

fn internal_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "semantic_cli_internal",
        message,
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_options_and_noncanonical_numbers_reject() {
        assert!(execute(vec!["unknown".to_owned()]).is_err());
        assert!(parse_usize("01", "limit").is_err());
        assert!(ensure_options(&["--wat".to_owned()], &["--limit"], &[]).is_err());
    }

    #[test]
    fn command_registry_has_unique_names_and_stable_digest() {
        let commands = command_registry();
        let names = commands
            .iter()
            .map(|entry| entry.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(commands.len(), names.len());
        assert_eq!(command_schema_digest().len(), 64);
    }
}
