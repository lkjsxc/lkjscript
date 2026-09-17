//! Admission of source evidence in its producing checkout, before a portable
//! release decision is emitted. This is deliberately not a relocation reader.
use super::model::{
    AggregateStatus, CHECK_CONTRACT_VERSION, CacheLookupStatus, CheckReceipt, ExecutionKind,
    GateReceipt, GateStatus, InputManifest, MAXIMUM_WORKERS,
};
use super::{cache, executor, registry, snapshot};
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof};
use crate::process::{self, ProcessStatus};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

pub(crate) fn read_release_source_receipt(
    path: &Path,
    repository: &Path,
    commit_sha: &str,
) -> Result<usize, DevError> {
    let path = canonical_regular(path)?;
    let repository = repository.canonicalize()?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("source receipt parent is absent"))?;
    require(
        root.starts_with(repository.join(".artifacts/lkjscript-dev/check"))
            && path.file_name().is_some_and(|name| name == "receipt.json"),
        "source receipt is outside its producing checker run",
    )?;
    let receipt: CheckReceipt = read_original(&path)?;
    let registry = registry::base_registry(&repository, root, &root.join("lkjscript-dev"))?;
    let requested = registry::profile("release-source")
        .ok_or_else(|| DevError::infrastructure("release-source profile is absent"))?;
    admit(
        &path,
        &repository,
        commit_sha,
        &receipt,
        &registry,
        &requested,
    )?;
    Ok(receipt.selected_gates.len())
}

fn admit(
    path: &Path,
    repository: &Path,
    commit_sha: &str,
    receipt: &CheckReceipt,
    registry: &registry::GateRegistry,
    requested: &[String],
) -> Result<(), DevError> {
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("source receipt parent is absent"))?;
    let selected = registry.closure(requested)?;
    require(
        receipt.contract_version == CHECK_CONTRACT_VERSION
            && receipt.profile == "release-source"
            && receipt.status == AggregateStatus::Passed
            && receipt.failure.is_none()
            && receipt.git_head.as_deref() == Some(commit_sha)
            && receipt.input_stable
            && receipt.fresh_required
            && receipt.requested_gates == requested
            && receipt.selected_gates == selected
            && receipt.gates.len() == selected.len()
            && receipt.passed_gates == selected.len()
            && receipt.fresh_passed_gates == selected.len()
            && receipt.reused_passed_gates == 0
            && receipt.unrun_gates.is_empty()
            && (1..=MAXIMUM_WORKERS).contains(&receipt.maximum_workers)
            && receipt.completed_unix_nanoseconds >= receipt.started_unix_nanoseconds
            && receipt.profile_definition_digest
                == registry.profile_digest("release-source", requested)?,
        "source receipt is not the complete fresh maintained profile",
    )?;
    let inputs_path = root.join("inputs.json");
    let dag_path = root.join("dag.json");
    require(
        receipt.input_manifest.as_deref()
            == Some(evidence::relative(repository, &inputs_path).as_str())
            && receipt.dag_manifest.as_deref()
                == Some(evidence::relative(repository, &dag_path).as_str()),
        "source receipt substituted an input or dependency manifest",
    )?;
    let input: InputManifest = read_original(&inputs_path)?;
    let current = snapshot::capture(repository)?;
    require(
        input.contract_version == CHECK_CONTRACT_VERSION
            && current.git_head == commit_sha
            && same(&input.snapshot, &current)?
            && receipt.worktree_input_digest.as_ref() == Some(&current.digest)
            && receipt.final_worktree_input_digest.as_ref() == Some(&current.digest),
        "source receipt input closure no longer matches the producing source",
    )?;
    let expected_dag = registry.manifest(requested, &selected, receipt.maximum_workers)?;
    let dag: super::model::DagManifest = read_original(&dag_path)?;
    require(
        same(&dag, &expected_dag)?,
        "source dependency closure was weakened",
    )?;
    let runtime = receipt
        .runtime
        .as_ref()
        .ok_or_else(|| DevError::corrupt("source runtime binding is absent"))?;
    snapshot::validate_runtime(
        repository,
        runtime,
        &root.join("lkjscript-dev"),
        registry.input_commands(&selected)?,
    )?;
    let observed_gates: BTreeMap<String, GateReceipt> = receipt
        .gates
        .iter()
        .map(|gate| (gate.name.clone(), gate.clone()))
        .collect();
    for (observed, name) in receipt.gates.iter().zip(&selected) {
        let gate = registry.gate(name)?;
        let dependencies = gate
            .dependencies
            .iter()
            .map(|name| {
                observed_gates
                    .get(name)
                    .cloned()
                    .map(|value| (name.clone(), value))
                    .ok_or_else(|| DevError::corrupt("source gate dependency was not admitted"))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        require(
            observed.name == *name
                && observed.command == gate.command
                && observed.dependencies == gate.dependencies
                && observed.failed_dependencies.is_empty()
                && observed.status == GateStatus::Passed
                && observed.execution == ExecutionKind::Fresh
                && observed.reason.is_none()
                && observed.cache.lookup == CacheLookupStatus::Bypassed
                && observed.started_unix_nanoseconds >= receipt.started_unix_nanoseconds
                && observed.completed_unix_nanoseconds >= observed.started_unix_nanoseconds
                && observed.completed_unix_nanoseconds <= receipt.completed_unix_nanoseconds
                && dependencies.values().all(|dependency| {
                    dependency.completed_unix_nanoseconds <= observed.started_unix_nanoseconds
                })
                && observed.input_fingerprint
                    == executor::gate_fingerprint(
                        repository,
                        gate,
                        &current,
                        runtime,
                        &dependencies,
                    )?,
            "source gate command, dependency, freshness or input binding changed",
        )?;
        let process = observed
            .process
            .as_ref()
            .ok_or_else(|| DevError::corrupt("source gate child observation is absent"))?;
        require(
            process.status == ProcessStatus::Passed
                && process.exit_code == Some(0)
                && process.signal.is_none()
                && process.reason.is_none()
                && !process.stdout_limit_exhausted
                && !process.stderr_limit_exhausted
                && process.stdout_limit_bytes == gate.maximum_stdout_bytes
                && process.stderr_limit_bytes == gate.maximum_stderr_bytes
                && u128::from(process.elapsed_nanoseconds) <= gate.timeout.as_nanos(),
            "source child failed, exceeded its bounds or did not finish cleanup",
        )?;
        for (stream, proof) in [("stdout", &process.stdout), ("stderr", &process.stderr)] {
            let expected = root.join(format!("{name}.{stream}.log"));
            verify_retained(repository, &expected, proof)?;
        }
        require(
            observed.outputs.len() == gate.required_outputs.len()
                && observed.retained_outputs.len() == observed.outputs.len(),
            "source output evidence is incomplete",
        )?;
        for (index, ((output, retained), expected)) in observed
            .outputs
            .iter()
            .zip(&observed.retained_outputs)
            .zip(&gate.required_outputs)
            .enumerate()
        {
            let path = root.join("retained").join(name).join(index.to_string());
            verify_retained(repository, &path, retained)?;
            let mut original = retained.clone();
            original.path = evidence::relative(repository, expected);
            require(original == *output, "source output identity changed")?;
        }
        require(
            cache::gate_evidence_digest(
                name,
                &observed.input_fingerprint,
                process,
                &observed.outputs,
            )? == observed.evidence_digest,
            "source child evidence digest changed",
        )?;
    }
    Ok(())
}

fn verify_retained(repository: &Path, path: &Path, proof: &FileProof) -> Result<(), DevError> {
    canonical_regular(path)?;
    require(
        proof.kind == FileKind::File
            && proof.path == evidence::relative(repository, path)
            && evidence::proof(path, proof.path.clone())? == *proof,
        "source original log/output bytes or mode changed",
    )
}

fn read_original<T: DeserializeOwned + Serialize>(path: &Path) -> Result<T, DevError> {
    canonical_regular(path)?;
    let bytes = process::read_bounded(path, 128 * 1024 * 1024)?;
    let value = serde_json::from_slice(&bytes)?;
    require(
        evidence::encode_json(&value)? == bytes,
        "source evidence is not canonical",
    )?;
    Ok(value)
}

fn canonical_regular(path: &Path) -> Result<std::path::PathBuf, DevError> {
    let metadata = fs::symlink_metadata(path)?;
    require(
        path.is_absolute()
            && !path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
            && metadata.is_file()
            && !metadata.file_type().is_symlink()
            && path.canonicalize()? == path,
        "source original is not a canonical regular file",
    )?;
    Ok(path.to_path_buf())
}

fn same(a: &impl Serialize, b: &impl Serialize) -> Result<bool, DevError> {
    Ok(serde_json::to_vec(a)? == serde_json::to_vec(b)?)
}

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Gate, RuntimeIdentity};
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    }

    #[test]
    fn original_source_reader_rejects_forged_weakened_and_incomplete_evidence_then_recovers() {
        let temporary = tempfile::tempdir().expect("owned source fixture");
        let repository = temporary.path().canonicalize().expect("repository path");
        fs::write(
            repository.join("Cargo.lock"),
            b"independent source fixture\n",
        )
        .expect("lock");
        fs::write(repository.join(".gitignore"), b".artifacts/\n").expect("ignore evidence");
        for arguments in [
            vec!["init", "--quiet"],
            vec!["add", "--", "Cargo.lock", ".gitignore"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(arguments)
                    .current_dir(&repository)
                    .status()
                    .expect("fixture git")
                    .success()
            );
        }
        let root = repository.join(".artifacts/lkjscript-dev/check/original");
        fs::create_dir_all(&root).expect("owned evidence root");
        let harness = root.join("lkjscript-dev");
        fs::copy(std::env::current_exe().expect("test verifier"), &harness)
            .expect("immutable verifier copy");
        let output = root.join("declared-output");
        let mut first = Gate::new(
            "first",
            vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!("printf original > '{}'", output.display()),
            ],
        );
        first.required_outputs.push(output.clone());
        let mut second = Gate::new("second", vec!["/bin/true".to_owned()]);
        second.dependencies.push("first".to_owned());
        let registry = registry::GateRegistry::new(vec![first, second]).expect("fixture registry");
        let requested = vec!["second".to_owned()];
        let selected = registry.closure(&requested).expect("fixture closure");
        let snapshot = snapshot::capture(&repository).expect("source snapshot");
        let runtime: RuntimeIdentity = snapshot::runtime_identity(
            &repository,
            registry.input_commands(&selected).expect("input tools"),
        )
        .expect("source runtime");
        let inputs_path = root.join("inputs.json");
        let dag_path = root.join("dag.json");
        evidence::publish_json(
            &inputs_path,
            &InputManifest {
                contract_version: CHECK_CONTRACT_VERSION,
                snapshot: snapshot.clone(),
            },
        )
        .expect("inputs");
        evidence::publish_json(
            &dag_path,
            &registry.manifest(&requested, &selected, 1).expect("DAG"),
        )
        .expect("DAG original");
        let started = now();
        let cache = cache::VerificationCache::new(&repository, &root.join("cache"));
        let gates = executor::execute_dag(
            &registry,
            &selected,
            &executor::ExecutionOptions {
                repository: &repository,
                run_directory: &root,
                snapshot: &snapshot,
                runtime: &runtime,
                maximum_workers: 1,
                allow_reuse: false,
                fresh_reason: "release_source_profile_requires_fresh",
                cache: &cache,
            },
        )
        .expect("actual bounded fixture processes");
        let baseline = CheckReceipt {
            contract_version: CHECK_CONTRACT_VERSION,
            status: AggregateStatus::Passed,
            profile: "release-source".to_owned(),
            profile_definition_digest: registry
                .profile_digest("release-source", &requested)
                .expect("profile identity"),
            started_unix_nanoseconds: started,
            completed_unix_nanoseconds: now(),
            elapsed_nanoseconds: 1,
            git_head: Some(snapshot.git_head.clone()),
            worktree_input_digest: Some(snapshot.digest.clone()),
            final_worktree_input_digest: Some(snapshot.digest.clone()),
            input_stable: true,
            input_manifest: Some(evidence::relative(&repository, &inputs_path)),
            dag_manifest: Some(evidence::relative(&repository, &dag_path)),
            runtime: Some(runtime),
            requested_gates: requested.clone(),
            selected_gates: selected,
            passed_gates: 2,
            fresh_passed_gates: 2,
            reused_passed_gates: 0,
            unrun_gates: Vec::new(),
            maximum_workers: 1,
            fresh_required: true,
            gates,
            failure: None,
        };
        let path = root.join("receipt.json");
        let read = |receipt: &CheckReceipt| {
            admit(
                &path,
                &repository,
                &snapshot.git_head,
                receipt,
                &registry,
                &requested,
            )
        };
        read(&baseline).expect("genuine original admission");
        // A later build replacing the mutable output does not replace its preserved original.
        fs::write(&output, b"different later build").expect("replace mutable output");
        read(&baseline).expect("original output retained");
        for case in 0..10 {
            let mut fault = baseline.clone();
            match case {
                0 => {
                    fault.gates.pop();
                }
                1 => {
                    fault.gates[1].dependencies.clear();
                }
                2 => {
                    fault.gates[0].command = vec!["/bin/true".to_owned()];
                }
                3 => {
                    fault.gates[0].execution = ExecutionKind::Reused;
                }
                4 => {
                    fault.gates[0].status = GateStatus::Skipped;
                }
                5 => {
                    fault.gates[0].process.as_mut().expect("process").status =
                        ProcessStatus::Timeout;
                }
                6 => {
                    fault.gates[0].process.as_mut().expect("process").reason =
                        Some("cleanup failed".to_owned());
                }
                7 => {
                    fault.gates[0].retained_outputs.clear();
                }
                8 => {
                    fault.git_head = Some("f".repeat(40));
                }
                9 => {
                    let runtime = fault.runtime.as_mut().expect("runtime");
                    runtime
                        .command_executables
                        .insert("invented-tool".to_owned(), runtime.harness.clone());
                    runtime.digest =
                        snapshot::runtime_digest(runtime).expect("consistently rehashed runtime");
                }
                _ => unreachable!(),
            }
            assert!(read(&fault).is_err(), "forged case {case} passed");
        }
        let original = root.join("retained/first/0");
        fs::write(&original, b"tampered").expect("tamper output original");
        assert!(read(&baseline).is_err());
        fs::write(&original, b"original").expect("restore output original");
        let log = root.join("first.stdout.log");
        fs::write(&log, b"forged passed summary").expect("tamper process log");
        assert!(read(&baseline).is_err());
        fs::write(&log, b"").expect("restore genuine log");
        read(&baseline).expect("complete restored genuine originals recover");
        // The production entry point must not accept this deliberately small fixture
        // even though its two processes and all their originals really passed.
        evidence::publish_json(&path, &baseline).expect("fixture receipt");
        assert!(read_release_source_receipt(&path, &repository, &snapshot.git_head).is_err());
    }
}
