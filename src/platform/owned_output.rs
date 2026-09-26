//! Create-new publication of bounded immutable derived output.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::semantic_id::RepositoryId;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedOutputReceipt {
    pub path: PathBuf,
    pub bytes: u64,
    pub visibility: &'static str,
    pub durability: &'static str,
    pub stage_cleanup: &'static str,
}

pub fn publish_create_new(
    path: &Path,
    bytes: &[u8],
    maximum_bytes: usize,
    label: &str,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    publish_with_access(path, bytes, maximum_bytes, label, false)
}

/// Create the private stage with owner-only access before writing any bytes.
/// Never broaden, chmod or take ownership of an existing destination.
pub(crate) fn publish_private_create_new(
    path: &Path,
    bytes: &[u8],
    maximum_bytes: usize,
    label: &str,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    publish_with_access(path, bytes, maximum_bytes, label, true)
}

fn publish_with_access(
    path: &Path,
    bytes: &[u8],
    maximum_bytes: usize,
    label: &str,
    owner_only: bool,
) -> Result<OwnedOutputReceipt, Diagnostic> {
    if bytes.len() > maximum_bytes {
        return Err(output_error(
            DiagnosticClass::Resource,
            "output_byte_limit",
            format!("{label} exceeds its {maximum_bytes}-byte output bound"),
        ));
    }
    let output = inspect_create_new(path)?;
    let parent = output.parent().ok_or_else(|| {
        output_error(
            DiagnosticClass::Source,
            "output_parent",
            "output has no parent",
        )
    })?;
    // A legal destination name can already use the filesystem's full component bound.
    // The owned stage must not append to that name and fail only after invocation.
    let stage = parent.join(format!(
        ".lkjscript-output-stage-{}",
        RepositoryId::generate()?
    ));
    let mut stage_created = false;
    let staged = (|| {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        if owner_only {
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            #[cfg(not(unix))]
            {
                return Err(output_error(
                    DiagnosticClass::Source,
                    "output_private_access",
                    "owner-only output publication requires POSIX file permissions",
                ));
            }
        }
        let mut file = options
            .open(&stage)
            .map_err(|error| io_error("output_stage_create", &stage, error))?;
        stage_created = true;
        file.write_all(bytes)
            .map_err(|error| io_error("output_stage_write", &stage, error))?;
        file.sync_all()
            .map_err(|error| io_error("output_stage_sync", &stage, error))?;
        fs::hard_link(&stage, &output).map_err(|error| output_link_error(&output, error))?;
        Ok::<(), Diagnostic>(())
    })();
    if let Err(mut error) = staged {
        if stage_created
            && let Err(cleanup) = fs::remove_file(&stage)
            && cleanup.kind() != std::io::ErrorKind::NotFound
        {
            error.notes.push(format!(
                "owned output stage '{}' could not be removed: {cleanup}",
                stage.display()
            ));
        }
        return Err(error);
    }

    let durability = if sync_directory(parent).is_ok() {
        "synchronized"
    } else {
        "uncertain"
    };
    let stage_cleanup = if fs::remove_file(&stage).is_ok() {
        "removed"
    } else {
        "retained"
    };
    let durability = if sync_directory(parent).is_ok() {
        durability
    } else {
        "uncertain"
    };
    Ok(OwnedOutputReceipt {
        path: output,
        bytes: bytes.len() as u64,
        visibility: "created",
        durability,
        stage_cleanup,
    })
}

/// Inspect an absent destination without creating or reserving resources. Publication
/// rechecks it: this observation does not promise later writability, capacity or absence.
pub fn inspect_create_new(path: &Path) -> Result<PathBuf, Diagnostic> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            output_error(
                DiagnosticClass::Source,
                "output_name",
                "output path must have a portable UTF-8 file name",
            )
        })?;
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    reject_symlinked_path(parent)?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        output_error(
            DiagnosticClass::Source,
            "output_parent",
            format!(
                "output parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(output_error(
            DiagnosticClass::Source,
            "output_parent_type",
            "output parent is not an ordinary directory",
        ));
    }
    let parent = parent.canonicalize().map_err(|error| {
        output_error(
            DiagnosticClass::Source,
            "output_parent",
            format!(
                "output parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    let output = parent.join(file_name);
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            return Err(output_error(
                DiagnosticClass::Source,
                "output_conflict",
                format!("output '{}' already exists", output.display()),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(output_error(
                DiagnosticClass::Infrastructure,
                "output_inspect",
                format!(
                    "output '{}' could not be inspected: {error}",
                    output.display()
                ),
            ));
        }
    }

    Ok(output)
}

pub(crate) fn reject_symlinked_path(path: &Path) -> Result<(), Diagnostic> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                output_error(
                    DiagnosticClass::Infrastructure,
                    "output_current_directory",
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
                return Err(output_error(
                    DiagnosticClass::Source,
                    "output_traversal",
                    "output parent may not contain '..'",
                ));
            }
            Component::Normal(value) => {
                checked.push(value);
                let metadata = fs::symlink_metadata(&checked).map_err(|error| {
                    output_error(
                        DiagnosticClass::Source,
                        "output_parent",
                        format!(
                            "output parent '{}' is unavailable: {error}",
                            checked.display()
                        ),
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(output_error(
                        DiagnosticClass::Source,
                        "output_symlink",
                        format!(
                            "output parent '{}' traverses a symbolic link",
                            checked.display()
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), std::io::Error> {
    File::open(path).and_then(|directory| directory.sync_all())
}

fn output_link_error(path: &Path, error: std::io::Error) -> Diagnostic {
    let (class, code) = if error.kind() == std::io::ErrorKind::AlreadyExists {
        (DiagnosticClass::Source, "output_conflict")
    } else {
        (DiagnosticClass::Infrastructure, "output_publish")
    };
    output_error(
        class,
        code,
        format!("output '{}' could not be created: {error}", path.display()),
    )
}

fn io_error(code: &'static str, path: &Path, error: std::io::Error) -> Diagnostic {
    output_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("owned output '{}' failed: {error}", path.display()),
    )
}

fn output_error(
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
    fn inspection_reserves_nothing_and_publication_rechecks_the_destination() {
        let temporary = tempfile::TempDir::new().expect("temporary output parent");
        let path = temporary.path().join("result.json");
        let inspected = inspect_create_new(&path).expect("inspect absent result");
        assert_eq!(fs::read_dir(temporary.path()).expect("parent").count(), 0);
        fs::write(&path, b"competing output").expect("competing writer");
        let error = publish_create_new(&inspected, b"new output", 16, "test result")
            .expect_err("late conflict");
        assert_eq!(error.code, "output_conflict");
        assert_eq!(
            fs::read(&path).expect("preserved bytes"),
            b"competing output"
        );
        assert_eq!(fs::read_dir(temporary.path()).expect("parent").count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn publication_accepts_a_full_length_destination_component() {
        let temporary = tempfile::TempDir::new().expect("temporary output parent");
        let output = temporary.path().join("x".repeat(255));
        let receipt = publish_create_new(&output, b"complete", 8, "test output")
            .expect("stage name does not extend the destination component");
        assert_eq!(receipt.stage_cleanup, "removed");
        assert_eq!(fs::read(&output).expect("read output"), b"complete");
        assert_eq!(fs::read_dir(temporary.path()).expect("parent").count(), 1);
    }

    #[test]
    fn publication_is_create_new_and_preserves_existing_bytes() {
        let temporary = tempfile::TempDir::new().expect("temporary output parent");
        let output = temporary.path().join("application.lkja");
        let receipt = publish_create_new(&output, b"complete", 8, "test output")
            .expect("publish create-new output");
        assert_eq!(receipt.visibility, "created");
        assert_eq!(fs::read(&output).expect("read output"), b"complete");
        let error = publish_create_new(&output, b"replacement", 16, "test output")
            .expect_err("existing output must be rejected");
        assert_eq!(error.code, "output_conflict");
        assert_eq!(fs::read(&output).expect("preserved output"), b"complete");
    }

    #[test]
    fn publication_rejects_invalid_parents_and_directories_as_outputs() {
        let temporary = tempfile::TempDir::new().expect("temporary output parent");
        let missing = temporary.path().join("missing/application.lkja");
        let error = publish_create_new(&missing, b"bytes", 8, "test output")
            .expect_err("missing parent must be rejected");
        assert_eq!(error.code, "output_parent");
        assert!(!missing.exists());

        let parent_file = temporary.path().join("ordinary-file");
        fs::write(&parent_file, b"preserve").expect("parent file");
        let error = publish_create_new(
            &parent_file.join("application.lkja"),
            b"bytes",
            8,
            "test output",
        )
        .expect_err("non-directory parent must be rejected");
        assert_eq!(error.code, "output_parent_type");
        assert_eq!(
            fs::read(&parent_file).expect("preserved parent file"),
            b"preserve"
        );

        let directory_output = temporary.path().join("existing-directory");
        fs::create_dir(&directory_output).expect("output directory");
        let error = publish_create_new(&directory_output, b"bytes", 8, "test output")
            .expect_err("directory output must be rejected");
        assert_eq!(error.code, "output_conflict");
        assert!(directory_output.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn publication_rejects_symlinked_parents_and_outputs() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::TempDir::new().expect("temporary output parent");
        let real_parent = temporary.path().join("real");
        fs::create_dir(&real_parent).expect("real output parent");
        let linked_parent = temporary.path().join("linked");
        symlink(&real_parent, &linked_parent).expect("linked output parent");
        let error = publish_create_new(
            &linked_parent.join("application.lkja"),
            b"bytes",
            8,
            "test output",
        )
        .expect_err("symlinked parent must be rejected");
        assert_eq!(error.code, "output_symlink");
        assert!(!real_parent.join("application.lkja").exists());

        let existing = temporary.path().join("existing.lkja");
        fs::write(&existing, b"preserve").expect("symlink target");
        let linked_output = temporary.path().join("application.lkja");
        symlink(&existing, &linked_output).expect("linked output");
        let error = publish_create_new(&linked_output, b"bytes", 8, "test output")
            .expect_err("symlink output must be rejected");
        assert_eq!(error.code, "output_conflict");
        assert_eq!(fs::read(&existing).expect("preserved target"), b"preserve");
    }
}
