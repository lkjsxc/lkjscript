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
    if bytes.len() > maximum_bytes {
        return Err(output_error(
            DiagnosticClass::Resource,
            "output_byte_limit",
            format!("{label} exceeds its {maximum_bytes}-byte output bound"),
        ));
    }
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

    let stage = parent.join(format!(".{file_name}.stage-{}", RepositoryId::generate()?));
    let staged = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage)
            .map_err(|error| io_error("output_stage_create", &stage, error))?;
        file.write_all(bytes)
            .map_err(|error| io_error("output_stage_write", &stage, error))?;
        file.sync_all()
            .map_err(|error| io_error("output_stage_sync", &stage, error))?;
        fs::hard_link(&stage, &output).map_err(|error| output_link_error(&output, error))?;
        Ok::<(), Diagnostic>(())
    })();
    if let Err(error) = staged {
        let _ = fs::remove_file(&stage);
        return Err(error);
    }

    let durability = if sync_directory(&parent).is_ok() {
        "synchronized"
    } else {
        "uncertain"
    };
    let stage_cleanup = if fs::remove_file(&stage).is_ok() {
        "removed"
    } else {
        "retained"
    };
    let durability = if sync_directory(&parent).is_ok() {
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

fn reject_symlinked_path(path: &Path) -> Result<(), Diagnostic> {
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
