#[cfg(test)]
#[path = "cargo_origin_tests.rs"]
pub(super) mod tests;

pub(super) use super::cargo_origin_context::Context;
use super::cargo_origin_context::TargetLease;
use super::cargo_origin_units::{self as units, Unit};
use super::model::{DEFAULT_MAXIMUM_STREAM_BYTES, Gate, InputSnapshot, RuntimeIdentity};
use super::snapshot;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof};
use crate::process::{ProcessObservation, ProcessStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

const CONTRACT_VERSION: u32 = 1;
const MAXIMUM_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_PROOF_UNITS: usize = 256;
const MAXIMUM_COMMAND_ARGUMENTS: usize = 1_024;
const MAXIMUM_OUTPUTS: usize = 256;
const MAXIMUM_TEXT_BYTES: usize = 8 * 1024;

/// Explicit archives preserve exactly this producer's observed outputs. Cargo's
/// `fresh` flag remains an observation, never a fabricated cold-build claim.
/// No mutable ledger or prior archive is needed to admit a new successful run.
pub(super) struct Run {
    repository: PathBuf,
    context: Context,
    snapshot: InputSnapshot,
    runtime: RuntimeIdentity,
    commands: Vec<(String, String)>,
    state: Mutex<()>,
    _lease: TargetLease,
}

pub(super) struct Transaction<'a> {
    run: &'a Run,
    _guard: MutexGuard<'a, ()>,
    producer: Producer,
    finished: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Producer {
    name: String,
    command: Vec<String>,
    identity_command: Vec<String>,
    environment: BTreeMap<String, String>,
    timeout_nanoseconds: u64,
    maximum_stdout_bytes: u64,
    maximum_stderr_bytes: u64,
    required_outputs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    contract_version: u32,
    context: Context,
    producer: Producer,
    process: ProcessObservation,
    units: Vec<Unit>,
}

impl Run {
    #[cfg(test)]
    pub(super) fn with_commands(
        repository: &Path,
        target: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
        commands: Vec<(String, String)>,
    ) -> Result<Self, DevError> {
        Self::construct(repository, target, snapshot, runtime, commands)
    }

    pub(super) fn with_commands_for_run(
        repository: &Path,
        target: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
        commands: Vec<(String, String)>,
        _run_directory: &Path,
    ) -> Result<Self, DevError> {
        Self::construct(repository, target, snapshot, runtime, commands)
    }

    fn construct(
        repository: &Path,
        target: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
        commands: Vec<(String, String)>,
    ) -> Result<Self, DevError> {
        let context = Context::capture_at(repository, target, snapshot, runtime)?;
        let lease = TargetLease::acquire(&context)?;
        let run = Self {
            repository: repository.to_path_buf(),
            context,
            snapshot: snapshot.clone(),
            runtime: runtime.clone(),
            commands,
            state: Mutex::new(()),
            _lease: lease,
        };
        run.recheck()?;
        Ok(run)
    }

    pub(super) fn begin(&self, gate: &Gate) -> Result<Transaction<'_>, DevError> {
        if !is_writer(gate) {
            return Err(refusal("Cargo transaction requires a writer"));
        }
        let guard = self
            .state
            .lock()
            .map_err(|_| refusal("Cargo writer state was poisoned"))?;
        supported_command(gate, &self.context)?;
        self.recheck()?;
        Ok(Transaction {
            run: self,
            _guard: guard,
            producer: Producer::capture(gate)?,
            finished: false,
        })
    }

    fn recheck(&self) -> Result<(), DevError> {
        let current_snapshot = snapshot::capture(&self.repository)?;
        let current_runtime = snapshot::runtime_identity(&self.repository, self.commands.clone())?;
        if current_snapshot.digest != self.snapshot.digest
            || current_runtime.digest != self.runtime.digest
        {
            return Err(refusal("source or runtime changed during Cargo ownership"));
        }
        self.context
            .recheck(&self.repository, &current_snapshot, &current_runtime)
    }
}

impl Transaction<'_> {
    pub(super) fn finish(
        &mut self,
        gate: &Gate,
        process: &ProcessObservation,
        run_directory: &Path,
    ) -> Result<Option<FileProof>, DevError> {
        self.ensure_unfinished()?;
        self.finished = true;
        if self.producer != Producer::capture(gate)? {
            return Err(refusal(
                "Cargo transaction finished with a different producer",
            ));
        }
        successful_process(gate, process)?;
        self.run.recheck()?;
        if !is_producer(gate) {
            return Ok(None);
        }
        process_paths(&self.run.repository, run_directory, gate, process)?;
        let stdout = proof_path(&self.run.repository, &process.stdout)?;
        super::cargo_outputs::verify(&self.run.repository, gate, &stdout, &process.stdout)?;
        let mut units = units::read_units(
            &self.run.repository,
            Path::new(&self.run.context.target),
            &self.run.context.workspace_manifests,
            gate,
            &stdout,
            &process.stdout,
        )?;
        units::retain_units(&self.run.repository, run_directory, gate, &mut units)?;
        self.run.recheck()?;
        let archive = Proof {
            contract_version: CONTRACT_VERSION,
            context: self.run.context.clone(),
            producer: self.producer.clone(),
            process: process.clone(),
            units,
        };
        let path = run_directory.join(format!("{}.cargo-origin.json", gate.name));
        if path.try_exists()? {
            return Err(refusal("refusing to replace original Cargo evidence"));
        }
        evidence::publish(&path, &metadata_bytes(&archive)?)?;
        let proof = evidence::proof(&path, evidence::relative(&self.run.repository, &path))?;
        verify(
            &self.run.repository,
            gate,
            process,
            &proof,
            &self.run.context,
        )?;
        Ok(Some(proof))
    }

    pub(super) fn finish_unchanged(&mut self) -> Result<(), DevError> {
        self.ensure_unfinished()?;
        self.finished = true;
        self.run.recheck()
    }

    fn ensure_unfinished(&self) -> Result<(), DevError> {
        if self.finished {
            return Err(refusal("Cargo transaction was already finished"));
        }
        Ok(())
    }
}

pub(super) fn verify(
    repository: &Path,
    gate: &Gate,
    process: &ProcessObservation,
    proof: &FileProof,
    expected_context: &Context,
) -> Result<(), DevError> {
    if !is_producer(gate) {
        return Err(refusal("Cargo evidence belongs to a nonproducer"));
    }
    regular_proof(proof, MAXIMUM_METADATA_BYTES)?;
    let path = proof_path(repository, proof)?;
    verify_file(&path, proof)?;
    let bytes = super::cache::read_bounded(&path, MAXIMUM_METADATA_BYTES)?;
    let archive: Proof = serde_json::from_slice(&bytes)?;
    if archive.contract_version != CONTRACT_VERSION
        || archive.context != *expected_context
        || archive.producer != Producer::capture(gate)?
        || serde_json::to_vec(&archive.process)? != serde_json::to_vec(process)?
        || archive.units.is_empty()
        || archive.units.len() > MAXIMUM_PROOF_UNITS
        || fs::canonicalize(repository)? != Path::new(&archive.context.repository)
    {
        return Err(refusal(
            "Cargo archive has a different source, producer or output context",
        ));
    }
    supported_command(gate, expected_context)?;
    successful_process(gate, process)?;
    let root = path
        .parent()
        .ok_or_else(|| refusal("Cargo archive has no run directory"))?;
    if path.file_name().and_then(|value| value.to_str())
        != Some(format!("{}.cargo-origin.json", gate.name).as_str())
    {
        return Err(refusal("Cargo archive has a different producer path"));
    }
    process_paths(repository, root, gate, process)?;
    let stdout = proof_path(repository, &process.stdout)?;
    verify_file(&stdout, &process.stdout)?;
    verify_file(&proof_path(repository, &process.stderr)?, &process.stderr)?;
    super::cargo_outputs::verify(repository, gate, &stdout, &process.stdout)?;
    for (index, unit) in archive.units.iter().enumerate() {
        retained_paths(repository, root, gate, unit, index)?;
    }
    units::verify_records(
        repository,
        Path::new(&expected_context.target),
        &expected_context.workspace_manifests,
        gate,
        &stdout,
        &process.stdout,
        &archive.units,
    )
}

pub(super) fn is_writer(gate: &Gate) -> bool {
    gate.command.first().map(String::as_str) == Some("cargo")
        && gate.command.get(1).map(String::as_str) != Some("fmt")
}

pub(super) fn is_producer(gate: &Gate) -> bool {
    is_writer(gate)
        && !gate.required_outputs.is_empty()
        && matches!(
            gate.command.get(1).map(String::as_str),
            Some("build" | "test")
        )
}

pub(super) fn bind_target(gate: &mut Gate, target: &Path) -> Result<(), DevError> {
    if !is_writer(gate) {
        return Ok(());
    }
    if !target.is_absolute() {
        return Err(refusal(
            "Cargo target binding requires a canonical absolute target",
        ));
    }
    let target_text = path_string(target)?;
    let parent = target
        .parent()
        .ok_or_else(|| refusal("Cargo target binding has no parent"))?;
    if normalized_target(parent, &target_text)? != target {
        return Err(refusal("Cargo target binding is routed or noncanonical"));
    }
    if let Some(selected) = command_target(gate)? {
        // Registry commands use absolute target labels. begin can also admit
        // relative labels against its actual checkout, without using shell cwd.
        if !Path::new(selected).is_absolute() || normalized_target(parent, selected)? != target {
            return Err(refusal(
                "existing Cargo target argument differs from its binding",
            ));
        }
        return Ok(());
    }
    let index = match gate.command.iter().position(|argument| argument == "--") {
        Some(index) => index,
        None => gate.command.len(),
    };
    gate.command
        .insert(index, format!("--target-dir={target_text}"));
    Ok(())
}

pub(super) fn successful_process(
    gate: &Gate,
    process: &ProcessObservation,
) -> Result<(), DevError> {
    if process.status != ProcessStatus::Passed
        || process.exit_code != Some(0)
        || process.signal.is_some()
        || process.reason.is_some()
        || process.stdout_limit_exhausted
        || process.stderr_limit_exhausted
        || process.stdout_limit_bytes != gate.maximum_stdout_bytes
        || process.stderr_limit_bytes != gate.maximum_stderr_bytes
    {
        return Err(refusal(
            "Cargo origin requires a complete successful child observation",
        ));
    }
    regular_proof(&process.stdout, gate.maximum_stdout_bytes)?;
    regular_proof(&process.stderr, gate.maximum_stderr_bytes)
}

fn process_paths(
    repository: &Path,
    run_directory: &Path,
    gate: &Gate,
    process: &ProcessObservation,
) -> Result<(), DevError> {
    component(&gate.name)?;
    for (stream, proof) in [("stdout", &process.stdout), ("stderr", &process.stderr)] {
        let expected = run_directory.join(format!("{}.{}.log", gate.name, stream));
        if proof_path(repository, proof)? != expected {
            return Err(refusal(
                "Cargo producer log is outside its original retained run",
            ));
        }
    }
    Ok(())
}

fn retained_paths(
    repository: &Path,
    run_directory: &Path,
    gate: &Gate,
    unit: &Unit,
    unit_index: usize,
) -> Result<(), DevError> {
    for (index, proof) in unit.retained.iter().enumerate() {
        let expected = run_directory
            .join("retained-cargo-origin")
            .join(&gate.name)
            .join(unit_index.to_string())
            .join(index.to_string());
        if proof_path(repository, proof)? != expected {
            return Err(refusal(
                "archived workspace artifact is outside its original retained run",
            ));
        }
    }
    Ok(())
}

fn metadata_bytes(value: &impl Serialize) -> Result<Vec<u8>, DevError> {
    let bytes = evidence::encode_json(value)?;
    if bytes.len() as u64 > MAXIMUM_METADATA_BYTES {
        return Err(refusal("Cargo origin metadata exceeds its byte limit"));
    }
    Ok(bytes)
}

fn proof_path(repository: &Path, proof: &FileProof) -> Result<PathBuf, DevError> {
    text(&proof.path)?;
    let path = Path::new(&proof.path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(refusal(
            "Cargo retained evidence path is not repository-relative",
        ));
    }
    let resolved = repository.join(path);
    // Parent symlinks could otherwise make a lexical retained label point at a
    // different run. Each retained file and its directory chain stay canonical.
    let parent = resolved
        .parent()
        .ok_or_else(|| refusal("Cargo retained evidence path has no parent"))?;
    if fs::canonicalize(parent)? != parent {
        return Err(refusal(
            "Cargo retained evidence has a routed parent directory",
        ));
    }
    Ok(resolved)
}

fn regular_proof(proof: &FileProof, maximum: u64) -> Result<(), DevError> {
    text(&proof.path)?;
    if proof.kind != FileKind::File
        || proof.link_target.is_some()
        || proof.digest.is_none()
        || !proof.bytes.is_some_and(|bytes| bytes <= maximum)
    {
        return Err(refusal(
            "Cargo origin evidence has an unsafe or oversized file proof",
        ));
    }
    Ok(())
}

fn verify_file(path: &Path, proof: &FileProof) -> Result<(), DevError> {
    if proof.kind != FileKind::File
        || proof.digest.is_none()
        || proof.bytes.is_none()
        || proof.link_target.is_some()
        || fs::canonicalize(path)? != path
        || evidence::proof(path, proof.path.clone())? != *proof
    {
        return Err(refusal("Cargo origin file changed or has unsafe routing"));
    }
    Ok(())
}

fn command(arguments: &[String]) -> Result<(), DevError> {
    if arguments.is_empty() || arguments.len() > MAXIMUM_COMMAND_ARGUMENTS {
        return Err(refusal("Cargo producer command exceeds its argument limit"));
    }
    for argument in arguments {
        text(argument)?;
    }
    Ok(())
}

pub(super) fn supported_command(gate: &Gate, context: &Context) -> Result<(), DevError> {
    // Cargo config and manifest overrides need their own effective context
    // binding. The physical target is explicitly admitted on every writer.
    if gate
        .command
        .iter()
        .take_while(|argument| *argument != "--")
        .any(|argument| {
            ["--config", "--manifest-path"]
                .iter()
                .any(|option| argument == option || argument.starts_with(&format!("{option}=")))
        })
    {
        return Err(refusal("Cargo command has an unsupported context override"));
    }
    let selected = command_target(gate)?
        .ok_or_else(|| refusal("Cargo writer has no explicit physical target argument"))?;
    if normalized_target(Path::new(&context.repository), selected)? != Path::new(&context.target) {
        return Err(refusal(
            "Cargo writer target differs from its owned physical target",
        ));
    }
    supported_build_target(gate)?;
    Ok(())
}

fn supported_build_target(gate: &Gate) -> Result<(), DevError> {
    let mut selected = false;
    let mut arguments = gate.command.iter().take_while(|argument| *argument != "--");
    while let Some(argument) = arguments.next() {
        let value = if argument == "--target" {
            Some(
                arguments
                    .next()
                    .ok_or_else(|| refusal("Cargo build target argument is missing"))?
                    .as_str(),
            )
        } else {
            argument.strip_prefix("--target=")
        };
        if let Some(value) = value {
            if selected
                || !matches!(
                    value,
                    "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl"
                )
            {
                return Err(refusal(
                    "Cargo custom or ambiguous build targets are unsupported",
                ));
            }
            selected = true;
        }
    }
    Ok(())
}

fn command_target(gate: &Gate) -> Result<Option<&str>, DevError> {
    let mut selected = None;
    let mut arguments = gate.command.iter().take_while(|argument| *argument != "--");
    while let Some(argument) = arguments.next() {
        let value = if argument == "--target-dir" {
            Some(
                arguments
                    .next()
                    .ok_or_else(|| refusal("Cargo target argument is missing"))?
                    .as_str(),
            )
        } else {
            argument.strip_prefix("--target-dir=")
        };
        if let Some(value) = value {
            text(value)?;
            if value.starts_with('-') || selected.replace(value).is_some() {
                return Err(refusal(
                    "Cargo writer has ambiguous physical target arguments",
                ));
            }
        }
    }
    Ok(selected)
}

fn normalized_target(repository: &Path, value: &str) -> Result<PathBuf, DevError> {
    text(value)?;
    let authored = Path::new(value);
    let absolute = if authored.is_absolute() {
        authored.to_path_buf()
    } else {
        repository.join(authored)
    };
    let mut resolved = PathBuf::new();
    for (index, part) in absolute.components().enumerate() {
        if index >= 64 {
            return Err(refusal(
                "Cargo target argument exceeds its path depth limit",
            ));
        }
        match part {
            Component::RootDir => resolved.push(part.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(refusal("Cargo target argument escapes its filesystem root"));
                }
            }
            Component::Normal(_) => {
                resolved.push(part.as_os_str());
                match fs::symlink_metadata(&resolved) {
                    Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                        return Err(refusal(
                            "Cargo target argument has unsafe directory routing",
                        ));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Component::Prefix(_) => {
                return Err(refusal("Cargo target argument has an unsupported prefix"));
            }
        }
    }
    if !resolved.is_absolute() {
        return Err(refusal(
            "Cargo target argument is not absolute after resolution",
        ));
    }
    Ok(resolved)
}

fn component(value: &str) -> Result<(), DevError> {
    text(value)?;
    if value == "."
        || value == ".."
        || value
            .chars()
            .any(|character| !character.is_ascii_alphanumeric() && !matches!(character, '_' | '-'))
    {
        return Err(refusal("Cargo origin gate name is not a safe component"));
    }
    Ok(())
}

fn text(value: &str) -> Result<(), DevError> {
    if value.is_empty() || value.len() > MAXIMUM_TEXT_BYTES || value.contains('\0') {
        return Err(refusal("Cargo origin text exceeds its representation"));
    }
    Ok(())
}

fn path_string(path: &Path) -> Result<String, DevError> {
    let value = path
        .to_str()
        .ok_or_else(|| refusal("Cargo origin path is not UTF-8"))?;
    text(value)?;
    Ok(value.to_owned())
}

fn refusal(message: impl Into<String>) -> DevError {
    DevError::corrupt(format!("Cargo compilation origin: {}", message.into()))
}

impl Producer {
    fn capture(gate: &Gate) -> Result<Self, DevError> {
        component(&gate.name)?;
        command(&gate.command)?;
        command(gate.identity_command())?;
        for (name, value) in &gate.environment {
            if name != "LKJSCRIPT_RELEASE_CANDIDATE" {
                return Err(refusal(
                    "Cargo producer has an unsupported environment addition",
                ));
            }
            text(value)?;
        }
        if gate.required_outputs.len() > MAXIMUM_OUTPUTS {
            return Err(refusal("Cargo producer output inventory exceeds its limit"));
        }
        if gate.maximum_stdout_bytes > DEFAULT_MAXIMUM_STREAM_BYTES
            || gate.maximum_stderr_bytes > DEFAULT_MAXIMUM_STREAM_BYTES
        {
            return Err(refusal(
                "Cargo producer exceeds the maintained stream limits",
            ));
        }
        let timeout_nanoseconds = u64::try_from(gate.timeout.as_nanos())
            .map_err(|_| refusal("Cargo producer timeout exceeds its representation"))?;
        let required_outputs = gate
            .required_outputs
            .iter()
            .map(|path| path_string(path))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            name: gate.name.clone(),
            command: gate.command.clone(),
            identity_command: gate.identity_command().to_vec(),
            environment: gate.environment.clone(),
            timeout_nanoseconds,
            maximum_stdout_bytes: gate.maximum_stdout_bytes,
            maximum_stderr_bytes: gate.maximum_stderr_bytes,
            required_outputs,
        })
    }
}
