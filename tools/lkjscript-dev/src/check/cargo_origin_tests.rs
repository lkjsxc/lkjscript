use super::super::executor::{self, FreshInput};
use super::super::model::{CacheLookupStatus, CacheObservation, GateReceipt, GateStatus};
use super::super::reader;
use super::*;
use crate::process::{self, ProcessControl, ProcessSpec};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, TryLockError, mpsc};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const STREAM_BYTES: u64 = 64 * 1024;
const CHILD_TIMEOUT: Duration = Duration::from_secs(60);
const THREAD_TIMEOUT: Duration = Duration::from_secs(10);

struct Fixture {
    _directory: tempfile::TempDir,
    repository: PathBuf,
    target: PathBuf,
    snapshot: InputSnapshot,
    runtime: RuntimeIdentity,
    phase_sequence: AtomicU64,
}

impl Fixture {
    fn new() -> Result<Self, DevError> {
        Self::with_feature(false)
    }

    fn with_feature(feature_variant: bool) -> Result<Self, DevError> {
        let directory = tempfile::tempdir()?;
        let repository = fs::canonicalize(directory.path())?;
        let target = repository.join("target");
        fs::create_dir_all(repository.join("src"))?;
        fs::create_dir_all(repository.join(".artifacts/lifecycle"))?;
        fs::create_dir_all(repository.join(".artifacts/lkjscript-dev/check"))?;
        fs::write(
            repository.join(".gitignore"),
            "/.artifacts/\n/target/\n/.cargo/\n",
        )?;
        fs::write(
            repository.join("Cargo.toml"),
            if feature_variant {
                "[package]\nname = \"lkjscript\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[features]\nvariant-b = []\n[workspace]\n"
            } else {
                "[package]\nname = \"lkjscript\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n"
            },
        )?;
        fs::write(
            repository.join("Cargo.lock"),
            "version = 4\n[[package]]\nname = \"lkjscript\"\nversion = \"0.0.0\"\n",
        )?;
        fs::write(
            repository.join("src/lib.rs"),
            if feature_variant {
                "#[cfg(not(feature = \"variant-b\"))]\npub fn value() -> u32 { 42 }\n#[cfg(feature = \"variant-b\")]\npub fn value() -> u32 { 7 }\n"
            } else {
                "pub fn value() -> u32 { 42 }\n"
            },
        )?;
        fs::write(
            repository.join("src/main.rs"),
            "fn main() { println!(\"{}\", lkjscript::value()); }\n",
        )?;
        git(
            &repository,
            "init",
            &[
                "-c",
                "init.defaultBranch=fixture",
                "init",
                "--template=",
                "--quiet",
            ],
        )?;
        git(&repository, "add", &["add", "."])?;
        git(
            &repository,
            "commit",
            &[
                "-c",
                "user.name=Cargo origin fixture",
                "-c",
                "user.email=cargo-origin-fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--no-verify",
                "--quiet",
                "-m",
                "Owned Cargo origin fixture",
            ],
        )?;
        let snapshot = snapshot::capture(&repository)?;
        let runtime = snapshot::runtime_identity(&repository, commands())?;
        Ok(Self {
            _directory: directory,
            repository,
            target,
            snapshot,
            runtime,
            phase_sequence: AtomicU64::new(1000),
        })
    }

    fn owner(&self) -> Result<Run, DevError> {
        Run::with_commands(
            &self.repository,
            &self.target,
            &self.snapshot,
            &self.runtime,
            commands(),
        )
    }

    fn writer(&self, name: &str, arguments: &[&str]) -> Result<Gate, DevError> {
        self.writer_at(name, arguments, &self.target)
    }

    fn writer_at(&self, name: &str, arguments: &[&str], target: &Path) -> Result<Gate, DevError> {
        let mut command = vec!["cargo".to_owned()];
        command.extend(arguments.iter().map(|argument| (*argument).to_owned()));
        let mut gate = Gate::new(name, command);
        gate.timeout = CHILD_TIMEOUT;
        gate.maximum_stdout_bytes = STREAM_BYTES;
        gate.maximum_stderr_bytes = STREAM_BYTES;
        gate.cacheable = false;
        bind_target(&mut gate, target)?;
        Ok(gate)
    }

    fn producer(&self, name: &str) -> Result<Gate, DevError> {
        let mut gate = self.writer(
            name,
            &[
                "build",
                "--workspace",
                "--release",
                "--locked",
                "--message-format=json",
            ],
        )?;
        gate.required_outputs = vec![self.target.join("release/lkjscript")];
        Ok(gate)
    }

    fn specification(&self, gate: &Gate) -> Result<ProcessSpec, DevError> {
        let sequence = self.phase_sequence.fetch_add(1, Ordering::Relaxed);
        let directory = self
            .repository
            .join(".artifacts/lkjscript-dev/check")
            .join(format!("{sequence}-{}-0", std::process::id()));
        fs::create_dir(&directory)?;
        let _ = fs::File::create(directory.join("ACTIVE.lock"))?;
        Ok(self.specification_at(gate, &directory))
    }

    fn specification_at(&self, gate: &Gate, directory: &Path) -> ProcessSpec {
        ProcessSpec {
            command: gate.command.clone(),
            cwd: self.repository.clone(),
            environment: process::environment(),
            timeout: gate.timeout,
            maximum_stdout_bytes: gate.maximum_stdout_bytes,
            maximum_stderr_bytes: gate.maximum_stderr_bytes,
            stdout_path: directory.join(format!("{}.stdout.log", gate.name)),
            stderr_path: directory.join(format!("{}.stderr.log", gate.name)),
            unavailable_exit_code: None,
        }
    }

    fn execute(&self, owner: &Run, gate: &Gate) -> Result<GateReceipt, DevError> {
        let specification = self.specification(gate)?;
        self.execute_specification(owner, gate, specification)
    }

    fn execute_specification(
        &self,
        owner: &Run,
        gate: &Gate,
        specification: ProcessSpec,
    ) -> Result<GateReceipt, DevError> {
        let started_wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| test_error(format!("fixture clock: {error}")))?
            .as_nanos();
        executor::execute_fresh_with_origin(
            FreshInput {
                repository: &self.repository,
                gate,
                program: Some(Path::new(env!("CARGO"))),
                fingerprint: VerificationDigest::of(b"real-cargo-origin-lifecycle-fixture"),
                stdout_path: specification.stdout_path,
                stderr_path: specification.stderr_path,
                started_wall,
                started: Instant::now(),
                cache_observation: CacheObservation {
                    eligible: false,
                    lookup: CacheLookupStatus::Bypassed,
                    reason: None,
                    record: None,
                    write: None,
                },
            },
            owner,
        )
    }

    fn cold(&self, owner: &Run) -> Result<(Gate, GateReceipt, Proof), DevError> {
        let gate = self.producer("cold")?;
        let receipt = self.execute(owner, &gate)?;
        passed(&receipt)?;
        let proof = read_proof(&self.repository, origin(&receipt)?)?;
        require(
            proof.units.len() >= 2
                && proof
                    .units
                    .iter()
                    .all(|entry| !entry.unit.fresh && entry.cold_proof.is_none())
                && proof.units.iter().any(|entry| shared_binary(&entry.unit))
                && proof.units.iter().any(|entry| !shared_binary(&entry.unit)),
            "cold producer must compile both library and binary units",
        )?;
        Ok((gate, receipt, proof))
    }
}

fn commands() -> Vec<(String, String)> {
    vec![(env!("CARGO").to_owned(), "cargo".to_owned())]
}

fn git(repository: &Path, name: &str, arguments: &[&str]) -> Result<(), DevError> {
    let directory = repository.join(".artifacts/lifecycle/git");
    fs::create_dir_all(&directory)?;
    let mut command = vec!["git".to_owned()];
    command.extend(arguments.iter().map(|argument| (*argument).to_owned()));
    let observation = process::run(
        &ProcessSpec {
            command,
            cwd: repository.to_path_buf(),
            environment: process::environment(),
            timeout: CHILD_TIMEOUT,
            maximum_stdout_bytes: STREAM_BYTES,
            maximum_stderr_bytes: STREAM_BYTES,
            stdout_path: directory.join(format!("{name}.stdout.log")),
            stderr_path: directory.join(format!("{name}.stderr.log")),
            unavailable_exit_code: None,
        },
        repository,
    );
    require(
        observation.status == ProcessStatus::Passed && observation.exit_code == Some(0),
        format!("Git fixture setup {name} did not pass: {observation:?}"),
    )
}

fn test_error(message: impl Into<String>) -> DevError {
    DevError::infrastructure(format!("Cargo origin lifecycle test: {}", message.into()))
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(test_error(message))
    }
}

fn refused<T>(result: Result<T, DevError>, message: &str) -> Result<(), DevError> {
    match result {
        Err(_) => Ok(()),
        Ok(_) => Err(test_error(message)),
    }
}

fn refused_because<T>(result: Result<T, DevError>, expected: &str) -> Result<(), DevError> {
    match result {
        Err(error) => require(
            error.message().contains(expected),
            format!("refusal did not exercise {expected}: {error}"),
        ),
        Ok(_) => Err(test_error(format!("expected refusal for {expected}"))),
    }
}

fn origin(receipt: &GateReceipt) -> Result<&FileProof, DevError> {
    receipt
        .cargo_origin
        .as_ref()
        .ok_or_else(|| test_error("successful producer has no origin sidecar"))
}

fn observation(receipt: &GateReceipt) -> Result<&ProcessObservation, DevError> {
    receipt
        .process
        .as_ref()
        .ok_or_else(|| test_error("producer has no joined process observation"))
}

fn read_proof(repository: &Path, proof: &FileProof) -> Result<Proof, DevError> {
    Ok(serde_json::from_slice(&fs::read(proof_path(
        repository, proof,
    )?)?)?)
}

fn passed(receipt: &GateReceipt) -> Result<(), DevError> {
    require(
        receipt.status == GateStatus::Passed
            && observation(receipt)?.status == ProcessStatus::Passed,
        format!("real Cargo producer did not pass: {receipt:?}"),
    )
}

fn warm_refused(receipt: &GateReceipt) -> Result<(), DevError> {
    require(
        observation(receipt)?.status == ProcessStatus::Passed
            && receipt.status == GateStatus::Failed
            && receipt.cargo_origin.is_none()
            && receipt
                .reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("cargo_output_binding:")),
        format!("unproved Cargo warm reuse was accepted: {receipt:?}"),
    )
}

fn durable_dirty(owner: &Run) -> Result<(), DevError> {
    let bytes = fs::read(&owner.ledger_path)?;
    let ledger: Ledger = serde_json::from_slice(&bytes)?;
    require(
        ledger.dirty,
        "unsuccessful or incomplete writer restored a clean durable inventory",
    )
}

#[test]
fn cold_warm_and_unrelated_debug_writer_preserve_original_release_origin() -> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let owner = fixture.owner()?;
    let (_, cold, cold_proof) = fixture.cold(&owner)?;
    let cold_origin = origin(&cold)?.clone();
    let warm_gate = fixture.producer("warm")?;
    let warm = fixture.execute(&owner, &warm_gate)?;
    passed(&warm)?;
    let warm_proof = read_proof(&fixture.repository, origin(&warm)?)?;
    require(
        warm_proof
            .units
            .iter()
            .all(|entry| entry.unit.fresh && entry.cold_proof.as_ref() == Some(&cold_origin)),
        "warm units did not retain their actual original cold producer",
    )?;
    let debug = fixture.writer(
        "debug",
        &["build", "--workspace", "--locked", "--message-format=json"],
    )?;
    let debug_receipt = fixture.execute(&owner, &debug)?;
    passed(&debug_receipt)?;
    require(
        debug_receipt.cargo_origin.is_none(),
        "nonproducer debug writer minted origin evidence",
    )?;
    for entry in &cold_proof.units {
        live(&entry.unit)?;
    }
    let after_debug_gate = fixture.producer("warm-after-debug")?;
    let after_debug = fixture.execute(&owner, &after_debug_gate)?;
    passed(&after_debug)?;
    let after_proof = read_proof(&fixture.repository, origin(&after_debug)?)?;
    require(
        after_proof
            .units
            .iter()
            .all(|entry| entry.unit.fresh && entry.cold_proof.as_ref() == Some(&cold_origin)),
        "unchanged release artifacts lost their cold authority after an unrelated debug writer",
    )?;
    verify(
        &fixture.repository,
        &warm_gate,
        observation(&warm)?,
        origin(&warm)?,
        owner.context(),
    )?;
    verify(
        &fixture.repository,
        &after_debug_gate,
        observation(&after_debug)?,
        origin(&after_debug)?,
        owner.context(),
    )
}

#[test]
fn missing_or_corrupt_cold_sidecar_and_retained_library_refuse_warm_reuse() -> Result<(), DevError>
{
    for fault in [
        "missing-sidecar",
        "corrupt-sidecar",
        "missing-library",
        "corrupt-library",
    ] {
        let fixture = Fixture::new()?;
        let owner = fixture.owner()?;
        let (_, cold, proof) = fixture.cold(&owner)?;
        let damaged = if fault.ends_with("sidecar") {
            proof_path(&fixture.repository, origin(&cold)?)?
        } else {
            let library = proof
                .units
                .iter()
                .find(|entry| !shared_binary(&entry.unit))
                .ok_or_else(|| test_error("cold library unit is absent"))?;
            let retained = library
                .unit
                .retained
                .first()
                .ok_or_else(|| test_error("retained cold library file is absent"))?;
            proof_path(&fixture.repository, retained)?
        };
        let original = fixture
            .repository
            .join(".artifacts/lifecycle")
            .join(format!("{fault}.original"));
        fs::copy(&damaged, original)?;
        if fault.starts_with("missing") {
            fs::remove_file(&damaged)?;
        } else {
            fs::write(&damaged, b"owned corruption counterexample\n")?;
        }
        let warm = fixture.execute(&owner, &fixture.producer("damaged-warm")?)?;
        warm_refused(&warm)?;
        durable_dirty(&owner)?;
    }
    Ok(())
}

#[test]
fn previously_absent_config_and_changed_compiler_proof_refuse_admission() -> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let owner = fixture.owner()?;
    let (gate, cold, mut proof) = fixture.cold(&owner)?;
    let config = fixture.repository.join(".cargo/config.toml");
    require(
        owner
            .context
            .configs
            .iter()
            .any(|entry| Path::new(&entry.path) == config && entry.kind == FileKind::Missing),
        "config absence was omitted from the original context",
    )?;
    fs::create_dir(fixture.repository.join(".cargo"))?;
    fs::write(&config, "[build]\ntarget-dir = \"elsewhere\"\n")?;
    require(
        snapshot::capture(&fixture.repository)?.digest == fixture.snapshot.digest,
        "ignored config changed source snapshot instead of config context",
    )?;
    refused_because(
        owner.begin(&fixture.producer("changed-config")?),
        "unsupported Cargo config include or dynamic routing",
    )?;
    fs::remove_file(config)?;
    owner
        .context
        .recheck(&fixture.repository, &fixture.snapshot, &fixture.runtime)?;
    let compiler = proof
        .context
        .compiler_files
        .last_mut()
        .ok_or_else(|| test_error("actual compiler proof is absent"))?;
    compiler.digest = Some(VerificationDigest::of(
        b"private compiler proof counterexample",
    ));
    let directory = fixture
        .repository
        .join(".artifacts/lifecycle/forged-compiler");
    fs::create_dir(&directory)?;
    let path = directory.join(format!("{}.cargo-origin.json", gate.name));
    fs::write(&path, serde_json::to_vec(&proof)?)?;
    let forged = evidence::proof(&path, evidence::relative(&fixture.repository, &path))?;
    refused_because(
        verify(
            &fixture.repository,
            &gate,
            observation(&cold)?,
            &forged,
            owner.context(),
        ),
        "foreign source or runtime context",
    )
}

#[test]
fn physical_target_lease_contends_and_run_writer_transactions_serialize() -> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let other = Fixture::new()?;
    let owner = Arc::new(fixture.owner()?);
    refused_because(
        Run::with_commands(
            &other.repository,
            &fixture.target,
            &other.snapshot,
            &other.runtime,
            commands(),
        ),
        "Cargo target lease is unavailable",
    )?;
    let gate = fixture.writer("serialized", &["build", "--workspace", "--locked"])?;
    let mut held = owner.begin(&gate)?;
    let (started_sender, started_receiver) = mpsc::channel();
    let (acquired_sender, acquired_receiver) = mpsc::channel();
    let worker_owner = Arc::clone(&owner);
    let worker_gate = gate.clone();
    let worker = thread::Builder::new()
        .name("cargo-origin-test-writer".to_owned())
        .spawn(move || -> Result<(), DevError> {
            require(
                matches!(worker_owner.state.try_lock(), Err(TryLockError::WouldBlock)),
                "controlled worker did not observe the held writer mutex",
            )?;
            started_sender
                .send(())
                .map_err(|error| test_error(format!("writer start signal: {error}")))?;
            let mut transaction = worker_owner.begin(&worker_gate)?;
            acquired_sender
                .send(())
                .map_err(|error| test_error(format!("writer acquisition signal: {error}")))?;
            transaction.finish_unchanged()
        })?;
    let contention = (|| -> Result<(), DevError> {
        started_receiver
            .recv_timeout(THREAD_TIMEOUT)
            .map_err(|error| test_error(format!("writer did not start: {error}")))?;
        require(
            matches!(
                acquired_receiver.recv_timeout(Duration::from_millis(50)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "second writer acquired a transaction while the first still owned it",
        )?;
        held.finish_unchanged()
    })();
    // Even a failed channel check releases the mutex before joining the worker.
    drop(held);
    let joined = worker
        .join()
        .map_err(|_| test_error("controlled writer thread panicked"))?;
    contention?;
    joined?;
    acquired_receiver
        .recv_timeout(THREAD_TIMEOUT)
        .map_err(|error| {
            test_error(format!(
                "serialized writer never acquired the released transaction: {error}"
            ))
        })?;
    let mut abandoned = owner.begin(&gate)?;
    durable_dirty(&owner)?;
    abandoned.finish_unchanged()?;
    drop(abandoned);
    drop(owner);
    let successor = other_owner(&other, &fixture.target)?;
    drop(successor);
    Ok(())
}

fn other_owner(fixture: &Fixture, target: &Path) -> Result<Run, DevError> {
    Run::with_commands(
        &fixture.repository,
        target,
        &fixture.snapshot,
        &fixture.runtime,
        commands(),
    )
}

#[test]
fn failed_cancelled_and_incomplete_writers_leave_durable_dirty_origin() -> Result<(), DevError> {
    for fault in ["failed", "cancelled", "incomplete", "abandoned"] {
        let fixture = Fixture::new()?;
        let owner = fixture.owner()?;
        let _ = fixture.cold(&owner)?;
        let gate = if fault == "failed" {
            fixture.writer(
                "failed",
                &[
                    "build",
                    "--locked",
                    "--package",
                    "origin-fixture-package-does-not-exist",
                ],
            )?
        } else {
            fixture.producer(fault)?
        };
        let mut transaction = owner.begin(&gate)?;
        if fault != "abandoned" {
            let mut specification = fixture.specification(&gate)?;
            let mut process = if fault == "cancelled" {
                let control = ProcessControl::default();
                control.kill();
                let program = specification
                    .command
                    .first_mut()
                    .ok_or_else(|| test_error("cancelled writer command is empty"))?;
                *program = env!("CARGO").to_owned();
                process::run_controlled(&specification, &fixture.repository, &control)
            } else {
                process::run_selected(
                    &specification,
                    &fixture.repository,
                    Some(Path::new(env!("CARGO"))),
                )
            };
            if fault == "incomplete" {
                require(
                    process.status == ProcessStatus::Passed,
                    "incomplete-record control Cargo process failed",
                )?;
                let text = fs::read_to_string(&specification.stdout_path)?;
                let mut shortened = String::new();
                let mut removed = false;
                for line in text.lines() {
                    if line.starts_with('{')
                        && serde_json::from_str::<serde_json::Value>(line)?
                            .get("reason")
                            .and_then(serde_json::Value::as_str)
                            == Some("build-finished")
                    {
                        removed = true;
                    } else {
                        shortened.push_str(line);
                        shortened.push('\n');
                    }
                }
                require(
                    removed,
                    "actual successful producer had no completion record to remove",
                )?;
                fs::copy(
                    &specification.stdout_path,
                    specification.stdout_path.with_extension("original"),
                )?;
                fs::write(&specification.stdout_path, shortened.as_bytes())?;
                process.stdout =
                    evidence::proof(&specification.stdout_path, process.stdout.path.clone())?;
                refused(
                    transaction.finish(
                        &gate,
                        &process,
                        specification
                            .stdout_path
                            .parent()
                            .ok_or_else(|| test_error("fixture log parent is absent"))?,
                    ),
                    "incomplete successful writer restored clean origin",
                )?;
            } else {
                require(
                    if fault == "cancelled" {
                        process.status == ProcessStatus::Signaled
                    } else {
                        process.status == ProcessStatus::Failed
                    },
                    "failed or cancelled Cargo control did not reach its intended process status",
                )?;
                require(
                    transaction
                        .finish(
                            &gate,
                            &process,
                            specification
                                .stdout_path
                                .parent()
                                .ok_or_else(|| test_error("fixture log parent is absent"))?,
                        )?
                        .is_none(),
                    "failed or cancelled writer minted origin proof",
                )?;
            }
        }
        drop(transaction);
        durable_dirty(&owner)?;
        drop(owner);
        let successor = fixture.owner()?;
        durable_dirty(&successor)?;
        let warm = fixture.execute(&successor, &fixture.producer("after-dirty")?)?;
        warm_refused(&warm)?;
    }
    Ok(())
}

#[test]
fn reader_requires_origin_and_rejects_coherent_fresh_only_cold_forgery() -> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let owner = fixture.owner()?;
    let _ = fixture.cold(&owner)?;
    let warm_gate = fixture.producer("warm")?;
    let warm = fixture.execute(&owner, &warm_gate)?;
    passed(&warm)?;
    refused_because(
        reader::verify_cargo_origin(
            &fixture.repository,
            &warm_gate,
            observation(&warm)?,
            None,
            Some(owner.context()),
        ),
        "Cargo producer origin proof is absent",
    )?;
    let mut forged = read_proof(&fixture.repository, origin(&warm)?)?;
    require(
        forged.units.iter().all(|entry| entry.unit.fresh),
        "forgery control was not actually Cargo fresh",
    )?;
    for entry in &mut forged.units {
        entry.cold_proof = None;
    }
    let gate = fixture.producer("forged-fresh-only")?;
    forged.producer = Producer::capture(&gate)?;
    let specification = fixture.specification(&gate)?;
    let original_process = observation(&warm)?;
    for (source, destination) in [
        (&original_process.stdout, &specification.stdout_path),
        (&original_process.stderr, &specification.stderr_path),
    ] {
        fs::copy(proof_path(&fixture.repository, source)?, destination)?;
    }
    forged.process.stdout = evidence::proof(
        &specification.stdout_path,
        evidence::relative(&fixture.repository, &specification.stdout_path),
    )?;
    forged.process.stderr = evidence::proof(
        &specification.stderr_path,
        evidence::relative(&fixture.repository, &specification.stderr_path),
    )?;
    let directory = specification
        .stdout_path
        .parent()
        .ok_or_else(|| test_error("forged log parent is absent"))?;
    let path = directory.join(format!("{}.cargo-origin.json", gate.name));
    fs::write(&path, serde_json::to_vec(&forged)?)?;
    let proof = evidence::proof(&path, evidence::relative(&fixture.repository, &path))?;
    match reader::verify_cargo_origin(
        &fixture.repository,
        &gate,
        &forged.process,
        Some(&proof),
        Some(owner.context()),
    ) {
        Err(error) => require(
            error.message().contains("launders workspace freshness"),
            format!("forgery failed before its freshness counterexample: {error}"),
        ),
        Ok(()) => Err(test_error(
            "fresh-only proof with coherent original hashes was admitted as cold origin",
        )),
    }
}

#[test]
fn rotation_retains_pinned_original_cold_run_and_removes_unrelated_expired_runs()
-> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let root = fixture.repository.join(".artifacts/lkjscript-dev/check");
    fs::create_dir_all(&root)?;
    let cold_directory = root.join("1-1-1");
    fs::create_dir(&cold_directory)?;
    let _ = fs::File::create(cold_directory.join("ACTIVE.lock"))?;
    let cold_owner = fixture.owner()?;
    let cold_gate = fixture.producer("retention-cold")?;
    let cold = fixture.execute_specification(
        &cold_owner,
        &cold_gate,
        fixture.specification_at(&cold_gate, &cold_directory),
    )?;
    passed(&cold)?;
    let cold_origin = origin(&cold)?.clone();
    let cold_proof = read_proof(&fixture.repository, &cold_origin)?;
    require(
        cold_proof
            .units
            .iter()
            .all(|entry| !entry.unit.fresh && entry.cold_proof.is_none()),
        "retention control did not establish an actual cold original",
    )?;
    drop(cold_owner);
    fs::File::open(&cold_directory)?
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1)))?;
    let mut expired = Vec::new();
    for number in [2_u64, 3] {
        let directory = root.join(format!("{number}-1-1"));
        fs::create_dir(&directory)?;
        let _ = fs::File::create(directory.join("ACTIVE.lock"))?;
        fs::File::open(&directory)?.set_times(
            fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(number)),
        )?;
        expired.push(directory);
    }
    let current = super::super::new_run_directory(&fixture.repository, "")?;
    let warm_owner = Run::with_commands_for_run(
        &fixture.repository,
        &fixture.target,
        &fixture.snapshot,
        &fixture.runtime,
        commands(),
        current.path(),
    )?;
    require(
        super::super::run_is_active(&cold_directory)?,
        "warm owner did not pin its original producer run before rotation",
    )?;
    super::super::rotate_runs_with_limit(&fixture.repository, current.path(), "", 2)?;
    require(
        cold_directory.is_dir() && expired.iter().all(|directory| !directory.exists()),
        "rotation lost the pinned cold original or retained unrelated expired runs",
    )?;
    require(
        evidence::proof(
            &proof_path(&fixture.repository, &cold_origin)?,
            cold_origin.path.clone(),
        )? == cold_origin,
        "rotation changed the original cold sidecar",
    )?;
    let warm_gate = fixture.producer("retention-warm")?;
    let warm = fixture.execute_specification(
        &warm_owner,
        &warm_gate,
        fixture.specification_at(&warm_gate, current.path()),
    )?;
    passed(&warm)?;
    let warm_proof = read_proof(&fixture.repository, origin(&warm)?)?;
    require(
        warm_proof
            .units
            .iter()
            .all(|entry| entry.unit.fresh && entry.cold_proof.as_ref() == Some(&cold_origin)),
        "warm reuse after rotation lost its original cold authority",
    )?;
    let context = warm_owner.context().clone();
    drop(warm_owner);
    drop(current);
    verify(
        &fixture.repository,
        &warm_gate,
        observation(&warm)?,
        origin(&warm)?,
        &context,
    )
}

#[test]
fn separate_bootstrap_target_and_retained_copy_allow_main_cold_then_valid_warm()
-> Result<(), DevError> {
    let fixture = Fixture::new()?;
    let bootstrap_root = fixture.repository.join(".artifacts/bootstrap-strategy");
    let bootstrap_target = bootstrap_root.join("target");
    fs::create_dir_all(&bootstrap_target)?;
    let mut bootstrap_gate = fixture.writer_at(
        "bootstrap",
        &[
            "build",
            "--workspace",
            "--release",
            "--locked",
            "--message-format=json",
        ],
        &bootstrap_target,
    )?;
    let bootstrap_binary = bootstrap_target.join("release/lkjscript");
    bootstrap_gate.required_outputs = vec![bootstrap_binary.clone()];
    let specification = fixture.specification(&bootstrap_gate)?;
    let process = process::run_selected(
        &specification,
        &fixture.repository,
        Some(Path::new(env!("CARGO"))),
    );
    successful_process(&bootstrap_gate, &process)?;
    super::super::cargo_outputs::verify(
        &fixture.repository,
        &bootstrap_gate,
        &specification.stdout_path,
        &process.stdout,
    )?;
    let bootstrap_units = units::read_units(
        &fixture.repository,
        &bootstrap_target,
        &[path_string(&fixture.repository.join("Cargo.toml"))?],
        &bootstrap_gate,
        &specification.stdout_path,
        &process.stdout,
    )?;
    require(
        bootstrap_units.len() >= 2
            && bootstrap_units.iter().all(|unit| !unit.fresh)
            && bootstrap_units.iter().all(|unit| {
                unit.filenames
                    .iter()
                    .all(|file| Path::new(&file.path).starts_with(&bootstrap_target))
            }),
        "bootstrap did not compile its workspace library and binary in the distinct owned target",
    )?;
    let retained_binary = bootstrap_root.join("retained-lkjscript");
    fs::copy(&bootstrap_binary, &retained_binary)?;
    let mut permissions = fs::metadata(&retained_binary)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&retained_binary, permissions)?;
    let bootstrap_file = evidence::proof(&bootstrap_binary, path_string(&bootstrap_binary)?)?;
    let retained_file = evidence::proof(
        &retained_binary,
        evidence::relative(&fixture.repository, &retained_binary),
    )?;
    require(
        retained_file.digest == bootstrap_file.digest
            && retained_file.bytes == bootstrap_file.bytes,
        "retained bootstrap copy differs from the selected producer binary",
    )?;
    require(
        snapshot::capture(&fixture.repository)?.digest == fixture.snapshot.digest,
        "ignored bootstrap target or copied binary changed the guarded source snapshot",
    )?;
    let owner = fixture.owner()?;
    let (_, cold, cold_proof) = fixture.cold(&owner)?;
    let cold_origin = origin(&cold)?.clone();
    require(
        cold_proof.units.iter().all(|entry| {
            entry
                .unit
                .filenames
                .iter()
                .all(|file| Path::new(&file.path).starts_with(&fixture.target))
        }),
        "first guarded producer used bootstrap artifacts instead of the separate main target",
    )?;
    let warm_gate = fixture.producer("bootstrap-main-warm")?;
    let warm = fixture.execute(&owner, &warm_gate)?;
    passed(&warm)?;
    let warm_proof = read_proof(&fixture.repository, origin(&warm)?)?;
    require(
        warm_proof
            .units
            .iter()
            .all(|entry| entry.unit.fresh && entry.cold_proof.as_ref() == Some(&cold_origin)),
        "main target warm reuse did not retain its own actual cold producer",
    )?;
    require(
        evidence::proof(&retained_binary, retained_file.path.clone())? == retained_file,
        "guarded main cold/warm producers changed the immutable bootstrap copy",
    )?;
    verify(
        &fixture.repository,
        &warm_gate,
        observation(&warm)?,
        origin(&warm)?,
        owner.context(),
    )
}

fn cold_binary(proof: &Proof) -> Result<&ProvedUnit, DevError> {
    let mut binaries = proof
        .units
        .iter()
        .filter(|entry| shared_binary(&entry.unit));
    let binary = binaries
        .next()
        .ok_or_else(|| test_error("producer has no shared binary unit"))?;
    require(
        binaries.next().is_none(),
        "fixture has more than one shared binary unit",
    )?;
    Ok(binary)
}

fn restore_cold_files(repository: &Path, proof: &Proof) -> Result<(), DevError> {
    for entry in &proof.units {
        require(
            !entry.unit.fresh && entry.cold_proof.is_none(),
            "restoration inventory does not belong to an actual cold producer",
        )?;
        units::verify_retained(repository, &entry.unit)?;
        for (original, retained) in entry.unit.filenames.iter().zip(&entry.unit.retained) {
            let path = Path::new(&original.path);
            require(
                path.starts_with(Path::new(&proof.context.target)),
                "restoration escaped its owned physical target",
            )?;
            if evidence::proof(path, original.path.clone())? != *original {
                fs::copy(proof_path(repository, retained)?, path)?;
            }
            verify_file(path, original)?;
        }
    }
    Ok(())
}

fn warm_points_to(
    fixture: &Fixture,
    receipt: &GateReceipt,
    original: &FileProof,
) -> Result<(), DevError> {
    passed(receipt)?;
    let proof = read_proof(&fixture.repository, origin(receipt)?)?;
    require(
        proof
            .units
            .iter()
            .all(|entry| entry.unit.fresh && entry.cold_proof.as_ref() == Some(original)),
        "warm observation did not retain its own exact original cold authority",
    )
}

#[test]
fn command_and_feature_variants_preserve_cold_history_without_admitting_unknown_bytes()
-> Result<(), DevError> {
    let fixture = Fixture::with_feature(true)?;
    let result = command_and_feature_variant_control(&fixture);
    if result.is_err() {
        let preserved = fixture._directory.keep();
        eprintln!(
            "Preserved Cargo-origin variant failure: {}",
            preserved.display()
        );
    }
    result
}

fn command_and_feature_variant_control(fixture: &Fixture) -> Result<(), DevError> {
    let owner = fixture.owner()?;
    let (a_gate, a_cold, a_proof) = fixture.cold(&owner)?;
    let a_origin = origin(&a_cold)?.clone();
    let a_binary = cold_binary(&a_proof)?;
    let shared_path = fixture.target.join("release/lkjscript");
    let a_shared_file = a_binary
        .unit
        .filenames
        .iter()
        .find(|file| Path::new(&file.path) == shared_path)
        .ok_or_else(|| test_error("A cold unit did not bind the shared top-level binary"))?;

    // Cargo can restore a deleted top-level output from its cached binary unit.
    // A content-preserving timestamp change makes only the owned binary source
    // newer than its completed prior compilation; the library stays warm.
    let main_path = fixture.repository.join("src/main.rs");
    let main_before = evidence::proof(&main_path, "src/main.rs".to_owned())?;
    let main_modified = fs::metadata(&main_path)?.modified()?;
    fs::File::open(&main_path)?.set_modified(SystemTime::now())?;
    require(
        evidence::proof(&main_path, "src/main.rs".to_owned())? == main_before
            && snapshot::capture(&fixture.repository)?.digest == fixture.snapshot.digest,
        "binary timestamp trigger changed its source authority",
    )?;
    fs::remove_file(&shared_path)?;
    let mut b_gate = fixture.writer(
        "variant-b-command",
        &[
            "build",
            "--workspace",
            "--release",
            "--locked",
            "--message-format=json",
            "--jobs=1",
        ],
    )?;
    b_gate.required_outputs = vec![shared_path.clone()];
    let b_cold = fixture.execute(&owner, &b_gate);
    fs::File::open(&main_path)?.set_modified(main_modified)?;
    require(
        evidence::proof(&main_path, "src/main.rs".to_owned())? == main_before,
        "restored binary timestamp changed its source authority",
    )?;
    let b_cold = b_cold?;
    passed(&b_cold)?;
    let b_origin = origin(&b_cold)?.clone();
    let b_proof = read_proof(&fixture.repository, &b_origin)?;
    let b_binary = cold_binary(&b_proof)?;
    require(
        !b_binary.unit.fresh
            && b_binary.cold_proof.is_none()
            && units::unit_key(&a_binary.unit)? == units::unit_key(&b_binary.unit)?
            && a_gate.identity_command() != b_gate.identity_command()
            && b_proof
                .units
                .iter()
                .any(|entry| !shared_binary(&entry.unit))
            && b_proof
                .units
                .iter()
                .filter(|entry| !shared_binary(&entry.unit))
                .all(|entry| entry.unit.fresh && entry.cold_proof.as_ref() == Some(&a_origin)),
        "alternate command did not compile its own same-metadata binary while reusing A's proved library",
    )?;
    verify(
        &fixture.repository,
        &a_gate,
        observation(&a_cold)?,
        &a_origin,
        owner.context(),
    )?;
    restore_cold_files(&fixture.repository, &a_proof)?;
    let a_warm_gate = fixture.producer("a-warm-after-command-b")?;
    let a_warm = fixture.execute(&owner, &a_warm_gate)?;
    warm_points_to(fixture, &a_warm, &a_origin)?;

    let mut feature_gate = fixture.writer(
        "variant-b-feature",
        &[
            "build",
            "--workspace",
            "--release",
            "--locked",
            "--message-format=json",
            "--bins",
            "--features",
            "variant-b",
        ],
    )?;
    feature_gate.required_outputs = vec![shared_path.clone()];
    let feature_cold = fixture.execute(&owner, &feature_gate)?;
    passed(&feature_cold)?;
    let feature_origin = origin(&feature_cold)?.clone();
    let feature_proof = read_proof(&fixture.repository, &feature_origin)?;
    let feature_binary = cold_binary(&feature_proof)?;
    let feature_shared_file = feature_binary
        .unit
        .filenames
        .iter()
        .find(|file| Path::new(&file.path) == shared_path)
        .ok_or_else(|| test_error("feature variant did not bind the same shared binary path"))?;
    require(
        feature_proof
            .units
            .iter()
            .all(|entry| !entry.unit.fresh && entry.cold_proof.is_none())
            && feature_binary.unit.features != a_binary.unit.features
            && feature_shared_file.digest != a_shared_file.digest,
        "feature variant did not produce distinct cold bytes at the shared binary path",
    )?;
    verify(
        &fixture.repository,
        &a_gate,
        observation(&a_cold)?,
        &a_origin,
        owner.context(),
    )?;
    restore_cold_files(&fixture.repository, &a_proof)?;
    let restored_gate = fixture.producer("a-warm-after-feature-b")?;
    let restored = fixture.execute(&owner, &restored_gate)?;
    warm_points_to(fixture, &restored, &a_origin)?;
    for original in [&a_origin, &b_origin, &feature_origin] {
        require(
            evidence::proof(
                &proof_path(&fixture.repository, original)?,
                original.path.clone(),
            )? == *original,
            "variant switching replaced or renamed an original cold proof",
        )?;
    }

    // History can admit only matching recorded bytes, and a dirty writer revokes
    // warm permission even after a known retained copy is restored.
    fs::copy(
        &shared_path,
        fixture
            .repository
            .join(".artifacts/lifecycle/variant-a-before-unknown.original"),
    )?;
    fs::write(&shared_path, b"unproved shared binary counterexample\n")?;
    let unknown = fixture.execute(&owner, &fixture.producer("a-unknown-bytes")?)?;
    warm_refused(&unknown)?;
    durable_dirty(&owner)?;
    restore_cold_files(&fixture.repository, &a_proof)?;
    let still_dirty = fixture.execute(&owner, &fixture.producer("a-known-bytes-still-dirty")?)?;
    warm_refused(&still_dirty)?;
    durable_dirty(&owner)?;
    combined_history_capacity_control(
        fixture,
        &owner,
        &[
            (&a_gate, &a_cold, &a_proof),
            (&b_gate, &b_cold, &b_proof),
            (&feature_gate, &feature_cold, &feature_proof),
        ],
    )
}

fn artifact_bytes<'a>(files: impl IntoIterator<Item = &'a FileProof>) -> Result<u64, DevError> {
    let mut total = 0_u64;
    for file in files {
        require(
            file.kind == FileKind::File,
            "capacity oracle contains a non-file artifact",
        )?;
        let bytes = file
            .bytes
            .ok_or_else(|| test_error("capacity oracle artifact has no length"))?;
        total = total
            .checked_add(bytes)
            .ok_or_else(|| test_error("capacity oracle byte count overflow"))?;
    }
    Ok(total)
}

fn combined_history_capacity_control(
    fixture: &Fixture,
    owner: &Run,
    originals: &[(&Gate, &GateReceipt, &Proof)],
) -> Result<(), DevError> {
    require(
        originals.len() == 3,
        "combined capacity control requires its three actual producer proofs",
    )?;
    let history = owner
        .state
        .lock()
        .map_err(|_| test_error("capacity control ledger was poisoned"))?
        .origins
        .clone();
    require(
        history.iter().all(|entry| {
            originals
                .iter()
                .any(|(_, receipt, _)| receipt.cargo_origin.as_ref() == Some(&entry.cold_proof))
        }),
        "capacity oracle omitted a retained cold producer",
    )?;
    let (_, a_receipt, a_proof) = originals
        .first()
        .ok_or_else(|| test_error("capacity control A proof is absent"))?;
    let a_origin = origin(a_receipt)?;
    let a_history: Vec<_> = history
        .iter()
        .filter(|entry| entry.cold_proof == *a_origin)
        .cloned()
        .collect();
    require(
        a_history.iter().any(|entry| shared_binary(&entry.unit))
            && a_history.iter().any(|entry| !shared_binary(&entry.unit)),
        "deduplication control lacks distinct library and binary origins sharing A's original proof",
    )?;
    let a_bytes = artifact_bytes(a_proof.units.iter().flat_map(|entry| &entry.unit.retained))?;
    require(
        a_bytes > 1,
        "actual A proof has no meaningful streamed artifact budget",
    )?;
    let (pins, streamed) = preflight_origins(
        &fixture.repository,
        owner.context(),
        &a_history,
        owner.current_run.as_deref(),
        a_bytes,
    )?;
    require(
        streamed == a_bytes,
        "shared original references were streamed more than once",
    )?;
    drop(pins);
    refused_because(
        preflight_origins(
            &fixture.repository,
            owner.context(),
            &a_history,
            owner.current_run.as_deref(),
            a_bytes - 1,
        ),
        "artifact streams exceed",
    )?;

    let mut union_bytes = 0_u64;
    let mut component_bytes = Vec::new();
    for (_, _, proof) in originals {
        let bytes = artifact_bytes(proof.units.iter().flat_map(|entry| &entry.unit.retained))?;
        component_bytes.push(bytes);
        union_bytes = union_bytes
            .checked_add(bytes)
            .ok_or_else(|| test_error("combined capacity oracle overflow"))?;
    }
    require(
        union_bytes > a_bytes && union_bytes <= MAXIMUM_CLOSURE_ARTIFACT_BYTES,
        "combined real artifact budget does not fit the production ceiling",
    )?;
    let (pins, streamed) = preflight_origins(
        &fixture.repository,
        owner.context(),
        &history,
        owner.current_run.as_deref(),
        union_bytes,
    )?;
    require(
        streamed == union_bytes,
        "combined original closure did not stream every unique producer proof exactly once",
    )?;
    drop(pins);
    let lower_limit = union_bytes - 1;
    refused_because(
        preflight_origins(
            &fixture.repository,
            owner.context(),
            &history,
            owner.current_run.as_deref(),
            lower_limit,
        ),
        "artifact streams exceed",
    )?;
    let mut lengths = component_bytes.into_iter();
    let a_component = lengths
        .next()
        .ok_or_else(|| test_error("A capacity component is absent"))?;
    let b_component = lengths
        .next()
        .ok_or_else(|| test_error("B capacity component is absent"))?;
    let feature_component = lengths
        .next()
        .ok_or_else(|| test_error("feature capacity component is absent"))?;
    let b_closure = a_component
        .checked_add(b_component)
        .ok_or_else(|| test_error("B original-reference closure overflow"))?;
    require(
        b_closure <= lower_limit && feature_component <= lower_limit,
        "an individual original closure already exceeds the union-only limit",
    )?;

    let cleanup = fixture.writer(
        "history-cap-clean",
        &["clean", "--release", "--package", "lkjscript", "--locked"],
    )?;
    let cleanup_receipt = fixture.execute(owner, &cleanup)?;
    passed(&cleanup_receipt)?;
    require(
        cleanup_receipt.cargo_origin.is_none(),
        "package cleanup minted producer evidence",
    )?;
    durable_dirty(owner)?;
    let gate = fixture.producer("history-cap-new-cold")?;
    let mut transaction = owner.begin(&gate)?;
    let specification = fixture.specification(&gate)?;
    let process = process::run_selected(
        &specification,
        &fixture.repository,
        Some(Path::new(env!("CARGO"))),
    );
    successful_process(&gate, &process)?;
    let selected = units::read_units(
        &fixture.repository,
        &fixture.target,
        &owner.context.workspace_manifests,
        &gate,
        &specification.stdout_path,
        &process.stdout,
    )?;
    require(
        selected.len() >= 2 && selected.iter().all(|unit| !unit.fresh),
        "owned package cleanup did not produce an actual complete cold establishment",
    )?;
    let new_cold_bytes = artifact_bytes(selected.iter().flat_map(|unit| &unit.filenames))?;
    let (_, _, b_proof) = originals
        .get(1)
        .ok_or_else(|| test_error("capacity control B proof is absent"))?;
    require(
        b_proof
            .units
            .iter()
            .any(|entry| entry.cold_proof.as_ref() == Some(a_origin)),
        "B's retained cold sidecar does not preserve A transitively in the candidate union",
    )?;
    let candidate_union_bytes = union_bytes
        .checked_add(new_cold_bytes)
        .ok_or_else(|| test_error("new plus historical capacity oracle overflow"))?;
    let commit_limit = union_bytes.max(new_cold_bytes);
    require(
        candidate_union_bytes > commit_limit
            && candidate_union_bytes <= MAXIMUM_CLOSURE_ARTIFACT_BYTES,
        "old and new cold subsets do not separately fit a meaningful combined-only limit",
    )?;
    refused_because(
        transaction.finish_with_artifact_limit(
            &gate,
            &process,
            specification
                .stdout_path
                .parent()
                .ok_or_else(|| test_error("capacity producer log parent is absent"))?,
            commit_limit,
        ),
        "artifact streams exceed",
    )?;
    drop(transaction);
    durable_dirty(owner)?;
    for (gate, receipt, _) in originals {
        let original = origin(receipt)?;
        require(
            evidence::proof(
                &proof_path(&fixture.repository, original)?,
                original.path.clone(),
            )? == *original,
            "combined capacity refusal changed an original cold proof",
        )?;
        verify(
            &fixture.repository,
            gate,
            observation(receipt)?,
            original,
            owner.context(),
        )?;
    }
    Ok(())
}
