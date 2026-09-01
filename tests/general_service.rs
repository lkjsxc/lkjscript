#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "the black-box test harness uses panic-on-failure assertions"
)]

use lkjscript::platform::{AdapterDescriptor, DeploymentDescriptor, decode_deployment};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APPLICATION: &str = "applications/lkjournal";
const ARTIFACT: &str = "applications/lkjournal/generated/lkjournal.lkja";

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn descriptor(name: &str) -> DeploymentDescriptor {
    let bytes = std::fs::read(repository().join(APPLICATION).join(name))
        .expect("read maintained deployment descriptor");
    decode_deployment(&bytes).expect("strict maintained deployment descriptor")
}

fn run_isolated(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(arguments)
        .current_dir(directory)
        .env_clear()
        .env("LANG", "C")
        .env("LKJOURNAL_BOOTSTRAP_TOKEN", "test-bootstrap-token")
        .output()
        .expect("run isolated service command")
}

fn stage_current_artifact(directory: &Path) -> PathBuf {
    let generated = directory.join("generated");
    std::fs::create_dir(&generated).expect("create isolated artifact directory");
    let artifact = generated.join("lkjournal.lkja");
    std::fs::copy(repository().join(ARTIFACT), &artifact).expect("copy maintained artifact");
    artifact
}

#[test]
fn maintained_descriptors_name_the_current_artifact_and_exact_targets() {
    let service = descriptor("service.deployment.json");
    let worker = descriptor("worker.deployment.json");
    for name in ["service.deployment.json", "worker.deployment.json"] {
        let bytes = std::fs::read(repository().join(APPLICATION).join(name))
            .expect("read maintained descriptor bytes");
        assert!(!String::from_utf8_lossy(&bytes).contains("contract_version"));
    }
    assert_eq!(service.artifact, "generated/lkjournal.lkja");
    assert_eq!(worker.artifact, "generated/lkjournal.lkja");
    assert_eq!(service.target, "serve");
    assert_eq!(worker.target, "work");
    assert!(service.http.is_some() && service.worker.is_none());
    assert!(worker.http.is_none() && worker.worker.is_some());
}

#[test]
fn removed_descriptor_versions_reject_before_artifact_or_live_effects() {
    let temporary = tempfile::tempdir().expect("isolated predecessor descriptor");
    for (name, descriptor_name, operation, nested) in [
        ("top", "service.deployment.json", "serve", None),
        (
            "runtime",
            "service.deployment.json",
            "serve",
            Some("runtime"),
        ),
        ("http", "service.deployment.json", "serve", Some("http")),
        ("worker", "worker.deployment.json", "worker", Some("worker")),
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(repository().join(APPLICATION).join(descriptor_name))
                .expect("read current descriptor"),
        )
        .expect("current descriptor JSON");
        value["artifact"] = serde_json::json!("missing-artifact.lkja");
        match nested {
            Some(owner) => value[owner]["contract_version"] = serde_json::json!(1),
            None => value["contract_version"] = serde_json::json!(1),
        }
        let descriptor_path = temporary.path().join(format!("{name}.json"));
        std::fs::write(
            &descriptor_path,
            serde_json::to_vec(&value).expect("predecessor descriptor JSON"),
        )
        .expect("write predecessor descriptor");
        let output = run_isolated(
            temporary.path(),
            &[
                operation,
                "--deployment",
                descriptor_path.to_str().expect("UTF-8 descriptor path"),
            ],
        );
        assert_eq!(output.status.code(), Some(2), "{name}");
        assert!(output.stderr.is_empty(), "{name}");
        let stdout = String::from_utf8(output.stdout).expect("diagnostic UTF-8");
        assert!(stdout.contains("deployment_json"), "{name}: {stdout}");
        assert!(!stdout.contains("deployment_read"), "{name}: {stdout}");
        let failure: serde_json::Value = serde_json::from_str(&stdout).expect("failure event JSON");
        assert!(failure.get("contract_version").is_none(), "{name}");
        assert!(!stdout.contains("\"event\":\"ready\""), "{name}: {stdout}");
        assert_eq!(
            std::fs::read_dir(temporary.path())
                .expect("read isolated directory")
                .count(),
            1,
            "{name}"
        );
        std::fs::remove_file(&descriptor_path).expect("remove predecessor descriptor");
    }
}

#[test]
fn maintained_descriptors_cover_every_selected_component_requirement() {
    let service = descriptor("service.deployment.json");
    let worker = descriptor("worker.deployment.json");
    assert_eq!(
        service
            .grants
            .iter()
            .map(|grant| grant.requirement.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "bootstrap",
            "clock",
            "config",
            "data",
            "identifiers",
            "jobs",
            "objects",
            "passwords",
            "random",
            "streams",
        ])
    );
    assert_eq!(
        worker
            .grants
            .iter()
            .map(|grant| grant.requirement.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["clock", "jobs"])
    );
}

#[test]
fn checked_in_service_artifact_uses_only_bundle_contract_11() {
    let bytes = std::fs::read(repository().join(ARTIFACT)).expect("read maintained artifact");
    assert_eq!(bytes.get(..8), Some(b"LKJART11".as_slice()));
    assert_ne!(bytes.get(..8), Some(b"LKJART10".as_slice()));
    assert_ne!(bytes.get(..8), Some(b"LKJART04".as_slice()));
    assert!(bytes.len() < 128 * 1024 * 1024);
}

#[test]
fn both_public_service_commands_reject_minimal_artifact4_before_readiness() {
    let temporary = tempfile::tempdir().expect("isolated predecessor deployment");
    let artifact = temporary.path().join("predecessor.lkja");
    let mut predecessor = vec![0_u8; 100];
    predecessor[..8].copy_from_slice(b"LKJART04");
    std::fs::write(&artifact, predecessor).expect("write minimal predecessor marker");
    for (operation, source) in [
        ("serve", "service.deployment.json"),
        ("worker", "worker.deployment.json"),
    ] {
        let mut descriptor = descriptor(source);
        descriptor.artifact = "predecessor.lkja".to_owned();
        let path = temporary.path().join(format!("{operation}.json"));
        std::fs::write(
            &path,
            serde_json::to_vec(&descriptor).expect("encode isolated descriptor"),
        )
        .expect("write isolated descriptor");
        let output = run_isolated(
            temporary.path(),
            &[
                operation,
                "--deployment",
                path.to_str().expect("UTF-8 descriptor path"),
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).expect("diagnostic UTF-8");
        assert!(stdout.contains("artifact_bundle_contract"), "{stdout}");
        assert!(!stdout.contains("\"event\":\"ready\""), "{stdout}");
    }
}

#[test]
fn isolated_current_deployment_reaches_adapter_preflight_without_project_authority() {
    let temporary = tempfile::tempdir().expect("isolated current deployment");
    let artifact = stage_current_artifact(temporary.path());
    std::fs::create_dir_all(temporary.path().join("state/objects"))
        .expect("named object host resource");
    let before = blake3::hash(&std::fs::read(&artifact).expect("read staged artifact"));
    let path = temporary.path().join("service.deployment.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&descriptor("service.deployment.json"))
            .expect("encode isolated current descriptor"),
    )
    .expect("write isolated current descriptor");

    let output = run_isolated(
        temporary.path(),
        &[
            "serve",
            "--deployment",
            path.to_str().expect("UTF-8 descriptor path"),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("diagnostic UTF-8");
    assert!(
        stdout.contains("normalized_deployment_directory_missing"),
        "{stdout}"
    );
    assert!(!stdout.contains("\"event\":\"ready\""), "{stdout}");
    assert!(!temporary.path().join(".lkjscript-project.json").exists());
    assert_eq!(
        blake3::hash(&std::fs::read(&artifact).expect("reread staged artifact")),
        before
    );
}

#[test]
fn standalone_preparation_rejects_target_runner_and_grant_mismatches_before_readiness() {
    let temporary = tempfile::tempdir().expect("isolated invalid deployments");
    stage_current_artifact(temporary.path());
    let service = descriptor("service.deployment.json");
    let mut cases = Vec::new();

    let mut missing_target = service.clone();
    missing_target.target = "foreign".to_owned();
    cases.push((
        "missing-target",
        missing_target,
        "deployment_target_missing",
    ));

    let mut wrong_runner = service.clone();
    wrong_runner.http = None;
    cases.push(("wrong-runner", wrong_runner, "deployment_http_incomplete"));

    let mut missing_grant = service.clone();
    missing_grant
        .grants
        .retain(|grant| grant.requirement != "config");
    cases.push(("missing-grant", missing_grant, "deployment_grant_missing"));

    let mut foreign_grant = service.clone();
    let mut extra = foreign_grant.grants[0].clone();
    extra.requirement = "foreign".to_owned();
    foreign_grant.grants.push(extra);
    cases.push(("foreign-grant", foreign_grant, "deployment_grant_foreign"));

    let mut wrong_interface = service;
    wrong_interface
        .grants
        .iter_mut()
        .find(|grant| grant.requirement == "config")
        .expect("configuration grant")
        .adapter = AdapterDescriptor::WallClock;
    cases.push((
        "wrong-interface",
        wrong_interface,
        "deployment_adapter_interface",
    ));

    for (name, descriptor, code) in cases {
        let path = temporary.path().join(format!("{name}.json"));
        std::fs::write(
            &path,
            serde_json::to_vec(&descriptor).expect("encode invalid descriptor"),
        )
        .expect("write invalid descriptor");
        let output = run_isolated(
            temporary.path(),
            &[
                "serve",
                "--deployment",
                path.to_str().expect("UTF-8 descriptor path"),
            ],
        );
        assert!(!output.status.success(), "{name}");
        assert!(output.stderr.is_empty(), "{name}");
        let stdout = String::from_utf8(output.stdout).expect("diagnostic UTF-8");
        assert!(stdout.contains(code), "{name}: {stdout}");
        assert!(!stdout.contains("\"event\":\"ready\""), "{name}: {stdout}");
    }
}

#[test]
fn deployment_decoder_remains_strict_and_rejects_trailing_input() {
    let mut bytes = std::fs::read(
        repository()
            .join(APPLICATION)
            .join("service.deployment.json"),
    )
    .expect("read maintained descriptor");
    bytes.extend_from_slice(b"{}\n");
    let error = decode_deployment(&bytes).expect_err("trailing deployment input");
    assert_eq!(error.code, "deployment_trailing_json");
}

#[test]
fn generic_native_platform_contains_no_service_product_vocabulary() {
    let root = repository().join("src/platform");
    for entry in std::fs::read_dir(root).expect("platform directory") {
        let entry = entry.expect("platform entry");
        if entry.path().extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(entry.path()).expect("platform source");
        for forbidden in [
            "lkjournal_",
            "resource_owner_denied",
            "initial_actor_denied",
            "route_missing",
        ] {
            assert!(
                !source.contains(forbidden),
                "generic adapter contains product vocabulary {forbidden}"
            );
        }
    }
}
