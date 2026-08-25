//! Atomic creation of one minimal normalized semantic-graph project.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    KernelSnapshot, Name, PackageId, SemanticRoot, SemanticRootDigest, SemanticStateDigest,
};
use super::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use super::publication::{GraphRepository, ReceiptObjectDigest, RevisionObjectDigest};
use super::semantic_id::{RepositoryId, RevisionId};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

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
}

pub fn create_minimal_project(
    destination: &Path,
    package_name: &str,
) -> Result<MinimalProjectCreation, Diagnostic> {
    create_minimal_project_before_publish(destination, package_name, |_| Ok(()))
}

fn create_minimal_project_before_publish<F>(
    destination: &Path,
    package_name: &str,
    before_publish: F,
) -> Result<MinimalProjectCreation, Diagnostic>
where
    F: FnOnce(&Path) -> Result<(), Diagnostic>,
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
    let logical = empty_snapshot(repository, package, package_name.clone());
    let private = parent.join(format!(".lkjscript-project-stage-{repository}"));

    let created = GraphRepository::create(
        &private,
        &logical,
        Some("public minimal project bootstrap".to_owned()),
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
    })
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
        Ok(_) if directory_is_empty(&destination)? => Ok(destination),
        Ok(_) => Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_not_empty",
            format!(
                "project destination '{}' is not empty",
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

fn directory_is_empty(path: &Path) -> Result<bool, Diagnostic> {
    let mut entries = fs::read_dir(path).map_err(|error| {
        creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_read",
            format!(
                "project destination '{}' could not be read: {error}",
                path.display()
            ),
        )
    })?;
    entries
        .next()
        .transpose()
        .map(|entry| entry.is_none())
        .map_err(|error| {
            creation_error(
                DiagnosticClass::Infrastructure,
                "new_destination_read",
                format!(
                    "project destination '{}' could not be read: {error}",
                    path.display()
                ),
            )
        })
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
