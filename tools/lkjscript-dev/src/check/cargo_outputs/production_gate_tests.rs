use super::*;
use std::fs;
use std::path::PathBuf;

pub(super) fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("owned Cargo fixture");
    fs::create_dir_all(root.path().join("src")).expect("fixture source");
    fs::create_dir_all(root.path().join("target/release")).expect("declared output parent");
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"lkjscript\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    )
    .expect("fixture manifest");
    fs::write(
        root.path().join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"lkjscript\"\nversion = \"0.0.0\"\n",
    )
    .expect("fixture lockfile");
    fs::write(root.path().join("src/main.rs"), "fn main() {}\n")
        .map_err(DevError::from)
        .and_then(|()| origin_prepare_single(root.path()))
        .expect("fixture program");
    fs::write(
        root.path().join("target/release/lkjscript"),
        b"stale-owned-output\n",
    )
    .expect("pre-existing regular output");
    root
}

pub(super) fn build(repository: &Path, target: &Path, name: &str) -> GateReceipt {
    let run = repository.join(".artifacts").join(name);
    fs::create_dir_all(&run).expect("fixture run directory");
    let mut command: Vec<String> = [
        "cargo",
        "build",
        "--workspace",
        "--release",
        "--locked",
        "--message-format=json",
        "--target-dir",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    command.push(target.to_str().expect("fixture target UTF-8").to_owned());
    let mut gate = Gate::new(name, command);
    gate.required_outputs = vec![repository.join("target/release/lkjscript")];
    origin_single_build(repository, target, &gate, &run).expect("joined producer observation")
}

#[test]
fn redirected_cargo_cannot_certify_a_preexisting_regular_output() {
    let fixture = fixture();
    let receipt = build(
        fixture.path(),
        &fixture.path().join("elsewhere"),
        "redirected",
    );
    assert_eq!(
        receipt.process.as_ref().expect("process").status,
        ProcessStatus::Passed
    );
    assert_eq!(
        fs::read(fixture.path().join("target/release/lkjscript")).expect("stale output"),
        b"stale-owned-output\n"
    );
    let actual = fixture.path().join("elsewhere/release/lkjscript");
    assert!(
        actual.is_file(),
        "Cargo did produce its separately selected executable"
    );
    if receipt.status != GateStatus::Failed {
        eprintln!("Preserved failing fixture: {}", fixture.keep().display());
    }
    assert_eq!(
        receipt.status,
        GateStatus::Failed,
        "stale output accepted: {receipt:#?}"
    );
    assert!(
        receipt
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("cargo_output_binding:"))
    );
}

#[test]
fn exact_cargo_output_and_unchanged_cargo_cache_hit_both_pass() {
    let fixture = fixture();
    let target: PathBuf = fixture.path().join("target");
    for name in ["cold", "warm"] {
        let receipt = build(fixture.path(), &target, name);
        assert_eq!(receipt.status, GateStatus::Passed, "{name}: {receipt:#?}");
        assert_eq!(receipt.outputs.len(), 1);
        assert_eq!(receipt.retained_outputs.len(), 1);
        let log = fs::read_to_string(
            fixture
                .path()
                .join(".artifacts")
                .join(name)
                .join(format!("{name}.stdout.log")),
        )
        .expect("original producer records");
        assert!(log.contains(if name == "cold" {
            "\"fresh\":false"
        } else {
            "\"fresh\":true"
        }));
    }
}

#[test]
fn a_working_predecessor_executable_cannot_stand_in_for_the_current_source() {
    let fixture = fixture();
    let root = fixture.path();
    let source = root.join("src/main.rs");
    fs::write(&source, "fn main() { println!(\"predecessor\"); }\n").expect("predecessor source");
    let before = build(root, &root.join("target"), "before");
    assert_eq!(before.status, GateStatus::Passed);
    let old_binary = root.join("target/release/lkjscript");
    let old_bytes = fs::read(&old_binary).expect("original executable bytes");
    fs::write(&source, "fn main() { println!(\"current-source\"); }\n").expect("changed source");
    let changed = build(root, &root.join("elsewhere"), "changed");
    assert_eq!(
        changed.process.as_ref().expect("Cargo process").status,
        ProcessStatus::Passed
    );
    assert_eq!(
        fs::read(&old_binary).expect("unchanged predecessor"),
        old_bytes
    );
    for (path, expected) in [
        (old_binary, b"predecessor\n".as_slice()),
        (
            root.join("elsewhere/release/lkjscript"),
            b"current-source\n".as_slice(),
        ),
    ] {
        let outcome = std::process::Command::new(path)
            .output()
            .expect("joined owned executable");
        assert!(outcome.status.success());
        assert_eq!(outcome.stdout, expected);
    }
    assert_eq!(
        changed.status,
        GateStatus::Failed,
        "a runnable old product is still the wrong producer output"
    );
    assert!(
        changed
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("cargo_output_binding:"))
    );
}

fn origin_runtime(repository: &Path) -> Result<RuntimeIdentity, DevError> {
    snapshot::runtime_identity(repository, [(env!("CARGO").to_owned(), "cargo".to_owned())])
}

fn origin_scope(
    repository: &Path,
    target: &Path,
    input: &InputSnapshot,
    runtime: &RuntimeIdentity,
) -> Result<crate::check::cargo_origin::Run, DevError> {
    crate::check::cargo_origin::Run::with_commands(
        repository,
        target,
        input,
        runtime,
        vec![(env!("CARGO").to_owned(), "cargo".to_owned())],
    )
}

fn origin_prepare_single(repository: &Path) -> Result<(), DevError> {
    let fixture = CargoOriginFixture::borrowed(repository)?;
    fs::write(
        repository.join(".gitignore"),
        ".artifacts/\ntarget/\nelsewhere/\n",
    )?;
    fixture.git(
        "git-init",
        vec![
            "init".to_owned(),
            "--initial-branch=main".to_owned(),
            format!("--template={}", origin_path(&fixture.template)?),
        ],
    )?;
    fixture.git(
        "git-add",
        [
            "add",
            "--",
            "Cargo.toml",
            "Cargo.lock",
            ".gitignore",
            "src/main.rs",
        ]
        .map(str::to_owned)
        .to_vec(),
    )?;
    fixture.git(
        "git-commit",
        ["commit", "-m", "owned Cargo output fixture"]
            .map(str::to_owned)
            .to_vec(),
    )
}

fn origin_single_build(
    repository: &Path,
    target: &Path,
    gate: &Gate,
    run: &Path,
) -> Result<GateReceipt, DevError> {
    let before = snapshot::capture(repository)?;
    let runtime = origin_runtime(repository)?;
    let origin = origin_scope(repository, target, &before, &runtime)?;
    let selected = snapshot::observe_executable(repository, env!("CARGO"))?;
    let fingerprint = fingerprint_with_executable(
        repository,
        gate,
        &before,
        &runtime,
        &BTreeMap::new(),
        selected.proof,
    )?;
    execute_fresh_with_origin(
        FreshInput {
            repository,
            gate,
            inputs: Vec::new(),
            program: Some(Path::new(env!("CARGO"))),
            fingerprint,
            stdout_path: run.join(format!("{}.stdout.log", gate.name)),
            stderr_path: run.join(format!("{}.stderr.log", gate.name)),
            started_wall: unix_nanoseconds()?,
            started: Instant::now(),
            cache_observation: CacheObservation {
                eligible: false,
                lookup: CacheLookupStatus::Bypassed,
                reason: None,
                record: None,
                write: None,
            },
        },
        &origin,
    )
}

// This fixture deliberately invokes raw Cargo separately from executor admission.
// Its tiny binaries are diagnostic literals, never admitted product executables.
struct CargoOriginFixture {
    directory: Option<tempfile::TempDir>,
    a: PathBuf,
    b: PathBuf,
    target: PathBuf,
    runs: PathBuf,
    template: PathBuf,
}

struct CargoOriginBuild {
    process: crate::process::ProcessObservation,
    directory: PathBuf,
    records: Vec<serde_json::Value>,
}

struct CargoOriginArtifact {
    fresh: bool,
    retained: Vec<FileProof>,
    executable: Option<PathBuf>,
}

enum CargoOriginAttempt {
    Produced(Box<(GateReceipt, CargoOriginBuild)>),
    Preflight(String),
}

impl Drop for CargoOriginFixture {
    fn drop(&mut self) {
        if let Some(directory) = self.directory.take() {
            eprintln!(
                "Preserved Cargo origin fixture: {}",
                directory.keep().display()
            );
        }
    }
}

impl CargoOriginFixture {
    fn borrowed(repository: &Path) -> Result<Self, DevError> {
        let repository = repository.canonicalize()?;
        let runs = repository.join(".artifacts/origin-setup");
        let template = runs.join("empty-git-template");
        fs::create_dir_all(&template)?;
        fs::write(runs.join("empty-git-config"), b"")?;
        Ok(Self {
            directory: None,
            a: repository.clone(),
            b: repository.clone(),
            target: repository.join("target"),
            runs,
            template,
        })
    }

    fn new() -> Result<Self, DevError> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?;
        let fixture = Self {
            directory: Some(directory),
            a: root.join("a"),
            b: root.join("b"),
            target: root.join("shared-target"),
            runs: root.join("originals"),
            template: root.join("empty-git-template"),
        };
        fs::create_dir_all(fixture.a.join("src"))?;
        fs::create_dir(&fixture.runs)?;
        fs::create_dir(&fixture.template)?;
        fs::write(fixture.runs.join("empty-git-config"), b"")?;
        fs::write(
            fixture.a.join("Cargo.toml"),
            "[package]\nname = \"lkjscript\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
        )?;
        fs::write(
            fixture.a.join("Cargo.lock"),
            "version = 4\n[[package]]\nname = \"lkjscript\"\nversion = \"0.0.0\"\n",
        )?;
        fs::write(fixture.a.join(".gitignore"), ".artifacts/\ntarget/\n")?;
        fixture.write_library(&fixture.a, 17)?;
        fixture.write_binary(&fixture.a, "A", false)?;
        fixture.git(
            "git-init",
            vec![
                "init".to_owned(),
                "--initial-branch=main".to_owned(),
                format!("--template={}", origin_path(&fixture.template)?),
            ],
        )?;
        fixture.git(
            "git-add",
            [
                "add",
                "--",
                "Cargo.toml",
                "Cargo.lock",
                ".gitignore",
                "src/lib.rs",
                "src/main.rs",
            ]
            .map(str::to_owned)
            .to_vec(),
        )?;
        fixture.git(
            "git-commit",
            ["commit", "-m", "owned Cargo origin fixture"]
                .map(str::to_owned)
                .to_vec(),
        )?;
        fixture.git(
            "git-worktree",
            vec![
                "worktree".to_owned(),
                "add".to_owned(),
                "--detach".to_owned(),
                origin_path(&fixture.b)?.to_owned(),
                "HEAD".to_owned(),
            ],
        )?;
        fixture.write_library(&fixture.b, 29)?;
        fixture.write_binary(&fixture.b, "B", false)?;
        for repository in [&fixture.a, &fixture.b] {
            for relative in [
                "Cargo.toml",
                "Cargo.lock",
                ".gitignore",
                "src/lib.rs",
                "src/main.rs",
            ] {
                fixture.backdate(&repository.join(relative))?;
            }
        }
        Ok(fixture)
    }

    fn git(&self, name: &str, arguments: Vec<String>) -> Result<(), DevError> {
        let mut command = vec![
            "git".to_owned(),
            "-c".to_owned(),
            "user.name=CargoOriginFixture".to_owned(),
            "-c".to_owned(),
            "user.email=cargo-origin-fixture@example.invalid".to_owned(),
            "-c".to_owned(),
            "commit.gpgsign=false".to_owned(),
            "-c".to_owned(),
            format!("core.hooksPath={}", origin_path(&self.template)?),
        ];
        command.extend(arguments);
        let (_, observation) = self.run(
            &self.a,
            name,
            command,
            None,
            std::time::Duration::from_secs(60),
            1024 * 1024,
        )?;
        origin_passed(&observation, name)
    }

    fn write_library(&self, repository: &Path, marker: u8) -> Result<(), DevError> {
        fs::write(
            repository.join("src/lib.rs"),
            format!(
                "pub fn marker() -> u8 {{ {marker} }}\npub fn origin() -> &'static str {{ env!(\"CARGO_MANIFEST_DIR\") }}\n"
            ),
        )?;
        Ok(())
    }

    fn write_binary(&self, repository: &Path, marker: &str, fail: bool) -> Result<(), DevError> {
        let failure = if fail {
            "compile_error!(\"owned-origin-fixture deliberate failure\");\n"
        } else {
            ""
        };
        fs::write(
            repository.join("src/main.rs"),
            format!(
                "{failure}fn main() {{ println!(\"{marker}|{{}}|{{}}|{{}}\", lkjscript::marker(), lkjscript::origin(), env!(\"CARGO_MANIFEST_DIR\")); }}\n"
            ),
        )?;
        Ok(())
    }

    fn old_time() -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(946_684_800)
    }

    fn backdate(&self, path: &Path) -> Result<(), DevError> {
        // File::set_modified is stable since Rust 1.75; fs::set_times is not.
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)?
            .set_modified(Self::old_time())?;
        origin_require(
            fs::metadata(path)?.modified()? == Self::old_time(),
            format!("owned fixture mtime was not retained: {}", path.display()),
        )
    }

    fn advance(&self, path: &Path) -> Result<(), DevError> {
        let library_invoked = self.invoked("lib")?;
        let binary_invoked = self.invoked("bin")?;
        let modified = fs::metadata(self.executable())?
            .modified()?
            .max(library_invoked)
            .max(binary_invoked)
            .checked_add(std::time::Duration::from_secs(60))
            .ok_or_else(|| DevError::infrastructure("fixture mtime overflow"))?;
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)?
            .set_modified(modified)?;
        origin_require(
            fs::metadata(path)?.modified()? == modified
                && modified > library_invoked
                && modified > binary_invoked,
            format!(
                "owned fixture dirty mtime was not retained: {}",
                path.display()
            ),
        )
    }

    fn executable(&self) -> PathBuf {
        self.target.join("release/lkjscript")
    }

    fn invoked(&self, kind: &str) -> Result<SystemTime, DevError> {
        let mut matching = Vec::new();
        for entry in fs::read_dir(self.target.join("release/.fingerprint"))? {
            let directory = entry?.path();
            if directory.join(format!("{kind}-lkjscript.json")).is_file() {
                matching.push(fs::metadata(directory.join("invoked.timestamp"))?.modified()?);
            }
        }
        origin_require(
            matching.len() == 1,
            format!("expected one fixture {kind} fingerprint"),
        )?;
        matching
            .first()
            .copied()
            .ok_or_else(|| DevError::corrupt("missing fixture timestamp"))
    }

    fn backdated_inputs(&self) -> Result<(), DevError> {
        let library_invoked = self.invoked("lib")?;
        let binary_invoked = self.invoked("bin")?;
        for repository in [&self.a, &self.b] {
            for relative in [
                "Cargo.toml",
                "Cargo.lock",
                ".gitignore",
                "src/lib.rs",
                "src/main.rs",
            ] {
                let modified = fs::metadata(repository.join(relative))?.modified()?;
                origin_require(
                    modified < library_invoked && modified < binary_invoked,
                    format!(
                        "fixture backdate does not predate Cargo: {}",
                        repository.join(relative).display()
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn command(&self, repository: &Path) -> Result<Vec<String>, DevError> {
        let mut command = [
            "cargo",
            "build",
            "--release",
            "--locked",
            "--offline",
            "--jobs",
            "1",
            "--message-format=json",
            "--bin",
            "lkjscript",
            "--manifest-path",
        ]
        .map(str::to_owned)
        .to_vec();
        command.push(origin_path(&repository.join("Cargo.toml"))?.to_owned());
        command.push("--target-dir".to_owned());
        command.push(origin_path(&self.target)?.to_owned());
        Ok(command)
    }

    fn guarded_command(&self, repository: &Path) -> Result<Vec<String>, DevError> {
        let mut command = self.command(repository)?;
        let index = command
            .iter()
            .position(|argument| argument == "--manifest-path")
            .ok_or_else(|| DevError::corrupt("fixture command lacks its canonical manifest"))?;
        origin_require(
            command.get(index + 1).map(String::as_str)
                == Some(origin_path(&repository.join("Cargo.toml"))?),
            "fixture command points at a different manifest",
        )?;
        // Guarded Cargo already binds this canonical repository as its cwd.
        // The raw adversary retains its original explicit manifest argument.
        drop(command.drain(index..index + 2));
        Ok(command)
    }

    fn run(
        &self,
        repository: &Path,
        name: &str,
        command: Vec<String>,
        selected: Option<&Path>,
        timeout: std::time::Duration,
        stream_limit: u64,
    ) -> Result<(PathBuf, crate::process::ProcessObservation), DevError> {
        let directory = repository
            .join(".artifacts/cargo-origin-fixture")
            .join(name);
        fs::create_dir_all(
            directory
                .parent()
                .ok_or_else(|| DevError::infrastructure("fixture run has no parent"))?,
        )?;
        fs::create_dir(&directory)?;
        let mut environment = process::environment();
        if command.first().map(String::as_str) == Some("git") {
            environment.insert("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned());
            environment.insert(
                "GIT_CONFIG_GLOBAL".to_owned(),
                origin_path(&self.runs.join("empty-git-config"))?.to_owned(),
            );
        }
        let specification = ProcessSpec {
            command,
            cwd: repository.to_path_buf(),
            environment,
            timeout,
            maximum_stdout_bytes: stream_limit,
            maximum_stderr_bytes: stream_limit,
            stdout_path: directory.join("stdout.log"),
            stderr_path: directory.join("stderr.log"),
            unavailable_exit_code: None,
        };
        let observation = match selected {
            Some(program) => process::run_selected(&specification, repository, Some(program)),
            None => {
                origin_require(
                    specification.command.first().map(String::as_str) == Some("git"),
                    "only fixture Git may use PATH selection",
                )?;
                process::run(&specification, repository)
            }
        };
        evidence::publish_json(&directory.join("process.json"), &observation)?;
        Ok((directory, observation))
    }

    fn raw(&self, repository: &Path, name: &str) -> Result<CargoOriginBuild, DevError> {
        let before = snapshot::capture(repository)?;
        let (directory, observation) = self.run(
            repository,
            name,
            self.command(repository)?,
            Some(Path::new(env!("CARGO"))),
            std::time::Duration::from_secs(120),
            1024 * 1024,
        )?;
        self.finish_capture(repository, directory, observation, &before, "stdout.log")
    }

    fn admitted(
        &self,
        repository: &Path,
        name: &str,
        runtime: &RuntimeIdentity,
    ) -> Result<(GateReceipt, CargoOriginBuild), DevError> {
        self.admitted_command(
            repository,
            name,
            runtime,
            self.guarded_command(repository)?,
            self.executable(),
        )
    }

    fn admitted_command(
        &self,
        repository: &Path,
        name: &str,
        runtime: &RuntimeIdentity,
        command: Vec<String>,
        output: PathBuf,
    ) -> Result<(GateReceipt, CargoOriginBuild), DevError> {
        let before = snapshot::capture(repository)?;
        let mut gate = Gate::new("cargo-origin", command);
        gate.timeout = std::time::Duration::from_secs(120);
        gate.maximum_stdout_bytes = 1024 * 1024;
        gate.maximum_stderr_bytes = 1024 * 1024;
        gate.required_outputs = vec![output];
        let selected = snapshot::observe_executable(repository, env!("CARGO"))?;
        let fingerprint = fingerprint_with_executable(
            repository,
            &gate,
            &before,
            runtime,
            &BTreeMap::new(),
            selected.proof,
        )?;
        let directory = repository
            .join(".artifacts/cargo-origin-fixture")
            .join(name);
        fs::create_dir_all(
            directory
                .parent()
                .ok_or_else(|| DevError::infrastructure("fixture run has no parent"))?,
        )?;
        fs::create_dir(&directory)?;
        let origin = origin_scope(repository, &self.target, &before, runtime)?;
        let receipt = execute_fresh_with_origin(
            FreshInput {
                repository,
                gate: &gate,
                inputs: Vec::new(),
                program: Some(Path::new(env!("CARGO"))),
                fingerprint,
                stdout_path: directory.join("cargo-origin.stdout.log"),
                stderr_path: directory.join("cargo-origin.stderr.log"),
                started_wall: unix_nanoseconds()?,
                started: Instant::now(),
                cache_observation: CacheObservation {
                    eligible: false,
                    lookup: CacheLookupStatus::Bypassed,
                    reason: None,
                    record: None,
                    write: None,
                },
            },
            &origin,
        )?;
        evidence::publish_json(&directory.join("gate.json"), &receipt)?;
        let observation = receipt.process.clone().ok_or_else(|| {
            DevError::corrupt(format!("missing original producer process: {name}"))
        })?;
        if receipt.status == GateStatus::Passed {
            let proof = receipt
                .cargo_origin
                .as_ref()
                .ok_or_else(|| DevError::corrupt("passing fixture producer has no origin proof"))?;
            crate::check::cargo_origin::verify(
                repository,
                &gate,
                &observation,
                proof,
                origin.context(),
            )?;
        }
        let build = self.finish_capture(
            repository,
            directory,
            observation,
            &before,
            "cargo-origin.stdout.log",
        )?;
        Ok((receipt, build))
    }

    fn finish_capture(
        &self,
        repository: &Path,
        directory: PathBuf,
        observation: crate::process::ProcessObservation,
        before: &InputSnapshot,
        stdout_file: &str,
    ) -> Result<CargoOriginBuild, DevError> {
        let after = snapshot::capture(repository)?;
        evidence::publish_json(&directory.join("before-source.json"), before)?;
        evidence::publish_json(&directory.join("after-source.json"), &after)?;
        origin_require(
            before.digest == after.digest,
            format!("fixture source changed during {}", directory.display()),
        )?;
        let bytes = fs::read(directory.join(stdout_file))?;
        origin_require(
            observation.stdout.bytes == Some(bytes.len() as u64)
                && observation.stdout.digest.as_ref() == Some(&VerificationDigest::of(&bytes)),
            "original Cargo stdout proof does not match retained bytes",
        )?;
        let mut records = Vec::new();
        for line in bytes.split(|byte| *byte == b'\n') {
            if line.first() == Some(&b'{') {
                records.push(serde_json::from_slice(line)?);
            }
        }
        Ok(CargoOriginBuild {
            process: observation,
            directory,
            records,
        })
    }

    fn negative_admitted(
        &self,
        repository: &Path,
        name: &str,
        runtime: &RuntimeIdentity,
    ) -> Result<CargoOriginAttempt, DevError> {
        let before = snapshot::capture(repository)?;
        match self.admitted(repository, name, runtime) {
            Ok(produced) => Ok(CargoOriginAttempt::Produced(Box::new(produced))),
            Err(error) if origin_preflight_refusal(&error) => {
                let directory = repository
                    .join(".artifacts/cargo-origin-fixture")
                    .join(name);
                origin_require(
                    !directory.join("cargo-origin.stdout.log").exists()
                        && !directory.join("cargo-origin.stderr.log").exists()
                        && !directory.join("gate.json").exists(),
                    "preflight refusal unexpectedly carries producer evidence",
                )?;
                let after = snapshot::capture(repository)?;
                origin_require(
                    before.digest == after.digest,
                    "preflight refusal changed fixture source",
                )?;
                evidence::publish_json(&directory.join("before-source.json"), &before)?;
                evidence::publish_json(&directory.join("after-source.json"), &after)?;
                evidence::publish_json(
                    &directory.join("preflight-refusal.json"),
                    &serde_json::json!({
                        "kind": error.kind(),
                        "message": error.message(),
                        "producer_executed": false,
                    }),
                )?;
                Ok(CargoOriginAttempt::Preflight(error.message().to_owned()))
            }
            Err(error) => Err(error),
        }
    }

    fn behavior(
        &self,
        name: &str,
        binary: &CargoOriginArtifact,
        expected: &str,
    ) -> Result<(), DevError> {
        let executable = binary.executable.as_ref().ok_or_else(|| {
            DevError::corrupt("fixture binary artifact has no retained executable")
        })?;
        let (directory, observation) = self.run(
            &self.a,
            name,
            vec![origin_path(executable)?.to_owned()],
            Some(executable),
            std::time::Duration::from_secs(10),
            64 * 1024,
        )?;
        origin_passed(&observation, name)?;
        origin_require(
            fs::read(directory.join("stdout.log"))? == expected.as_bytes(),
            format!("independent fixture behavior differs: {name}; expected {expected:?}"),
        )?;
        origin_require(
            fs::read(directory.join("stderr.log"))?.is_empty(),
            format!("fixture behavior produced unexpected stderr: {name}"),
        )
    }

    fn clean(&self) -> Result<(), DevError> {
        let command = vec![
            "cargo".to_owned(),
            "clean".to_owned(),
            "--release".to_owned(),
            "--locked".to_owned(),
            "--offline".to_owned(),
            "-p".to_owned(),
            "lkjscript".to_owned(),
            "--manifest-path".to_owned(),
            origin_path(&self.a.join("Cargo.toml"))?.to_owned(),
            "--target-dir".to_owned(),
            origin_path(&self.target)?.to_owned(),
        ];
        let (_, observation) = self.run(
            &self.a,
            "owned-target-clean",
            command,
            Some(Path::new(env!("CARGO"))),
            std::time::Duration::from_secs(120),
            1024 * 1024,
        )?;
        origin_passed(&observation, "owned-target-clean")
    }

    fn finish(mut self) -> Result<(), DevError> {
        let directory = self
            .directory
            .take()
            .ok_or_else(|| DevError::infrastructure("fixture resource was already retired"))?;
        directory.close()?;
        Ok(())
    }
}

impl CargoOriginBuild {
    fn completed(&self, expected: bool) -> Result<(), DevError> {
        let finished: Vec<_> = self
            .records
            .iter()
            .filter(|record| {
                record.get("reason").and_then(serde_json::Value::as_str) == Some("build-finished")
            })
            .collect();
        origin_require(
            finished.len() == 1
                && finished
                    .first()
                    .and_then(|record| record.get("success"))
                    .and_then(serde_json::Value::as_bool)
                    == Some(expected),
            format!(
                "unexpected build-finished evidence: {}",
                self.directory.display()
            ),
        )
    }

    fn artifact(&self, repository: &Path, kind: &str) -> Result<CargoOriginArtifact, DevError> {
        let manifest = repository.join("Cargo.toml");
        let matching: Vec<_> = self
            .records
            .iter()
            .filter(|record| {
                record.get("reason").and_then(serde_json::Value::as_str)
                    == Some("compiler-artifact")
                    && record
                        .get("manifest_path")
                        .and_then(serde_json::Value::as_str)
                        .map(Path::new)
                        == Some(manifest.as_path())
                    && record
                        .pointer("/target/name")
                        .and_then(serde_json::Value::as_str)
                        == Some("lkjscript")
                    && record
                        .pointer("/target/kind")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|kinds| kinds.len() == 1 && kinds[0].as_str() == Some(kind))
                    && record
                        .pointer("/profile/test")
                        .and_then(serde_json::Value::as_bool)
                        == Some(false)
            })
            .collect();
        origin_require(
            matching.len() == 1,
            format!(
                "expected one exact {kind} artifact: {}",
                self.directory.display()
            ),
        )?;
        let record = matching
            .first()
            .ok_or_else(|| DevError::corrupt("missing fixture artifact"))?;
        let fresh = record
            .get("fresh")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| DevError::corrupt("fixture artifact lacks Cargo fresh state"))?;
        origin_require(
            record
                .get("package_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty()),
            "fixture artifact lacks package identity",
        )?;
        let filenames = record
            .get("filenames")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| DevError::corrupt("fixture artifact lacks output filenames"))?;
        origin_require(!filenames.is_empty(), "fixture artifact has no outputs")?;
        let mut retained = Vec::new();
        let mut executable = None;
        for (index, filename) in filenames.iter().enumerate() {
            let source = Path::new(
                filename
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("fixture artifact output is not a path"))?,
            );
            let original = evidence::proof(source, origin_path(source)?.to_owned())?;
            origin_require(
                original.kind == FileKind::File,
                "fixture output is not a regular file",
            )?;
            let saved = self.directory.join(format!("selected-{kind}-{index}"));
            fs::copy(source, &saved)?;
            let saved_proof = evidence::proof(&saved, origin_path(&saved)?.to_owned())?;
            let mut comparison = saved_proof.clone();
            comparison.path.clone_from(&original.path);
            origin_require(
                comparison == original,
                "fixture output changed during retention",
            )?;
            if record
                .get("executable")
                .and_then(serde_json::Value::as_str)
                .map(Path::new)
                == Some(source)
            {
                origin_require(executable.is_none(), "fixture executable appeared twice")?;
                executable = Some(saved);
            }
            retained.push(saved_proof);
        }
        origin_require(
            (kind == "bin" && executable.is_some())
                || (kind == "lib"
                    && record
                        .get("executable")
                        .is_some_and(serde_json::Value::is_null)),
            "fixture target executable identity differs",
        )?;
        evidence::publish_json(
            &self.directory.join(format!("selected-{kind}.json")),
            &retained,
        )?;
        Ok(CargoOriginArtifact {
            fresh,
            retained,
            executable,
        })
    }
}

fn origin_path(path: &Path) -> Result<&str, DevError> {
    path.to_str()
        .ok_or_else(|| DevError::infrastructure("fixture path is not UTF-8"))
}

fn origin_require(condition: bool, message: impl Into<String>) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

fn origin_preflight_refusal(error: &DevError) -> bool {
    error.kind() == "corrupt"
        && error.message()
            == "Cargo compilation origin: warm workspace unit has no admitted cold origin"
}

fn origin_refused(receipt: &GateReceipt, phase: &str) -> Result<(), DevError> {
    origin_require(
        receipt.status == GateStatus::Failed
            && receipt.cargo_origin.is_none()
            && receipt.reason.as_deref()
                == Some(
                    "cargo_output_binding:Cargo compilation origin: warm workspace unit has no admitted cold origin",
                ),
        format!("foreign cached compile origin was not precisely refused at {phase}: {receipt:#?}"),
    )
}

fn origin_positive(receipt: &GateReceipt, phase: &str) -> Result<(), DevError> {
    origin_require(
        receipt.status == GateStatus::Passed
            && receipt
                .cargo_origin
                .as_ref()
                .is_some_and(|proof| proof.kind == FileKind::File && proof.digest.is_some()),
        format!("healthy producer lacks complete origin at {phase}: {receipt:#?}"),
    )
}

fn origin_cold_closure(
    repository: &Path,
    warm: &GateReceipt,
    cold: &GateReceipt,
) -> Result<(), DevError> {
    let warm_proof = warm
        .cargo_origin
        .as_ref()
        .ok_or_else(|| DevError::corrupt("warm fixture has no complete origin proof"))?;
    let cold_proof = cold
        .cargo_origin
        .as_ref()
        .ok_or_else(|| DevError::corrupt("cold fixture has no complete origin proof"))?;
    let cold_value = serde_json::to_value(cold_proof)?;
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(repository.join(&warm_proof.path))?)?;
    let units = value
        .get("units")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| DevError::corrupt("warm origin proof has no unit closure"))?;
    origin_require(
        units.len() == 2
            && units.iter().all(|entry| {
                entry.get("cold_proof") == Some(&cold_value)
                    && entry
                        .pointer("/unit/fresh")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
            }),
        "warm root lib/bin closure does not point to the exact initial cold producer",
    )
}

fn origin_passed(
    observation: &crate::process::ProcessObservation,
    phase: &str,
) -> Result<(), DevError> {
    origin_require(
        observation.status == ProcessStatus::Passed && observation.exit_code == Some(0),
        format!("owned fixture producer did not pass {phase}: {observation:?}"),
    )
}

fn origin_same_outputs(left: &CargoOriginArtifact, right: &CargoOriginArtifact) -> bool {
    left.retained.len() == right.retained.len()
        && left
            .retained
            .iter()
            .zip(&right.retained)
            .all(|(left, right)| {
                left.kind == right.kind
                    && left.mode == right.mode
                    && left.bytes == right.bytes
                    && left.digest == right.digest
            })
}

#[test]
fn shared_target_reuse_requires_exact_project_compile_origin() -> Result<(), DevError> {
    let fixture = CargoOriginFixture::new()?;
    let runtime = origin_runtime(&fixture.a)?;
    origin_require(
        runtime.cargo.starts_with("cargo 1.98.0 ") && runtime.rustc.starts_with("rustc 1.98.0 "),
        "Cargo origin fixture requires pinned Rust 1.98.0",
    )?;
    let a = origin_path(&fixture.a)?;
    let b = origin_path(&fixture.b)?;
    let healthy_a = format!("A|17|{a}|{a}\n");
    let (cold_receipt, cold) = fixture.admitted(&fixture.a, "a-cold", &runtime)?;
    origin_positive(&cold_receipt, "a-cold")?;
    origin_passed(&cold.process, "a-cold")?;
    cold.completed(true)?;
    let cold_library = cold.artifact(&fixture.a, "lib")?;
    let cold_binary = cold.artifact(&fixture.a, "bin")?;
    origin_require(
        !cold_library.fresh && !cold_binary.fresh,
        "cold project artifacts were reused",
    )?;
    fixture.behavior("a-cold-behavior", &cold_binary, &healthy_a)?;
    fixture.backdated_inputs()?;

    let mut debug_command = fixture.guarded_command(&fixture.a)?;
    debug_command.retain(|argument| argument != "--release");
    let (debug_receipt, debug) = fixture.admitted_command(
        &fixture.a,
        "a-unrelated-debug-writer",
        &runtime,
        debug_command,
        fixture.target.join("debug/lkjscript"),
    )?;
    origin_positive(&debug_receipt, "a-unrelated-debug-writer")?;
    origin_passed(&debug.process, "a-unrelated-debug-writer")?;
    debug.completed(true)?;
    let debug_library = debug.artifact(&fixture.a, "lib")?;
    let debug_binary = debug.artifact(&fixture.a, "bin")?;
    origin_require(
        !debug_library.fresh && !debug_binary.fresh,
        "first unrelated debug writer reused project artifacts",
    )?;
    fixture.behavior("a-debug-writer-behavior", &debug_binary, &healthy_a)?;

    let (warm_receipt, warm) = fixture.admitted(&fixture.a, "a-warm", &runtime)?;
    origin_positive(&warm_receipt, "a-warm")?;
    origin_cold_closure(&fixture.a, &warm_receipt, &cold_receipt)?;
    origin_passed(&warm.process, "a-warm")?;
    warm.completed(true)?;
    let warm_library = warm.artifact(&fixture.a, "lib")?;
    let warm_binary = warm.artifact(&fixture.a, "bin")?;
    origin_require(
        warm_library.fresh
            && warm_binary.fresh
            && origin_same_outputs(&cold_library, &warm_library)
            && origin_same_outputs(&cold_binary, &warm_binary),
        "matching warm reuse changed outputs or rebuilt project artifacts",
    )?;
    fixture.behavior("a-warm-behavior", &warm_binary, &healthy_a)?;

    // Raw observations below cannot create a passing executor-origin receipt.
    let foreign = fixture.raw(&fixture.b, "b-foreign-raw")?;
    origin_passed(&foreign.process, "b-foreign-raw")?;
    foreign.completed(true)?;
    let foreign_library = foreign.artifact(&fixture.b, "lib")?;
    let foreign_binary = foreign.artifact(&fixture.b, "bin")?;
    origin_require(
        foreign_library.fresh
            && foreign_binary.fresh
            && origin_same_outputs(&cold_library, &foreign_library)
            && origin_same_outputs(&cold_binary, &foreign_binary),
        "pinned Cargo did not reproduce two-worktree cached reuse",
    )?;
    fixture.behavior("b-foreign-behavior", &foreign_binary, &healthy_a)?;

    fixture.advance(&fixture.b.join("src/main.rs"))?;
    let mixed = fixture.raw(&fixture.b, "b-fresh-bin-raw")?;
    origin_passed(&mixed.process, "b-fresh-bin-raw")?;
    mixed.completed(true)?;
    let mixed_library = mixed.artifact(&fixture.b, "lib")?;
    let mixed_binary = mixed.artifact(&fixture.b, "bin")?;
    origin_require(
        mixed_library.fresh
            && !mixed_binary.fresh
            && origin_same_outputs(&cold_library, &mixed_library),
        "fresh-bin/stale-library reproduction differed",
    )?;
    fixture.behavior(
        "b-mixed-behavior",
        &mixed_binary,
        &format!("B|17|{a}|{b}\n"),
    )?;
    match fixture.negative_admitted(&fixture.b, "b-mixed-admission", &runtime)? {
        CargoOriginAttempt::Produced(produced) => {
            let (receipt, mixed_admission) = *produced;
            origin_passed(&mixed_admission.process, "b-mixed-admission")?;
            mixed_admission.completed(true)?;
            let mixed_admitted_library = mixed_admission.artifact(&fixture.b, "lib")?;
            let mixed_admitted_binary = mixed_admission.artifact(&fixture.b, "bin")?;
            origin_require(
                mixed_admitted_library.fresh
                    && !mixed_admitted_binary.fresh
                    && origin_same_outputs(&cold_library, &mixed_admitted_library),
                "binary-only admission lost the foreign-library adversary",
            )?;
            fixture.behavior(
                "b-mixed-admission-behavior",
                &mixed_admitted_binary,
                &format!("B|17|{a}|{b}\n"),
            )?;
            origin_refused(&receipt, "b-mixed-admission")?;
        }
        CargoOriginAttempt::Preflight(message) => {
            eprintln!("b-mixed-admission precisely refused without a child: {message}");
        }
    }

    fixture.write_binary(&fixture.b, "B", true)?;
    fixture.advance(&fixture.b.join("src/lib.rs"))?;
    fixture.advance(&fixture.b.join("src/main.rs"))?;
    let partial = fixture.raw(&fixture.b, "b-partial-failed-raw")?;
    origin_require(
        partial.process.status == ProcessStatus::Failed,
        "deliberate Cargo failure did not fail",
    )?;
    partial.completed(false)?;
    let partial_library = partial.artifact(&fixture.b, "lib")?;
    origin_require(
        !partial_library.fresh && !origin_same_outputs(&cold_library, &partial_library),
        "failed build did not replace the project library",
    )?;
    origin_require(
        !partial.records.iter().any(|record| {
            record.get("reason").and_then(serde_json::Value::as_str) == Some("compiler-artifact")
                && record
                    .pointer("/target/kind")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")))
        }),
        "failed binary unexpectedly produced a compiler artifact",
    )?;
    origin_require(
        partial.records.iter().any(|record| {
            record.get("reason").and_then(serde_json::Value::as_str) == Some("compiler-message")
                && record
                    .pointer("/message/message")
                    .and_then(serde_json::Value::as_str)
                    == Some("owned-origin-fixture deliberate failure")
        }),
        "failed build did not retain the independent compile-error marker",
    )?;
    fixture.write_binary(&fixture.b, "B", false)?;
    fixture.backdate(&fixture.b.join("src/main.rs"))?;
    fixture.backdate(&fixture.b.join("src/lib.rs"))?;

    let poisoned = fixture.raw(&fixture.a, "a-after-partial-raw")?;
    origin_passed(&poisoned.process, "a-after-partial-raw")?;
    poisoned.completed(true)?;
    let poisoned_library = poisoned.artifact(&fixture.a, "lib")?;
    let poisoned_binary = poisoned.artifact(&fixture.a, "bin")?;
    origin_require(
        poisoned_library.fresh
            && !poisoned_binary.fresh
            && origin_same_outputs(&partial_library, &poisoned_library),
        "failed foreign producer did not leave the expected shared library",
    )?;
    fixture.behavior(
        "a-poisoned-behavior",
        &poisoned_binary,
        &format!("A|29|{b}|{a}\n"),
    )?;
    match fixture.negative_admitted(&fixture.a, "a-poisoned-admission", &runtime)? {
        CargoOriginAttempt::Produced(produced) => {
            let (receipt, poisoned_admission) = *produced;
            origin_passed(&poisoned_admission.process, "a-poisoned-admission")?;
            poisoned_admission.completed(true)?;
            let poisoned_admitted_library = poisoned_admission.artifact(&fixture.a, "lib")?;
            let poisoned_admitted_binary = poisoned_admission.artifact(&fixture.a, "bin")?;
            origin_require(
                poisoned_admitted_library.fresh
                    && poisoned_admitted_binary.fresh
                    && origin_same_outputs(&partial_library, &poisoned_admitted_library),
                "partial-failure admission lost the overwritten-library adversary",
            )?;
            fixture.behavior(
                "a-poisoned-admission-behavior",
                &poisoned_admitted_binary,
                &format!("A|29|{b}|{a}\n"),
            )?;
            origin_refused(&receipt, "a-poisoned-admission")?;
        }
        CargoOriginAttempt::Preflight(message) => {
            eprintln!("a-poisoned-admission precisely refused without a child: {message}");
        }
    }

    fixture.clean()?;
    let (recovered_receipt, recovered) = fixture.admitted(&fixture.a, "a-recovered", &runtime)?;
    origin_positive(&recovered_receipt, "a-recovered")?;
    origin_passed(&recovered.process, "a-recovered")?;
    recovered.completed(true)?;
    let recovered_library = recovered.artifact(&fixture.a, "lib")?;
    let recovered_binary = recovered.artifact(&fixture.a, "bin")?;
    origin_require(
        !recovered_library.fresh && !recovered_binary.fresh,
        "recovery reused project artifacts",
    )?;
    fixture.behavior("a-recovered-behavior", &recovered_binary, &healthy_a)?;
    let (reused_receipt, reused) = fixture.admitted(&fixture.a, "a-recovered-warm", &runtime)?;
    origin_positive(&reused_receipt, "a-recovered-warm")?;
    origin_cold_closure(&fixture.a, &reused_receipt, &recovered_receipt)?;
    origin_passed(&reused.process, "a-recovered-warm")?;
    reused.completed(true)?;
    let reused_library = reused.artifact(&fixture.a, "lib")?;
    let reused_binary = reused.artifact(&fixture.a, "bin")?;
    origin_require(
        reused_library.fresh
            && reused_binary.fresh
            && origin_same_outputs(&recovered_library, &reused_library)
            && origin_same_outputs(&recovered_binary, &reused_binary),
        "recovered warm reuse changed outputs",
    )?;
    fixture.behavior("a-recovered-warm-behavior", &reused_binary, &healthy_a)?;

    let final_foreign = fixture.raw(&fixture.b, "b-final-foreign-raw")?;
    origin_passed(&final_foreign.process, "b-final-foreign-raw")?;
    final_foreign.completed(true)?;
    let final_foreign_library = final_foreign.artifact(&fixture.b, "lib")?;
    let final_foreign_binary = final_foreign.artifact(&fixture.b, "bin")?;
    origin_require(
        final_foreign_library.fresh
            && final_foreign_binary.fresh
            && origin_same_outputs(&recovered_library, &final_foreign_library)
            && origin_same_outputs(&recovered_binary, &final_foreign_binary),
        "recovered target did not reproduce the final foreign-cache adversary",
    )?;
    fixture.behavior(
        "b-final-foreign-behavior",
        &final_foreign_binary,
        &healthy_a,
    )?;

    // Cargo success/current labels must not turn A bytes into B source proof.
    match fixture.negative_admitted(&fixture.b, "b-origin-admission", &runtime)? {
        CargoOriginAttempt::Produced(produced) => {
            let (receipt, rejected) = *produced;
            origin_passed(&rejected.process, "b-origin-admission")?;
            rejected.completed(true)?;
            let rejected_library = rejected.artifact(&fixture.b, "lib")?;
            let rejected_binary = rejected.artifact(&fixture.b, "bin")?;
            origin_require(
                rejected_library.fresh
                    && rejected_binary.fresh
                    && origin_same_outputs(&recovered_library, &rejected_library)
                    && origin_same_outputs(&recovered_binary, &rejected_binary),
                "final raw candidate no longer carries the foreign-cache adversary",
            )?;
            fixture.behavior("b-rejected-behavior", &rejected_binary, &healthy_a)?;
            origin_refused(&receipt, "b-origin-admission")?;
        }
        CargoOriginAttempt::Preflight(message) => {
            eprintln!("b-origin-admission precisely refused without a child: {message}");
        }
    }
    fixture.finish()
}
