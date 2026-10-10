use super::super::model::{Gate, InputSnapshot};
use super::*;
use serde_json::json;

// Extend the existing independently executed two-gate original-reader fixture.
// Rehash every changed command/log/dependency consistently: origin, not merely
// a broken digest, must be the reason an artifact-free Cargo claim is rejected.
pub(super) fn rejects_artifact_free_cargo_claim_and_recovers(
    baseline: &CheckReceipt,
    repository: &Path,
    root: &Path,
    source: &InputSnapshot,
) {
    let mut producer = Gate::new(
        "first",
        vec![
            "cargo".into(),
            "build".into(),
            "--message-format=json".into(),
        ],
    );
    let output = repository.join(&baseline.gates[0].outputs[0].path);
    producer.required_outputs.push(output.clone());
    let mut consumer = Gate::new("second", vec!["/bin/true".into()]);
    consumer.dependencies.push("first".into());
    let registry =
        registry::GateRegistry::new(vec![producer, consumer]).expect("Cargo claim registry");
    let requested = vec!["second".to_owned()];
    let selected = registry.closure(&requested).expect("claim closure");
    let runtime = snapshot::runtime_identity(
        repository,
        registry
            .input_commands(&selected)
            .expect("claim input commands"),
    )
    .expect("actual tool identity");
    evidence::publish_json(
        &root.join("dag.json"),
        &registry
            .manifest(&requested, &selected, 1)
            .expect("claim DAG"),
    )
    .expect("consistent DAG");
    let artifact = json!({"reason":"compiler-artifact", "package_id":"owned-fixture",
        "manifest_path":repository.join("Cargo.toml"),
        "target":{"name":output.file_name().and_then(|name| name.to_str()).expect("UTF-8 target name"),"kind":["bin"]},
        "profile":{"test":false}, "executable":output, "filenames":[output], "fresh":true});
    let terminal = json!({"reason":"build-finished","success":true});
    for has_artifact in [true, false, true] {
        let log = if has_artifact {
            format!("{artifact}\n{terminal}\n")
        } else {
            format!("{terminal}\n")
        };
        let stdout = root.join("first.stdout.log");
        fs::write(&stdout, log).expect("consistent controlled claim");
        let mut claim = baseline.clone();
        claim.runtime = Some(runtime.clone());
        claim.profile_definition_digest = registry
            .profile_digest("release-source", &requested)
            .expect("consistent profile identity");
        claim.gates[0].process.as_mut().expect("process").stdout =
            evidence::proof(&stdout, evidence::relative(repository, &stdout))
                .expect("claim log proof");
        for index in 0..claim.gates.len() {
            let gate = registry.gate(&claim.gates[index].name).expect("claim gate");
            let dependencies: BTreeMap<_, _> = claim.gates[..index]
                .iter()
                .filter(|prior| gate.dependencies.contains(&prior.name))
                .map(|prior| (prior.name.clone(), prior.clone()))
                .collect();
            let observed = &mut claim.gates[index];
            observed.command = gate.command.clone();
            observed.input_fingerprint =
                executor::gate_fingerprint(repository, gate, source, &runtime, &dependencies)
                    .expect("consistent input identity");
            observed.evidence_digest = cache::gate_evidence_digest(
                &gate.name,
                &observed.input_fingerprint,
                observed.process.as_ref().expect("process"),
                &observed.outputs,
            )
            .expect("consistent evidence identity");
        }
        let result = admit(
            &root.join("receipt.json"),
            repository,
            &source.git_head,
            &claim,
            &registry,
            &requested,
        );
        if has_artifact {
            result.expect("complete consistent claim and restoration");
        } else {
            let error =
                result.expect_err("artifact-free claim cannot rely on a stale regular output");
            assert!(
                error
                    .message()
                    .contains("missing exact root-package executable artifact"),
                "{error}"
            );
        }
    }
}
