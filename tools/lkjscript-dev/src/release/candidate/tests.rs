use super::*;
use crate::evidence::{FileKind, FileProof, VerificationDigest};
use std::os::unix::fs::symlink;

fn identity(name: &str) -> ArtifactIdentity {
    ArtifactIdentity {
        name: name.to_owned(),
        byte_length: 8,
        sha256: Sha256Digest::new("2".repeat(64)).expect("fixture digest"),
    }
}

fn observation() -> process::ProcessObservation {
    let stream = FileProof {
        path: "retained.log".to_owned(),
        kind: FileKind::File,
        mode: Some(0o644),
        bytes: Some(0),
        digest: Some(VerificationDigest::of(b"")),
        link_target: None,
    };
    process::ProcessObservation {
        status: process::ProcessStatus::Passed,
        exit_code: Some(0),
        signal: None,
        reason: None,
        elapsed_nanoseconds: 1,
        cpu_nanoseconds: None,
        peak_rss_kib: None,
        stdout_limit_bytes: 16 * 1024 * 1024,
        stderr_limit_bytes: 16 * 1024 * 1024,
        stdout_limit_exhausted: false,
        stderr_limit_exhausted: false,
        stdout: stream.clone(),
        stderr: stream,
    }
}

// This fixture describes only the portable terminal reader's expected contract. It is not an
// accepted producer: real source/target originals and authenticated service provenance have
// separate owning readers. Counts and inventories below are independently specified expectations.
fn terminal() -> Terminal {
    Terminal {
        schema: SchemaIdentity {
            identity: "lkjscript-candidate-terminal".to_owned(),
            version: 1,
        },
        status: Status::CandidateAccepted,
        phase: "complete".to_owned(),
        source_commit: "1".repeat(40),
        controller_source_commit: "1".repeat(40),
        tag: "v0.1.39".to_owned(),
        acceptance_contract: "lkjscript-final-candidate-acceptance-1".to_owned(),
        workload: "release-source+six-target-owners+two-pinned-userlands+installed-recovery-1"
            .to_owned(),
        target_triple: "x86_64-unknown-linux-musl".to_owned(),
        target_policy_sha256: target::policy_sha256().expect("current target policy"),
        producer: HostedContext {
            github_actions: Some("true".to_owned()),
            repository: Some("lkjsxc/lkjscript".to_owned()),
            workflow: Some("Release".to_owned()),
            job: Some("candidate".to_owned()),
            run_id: Some("101".to_owned()),
            run_attempt: Some("2".to_owned()),
            run_url: Some("https://github.com/lkjsxc/lkjscript/actions/runs/101".to_owned()),
            runner_os: Some("Linux".to_owned()),
            runner_architecture: Some("X64".to_owned()),
            runner_image_os: Some("ubuntu24".to_owned()),
            runner_image_version: Some("fixture-image".to_owned()),
        },
        verifier: identity("lkjscript-dev"),
        assets: [
            "lkjscript-x86_64-unknown-linux-musl.tar.gz",
            "SHA256SUMS",
            "install.sh",
        ]
        .into_iter()
        .map(identity)
        .collect(),
        manifest_sha256: Some(Sha256Digest::new("3".repeat(64)).expect("manifest digest")),
        executable: Some(identity("lkjscript")),
        source_gates: 20,
        target_owners: 6,
        userlands: 2,
        proofs: ["release-source", "final-target", "installation"]
            .into_iter()
            .map(|name| Proof {
                name: name.to_owned(),
                receipt: identity("receipt.json"),
            })
            .collect(),
        stages: ["target-admission", "installation", "installation-reader"]
            .into_iter()
            .map(|name| Stage {
                name: name.to_owned(),
                process: observation(),
            })
            .collect(),
        started_unix_nanoseconds: 1,
        completed_unix_nanoseconds: Some(2),
        elapsed_nanoseconds: 1,
        cleanup_complete: true,
        failure: None,
    }
}

fn write_terminal(path: &Path, terminal: &Terminal) {
    fs::write(
        path,
        evidence::encode_json(terminal).expect("canonical fixture"),
    )
    .expect("write owned terminal fixture");
}

#[test]
fn candidate_terminal_reader_rejects_weakened_and_incomplete_outcomes_then_recovers() {
    let temporary = tempfile::tempdir().expect("terminal fixtures");
    let path = temporary.path().join("terminal.json");
    let genuine = terminal();
    write_terminal(&path, &genuine);
    assert!(read_terminal(&path).is_ok());
    for status in [
        Status::Incomplete,
        Status::Failed,
        Status::Cancelled,
        Status::Unavailable,
    ] {
        let mut changed = genuine.clone();
        changed.status = status;
        write_terminal(&path, &changed);
        assert!(
            read_terminal(&path).is_err(),
            "accepted terminal {status:?}"
        );
    }
    for fault in 0..17 {
        let mut changed = genuine.clone();
        match fault {
            0 => changed.cleanup_complete = false,
            1 => changed.source_gates = 1,
            2 => changed.target_owners = 5,
            3 => changed.userlands = 1,
            4 => {
                changed.proofs.pop();
            }
            5 => {
                changed.stages.pop();
            }
            6 => changed.stages[1].name = "target-admission".to_owned(),
            7 => changed.proofs[1].name = "release-source".to_owned(),
            8 => changed.manifest_sha256 = None,
            9 => changed.executable = None,
            10 => changed.verifier.byte_length = 0,
            11 => changed.target_policy_sha256 = "9".repeat(64),
            12 => changed.workload = "smoke-only".to_owned(),
            13 => changed.completed_unix_nanoseconds = Some(0),
            14 => changed.failure = Some("cleanup failed".to_owned()),
            15 => changed.assets[1].name = "install.sh".to_owned(),
            _ => changed.phase = "installation".to_owned(),
        }
        write_terminal(&path, &changed);
        assert!(
            read_terminal(&path).is_err(),
            "accepted terminal fault {fault}"
        );
    }
    for status in [
        process::ProcessStatus::Failed,
        process::ProcessStatus::Timeout,
        process::ProcessStatus::Unavailable,
        process::ProcessStatus::OutputExhausted,
        process::ProcessStatus::InfrastructureFailure,
        process::ProcessStatus::Signaled,
    ] {
        let mut changed = genuine.clone();
        changed.stages[0].process.status = status;
        write_terminal(&path, &changed);
        assert!(
            read_terminal(&path).is_err(),
            "accepted failed child {status:?}"
        );
    }
    let mut changed = genuine.clone();
    changed.stages[1].process.reason = Some("descendant cleanup failed".to_owned());
    write_terminal(&path, &changed);
    assert!(read_terminal(&path).is_err());
    changed = genuine.clone();
    changed.stages[1].process.stdout_limit_exhausted = true;
    write_terminal(&path, &changed);
    assert!(read_terminal(&path).is_err());
    write_terminal(&path, &genuine);
    assert!(read_terminal(&path).is_ok());
}

#[test]
fn candidate_terminal_binds_exact_authenticated_producer_and_verifier() {
    let temporary = tempfile::tempdir().expect("producer fixture");
    let path = temporary.path().join("terminal.json");
    let genuine = terminal();
    write_terminal(&path, &genuine);
    let read = || controller_content(&path, &"1".repeat(40), &"2".repeat(64), 101, 2);
    assert!(read().is_ok());
    assert!(controller_content(&path, &"4".repeat(40), &"2".repeat(64), 101, 2).is_err());
    assert!(controller_content(&path, &"1".repeat(40), &"4".repeat(64), 101, 2).is_err());
    assert!(controller_content(&path, &"1".repeat(40), &"2".repeat(64), 102, 2).is_err());
    assert!(controller_content(&path, &"1".repeat(40), &"2".repeat(64), 101, 3).is_err());
    for fault in 0..6 {
        let mut changed = genuine.clone();
        match fault {
            0 => changed.source_commit = "4".repeat(40),
            1 => changed.controller_source_commit = "4".repeat(40),
            2 => changed.producer.github_actions = None,
            3 => changed.producer.repository = Some("foreign/lkjscript".to_owned()),
            4 => changed.producer.workflow = Some("Pull Request".to_owned()),
            _ => {
                changed.verifier.sha256 = Sha256Digest::new("4".repeat(64)).expect("wrong verifier")
            }
        }
        write_terminal(&path, &changed);
        assert!(read().is_err(), "accepted producer identity fault {fault}");
    }
    write_terminal(&path, &genuine);
    let admitted = read().expect("restored producer fixture");
    assert_eq!(admitted.source_commit, "1".repeat(40));
    assert_eq!(admitted.assets.len(), 3);
    fs::remove_file(&path).expect("simulate unavailable handoff");
    assert!(read().is_err());
}

#[test]
fn candidate_inventory_rejects_missing_extra_linked_and_oversized_assets() {
    let temporary = tempfile::tempdir().expect("asset fixtures");
    let root = temporary.path().join("assets");
    fs::create_dir(&root).expect("asset directory");
    let names = [
        archive::ARCHIVE_NAME,
        archive::CHECKSUM_NAME,
        bootstrap::NAME,
    ];
    for name in names {
        fs::write(root.join(name), b"original").expect("asset fixture");
    }
    let accepted = assets(&root).expect("exact inventory");
    fs::write(root.join("extra"), b"unselected").expect("extra asset");
    assert!(assets(&root).is_err());
    fs::remove_file(root.join("extra")).expect("remove owned extra");
    fs::remove_file(root.join(bootstrap::NAME)).expect("missing asset");
    assert!(assets(&root).is_err());
    let outside = temporary.path().join("sentinel");
    fs::write(&outside, b"sentinel").expect("outside sentinel");
    symlink(&outside, root.join(bootstrap::NAME)).expect("linked asset");
    assert!(assets(&root).is_err());
    assert_eq!(fs::read(&outside).expect("preserved sentinel"), b"sentinel");
    fs::remove_file(root.join(bootstrap::NAME)).expect("remove owned link");
    fs::write(
        root.join(bootstrap::NAME),
        vec![b'x'; bootstrap::MAXIMUM_BYTES as usize + 1],
    )
    .expect("oversized bootstrap");
    assert!(assets(&root).is_err());
    fs::write(root.join(bootstrap::NAME), b"original").expect("restore bootstrap");
    fs::write(root.join(archive::CHECKSUM_NAME), b"originaL").expect("one byte mutation");
    assert_ne!(assets(&root).expect("mutated inventory identity"), accepted);
    fs::write(root.join(archive::CHECKSUM_NAME), b"original").expect("restore checksum");
    assert_eq!(assets(&root).expect("restored inventory"), accepted);
}

#[test]
fn candidate_real_stage_retains_failure_timeout_and_cancellation_without_acceptance() {
    for case in ["failed", "timeout", "cancelled"] {
        let temporary = tempfile::tempdir().expect("owned stage fixture");
        let root = temporary.path();
        let options = Options {
            assets: root.join("unused-assets"),
            source_receipt: root.join("unused-source"),
            build_receipt: root.join("unused-build"),
            verifier_identity: root.join("unused-verifier"),
            evidence_root: root.to_path_buf(),
            output: root.join("terminal.json"),
        };
        let mut observed = terminal();
        observed.status = Status::Incomplete;
        observed.cleanup_complete = false;
        observed.stages.clear();
        let control = process::ProcessControl::default();
        if case == "cancelled" {
            control.kill();
        }
        let script = if case == "failed" {
            "exit 37"
        } else {
            "exec sleep 5"
        };
        assert!(
            stage(
                &options,
                root,
                Path::new("/bin/sh"),
                &mut observed,
                &control,
                "target-admission",
                vec!["-c".to_owned(), script.to_owned()],
                Duration::from_millis(30)
            )
            .is_err()
        );
        assert_eq!(observed.stages.len(), 1);
        assert!(!passed(&observed.stages[0].process));
        assert!(!observed.cleanup_complete);
        assert!(read_terminal(&options.output).is_err());
        assert!(root.join("target-admission.stdout.log").is_file());
        assert!(root.join("target-admission.stderr.log").is_file());
    }
}
