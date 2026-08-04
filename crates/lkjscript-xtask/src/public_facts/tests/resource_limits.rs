use std::fs;

use serde_json::{json, Value};

use super::support;

#[test]
fn cumulative_content_limit_rejects_during_the_first_excess_read() {
    let paths: Vec<_> = ('a'..='e')
        .map(|suffix| format!("docs/large-{suffix}.bin"))
        .collect();
    let mut fact = support::fact("aggregate-content", vec![]);
    fact["implementation_anchors"] = Value::Array(
        paths
            .iter()
            .map(|path| Value::String(path.clone()))
            .collect(),
    );
    let root = support::fixture("aggregate-content", vec![fact]);
    for path in &paths {
        assert!(fs::write(root.join(path), vec![b'x'; 3_500_000]).is_ok());
    }
    let error = super::super::load(&root).err().unwrap_or_default();
    assert!(error.contains("exceeds read cap"));
    support::cleanup(&root);
}

#[test]
fn cumulative_shard_limit_rejects_during_the_first_excess_read() {
    let root = support::fixture(
        "aggregate-shards",
        vec![support::fact("alpha-fact", vec![])],
    );
    let directory = root.join("meta/config/public-facts");
    assert!(fs::remove_file(directory.join("facts.json")).is_ok());
    let contract = support::contract();
    let manifest = json!({
        "schema": "lkjscript.public-facts",
        "contract": contract,
        "platform_revision": lkjscript_contracts::PLATFORM_REVISION,
        "shards": [
            {"path": "alpha.json", "first": "alpha-fact", "last": "alpha-fact"},
            {"path": "beta.json", "first": "beta-fact", "last": "beta-fact"}
        ]
    });
    assert!(fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap_or_default()
    )
    .is_ok());
    for (name, id) in [("alpha.json", "alpha-fact"), ("beta.json", "beta-fact")] {
        let shard = json!({
            "schema": "lkjscript.public-fact-shard",
            "contract": contract,
            "facts": [support::fact(id, vec![])]
        });
        let mut bytes = serde_json::to_vec(&shard).unwrap_or_default();
        bytes.resize(8_500_000, b' ');
        assert!(fs::write(directory.join(name), bytes).is_ok());
    }
    let error = super::super::load(&root).err().unwrap_or_default();
    assert!(error.contains("exceeds read cap"));
    support::cleanup(&root);
}
