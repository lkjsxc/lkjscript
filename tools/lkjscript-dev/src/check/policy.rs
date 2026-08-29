use super::snapshot;
use crate::error::DevError;
use serde::Serialize;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path};

const MAXIMUM_SHEBANG_BYTES: u64 = 512;
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
    for relative in paths {
        let path = validate_relative_path(relative)?;
        if python_extension(path) {
            violations.push(Violation {
                path: relative.clone(),
                kind: ViolationKind::PythonFile,
            });
            continue;
        }
        let absolute = repository.join(path);
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(DevError::infrastructure(format!(
                    "inspect repository policy path '{}': {error}",
                    absolute.display()
                )));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        if python_shebang(&absolute)? {
            violations.push(Violation {
                path: relative.clone(),
                kind: ViolationKind::PythonShebang,
            });
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

fn python_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("py"))
}

fn python_shebang(path: &Path) -> Result<bool, DevError> {
    let mut bytes = Vec::with_capacity(MAXIMUM_SHEBANG_BYTES as usize);
    File::open(path)
        .and_then(|file| file.take(MAXIMUM_SHEBANG_BYTES).read_to_end(&mut bytes))
        .map_err(|error| {
            DevError::infrastructure(format!(
                "read repository policy path '{}': {error}",
                path.display()
            ))
        })?;
    let line = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    if !line.starts_with(b"#!") {
        return Ok(false);
    }
    Ok(line
        .windows(b"python".len())
        .any(|window| window.eq_ignore_ascii_case(b"python")))
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
