mod resource_limits;
mod support;

use std::fs;

use serde_json::Value;

use super::{check, generate, load};

#[test]
fn matching_projection_and_deterministic_report_pass() {
    let root = support::fixture("matching", vec![support::fact("test-capability", vec![])]);
    support::install_claim(&root, "test-capability");
    assert_eq!(check(&root), 0);
    assert_eq!(generate(&root), 0);
    let first = fs::read(root.join("target/lkjscript/documentation/facts.json"));
    assert_eq!(check(&root), 0);
    assert_eq!(generate(&root), 0);
    let second = fs::read(root.join("target/lkjscript/documentation/facts.json"));
    assert!(first.is_ok());
    assert_eq!(first.ok(), second.ok());
    support::cleanup(&root);
}

#[test]
fn stale_projection_fails_after_authority_changes() {
    let root = support::fixture("stale", vec![support::fact("test-capability", vec![])]);
    support::install_claim(&root, "test-capability");
    assert!(fs::write(root.join("docs/authority.md"), "# Changed Authority\n").is_ok());
    assert!(check(&root) > 0);
    assert!(!root
        .join("target/lkjscript/documentation/facts.json")
        .exists());
    support::cleanup(&root);
}

#[test]
fn obsolete_marker_fails() {
    let root = support::fixture("obsolete", vec![support::fact("test-capability", vec![])]);
    assert!(fs::write(
        root.join("docs/claim.md"),
        "# Claim\n\n## Status\n\n<!-- LKJ-STATUS id=test-capability status=current -->\n"
    )
    .is_ok());
    assert!(check(&root) > 0);
    support::cleanup(&root);
}

#[test]
fn unknown_manifest_field_fails() {
    let root = support::fixture("unknown", vec![support::fact("test-capability", vec![])]);
    let path = root.join("meta/config/public-facts/manifest.json");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&path).unwrap_or_default()).unwrap_or(Value::Null);
    value["unknown"] = Value::Bool(true);
    assert!(fs::write(path, serde_json::to_vec(&value).unwrap_or_default()).is_ok());
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn duplicate_json_field_fails() {
    let root = support::fixture(
        "duplicate-field",
        vec![support::fact("test-capability", vec![])],
    );
    let path = root.join("meta/config/public-facts/manifest.json");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let duplicate = text.replacen(
        "\"schema\": \"lkjscript.public-facts\"",
        "\"schema\": \"lkjscript.public-facts\",\n  \"schema\": \"lkjscript.public-facts\"",
        1,
    );
    assert!(fs::write(path, duplicate).is_ok());
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn duplicate_fact_fails() {
    let fact = support::fact("test-capability", vec![]);
    let root = support::fixture("duplicate", vec![fact.clone(), fact]);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn unlisted_shard_fails() {
    let root = support::fixture("unlisted", vec![support::fact("test-capability", vec![])]);
    assert!(fs::write(root.join("meta/config/public-facts/extra.json"), b"{}").is_ok());
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn dependency_cycle_has_a_deterministic_witness() {
    let root = support::fixture(
        "cycle",
        vec![
            support::fact("alpha-fact", vec!["beta-fact"]),
            support::fact("beta-fact", vec!["alpha-fact"]),
        ],
    );
    let error = load(&root).err().unwrap_or_default();
    assert!(error.contains("alpha-fact"));
    support::cleanup(&root);
}

#[test]
fn invalidation_cycle_fails() {
    let mut alpha = support::fact("alpha-fact", vec![]);
    let mut beta = support::fact("beta-fact", vec![]);
    alpha["invalidated_by"] = serde_json::json!(["beta-fact"]);
    beta["invalidated_by"] = serde_json::json!(["alpha-fact"]);
    let root = support::fixture("invalidation-cycle", vec![alpha, beta]);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn missing_exclusion_fails() {
    let mut fact = support::fact("test-capability", vec![]);
    fact["exclusions"] = Value::Array(Vec::new());
    let root = support::fixture("exclusion", vec![fact]);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn current_fact_without_evidence_fails() {
    let mut fact = support::fact("test-capability", vec![]);
    fact["evidence"] = Value::Array(Vec::new());
    let root = support::fixture("missing-evidence", vec![fact]);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn excessive_fact_collection_fails() {
    let mut fact = support::fact("test-capability", vec![]);
    fact["scope"] = Value::Array(
        (0..33)
            .map(|index| Value::String(format!("scope-{index:02}")))
            .collect(),
    );
    let root = support::fixture("excessive", vec![fact]);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn nested_aggregate_member_limit_fails() {
    let contracts = vec![Value::String("a".repeat(64)); 32];
    let facts = ('a'..='i')
        .map(|suffix| {
            let mut fact = support::fact(&format!("aggregate-{suffix}"), vec![]);
            fact["evidence"] = Value::Array(
                (0..32)
                    .map(|_| {
                        serde_json::json!({
                            "path": "docs/fact.md",
                            "class": "implementation-test",
                            "commit": null,
                            "contracts": contracts.clone(),
                            "not-tested": []
                        })
                    })
                    .collect(),
            );
            fact
        })
        .collect();
    let root = support::fixture("nested-members", facts);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[test]
fn path_escape_fails_before_claim_scan() {
    let mut fact = support::fact("test-capability", vec![]);
    fact["authority"]["path"] = Value::String("../escape.md".into());
    let root = support::fixture("escape", vec![fact]);
    assert!(load(&root).is_err());
    support::cleanup(&root);
}

#[cfg(unix)]
#[test]
fn symlink_escape_fails() {
    use std::os::unix::fs::symlink;

    let mut fact = support::fact("test-capability", vec![]);
    fact["authority"]["path"] = Value::String("docs/escape.md".into());
    let root = support::fixture("symlink", vec![fact]);
    assert!(symlink("/etc/passwd", root.join("docs/escape.md")).is_ok());
    assert!(load(&root).is_err());
    support::cleanup(&root);
}
