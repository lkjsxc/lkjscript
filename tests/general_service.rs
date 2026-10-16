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
    let live = descriptor("live.deployment.json");
    for name in [
        "service.deployment.json",
        "worker.deployment.json",
        "live.deployment.json",
    ] {
        let bytes = std::fs::read(repository().join(APPLICATION).join(name))
            .expect("read maintained descriptor bytes");
        assert!(!String::from_utf8_lossy(&bytes).contains("contract_version"));
    }
    assert_eq!(service.artifact, "generated/lkjournal.lkja");
    assert_eq!(worker.artifact, "generated/lkjournal.lkja");
    assert_eq!(service.target, "serve");
    assert_eq!(worker.target, "work");
    assert_eq!(live.target, "lkjournal-live-1");
    assert!(service.http.is_some() && service.worker.is_none());
    assert!(worker.http.is_none() && worker.worker.is_some());
    assert!(
        live.http.is_none()
            && live.session.is_some()
            && live.worker.is_none()
            && live.listen.is_some()
    );
}

#[test]
fn unknown_descriptor_fields_reject_before_artifact_or_live_effects() {
    let temporary = tempfile::tempdir().expect("isolated invalid descriptor");
    for (name, descriptor_name, operation, nested) in [
        ("top", "service.deployment.json", "serve", None),
        (
            "runtime",
            "service.deployment.json",
            "serve",
            Some("runtime"),
        ),
        ("http", "service.deployment.json", "serve", Some("http")),
        ("session", "live.deployment.json", "serve", Some("session")),
        ("worker", "worker.deployment.json", "worker", Some("worker")),
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(repository().join(APPLICATION).join(descriptor_name))
                .expect("read current descriptor"),
        )
        .expect("current descriptor JSON");
        value["artifact"] = serde_json::json!("missing-artifact.lkja");
        match nested {
            Some(owner) => value[owner]["unknown_policy"] = serde_json::json!(true),
            None => value["unknown_policy"] = serde_json::json!(true),
        }
        let descriptor_path = temporary.path().join(format!("{name}.json"));
        std::fs::write(
            &descriptor_path,
            serde_json::to_vec(&value).expect("invalid descriptor JSON"),
        )
        .expect("write invalid descriptor");
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
        std::fs::remove_file(&descriptor_path).expect("remove invalid descriptor");
    }
}

#[test]
fn maintained_descriptors_cover_every_selected_component_requirement() {
    let service = descriptor("service.deployment.json");
    let worker = descriptor("worker.deployment.json");
    let live = descriptor("live.deployment.json");
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
    assert_eq!(
        live.grants
            .iter()
            .map(|grant| grant.requirement.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["clock", "data", "streams"])
    );
    assert!(live.configuration.is_empty());
    assert!(live.secrets.is_empty());
}

#[test]
fn checked_in_service_artifact_uses_current_bundle_contract() {
    let bytes = std::fs::read(repository().join(ARTIFACT)).expect("read maintained artifact");
    assert_eq!(bytes.get(..8), Some(b"LKJARTFC".as_slice()));
    assert_eq!(
        bytes.get(8..10),
        Some(
            lkjscript::platform::canonical_schema::SCHEMA_EPOCH
                .to_le_bytes()
                .as_slice()
        ),
    );
    lkjscript::platform::contributor::strict_artifact_admission_probe(&bytes)
        .expect("strict current service artifact admission");
    assert!(bytes.len() < 128 * 1024 * 1024);
}

#[test]
fn every_public_resident_command_rejects_invalid_current_artifacts_before_readiness() {
    let current = std::fs::read(repository().join(ARTIFACT)).expect("read maintained artifact");
    lkjscript::platform::contributor::strict_artifact_admission_probe(&current)
        .expect("healthy current artifact before independent mutations");
    let temporary = tempfile::tempdir().expect("isolated invalid artifact deployments");
    for (fault, code, class, exit) in [
        ("schema", "artifact_bundle_schema", "source", 2),
        ("checksum", "artifact_bundle_checksum", "corrupt", 5),
    ] {
        let artifact = temporary.path().join(format!("{fault}.lkja"));
        let mut invalid = current.clone();
        match fault {
            "schema" => invalid[8] ^= 1,
            "checksum" => {
                let checksum = invalid.len() - 40;
                invalid[checksum] ^= 1;
            }
            _ => unreachable!(),
        }
        std::fs::write(&artifact, &invalid).expect("write mutated current artifact");
        for (name, operation, source) in [
            ("http", "serve", "service.deployment.json"),
            ("interactive", "serve", "live.deployment.json"),
            ("worker", "worker", "worker.deployment.json"),
        ] {
            let mut descriptor = descriptor(source);
            descriptor.artifact = artifact
                .file_name()
                .expect("invalid artifact name")
                .to_string_lossy()
                .into_owned();
            let path = temporary.path().join(format!("{name}-{fault}.json"));
            let descriptor_bytes =
                serde_json::to_vec(&descriptor).expect("encode isolated descriptor");
            std::fs::write(&path, &descriptor_bytes).expect("write isolated descriptor");
            let output = run_isolated(
                temporary.path(),
                &[
                    operation,
                    "--deployment",
                    path.to_str().expect("UTF-8 descriptor path"),
                ],
            );
            assert_eq!(output.status.code(), Some(exit), "{name}/{fault}");
            assert!(output.stderr.is_empty(), "{name}/{fault}");
            let stdout = String::from_utf8(output.stdout).expect("diagnostic UTF-8");
            let failure: serde_json::Value =
                serde_json::from_str(&stdout).expect("one failure event before readiness");
            assert_eq!(failure["ok"], false, "{name}/{fault}: {stdout}");
            assert_eq!(failure["status"], "failure", "{name}/{fault}: {stdout}");
            assert_eq!(failure["error"]["code"], code, "{name}/{fault}: {stdout}");
            assert_eq!(failure["error"]["class"], class, "{name}/{fault}: {stdout}");
            assert!(!stdout.contains("\"event\":\"ready\""), "{stdout}");
            assert_eq!(std::fs::read(&artifact).unwrap(), invalid, "{name}/{fault}");
            assert_eq!(
                std::fs::read(&path).unwrap(),
                descriptor_bytes,
                "{name}/{fault}"
            );
            assert_eq!(
                std::fs::read_dir(temporary.path()).unwrap().count(),
                2,
                "{name}/{fault}: artifact admission must precede adapter state creation",
            );
            std::fs::remove_file(&path).expect("remove isolated descriptor");
        }
        std::fs::remove_file(&artifact).expect("remove mutated current artifact");
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
