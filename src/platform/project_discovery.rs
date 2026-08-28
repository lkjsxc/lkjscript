//! Safe discovery of the current normalized repository authority.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::publication::GraphRepository;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Finds the nearest normalized repository without following a symbolic-link component or
/// consulting a predecessor reader.
pub(crate) fn discover_project(start: &Path) -> Result<GraphRepository, Diagnostic> {
    let mut current = ordinary_absolute_directory(start)?;
    loop {
        let normalized = marker_exists(&current.join("HEAD"))?;
        let predecessor = marker_exists(&current.join(".lkjscript"))?
            || marker_exists(&current.join("lkjscript.package.json"))?;

        if predecessor {
            return Err(discovery_error(
                DiagnosticClass::Source,
                "predecessor_contract",
                format!(
                    "project discovery found predecessor authority at '{}'; only the current semantic repository contract is accepted",
                    current.display()
                ),
            ));
        }
        if normalized {
            let metadata = fs::symlink_metadata(current.join("HEAD")).map_err(|error| {
                discovery_error(
                    DiagnosticClass::Infrastructure,
                    "project_io",
                    format!(
                        "normalized project marker at '{}' could not be inspected: {error}",
                        current.display()
                    ),
                )
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(discovery_error(
                    DiagnosticClass::Corrupt,
                    "project_marker",
                    format!(
                        "normalized project marker at '{}' is not an ordinary file",
                        current.display()
                    ),
                ));
            }
            return GraphRepository::open(&current);
        }
        if !current.pop() {
            return Err(discovery_error(
                DiagnosticClass::Source,
                "project_not_found",
                format!(
                    "no current semantic repository was found from '{}' to the filesystem root",
                    start.display()
                ),
            ));
        }
    }
}

fn ordinary_absolute_directory(path: &Path) -> Result<PathBuf, Diagnostic> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                discovery_error(
                    DiagnosticClass::Infrastructure,
                    "project_io",
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
                return Err(discovery_error(
                    DiagnosticClass::Source,
                    "project_path",
                    "project discovery path may not contain '..'",
                ));
            }
            Component::Normal(value) => {
                checked.push(value);
                let metadata = fs::symlink_metadata(&checked).map_err(|error| {
                    let class = if error.kind() == std::io::ErrorKind::NotFound {
                        DiagnosticClass::Source
                    } else {
                        DiagnosticClass::Infrastructure
                    };
                    discovery_error(
                        class,
                        if class == DiagnosticClass::Source {
                            "project_path"
                        } else {
                            "project_io"
                        },
                        format!(
                            "project discovery path '{}' could not be inspected: {error}",
                            checked.display()
                        ),
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(discovery_error(
                        DiagnosticClass::Source,
                        "project_path",
                        format!(
                            "project discovery path '{}' traverses a symbolic link",
                            checked.display()
                        ),
                    ));
                }
            }
        }
    }
    let metadata = fs::symlink_metadata(&checked).map_err(|error| {
        discovery_error(
            DiagnosticClass::Source,
            "project_path",
            format!(
                "project discovery path '{}' could not be inspected: {error}",
                checked.display()
            ),
        )
    })?;
    if !metadata.is_dir() {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "project_path",
            format!(
                "project discovery path '{}' is not an ordinary directory",
                checked.display()
            ),
        ));
    }
    Ok(checked)
}

fn marker_exists(path: &Path) -> Result<bool, Diagnostic> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(discovery_error(
            DiagnosticClass::Infrastructure,
            "project_io",
            format!(
                "project marker '{}' could not be inspected: {error}",
                path.display()
            ),
        )),
    }
}

fn discovery_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::project_creation::{ProjectTemplate, create_project};

    #[test]
    fn discovers_from_root_and_nested_directory() {
        let temporary = tempfile::TempDir::new().expect("temporary parent");
        let project = temporary.path().join("project");
        let created = create_project(&project, "project", ProjectTemplate::Minimal)
            .expect("normalized project");
        let nested = project.join("ordinary/nested");
        fs::create_dir_all(&nested).expect("nested directory");

        for start in [&project, &nested] {
            let repository = discover_project(start).expect("discovered repository");
            assert_eq!(repository.root(), project.canonicalize().unwrap());
            assert_eq!(
                repository.current().unwrap().head.revision,
                created.revision
            );
        }
    }

    #[test]
    fn rejects_predecessor_and_missing_authority() {
        let temporary = tempfile::TempDir::new().expect("temporary parent");
        let predecessor = temporary.path().join("predecessor");
        fs::create_dir_all(predecessor.join(".lkjscript/source-v1")).expect("predecessor marker");
        let error = discover_project(&predecessor).expect_err("predecessor rejected");
        assert_eq!(error.code, "predecessor_contract");

        let manifest = temporary.path().join("manifest-predecessor");
        fs::create_dir(&manifest).expect("manifest predecessor directory");
        fs::write(manifest.join("lkjscript.package.json"), b"{}\n")
            .expect("manifest predecessor marker");
        let error = discover_project(&manifest).expect_err("manifest predecessor rejected");
        assert_eq!(error.code, "predecessor_contract");

        let ambiguous = temporary.path().join("ambiguous");
        create_project(&ambiguous, "ambiguous", ProjectTemplate::Minimal)
            .expect("normalized project");
        fs::create_dir(ambiguous.join(".lkjscript")).expect("foreign predecessor marker");
        let error = discover_project(&ambiguous).expect_err("ambiguous authority rejected");
        assert_eq!(error.code, "predecessor_contract");

        let ordinary = temporary.path().join("ordinary");
        fs::create_dir(&ordinary).expect("ordinary directory");
        let error = discover_project(&ordinary).expect_err("missing project rejected");
        assert_eq!(error.code, "project_not_found");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_components_without_following_them() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::TempDir::new().expect("temporary parent");
        let project = temporary.path().join("project");
        create_project(&project, "project", ProjectTemplate::Minimal).expect("normalized project");
        let link = temporary.path().join("project-link");
        symlink(&project, &link).expect("project symlink");

        let error = discover_project(&link).expect_err("symlink rejected");
        assert_eq!(error.code, "project_path");

        let actual_nested = project.join("actual-nested");
        fs::create_dir(&actual_nested).expect("actual nested directory");
        let linked_nested = project.join("linked-nested");
        symlink(&actual_nested, &linked_nested).expect("nested symlink");
        let error = discover_project(&linked_nested).expect_err("nested symlink rejected");
        assert_eq!(error.code, "project_path");
    }
}
