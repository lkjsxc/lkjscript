//! Complete Cargo-selected workspace harnesses, with bounded parallel readers.
use super::cargo_origin_units::Unit;
use super::cargo_tests::{self, TestExecutable};
use super::model::Gate;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof};
use crate::process::{self, ProcessObservation, ProcessSpec, ProcessStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

pub(super) const TARGET: &str = "@workspace";
pub(super) const TARGET_COUNT: usize = 8;
const WORKERS: usize = 2;
const TEST_THREADS: &str = "16";
const STREAM_LIMIT: u64 = 64 * 1024 * 1024;
const CHILD_TIMEOUT: Duration = Duration::from_secs(9_000);
const SCHEMA: &str = "lkjscript-workspace-tests-1";

// A closed inventory prevents an omitted or newly added target from silently
// reducing workspace coverage. The release producer admits all workspace units.
struct Target {
    source: &'static str,
    manifest: &'static str,
    kind: &'static str,
    name: &'static str,
}

const TARGETS: [Target; TARGET_COUNT] = [
    Target {
        source: "src/bin/lkjscript.rs",
        manifest: "Cargo.toml",
        kind: "bin",
        name: "lkjscript",
    },
    Target {
        source: "src/lib.rs",
        manifest: "Cargo.toml",
        kind: "lib",
        name: "lkjscript",
    },
    Target {
        source: "tests/public_cli.rs",
        manifest: "Cargo.toml",
        kind: "test",
        name: "public_cli",
    },
    Target {
        source: "tools/lkjscript-dev/src/lib.rs",
        manifest: "tools/lkjscript-dev/Cargo.toml",
        kind: "lib",
        name: "lkjscript_dev",
    },
    Target {
        source: "tools/lkjscript-dev/src/main.rs",
        manifest: "tools/lkjscript-dev/Cargo.toml",
        kind: "bin",
        name: "lkjscript-dev",
    },
    Target {
        source: "tools/lkjscript-dev/src/release/controller_main.rs",
        manifest: "tools/lkjscript-dev/Cargo.toml",
        kind: "bin",
        name: "lkjscript-release-controller",
    },
    Target {
        source: "tools/lkjscript-site/src/lib.rs",
        manifest: "tools/lkjscript-site/Cargo.toml",
        kind: "lib",
        name: "lkjscript_site",
    },
    Target {
        source: "tools/lkjscript-site/src/main.rs",
        manifest: "tools/lkjscript-site/Cargo.toml",
        kind: "bin",
        name: "lkjscript-site",
    },
];

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

pub(super) fn selected(gate: &Gate) -> bool {
    gate.cargo_test
        .as_ref()
        .is_some_and(|input| input.target == TARGET)
}

pub(super) fn inventory(
    repository: &Path,
    units: &[Unit],
) -> Result<BTreeMap<String, TestExecutable>, DevError> {
    let mut selected = BTreeMap::new();
    for unit in units
        .iter()
        .filter(|unit| unit.profile.get("test") == Some(&Value::Bool(true)))
    {
        let target = TARGETS
            .iter()
            .find(|target| {
                unit.target
                    .get("src_path")
                    .and_then(Value::as_str)
                    .map(Path::new)
                    == Some(repository.join(target.source).as_path())
            })
            .ok_or_else(|| DevError::corrupt("unrecognized workspace test target"))?;
        require(
            Path::new(&unit.manifest_path) == repository.join(target.manifest)
                && unit.target.get("kind") == Some(&serde_json::json!([target.kind]))
                && unit.target.get("name").and_then(Value::as_str) == Some(target.name)
                && unit.profile.get("opt_level").and_then(Value::as_str) == Some("3"),
            "workspace test unit differs from its release target",
        )?;
        let test = TestExecutable {
            source: repository.join(target.source).display().to_string(),
            executable: cargo_tests::executable(unit)?,
        };
        validate_executable(repository, target, &test.executable)?;
        require(
            selected.insert(target.source.to_owned(), test).is_none(),
            "duplicate workspace test target",
        )?;
    }
    Ok(selected)
}

fn validate_executable(
    repository: &Path,
    target: &Target,
    proof: &FileProof,
) -> Result<(), DevError> {
    let path = repository.join(&proof.path);
    require(
        path.parent() == Some(repository.join("target/release/deps").as_path())
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(&format!("{}-", target.name.replace('-', "_")))
                })
            && proof.kind == FileKind::File
            && proof.bytes.is_some_and(|bytes| bytes > 0)
            && proof.digest.is_some()
            && proof.mode.is_some_and(|mode| mode & 0o111 != 0)
            && proof.link_target.is_none(),
        "workspace test is not its exact release executable",
    )
}

pub(super) fn input_proofs(repository: &Path, origin: &Value) -> Result<Vec<FileProof>, DevError> {
    let tests: BTreeMap<String, TestExecutable> =
        if origin.get("schema").and_then(Value::as_str) == Some("lkjscript-live-cargo-1") {
            serde_json::from_value(
                origin
                    .get("workspace_test_executables")
                    .cloned()
                    .ok_or_else(|| DevError::corrupt("workspace test inventory absent"))?,
            )?
        } else {
            let units: Vec<Unit> = serde_json::from_value(
                origin
                    .get("units")
                    .cloned()
                    .ok_or_else(|| DevError::corrupt("workspace unit inventory absent"))?,
            )?;
            inventory(repository, &units)?
        };
    require(
        tests.len() == TARGET_COUNT,
        "workspace test inventory is incomplete",
    )?;
    let mut paths = BTreeSet::new();
    TARGETS
        .iter()
        .map(|target| {
            let selected = tests
                .get(target.source)
                .ok_or_else(|| DevError::corrupt("workspace target was omitted"))?;
            require(
                Path::new(&selected.source) == repository.join(target.source),
                "workspace target source changed",
            )?;
            validate_executable(repository, target, &selected.executable)?;
            let mut proof = selected.executable.clone();
            proof.path = evidence::relative(repository, Path::new(&proof.path));
            require(
                paths.insert(proof.path.clone()),
                "workspace targets share one substituted executable",
            )?;
            Ok(proof)
        })
        .collect()
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    repository: PathBuf,
    directory: PathBuf,
    inputs: Vec<FileProof>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    workers: usize,
    test_threads: String,
    harnesses: Vec<Harness>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Harness {
    source: String,
    executable: FileProof,
    listing: ProcessObservation,
    ignored: ProcessObservation,
    execution: ProcessObservation,
    failure: Option<String>,
}

fn directory(stdout: &Path) -> Result<PathBuf, DevError> {
    Ok(stdout
        .parent()
        .ok_or_else(|| DevError::corrupt("workspace stdout parent absent"))?
        .join("workspace-tests"))
}

pub(super) fn request(
    repository: &Path,
    stdout: &Path,
    inputs: &[FileProof],
) -> Result<String, DevError> {
    Ok(serde_json::to_string(&Request {
        repository: repository.to_owned(),
        directory: directory(stdout)?,
        inputs: inputs.to_vec(),
    })?)
}

fn validate_inputs(repository: &Path, inputs: &[FileProof]) -> Result<(), DevError> {
    require(
        inputs.len() == TARGET_COUNT + 1,
        "workspace inputs are incomplete",
    )?;
    require(
        inputs[0].path == "target/release/lkjscript",
        "workspace product input changed",
    )?;
    let mut paths = BTreeSet::new();
    for (target, proof) in TARGETS.iter().zip(&inputs[1..]) {
        validate_executable(repository, target, proof)?;
        require(
            paths.insert(&proof.path),
            "workspace test executable was repeated",
        )?;
    }
    for proof in inputs {
        require(
            evidence::proof(&repository.join(&proof.path), proof.path.clone())? == *proof,
            "workspace input bytes changed",
        )?;
    }
    Ok(())
}

pub(crate) fn command(arguments: impl IntoIterator<Item = OsString>) -> Result<u8, DevError> {
    let mut arguments = arguments.into_iter();
    require(
        arguments.next().as_deref() == Some(std::ffi::OsStr::new("--no-fail-fast")),
        "workspace runner requires all target outcomes",
    )?;
    let request = arguments
        .next()
        .ok_or_else(|| DevError::usage("workspace test request absent"))?;
    require(
        arguments.next().is_none(),
        "unexpected workspace runner arguments",
    )?;
    let request: Request = serde_json::from_str(
        request
            .to_str()
            .ok_or_else(|| DevError::usage("workspace test request is not UTF-8"))?,
    )?;
    validate_inputs(&request.repository, &request.inputs)?;
    require(
        request.directory.is_absolute() && request.directory.starts_with(&request.repository),
        "workspace scratch is outside the checkout",
    )?;
    fs::create_dir(&request.directory)?;
    // Start the two large harnesses first; all eight still run after a failure.
    let queue = Mutex::new(VecDeque::from([2, 1, 3, 0, 4, 5, 6, 7]));
    let harnesses = std::thread::scope(|scope| -> Result<Vec<Harness>, DevError> {
        let handles = (0..WORKERS)
            .map(|_| {
                scope.spawn(|| {
                    let mut results = Vec::new();
                    loop {
                        let next = queue
                            .lock()
                            .map_err(|_| DevError::infrastructure("workspace queue poisoned"))?
                            .pop_front();
                        let Some(index) = next else {
                            break;
                        };
                        results.push(run_harness(&request, index));
                    }
                    Ok::<_, DevError>(results)
                })
            })
            .collect::<Vec<_>>();
        let mut results = Vec::new();
        let mut failure = None;
        for handle in handles {
            match handle.join() {
                Ok(Ok(harnesses)) => results.extend(harnesses),
                Ok(Err(error)) => failure = Some(error),
                Err(_) => {
                    failure = Some(DevError::infrastructure(
                        "workspace worker panicked after joining children",
                    ))
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        results.sort_by(|left, right| left.source.cmp(&right.source));
        Ok(results)
    })?;
    validate_inputs(&request.repository, &request.inputs)?;
    let passed = harnesses.len() == TARGET_COUNT
        && harnesses.iter().all(|harness| harness.failure.is_none());
    for harness in &harnesses {
        if let Some(failure) = &harness.failure {
            eprintln!("workspace {}: {failure}", harness.source);
        }
    }
    println!(
        "{}",
        serde_json::to_string(&Receipt {
            schema: SCHEMA.into(),
            workers: WORKERS,
            test_threads: TEST_THREADS.into(),
            harnesses
        })?
    );
    Ok(u8::from(!passed))
}

fn specification(request: &Request, index: usize, operation: &str) -> ProcessSpec {
    let mut command = vec![
        request
            .repository
            .join(&request.inputs[index + 1].path)
            .display()
            .to_string(),
    ];
    match operation {
        "list" => command.extend(["--list", "--format", "terse"].map(str::to_owned)),
        "ignored" => {
            command.extend(["--ignored", "--list", "--format", "terse"].map(str::to_owned))
        }
        _ => {
            command
                .extend(["--test-threads", TEST_THREADS, "--format", "pretty"].map(str::to_owned));
            if TARGETS[index].source == "tests/public_cli.rs" {
                command.extend(["--exact", "--skip", cargo_tests::LIFECYCLE].map(str::to_owned));
            }
        }
    }
    let mut environment = process::environment();
    environment.remove("CARGO_TARGET_DIR");
    ProcessSpec {
        command,
        cwd: request.repository.clone(),
        environment,
        timeout: CHILD_TIMEOUT,
        maximum_stdout_bytes: STREAM_LIMIT,
        maximum_stderr_bytes: STREAM_LIMIT,
        stdout_path: request
            .directory
            .join(format!("{index}-{operation}.stdout")),
        stderr_path: request
            .directory
            .join(format!("{index}-{operation}.stderr")),
        unavailable_exit_code: None,
    }
}

fn run_harness(request: &Request, index: usize) -> Harness {
    let observe = |operation| {
        process::run(
            &specification(request, index, operation),
            &request.repository,
        )
    };
    let mut harness = Harness {
        source: TARGETS[index].source.into(),
        executable: request.inputs[index + 1].clone(),
        listing: observe("list"),
        ignored: observe("ignored"),
        execution: observe("run"),
        failure: None,
    };
    if let Err(error) = verify_harness(request, index, &harness) {
        let mut failure = error.to_string();
        // Keep named failures in the compact parent receipt after joined streams
        // are removed. The complete observation and original terminal stay bound.
        if let Ok(bytes) = process::read_bounded(
            &request.directory.join(format!("{index}-run.stdout")),
            STREAM_LIMIT,
        ) {
            let text = String::from_utf8_lossy(&bytes);
            for line in text
                .lines()
                .filter(|line| line.starts_with("test ") && line.ends_with(" ... FAILED"))
                .take(4)
            {
                failure.push('\n');
                failure.extend(line.chars().take(240));
            }
            if let Some(summary) = text
                .lines()
                .rev()
                .find(|line| line.starts_with("test result:"))
            {
                failure.push('\n');
                failure.extend(summary.chars().take(240));
            }
        }
        harness.failure = Some(failure);
    }
    harness
}

fn stream_text(
    request: &Request,
    index: usize,
    operation: &str,
    observation: &ProcessObservation,
) -> Result<String, DevError> {
    let spec = specification(request, index, operation);
    require(
        observation.status == ProcessStatus::Passed
            && observation.exit_code == Some(0)
            && observation.signal.is_none()
            && observation.reason.is_none()
            && !observation.stdout_limit_exhausted
            && !observation.stderr_limit_exhausted
            && observation.stdout_limit_bytes == STREAM_LIMIT
            && observation.stderr_limit_bytes == STREAM_LIMIT
            && u128::from(observation.elapsed_nanoseconds) <= CHILD_TIMEOUT.as_nanos(),
        "workspace child failed or exceeded its execution bounds",
    )?;
    for (path, proof) in [
        (&spec.stdout_path, &observation.stdout),
        (&spec.stderr_path, &observation.stderr),
    ] {
        require(
            proof.path == evidence::relative(&request.repository, path)
                && evidence::proof(path, proof.path.clone())? == *proof,
            "workspace child stream changed",
        )?;
    }
    String::from_utf8(process::read_bounded(&spec.stdout_path, STREAM_LIMIT)?)
        .map_err(|_| DevError::corrupt("workspace test output is not UTF-8"))
}

fn listing(text: &str) -> Result<BTreeSet<String>, DevError> {
    let mut names = BTreeSet::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let name = line
            .strip_suffix(": test")
            .ok_or_else(|| DevError::corrupt("unrecognized workspace test listing"))?;
        require(
            !name.is_empty() && names.insert(name.to_owned()),
            "empty or duplicate workspace test name",
        )?;
    }
    Ok(names)
}

fn verify_outcomes(
    target: &Target,
    all: &BTreeSet<String>,
    ignored: &BTreeSet<String>,
    output: &str,
) -> Result<(), DevError> {
    require(
        ignored.is_subset(all),
        "ignored tests are not in the complete inventory",
    )?;
    require(
        target.kind == "bin" || !all.is_empty(),
        "library or public test inventory is empty",
    )?;
    let skip = usize::from(target.source == "tests/public_cli.rs");
    if skip == 1 {
        require(
            all.contains(cargo_tests::LIFECYCLE) && !ignored.contains(cargo_tests::LIFECYCLE),
            "standalone candidate lifecycle is missing or ignored",
        )?;
    }
    let active = all
        .len()
        .checked_sub(ignored.len() + skip)
        .ok_or_else(|| DevError::corrupt("workspace test counts underflow"))?;
    for name in all
        .iter()
        .filter(|name| skip == 0 || name.as_str() != cargo_tests::LIFECYCLE)
    {
        let expected = format!(
            "test {name} ... {}",
            if ignored.contains(name) {
                "ignored"
            } else {
                "ok"
            }
        );
        require(
            output
                .lines()
                .filter(|line| {
                    *line == expected
                        || (ignored.contains(name) && line.starts_with(&format!("{expected}, ")))
                })
                .count()
                == 1,
            "workspace test did not produce exactly one expected outcome",
        )?;
    }
    let expected = format!(
        "test result: ok. {active} passed; 0 failed; {} ignored; 0 measured; {skip} filtered out;",
        ignored.len()
    );
    require(
        output
            .lines()
            .rev()
            .find(|line| line.starts_with("test result:"))
            .is_some_and(|line| line.starts_with(&expected)),
        "workspace aggregate does not cover its complete test inventory",
    )
}

fn verify_harness(request: &Request, index: usize, harness: &Harness) -> Result<(), DevError> {
    require(
        harness.source == TARGETS[index].source
            && harness.executable == request.inputs[index + 1]
            && harness.failure.is_none(),
        "workspace harness binding changed or failed",
    )?;
    let all = listing(&stream_text(request, index, "list", &harness.listing)?)?;
    let ignored = listing(&stream_text(request, index, "ignored", &harness.ignored)?)?;
    let output = stream_text(request, index, "run", &harness.execution)?;
    verify_outcomes(&TARGETS[index], &all, &ignored, &output)
}

pub(super) fn verify_result(
    repository: &Path,
    stdout: &Path,
    inputs: &[FileProof],
) -> Result<(), DevError> {
    let receipt: Receipt = serde_json::from_slice(&process::read_bounded(stdout, 1024 * 1024)?)?;
    require(
        receipt.schema == SCHEMA
            && receipt.workers == WORKERS
            && receipt.test_threads == TEST_THREADS
            && receipt.harnesses.len() == TARGET_COUNT
            && inputs.len() == TARGET_COUNT + 1,
        "workspace receipt contract changed",
    )?;
    let request = Request {
        repository: repository.to_owned(),
        directory: directory(stdout)?,
        inputs: inputs.to_vec(),
    };
    for (index, harness) in receipt.harnesses.iter().enumerate() {
        verify_harness(&request, index, harness)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
