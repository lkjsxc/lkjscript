#[cfg(test)]
#[path = "cargo_origin_tests.rs"]
mod tests;

pub(super) use super::cargo_origin_context::Context;
use super::cargo_origin_context::TargetLease;
use super::cargo_origin_units::{
    self as units, MAXIMUM_ORIGIN_ARTIFACT_BYTES, MAXIMUM_ORIGIN_INVENTORY_BYTES, Unit,
};
use super::model::{DEFAULT_MAXIMUM_STREAM_BYTES, Gate, InputSnapshot, RuntimeIdentity};
use super::snapshot;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof, VerificationDigest};
use crate::process::{ProcessObservation, ProcessStatus};
#[cfg(unix)]
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

const CONTRACT_VERSION: u32 = 2;
const MAXIMUM_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_CLOSURE_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_CLOSURE_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAXIMUM_CLOSURE_LOG_BYTES: u64 = 256 * 1024 * 1024;
const MAXIMUM_LEDGER_UNITS: usize = 1_024;
const MAXIMUM_PROOF_UNITS: usize = 256;
const MAXIMUM_ORIGINAL_PROOFS: usize = 256;
const MAXIMUM_CLOSURE_DEPTH: usize = 32;
const MAXIMUM_COMMAND_ARGUMENTS: usize = 1_024;
const MAXIMUM_OUTPUTS: usize = 256;
const MAXIMUM_TEXT_BYTES: usize = 8 * 1024;

// The lease coordinates maintained checkers, while the mutex coordinates their
// Cargo gates. Uncoordinated host writers remain outside this trusted-host model.
#[derive(Debug)]
pub(super) struct Run {
    repository: PathBuf,
    context: Context,
    snapshot: InputSnapshot,
    runtime: RuntimeIdentity,
    commands: Vec<(String, String)>,
    ledger_path: PathBuf,
    state: Mutex<Ledger>,
    _lease: TargetLease,
    current_run: Option<PathBuf>,
    _pins: BTreeMap<PathBuf, File>,
}

pub(super) struct Transaction<'a> {
    run: &'a Run,
    guard: Option<MutexGuard<'a, Ledger>>,
    owner: Option<Producer>,
    prior: Vec<Origin>,
    history: Vec<Origin>,
    initially_dirty: bool,
    pins: BTreeMap<PathBuf, File>,
    finished: bool,
    #[cfg(test)]
    commit_artifact_limit: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    contract_version: u32,
    context: Context,
    dirty: bool,
    origins: Vec<Origin>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Origin {
    unit: Unit,
    cold_proof: FileProof,
    identity_command: Vec<String>,
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
    units: Vec<ProvedUnit>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProvedUnit {
    unit: Unit,
    // Ordinary warm units point directly to their actual cold compilation.
    // The root schema generator instead retains an honest cached observation;
    // its generated input is independently compared with compiled source bytes.
    cold_proof: Option<FileProof>,
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
        Self::construct(repository, target, snapshot, runtime, commands, None)
    }

    pub(super) fn with_commands_for_run(
        repository: &Path,
        target: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
        commands: Vec<(String, String)>,
        run_directory: &Path,
    ) -> Result<Self, DevError> {
        Self::construct(
            repository,
            target,
            snapshot,
            runtime,
            commands,
            Some(run_directory),
        )
    }

    fn construct(
        repository: &Path,
        target: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
        commands: Vec<(String, String)>,
        current_run: Option<&Path>,
    ) -> Result<Self, DevError> {
        if commands.len() > MAXIMUM_COMMAND_ARGUMENTS {
            return Err(refusal("command executable inventory exceeds its limit"));
        }
        for (command, identity) in &commands {
            text(command)?;
            text(identity)?;
        }
        let context = Context::capture_at(repository, target, snapshot, runtime)?;
        let lease = TargetLease::acquire(&context)?;
        let ledger_path = context.ledger_path()?;
        let repository = PathBuf::from(&context.repository);
        let current_run = current_run
            .map(|run| owned_run(&repository, run))
            .transpose()?;
        let mut state = load_ledger(&ledger_path, &context)?;
        // Pin the retained producer closure before the caller may rotate runs.
        // The target lease alone does not exclude another target's collector.
        let (origins, pins) = admitted_origins(
            &repository,
            &context,
            &state.origins,
            current_run.as_deref(),
        );
        state.origins = origins;
        let run = Self {
            repository,
            context,
            snapshot: snapshot.clone(),
            runtime: runtime.clone(),
            commands,
            ledger_path,
            state: Mutex::new(state),
            _lease: lease,
            current_run,
            _pins: pins,
        };
        run.recheck()?;
        Ok(run)
    }

    #[cfg(test)]
    pub(super) fn context(&self) -> &Context {
        &self.context
    }

    pub(super) fn begin(&self, gate: &Gate) -> Result<Transaction<'_>, DevError> {
        if !is_writer(gate) {
            return Ok(Transaction {
                run: self,
                guard: None,
                owner: None,
                prior: Vec::new(),
                history: Vec::new(),
                initially_dirty: false,
                pins: BTreeMap::new(),
                finished: false,
                #[cfg(test)]
                commit_artifact_limit: MAXIMUM_CLOSURE_ARTIFACT_BYTES,
            });
        }
        let mut guard = self
            .state
            .lock()
            .map_err(|_| refusal("Cargo target writer state was poisoned"))?;
        let owner = Producer::capture(gate)?;
        supported_command(gate, &self.context)?;
        self.recheck()?;
        let initially_dirty = guard.dirty;
        let (history, pins) = if guard.context != self.context {
            (Vec::new(), BTreeMap::new())
        } else {
            admitted_origins(
                &self.repository,
                &self.context,
                &guard.origins,
                self.current_run.as_deref(),
            )
        };
        // A crash, failed child, refusal, or abandoned transaction cannot leave
        // the previous clean inventory available to another checker.
        let prior = if initially_dirty {
            Vec::new()
        } else {
            history.clone()
        };
        *guard = Ledger {
            contract_version: CONTRACT_VERSION,
            context: self.context.clone(),
            dirty: true,
            origins: history.clone(),
        };
        publish_ledger(&self.ledger_path, &guard)?;
        Ok(Transaction {
            run: self,
            guard: Some(guard),
            owner: Some(owner),
            prior,
            history,
            initially_dirty,
            pins,
            finished: false,
            #[cfg(test)]
            commit_artifact_limit: MAXIMUM_CLOSURE_ARTIFACT_BYTES,
        })
    }

    fn recheck(&self) -> Result<(), DevError> {
        let current_snapshot = snapshot::capture(&self.repository)?;
        let current_runtime = snapshot::runtime_identity(&self.repository, self.commands.clone())?;
        if current_snapshot.digest != self.snapshot.digest
            || current_snapshot.cargo_lock_digest != self.snapshot.cargo_lock_digest
            || current_runtime.digest != self.runtime.digest
        {
            return Err(refusal("source or runtime changed during Cargo ownership"));
        }
        self.context
            .recheck(&self.repository, &current_snapshot, &current_runtime)
    }
}

impl Ledger {
    fn dirty(context: &Context) -> Self {
        Self {
            contract_version: CONTRACT_VERSION,
            context: context.clone(),
            dirty: true,
            origins: Vec::new(),
        }
    }
}

impl Transaction<'_> {
    #[cfg(test)]
    fn finish_with_artifact_limit(
        &mut self,
        gate: &Gate,
        process: &ProcessObservation,
        run_directory: &Path,
        artifact_limit: u64,
    ) -> Result<Option<FileProof>, DevError> {
        if artifact_limit > MAXIMUM_CLOSURE_ARTIFACT_BYTES {
            return Err(refusal(
                "test origin artifact bound cannot exceed production",
            ));
        }
        self.commit_artifact_limit = artifact_limit;
        self.finish(gate, process, run_directory)
    }

    pub(super) fn finish(
        &mut self,
        gate: &Gate,
        process: &ProcessObservation,
        run_directory: &Path,
    ) -> Result<Option<FileProof>, DevError> {
        self.ensure_unfinished()?;
        self.finished = true;
        if self.guard.is_none() {
            if is_writer(gate) {
                return Err(refusal("Cargo writer has no owned transaction"));
            }
            return Ok(None);
        }
        if !is_writer(gate) {
            return Err(refusal("Cargo transaction finished with a different owner"));
        }
        if self.owner.as_ref() != Some(&Producer::capture(gate)?) {
            return Err(refusal(
                "Cargo transaction finished with a different producer binding",
            ));
        }
        if process.status != ProcessStatus::Passed {
            return Ok(None);
        }
        self.run.recheck()?;
        successful_process(gate, process)?;
        if !is_producer(gate) {
            self.commit_unchanged()?;
            return Ok(None);
        }
        let producer = Producer::capture(gate)?;
        let stdout_path = proof_path(&self.run.repository, &process.stdout)?;
        process_paths(&self.run.repository, run_directory, gate, process)?;
        super::cargo_outputs::verify(&self.run.repository, gate, &stdout_path, &process.stdout)?;
        let mut selected = units::read_units(
            &self.run.repository,
            Path::new(&self.run.context.target),
            &self.run.context.workspace_manifests,
            gate,
            &stdout_path,
            &process.stdout,
        )?;
        let mut prior_by_key = BTreeMap::new();
        for origin in &self.prior {
            prior_by_key.insert(origin_key(&origin.unit, &origin.identity_command)?, origin);
        }
        let mut newly_retained = Vec::new();
        let mut original_proofs = Vec::with_capacity(selected.len());
        for unit in &mut selected {
            if unit.fresh && !units::schema_generator(unit) {
                let previous = prior_by_key
                    .get(&origin_key(unit, &producer.identity_command)?)
                    .ok_or_else(|| refusal("warm workspace unit has no admitted cold origin"))?;
                if unit.filenames != previous.unit.filenames
                    || (shared_binary(unit)
                        && producer.identity_command != previous.identity_command)
                {
                    return Err(refusal(
                        "warm workspace unit differs from its exact cold producer",
                    ));
                }
                units::verify_retained(&self.run.repository, &previous.unit)?;
                unit.retained.clone_from(&previous.unit.retained);
                original_proofs.push(Some(previous.cold_proof.clone()));
            } else {
                newly_retained.push(unit.clone());
                original_proofs.push(None);
            }
        }
        if !newly_retained.is_empty() {
            units::retain_units(
                &self.run.repository,
                run_directory,
                gate,
                &mut newly_retained,
            )?;
            let mut retained_by_key = BTreeMap::new();
            for unit in newly_retained {
                retained_by_key.insert(key(&unit)?, unit.retained);
            }
            for unit in &mut selected {
                if !unit.fresh || units::schema_generator(unit) {
                    unit.retained = retained_by_key
                        .remove(&key(unit)?)
                        .ok_or_else(|| refusal("missing newly retained cold workspace unit"))?;
                }
            }
        }
        self.run.recheck()?;
        let units = selected
            .into_iter()
            .zip(original_proofs)
            .map(|(unit, cold_proof)| ProvedUnit { unit, cold_proof })
            .collect();
        let retained = Proof {
            contract_version: CONTRACT_VERSION,
            context: self.run.context.clone(),
            producer,
            process: process.clone(),
            units,
        };
        let path = run_directory.join(format!("{}.cargo-origin.json", gate.name));
        new_sidecar(&path, &retained)?;
        let proof = evidence::proof(&path, evidence::relative(&self.run.repository, &path))?;
        // Read the published original through the same independent admission as
        // a later receipt reader before making the mutable inventory clean.
        let mut pins = verify_owned(
            &self.run.repository,
            gate,
            process,
            &proof,
            &self.run.context,
            self.run.current_run.as_deref(),
        )?;
        self.pins.append(&mut pins);
        self.run.recheck()?;
        let mut selected_live_bytes = 0_u64;
        for proved in &retained.units {
            charge_live(&proved.unit, &mut selected_live_bytes)?;
            live(&proved.unit)?;
        }
        let mut origins: BTreeMap<String, Origin> = self
            .history
            .iter()
            .cloned()
            .map(|origin| Ok((origin_key(&origin.unit, &origin.identity_command)?, origin)))
            .collect::<Result<_, DevError>>()?;
        for proved in &retained.units {
            // The script may have been cached by an earlier nonproducer. Its
            // executable/freshness is retained honestly, while generated meaning
            // is independently byte-checked. Never mint a cold origin for it.
            if units::schema_generator(&proved.unit) {
                continue;
            }
            let origin = match &proved.cold_proof {
                Some(original) => self
                    .prior
                    .iter()
                    .find(|prior| {
                        prior.cold_proof == *original
                            && prior.unit.filenames == proved.unit.filenames
                            && origin_key(&prior.unit, &prior.identity_command).ok()
                                == origin_key(&proved.unit, &retained.producer.identity_command)
                                    .ok()
                    })
                    .cloned()
                    .ok_or_else(|| refusal("warm origin disappeared during commit"))?,
                None => Origin {
                    unit: proved.unit.clone(),
                    cold_proof: proof.clone(),
                    identity_command: retained.producer.identity_command.clone(),
                },
            };
            origins.insert(origin_key(&origin.unit, &origin.identity_command)?, origin);
        }
        self.commit(origins.into_values().collect(), false)?;
        Ok(Some(proof))
    }

    // A cache hit performs no Cargo mutation. Preserve authenticated cold
    // history without granting warm permission to an initially dirty target.
    pub(super) fn finish_unchanged(&mut self) -> Result<(), DevError> {
        self.ensure_unfinished()?;
        self.finished = true;
        if self.guard.is_none() {
            return Ok(());
        }
        self.run.recheck()?;
        self.commit_unchanged()
    }

    fn ensure_unfinished(&self) -> Result<(), DevError> {
        if self.finished {
            return Err(refusal("Cargo transaction was already finished"));
        }
        Ok(())
    }

    fn commit_unchanged(&mut self) -> Result<(), DevError> {
        self.run.recheck()?;
        self.commit(self.history.clone(), self.initially_dirty)
    }

    fn commit(&mut self, origins: Vec<Origin>, dirty: bool) -> Result<(), DevError> {
        if origins.len() > MAXIMUM_LEDGER_UNITS {
            return Err(refusal("Cargo origin inventory exceeds its unit limit"));
        }
        if !dirty {
            #[cfg(test)]
            let artifact_limit = self.commit_artifact_limit;
            #[cfg(not(test))]
            let artifact_limit = MAXIMUM_CLOSURE_ARTIFACT_BYTES;
            // The complete eligible history and new origins must fit together.
            // Separate successful passes over either subset cannot authorize a
            // larger clean ledger, and a failed entry must never be filtered.
            let (mut pins, _) = preflight_origins(
                &self.run.repository,
                &self.run.context,
                &origins,
                self.run.current_run.as_deref(),
                artifact_limit,
            )?;
            self.pins.append(&mut pins);
            self.run.recheck()?;
        }
        let next = Ledger {
            contract_version: CONTRACT_VERSION,
            context: self.run.context.clone(),
            dirty,
            origins,
        };
        let guard = self
            .guard
            .as_mut()
            .ok_or_else(|| refusal("Cargo origin commit has no writer guard"))?;
        if let Err(error) = publish_ledger(&self.run.ledger_path, &next) {
            **guard = Ledger {
                contract_version: CONTRACT_VERSION,
                context: self.run.context.clone(),
                dirty: true,
                origins: next.origins,
            };
            if let Err(dirty_error) = publish_ledger(&self.run.ledger_path, guard) {
                return Err(DevError::infrastructure(format!(
                    "Cargo origin commit failed: {error}; dirty-state persistence also failed: {dirty_error}"
                )));
            }
            return Err(error);
        }
        **guard = next;
        Ok(())
    }
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

pub(super) fn verify(
    repository: &Path,
    gate: &Gate,
    process: &ProcessObservation,
    proof: &FileProof,
    expected_context: &Context,
) -> Result<(), DevError> {
    let _pins = verify_owned(repository, gate, process, proof, expected_context, None)?;
    Ok(())
}

fn verify_owned(
    repository: &Path,
    gate: &Gate,
    process: &ProcessObservation,
    proof: &FileProof,
    expected_context: &Context,
    current_run: Option<&Path>,
) -> Result<BTreeMap<PathBuf, File>, DevError> {
    if !is_producer(gate) {
        return Err(refusal("Cargo origin proof belongs to a nonproducer"));
    }
    let mut closure = Closure::new(repository, expected_context, current_run);
    let original = closure.read(proof, 0)?;
    if original.producer != Producer::capture(gate)?
        || serde_json::to_vec(&original.process)? != serde_json::to_vec(process)?
    {
        return Err(refusal(
            "Cargo origin proof has a different producer observation",
        ));
    }
    Ok(closure.pins)
}

struct Closure<'a> {
    repository: &'a Path,
    context: &'a Context,
    active: BTreeSet<String>,
    checked: BTreeMap<String, (FileProof, Proof)>,
    metadata_bytes: u64,
    artifact_bytes: u64,
    artifact_limit: u64,
    log_bytes: u64,
    current_run: Option<&'a Path>,
    pins: BTreeMap<PathBuf, File>,
    referenced: BTreeSet<String>,
}

impl<'a> Closure<'a> {
    fn new(repository: &'a Path, context: &'a Context, current_run: Option<&'a Path>) -> Self {
        Self {
            repository,
            context,
            active: BTreeSet::new(),
            checked: BTreeMap::new(),
            metadata_bytes: 0,
            artifact_bytes: 0,
            artifact_limit: MAXIMUM_CLOSURE_ARTIFACT_BYTES,
            log_bytes: 0,
            current_run,
            pins: BTreeMap::new(),
            referenced: BTreeSet::new(),
        }
    }

    fn read(&mut self, proof: &FileProof, depth: usize) -> Result<Proof, DevError> {
        if depth > MAXIMUM_CLOSURE_DEPTH {
            return Err(refusal(
                "Cargo cold-origin proof closure exceeds its depth limit",
            ));
        }
        regular_proof(proof, MAXIMUM_METADATA_BYTES)?;
        let path = proof_path(self.repository, proof)?;
        let label = path_string(&path)?;
        if !self.referenced.contains(&label) {
            if self.referenced.len() >= MAXIMUM_ORIGINAL_PROOFS {
                return Err(refusal(
                    "Cargo cold-origin proof attempts exceed their count limit",
                ));
            }
            self.referenced.insert(label.clone());
        }
        self.pin_path(&path)?;
        if let Some((original_proof, previous)) = self.checked.get(&label) {
            if original_proof != proof {
                return Err(refusal(
                    "original Cargo origin sidecar has inconsistent references",
                ));
            }
            return Ok(previous.clone());
        }
        if self.active.len() + self.checked.len() >= MAXIMUM_ORIGINAL_PROOFS {
            return Err(refusal(
                "Cargo cold-origin proof closure exceeds its count limit",
            ));
        }
        if !self.active.insert(label.clone()) {
            return Err(refusal("Cargo cold-origin proof closure is cyclic"));
        }
        let result = self.read_uncached(proof, &path, depth);
        self.active.remove(&label);
        let retained = result?;
        self.checked
            .insert(label, (proof.clone(), retained.clone()));
        Ok(retained)
    }

    fn pin_path(&mut self, path: &Path) -> Result<(), DevError> {
        let Some(run) = managed_root(self.repository, path)? else {
            return Ok(());
        };
        // Only the constructor's exact caller-owned current run can avoid a
        // shared pin; no sidecar or process field grants that ownership.
        if self.current_run == Some(run.as_path()) || self.pins.contains_key(&run) {
            return Ok(());
        }
        if self.pins.len() >= MAXIMUM_ORIGINAL_PROOFS {
            return Err(refusal(
                "Cargo retained producer pins exceed their count limit",
            ));
        }
        let lease = shared_run_pin(&run)?;
        self.pins.insert(run, lease);
        Ok(())
    }

    fn read_uncached(
        &mut self,
        proof: &FileProof,
        path: &Path,
        depth: usize,
    ) -> Result<Proof, DevError> {
        let expected = proof
            .bytes
            .ok_or_else(|| refusal("Cargo origin sidecar has no length"))?;
        self.metadata_bytes = self
            .metadata_bytes
            .checked_add(expected)
            .ok_or_else(|| refusal("Cargo origin closure byte count overflow"))?;
        if self.metadata_bytes > MAXIMUM_CLOSURE_BYTES {
            return Err(refusal(
                "Cargo origin closure exceeds its metadata byte limit",
            ));
        }
        let bytes = super::cache::read_bounded(path, MAXIMUM_METADATA_BYTES)?;
        if bytes.len() as u64 != expected
            || proof.digest.as_ref() != Some(&VerificationDigest::of(&bytes))
            || evidence::proof(path, proof.path.clone())? != *proof
        {
            return Err(refusal("original Cargo origin sidecar changed"));
        }
        let retained: Proof = serde_json::from_slice(&bytes)?;
        if retained.contract_version != CONTRACT_VERSION || retained.context != *self.context {
            return Err(refusal(
                "Cargo origin sidecar has a foreign source or runtime context",
            ));
        }
        if retained.units.is_empty() || retained.units.len() > MAXIMUM_PROOF_UNITS {
            return Err(refusal(
                "Cargo origin sidecar has an invalid workspace inventory",
            ));
        }
        if fs::canonicalize(self.repository)? != Path::new(&retained.context.repository) {
            return Err(refusal("Cargo origin repository binding changed"));
        }
        let gate = retained.producer.gate()?;
        supported_command(&gate, &retained.context)?;
        if !is_producer(&gate)
            || path.file_name().and_then(|value| value.to_str())
                != Some(format!("{}.cargo-origin.json", gate.name).as_str())
        {
            return Err(refusal("Cargo origin sidecar has an invalid producer path"));
        }
        successful_process(&gate, &retained.process)?;
        let run_directory = path
            .parent()
            .ok_or_else(|| refusal("Cargo origin sidecar has no retained run"))?;
        process_paths(self.repository, run_directory, &gate, &retained.process)?;
        let stdout_path = proof_path(self.repository, &retained.process.stdout)?;
        let stderr_path = proof_path(self.repository, &retained.process.stderr)?;
        // This verifier and the two Cargo record readers each hash/read stdout;
        // precharge all four streams before any of those reads. Stderr is hashed
        // once. The original gate's existing stream limits remain unchanged.
        for _ in 0..4 {
            charge_artifact(
                &stdout_path,
                &retained.process.stdout,
                &mut self.log_bytes,
                MAXIMUM_CLOSURE_LOG_BYTES,
            )?;
        }
        charge_artifact(
            &stderr_path,
            &retained.process.stderr,
            &mut self.log_bytes,
            MAXIMUM_CLOSURE_LOG_BYTES,
        )?;
        verify_file(&stdout_path, &retained.process.stdout)?;
        verify_file(&stderr_path, &retained.process.stderr)?;
        super::cargo_outputs::verify(
            self.repository,
            &gate,
            &stdout_path,
            &retained.process.stdout,
        )?;
        let recorded: Vec<_> = retained
            .units
            .iter()
            .map(|entry| entry.unit.clone())
            .collect();
        // verify_records streams every retained occurrence. Charge those actual
        // reads, including a repeated retained path in distinct producer proofs,
        // before entering the helper. Repeated whole-proof references are cached.
        for unit in &recorded {
            for (original, file) in unit.filenames.iter().zip(&unit.retained) {
                let retained_path = proof_path(self.repository, file)?;
                self.pin_path(&retained_path)?;
                charge_artifact(
                    &retained_path,
                    file,
                    &mut self.artifact_bytes,
                    self.artifact_limit,
                )?;
                if unit.build_script.is_some() && units::schema_source_file(original) {
                    // Replay also compares these small generated bytes directly
                    // with the immutable checker's source-derived expectation.
                    charge_artifact(
                        &retained_path,
                        file,
                        &mut self.artifact_bytes,
                        self.artifact_limit,
                    )?;
                }
            }
        }
        units::verify_records(
            self.repository,
            Path::new(&retained.context.target),
            &retained.context.workspace_manifests,
            &gate,
            &stdout_path,
            &retained.process.stdout,
            &recorded,
        )?;
        let mut keys = BTreeSet::new();
        let mut cold_index = 0_usize;
        for entry in &retained.units {
            if !keys.insert(key(&entry.unit)?) {
                return Err(refusal("Cargo origin proof repeats a workspace unit"));
            }
            match (entry.unit.fresh, &entry.cold_proof) {
                (false, None) => {
                    cold_paths(
                        self.repository,
                        run_directory,
                        &gate,
                        &entry.unit,
                        cold_index,
                    )?;
                    cold_index += 1;
                }
                (true, None) if units::schema_generator(&entry.unit) => {
                    cold_paths(
                        self.repository,
                        run_directory,
                        &gate,
                        &entry.unit,
                        cold_index,
                    )?;
                    cold_index += 1;
                }
                (true, Some(original)) => {
                    let cold = self.read(original, depth + 1)?;
                    let original_unit = exact_cold_unit(&cold, &entry.unit)?;
                    if original_unit.retained != entry.unit.retained
                        || (shared_binary(&entry.unit)
                            && retained.producer.identity_command != cold.producer.identity_command)
                    {
                        return Err(refusal(
                            "warm Cargo evidence differs from its cold original",
                        ));
                    }
                }
                _ => return Err(refusal("Cargo origin proof launders workspace freshness")),
            }
        }
        Ok(retained)
    }
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

    fn gate(&self) -> Result<Gate, DevError> {
        let mut gate = Gate::new(&self.name, self.command.clone());
        gate.identity_command = Some(self.identity_command.clone());
        gate.environment = self.environment.clone();
        gate.timeout = Duration::from_nanos(self.timeout_nanoseconds);
        gate.maximum_stdout_bytes = self.maximum_stdout_bytes;
        gate.maximum_stderr_bytes = self.maximum_stderr_bytes;
        gate.required_outputs = self.required_outputs.iter().map(PathBuf::from).collect();
        if Self::capture(&gate)? != *self {
            return Err(refusal("Cargo producer binding is inconsistent"));
        }
        Ok(gate)
    }
}

fn admitted_origins(
    repository: &Path,
    context: &Context,
    origins: &[Origin],
    current_run: Option<&Path>,
) -> (Vec<Origin>, BTreeMap<PathBuf, File>) {
    let mut admitted = Vec::new();
    let mut closure = Closure::new(repository, context, current_run);
    for origin in origins {
        let result = (|| {
            let proof = closure.read(&origin.cold_proof, 0)?;
            let original = exact_cold_unit(&proof, &origin.unit)?;
            if original != &origin.unit
                || origin.identity_command != proof.producer.identity_command
            {
                return Err(refusal(
                    "Cargo inventory differs from its original cold producer",
                ));
            }
            Ok(())
        })();
        // This authenticates retained cold history, independently of whatever
        // currently occupies the mutable Cargo path. Actual warm admission
        // separately requires the selected post-Cargo file proofs to match.
        if result.is_ok() {
            admitted.push(origin.clone());
        }
    }
    (admitted, closure.pins)
}

fn preflight_origins(
    repository: &Path,
    context: &Context,
    origins: &[Origin],
    current_run: Option<&Path>,
    artifact_limit: u64,
) -> Result<(BTreeMap<PathBuf, File>, u64), DevError> {
    if artifact_limit > MAXIMUM_CLOSURE_ARTIFACT_BYTES || origins.len() > MAXIMUM_LEDGER_UNITS {
        return Err(refusal(
            "candidate origin history exceeds its preflight bound",
        ));
    }
    let mut closure = Closure::new(repository, context, current_run);
    closure.artifact_limit = artifact_limit;
    let mut keys = BTreeSet::new();
    for origin in origins {
        if !keys.insert(origin_key(&origin.unit, &origin.identity_command)?) {
            return Err(refusal("candidate origin history repeats an exact origin"));
        }
        let proof = closure.read(&origin.cold_proof, 0)?;
        let original = exact_cold_unit(&proof, &origin.unit)?;
        if original != &origin.unit || origin.identity_command != proof.producer.identity_command {
            return Err(refusal(
                "candidate origin differs from its actual cold producer",
            ));
        }
    }
    Ok((closure.pins, closure.artifact_bytes))
}

fn exact_cold_unit<'a>(proof: &'a Proof, current: &Unit) -> Result<&'a Unit, DevError> {
    let expected = key(current)?;
    let mut matches = proof
        .units
        .iter()
        .filter(|entry| key(&entry.unit).ok().as_ref() == Some(&expected));
    let original = matches
        .next()
        .ok_or_else(|| refusal("original producer has no exact workspace unit"))?;
    if matches.next().is_some()
        || original.unit.fresh
        || units::schema_generator(&original.unit)
        || original.cold_proof.is_some()
        || original.unit.filenames != current.filenames
    {
        return Err(refusal(
            "workspace unit does not have an actual cold producer",
        ));
    }
    Ok(&original.unit)
}

fn live(unit: &Unit) -> Result<(), DevError> {
    key(unit)?;
    for proof in &unit.filenames {
        verify_file(Path::new(&proof.path), proof)?;
    }
    Ok(())
}

fn charge_live(unit: &Unit, total: &mut u64) -> Result<(), DevError> {
    for proof in &unit.filenames {
        charge_artifact(
            Path::new(&proof.path),
            proof,
            total,
            MAXIMUM_ORIGIN_INVENTORY_BYTES,
        )?;
    }
    Ok(())
}

fn charge_artifact(
    path: &Path,
    proof: &FileProof,
    total: &mut u64,
    maximum: u64,
) -> Result<(), DevError> {
    regular_proof(proof, MAXIMUM_ORIGIN_ARTIFACT_BYTES)?;
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || proof.bytes != Some(metadata.len())
        || metadata.len() > MAXIMUM_ORIGIN_ARTIFACT_BYTES
    {
        return Err(refusal(
            "Cargo artifact has an unsafe or oversized original stream",
        ));
    }
    *total = total
        .checked_add(metadata.len())
        .ok_or_else(|| refusal("Cargo artifact stream budget overflow"))?;
    if *total > maximum {
        return Err(refusal(
            "Cargo artifact streams exceed their aggregate origin bound",
        ));
    }
    Ok(())
}

fn shared_binary(unit: &Unit) -> bool {
    unit.target
        .get("kind")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")))
        && unit
            .profile
            .get("test")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
        && unit.filenames.iter().any(|file| {
            Path::new(&file.path)
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name != std::ffi::OsStr::new("deps"))
        })
}

fn key(unit: &Unit) -> Result<String, DevError> {
    Ok(units::unit_key(unit)?.as_str().to_owned())
}

fn origin_key(unit: &Unit, identity_command: &[String]) -> Result<String, DevError> {
    let unit_key = key(unit)?;
    if !shared_binary(unit) {
        return Ok(unit_key);
    }
    command(identity_command)?;
    #[derive(Serialize)]
    struct BinaryOrigin<'a> {
        domain: &'static str,
        unit_key: &'a str,
        identity_command: &'a [String],
    }
    let bytes = serde_json::to_vec(&BinaryOrigin {
        domain: "lkjscript.dev.cargo-origin-shared-binary.v1",
        unit_key: &unit_key,
        identity_command,
    })?;
    if bytes.len() as u64 > MAXIMUM_METADATA_BYTES {
        return Err(refusal(
            "Cargo binary origin key exceeds its metadata bound",
        ));
    }
    Ok(VerificationDigest::of(&bytes).as_str().to_owned())
}

fn successful_process(gate: &Gate, process: &ProcessObservation) -> Result<(), DevError> {
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

fn cold_paths(
    repository: &Path,
    run_directory: &Path,
    gate: &Gate,
    unit: &Unit,
    cold_index: usize,
) -> Result<(), DevError> {
    for (index, proof) in unit.retained.iter().enumerate() {
        let expected = run_directory
            .join("retained-cargo-origin")
            .join(&gate.name)
            .join(cold_index.to_string())
            .join(index.to_string());
        if proof_path(repository, proof)? != expected {
            return Err(refusal(
                "cold workspace artifact is outside its original retained run",
            ));
        }
    }
    Ok(())
}

fn load_ledger(path: &Path, context: &Context) -> Result<Ledger, DevError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Ledger::dirty(context));
        }
        Err(error) => return Err(error.into()),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(refusal("Cargo origin inventory is not a regular file"));
        }
        Ok(_) => {}
    }
    let bytes = super::cache::read_bounded(path, MAXIMUM_METADATA_BYTES)?;
    let ledger: Ledger = serde_json::from_slice(&bytes)?;
    if ledger.contract_version != CONTRACT_VERSION {
        return Err(refusal(
            "Cargo origin inventory has an unsupported contract",
        ));
    }
    if ledger.origins.len() > MAXIMUM_LEDGER_UNITS {
        return Err(refusal("Cargo origin inventory exceeds its unit limit"));
    }
    if ledger.context != *context {
        return Ok(Ledger::dirty(context));
    }
    let mut keys = BTreeSet::new();
    for origin in &ledger.origins {
        if origin.unit.fresh || !keys.insert(origin_key(&origin.unit, &origin.identity_command)?) {
            return Err(refusal("Cargo origin inventory has an invalid cold unit"));
        }
        command(&origin.identity_command)?;
        regular_proof(&origin.cold_proof, MAXIMUM_METADATA_BYTES)?;
    }
    Ok(ledger)
}

fn publish_ledger(path: &Path, ledger: &Ledger) -> Result<(), DevError> {
    let bytes = metadata_bytes(ledger)?;
    evidence::publish(path, &bytes)?;
    Ok(())
}

fn new_sidecar(path: &Path, proof: &Proof) -> Result<(), DevError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => {
            return Err(refusal(
                "refusing to replace original Cargo origin evidence",
            ));
        }
    }
    let bytes = metadata_bytes(proof)?;
    evidence::publish(path, &bytes)?;
    Ok(())
}

fn owned_run(repository: &Path, run: &Path) -> Result<PathBuf, DevError> {
    let root = repository.join(".artifacts/lkjscript-dev/check");
    if run.parent() != Some(root.as_path()) || fs::canonicalize(run)? != run {
        return Err(refusal(
            "owned Cargo producing run is not a canonical managed run",
        ));
    }
    let name = run
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| refusal("owned Cargo run name is not UTF-8"))?;
    let fields: Vec<_> = name.split('-').collect();
    if fields.len() != 3
        || fields
            .iter()
            .any(|field| field.is_empty() || !field.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(refusal(
            "owned Cargo producing run has an invalid managed name",
        ));
    }
    let lease = run.join("ACTIVE.lock");
    let metadata = fs::symlink_metadata(&lease)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(refusal(
            "owned Cargo producing run has no regular active lease",
        ));
    }
    // Its exclusive ACTIVE lease is held by the trusted RunDirectory caller.
    // This path is supplied by that caller, never inferred from retained proof.
    Ok(run.to_path_buf())
}

fn managed_root(repository: &Path, path: &Path) -> Result<Option<PathBuf>, DevError> {
    let root = repository.join(".artifacts/lkjscript-dev/check");
    let relative = match path.strip_prefix(&root) {
        Ok(relative) => relative,
        Err(_) => return Ok(None),
    };
    let first = match relative.components().next() {
        Some(Component::Normal(first)) => first,
        _ => return Err(refusal("Cargo retained path has no managed producer run")),
    };
    let run = root.join(first);
    let metadata = fs::symlink_metadata(&run)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || fs::canonicalize(&run)? != run {
        return Err(refusal(
            "Cargo retained producer run has unsafe directory routing",
        ));
    }
    Ok(Some(run))
}

fn shared_run_pin(run: &Path) -> Result<File, DevError> {
    #[cfg(unix)]
    {
        let path = run.join("ACTIVE.lock");
        let descriptor = rustix::fs::open(
            &path,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::NONBLOCK,
            rustix::fs::Mode::empty(),
        )
        .map_err(|error| {
            DevError::infrastructure(format!("open retained Cargo producer pin: {error}"))
        })?;
        let file = File::from(descriptor);
        let opened = file.metadata()?;
        if !opened.is_file() || opened.uid() != rustix::process::geteuid().as_raw() {
            return Err(refusal(
                "retained Cargo producer lease is not an owned regular file",
            ));
        }
        FileExt::try_lock_shared(&file).map_err(|error| {
            DevError::infrastructure(format!(
                "retained Cargo producer pin is unavailable: {error}"
            ))
        })?;
        let selected = fs::symlink_metadata(&path)?;
        if selected.file_type().is_symlink()
            || !selected.is_file()
            || opened.dev() != selected.dev()
            || opened.ino() != selected.ino()
            || fs::canonicalize(run)? != run
        {
            return Err(refusal(
                "retained Cargo producer lease changed while pinning",
            ));
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        let _ = run;
        Err(DevError::unavailable(
            "Cargo retained producer pins require Unix leases",
        ))
    }
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

fn supported_command(gate: &Gate, context: &Context) -> Result<(), DevError> {
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
