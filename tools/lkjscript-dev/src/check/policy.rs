mod native;

use super::snapshot;
use crate::error::DevError;
use serde::Serialize;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path};

const MAXIMUM_SHEBANG_BYTES: u64 = 512;
// At most 32,768 raw observed bytes per call, encoded as bounded typed Bytes.
const OBSERVATIONS_PER_BATCH: usize = 64;
const MAXIMUM_REPORTED_VIOLATIONS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ViolationKind {
    PythonFile,
    PythonShebang,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Violation {
    path: String,
    kind: ViolationKind,
}

pub(crate) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let policy = crate::next_utf8(&mut arguments, "policy")?
        .ok_or_else(|| DevError::usage("policy name is required"))?;
    if policy == "product-surface" {
        return super::surface::command(arguments);
    }
    if policy != "no-python" {
        return Err(DevError::usage(format!("unknown policy '{policy}'")));
    }
    let mut machine = false;
    while let Some(argument) = crate::next_utf8(&mut arguments, "policy option")? {
        match argument.as_str() {
            "--machine" if !machine => machine = true,
            value => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate policy option '{value}'"
                )));
            }
        }
    }
    let repository = repository_root()?;
    let violations = inspect_repository(&repository)?;
    print_result(machine, &violations)?;
    Ok(if violations.is_empty() { 0 } else { 1 })
}

fn inspect_repository(repository: &Path) -> Result<Vec<Violation>, DevError> {
    let paths = snapshot::repository_paths(repository)?;
    inspect_paths(repository, &paths)
}

fn inspect_paths(repository: &Path, paths: &[String]) -> Result<Vec<Violation>, DevError> {
    let mut violations = Vec::new();
    for batch in paths.chunks(OBSERVATIONS_PER_BATCH) {
        let validated = batch
            .iter()
            .map(|relative| validate_relative_path(relative))
            .collect::<Result<Vec<_>, _>>()?;
        let extensions = validated
            .iter()
            .map(|path| {
                path.extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let python_files = native::extensions(&extensions)?;
        let mut prefixes = Vec::with_capacity(batch.len());
        for (path, python_file) in validated.iter().zip(&python_files) {
            // Native extension decisions retain precedence, even for missing files or links.
            prefixes.push(if *python_file {
                Vec::new()
            } else {
                observe_prefix(&repository.join(path))?
            });
        }
        let python_shebangs = native::shebangs(&prefixes)?;
        for ((relative, python_file), python_shebang) in
            batch.iter().zip(python_files).zip(python_shebangs)
        {
            let kind = if python_file {
                Some(ViolationKind::PythonFile)
            } else if python_shebang {
                Some(ViolationKind::PythonShebang)
            } else {
                None
            };
            if let Some(kind) = kind {
                violations.push(Violation {
                    path: relative.clone(),
                    kind,
                });
            }
        }
    }
    Ok(violations)
}

fn validate_relative_path(path: &str) -> Result<&Path, DevError> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(DevError::infrastructure(format!(
            "repository policy received unsafe path '{}'",
            path.display()
        )));
    }
    Ok(path)
}

fn observe_prefix(path: &Path) -> Result<Vec<u8>, DevError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "inspect repository policy path '{}': {error}",
                path.display()
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(Vec::new());
    }
    let mut bytes = Vec::with_capacity(MAXIMUM_SHEBANG_BYTES as usize);
    File::open(path)
        .and_then(|file| file.take(MAXIMUM_SHEBANG_BYTES).read_to_end(&mut bytes))
        .map_err(|error| {
            DevError::infrastructure(format!(
                "read repository policy path '{}': {error}",
                path.display()
            ))
        })?;
    Ok(bytes)
}

fn print_result(machine: bool, violations: &[Violation]) -> Result<(), DevError> {
    #[derive(Serialize)]
    struct Summary<'a> {
        contract_version: u32,
        status: &'static str,
        policy: &'static str,
        violations: usize,
        reported: &'a [Violation],
    }
    let reported = &violations[..violations.len().min(MAXIMUM_REPORTED_VIOLATIONS)];
    let summary = Summary {
        contract_version: 1,
        status: if violations.is_empty() {
            "passed"
        } else {
            "failed"
        },
        policy: "no-python",
        violations: violations.len(),
        reported,
    };
    if machine {
        println!(
            "{}",
            serde_json::to_string(&summary).map_err(|error| {
                DevError::infrastructure(format!("encode repository policy result: {error}"))
            })?
        );
    } else if violations.is_empty() {
        println!("policy passed: no first-party Python files or Python shebangs");
    } else {
        println!(
            "policy failed: {} first-party Python file or shebang violation(s)",
            violations.len()
        );
        for violation in reported {
            println!("{}: {:?}", violation.path, violation.kind);
        }
    }
    Ok(())
}

fn repository_root() -> Result<std::path::PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| DevError::infrastructure("resolve repository root"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_rejects_python_extensions_and_shebangs_without_following_links() {
        let repository = tempfile::tempdir().expect("temporary policy repository");
        fs::write(repository.path().join("script.py"), b"print('no')\n")
            .expect("write Python extension fixture");
        fs::write(
            repository.path().join("tool"),
            b"#!/usr/bin/env python3\nprint('no')\n",
        )
        .expect("write Python shebang fixture");
        fs::write(repository.path().join("rust-tool"), b"#!/usr/bin/env sh\n")
            .expect("write allowed shell fixture");
        #[cfg(unix)]
        std::os::unix::fs::symlink("missing", repository.path().join("external-link"))
            .expect("write policy symlink fixture");
        let paths = vec![
            "external-link".to_owned(),
            "rust-tool".to_owned(),
            "script.py".to_owned(),
            "tool".to_owned(),
        ];
        let violations = inspect_paths(repository.path(), &paths).expect("inspect policy fixtures");
        assert_eq!(
            violations,
            [
                Violation {
                    path: "script.py".to_owned(),
                    kind: ViolationKind::PythonFile,
                },
                Violation {
                    path: "tool".to_owned(),
                    kind: ViolationKind::PythonShebang,
                },
            ]
        );
    }

    #[test]
    fn policy_rejects_unsafe_repository_paths() {
        let repository = tempfile::tempdir().expect("temporary policy repository");
        assert!(inspect_paths(repository.path(), &["../escape.py".to_owned()]).is_err());
    }
}

#[cfg(test)]
mod native_adoption_tests {
    use super::*;

    #[test]
    fn native_decisions_keep_extension_precedence_and_final_symlink_nonfollowing() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("directory.py")).unwrap();
        fs::write(root.path().join(".py"), b"ordinary dot file").unwrap();
        fs::write(root.path().join("raw"), b"#!\xff\0pYtHoN\n").unwrap();
        fs::write(root.path().join("after-first-line"), b"#!sh\npython").unwrap();
        fs::write(root.path().join("foreign"), b"#!python\n").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("foreign", root.path().join("alias")).unwrap();
            std::os::unix::fs::symlink("missing", root.path().join("alias.PY")).unwrap();
        }
        let paths = [
            ".py",
            "after-first-line",
            "alias",
            "alias.PY",
            "directory.py",
            "missing.Py",
            "raw",
        ]
        .map(str::to_owned);
        let actual = inspect_paths(root.path(), &paths).unwrap();
        let expected = [
            ("alias.PY", ViolationKind::PythonFile),
            ("directory.py", ViolationKind::PythonFile),
            ("missing.Py", ViolationKind::PythonFile),
            ("raw", ViolationKind::PythonShebang),
        ]
        .map(|(path, kind)| Violation {
            path: path.to_owned(),
            kind,
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn multiple_native_batches_keep_full_counts_and_original_path_order() {
        let root = tempfile::tempdir().unwrap();
        let mut paths = Vec::new();
        let mut expected = Vec::new();
        for index in 0..137 {
            let path = if index % 3 == 0 {
                format!("{index:03}.PY")
            } else {
                format!("{index:03}")
            };
            fs::write(
                root.path().join(&path),
                if index % 3 == 1 {
                    b"#!python".as_slice()
                } else {
                    b"#!sh".as_slice()
                },
            )
            .unwrap();
            if index % 3 != 2 {
                expected.push(Violation {
                    path: path.clone(),
                    kind: if index % 3 == 0 {
                        ViolationKind::PythonFile
                    } else {
                        ViolationKind::PythonShebang
                    },
                });
            }
            paths.push(path);
        }
        let actual = inspect_paths(root.path(), &paths).unwrap();
        assert_eq!(actual, expected);
        assert!(actual.len() > MAXIMUM_REPORTED_VIOLATIONS);
    }

    #[test]
    fn filesystem_observer_preserves_the_exact_bounded_raw_prefix() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("raw");
        let bytes = (0..1024)
            .map(|value| (value % 256) as u8)
            .collect::<Vec<_>>();
        fs::write(&path, &bytes).unwrap();
        assert_eq!(observe_prefix(&path).unwrap(), bytes[..512]);
        assert!(
            observe_prefix(&root.path().join("absent"))
                .unwrap()
                .is_empty()
        );
        assert!(observe_prefix(root.path()).unwrap().is_empty());
    }
}
