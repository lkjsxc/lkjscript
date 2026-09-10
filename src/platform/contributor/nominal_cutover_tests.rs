//! Fixed Graph 12 producer facts, independent of Graph 13 materialization and code generation.
use super::*;
use serde_json::Value;

fn neutral(value: &mut Value) {
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
fn maintained_graph13_cutover_preserves_every_predecessor_owner_type_and_retirement() {
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
        assert_eq!(
            fixture["source_commit"],
            "c0c4975851f34a9a2b3be8cbd3008a1f8bbafc18"
        );
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(project);
        let before = std::fs::read(path.join("HEAD")).unwrap();
        let repository = GraphRepository::open(&path).unwrap();
        let snapshot = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
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
            .filter(|(owner, _)| !added.contains(&serde_json::to_value(owner).unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(
            retained.len() as u64,
            fixture["stable_owner_count"].as_u64().unwrap()
        );
        let mut owners = serde_json::to_value(retained).unwrap();
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
                .or_else(|| snapshot.dependency_types.get(&digest))
                .expect("unchanged type exists");
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
