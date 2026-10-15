//! A closed final-image boundary manifest, authenticated by the original source owner.
use super::*;
use serde_json::{Value, json};

#[path = "boundary_author.rs"]
mod author;
#[path = "boundary_effects.rs"]
mod effects;
#[path = "boundary_service.rs"]
mod service;

const FIXTURES: &[&str] = &[
    "foreground-1-original.lkja",
    "foreground-1.deployment.json",
    "requirement-consumer.lkja",
    "standalone.lkja",
    "standalone.deployment.json",
    "nominal-session.lkja",
    "nominal-session.deployment.json",
];
pub(super) const MANIFEST: &[&str] = &[
    "capabilities",
    "builtin-inspect",
    "producer-new",
    "producer-plan",
    "producer-apply",
    "producer-export",
    "consumer-new",
    "consumer-stage",
    "consumer-plan",
    "consumer-apply",
    "consumer-export",
    "build-cold",
    "build-warm",
    "build-missing-cache",
    "build-corrupt-cache",
    "check",
    "project-run",
    "source-batch",
    "bad-transport",
    "canonical-source-refusal",
    "bad-artifact",
    "detached-result",
    "existing-result",
    "foreground-data-initialize",
    "conflict-data-initialize",
    "conflict-left",
    "conflict-right",
    "sigint-after-commit",
    "sigterm-after-commit",
    "closed-stdout-after-commit",
    "http-start-stop",
    "websocket-start-stop",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundaryReceipt {
    pub schema: String,
    pub purpose: String,
    pub status: String,
    pub candidate_sha256: String,
    pub verifier_sha256: String,
    source_receipt: evidence::FileProof,
    source_files: Vec<evidence::FileProof>,
    source_root: String,
    evidence_root: String,
    isolated_root: String,
    executable: String,
    pub elapsed_nanoseconds: u64,
    processes: Vec<BoundaryProcess>,
    assertions: BTreeMap<String, Value>,
    files: Vec<evidence::FileProof>,
    pub cleanup_complete: bool,
    failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryProcess {
    name: String,
    command: Vec<String>,
    cwd: String,
    expects_success: bool,
    observation: process::ProcessObservation,
}

impl BoundaryReceipt {
    pub(crate) fn source_receipt_proof(&self) -> &evidence::FileProof {
        &self.source_receipt
    }
    pub(crate) fn command_count(&self) -> usize {
        self.processes.len() - self.runner_count()
    }
    pub(crate) fn runner_count(&self) -> usize {
        self.processes
            .iter()
            .filter(|call| call.command.get(1).is_some_and(|arg| arg == "serve"))
            .count()
    }
    pub(crate) fn request_count(&self) -> usize {
        2
    }
}

struct BoundaryContext {
    root: PathBuf,
    evidence: PathBuf,
    binary: PathBuf,
    receipt: BoundaryReceipt,
}

impl BoundaryContext {
    fn specification(&self, name: &str, cwd: &Path, args: &[String]) -> process::ProcessSpec {
        process::ProcessSpec {
            command: std::iter::once(self.binary.display().to_string())
                .chain(args.iter().cloned())
                .collect(),
            cwd: cwd.to_path_buf(),
            environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
            timeout: Duration::from_secs(120),
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: self.evidence.join(format!("{name}.stdout")),
            stderr_path: self.evidence.join(format!("{name}.stderr")),
            unavailable_exit_code: None,
        }
    }
    fn retain(
        &mut self,
        name: &str,
        spec: &process::ProcessSpec,
        passes: bool,
        observation: process::ProcessObservation,
    ) -> Result<Vec<CompactRecord>, DevError> {
        require(
            self.receipt.processes.len() < MANIFEST.len()
                && MANIFEST[self.receipt.processes.len()] == name,
            "final boundary invocation does not match its closed ordered manifest",
        )?;
        CallObservation::Process(observation.clone()).require_outcome(passes)?;
        let records = if name.ends_with("start-stop") || name == "closed-stdout-after-commit" {
            Vec::new()
        } else {
            parse_records(
                name,
                &process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT_BYTES)?,
            )
            .map_err(|error| {
                DevError::corrupt(format!("final boundary public response: {error:?}"))
            })?
        };
        self.receipt.processes.push(BoundaryProcess {
            name: name.into(),
            command: spec.command.clone(),
            cwd: spec.cwd.display().to_string(),
            expects_success: passes,
            observation,
        });
        Ok(records)
    }
    fn cli(
        &mut self,
        name: &str,
        project: Option<&Path>,
        args: &[&str],
        passes: bool,
    ) -> Result<Vec<CompactRecord>, DevError> {
        let mut arguments = Vec::new();
        if let Some(project) = project {
            arguments.extend(["--project".into(), project.display().to_string()]);
        }
        arguments.extend(args.iter().map(|arg| (*arg).to_owned()));
        let spec = self.specification(name, &self.root, &arguments);
        let observation = process::run(&spec, &self.evidence);
        self.retain(name, &spec, passes, observation)
    }
    fn fixture(&self, name: &str) -> PathBuf {
        self.evidence.join(format!("fixture-{name}"))
    }
}

pub(super) fn command(
    binary: PathBuf,
    source: PathBuf,
    output: Option<PathBuf>,
    machine: bool,
) -> Result<u8, DevError> {
    let verifier = std::env::current_exe()?.canonicalize()?;
    let source = source.canonicalize()?;
    let admitted = read_semantic_receipt(&source, &verifier)?;
    let source_root = source
        .parent()
        .ok_or_else(|| DevError::corrupt("source receipt parent missing"))?;
    let original_receipt = evidence::proof(&source, source.display().to_string())?;
    let original_files = FIXTURES
        .iter()
        .map(|name| {
            let proof = evidence::proof(&source_root.join(name), (*name).into())?;
            require(
                admitted.files.iter().filter(|file| **file == proof).count() == 1,
                "final boundary fixture is not retained by the admitted original source owner",
            )?;
            Ok(proof)
        })
        .collect::<Result<Vec<_>, DevError>>()?;
    let binary = binary.canonicalize()?;
    let candidate_sha256 = digest_file(&binary, MAXIMUM_EXECUTABLE_BYTES)?;
    let output = match output {
        Some(path) => create_evidence_root(&path)?,
        None => {
            let parent = std::env::current_dir()?.join(".artifacts/offline-boundary");
            fs::create_dir_all(&parent)?;
            tempfile::Builder::new()
                .prefix("run-")
                .tempdir_in(parent.canonicalize()?)?
                .keep()
        }
    };
    let executable = output.join("candidate");
    fs::copy(&binary, &executable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o555))?;
    }
    require(
        digest_file(&executable, MAXIMUM_EXECUTABLE_BYTES)? == candidate_sha256,
        "final image copy changed bytes",
    )?;
    for name in FIXTURES {
        fs::copy(
            source_root.join(name),
            output.join(format!("fixture-{name}")),
        )?;
    }
    let isolated = tempfile::Builder::new()
        .prefix("lkjscript-offline-boundary-")
        .tempdir()?;
    let started = Instant::now();
    let mut context = BoundaryContext {
        root: isolated.path().to_path_buf(),
        evidence: output.clone(),
        binary: executable.clone(),
        receipt: BoundaryReceipt {
            schema: "lkjscript-offline-boundary-1".into(),
            purpose: "candidate_acceptance_supplement".into(),
            status: "failed".into(),
            candidate_sha256,
            verifier_sha256: digest_file(&verifier, MAXIMUM_EXECUTABLE_BYTES)?,
            source_receipt: original_receipt,
            source_files: original_files,
            source_root: source_root.display().to_string(),
            evidence_root: output.display().to_string(),
            isolated_root: isolated.path().display().to_string(),
            executable: executable.display().to_string(),
            elapsed_nanoseconds: 0,
            processes: Vec::new(),
            assertions: BTreeMap::new(),
            files: Vec::new(),
            cleanup_complete: false,
            failure: None,
        },
    };
    let outcome = author::workflow(&mut context)
        .and_then(|()| effects::workflow(&mut context))
        .and_then(|()| service::workflow(&mut context));
    context.receipt.elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| DevError::infrastructure("boundary elapsed overflow"))?;
    isolated.close()?;
    context.receipt.cleanup_complete = !context.root.exists();
    let outcome = outcome.and_then(|()| {
        require(
            evidence::proof(&source, source.display().to_string())?
                == context.receipt.source_receipt
                && digest_file(&binary, MAXIMUM_EXECUTABLE_BYTES)?
                    == context.receipt.candidate_sha256
                && digest_file(&executable, MAXIMUM_EXECUTABLE_BYTES)?
                    == context.receipt.candidate_sha256,
            "source receipt or exact final image changed during boundary observations",
        )
    });
    match outcome {
        Ok(()) => context.receipt.status = "fresh passed".into(),
        Err(error) => context.receipt.failure = Some(error.to_string()),
    }
    let mut entries = fs::read_dir(&output)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    context.receipt.files = entries
        .iter()
        .map(|entry| {
            evidence::proof(
                &entry.path(),
                entry.file_name().to_string_lossy().to_string(),
            )
        })
        .collect::<Result<_, _>>()?;
    let published = evidence::publish_json(&output.join("receipt.json"), &context.receipt)?;
    let success = context.receipt.status == "fresh passed";
    if success {
        read_transferred_receipt(&published.path, &binary, &verifier)?;
    }
    if machine {
        println!(
            "{}",
            json!({"status":if success {"passed"} else {"failed"},"receipt":published.path,"digest":published.digest,"failure":context.receipt.failure})
        );
    } else {
        println!(
            "offline boundary: {} ({})",
            context.receipt.status,
            published.path.display()
        );
    }
    Ok(if success { 0 } else { 1 })
}

pub(crate) fn read_transferred_receipt(
    path: &Path,
    candidate: &Path,
    verifier: &Path,
) -> Result<BoundaryReceipt, DevError> {
    let bytes = process::read_bounded(path, 64 * 1024 * 1024)?;
    let receipt: BoundaryReceipt = serde_json::from_slice(&bytes)?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("boundary receipt parent missing"))?
        .canonicalize()?;
    require(
        evidence::encode_json(&receipt)? == bytes
            && path.canonicalize()? == root.join("receipt.json")
            && receipt.schema == "lkjscript-offline-boundary-1"
            && receipt.purpose == "candidate_acceptance_supplement"
            && receipt.status == "fresh passed"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && !Path::new(&receipt.isolated_root).exists()
            && receipt.evidence_root == root.display().to_string()
            && receipt.executable == root.join("candidate").display().to_string()
            && receipt.candidate_sha256 == digest_file(candidate, MAXIMUM_EXECUTABLE_BYTES)?
            && receipt.candidate_sha256
                == digest_file(&root.join("candidate"), MAXIMUM_EXECUTABLE_BYTES)?
            && receipt.verifier_sha256 == digest_file(verifier, MAXIMUM_EXECUTABLE_BYTES)?
            && receipt.elapsed_nanoseconds > 0
            && receipt.processes.len() == MANIFEST.len(),
        "final boundary receipt header, image or exact manifest changed",
    )?;
    let source_path = Path::new(&receipt.source_receipt.path);
    require(
        source_path.is_absolute()
            && source_path.parent() == Some(Path::new(&receipt.source_root))
            && evidence::proof(source_path, receipt.source_receipt.path.clone())?
                == receipt.source_receipt,
        "final boundary original source receipt changed",
    )?;
    let source = read_semantic_receipt(source_path, verifier)?;
    require(
        receipt.source_files.len() == FIXTURES.len(),
        "final boundary source fixture inventory incomplete",
    )?;
    for (proof, name) in receipt.source_files.iter().zip(FIXTURES) {
        require(
            proof.path == *name
                && source.files.iter().filter(|file| *file == proof).count() == 1
                && evidence::proof(&Path::new(&receipt.source_root).join(name), (*name).into())?
                    == *proof
                && evidence::proof(&root.join(format!("fixture-{name}")), (*name).into())?
                    == *proof,
            "final boundary artifact differs from the exact original source fixture",
        )?;
    }
    let mut names = std::collections::BTreeSet::new();
    for proof in &receipt.files {
        require(
            Path::new(&proof.path).components().count() == 1
                && names.insert(&proof.path)
                && proof.kind == evidence::FileKind::File
                && evidence::proof(&root.join(&proof.path), proof.path.clone())? == *proof,
            "final boundary retained file omitted or substituted",
        )?;
    }
    let actual = fs::read_dir(&root)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().to_string()))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    require(
        actual
            == names
                .into_iter()
                .cloned()
                .chain(std::iter::once("receipt.json".into()))
                .collect(),
        "final boundary has unbound evidence files",
    )?;
    for (call, expected) in receipt.processes.iter().zip(MANIFEST) {
        require(
            call.name == *expected
                && call.command.first() == Some(&receipt.executable)
                && Path::new(&call.cwd).starts_with(&receipt.isolated_root),
            "final boundary command attribution changed",
        )?;
        let observation = CallObservation::Process(call.observation.clone());
        observation.require_outcome(call.expects_success)?;
        observation.verify_files(&receipt.files, expected)?;
        require(
            call.observation.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
                && call.observation.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES
                && !call.observation.stdout_limit_exhausted
                && !call.observation.stderr_limit_exhausted,
            "final boundary output exceeded its owned bounded capture",
        )?;
    }
    author::validate(&receipt, &root)?;
    effects::validate(&receipt, &root)?;
    service::validate(&receipt, &root)?;
    Ok(receipt)
}

fn records(
    receipt: &BoundaryReceipt,
    root: &Path,
    name: &str,
    project: Option<&str>,
    args: &[String],
    passes: bool,
) -> Result<Vec<CompactRecord>, DevError> {
    let call = receipt
        .processes
        .iter()
        .find(|call| call.name == name)
        .ok_or_else(|| DevError::corrupt("final boundary command omitted"))?;
    let mut command = vec![receipt.executable.clone()];
    if let Some(project) = project {
        command.extend([
            "--project".into(),
            Path::new(&receipt.isolated_root)
                .join(project)
                .display()
                .to_string(),
        ]);
    }
    command.extend_from_slice(args);
    require(
        call.command == command && call.expects_success == passes,
        "final boundary exact command or result class substituted",
    )?;
    parse_records(
        name,
        &process::read_bounded(&root.join(format!("{name}.stdout")), MAXIMUM_OUTPUT_BYTES)?,
    )
    .map_err(|error| DevError::corrupt(format!("final retained response: {error:?}")))
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn closed_execution(
    records: &[CompactRecord],
    artifact: &str,
    target: &str,
) -> Result<(), DevError> {
    require(
        field(records, "result", "status")? == "success"
            && field(records, "result", "command")? == "run"
            && field(records, "execution", "artifact")? == artifact
            && field(records, "execution", "target")? == target
            && field(records, "execution", "execution-mode")? == "production"
            && field(records, "execution", "verification")? == "not-performed",
        "final detached execution changed exact fixture, target or execution owner",
    )?;
    let work: Value = strict_from_str(&field(records, "execution", "production-observation")?)?;
    let cleanup: Value = strict_from_str(&field(records, "execution", "cleanup")?)?;
    let executor: Value = strict_from_str(&field(records, "execution", "executor-observation")?)?;
    require(
        work["instructions"].as_u64().is_some_and(|count| count > 0)
            && cleanup["admission_stopped"] == true
            && cleanup["remaining_tasks"] == 0
            && cleanup["cleanup_failures"] == json!([])
            && executor["dispatch_open"] == false
            && executor["active_dispatches"] == 0
            && executor["remaining_workers"] == 0,
        "final detached execution omitted actual VM work or joined runtime/executor cleanup",
    )?;
    for key in [
        "live_call_frames_after",
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_transactions_after",
        "live_type_bindings_after",
    ] {
        require(
            work[key] == 0,
            "final detached execution retained VM custody",
        )?;
    }
    Ok(())
}
