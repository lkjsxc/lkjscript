//! Fixed Graph 12 producer facts, independent of Graph 13 materialization and code generation.
use super::*;
use serde_json::Value;

fn neutral(value: &mut Value) {
    super::effect_cutover_tests::neutral_effect_fields(value);
    match value {
        Value::Object(fields) => {
            fields.remove("contract_version");
            fields.remove("graph_contract_version");
            if matches!(
                fields.get("kind").and_then(Value::as_str),
                Some("record" | "variant")
            ) {
                for key in ["type_parameters", "type_arguments"] {
                    if fields
                        .get(key)
                        .is_some_and(|v| v.as_array().is_some_and(Vec::is_empty))
                    {
                        fields.remove(key);
                    }
                }
            }
            for value in fields.values_mut() {
                neutral(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                neutral(value);
            }
        }
        _ => {}
    }
}

#[test]
fn frozen_graph13_cutover_preserves_every_predecessor_owner_type_and_retirement() {
    // The retained Graph 12 oracle stays immutable. Account only for the separately
    // enumerated Graph 14 additions and explicit task-port type replacements.
    let effects: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/graph13-effect-cutover.json"
    ))
    .unwrap();
    for (project, fixture) in [
        (
            "packages/standard",
            include_str!("../../../tests/fixtures/graph12-standard-nominal-cutover.json"),
        ),
        (
            "applications/lkjournal",
            include_str!("../../../tests/fixtures/graph12-lkjournal-nominal-cutover.json"),
        ),
    ] {
        let fixture: Value = serde_json::from_str(fixture).unwrap();
        let transition = effects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["project"] == project)
            .unwrap();
        assert_eq!(
            fixture["source_commit"],
            "c0c4975851f34a9a2b3be8cbd3008a1f8bbafc18"
        );
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(project);
        let before = std::fs::read(path.join("HEAD")).unwrap();
        let repository = GraphRepository::open(&path).unwrap();
        let snapshot = if project == "packages/standard" {
            super::effect_cutover_tests::standard_before_task_iteration()
        } else {
            repository
                .view_current()
                .unwrap()
                .reconstruct_full_oracle()
                .unwrap()
                .value
        };
        assert_eq!(
            serde_json::to_value(snapshot.root.repository_id).unwrap(),
            fixture["repository"]
        );
        assert_eq!(
            serde_json::to_value(snapshot.root.package_id).unwrap(),
            fixture["package"]
        );
        let added = fixture["added_owners"].as_array().unwrap();
        for key in added {
            let key = serde_json::from_value(key.clone()).unwrap();
            assert!(
                snapshot.owners.contains_key(&key),
                "new nominal owner omitted"
            );
        }
        let retained = snapshot
            .owners
            .iter()
            .filter(|(owner, _)| {
                let key = serde_json::to_value(owner).unwrap();
                !added.contains(&key)
                    && !transition["added_owners"]
                        .as_array()
                        .unwrap()
                        .contains(&key)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            retained.len() as u64,
            fixture["stable_owner_count"].as_u64().unwrap()
        );
        let mut owners = serde_json::to_value(retained).unwrap();
        for owner in owners.as_array_mut().unwrap() {
            if let Some(change) = transition["port_changes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|change| change["owner"] == owner[0])
            {
                assert_eq!(owner[1]["record"]["function_type"], change["current_type"]);
                owner[1]["record"]["function_type"] = change["previous_type"].clone();
            }
        }
        neutral(&mut owners);
        let hash = |value: &Value| {
            blake3::hash(&serde_json::to_vec(value).unwrap())
                .to_hex()
                .to_string()
        };
        assert_eq!(
            hash(&owners),
            fixture["stable_owner_meaning_blake3"],
            "{project} stable identity/meaning"
        );
        let mut retirements =
            serde_json::to_value(snapshot.retirements.iter().collect::<Vec<_>>()).unwrap();
        neutral(&mut retirements);
        assert_eq!(
            hash(&retirements),
            fixture["retirement_meaning_blake3"],
            "{project} retirement continuity"
        );
        for expected in fixture["type_bytes"].as_array().unwrap() {
            let digest = serde_json::from_value(expected["type"].clone()).unwrap();
            let object = snapshot
                .types
                .get(&digest)
                .or_else(|| snapshot.dependency_types.get(&digest));
            let retired_port;
            let object = if let Some(object) = object {
                object
            } else {
                assert!(
                    transition["port_changes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|change| change["previous_type"] == expected["type"]),
                    "unreviewed missing predecessor type"
                );
                let bytes = expected["bytes"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                retired_port = crate::platform::kernel::decode_type_object(&bytes, digest).unwrap();
                &retired_port
            };
            let (actual, bytes) = crate::platform::kernel::encode_type_object(object).unwrap();
            assert_eq!(actual, digest);
            assert_eq!(
                crate::platform::semantic_id::encode_hex(&bytes),
                expected["bytes"],
                "{project} exact type bytes"
            );
        }
        assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before);
    }
}
