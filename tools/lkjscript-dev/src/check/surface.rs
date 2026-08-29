use crate::error::DevError;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

const MAXIMUM_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAXIMUM_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const MAXIMUM_DIRECTORY_FILES: usize = 128;
const MAXIMUM_VIOLATIONS: usize = 64;

const CURRENT_FILES: &[&str] = &[
    ".github/workflows/release.yml",
    "README.md",
    "applications/lkjournal/README.md",
    "applications/lkjournal/service.deployment.json",
    "applications/lkjournal/worker.deployment.json",
    "docs/architecture.md",
    "docs/release.md",
    "docs/roadmap.md",
    "docs/security.md",
    "docs/status.md",
    "packages/standard/README.md",
];

const CURRENT_DIRECTORIES: &[&str] = &["docs/generated", "docs/spec"];
const HISTORICAL_FILES: &[&str] = &["docs/spec/semantic-diff-merge.md"];

const PUBLIC_COMMANDS: &[&str] = &[
    "capabilities",
    "new",
    "status",
    "inspect",
    "query",
    "change",
    "package",
    "check",
    "build",
    "run",
    "serve",
    "worker",
];

const PUBLIC_SECTIONS: &[&str] = &[
    "operations",
    "change",
    "query",
    "type",
    "expression",
    "owners",
    "relations",
    "limits",
    "diagnostics",
    "templates",
    "runners",
    "deployment",
    "security",
];

const FORBIDDEN: &[&str] = &[
    "lkjscript-contract-registry-",
    "lkjscript-meaning-graph-",
    "lkjscript-cli-",
    "lkjscript-change-records-",
    "lkjscript-authored-change-codec-",
    "lkjscript-logical-change-plan-",
    "lkjscript-query-",
    "lkjscript-deployment-",
    "lkjscript-artifact-",
    "artifact-10",
    "graph 5",
    "graph5",
    "cli contract",
    "contract registry",
    "executable registry",
    "registry digest",
    "registry identity",
    "release manifest schema",
    "manifest_schema",
    "manifest-schema",
    "executable_registry_digest",
    "cli_contract",
    "contract_version",
    "name=contract-version",
    "--known-registry",
    "--section contracts",
    "docs/generated/contracts.md",
    "contract-1",
    "contract-2",
    "contract-3",
    "contract-4",
    "contract-5",
    "contract-6",
    "contract-7",
    "contract-8",
    "contract-9",
    "schema-1",
    "schema-2",
    "schema-3",
    "schema-4",
    "schema-5",
    "schema-6",
    "schema-7",
    "schema-8",
    "schema-9",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ViolationKind {
    TextLeak,
    Protocol,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Violation {
    path: String,
    kind: ViolationKind,
    detail: String,
}

pub(super) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let mut binary = None;
    let mut machine = false;
    while let Some(argument) = crate::next_utf8(&mut arguments, "product-surface option")? {
        match argument.as_str() {
            "--binary" if binary.is_none() => {
                binary = crate::next_utf8(&mut arguments, "product-surface binary")?;
            }
            "--machine" if !machine => machine = true,
            value => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate product-surface option '{value}'"
                )));
            }
        }
    }
    let binary =
        binary.ok_or_else(|| DevError::usage("product-surface policy requires --binary PATH"))?;
    let repository = repository_root()?;
    let (mut violations, scanned_files) = inspect_files(&repository)?;
    inspect_outputs(&repository, Path::new(&binary), &mut violations)?;
    print_result(machine, scanned_files, &violations)?;
    Ok(if violations.is_empty() { 0 } else { 1 })
}

fn inspect_files(repository: &Path) -> Result<(Vec<Violation>, usize), DevError> {
    let mut paths = CURRENT_FILES
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    for directory in CURRENT_DIRECTORIES {
        let absolute = repository.join(directory);
        let metadata = fs::symlink_metadata(&absolute).map_err(|error| {
            DevError::infrastructure(format!(
                "inspect product-surface directory '{}': {error}",
                absolute.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(DevError::infrastructure(format!(
                "product-surface directory '{}' is not a regular directory",
                absolute.display()
            )));
        }
        let mut entries = fs::read_dir(&absolute)
            .map_err(|error| {
                DevError::infrastructure(format!(
                    "read product-surface directory '{}': {error}",
                    absolute.display()
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                DevError::infrastructure(format!(
                    "enumerate product-surface directory '{}': {error}",
                    absolute.display()
                ))
            })?;
        if entries.len() > MAXIMUM_DIRECTORY_FILES {
            return Err(DevError::infrastructure(format!(
                "product-surface directory '{}' exceeds {MAXIMUM_DIRECTORY_FILES} entries",
                absolute.display()
            )));
        }
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_name = entry.file_name();
            let file_name = file_name.to_str().ok_or_else(|| {
                DevError::infrastructure(format!(
                    "product-surface directory '{}' contains a non-UTF-8 name",
                    absolute.display()
                ))
            })?;
            if !file_name.ends_with(".md") {
                return Err(DevError::infrastructure(format!(
                    "product-surface directory '{}' contains unexpected file '{file_name}'",
                    absolute.display()
                )));
            }
            let relative = format!("{directory}/{file_name}");
            if !HISTORICAL_FILES.contains(&relative.as_str()) {
                paths.push(relative);
            }
        }
    }
    paths.sort();
    paths.dedup();
    let scanned_files = paths.len();
    let mut violations = Vec::new();
    if fs::symlink_metadata(repository.join("docs/generated/contracts.md")).is_ok() {
        violations.push(violation(
            "docs/generated/contracts.md",
            ViolationKind::TextLeak,
            "obsolete generated contract table exists",
        ));
    }
    for relative in paths {
        let text = read_text(repository, &relative)?;
        append_text_violations(&relative, &text, &mut violations);
        if violations.len() >= MAXIMUM_VIOLATIONS {
            break;
        }
    }
    Ok((violations, scanned_files))
}

fn read_text(repository: &Path, relative: &str) -> Result<String, DevError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(DevError::infrastructure(format!(
            "product-surface owner contains unsafe path '{relative}'"
        )));
    }
    let absolute = repository.join(path);
    let metadata = fs::symlink_metadata(&absolute).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect product-surface path '{}': {error}",
            absolute.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "product-surface path '{}' is not a regular file",
            absolute.display()
        )));
    }
    if metadata.len() > MAXIMUM_FILE_BYTES {
        return Err(DevError::infrastructure(format!(
            "product-surface path '{}' exceeds {MAXIMUM_FILE_BYTES} bytes",
            absolute.display()
        )));
    }
    fs::read_to_string(&absolute).map_err(|error| {
        DevError::infrastructure(format!(
            "read UTF-8 product-surface path '{}': {error}",
            absolute.display()
        ))
    })
}

fn append_text_violations(path: &str, text: &str, violations: &mut Vec<Violation>) {
    let lowercase = text.to_ascii_lowercase();
    for needle in FORBIDDEN {
        if let Some(offset) = lowercase.find(needle) {
            let line = text.as_bytes()[..offset]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                + 1;
            violations.push(violation(
                path,
                ViolationKind::TextLeak,
                &format!("line={line} forbidden={needle}"),
            ));
            if violations.len() >= MAXIMUM_VIOLATIONS {
                return;
            }
        }
    }
}

fn inspect_outputs(
    repository: &Path,
    binary: &Path,
    violations: &mut Vec<Violation>,
) -> Result<(), DevError> {
    let metadata = fs::symlink_metadata(binary).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect product-surface binary '{}': {error}",
            binary.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "product-surface binary '{}' is not a regular file",
            binary.display()
        )));
    }
    let version = run_success(repository, binary, &["--version"])?;
    let expected = format!("lkjscript {}\n", lkjscript::PRODUCT_VERSION);
    if version != expected.as_bytes() {
        violations.push(violation(
            "<binary>:--version",
            ViolationKind::Protocol,
            "version output is not the exact product line",
        ));
    }

    let mut requests = vec![("capabilities".to_owned(), vec!["capabilities".to_owned()])];
    requests.extend(PUBLIC_COMMANDS.iter().map(|command| {
        (
            format!("capabilities:{command}"),
            vec!["capabilities".to_owned(), (*command).to_owned()],
        )
    }));
    requests.extend(PUBLIC_SECTIONS.iter().map(|section| {
        (
            format!("capabilities-section:{section}"),
            vec![
                "capabilities".to_owned(),
                "--section".to_owned(),
                (*section).to_owned(),
            ],
        )
    }));
    for (label, arguments) in requests {
        let borrowed = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let bytes = run_success(repository, binary, &borrowed)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            DevError::corrupt(format!("product-surface output '{label}' is not UTF-8"))
        })?;
        let prefix = format!(
            "product name=lkjscript version={}\ncapabilities digest=",
            lkjscript::PRODUCT_VERSION
        );
        if !text.starts_with(&prefix) {
            violations.push(violation(
                &format!("<binary>:{label}"),
                ViolationKind::Protocol,
                "capability output omitted canonical product and digest records",
            ));
        }
        append_text_violations(&format!("<binary>:{label}"), text, violations);
    }

    let predecessor_requests = [
        vec![
            "capabilities".to_owned(),
            "--known-registry".to_owned(),
            "0".repeat(64),
        ],
        vec![
            "capabilities".to_owned(),
            "--section".to_owned(),
            "contracts".to_owned(),
        ],
    ];
    for arguments in predecessor_requests {
        let output = Command::new(binary)
            .args(&arguments)
            .current_dir(repository)
            .env_clear()
            .envs(crate::process::environment())
            .output()
            .map_err(|error| {
                DevError::infrastructure(format!(
                    "start product-surface predecessor probe '{}': {error}",
                    binary.display()
                ))
            })?;
        if output.status.code() != Some(2)
            || !output.stderr.is_empty()
            || output.stdout.len() > MAXIMUM_OUTPUT_BYTES
            || !String::from_utf8_lossy(&output.stdout).contains("code=cli_usage")
        {
            violations.push(violation(
                "<binary>:predecessor-capabilities",
                ViolationKind::Protocol,
                "predecessor capability spelling did not use bounded usage rejection",
            ));
        }
    }
    Ok(())
}

fn run_success(repository: &Path, binary: &Path, arguments: &[&str]) -> Result<Vec<u8>, DevError> {
    let output = Command::new(binary)
        .args(arguments)
        .current_dir(repository)
        .env_clear()
        .envs(crate::process::environment())
        .output()
        .map_err(|error| {
            DevError::infrastructure(format!(
                "start product-surface command '{}': {error}",
                binary.display()
            ))
        })?;
    let bytes = output
        .stdout
        .len()
        .checked_add(output.stderr.len())
        .ok_or_else(|| DevError::infrastructure("product-surface output length overflow"))?;
    if bytes > MAXIMUM_OUTPUT_BYTES {
        return Err(DevError::infrastructure(format!(
            "product-surface command output exceeds {MAXIMUM_OUTPUT_BYTES} bytes"
        )));
    }
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(DevError::corrupt(format!(
            "product-surface command {arguments:?} failed or wrote stderr"
        )));
    }
    Ok(output.stdout)
}

fn violation(path: &str, kind: ViolationKind, detail: &str) -> Violation {
    Violation {
        path: path.to_owned(),
        kind,
        detail: detail.to_owned(),
    }
}

fn print_result(
    machine: bool,
    scanned_files: usize,
    violations: &[Violation],
) -> Result<(), DevError> {
    #[derive(Serialize)]
    struct Summary<'a> {
        contract_version: u32,
        status: &'static str,
        policy: &'static str,
        explicit_files: usize,
        current_directories: usize,
        scanned_files: usize,
        historical_files: usize,
        executable_commands: usize,
        exceptions: usize,
        violations: usize,
        reported: &'a [Violation],
    }
    let reported = &violations[..violations.len().min(MAXIMUM_VIOLATIONS)];
    let summary = Summary {
        contract_version: 1,
        status: if violations.is_empty() {
            "passed"
        } else {
            "failed"
        },
        policy: "product-surface",
        explicit_files: CURRENT_FILES.len(),
        current_directories: CURRENT_DIRECTORIES.len(),
        scanned_files,
        historical_files: HISTORICAL_FILES.len(),
        executable_commands: 1 + PUBLIC_COMMANDS.len() + PUBLIC_SECTIONS.len(),
        exceptions: 0,
        violations: violations.len(),
        reported,
    };
    if machine {
        println!(
            "{}",
            serde_json::to_string(&summary).map_err(|error| {
                DevError::infrastructure(format!("encode product-surface result: {error}"))
            })?
        );
    } else if violations.is_empty() {
        println!("policy passed: current product surface exposes only product version identity");
    } else {
        println!(
            "policy failed: {} product-surface violation(s)",
            violations.len()
        );
        for item in reported {
            println!("{}: {:?}: {}", item.path, item.kind, item.detail);
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
    fn patterns_reject_numbered_subsystems_without_false_positives() {
        let allowed = "lkjscript 0.1.11 revision=rev_123 digest=abc target=x86_64-unknown-linux-musl HTTP status 200 Rust 1.98.0 PostgreSQL 18.0 2026-08-30 1024 bytes";
        let mut violations = Vec::new();
        append_text_violations("allowed", allowed, &mut violations);
        assert!(violations.is_empty());
        for forbidden in [
            "Graph 5",
            "CLI contract 13",
            "lkjscript-cli-13",
            "artifact-10",
            "lkjscript-deployment-1",
            "lkjscript-change-records-6",
            "lkjscript-contract-registry-3",
            "contract-3",
            "schema-2",
            "contract_version",
            "name=contract-version",
            "--known-registry",
            "--section contracts",
            "executable_registry_digest",
        ] {
            let mut observed = Vec::new();
            append_text_violations("forbidden", forbidden, &mut observed);
            assert!(!observed.is_empty(), "missed {forbidden}");
        }
    }
}
