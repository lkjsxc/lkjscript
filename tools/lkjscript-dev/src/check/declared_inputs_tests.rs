use super::*;
use crate::check::model::GateInput;
use std::fs;

fn run(root: &Path, gate: &Gate, inputs: Vec<FileProof>) -> Result<GateReceipt, DevError> {
    let directory = root.join(&gate.name);
    fs::create_dir(&directory)?;
    execute_fresh_inner(
        FreshInput {
            repository: root,
            gate,
            inputs,
            program: Some(Path::new("/bin/sh")),
            fingerprint: VerificationDigest::of(b"declared-input-process-fixture"),
            stdout_path: directory.join("stdout.log"),
            stderr_path: directory.join("stderr.log"),
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
        None,
    )
}

fn fixture() -> (tempfile::TempDir, Gate, BTreeMap<String, GateReceipt>) {
    let root = tempfile::tempdir().expect("declared candidate fixture");
    let candidate = root.path().join("optimized-candidate");
    let mut producer = Gate::new(
        "build",
        vec![
            "/bin/sh".into(),
            "-c".into(),
            r#"printf '%s\n' "printf '%s\n' selected-optimized-candidate" > "$1""#.into(),
            "fixture".into(),
            candidate.display().to_string(),
        ],
    );
    producer.required_outputs.push(candidate.clone());
    let receipt = run(root.path(), &producer, Vec::new()).expect("real candidate producer");
    assert_eq!(receipt.status, GateStatus::Passed);
    let mut consumer = Gate::new("lifecycle", vec![
        "/bin/sh".into(), "-c".into(),
        "test \"$LKJSCRIPT_RELEASE_CANDIDATE\" = \"$1\" && /bin/sh \"$LKJSCRIPT_RELEASE_CANDIDATE\"".into(),
        "fixture".into(), candidate.display().to_string(),
    ]);
    consumer.cacheable = false;
    consumer.dependencies.push("build".into());
    consumer.required_inputs.push(GateInput {
        path: candidate.clone(),
        producer: "build".into(),
    });
    consumer.environment.insert(
        "LKJSCRIPT_RELEASE_CANDIDATE".into(),
        candidate.display().to_string(),
    );
    (root, consumer, BTreeMap::from([("build".into(), receipt)]))
}

#[test]
fn candidate_environment_executes_the_admitted_input_without_claiming_it_as_an_output() {
    let (root, gate, dependencies) = fixture();
    let inputs = expected_input_proofs(root.path(), &gate, &dependencies).expect("admitted input");
    let receipt = run(root.path(), &gate, inputs.clone()).expect("candidate lifecycle");
    assert_eq!(receipt.status, GateStatus::Passed);
    assert_eq!(receipt.inputs, inputs);
    assert!(receipt.outputs.is_empty());
    assert_eq!(
        fs::read(root.path().join("lifecycle/stdout.log")).unwrap(),
        b"selected-optimized-candidate\n"
    );
}

#[test]
fn changed_candidate_is_rejected_before_execution_and_after_mutation_by_the_consumer() {
    let (root, mut gate, dependencies) = fixture();
    let inputs = expected_input_proofs(root.path(), &gate, &dependencies).expect("admitted input");
    let candidate = &gate.required_inputs[0].path;
    let original = fs::read(candidate).unwrap();
    fs::write(candidate, b"exit 0\n").unwrap();
    assert!(run(root.path(), &gate, inputs.clone()).is_err());
    assert!(!root.path().join("lifecycle/stdout.log").exists());
    fs::write(candidate, original).unwrap();
    gate.name = "mutates-input".into();
    gate.command[2] = "printf substituted > \"$LKJSCRIPT_RELEASE_CANDIDATE\"".into();
    let receipt = run(root.path(), &gate, inputs).expect("joined mutating consumer");
    assert_eq!(
        receipt.process.as_ref().unwrap().status,
        ProcessStatus::Passed
    );
    assert_eq!(receipt.status, GateStatus::Failed);
    assert!(
        receipt
            .reason
            .as_deref()
            .unwrap()
            .starts_with("declared_input_binding:")
    );
}

#[test]
fn candidate_fingerprints_bind_environment_and_the_producer_artifact_digest() {
    let (root, mut gate, mut dependencies) = fixture();
    let source = InputSnapshot {
        digest: VerificationDigest::of(b"fixture source"),
        git_head: "0".repeat(40),
        cargo_lock_digest: VerificationDigest::of(b"fixture lock"),
        file_count: 0,
        total_bytes: 0,
        entries: Vec::new(),
    };
    let runtime =
        snapshot::runtime_identity(root.path(), [("/bin/sh".into(), "/bin/sh".into())]).unwrap();
    let baseline = gate_fingerprint(root.path(), &gate, &source, &runtime, &dependencies).unwrap();
    let original = gate.environment["LKJSCRIPT_RELEASE_CANDIDATE"].clone();
    gate.environment
        .insert("LKJSCRIPT_RELEASE_CANDIDATE".into(), "substituted".into());
    assert_ne!(
        baseline,
        gate_fingerprint(root.path(), &gate, &source, &runtime, &dependencies).unwrap()
    );
    gate.environment
        .insert("LKJSCRIPT_RELEASE_CANDIDATE".into(), original);
    dependencies.get_mut("build").unwrap().outputs[0].digest =
        Some(VerificationDigest::of(b"substitute bytes"));
    assert_ne!(
        baseline,
        gate_fingerprint(root.path(), &gate, &source, &runtime, &dependencies).unwrap()
    );
    dependencies.get_mut("build").unwrap().status = GateStatus::Failed;
    assert!(expected_input_proofs(root.path(), &gate, &dependencies).is_err());
}

#[test]
fn registry_rejects_unbound_candidate_environment_and_input_output_aliases() {
    let (root, mut consumer, _) = fixture();
    let mut producer = Gate::new("build", vec!["/bin/true".into()]);
    producer
        .required_outputs
        .push(root.path().join("optimized-candidate"));
    assert!(GateRegistry::new(vec![producer.clone(), consumer.clone()]).is_ok());
    consumer
        .required_outputs
        .push(consumer.required_inputs[0].path.clone());
    assert!(GateRegistry::new(vec![producer.clone(), consumer.clone()]).is_err());
    consumer.required_outputs.clear();
    consumer.environment.insert(
        "LKJSCRIPT_RELEASE_CANDIDATE".into(),
        root.path().join("wrong-candidate").display().to_string(),
    );
    assert!(GateRegistry::new(vec![producer.clone(), consumer.clone()]).is_err());
    consumer.environment.clear();
    consumer
        .environment
        .insert("RUSTC_WRAPPER".into(), "/bin/true".into());
    assert!(GateRegistry::new(vec![producer, consumer]).is_err());
}
