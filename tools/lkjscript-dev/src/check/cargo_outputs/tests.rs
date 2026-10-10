use super::*;
use serde_json::json;

pub(super) fn artifact() -> Value {
    json!({"reason":"compiler-artifact", "package_id":"path+file:///fixture#lkjscript@0.0.0",
        "manifest_path":"/fixture/Cargo.toml", "target":{"name":"lkjscript","kind":["bin"]},
        "profile":{"test":false}, "executable":"/fixture/target/release/lkjscript",
        "filenames":["/fixture/target/release/lkjscript"], "fresh":false})
}
pub(super) fn finish() -> Value {
    json!({"reason":"build-finished","success":true})
}
pub(super) fn transcript(messages: &[Value]) -> Vec<u8> {
    messages
        .iter()
        .map(|value| format!("{value}\n"))
        .collect::<String>()
        .into_bytes()
}
fn check(bytes: &[u8]) -> Result<(), DevError> {
    records(
        bytes,
        Path::new("/fixture/Cargo.toml"),
        &[PathBuf::from("/fixture/target/release/lkjscript")],
    )
}

#[test]
fn both_compiled_and_cargo_fresh_artifacts_are_admitted() {
    for fresh in [false, true] {
        let mut message = artifact();
        message["fresh"] = json!(fresh);
        assert!(check(&transcript(&[message, finish()])).is_ok());
    }
}

#[test]
fn missing_and_failed_completion_are_not_acceptance() {
    for messages in [
        vec![],
        vec![artifact()],
        vec![finish()],
        vec![
            artifact(),
            json!({"reason":"build-finished","success":false}),
        ],
        vec![
            artifact(),
            json!({"reason":"build-finished","success":"true"}),
        ],
    ] {
        assert!(check(&transcript(&messages)).is_err(), "{messages:?}");
    }
}

#[test]
fn exact_manifest_target_kind_and_non_test_profile_are_required() {
    for (pointer, value) in [
        ("/manifest_path", json!("/other/Cargo.toml")),
        ("/target/name", json!("another")),
        ("/target/kind", json!(["lib"])),
        ("/target/kind", json!(["bin", "example"])),
        ("/profile/test", json!(true)),
        ("/profile/test", Value::Null),
        ("/profile/test", json!("false")),
    ] {
        let mut message = artifact();
        *message.pointer_mut(pointer).expect("fixture field") = value;
        assert!(
            check(&transcript(&[message, finish()])).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn different_or_incomplete_executable_identity_is_refused() {
    for (pointer, value) in [
        ("/executable", json!("/elsewhere/lkjscript")),
        ("/executable", Value::Null),
        ("/filenames", json!([])),
        ("/filenames", json!(["/elsewhere/lkjscript"])),
        (
            "/filenames",
            json!(["/fixture/target/release/lkjscript", 4]),
        ),
        ("/fresh", Value::Null),
        ("/fresh", json!("true")),
        ("/package_id", json!("")),
    ] {
        let mut message = artifact();
        *message.pointer_mut(pointer).expect("fixture field") = value;
        assert!(
            check(&transcript(&[message, finish()])).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn duplicate_and_conflicting_artifacts_cannot_be_hidden() {
    let mut conflicting = artifact();
    conflicting["executable"] = json!("/elsewhere/lkjscript");
    for messages in [
        vec![artifact(), artifact(), finish()],
        vec![artifact(), conflicting.clone(), finish()],
        vec![conflicting, artifact(), finish()],
    ] {
        assert!(check(&transcript(&messages)).is_err());
    }
}

#[test]
fn a_test_harness_or_other_package_cannot_substitute_for_the_product() {
    let mut harness = artifact();
    harness["profile"]["test"] = json!(true);
    harness["executable"] = json!("/fixture/target/release/deps/lkjscript-test");
    let mut other = artifact();
    other["manifest_path"] = json!("/dependency/Cargo.toml");
    assert!(check(&transcript(&[harness.clone(), other.clone(), finish()])).is_err());
    assert!(check(&transcript(&[harness, other, artifact(), finish()])).is_ok());
}

#[test]
fn post_build_test_output_cannot_supply_a_missing_artifact() {
    assert!(check(&transcript(&[finish(), artifact()])).is_err());
    let mut bytes = b"non-JSON build output\n".to_vec();
    bytes.extend(transcript(&[artifact(), finish()]));
    bytes.extend(b"{invalid test output\ntest result: ok\n");
    assert!(check(&bytes).is_ok());
}

#[test]
fn malformed_build_json_and_missing_artifact_fields_are_refused() {
    assert!(check(b"{broken\n").is_err());
    for field in [
        "manifest_path",
        "target",
        "profile",
        "executable",
        "filenames",
        "fresh",
        "package_id",
    ] {
        let mut message = artifact();
        message
            .as_object_mut()
            .expect("artifact object")
            .remove(field);
        assert!(check(&transcript(&[message, finish()])).is_err(), "{field}");
    }
}

#[test]
fn every_declared_output_needs_an_unambiguous_artifact() {
    let outputs = [
        PathBuf::from("/fixture/target/release/lkjscript"),
        PathBuf::from("/other/lkjscript"),
    ];
    assert!(
        records(
            &transcript(&[artifact(), finish()]),
            Path::new("/fixture/Cargo.toml"),
            &outputs
        )
        .is_err()
    );
    let outputs = [
        outputs[0].clone(),
        PathBuf::from("/fixture/target/release/other"),
    ];
    assert!(
        records(
            &transcript(&[artifact(), finish()]),
            Path::new("/fixture/Cargo.toml"),
            &outputs
        )
        .is_err()
    );
}
