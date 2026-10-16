use super::*;
use crate::check::executor::{self, FreshInput};
use crate::check::model::{CacheLookupStatus, CacheObservation, GateReceipt, GateStatus};
use crate::evidence::VerificationDigest;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::sync::TryLockError;
use std::time::Instant;

// These are bounded synthetic Cargo records. No test creates a second real
// compilation target or invokes Cargo; production selection is checked by the
// same executor, transcript parser and archive reader used by actual gates.
pub(crate) fn prepare(repository: &Path) -> Result<(), DevError> {
    fs::create_dir_all(repository.join("src"))?;
    fs::create_dir_all(repository.join("target/release"))?;
    fs::write(
        repository.join(".gitignore"),
        ".artifacts/\ntarget/\nelsewhere/\n",
    )?;
    fs::write(
        repository.join("Cargo.toml"),
        "[package]\nname=\"lkjscript\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[workspace]\n",
    )?;
    fs::write(
        repository.join("Cargo.lock"),
        "version=4\n[[package]]\nname=\"lkjscript\"\nversion=\"0.0.0\"\n",
    )?;
    fs::write(repository.join("src/main.rs"), "fn main() {}\n")?;
    fs::write(
        repository.join("target/release/lkjscript"),
        b"unselected predecessor",
    )?;
    for arguments in [
        vec!["init", "--quiet", "--template="],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "--quiet",
            "-m",
            "synthetic Cargo evidence",
        ],
    ] {
        let mut command = vec!["git"];
        command.extend(arguments);
        snapshot::checked_text(repository, &command)?;
    }
    Ok(())
}

fn commands() -> Vec<(String, String)> {
    vec![("cargo".to_owned(), "cargo".to_owned())]
}

pub(crate) fn produce(
    repository: &Path,
    target: &Path,
    name: &str,
    fresh: bool,
) -> Result<GateReceipt, DevError> {
    let run = repository.join(".artifacts/lkjscript-dev/check").join(name);
    fs::create_dir_all(&run)?;
    let selected = target.join("release/lkjscript");
    fs::create_dir_all(
        selected
            .parent()
            .ok_or_else(|| refusal("fixture target parent absent"))?,
    )?;
    fs::write(&selected, fs::read(repository.join("src/main.rs"))?)?;
    let mut gate = Gate::new(
        name,
        vec![
            "cargo".into(),
            "build".into(),
            "--release".into(),
            "--locked".into(),
            "--message-format=json".into(),
        ],
    );
    bind_target(&mut gate, target)?;
    gate.required_outputs = vec![repository.join("target/release/lkjscript")];
    gate.cacheable = false;
    let source = snapshot::capture(repository)?;
    let runtime = snapshot::runtime_identity(repository, commands())?;
    let owner = Run::with_commands(repository, target, &source, &runtime, commands())?;
    let program = run.join("emit-records");
    fs::write(&program, b"#!/bin/sh\ncat \"$0.messages\"\n")?;
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700))?;
    let artifact = json!({
        "reason":"compiler-artifact", "package_id":format!("path+file://{}#lkjscript@0.0.0", repository.display()),
        "manifest_path":repository.join("Cargo.toml"),
        "target":{"name":"lkjscript","kind":["bin"],"crate_types":["bin"],"src_path":repository.join("src/main.rs"),"edition":"2024","doc":true,"doctest":false,"test":true},
        "profile":{"opt_level":"3","debuginfo":0,"debug_assertions":false,"overflow_checks":false,"test":false},
        "features":[], "executable":selected,"filenames":[selected],"fresh":fresh,
    });
    fs::write(
        run.join("emit-records.messages"),
        format!(
            "{artifact}\n{}\n",
            json!({"reason":"build-finished","success":true})
        ),
    )?;
    executor::execute_fresh_with_origin(
        FreshInput {
            repository,
            gate: &gate,
            inputs: Vec::new(),
            program: Some(&program),
            fingerprint: VerificationDigest::of(b"synthetic-cargo-record-fixture"),
            stdout_path: run.join(format!("{name}.stdout.log")),
            stderr_path: run.join(format!("{name}.stderr.log")),
            started_wall: 1,
            started: Instant::now(),
            cache_observation: CacheObservation {
                eligible: false,
                lookup: CacheLookupStatus::Bypassed,
                reason: None,
                record: None,
                write: None,
            },
        },
        &owner,
    )
}

fn binding(repository: &Path, receipt: &GateReceipt) -> Result<(Gate, Proof, FileProof), DevError> {
    let proof = receipt
        .cargo_origin
        .clone()
        .ok_or_else(|| refusal("archive fixture has no Cargo evidence"))?;
    let original: Proof = serde_json::from_slice(&fs::read(proof_path(repository, &proof)?)?)?;
    let mut gate = Gate::new(&receipt.name, receipt.command.clone());
    gate.required_outputs = vec![repository.join("target/release/lkjscript")];
    Ok((gate, original, proof))
}

#[test]
fn warm_cargo_selection_has_standalone_honest_archived_bytes() -> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    prepare(&root)?;
    let cold = produce(&root, &root.join("target"), "cold", false)?;
    assert_eq!(cold.status, GateStatus::Passed);
    fs::remove_dir_all(root.join(".artifacts/lkjscript-dev/check/cold"))?;
    let warm = produce(&root, &root.join("target"), "warm", true)?;
    assert_eq!(warm.status, GateStatus::Passed);
    let (gate, archive, proof) = binding(&root, &warm)?;
    assert!(archive.units.iter().all(|unit| unit.fresh));
    assert!(archive.units.iter().all(|unit| !unit.retained.is_empty()));
    assert!(
        !root
            .join(".artifacts/lkjscript-dev/cargo-origin-target-locks")
            .exists()
    );
    fs::write(root.join("target/release/lkjscript"), b"later Cargo output")?;
    verify(&root, &gate, &archive.process, &proof, &archive.context)?;
    Ok(())
}

#[test]
fn archive_rejects_changed_bytes_records_context_and_producer() -> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    prepare(&root)?;
    let receipt = produce(&root, &root.join("target"), "selected", true)?;
    assert_eq!(receipt.status, GateStatus::Passed);
    let (gate, archive, proof) = binding(&root, &receipt)?;
    let file = proof_path(&root, &archive.units[0].retained[0])?;
    let bytes = fs::read(&file)?;
    fs::write(&file, b"changed archive")?;
    assert!(verify(&root, &gate, &archive.process, &proof, &archive.context).is_err());
    fs::write(file, bytes)?;
    let mut context = archive.context.clone();
    context.input_digest = VerificationDigest::of(b"different source");
    assert!(verify(&root, &gate, &archive.process, &proof, &context).is_err());
    let mut other = gate.clone();
    other.name = "different_producer".into();
    assert!(verify(&root, &other, &archive.process, &proof, &archive.context).is_err());
    let mut altered = archive.clone();
    altered.units[0].fresh = false;
    let path = proof_path(&root, &proof)?;
    evidence::publish_json(&path, &altered)?;
    let changed_proof = evidence::proof(&path, proof.path.clone())?;
    assert!(
        verify(
            &root,
            &gate,
            &archive.process,
            &changed_proof,
            &archive.context
        )
        .is_err()
    );
    evidence::publish_json(&path, &archive)?;
    verify(&root, &gate, &archive.process, &proof, &archive.context)?;
    Ok(())
}

#[test]
fn target_lease_writer_serialization_and_changed_source_are_enforced() -> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    prepare(&root)?;
    let source = snapshot::capture(&root)?;
    let runtime = snapshot::runtime_identity(&root, commands())?;
    let owner = Run::with_commands(&root, &root.join("target"), &source, &runtime, commands())?;
    assert!(
        Run::with_commands(&root, &root.join("target"), &source, &runtime, commands()).is_err()
    );
    let mut gate = Gate::new("writer", vec!["cargo".into(), "check".into()]);
    bind_target(&mut gate, &root.join("target"))?;
    let mut transaction = owner.begin(&gate)?;
    assert!(matches!(
        owner.state.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    transaction.finish_unchanged()?;
    assert!(transaction.finish_unchanged().is_err());
    drop(transaction);
    assert!(owner.state.try_lock().is_ok());
    fs::write(root.join("src/main.rs"), "fn main() { let _ = 1; }\n")?;
    assert!(owner.begin(&gate).is_err());
    drop(owner);
    Ok(())
}

#[test]
fn failed_child_cannot_publish_successful_cargo_evidence() -> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    prepare(&root)?;
    let receipt = produce(&root, &root.join("target"), "success", false)?;
    let (gate, archive, _) = binding(&root, &receipt)?;
    for status in [
        ProcessStatus::Failed,
        ProcessStatus::Unavailable,
        ProcessStatus::Timeout,
        ProcessStatus::OutputExhausted,
        ProcessStatus::Signaled,
        ProcessStatus::InfrastructureFailure,
    ] {
        let mut process = archive.process.clone();
        process.status = status;
        assert!(successful_process(&gate, &process).is_err());
    }
    Ok(())
}
