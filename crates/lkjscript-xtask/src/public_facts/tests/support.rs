use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{json, Value};

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub fn fixture(name: &str, facts: Vec<Value>) -> PathBuf {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "lkjscript-public-facts-{name}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    assert!(fs::create_dir_all(root.join("meta/config/public-facts")).is_ok());
    assert!(fs::create_dir_all(root.join("docs")).is_ok());
    assert!(fs::write(root.join("docs/authority.md"), "# Authority\n").is_ok());
    assert!(fs::write(root.join("docs/evidence.md"), "# Evidence\n").is_ok());
    assert!(fs::write(root.join("docs/claim.md"), "# Claim\n\n## Status\n").is_ok());
    write_registry(&root, facts);
    root
}

pub fn fact(id: &str, dependencies: Vec<&str>) -> Value {
    json!({
        "id": id,
        "kind": "capability",
        "status": "current",
        "scope": ["repository"],
        "interface": "test command",
        "exclusions": [{
            "id": "outside-interface",
            "interface": "No behavior outside the exact test interface is Current."
        }],
        "authority": {"kind": "repository-path", "path": "docs/authority.md"},
        "implementation_anchors": ["docs/authority.md"],
        "evidence": [{"path": "docs/evidence.md", "class": "contract-record"}],
        "projections": ["docs/claim.md"],
        "dependencies": dependencies,
        "invalidated_by": [],
        "platform_revision": lkjscript_contracts::PLATFORM_REVISION,
        "contracts": []
    })
}

pub fn write_registry(root: &Path, facts: Vec<Value>) {
    let contract = contract();
    let first = facts
        .first()
        .and_then(|fact| fact["id"].as_str())
        .unwrap_or("");
    let last = facts
        .last()
        .and_then(|fact| fact["id"].as_str())
        .unwrap_or("");
    let manifest = json!({
        "schema": "lkjscript.public-facts",
        "contract": contract,
        "platform_revision": lkjscript_contracts::PLATFORM_REVISION,
        "shards": [{"path": "facts.json", "first": first, "last": last}]
    });
    let shard = json!({
        "schema": "lkjscript.public-fact-shard",
        "contract": contract,
        "facts": facts
    });
    assert!(fs::write(
        root.join("meta/config/public-facts/manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap_or_default()
    )
    .is_ok());
    assert!(fs::write(
        root.join("meta/config/public-facts/facts.json"),
        serde_json::to_vec_pretty(&shard).unwrap_or_default()
    )
    .is_ok());
}

pub fn install_claim(root: &Path, id: &str) {
    let Some(registry) = super::super::load(root).ok() else {
        return;
    };
    let Some(fact) = registry.facts.get(id) else {
        return;
    };
    let Some(digest) = super::super::digest::projection(&fact.digest).ok() else {
        return;
    };
    let marker = format!("<!-- LKJ-F {id} {} {digest} -->", fact.fact.status.name(),);
    assert!(fs::write(
        root.join("docs/claim.md"),
        format!("# Claim\n\n## Status\n\n{marker}\n")
    )
    .is_ok());
}

pub fn contract() -> String {
    lkjscript_contracts::current_contracts()
        .ok()
        .and_then(|contracts| {
            contracts
                .get(lkjscript_contracts::PUBLIC_FACTS)
                .map(lkjscript_contracts::RegisteredContract::digest)
        })
        .map_or_else(String::new, |digest| digest.to_hex())
}

pub fn cleanup(root: &Path) {
    assert!(fs::remove_dir_all(root).is_ok());
}
