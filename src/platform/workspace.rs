//! Bounded public workspace discovery and exact dependency loading.

use super::artifact::{ArtifactReceipt, MAXIMUM_ARTIFACT_BYTES, build_artifact, load_artifact};
use super::authority::{ApplyReceipt, DependencyArtifact, ProjectAuthority, WorkingStatus};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::language::Declaration;
use super::package::{PACKAGE_FILE_NAME, PackageDescriptor, PackageId, decode_package};
use super::semantic::ValidatedPackage;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const WORKSPACE_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceOrientation {
    pub contract_version: u16,
    pub root: String,
    pub package_id: PackageId,
    pub package_name: String,
    pub revision: u64,
    pub record_digest: String,
    pub authored_digest: String,
    pub semantic_digest: String,
    pub modules: Vec<ModuleOrientation>,
    pub dependencies: Vec<DependencyOrientation>,
    pub targets: Vec<TargetOrientation>,
    pub expansion_commands: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleOrientation {
    pub name: String,
    pub path: String,
    pub declarations: usize,
    pub exports: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyOrientation {
    pub alias: String,
    pub package_id: PackageId,
    pub revision_digest: String,
    pub artifact_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetOrientation {
    pub name: String,
    pub component: String,
    pub port: String,
    pub runner: super::package::RunnerKind,
}

#[derive(Clone, Debug)]
pub struct SourceWorkspace {
    root: PathBuf,
    authority: ProjectAuthority,
}

impl SourceWorkspace {
    pub fn discover(start: &Path) -> Result<PathBuf, Diagnostic> {
        let mut current = canonical_start(start)?;
        if current.is_file() {
            current.pop();
        }
        loop {
            if current.join(PACKAGE_FILE_NAME).is_file() {
                return Ok(current);
            }
            if !current.pop() {
                return Err(workspace_error(
                    DiagnosticClass::Source,
                    "workspace_not_found",
                    format!(
                        "no '{}' was found from '{}' to the filesystem root",
                        PACKAGE_FILE_NAME,
                        start.display()
                    ),
                ));
            }
        }
    }

    pub fn initialize(root: &Path) -> Result<(Self, ApplyReceipt), Diagnostic> {
        let root = Self::discover(root)?;
        let descriptor = read_working_descriptor(&root)?;
        let dependencies = read_dependency_artifacts(&root, &descriptor)?;
        let (authority, receipt) = ProjectAuthority::initialize(&root, &dependencies)?;
        Ok((Self { root, authority }, receipt))
    }

    pub fn open(start: &Path) -> Result<Self, Diagnostic> {
        let root = Self::discover(start)?;
        let authority = ProjectAuthority::open(&root)?;
        Ok(Self { root, authority })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn authority(&self) -> &ProjectAuthority {
        &self.authority
    }

    pub fn status(&self) -> Result<WorkingStatus, Diagnostic> {
        let descriptor = read_working_descriptor(&self.root)?;
        let dependencies = read_dependency_artifacts(&self.root, &descriptor)?;
        self.authority.status(&dependencies)
    }

    pub fn validate(
        &self,
        expected_revision: u64,
        expected_record_digest: &str,
    ) -> Result<ApplyReceipt, Diagnostic> {
        let descriptor = read_working_descriptor(&self.root)?;
        let dependencies = read_dependency_artifacts(&self.root, &descriptor)?;
        self.authority
            .validate_working(expected_revision, expected_record_digest, &dependencies)
    }

    pub fn apply(
        &self,
        expected_revision: u64,
        expected_record_digest: &str,
    ) -> Result<ApplyReceipt, Diagnostic> {
        let descriptor = read_working_descriptor(&self.root)?;
        let dependencies = read_dependency_artifacts(&self.root, &descriptor)?;
        self.authority
            .apply_working(expected_revision, expected_record_digest, &dependencies)
    }

    pub fn orient(&self) -> Result<WorkspaceOrientation, Diagnostic> {
        let current = self.authority.current()?;
        let mut modules = current
            .package
            .modules
            .iter()
            .map(|module| ModuleOrientation {
                name: module.module.name.clone(),
                path: module.path.clone(),
                declarations: module.module.declarations.len(),
                exports: module.module.exports.len(),
            })
            .collect::<Vec<_>>();
        modules.sort_by(|left, right| left.name.cmp(&right.name));
        let mut dependencies = current
            .package
            .descriptor
            .dependencies
            .iter()
            .map(|dependency| DependencyOrientation {
                alias: dependency.alias.clone(),
                package_id: dependency.package_id.clone(),
                revision_digest: dependency.revision_digest.clone(),
                artifact_digest: dependency.artifact_digest.clone(),
            })
            .collect::<Vec<_>>();
        dependencies.sort_by(|left, right| left.alias.cmp(&right.alias));
        let mut targets = current
            .package
            .descriptor
            .targets
            .iter()
            .map(|target| TargetOrientation {
                name: target.name.clone(),
                component: target.component.clone(),
                port: target.port.clone(),
                runner: target.runner,
            })
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(WorkspaceOrientation {
            contract_version: WORKSPACE_CONTRACT_VERSION,
            root: self.root.display().to_string(),
            package_id: current.package.descriptor.package_id.clone(),
            package_name: current.package.descriptor.name.clone(),
            revision: current.revision,
            record_digest: current.record_digest,
            authored_digest: current.authored_digest,
            semantic_digest: current.package.revision_digest,
            modules,
            dependencies,
            targets,
            expansion_commands: vec![
                "lkjscript module list --project <root>".to_owned(),
                "lkjscript module show <module> --project <root>".to_owned(),
                "lkjscript component inspect <component> --project <root>".to_owned(),
                "lkjscript target list --project <root>".to_owned(),
            ],
        })
    }

    pub fn module_source(&self, name: &str) -> Result<Vec<u8>, Diagnostic> {
        let current = self.authority.current()?;
        let module = current
            .package
            .modules
            .iter()
            .find(|module| module.module.name == name)
            .ok_or_else(|| {
                workspace_error(
                    DiagnosticClass::Source,
                    "workspace_module_missing",
                    format!("package has no module '{name}'"),
                )
            })?;
        current.files.get(&module.path).cloned().ok_or_else(|| {
            workspace_error(
                DiagnosticClass::Corrupt,
                "workspace_module_source",
                format!("accepted module '{}' has no retained source", module.path),
            )
        })
    }

    pub fn build_artifact(&self) -> Result<(Vec<u8>, ArtifactReceipt), Diagnostic> {
        let current = self.authority.current()?;
        let mut closure: BTreeMap<PackageId, ValidatedPackage> = BTreeMap::new();
        for dependency in &current.dependencies {
            let loaded = load_artifact(&dependency.bytes)?;
            for (identity, package) in loaded.packages {
                match closure.get(&identity) {
                    Some(existing) if existing.revision_digest != package.revision_digest => {
                        return Err(workspace_error(
                            DiagnosticClass::Semantic,
                            "workspace_dependency_conflict",
                            format!(
                                "artifact closure contains two revisions of package '{}'",
                                identity.as_str()
                            ),
                        ));
                    }
                    Some(_) => {}
                    None => {
                        closure.insert(identity, package);
                    }
                }
            }
        }
        closure.insert(
            current.package.descriptor.package_id.clone(),
            current.package.clone(),
        );
        let root = closure
            .get(&current.package.descriptor.package_id)
            .ok_or_else(|| {
                workspace_error(
                    DiagnosticClass::Infrastructure,
                    "workspace_build_root",
                    "artifact build lost its root package",
                )
            })?;
        let packages = closure.values().collect::<Vec<_>>();
        build_artifact(root, &packages)
    }

    pub fn declaration_names(&self, module_name: &str) -> Result<Vec<String>, Diagnostic> {
        let current = self.authority.current()?;
        let module = current
            .package
            .modules
            .iter()
            .find(|module| module.module.name == module_name)
            .ok_or_else(|| {
                workspace_error(
                    DiagnosticClass::Source,
                    "workspace_module_missing",
                    format!("package has no module '{module_name}'"),
                )
            })?;
        Ok(module
            .module
            .declarations
            .iter()
            .map(Declaration::name)
            .map(str::to_owned)
            .collect())
    }
}

pub fn read_working_descriptor(root: &Path) -> Result<PackageDescriptor, Diagnostic> {
    let bytes = read_regular_bounded(
        &root.join(PACKAGE_FILE_NAME),
        super::package::MAXIMUM_PACKAGE_BYTES,
        "package descriptor",
    )?;
    decode_package(&bytes)
}

pub fn read_dependency_artifacts(
    root: &Path,
    descriptor: &PackageDescriptor,
) -> Result<Vec<DependencyArtifact>, Diagnostic> {
    descriptor
        .dependencies
        .iter()
        .map(|dependency| {
            let path = confined_path(root, &dependency.artifact)?;
            let bytes = read_regular_bounded(&path, MAXIMUM_ARTIFACT_BYTES, "dependency artifact")?;
            DependencyArtifact::decode(dependency.alias.clone(), bytes)
        })
        .collect()
}

fn canonical_start(start: &Path) -> Result<PathBuf, Diagnostic> {
    let metadata = fs::symlink_metadata(start)
        .map_err(|error| io_error("workspace_start_metadata", start, error))?;
    if metadata.file_type().is_symlink() {
        return Err(workspace_error(
            DiagnosticClass::Source,
            "workspace_start_symlink",
            "workspace discovery start may not be a symbolic link",
        ));
    }
    fs::canonicalize(start).map_err(|error| io_error("workspace_start_canonicalize", start, error))
}

fn confined_path(root: &Path, relative: &str) -> Result<PathBuf, Diagnostic> {
    super::package::validate_relative_path(relative, "dependency artifact locator", false)?;
    let mut path = root.to_path_buf();
    for component in relative.split('/') {
        path.push(component);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| io_error("workspace_dependency_metadata", &path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(workspace_error(
                DiagnosticClass::Source,
                "workspace_dependency_symlink",
                format!(
                    "dependency artifact path component '{}' is a symbolic link",
                    path.display()
                ),
            ));
        }
    }
    Ok(path)
}

fn read_regular_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("workspace_read_metadata", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(workspace_error(
            DiagnosticClass::Source,
            "workspace_read_type",
            format!(
                "{label} '{}' is not a regular non-symlink file",
                path.display()
            ),
        ));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| {
        workspace_error(
            DiagnosticClass::Resource,
            "workspace_read_length",
            format!("{label} length cannot be represented"),
        )
    })?;
    if length > maximum {
        return Err(workspace_error(
            DiagnosticClass::Resource,
            "workspace_read_limit",
            format!("{label} has {length} bytes; the limit is {maximum}"),
        ));
    }
    let file = File::open(path).map_err(|error| io_error("workspace_read_open", path, error))?;
    let mut bytes = Vec::with_capacity(length);
    file.take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("workspace_read", path, error))?;
    if bytes.len() != length || bytes.len() > maximum {
        return Err(workspace_error(
            DiagnosticClass::Resource,
            "workspace_read_changed",
            format!("{label} changed while it was read"),
        ));
    }
    Ok(bytes)
}

fn workspace_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn io_error(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    workspace_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn nearest_workspace_is_discovered_and_oriented_compactly() {
        let temporary = TempDir::new().expect("temporary project");
        fs::create_dir_all(temporary.path().join("src/nested")).expect("source directories");
        fs::write(
            temporary.path().join(PACKAGE_FILE_NAME),
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[]}
"#,
        )
        .expect("descriptor");
        fs::write(
            temporary.path().join("src/main.lkj"),
            b"(module main (export Item) (record Item (name Text)))\n",
        )
        .expect("module");
        let (workspace, _) = SourceWorkspace::initialize(temporary.path()).expect("initialize");
        assert_eq!(
            SourceWorkspace::discover(&temporary.path().join("src/nested"))
                .expect("discover nested"),
            fs::canonicalize(temporary.path()).expect("canonical root")
        );
        let orientation = workspace.orient().expect("orientation");
        assert_eq!(orientation.modules.len(), 1);
        assert_eq!(orientation.modules[0].declarations, 1);
        assert_eq!(
            workspace.declaration_names("main").expect("names"),
            ["Item"]
        );
    }
}
