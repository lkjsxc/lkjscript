use super::cargo_origin_context::{Context, TargetLease};
use super::model::{CheckReceipt, Gate, InputSnapshot, RuntimeIdentity};
use crate::error::DevError;
use crate::evidence::{self, FileProof, PublishedEvidence, VerificationDigest};
use crate::process::ProcessObservation;
use fs2::FileExt;
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(super) const MAXIMUM_RECEIPT_BYTES: usize = 64 * 1024;

/// A run owns one replaceable scratch directory. The lease lives in the shared
/// target so deleting scratch never unlinks the object used for exclusion.
pub(super) struct Scratch {
    path: PathBuf,
    _lease: File,
}

impl Scratch {
    pub(super) fn new(repository: &Path) -> Result<Self, DevError> {
        let target = Context::target(repository)?;
        Self::at(repository, &target)
    }

    fn at(repository: &Path, target: &Path) -> Result<Self, DevError> {
        fs::create_dir_all(target)?;
        let lease_path = target.join(".lkjscript-check-run.lock");
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lease_path)?;
        lease.try_lock_exclusive().map_err(|error| {
            DevError::infrastructure(format!(
                "another transient check owns the workspace: {error}"
            ))
        })?;
        let path = repository.join(".artifacts/lkjscript-dev/check/transient");
        if path.try_exists()? {
            if fs::symlink_metadata(&path)?.file_type().is_symlink() {
                return Err(DevError::corrupt("transient check directory is a symlink"));
            }
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir_all(&path)?;
        Ok(Self {
            path,
            _lease: lease,
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn cleanup(&self) -> Result<(), DevError> {
        if self.path.try_exists()? {
            fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Cargo remains the compilation cache owner. A check holds the target lease
/// through every joined gate and hashes the actual outputs selected by Cargo.
/// Readers cannot race a Cargo writer in this run; no executable is copied.
pub(super) struct Run {
    repository: PathBuf,
    context: Context,
    snapshot: InputSnapshot,
    runtime: RuntimeIdentity,
    commands: Vec<(String, String)>,
    access: RwLock<()>,
    _lease: TargetLease,
}

pub(super) enum Access<'a> {
    Reader { _guard: RwLockReadGuard<'a, ()> },
    Writer { _guard: RwLockWriteGuard<'a, ()> },
}

impl Run {
    pub(super) fn new(
        repository: &Path,
        snapshot: &InputSnapshot,
        runtime: &RuntimeIdentity,
        commands: Vec<(String, String)>,
    ) -> Result<Self, DevError> {
        let context = Context::capture(repository, snapshot, runtime)?;
        let lease = TargetLease::acquire(&context)?;
        Ok(Self {
            repository: repository.to_path_buf(),
            context,
            snapshot: snapshot.clone(),
            runtime: runtime.clone(),
            commands,
            access: RwLock::new(()),
            _lease: lease,
        })
    }

    pub(super) fn enter(&self, gate: &Gate) -> Result<Access<'_>, DevError> {
        if super::cargo_origin::is_writer(gate) {
            let guard = self
                .access
                .write()
                .map_err(|_| DevError::infrastructure("Cargo target writer was poisoned"))?;
            self.recheck()?;
            super::cargo_origin::supported_command(gate, &self.context)?;
            Ok(Access::Writer { _guard: guard })
        } else {
            Ok(Access::Reader {
                _guard: self
                    .access
                    .read()
                    .map_err(|_| DevError::infrastructure("Cargo target reader was poisoned"))?,
            })
        }
    }

    pub(super) fn finish(
        &self,
        gate: &Gate,
        process: &ProcessObservation,
        run_directory: &Path,
    ) -> Result<Option<FileProof>, DevError> {
        if !super::cargo_origin::is_writer(gate) {
            return Ok(None);
        }
        super::cargo_origin::successful_process(gate, process)?;
        self.recheck()?;
        if !super::cargo_origin::is_producer(gate) {
            return Ok(None);
        }
        let units = super::cargo_origin_units::read_units(
            &self.repository,
            Path::new(&self.context.target),
            &self.context.workspace_manifests,
            gate,
            &self.repository.join(&process.stdout.path),
            &process.stdout,
        )?;
        let inventory = json!({
            "schema": "lkjscript-live-cargo-1",
            "context_digest": VerificationDigest::of(&serde_json::to_vec(&self.context)?),
            "inventory_digest": VerificationDigest::of(&serde_json::to_vec(&units)?),
            "units": units.len(),
            "cargo_fresh_units": units.iter().filter(|unit| unit.fresh).count(),
            "files": units.iter().map(|unit| unit.filenames.len()).sum::<usize>(),
            "bytes": units.iter().flat_map(|unit| &unit.filenames)
                .filter_map(|file| file.bytes).fold(0_u64, u64::saturating_add),
            "test_executables": super::cargo_tests::inventory(&self.repository, &units)?,
            "workspace_test_executables": super::workspace_tests::inventory(&self.repository, &units)?,
        });
        let path = run_directory.join(format!("{}.live-cargo.json", gate.name));
        evidence::publish_json(&path, &inventory)?;
        Ok(Some(evidence::proof(
            &path,
            evidence::relative(&self.repository, &path),
        )?))
    }

    fn recheck(&self) -> Result<(), DevError> {
        let snapshot = super::snapshot::capture(&self.repository)?;
        let runtime = super::snapshot::runtime_identity(&self.repository, self.commands.clone())?;
        if snapshot.digest != self.snapshot.digest || runtime.digest != self.runtime.digest {
            return Err(DevError::corrupt(
                "source or runtime changed during Cargo ownership",
            ));
        }
        self.context.recheck(&self.repository, &snapshot, &runtime)
    }
}

/// This is an observation summary, never an archived source-admission receipt.
/// Every selected gate keeps its actual terminal status, even when logs are gone.
pub(super) fn receipt(
    repository: &Path,
    receipt: &CheckReceipt,
    archive: Option<&PublishedEvidence>,
) -> Result<Value, DevError> {
    let gates = receipt.gates.iter().map(|gate| -> Result<Value, DevError> {
        let cargo = if let Some(proof) = gate.cargo_origin.as_ref().filter(|proof| proof.path.ends_with(".live-cargo.json")) {
            let path = repository.join(&proof.path);
            if evidence::proof(&path, proof.path.clone())? != *proof {
                return Err(DevError::corrupt("live Cargo inventory changed before summarizing"));
            }
            let mut inventory = serde_json::from_slice::<Value>(&super::cache::read_bounded(&path, MAXIMUM_RECEIPT_BYTES as u64)?)?;
            for field in ["test_executables", "workspace_test_executables"] {
                if let Some(tests) = inventory.as_object_mut().and_then(|value| value.remove(field)) {
                    inventory[format!("{field}_digest")] = serde_json::to_value(VerificationDigest::of(&serde_json::to_vec(&tests)?))?;
                    inventory[field] = json!(tests.as_object().map_or(0, |tests| tests.len()));
                }
            }
            Some(inventory)
        } else {
            None
        };
        Ok(json!({
            "name": gate.name,
            "status": gate.status,
            "execution": gate.execution,
            "elapsed_nanoseconds": gate.elapsed_nanoseconds,
            "input_fingerprint": gate.input_fingerprint,
            "evidence_digest": gate.evidence_digest,
            "inputs": gate.inputs,
            "outputs": gate.outputs,
            "cargo": cargo,
            "exit_code": gate.process.as_ref().and_then(|process| process.exit_code),
            "signal": gate.process.as_ref().and_then(|process| process.signal),
            "stdout_digest": gate.process.as_ref().and_then(|process| process.stdout.digest.as_ref()),
            "stderr_digest": gate.process.as_ref().and_then(|process| process.stderr.digest.as_ref()),
            "reason": gate.reason.as_deref().map(bounded_text),
            "stdout_excerpt": gate.stdout_excerpt.as_deref().map(bounded_text),
            "stderr_excerpt": gate.stderr_excerpt.as_deref().map(bounded_text),
        }))
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "schema": "lkjscript-check-compact-1",
        "status": receipt.status,
        "profile": receipt.profile,
        "profile_definition_digest": receipt.profile_definition_digest,
        "started_unix_nanoseconds": receipt.started_unix_nanoseconds,
        "completed_unix_nanoseconds": receipt.completed_unix_nanoseconds,
        "elapsed_nanoseconds": receipt.elapsed_nanoseconds,
        "git_head": receipt.git_head,
        "worktree_input_digest": receipt.worktree_input_digest,
        "final_worktree_input_digest": receipt.final_worktree_input_digest,
        "input_stable": receipt.input_stable,
        "runtime_digest": receipt.runtime.as_ref().map(|runtime| &runtime.digest),
        "harness": receipt.runtime.as_ref().map(|runtime| &runtime.harness),
        "requested_gates": receipt.requested_gates,
        "selected_gates": receipt.selected_gates,
        "passed_gates": receipt.passed_gates,
        "fresh_passed_gates": receipt.fresh_passed_gates,
        "reused_passed_gates": receipt.reused_passed_gates,
        "unrun_gates": receipt.unrun_gates,
        "maximum_workers": receipt.maximum_workers,
        "fresh_required": receipt.fresh_required,
        "originals_retained": archive.is_some(),
        "archive": archive.map(|proof| json!({
            "path": evidence::relative(repository, &proof.path),
            "bytes": proof.bytes,
            "digest": proof.digest,
        })),
        "gates": gates,
        "failure": receipt.failure.as_ref().map(|failure| json!({
            "owner": failure.owner,
            "status": failure.status,
            "reason": bounded_text(&failure.reason),
        })),
    }))
}

pub(super) fn publish(
    repository: &Path,
    mut receipt: Value,
) -> Result<PublishedEvidence, DevError> {
    let mut bytes = serde_json::to_vec(&receipt)?;
    if bytes.len() >= MAXIMUM_RECEIPT_BYTES {
        if let Some(gates) = receipt.get_mut("gates").and_then(Value::as_array_mut) {
            for gate in gates {
                if let Some(gate) = gate.as_object_mut() {
                    gate.remove("stdout_excerpt");
                    gate.remove("stderr_excerpt");
                }
            }
        }
        bytes = serde_json::to_vec(&receipt)?;
    }
    if bytes.len() >= MAXIMUM_RECEIPT_BYTES {
        return Err(DevError::infrastructure(
            "compact check receipt exceeds 64 KiB",
        ));
    }
    evidence::publish(
        &repository.join(".artifacts/lkjscript-dev/latest-receipt.json"),
        &bytes,
    )
}

fn bounded_text(value: &str) -> String {
    let mut end = value.len().min(512);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    let mut value = value[..end].to_owned();
    if end == 512 {
        value.push_str("...");
    }
    value
}

#[cfg(test)]
mod tests;
