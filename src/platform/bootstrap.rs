//! Exact executable-owned bootstrap meaning and blank-directory project publication.

use super::artifact::{LoadedArtifact, load_artifact};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::{DependencyBinding, GraphRoot, ModuleObjectRef};
use super::language::{DeclarationReference, Import, Module, ModuleReference};
use super::meaning::{
    GRAPH_CONTRACT_IDENTITY, GRAPH_CONTRACT_VERSION, MeaningModule, RequestIdentityAllocator,
};
use super::package::{PackageId, RunnerKind};
use super::repository::{DependencyArtifactObject, InitialPublication, SemanticRepository};
use super::revision::ReceiptStatus;
use super::semantic::{ExactGraphDependency, canonicalize_graph_package};
use super::semantic_change::{
    AllocatedIdentity, Change, ChangeRequest, ExpressionForm, PortForm, TypeForm, execute_change,
};
use super::semantic_digest::{RootObjectDigest, SemanticDiffDigest, TransactionDigest};
use super::semantic_id::{RepositoryId, RevisionId};
use super::semantic_transaction::TransactionBudget;
use super::syntax::SourceSpan;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Component as PathComponent;
use std::path::{Path, PathBuf};

pub const BOOTSTRAP_CONTRACT_VERSION: u16 = 2;
pub const BUILTIN_STANDARD_BYTES: &[u8] =
    include_bytes!("../../applications/lkjournal/dependencies/standard.lkja");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTemplate {
    Minimal,
    Command,
}

impl ProjectTemplate {
    pub fn parse(value: &str) -> Result<Self, Diagnostic> {
        match value {
            "minimal" => Ok(Self::Minimal),
            "command" => Ok(Self::Command),
            _ => Err(bootstrap_error(
                DiagnosticClass::Source,
                "bootstrap_template",
                format!("unknown project template '{value}'; expected minimal or command"),
            )),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Command => "command",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuiltinPackageInfo {
    pub contract_version: u16,
    pub graph_contract: &'static str,
    pub package_id: PackageId,
    pub semantic_revision: RevisionId,
    pub artifact: super::semantic_digest::ArtifactDigest,
    pub bundle_digest: String,
    pub bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectCreationReceipt {
    pub contract_version: u16,
    pub template: ProjectTemplate,
    pub project: PathBuf,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub revision: RevisionId,
    pub root: RootObjectDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin_dependency: Option<BuiltinPackageInfo>,
    pub allocated_identities: BTreeMap<String, AllocatedIdentity>,
    pub publication: super::revision::TransactionReceipt,
}

pub fn builtin_package_info() -> Result<BuiltinPackageInfo, Diagnostic> {
    let artifact = builtin_artifact()?;
    Ok(BuiltinPackageInfo {
        contract_version: BOOTSTRAP_CONTRACT_VERSION,
        graph_contract: GRAPH_CONTRACT_IDENTITY,
        package_id: artifact.root_package_id,
        semantic_revision: artifact.root_revision,
        artifact: artifact.root_package_artifact,
        bundle_digest: artifact.artifact_digest,
        bytes: BUILTIN_STANDARD_BYTES.len(),
    })
}

pub fn export_builtin_standard(path: &Path) -> Result<(BuiltinPackageInfo, usize), Diagnostic> {
    let info = builtin_package_info()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            bootstrap_error(
                DiagnosticClass::Infrastructure,
                "builtin_export_open",
                format!(
                    "'{}' could not be created exclusively: {error}",
                    path.display()
                ),
            )
        })?;
    if let Err(error) = file
        .write_all(BUILTIN_STANDARD_BYTES)
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(path);
        return Err(bootstrap_error(
            DiagnosticClass::Infrastructure,
            "builtin_export_write",
            format!("'{}' could not be written durably: {error}", path.display()),
        ));
    }
    Ok((info, BUILTIN_STANDARD_BYTES.len()))
}

pub fn create_project(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
) -> Result<ProjectCreationReceipt, Diagnostic> {
    let destination = prepare_destination(destination)?;
    let parent = destination.parent().ok_or_else(|| {
        bootstrap_error(
            DiagnosticClass::Source,
            "bootstrap_destination_parent",
            "project destination has no parent directory",
        )
    })?;
    let repository_id = RepositoryId::generate()?;
    let package_id = PackageId::generate()?;
    let stage = parent.join(format!(".lkjscript-project-stage-{repository_id}"));
    fs::create_dir(&stage).map_err(|error| {
        bootstrap_error(
            DiagnosticClass::Infrastructure,
            "bootstrap_stage_create",
            format!(
                "private project stage '{}' could not be created: {error}",
                stage.display()
            ),
        )
    })?;

    let initialized =
        build_initial_publication(repository_id, package_id.clone(), package_name, template)
            .and_then(|initial| SemanticRepository::initialize(&stage, initial));
    let (repository, initial_publication) = match initialized {
        Ok(value) => value,
        Err(error) => {
            remove_owned_stage(&stage);
            return Err(error);
        }
    };
    let completed_template = (|| -> Result<_, Diagnostic> {
        Ok(match template {
            ProjectTemplate::Minimal => (initial_publication, BTreeMap::new()),
            ProjectTemplate::Command => {
                let current = repository.current()?;
                let module = current
                    .root
                    .modules
                    .first()
                    .ok_or_else(|| {
                        bootstrap_error(
                            DiagnosticClass::Corrupt,
                            "bootstrap_module_missing",
                            "initial project has no module for its command recipe",
                        )
                    })?
                    .id;
                let result = execute_change(
                    &repository,
                    &command_recipe(
                        current.head.revision,
                        module,
                        builtin_declaration_reference("core", "identity")?,
                    ),
                    true,
                )?;
                let publication = result.transaction.receipt.ok_or_else(|| {
                    bootstrap_error(
                        DiagnosticClass::Semantic,
                        "bootstrap_recipe_rejected",
                        format!(
                            "command template did not publish: {:?}",
                            result.transaction.status
                        ),
                    )
                })?;
                (publication, result.allocated_identities)
            }
        })
    })();
    let (publication, allocated_identities) = match completed_template {
        Ok(value) => value,
        Err(error) => {
            remove_owned_stage(&stage);
            return Err(error);
        }
    };

    if destination.exists() {
        fs::remove_dir(&destination).map_err(|error| {
            remove_owned_stage(&stage);
            bootstrap_error(
                DiagnosticClass::Infrastructure,
                "bootstrap_destination_replace",
                format!(
                    "empty destination '{}' could not be replaced: {error}",
                    destination.display()
                ),
            )
        })?;
    }
    if let Err(error) = fs::rename(&stage, &destination) {
        remove_owned_stage(&stage);
        return Err(bootstrap_error(
            DiagnosticClass::Infrastructure,
            "bootstrap_publish",
            format!(
                "project could not be made visible at '{}': {error}",
                destination.display()
            ),
        ));
    }
    sync_directory(parent)?;

    let repository = SemanticRepository::open(&destination)?;
    let current = repository.current_binding()?;
    let builtin_dependency = match template {
        ProjectTemplate::Minimal => None,
        ProjectTemplate::Command => Some(builtin_package_info()?),
    };
    Ok(ProjectCreationReceipt {
        contract_version: BOOTSTRAP_CONTRACT_VERSION,
        template,
        project: destination,
        repository_id,
        package_id,
        revision: current.head.revision,
        root: current.record.core.root,
        builtin_dependency,
        allocated_identities,
        publication,
    })
}

fn builtin_artifact() -> Result<LoadedArtifact, Diagnostic> {
    load_artifact(BUILTIN_STANDARD_BYTES).map_err(|error| {
        bootstrap_error(
            DiagnosticClass::Corrupt,
            "builtin_standard_integrity",
            format!(
                "embedded standard package failed integrity verification: {}",
                error.message
            ),
        )
    })
}

fn build_initial_publication(
    repository_id: RepositoryId,
    package_id: PackageId,
    package_name: &str,
    template: ProjectTemplate,
) -> Result<InitialPublication, Diagnostic> {
    let builtin = (template == ProjectTemplate::Command)
        .then(builtin_artifact)
        .transpose()?;
    let mut seed = Vec::with_capacity(32);
    seed.extend_from_slice(&repository_id.bytes());
    seed.extend_from_slice(&package_id.bytes());
    let mut allocator = RequestIdentityAllocator::new(seed);
    let builtin_core = builtin
        .as_ref()
        .map(|artifact| {
            let package = artifact.root()?;
            let module = package
                .modules
                .iter()
                .find(|module| module.module.name == "core")
                .ok_or_else(|| {
                    bootstrap_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_core_missing",
                        "embedded standard package has no core module",
                    )
                })?;
            Ok(ModuleReference {
                package: package.descriptor.package_id.clone(),
                module: module.module_id,
            })
        })
        .transpose()?;
    let mut meaning = MeaningModule::create(empty_module(template, builtin_core)?, &mut allocator)?;
    let module_id = meaning.module_id;
    let dependencies = builtin
        .as_ref()
        .map(|artifact| {
            vec![DependencyBinding {
                alias: "std".to_owned(),
                package_id: artifact.root_package_id.clone(),
                semantic_revision: artifact.root_revision,
                artifact: artifact.root_package_artifact,
            }]
        })
        .unwrap_or_default();
    let mut root = GraphRoot {
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id,
        package_id: package_id.clone(),
        package_name: package_name.to_owned(),
        modules: vec![ModuleObjectRef {
            id: module_id,
            name: meaning.module.name.clone(),
            object: meaning.digest()?,
        }],
        dependencies,
        targets: Vec::new(),
        tombstones: Vec::new(),
    };
    let exact_dependencies = builtin
        .as_ref()
        .map(|artifact| {
            Ok(vec![ExactGraphDependency {
                alias: "std",
                package: artifact.root()?,
                artifact: artifact.root_package_artifact,
            }])
        })
        .transpose()?
        .unwrap_or_default();
    canonicalize_graph_package(
        &mut root,
        std::slice::from_mut(&mut meaning),
        &exact_dependencies,
    )?;
    let root_bytes = root.encode()?;
    let recipe = serde_json::to_vec(&serde_json::json!({
        "contract_version": BOOTSTRAP_CONTRACT_VERSION,
        "operation": "create_project",
        "template": template,
        "repository_id": repository_id,
        "package_id": package_id,
        "package_name": package_name,
        "root": root.digest()?,
    }))
    .map_err(|error| {
        bootstrap_error(
            DiagnosticClass::Infrastructure,
            "bootstrap_recipe_encode",
            format!("bootstrap recipe could not be encoded: {error}"),
        )
    })?;
    let dependency_artifacts = builtin
        .map(|artifact| {
            artifact
                .package_objects
                .into_iter()
                .map(|(digest, bytes)| DependencyArtifactObject { digest, bytes })
                .collect()
        })
        .unwrap_or_default();
    Ok(InitialPublication {
        root,
        modules: vec![meaning],
        transaction: TransactionDigest::of(&recipe),
        semantic_diff: SemanticDiffDigest::of(&root_bytes),
        intent: Some(format!("create {} project", template.name())),
        validation_profile: Some("bootstrap_full_oracle".to_owned()),
        dependency_artifacts,
        status: ReceiptStatus::ProjectCreated,
    })
}

fn empty_module(
    template: ProjectTemplate,
    builtin_core: Option<ModuleReference>,
) -> Result<Module, Diagnostic> {
    Ok(Module {
        name: "app".to_owned(),
        imports: match template {
            ProjectTemplate::Minimal => Vec::new(),
            ProjectTemplate::Command => vec![Import {
                alias: "core".to_owned(),
                target: builtin_core.ok_or_else(|| {
                    bootstrap_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_core_missing",
                        "command bootstrap requires the exact embedded standard core module",
                    )
                })?,
                span: SourceSpan {
                    byte_start: 0,
                    byte_end: 0,
                    line: 1,
                    column: 1,
                },
            }],
        },
        exports: Vec::new(),
        declarations: Vec::new(),
    })
}

fn command_recipe(
    base_revision: RevisionId,
    module: super::semantic_id::ModuleId,
    identity: DeclarationReference,
) -> ChangeRequest {
    ChangeRequest {
        contract_version: super::semantic_change::CHANGE_CONTRACT_VERSION,
        base_revision: Some(base_revision),
        idempotency_key: Some("builtin-command-template-v2".to_owned()),
        preconditions: Vec::new(),
        changes: vec![
            Change::CreateFunction {
                r#as: "$main".to_owned(),
                module: module.to_string(),
                name: "main".to_owned(),
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: TypeForm::Text,
                body: ExpressionForm::Call {
                    call: exact_reference_selector(&identity),
                    type_arguments: vec![TypeForm::Text],
                    arguments: vec![ExpressionForm::Text {
                        text: "hello".to_owned(),
                    }],
                },
                exported: true,
            },
            Change::CreateComponent {
                r#as: "$app".to_owned(),
                module: module.to_string(),
                name: "App".to_owned(),
                ports: vec![PortForm {
                    r#as: "$main-port".to_owned(),
                    name: "main".to_owned(),
                    parameters: Vec::new(),
                    result: TypeForm::Text,
                    function: "$main".to_owned(),
                }],
                exported: true,
            },
            Change::CreateTest {
                r#as: "$main-test".to_owned(),
                module: module.to_string(),
                name: "main-returns-hello".to_owned(),
                actual: ExpressionForm::Call {
                    call: "$main".to_owned(),
                    type_arguments: Vec::new(),
                    arguments: Vec::new(),
                },
                expected: ExpressionForm::Text {
                    text: "hello".to_owned(),
                },
            },
            Change::CreateTarget {
                r#as: "$main-target".to_owned(),
                name: "main".to_owned(),
                component: "$app".to_owned(),
                port: "$main-port".to_owned(),
                runner: RunnerKind::Command,
            },
        ],
        budget: TransactionBudget::default(),
        intent: Some("apply built-in command template v2".to_owned()),
    }
}

fn builtin_declaration_reference(
    module_name: &str,
    declaration_name: &str,
) -> Result<DeclarationReference, Diagnostic> {
    let artifact = builtin_artifact()?;
    let package = artifact.root()?;
    let module = package
        .modules
        .iter()
        .find(|module| module.module.name == module_name)
        .ok_or_else(|| {
            bootstrap_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_module_missing",
                format!("embedded standard package has no '{module_name}' module"),
            )
        })?;
    let declaration = module
        .declaration_identities
        .iter()
        .find(|identity| identity.name == declaration_name)
        .ok_or_else(|| {
            bootstrap_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_declaration_missing",
                format!(
                    "embedded standard package has no '{module_name}.{declaration_name}' declaration"
                ),
            )
        })?;
    Ok(DeclarationReference {
        package: package.descriptor.package_id.clone(),
        module: module.module_id,
        declaration: declaration.id,
    })
}

fn exact_reference_selector(reference: &DeclarationReference) -> String {
    format!(
        "exact:{}/{}/{}",
        reference.package.as_str(),
        reference.module,
        reference.declaration
    )
}

fn prepare_destination(destination: &Path) -> Result<PathBuf, Diagnostic> {
    let file_name = destination.file_name().ok_or_else(|| {
        bootstrap_error(
            DiagnosticClass::Source,
            "bootstrap_destination",
            "project destination must name one directory below an existing parent",
        )
    })?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    reject_symlinked_parent(parent)?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        bootstrap_error(
            DiagnosticClass::Source,
            "bootstrap_destination_parent",
            format!(
                "project parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    if !parent.is_dir() {
        return Err(bootstrap_error(
            DiagnosticClass::Source,
            "bootstrap_destination_parent_type",
            format!("project parent '{}' is not a directory", parent.display()),
        ));
    }
    let destination = parent.join(file_name);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(bootstrap_error(
                DiagnosticClass::Source,
                "bootstrap_destination_type",
                format!(
                    "project destination '{}' is not an ordinary directory",
                    destination.display()
                ),
            ));
        }
        Ok(_) => {
            let mut entries = fs::read_dir(&destination).map_err(|error| {
                bootstrap_error(
                    DiagnosticClass::Infrastructure,
                    "bootstrap_destination_read",
                    format!(
                        "project destination '{}' could not be inspected: {error}",
                        destination.display()
                    ),
                )
            })?;
            if entries
                .next()
                .transpose()
                .map_err(|error| {
                    bootstrap_error(
                        DiagnosticClass::Infrastructure,
                        "bootstrap_destination_read",
                        format!(
                            "project destination '{}' could not be inspected: {error}",
                            destination.display()
                        ),
                    )
                })?
                .is_some()
            {
                return Err(bootstrap_error(
                    DiagnosticClass::Source,
                    "bootstrap_destination_not_empty",
                    format!(
                        "project destination '{}' is not empty",
                        destination.display()
                    ),
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(bootstrap_error(
                DiagnosticClass::Infrastructure,
                "bootstrap_destination_inspect",
                format!(
                    "project destination '{}' could not be inspected: {error}",
                    destination.display()
                ),
            ));
        }
    }
    Ok(destination)
}

fn reject_symlinked_parent(parent: &Path) -> Result<(), Diagnostic> {
    let absolute = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                bootstrap_error(
                    DiagnosticClass::Infrastructure,
                    "bootstrap_current_directory",
                    format!("current directory is unavailable: {error}"),
                )
            })?
            .join(parent)
    };
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        match component {
            PathComponent::Prefix(prefix) => checked.push(prefix.as_os_str()),
            PathComponent::RootDir => checked.push(Path::new("/")),
            PathComponent::CurDir => {}
            PathComponent::ParentDir => {
                return Err(bootstrap_error(
                    DiagnosticClass::Source,
                    "bootstrap_destination_parent_traversal",
                    "project destination parent may not contain '..'",
                ));
            }
            PathComponent::Normal(value) => {
                checked.push(value);
                let metadata = fs::symlink_metadata(&checked).map_err(|error| {
                    bootstrap_error(
                        DiagnosticClass::Source,
                        "bootstrap_destination_parent",
                        format!(
                            "project parent '{}' is unavailable: {error}",
                            checked.display()
                        ),
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(bootstrap_error(
                        DiagnosticClass::Source,
                        "bootstrap_destination_parent_symlink",
                        format!(
                            "project parent '{}' traverses a symbolic link",
                            checked.display()
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            bootstrap_error(
                DiagnosticClass::Infrastructure,
                "bootstrap_parent_sync",
                format!(
                    "project parent '{}' could not be synchronized: {error}",
                    path.display()
                ),
            )
        })
}

fn remove_owned_stage(stage: &Path) {
    let _ = fs::remove_dir_all(stage);
}

fn bootstrap_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_standard_is_exact_and_exportable() {
        let info = builtin_package_info().expect("embedded standard");
        assert_eq!(info.bytes, BUILTIN_STANDARD_BYTES.len());
        let temporary = tempfile::TempDir::new().expect("temporary directory");
        let output = temporary.path().join("standard.lkja");
        export_builtin_standard(&output).expect("export built-in");
        assert_eq!(
            fs::read(output).expect("read export"),
            BUILTIN_STANDARD_BYTES
        );
    }

    #[test]
    fn blank_command_project_is_ordinary_authority() {
        let temporary = tempfile::TempDir::new().expect("temporary parent");
        let project = temporary.path().join("app");
        let receipt = create_project(&project, "sample", ProjectTemplate::Command)
            .expect("create command project");
        let repository = SemanticRepository::open(&project).expect("open project");
        let current = repository.current().expect("current revision");
        assert_eq!(receipt.revision, current.head.revision);
        assert_eq!(current.root.package_name, "sample");
        assert_eq!(current.root.modules.len(), 1);
        assert_eq!(current.root.dependencies.len(), 1);
        assert_eq!(current.root.targets.len(), 1);
    }

    #[test]
    fn nonempty_destination_is_untouched() {
        let temporary = tempfile::TempDir::new().expect("temporary parent");
        let project = temporary.path().join("app");
        fs::create_dir(&project).expect("destination");
        fs::write(project.join("keep"), b"owned").expect("user file");
        let error = create_project(&project, "sample", ProjectTemplate::Minimal)
            .expect_err("nonempty destination must reject");
        assert_eq!(error.code, "bootstrap_destination_not_empty");
        assert_eq!(fs::read(project.join("keep")).expect("preserved"), b"owned");
    }
}
