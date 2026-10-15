use super::*;
use serde_json::{Value, json};

pub(crate) fn arguments(boundary: &str) -> Vec<String> {
    let values: &[&str] = match boundary {
        "review_refresh" => &["--project", "/fixture", "change", "refresh"],
        "cache_read_fifo" => &["--project", "/fixture", "check"],
        "cache_write_fifo" | "publication_lock" => &["--project", "/fixture", "change", "apply"],
        "detached_foreground" => &["run", "--deployment", "detached.json"],
        "deployment_batch" => &[
            "run",
            "--deployment",
            "detached.json",
            "--batch-input-file",
            "input.json",
            "--batch-result-file",
            "result.json",
        ],
        "shared_parallel_http" => &[
            "serve",
            "--deployment",
            "one.json",
            "--deployment",
            "two.json",
        ],
        _ => &[],
    };
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn marker(boundary: &str, candidate: &ArtifactIdentity) -> Value {
    json!({
        "schema":{"identity":"lkjscript-native-boundary-smoke","version":1},
        "test":public_inventory::boundary_name(boundary),
        "candidate_sha256":candidate.sha256,
        "observations":[{"arguments":arguments(boundary),"exit_code":0,"joined":true,"environment_cleared":true,"executable_sha256":candidate.sha256}],
        "cleanup_complete":true,
    })
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, ArtifactIdentity) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("boundaries");
    fs::create_dir(&root).unwrap();
    let candidate = temporary.path().join("candidate");
    fs::write(&candidate, b"selected immutable final candidate").unwrap();
    let candidate = file(&candidate, "lkjscript").unwrap();
    for boundary in public_inventory::BOUNDARIES {
        fs::write(
            root.join(format!("{boundary}.json")),
            marker(boundary, &candidate).to_string(),
        )
        .unwrap();
    }
    (temporary, root, candidate)
}

#[test]
fn seven_boundary_markers_require_exact_identity_actual_byte_binding_and_joined_observations() {
    let (_temporary, root, candidate) = fixture();
    let accepted = read(&root, &candidate).unwrap();
    assert_eq!(accepted.len(), 7);
    assert!(accepted.iter().all(|proof| proof.process_observations == 1));
    for boundary in public_inventory::BOUNDARIES {
        let path = root.join(format!("{boundary}.json"));
        let original = marker(boundary, &candidate);
        for (pointer, replacement) in [
            ("/schema/identity", json!("foreign-boundary-smoke")),
            ("/schema/version", json!(0)),
            ("/test", json!("native_boundary_smokes::foreign")),
            ("/candidate_sha256", json!("0".repeat(64))),
            ("/cleanup_complete", json!(false)),
            ("/observations", json!([])),
            ("/observations/0/joined", json!(false)),
            ("/observations/0/environment_cleared", json!(false)),
            ("/observations/0/executable_sha256", json!("0".repeat(64))),
            ("/observations/0/exit_code", json!(-1)),
            ("/observations/0/exit_code", json!(256)),
            ("/observations/0/arguments", json!([])),
            ("/observations/0/arguments", json!(["--version"])),
            ("/observations/0/arguments", json!(["bad\u{0}argument"])),
            (
                "/observations/0/arguments",
                json!(["a".repeat(16 * 1024 + 1)]),
            ),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = replacement;
            fs::write(&path, changed.to_string()).unwrap();
            assert!(
                read(&root, &candidate).is_err(),
                "accepted {boundary} {pointer}"
            );
        }
        let mut changed = original.clone();
        changed["observations"][0]["execution_mode"] = json!("host-semantic");
        fs::write(&path, changed.to_string()).unwrap();
        assert!(read(&root, &candidate).is_err());
        for extra_arguments in [arguments(boundary), vec!["--version".to_owned()]] {
            let mut extra = original["observations"][0].clone();
            extra["arguments"] = json!(extra_arguments);
            changed = original.clone();
            changed["observations"].as_array_mut().unwrap().push(extra);
            fs::write(&path, changed.to_string()).unwrap();
            assert!(
                read(&root, &candidate).is_err(),
                "accepted duplicate or surplus selected-candidate child for {boundary}"
            );
        }
        fs::write(&path, original.to_string()).unwrap();
        read(&root, &candidate).unwrap();
    }
}

#[test]
fn missing_extra_symlink_duplicate_fields_and_oversized_markers_are_not_complete_evidence() {
    let (_temporary, root, candidate) = fixture();
    let path = root.join("deployment_batch.json");
    let original = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(read(&root, &candidate).is_err());
    fs::write(&path, &original).unwrap();
    let extra = root.join("unselected.json");
    fs::write(&extra, &original).unwrap();
    assert!(read(&root, &candidate).is_err());
    fs::remove_file(extra).unwrap();
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(root.join("review_refresh.json"), &path).unwrap();
    assert!(read(&root, &candidate).is_err());
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"{\"test\":\"native_boundary_smokes::deployment_batch\",\"test\":\"native_boundary_smokes::deployment_batch\"}").unwrap();
    assert!(read(&root, &candidate).is_err());
    let oversized = fs::File::create(&path).unwrap();
    oversized.set_len(MAXIMUM_BYTES + 1).unwrap();
    assert!(read(&root, &candidate).is_err());
    fs::write(&path, &original).unwrap();
    read(&root, &candidate).unwrap();
}

#[test]
fn required_boundary_options_have_values_and_cannot_be_replaced_with_version() {
    for boundary in public_inventory::BOUNDARIES {
        let valid = arguments(boundary);
        assert!(operation(boundary, &valid));
        assert!(!operation(boundary, &["--version".to_owned()]));
        for (index, argument) in valid.iter().enumerate() {
            if argument.starts_with("--") && index + 1 < valid.len() {
                let mut empty = valid.clone();
                empty[index + 1].clear();
                assert!(!operation(boundary, &empty), "empty {boundary} {argument}");
                let mut absent = valid.clone();
                absent.remove(index + 1);
                assert!(
                    !operation(boundary, &absent),
                    "absent {boundary} {argument}"
                );
            }
        }
    }
}
