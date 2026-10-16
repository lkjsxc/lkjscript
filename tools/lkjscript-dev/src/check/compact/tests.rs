use super::*;
use crate::check::model::{
    AggregateStatus, CacheLookupStatus, CacheObservation, ExecutionKind, GateReceipt, GateStatus,
};
use crate::check::{Options, failure_receipt, parse};
use std::ffi::OsString;
use std::time::Instant;

#[test]
fn archive_retention_is_explicit_and_duplicate_options_are_rejected() {
    let parse_options = |values: &[&str]| parse(values.iter().map(OsString::from));
    assert!(
        !parse_options(&["full", "--fresh", "--machine"])
            .expect("default options")
            .retain_archive
    );
    assert!(
        parse_options(&["full", "--retain-archive"])
            .expect("archive options")
            .retain_archive
    );
    assert!(parse_options(&["full", "--retain-archive", "--retain-archive"]).is_err());
}

#[test]
fn scratch_excludes_concurrent_runs_and_removes_outputs_on_every_exit() -> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    let target = root.join("target");
    let stale = root.join(".artifacts/lkjscript-dev/check/transient");
    fs::create_dir_all(&stale)?;
    fs::write(stale.join("interrupted-fixture"), b"stale")?;
    {
        let scratch = Scratch::at(&root, &target)?;
        assert!(!scratch.path().join("interrupted-fixture").exists());
        assert!(Scratch::at(&root, &target).is_err());
        fs::create_dir(scratch.path().join("outputs"))?;
        fs::write(
            scratch.path().join("outputs/executable-fixture"),
            b"owned bytes",
        )?;
    }
    assert!(!stale.exists());
    let second = Scratch::at(&root, &target)?;
    second.cleanup()?;
    assert!(!stale.exists());
    Ok(())
}

#[test]
fn compact_full_matrix_retains_actual_failures_and_replaces_the_latest_receipt()
-> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let options = Options {
        profile: "full".into(),
        machine: true,
        details: false,
        fresh: true,
        retain_archive: false,
        evidence_root: None,
        jobs: 2,
    };
    let mut observed = failure_receipt(
        &options,
        1,
        Instant::now(),
        DevError::infrastructure("fixture failure"),
    );
    let states = [
        GateStatus::Passed,
        GateStatus::Failed,
        GateStatus::Unavailable,
        GateStatus::Timeout,
        GateStatus::OutputExhausted,
        GateStatus::Signaled,
        GateStatus::InfrastructureFailure,
        GateStatus::Skipped,
    ];
    for index in 0..40 {
        let name = format!("gate_{index}");
        let status = states[index % states.len()];
        observed.selected_gates.push(name.clone());
        observed.gates.push(GateReceipt {
            name,
            status,
            execution: if status == GateStatus::Skipped {
                ExecutionKind::Skipped
            } else {
                ExecutionKind::Fresh
            },
            command: vec!["fixture".into()],
            dependencies: Vec::new(),
            failed_dependencies: Vec::new(),
            started_unix_nanoseconds: 1,
            completed_unix_nanoseconds: 2,
            elapsed_nanoseconds: 1,
            process: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            retained_outputs: Vec::new(),
            cargo_origin: None,
            input_fingerprint: VerificationDigest::of(b"input"),
            evidence_digest: VerificationDigest::of(b"output"),
            cache: CacheObservation {
                eligible: false,
                lookup: CacheLookupStatus::Bypassed,
                reason: None,
                record: None,
                write: None,
            },
            reason: Some("actual failure".into()),
            stdout_excerpt: Some("x".repeat(2048)),
            stderr_excerpt: Some("y".repeat(2048)),
        });
    }
    let compact = receipt(temporary.path(), &observed, None)?;
    assert_eq!(compact["status"], "failed");
    assert_eq!(compact["originals_retained"], false);
    assert_eq!(compact["selected_gates"].as_array().map(Vec::len), Some(40));
    assert!(serde_json::from_value::<CheckReceipt>(compact.clone()).is_err());
    let published = publish(temporary.path(), compact)?;
    assert!(published.bytes < MAXIMUM_RECEIPT_BYTES as u64);
    let saved: Value = serde_json::from_slice(&fs::read(&published.path)?)?;
    for (index, gate) in observed.gates.iter().enumerate() {
        assert_eq!(
            saved["gates"][index]["status"],
            serde_json::to_value(gate.status)?
        );
    }
    observed.status = AggregateStatus::Passed;
    observed.gates.clear();
    observed.selected_gates.clear();
    observed.failure = None;
    let replaced = publish(
        temporary.path(),
        receipt(temporary.path(), &observed, None)?,
    )?;
    assert_eq!(published.path, replaced.path);
    assert_ne!(published.digest, replaced.digest);
    let saved: Value = serde_json::from_slice(&fs::read(replaced.path)?)?;
    assert_eq!(saved["status"], "passed");
    assert!(saved["failure"].is_null());
    Ok(())
}

#[test]
fn gate_owned_evidence_is_scoped_to_the_run_without_reducing_coverage() -> Result<(), DevError> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    let run = root.join("run");
    let registry = crate::check::registry::base_registry(&root, &run, Path::new("/bin/true"))?;
    let requested = crate::check::registry::profile("full")
        .ok_or_else(|| DevError::corrupt("full profile absent"))?;
    let selected = registry.closure(&requested)?;
    assert_eq!(selected.len(), 29);
    for name in [
        "distributed_http_application",
        "outbound_http_application",
        "stateful_http_application",
        "service_acceptance",
        "checker_self_test",
        "offline_packages",
        "pure_tail",
    ] {
        let gate = registry.gate(name)?;
        assert!(selected.iter().any(|selected| selected == name));
        let evidence = gate
            .command
            .windows(2)
            .find(|pair| pair[0] == "--evidence-root")
            .ok_or_else(|| DevError::corrupt("gate evidence root absent"))?;
        assert!(Path::new(&evidence[1]).starts_with(run.join("outputs")));
    }
    Ok(())
}

#[test]
fn transient_executor_references_outputs_without_copying_them() -> Result<(), DevError> {
    use crate::check::{cache, executor, registry, snapshot};
    use std::sync::Arc;
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().canonicalize()?;
    crate::check::cargo_origin::tests::prepare(&root)?;
    let scratch = Scratch::at(&root, &root.join("target"))?;
    let output = scratch.path().join("output");
    let mut gate = Gate::new(
        "produce",
        vec![
            "/bin/sh".into(),
            "-c".into(),
            "printf output > \"$1\"".into(),
            "fixture".into(),
            output.display().to_string(),
        ],
    );
    gate.required_outputs.push(output.clone());
    let registry = registry::GateRegistry::new(vec![gate])?;
    let names = vec!["produce".to_owned()];
    let source = snapshot::capture(&root)?;
    let commands = registry.input_commands(&names)?;
    let runtime = snapshot::runtime_identity(&root, commands.clone())?;
    let live = Arc::new(Run::new(
        &root,
        &source,
        &runtime,
        commands.into_iter().collect(),
    )?);
    let cache = cache::VerificationCache::new(&root, &scratch.path().join("unused-cache"));
    let results = executor::execute_dag(
        &registry,
        &names,
        &executor::ExecutionOptions {
            repository: &root,
            run_directory: scratch.path(),
            snapshot: &source,
            runtime: &runtime,
            maximum_workers: 1,
            allow_reuse: false,
            fresh_reason: "transient",
            cache: &cache,
            origin: None,
            live: Some(live),
        },
    )?;
    assert_eq!(results[0].status, GateStatus::Passed);
    assert_eq!(results[0].outputs.len(), 1);
    assert!(results[0].retained_outputs.is_empty());
    assert!(!scratch.path().join("retained").exists());
    assert!(!scratch.path().join("unused-cache").exists());
    assert_eq!(fs::read(&output)?, b"output");
    scratch.cleanup()?;
    assert!(!output.exists());
    Ok(())
}
