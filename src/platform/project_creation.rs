//! Atomic typed creation of the executable's closed normalized project recipe set.

#[cfg(test)]
mod projection_tests;
mod recipes;

use self::recipes::{command_recipe, http_recipe, minimal_recipe, nostr_relay_info_recipe};
use super::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
use super::control::{LogicalChangePlan, encode_logical_change_plan, normalize_change_request};
use super::deployment::{
    STARTER_HTTP_ARTIFACT_DIRECTORY, STARTER_HTTP_ARTIFACT_PATH, STARTER_HTTP_DESCRIPTOR_PATH,
    STARTER_HTTP_LISTENER, STARTER_HTTP_TARGET,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    KernelSnapshot, Name, OwnerKind, PackageId, SemanticRoot, SemanticRootDigest,
    SemanticStateDigest, validate_full,
};
use super::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use super::publication::{
    GraphRepository, InitialPackageTransport, PublicationOptions, PublicationOutcome,
    ReceiptObjectDigest, RevisionObjectDigest,
};
use super::semantic_id::{RepositoryId, RevisionId};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const PROJECT_CREATION_CONTRACT_IDENTITY: &str = "lkjscript-project-creation-3";
pub const PROJECT_CREATION_CONTRACT_VERSION: u16 = 3;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ProjectTemplate {
    Minimal,
    Command,
    Http,
    NostrRelayInfo,
}

impl ProjectTemplate {
    pub(crate) const ALL: [Self; 4] = [
        Self::Minimal,
        Self::Command,
        Self::Http,
        Self::NostrRelayInfo,
    ];

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|template| template.name() == value)
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Command => "command",
            Self::Http => "http",
            Self::NostrRelayInfo => "nostr-relay-info",
        }
    }

    pub(crate) const fn purpose(self) -> &'static str {
        match self {
            Self::Minimal => "Create the smallest normalized accepted project authority.",
            Self::Command => {
                "Create a tested pure command with one exact built-in standard dependency."
            }
            Self::Http => {
                "Create an editable tested HTTP application and loopback starter deployment."
            }
            Self::NostrRelayInfo => {
                "Create a tested NIP-11 relay-information proxy with one deployment-bound endpoint."
            }
        }
    }

    pub(crate) const fn runner(self) -> &'static str {
        match self {
            Self::Minimal => "none",
            Self::Command => "command",
            Self::Http | Self::NostrRelayInfo => "http",
        }
    }

    pub(crate) const fn emits_deployment(self) -> bool {
        matches!(self, Self::Http | Self::NostrRelayInfo)
    }

    pub(crate) const fn recommended_artifact_output(self) -> Option<&'static str> {
        match self {
            Self::Http | Self::NostrRelayInfo => Some(STARTER_HTTP_ARTIFACT_PATH),
            Self::Minimal | Self::Command => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatedDeployment {
    pub descriptor: PathBuf,
    pub recommended_artifact_output: PathBuf,
    pub target: &'static str,
    pub runner: &'static str,
    pub configured_listener: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCreation {
    pub project: PathBuf,
    pub package_name: Name,
    pub repository: RepositoryId,
    pub package: PackageId,
    pub revision: RevisionId,
    pub semantic_state: SemanticStateDigest,
    pub semantic_root: SemanticRootDigest,
    pub revision_record: RevisionObjectDigest,
    pub receipt: ReceiptObjectDigest,
    pub(crate) template: ProjectTemplate,
    pub owners: u64,
    pub dependencies: u64,
    pub targets: u64,
    pub tests: u64,
    pub deployment: Option<CreatedDeployment>,
}

pub(crate) fn create_project(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
) -> Result<ProjectCreation, Diagnostic> {
    create_project_with_hook(destination, package_name, template, |_, _, _| Ok(()))
}

pub(crate) fn create_project_with_relay(
    destination: &Path,
    package_name: &str,
    relay_url: &str,
) -> Result<ProjectCreation, Diagnostic> {
    create_project_with_options(
        destination,
        package_name,
        ProjectTemplate::NostrRelayInfo,
        Some(relay_url),
        |_, _, _| Ok(()),
    )
}

struct ProjectRecipe {
    changes: Vec<AuthoredChange>,
    transports: Vec<InitialPackageTransport>,
    template: ProjectTemplate,
    auxiliary: Option<ProjectAuxiliary>,
}

struct ProjectAuxiliary {
    descriptor: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreationPoint {
    GraphPublished,
    BeforeDescriptor,
    DescriptorPublished,
    GeneratedDirectoryPublished,
    BeforeVisibility,
}

fn create_project_with_hook<F>(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
    hook: F,
) -> Result<ProjectCreation, Diagnostic>
where
    F: FnMut(CreationPoint, &Path, &Path) -> Result<(), Diagnostic>,
{
    create_project_with_options(destination, package_name, template, None, hook)
}

fn create_project_with_options<F>(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
    relay_url: Option<&str>,
    mut hook: F,
) -> Result<ProjectCreation, Diagnostic>
where
    F: FnMut(CreationPoint, &Path, &Path) -> Result<(), Diagnostic>,
{
    let destination = safe_destination(destination)?;
    let parent = destination.parent().ok_or_else(|| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination_parent",
            "project destination has no existing parent",
        )
    })?;
    let package_name = Name::new(package_name)?;
    let repository = RepositoryId::generate()?;
    let package = PackageId::generate()?;
    let recipe = match template {
        ProjectTemplate::Minimal => Ok(minimal_recipe()),
        ProjectTemplate::Command => command_recipe(),
        ProjectTemplate::Http => http_recipe(),
        ProjectTemplate::NostrRelayInfo => nostr_relay_info_recipe(relay_url.ok_or_else(|| {
            creation_error(
                DiagnosticClass::Source,
                "new_relay_url_required",
                "nostr-relay-info requires one exact relay URL",
            )
        })?),
    }?;
    validate_auxiliary_inventory(recipe.auxiliary.as_ref())?;
    let snapshot = lower_recipe(parent, repository, package, package_name.clone(), &recipe)?;
    validate_recipe_snapshot(&snapshot)?;
    let owners = snapshot.owners.len() as u64;
    let dependencies = snapshot.dependencies.len() as u64;
    let targets = snapshot
        .owners
        .values()
        .filter(|record| record.kind() == OwnerKind::Target)
        .count() as u64;
    let tests = snapshot
        .owners
        .values()
        .filter(|record| record.kind() == OwnerKind::Test)
        .count() as u64;
    let private = parent.join(format!(".lkjscript-project-stage-{repository}"));

    let created = GraphRepository::create_with_package_transports(
        &private,
        &snapshot,
        Some(format!(
            "public {} project bootstrap",
            recipe.template.name()
        )),
        &recipe.transports,
    )?;
    let staged = (|| {
        hook(CreationPoint::GraphPublished, &private, &destination)?;
        if let Some(auxiliary) = &recipe.auxiliary {
            publish_auxiliary(&private, auxiliary, &mut hook, &destination)?;
        }
        sync_stage_directory(&private)?;
        hook(CreationPoint::BeforeVisibility, &private, &destination)?;
        Ok(())
    })();
    if let Err(error) = staged {
        cleanup_owned_directory(&private, Some(&error))?;
        return Err(error);
    }
    if let Err(error) = fs::rename(&private, &destination) {
        let publish = creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_publish",
            format!(
                "normalized project could not be published at '{}': {error}",
                destination.display()
            ),
        );
        cleanup_owned_directory(&private, Some(&publish))?;
        return Err(publish);
    }
    sync_parent_directory(parent)?;

    let visible = GraphRepository::open(&destination)?;
    let current = visible.current()?;
    if current.head != created.current.head
        || current.semantic_root.package_id != package
        || current.semantic_root.repository_id != repository
    {
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_reconcile",
            "visible normalized project disagrees with its privately accepted publication",
        ));
    }
    reconcile_auxiliary(&destination, recipe.auxiliary.as_ref())?;
    let deployment = recipe.auxiliary.as_ref().map(|_| CreatedDeployment {
        descriptor: destination.join(STARTER_HTTP_DESCRIPTOR_PATH),
        recommended_artifact_output: destination.join(STARTER_HTTP_ARTIFACT_PATH),
        target: STARTER_HTTP_TARGET,
        runner: "http",
        configured_listener: STARTER_HTTP_LISTENER,
    });
    Ok(ProjectCreation {
        project: destination,
        package_name,
        repository,
        package,
        revision: current.head.revision,
        semantic_state: current.accepted.semantic_state,
        semantic_root: current.accepted.semantic_root,
        revision_record: current.head.record,
        receipt: current.accepted.receipt,
        template,
        owners,
        dependencies,
        targets,
        tests,
        deployment,
    })
}

/// Lowers every recipe through the same authored-change preparation, allocation, impact,
/// validation, test-selection, and logical-plan machinery as public plan/apply. The temporary
/// repository supplies only a fresh zero base and exact built-in package transports; its history
/// is discarded before one fully populated initial revision becomes visible at the destination.
fn lower_recipe(
    parent: &Path,
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
    recipe: &ProjectRecipe,
) -> Result<KernelSnapshot, Diagnostic> {
    let empty = empty_snapshot(repository, package, package_name);
    if recipe.changes.is_empty() {
        return Ok(empty);
    }
    let lowering = parent.join(format!(".lkjscript-recipe-lowering-{repository}"));
    ensure_absent_owned_path(&lowering)?;
    let result = (|| {
        let created = GraphRepository::create_with_package_transports(
            &lowering,
            &empty,
            Some(format!(
                "private {} recipe zero base",
                recipe.template.name()
            )),
            &recipe.transports,
        )?;
        let semantic = AuthoredChangeSet {
            base: created.current.head.revision,
            preconditions: Vec::new(),
            changes: recipe.changes.clone(),
            budget: ChangeBudget::default(),
        };
        let normalized = normalize_change_request(
            semantic,
            PublicationOptions {
                intent: Some(format!(
                    "{} recipe authored lowering",
                    recipe.template.name()
                )),
                ..PublicationOptions::default()
            },
        )?;
        let prepared = created
            .repository
            .prepare_authored_change(&normalized.semantic, normalized.options)
            .map_err(first_diagnostic)?;
        let plan = LogicalChangePlan::new(normalized.request_commitment, &prepared)?;
        let _ = encode_logical_change_plan(&plan, |_| Ok(()))?;
        match created.repository.publish(&prepared.publication)? {
            PublicationOutcome::Accepted { .. } => {}
            PublicationOutcome::AlreadyAccepted { .. } => {
                return Err(creation_error(
                    DiagnosticClass::Corrupt,
                    "new_recipe_publication_already_accepted",
                    "fresh recipe lowering unexpectedly resolved to an existing publication",
                ));
            }
            PublicationOutcome::Stale { .. } => {
                return Err(creation_error(
                    DiagnosticClass::Corrupt,
                    "new_recipe_publication_stale",
                    "fresh recipe lowering became stale before private publication",
                ));
            }
        }
        let read = created
            .repository
            .view_current()?
            .reconstruct_full_oracle()?;
        validate_recipe_snapshot(&read.value)?;
        Ok(read.value)
    })();
    cleanup_owned_directory(&lowering, result.as_ref().err())?;
    result
}

fn first_diagnostic(diagnostics: Vec<Diagnostic>) -> Diagnostic {
    match diagnostics.into_iter().next() {
        Some(diagnostic) => diagnostic,
        None => creation_error(
            DiagnosticClass::Corrupt,
            "new_recipe_diagnostic_missing",
            "recipe authored lowering failed without an exact diagnostic",
        ),
    }
}

fn validate_recipe_snapshot(snapshot: &KernelSnapshot) -> Result<(), Diagnostic> {
    validate_full(snapshot)
        .map(|_| ())
        .map_err(first_diagnostic)
}

fn publish_auxiliary<F>(
    private: &Path,
    auxiliary: &ProjectAuxiliary,
    hook: &mut F,
    destination: &Path,
) -> Result<(), Diagnostic>
where
    F: FnMut(CreationPoint, &Path, &Path) -> Result<(), Diagnostic>,
{
    hook(CreationPoint::BeforeDescriptor, private, destination)?;
    let descriptor = private.join(STARTER_HTTP_DESCRIPTOR_PATH);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&descriptor)
        .map_err(|error| auxiliary_io("new_descriptor_create", &descriptor, error))?;
    output
        .write_all(&auxiliary.descriptor)
        .and_then(|()| output.sync_all())
        .map_err(|error| auxiliary_io("new_descriptor_publish", &descriptor, error))?;
    hook(CreationPoint::DescriptorPublished, private, destination)?;

    let generated = private.join(STARTER_HTTP_ARTIFACT_DIRECTORY);
    fs::create_dir(&generated)
        .map_err(|error| auxiliary_io("new_generated_directory", &generated, error))?;
    sync_stage_directory(&generated)?;
    hook(
        CreationPoint::GeneratedDirectoryPublished,
        private,
        destination,
    )?;
    Ok(())
}

fn reconcile_auxiliary(
    destination: &Path,
    auxiliary: Option<&ProjectAuxiliary>,
) -> Result<(), Diagnostic> {
    let Some(auxiliary) = auxiliary else {
        return Ok(());
    };
    let descriptor = destination.join(STARTER_HTTP_DESCRIPTOR_PATH);
    let metadata = fs::symlink_metadata(&descriptor)
        .map_err(|error| auxiliary_io("new_destination_reconcile", &descriptor, error))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || fs::read(&descriptor)
            .map(|bytes| bytes != auxiliary.descriptor)
            .unwrap_or(true)
    {
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_reconcile",
            "visible starter deployment disagrees with its synchronized private publication",
        ));
    }
    let generated = destination.join(STARTER_HTTP_ARTIFACT_DIRECTORY);
    let metadata = fs::symlink_metadata(&generated)
        .map_err(|error| auxiliary_io("new_destination_reconcile", &generated, error))?;
    let empty = metadata.is_dir()
        && !metadata.file_type().is_symlink()
        && fs::read_dir(&generated)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
    if !empty {
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_reconcile",
            "visible generated output directory is absent, foreign, or unexpectedly populated",
        ));
    }
    Ok(())
}

fn auxiliary_io(code: &'static str, path: &Path, error: std::io::Error) -> Diagnostic {
    creation_error(
        DiagnosticClass::Infrastructure,
        code,
        format!(
            "project auxiliary path '{}' failed: {error}",
            path.display()
        ),
    )
}

fn validate_auxiliary_inventory(auxiliary: Option<&ProjectAuxiliary>) -> Result<(), Diagnostic> {
    let Some(_) = auxiliary else {
        return Ok(());
    };
    for (path, label) in [
        (STARTER_HTTP_DESCRIPTOR_PATH, "starter descriptor"),
        (STARTER_HTTP_ARTIFACT_DIRECTORY, "generated directory"),
        (STARTER_HTTP_ARTIFACT_PATH, "recommended artifact"),
    ] {
        let path = Path::new(path);
        if path.is_absolute()
            || path.as_os_str().is_empty()
            || path.components().any(
                |component| !matches!(component, Component::Normal(value) if !value.is_empty()),
            )
        {
            return Err(creation_error(
                DiagnosticClass::Corrupt,
                "new_auxiliary_path",
                format!("{label} path is not a canonical relative path"),
            ));
        }
    }
    if Path::new(STARTER_HTTP_ARTIFACT_PATH).parent()
        != Some(Path::new(STARTER_HTTP_ARTIFACT_DIRECTORY))
        || STARTER_HTTP_DESCRIPTOR_PATH == STARTER_HTTP_ARTIFACT_DIRECTORY
        || STARTER_HTTP_DESCRIPTOR_PATH == STARTER_HTTP_ARTIFACT_PATH
    {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_auxiliary_inventory",
            "starter descriptor, generated directory, and artifact output paths overlap",
        ));
    }
    Ok(())
}

fn empty_snapshot(
    repository_id: RepositoryId,
    package_id: PackageId,
    package_name: Name,
) -> KernelSnapshot {
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id,
            package_id,
            package_name,
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn safe_destination(destination: &Path) -> Result<PathBuf, Diagnostic> {
    let name = destination.file_name().ok_or_else(|| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination",
            "project destination must name one directory below an existing parent",
        )
    })?;
    if destination
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_traversal",
            "project destination may not contain '..'",
        ));
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    reject_symlinked_path(parent)?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination_parent",
            format!(
                "project parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_parent_type",
            format!(
                "project parent '{}' is not an ordinary directory",
                parent.display()
            ),
        ));
    }
    let parent = parent.canonicalize().map_err(|error| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination_parent",
            format!(
                "project parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    let destination = parent.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(creation_error(
                DiagnosticClass::Source,
                "new_destination_type",
                format!(
                    "project destination '{}' is not an ordinary directory",
                    destination.display()
                ),
            ))
        }
        Ok(_) if predecessor_project(&destination) => Err(creation_error(
            DiagnosticClass::Source,
            "predecessor_contract",
            format!(
                "project destination '{}' contains predecessor authority",
                destination.display()
            ),
        )),
        Ok(_) => Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_not_empty",
            format!(
                "project destination '{}' already exists",
                destination.display()
            ),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(destination),
        Err(error) => Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_inspect",
            format!(
                "project destination '{}' could not be inspected: {error}",
                destination.display()
            ),
        )),
    }
}

fn reject_symlinked_path(path: &Path) -> Result<(), Diagnostic> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                creation_error(
                    DiagnosticClass::Infrastructure,
                    "new_current_directory",
                    format!("current directory is unavailable: {error}"),
                )
            })?
            .join(path)
    };
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => checked.push(prefix.as_os_str()),
            Component::RootDir => checked.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(creation_error(
                    DiagnosticClass::Source,
                    "new_destination_traversal",
                    "project destination may not contain '..'",
                ));
            }
            Component::Normal(value) => {
                checked.push(value);
                let metadata = fs::symlink_metadata(&checked).map_err(|error| {
                    creation_error(
                        DiagnosticClass::Source,
                        "new_destination_parent",
                        format!(
                            "project parent '{}' is unavailable: {error}",
                            checked.display()
                        ),
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(creation_error(
                        DiagnosticClass::Source,
                        "new_destination_symlink",
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

fn predecessor_project(path: &Path) -> bool {
    [path.join(".lkjscript"), path.join("lkjscript.package.json")]
        .into_iter()
        .any(|marker| fs::symlink_metadata(marker).is_ok())
}

fn ensure_absent_owned_path(path: &Path) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_recipe_lowering_path_exists",
            format!(
                "private recipe lowering path '{}' already exists",
                path.display()
            ),
        )),
        Err(error) => Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_recipe_lowering_path_inspect",
            format!(
                "private recipe lowering path '{}' could not be inspected: {error}",
                path.display()
            ),
        )),
    }
}

fn cleanup_owned_directory(path: &Path, prior: Option<&Diagnostic>) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).map_err(|error| {
                let context = prior
                    .map(|diagnostic| format!(" after {}", diagnostic.code))
                    .unwrap_or_default();
                creation_error(
                    DiagnosticClass::Infrastructure,
                    "new_private_cleanup",
                    format!(
                        "owned private directory '{}' could not be removed{context}: {error}",
                        path.display()
                    ),
                )
            })
        }
        Ok(_) => Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_private_cleanup_type",
            format!(
                "owned private path '{}' changed type before cleanup",
                path.display()
            ),
        )),
        Err(error) => Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_private_cleanup_inspect",
            format!(
                "owned private path '{}' could not be inspected before cleanup: {error}",
                path.display()
            ),
        )),
    }
}

fn sync_parent_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            creation_error(
                DiagnosticClass::Infrastructure,
                "new_parent_sync",
                format!(
                    "project parent '{}' could not be synchronized: {error}",
                    path.display()
                ),
            )
        })
}

fn sync_stage_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            creation_error(
                DiagnosticClass::Infrastructure,
                "new_stage_sync",
                format!(
                    "private project path '{}' could not be synchronized: {error}",
                    path.display()
                ),
            )
        })
}

fn creation_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_recipes_use_authored_lowering_and_retain_expected_inventory() {
        let temporary = tempfile::TempDir::new().expect("temporary recipe parent");
        for (template, owners, dependencies, targets, tests) in [
            (ProjectTemplate::Minimal, 0, 0, 0, 0),
            (ProjectTemplate::Command, 10, 1, 1, 1),
            (ProjectTemplate::Http, 20, 1, 1, 1),
        ] {
            let destination = temporary.path().join(template.name());
            let created = create_project(&destination, template.name(), template)
                .expect("create authored recipe");
            assert_eq!(created.owners, owners, "{} owners", template.name());
            assert_eq!(
                created.dependencies,
                dependencies,
                "{} dependencies",
                template.name()
            );
            assert_eq!(created.targets, targets, "{} targets", template.name());
            assert_eq!(created.tests, tests, "{} tests", template.name());
            assert!(
                !temporary
                    .path()
                    .join(format!(".lkjscript-recipe-lowering-{}", created.repository))
                    .exists()
            );
        }
    }

    #[test]
    fn every_recipe_operation_is_owned_by_the_public_compact_grammar() {
        let recipes = [
            minimal_recipe(),
            command_recipe().expect("command intent"),
            http_recipe().expect("HTTP intent"),
            nostr_relay_info_recipe("ws://127.0.0.1:7447/nostr").expect("Nostr intent"),
        ];
        for recipe in recipes {
            for change in recipe.changes {
                let operation = match change {
                    AuthoredChange::AddDependency { .. } => "add.dependency",
                    AuthoredChange::CreateModule { .. } => "create.module",
                    AuthoredChange::CreateFunction {
                        type_parameters,
                        parameters,
                        ..
                    } => {
                        assert!(
                            type_parameters.is_empty() && parameters.is_empty(),
                            "recipe function children must use public add operations"
                        );
                        "create.function"
                    }
                    AuthoredChange::AddParameter { .. } => "add.parameter",
                    AuthoredChange::CreateComponent {
                        requirements,
                        ports,
                        ..
                    } => {
                        assert!(
                            requirements.is_empty() && ports.is_empty(),
                            "recipe component children must use public add operations"
                        );
                        "create.component"
                    }
                    AuthoredChange::AddRequirement { .. } => "add.requirement",
                    AuthoredChange::AddPort { port, .. } => {
                        assert!(matches!(
                            port.implementation,
                            super::super::change::AuthoredPortImplementation::Function { .. }
                        ));
                        "add.port"
                    }
                    AuthoredChange::CreateTarget { .. } => "create.target",
                    AuthoredChange::CreateTest { .. } => "create.test",
                    other => panic!("recipe retained non-public authored operation: {other:?}"),
                };
                assert!(
                    super::super::control::compact_change_operation_descriptor(operation).is_some(),
                    "{} recipe operation {operation}",
                    recipe.template.name()
                );
            }
        }
    }

    #[test]
    fn command_recipe_checks_and_runs() {
        let temporary = tempfile::TempDir::new().expect("temporary command parent");
        let destination = temporary.path().join("command");
        create_project(&destination, "command", ProjectTemplate::Command).expect("command project");
        let prepared = super::super::normalized_lifecycle::prepare_application(&destination)
            .expect("prepare command");
        let control = super::super::execution::ExecutionControl::default();
        let checked = prepared.check(&control).expect("check command");
        assert_eq!(checked.failed, 0);
        assert_eq!(checked.differential, "equal");
        let run = prepared
            .run(
                &Name::new("main").expect("target"),
                b"[]",
                super::super::execution::normalized::NormalizedCommandPolicy::default(),
                &control,
            )
            .expect("run command");
        assert_eq!(run.result_json, b"\"hello\"");
        assert_eq!(run.differential, "equal");
    }

    #[test]
    fn http_recipe_keeps_atomic_starter_deployment() {
        let temporary = tempfile::TempDir::new().expect("temporary HTTP parent");
        let destination = temporary.path().join("http");
        let created =
            create_project(&destination, "http", ProjectTemplate::Http).expect("HTTP project");
        let deployment = created.deployment.expect("starter deployment");
        assert_eq!(deployment.target, "serve");
        assert_eq!(deployment.runner, "http");
        assert!(deployment.descriptor.is_file());
        assert!(destination.join(STARTER_HTTP_ARTIFACT_DIRECTORY).is_dir());
        assert!(!deployment.recommended_artifact_output.exists());

        let descriptor = super::super::deployment::decode_deployment(
            &fs::read(&deployment.descriptor).expect("descriptor"),
        )
        .expect("strict descriptor");
        assert_eq!(descriptor.target, STARTER_HTTP_TARGET);
        assert_eq!(descriptor.listen.as_deref(), Some(STARTER_HTTP_LISTENER));
        assert_eq!(descriptor.grants.len(), 1);
        assert_eq!(descriptor.grants[0].requirement, "streams");
    }

    #[test]
    fn nostr_recipe_retains_two_requirements_and_two_tests() {
        let temporary = tempfile::TempDir::new().expect("temporary Nostr parent");
        let destination = temporary.path().join("nostr");
        let created = create_project_with_relay(&destination, "nostr", "ws://127.0.0.1:7447/nostr")
            .expect("Nostr project");
        assert_eq!(created.owners, 86);
        assert_eq!(created.dependencies, 1);
        assert_eq!(created.targets, 1);
        assert_eq!(created.tests, 2);
        let deployment = created.deployment.expect("deployment");
        let descriptor = super::super::deployment::decode_deployment(
            &fs::read(deployment.descriptor).expect("descriptor"),
        )
        .expect("strict descriptor");
        assert_eq!(descriptor.grants.len(), 2);
    }

    #[test]
    fn auxiliary_failure_removes_every_owned_private_path() {
        let temporary = tempfile::TempDir::new().expect("temporary HTTP parent");
        let destination = temporary.path().join("http");
        let error = create_project_with_hook(
            &destination,
            "http",
            ProjectTemplate::Http,
            |point, _, _| {
                if point == CreationPoint::BeforeDescriptor {
                    Err(creation_error(
                        DiagnosticClass::Infrastructure,
                        "test_auxiliary_failure",
                        "injected auxiliary failure",
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .expect_err("injected failure");
        assert_eq!(error.code, "test_auxiliary_failure");
        assert!(!destination.exists());
        let private = fs::read_dir(temporary.path())
            .expect("parent")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".lkjscript-")
            })
            .count();
        assert_eq!(private, 0);
    }
}
