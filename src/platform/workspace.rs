//! Public workspace discovery over canonical semantic graph authority.

use super::artifact::{ArtifactReceipt, MAXIMUM_ARTIFACT_BYTES};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::PreparedProgram;
use super::meaning::{GRAPH_CONTRACT_IDENTITY, GRAPH_CONTRACT_VERSION};
use super::package::{PackageId, RunnerKind};
use super::repository::SemanticRepository;
use super::revision::TransactionReceipt;
use super::semantic_digest::{ArtifactDigest, RevisionRecordDigest, RootObjectDigest};
use super::semantic_draft::SemanticDraftStore;
use super::semantic_id::{ModuleId, RepositoryId, RevisionId, TargetId};
use super::semantic_query::SemanticQueryIndex;
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const WORKSPACE_CONTRACT_VERSION: u16 = 3;
pub const DEFAULT_ORIENTATION_ITEMS: usize = 64;
pub const MAXIMUM_ORIENTATION_ITEMS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStatus {
    pub contract_version: u16,
    pub authority: &'static str,
    pub graph_contract: &'static str,
    pub graph_contract_version: u16,
    pub root: String,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub package_name: String,
    pub revision: RevisionId,
    pub record: RevisionRecordDigest,
    pub root_object: RootObjectDigest,
    pub graph_valid: bool,
    pub drafts: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceOrientation {
    pub contract_version: u16,
    pub authority: &'static str,
    pub graph_contract: &'static str,
    pub root: String,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub package_name: String,
    pub revision: RevisionId,
    pub modules: Vec<ModuleOrientation>,
    pub dependencies: Vec<DependencyOrientation>,
    pub targets: Vec<TargetOrientation>,
    pub module_count: usize,
    pub dependency_count: usize,
    pub target_count: usize,
    pub truncated: bool,
    pub expansion_commands: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleOrientation {
    pub id: ModuleId,
    pub name: String,
    pub declarations: usize,
    pub exports: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyOrientation {
    pub alias: String,
    pub package_id: PackageId,
    pub semantic_revision: RevisionId,
    pub artifact: ArtifactDigest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetOrientation {
    pub id: TargetId,
    pub name: String,
    pub component_module: ModuleId,
    pub component_module_name: String,
    pub component_name: String,
    pub port_name: String,
    pub runner: RunnerKind,
}

#[derive(Clone, Debug)]
pub struct SemanticWorkspace {
    root: PathBuf,
    repository: SemanticRepository,
}

impl SemanticWorkspace {
    pub fn discover(start: &Path) -> Result<PathBuf, Diagnostic> {
        SemanticRepository::discover(start)
    }

    pub fn open(start: &Path) -> Result<Self, Diagnostic> {
        let repository = SemanticRepository::open(start)?;
        Ok(Self {
            root: repository.project_root().to_path_buf(),
            repository,
        })
    }

    pub fn initialize_from_artifact(
        root: &Path,
        artifact_path: &Path,
    ) -> Result<(Self, TransactionReceipt), Diagnostic> {
        let bytes = read_bounded_artifact(artifact_path)?;
        let (repository, receipt) = SemanticRepository::initialize_from_artifact(root, &bytes)?;
        Ok((
            Self {
                root: repository.project_root().to_path_buf(),
                repository,
            },
            receipt,
        ))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn repository(&self) -> &SemanticRepository {
        &self.repository
    }

    pub fn status(&self) -> Result<WorkspaceStatus, Diagnostic> {
        let current = self.repository.current_binding()?;
        Ok(WorkspaceStatus {
            contract_version: WORKSPACE_CONTRACT_VERSION,
            authority: "typed_semantic_graph",
            graph_contract: GRAPH_CONTRACT_IDENTITY,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            root: self.root.display().to_string(),
            repository_id: current.head.repository_id,
            package_id: current.stored_root.package_id,
            package_name: current.stored_root.package_name,
            revision: current.head.revision,
            record: current.head.record,
            root_object: current.record.core.root,
            graph_valid: true,
            drafts: SemanticDraftStore::new(&self.repository).count()?,
        })
    }

    pub fn orient(&self, maximum_items: usize) -> Result<WorkspaceOrientation, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_ORIENTATION_ITEMS {
            return Err(workspace_error(
                DiagnosticClass::Resource,
                "semantic_orientation_limit",
                format!("orientation item limit must be 1 through {MAXIMUM_ORIENTATION_ITEMS}"),
            ));
        }
        let current = self.repository.current()?;
        let root = &current.root;
        let index = SemanticQueryIndex::current(&self.repository)?;
        let mut modules = index
            .module_facts()?
            .into_iter()
            .map(|module| ModuleOrientation {
                id: module.id,
                name: module.name,
                declarations: module.declarations,
                exports: module.exports,
            })
            .collect::<Vec<_>>();
        modules.sort_by(|left, right| {
            (left.name.as_str(), left.id).cmp(&(right.name.as_str(), right.id))
        });
        let module_count = modules.len();
        modules.truncate(maximum_items);

        let mut dependencies = root
            .dependencies
            .iter()
            .map(|dependency| DependencyOrientation {
                alias: dependency.alias.clone(),
                package_id: dependency.package_id.clone(),
                semantic_revision: dependency.semantic_revision,
                artifact: dependency.artifact,
            })
            .collect::<Vec<_>>();
        dependencies.sort_by(|left, right| left.alias.cmp(&right.alias));
        let dependency_count = dependencies.len();
        dependencies.truncate(maximum_items);

        let mut targets = root
            .targets
            .iter()
            .map(|target| {
                let module = index
                    .owner_summary(&target.component_module.to_string())
                    .ok_or_else(|| {
                        workspace_error(
                            DiagnosticClass::Corrupt,
                            "semantic_orientation_target_module",
                            "target presentation lost its exact component module",
                        )
                    })?;
                let component = index
                    .owner_summary(&target.component.to_string())
                    .ok_or_else(|| {
                        workspace_error(
                            DiagnosticClass::Corrupt,
                            "semantic_orientation_target_component",
                            "target presentation lost its exact component",
                        )
                    })?;
                let port = index
                    .owner_summary(&target.port.to_string())
                    .ok_or_else(|| {
                        workspace_error(
                            DiagnosticClass::Corrupt,
                            "semantic_orientation_target_port",
                            "target presentation lost its exact component port",
                        )
                    })?;
                Ok(TargetOrientation {
                    id: target.id,
                    name: target.name.clone(),
                    component_module: target.component_module,
                    component_module_name: module.name.clone(),
                    component_name: component.name.clone(),
                    port_name: port.name.clone(),
                    runner: target.runner,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        targets.sort_by(|left, right| {
            (left.name.as_str(), left.id).cmp(&(right.name.as_str(), right.id))
        });
        let target_count = targets.len();
        targets.truncate(maximum_items);

        let truncated = module_count > modules.len()
            || dependency_count > dependencies.len()
            || target_count > targets.len();
        Ok(WorkspaceOrientation {
            contract_version: WORKSPACE_CONTRACT_VERSION,
            authority: "typed_semantic_graph",
            graph_contract: GRAPH_CONTRACT_IDENTITY,
            root: self.root.display().to_string(),
            repository_id: root.repository_id,
            package_id: root.package_id.clone(),
            package_name: root.package_name.clone(),
            revision: current.head.revision,
            modules,
            dependencies,
            targets,
            module_count,
            dependency_count,
            target_count,
            truncated,
            expansion_commands: vec![
                "lkjscript query owners --kind module --limit 64".to_owned(),
                "lkjscript query owners --limit 64".to_owned(),
                "lkjscript inspect targets --limit 64".to_owned(),
            ],
        })
    }

    pub fn build_artifact(&self) -> Result<(Vec<u8>, ArtifactReceipt), Diagnostic> {
        self.repository.build_artifact()
    }

    pub fn prepare(&self) -> Result<PreparedProgram, Diagnostic> {
        let (bytes, _) = self.build_artifact()?;
        PreparedProgram::prepare(super::artifact::load_artifact(&bytes)?)
    }
}

fn read_bounded_artifact(path: &Path) -> Result<Vec<u8>, Diagnostic> {
    let mut file =
        File::open(path).map_err(|error| workspace_io("semantic_import_open", path, error))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAXIMUM_ARTIFACT_BYTES as u64) + 51)
        .read_to_end(&mut bytes)
        .map_err(|error| workspace_io("semantic_import_read", path, error))?;
    if bytes.len() > MAXIMUM_ARTIFACT_BYTES + 50 {
        return Err(workspace_error(
            DiagnosticClass::Resource,
            "semantic_import_limit",
            format!("artifact exceeds {MAXIMUM_ARTIFACT_BYTES} payload bytes"),
        ));
    }
    Ok(bytes)
}

fn workspace_io(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    workspace_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn workspace_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
