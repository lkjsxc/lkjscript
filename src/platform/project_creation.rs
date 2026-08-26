//! Atomic creation of one minimal normalized semantic-graph project.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    ComparisonPolicy, DeclarationPayload, DeclarationRecord, DeclarationReference,
    DeclarationVisibility, DependencyRecord, ExpressionOperation, ExpressionRecord,
    FunctionDeclaration, FunctionEffect, KernelSnapshot, Name, OwnerHeader, OwnerKey, OwnerKind,
    OwnerRecord, PackageId, PortImplementation, PortRecord, PortReference, SemanticRoot,
    SemanticRootDigest, SemanticStateDigest, TargetRecord, TextValue, TypeForm, TypeObjectInterner,
    validate_full,
};
use super::package::RunnerKind;
use super::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use super::publication::{
    GraphRepository, InitialPackageTransport, ReceiptObjectDigest, RevisionObjectDigest,
};
use super::semantic_id::{
    DeclarationId, ExpressionId, ModuleId, PortId, RepositoryId, RevisionId, TargetId,
};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

pub const PROJECT_CREATION_CONTRACT_IDENTITY: &str = "lkjscript-project-creation-1";
pub const PROJECT_CREATION_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalProjectCreation {
    pub project: PathBuf,
    pub package_name: Name,
    pub repository: RepositoryId,
    pub package: PackageId,
    pub revision: RevisionId,
    pub semantic_state: SemanticStateDigest,
    pub semantic_root: SemanticRootDigest,
    pub revision_record: RevisionObjectDigest,
    pub receipt: ReceiptObjectDigest,
    pub template: &'static str,
    pub owners: u64,
    pub dependencies: u64,
    pub targets: u64,
    pub tests: u64,
}

pub fn create_minimal_project(
    destination: &Path,
    package_name: &str,
) -> Result<MinimalProjectCreation, Diagnostic> {
    create_minimal_project_before_publish(destination, package_name, |_| Ok(()))
}

pub fn create_command_project(
    destination: &Path,
    package_name: &str,
) -> Result<MinimalProjectCreation, Diagnostic> {
    create_project_before_publish(destination, package_name, command_recipe, |_| Ok(()))
}

fn create_minimal_project_before_publish<F>(
    destination: &Path,
    package_name: &str,
    before_publish: F,
) -> Result<MinimalProjectCreation, Diagnostic>
where
    F: FnOnce(&Path) -> Result<(), Diagnostic>,
{
    create_project_before_publish(destination, package_name, minimal_recipe, before_publish)
}

struct ProjectRecipe {
    snapshot: KernelSnapshot,
    transports: Vec<InitialPackageTransport>,
    template: &'static str,
    targets: u64,
    tests: u64,
}

fn create_project_before_publish<F, R>(
    destination: &Path,
    package_name: &str,
    recipe: R,
    before_publish: F,
) -> Result<MinimalProjectCreation, Diagnostic>
where
    F: FnOnce(&Path) -> Result<(), Diagnostic>,
    R: FnOnce(RepositoryId, PackageId, Name) -> Result<ProjectRecipe, Diagnostic>,
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
    let recipe = recipe(repository, package, package_name.clone())?;
    validate_full(&recipe.snapshot).map_err(|diagnostics| {
        diagnostics.into_iter().next().unwrap_or_else(|| {
            creation_error(
                DiagnosticClass::Corrupt,
                "new_recipe_validation",
                "typed project recipe failed without an exact diagnostic",
            )
        })
    })?;
    let private = parent.join(format!(".lkjscript-project-stage-{repository}"));

    let created = GraphRepository::create_with_package_transports(
        &private,
        &recipe.snapshot,
        Some(format!("public {} project bootstrap", recipe.template)),
        &recipe.transports,
    )?;
    if let Err(error) = before_publish(&destination) {
        remove_owned_stage(&private);
        return Err(error);
    }
    if let Err(error) = fs::rename(&private, &destination) {
        remove_owned_stage(&private);
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_publish",
            format!(
                "normalized project could not be published at '{}': {error}",
                destination.display()
            ),
        ));
    }
    sync_directory(parent)?;

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
    Ok(MinimalProjectCreation {
        project: destination,
        package_name,
        repository,
        package,
        revision: current.head.revision,
        semantic_state: current.accepted.semantic_state,
        semantic_root: current.accepted.semantic_root,
        revision_record: current.head.record,
        receipt: current.accepted.receipt,
        template: recipe.template,
        owners: recipe.snapshot.owners.len() as u64,
        dependencies: recipe.snapshot.dependencies.len() as u64,
        targets: recipe.targets,
        tests: recipe.tests,
    })
}

fn minimal_recipe(
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
) -> Result<ProjectRecipe, Diagnostic> {
    Ok(ProjectRecipe {
        snapshot: empty_snapshot(repository, package, package_name),
        transports: Vec::new(),
        template: "minimal",
        targets: 0,
        tests: 0,
    })
}

fn command_recipe(
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
) -> Result<ProjectRecipe, Diagnostic> {
    let standard = super::builtin_standard::BuiltinStandard::load()?;
    let (text_from_static, static_text_type, text_type) = standard.command_text_signature()?;
    let seed = repository.bytes();
    let module = ModuleId::migrate(&seed, 0);
    let function = DeclarationId::migrate(&seed, 0);
    let component = DeclarationId::migrate(&seed, 1);
    let test = DeclarationId::migrate(&seed, 2);
    let literal = ExpressionId::migrate(&seed, 0);
    let body = ExpressionId::migrate(&seed, 1);
    let test_actual = ExpressionId::migrate(&seed, 2);
    let test_expected = ExpressionId::migrate(&seed, 3);
    let port = PortId::migrate(&seed, 0);
    let target = TargetId::migrate(&seed, 0);

    let mut interner = TypeObjectInterner::default();
    let local_text = interner.intern(TypeForm::Text)?;
    if local_text != text_type
        || !matches!(
            standard
                .interface_types
                .get(&static_text_type)
                .map(|value| &value.form),
            Some(TypeForm::StaticText)
        )
    {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_command_standard_types",
            "built-in standard primitive types disagree with canonical Graph 5 types",
        ));
    }
    let function_type = interner.intern(TypeForm::Function {
        parameters: Vec::new(),
        result: text_type,
    })?;

    let mut owners = BTreeMap::new();
    insert_owner(
        &mut owners,
        OwnerRecord::Module(super::kernel::ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("application")?,
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            literal,
            ExpressionOperation::StaticText {
                value: TextValue::Inline {
                    text: "hello".to_owned(),
                },
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            body,
            ExpressionOperation::Call {
                function: text_from_static,
                type_arguments: Vec::new(),
                arguments: vec![literal],
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(function), OwnerKind::PureFunction),
            module,
            name: Name::new("greet")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: text_type,
                effect: FunctionEffect::Pure,
                body,
            }),
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Port(PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component,
            name: Name::new("main")?,
            function_type,
            implementation: PortImplementation::Function(DeclarationReference {
                package,
                declaration: function,
            }),
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(component), OwnerKind::Component),
            module,
            name: Name::new("application")?,
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::Component {
                requirements: Vec::new(),
                ports: vec![port],
            },
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Target(TargetRecord {
            header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
            name: Name::new("main")?,
            component: DeclarationReference {
                package,
                declaration: component,
            },
            port: PortReference { package, port },
            runner: RunnerKind::Command,
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            test_actual,
            ExpressionOperation::Call {
                function: DeclarationReference {
                    package,
                    declaration: function,
                },
                type_arguments: Vec::new(),
                arguments: Vec::new(),
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            test_expected,
            ExpressionOperation::Text {
                value: TextValue::Inline {
                    text: "hello".to_owned(),
                },
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(test), OwnerKind::Test),
            module,
            name: Name::new("main-returns-hello")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Test {
                actual: test_actual,
                expected: test_expected,
                comparison: ComparisonPolicy::Exact,
            },
        }),
    )?;

    let dependency = DependencyRecord {
        graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: standard.package,
        semantic_revision: standard.semantic_revision,
        package_revision: standard.package_revision,
    };
    let dependency_interfaces = BTreeMap::from([(
        standard.package_revision,
        standard
            .interface_owners
            .iter()
            .map(|(owner, value)| (*owner, value.record.clone()))
            .collect(),
    )]);
    let dependency_types = standard.interface_types.clone();
    let owners_len = owners.len();
    Ok(ProjectRecipe {
        snapshot: KernelSnapshot {
            root: SemanticRoot {
                graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: repository,
                package_id: package,
                package_name,
                owners: placeholder_map(owners_len),
                dependencies: placeholder_map(1),
                retirements: placeholder_map(0),
            },
            owners,
            types: interner.into_objects(),
            dependency_interfaces,
            dependency_types,
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::from([(standard.package, dependency)]),
            retirements: BTreeMap::new(),
        },
        transports: vec![standard.transport()],
        template: "command",
        targets: 1,
        tests: 1,
    })
}

fn insert_owner(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    record: OwnerRecord,
) -> Result<(), Diagnostic> {
    let owner = record.owner();
    if owners.insert(owner, record).is_some() {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_recipe_owner_duplicate",
            format!("typed project recipe repeats owner {owner}"),
        ));
    }
    Ok(())
}

fn placeholder_map(entries: usize) -> MapRoot {
    MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        u64::try_from(entries).unwrap_or(u64::MAX),
        MapContentDigest::from_bytes([0; 32]),
    )
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

fn sync_directory(path: &Path) -> Result<(), Diagnostic> {
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

fn remove_owned_stage(stage: &Path) {
    let _ = fs::remove_dir_all(stage);
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
    fn command_recipe_prepares_checks_and_runs_through_both_normalized_tiers() {
        let temporary = tempfile::TempDir::new().expect("temporary command parent");
        let destination = temporary.path().join("command");
        let created = create_command_project(&destination, "command").expect("command project");
        assert_eq!(created.template, "command");
        assert_eq!(created.dependencies, 1);
        assert_eq!(created.targets, 1);
        assert_eq!(created.tests, 1);

        let prepared = super::super::normalized_lifecycle::prepare_application(&destination)
            .expect("prepare normalized command");
        let control = super::super::execution::ExecutionControl::default();
        let checked = prepared.check(&control).expect("check command graph tests");
        assert_eq!(checked.passed, 8);
        assert_eq!(checked.failed, 0);
        assert_eq!(checked.differential, "equal");
        let run = prepared
            .run(
                &Name::new("main").expect("target name"),
                b"[]",
                super::super::execution::normalized::NormalizedCommandPolicy::default(),
                &control,
            )
            .expect("run normalized command");
        assert_eq!(run.result_json, b"\"hello\"");
        assert_eq!(run.differential, "equal");
    }

    #[test]
    fn failed_visibility_rename_removes_only_the_owned_private_stage() {
        let temporary = tempfile::TempDir::new().expect("temporary creation parent");
        let destination = temporary.path().join("raced");
        let error =
            create_minimal_project_before_publish(&destination, "raced", |visible_destination| {
                fs::create_dir(visible_destination).map_err(|error| {
                    creation_error(
                        DiagnosticClass::Infrastructure,
                        "test_destination_race",
                        error.to_string(),
                    )
                })?;
                fs::write(visible_destination.join("owned.txt"), b"preserve\n").map_err(
                    |error| {
                        creation_error(
                            DiagnosticClass::Infrastructure,
                            "test_destination_race",
                            error.to_string(),
                        )
                    },
                )?;
                Ok(())
            })
            .expect_err("nonempty raced destination must reject publication");
        assert_eq!(error.code, "new_destination_publish");
        assert_eq!(
            fs::read(destination.join("owned.txt")).expect("preserved destination"),
            b"preserve\n"
        );
        let stages = fs::read_dir(temporary.path())
            .expect("creation parent")
            .map(|entry| entry.expect("parent entry").file_name())
            .filter(|name| {
                name.to_string_lossy()
                    .starts_with(".lkjscript-project-stage-")
            })
            .collect::<Vec<_>>();
        assert!(stages.is_empty(), "owned private stages remain: {stages:?}");
    }
}
