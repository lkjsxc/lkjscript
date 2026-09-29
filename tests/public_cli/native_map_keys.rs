//! Literal map authoring and detached execution through a copied executable.
use super::*;
use base64::Engine;
use serde_json::json;

#[test]
fn native_map_keys_preserve_versions_and_project_without_payload_copies() {
    let public = Native::template("command");
    let input = public.input(
        "map-keys.lkjc",
        &include_str!("../fixtures/shared-map-keys.lkjc").replacen(
            "base=BASE",
            &format!("base={}", public.revision()),
            1,
        ),
    );
    public.apply(&input, &public.plan(&input, true), true);
    public.cli(&["check"], true);
    let artifact = public.root.path().join("maps.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let mut deployment: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    deployment["artifact"] = json!("maps.lkja");
    let bytes = |data: &[u8]| {
        json!({
            "$bytes": base64::engine::general_purpose::STANDARD.encode(data)
        })
    };
    let cases = [
        ("text", json!(""), json!("日本語".repeat(64))),
        ("bytes", bytes(&[]), bytes(&[0, 1, 128, 255].repeat(128))),
    ];
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    for detached in [false, true] {
        if detached {
            std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
            std::fs::remove_file(&input).unwrap();
        }
        for (target, other, selected) in &cases {
            let arguments = public.input(
                "arguments.json",
                &json!([[[other, 99], [selected, 11]], selected]).to_string(),
            );
            deployment["target"] = json!(target);
            let descriptor = public.input("maps.deployment.json", &deployment.to_string());
            let result = public.root.path().join("result.json");
            let mut route = if detached {
                vec!["run", "--deployment", path(&descriptor)]
            } else {
                vec!["run", target]
            };
            route.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&result),
            ]);
            let records = public.cli(&route, true);
            let observed: Value = serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
            let before = json!([{"key":other,"value":99},{"key":selected,"value":11}]);
            let changed = json!([{"key":other,"value":99},{"key":selected,"value":12}]);
            let removed = json!([{"key":other,"value":99}]);
            assert_eq!(observed, json!([before, changed, removed, before]));
            let execution = compact_record(&records, "execution");
            if detached {
                let observation: Value =
                    serde_json::from_str(compact_field(execution, "production-observation"))
                        .unwrap();
                assert_eq!(observation["value_work"]["maps"]["key_bytes_copied"], 0);
                assert_eq!(observation["live_handles_after"], 0);
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
                assert_eq!(
                    compact_field(execution, "production-map-key-bytes-copied"),
                    "0"
                );
                assert_eq!(
                    compact_field(execution, "reference-map-key-bytes-copied"),
                    "0"
                );
            }
            std::fs::remove_file(result).unwrap();
        }
    }
    assert_eq!(
        std::fs::read(public.root.path().join("retained-project/HEAD")).unwrap(),
        head
    );
}
