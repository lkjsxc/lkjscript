use super::cargo_output_tests::{build, fixture};
use super::*;
use crate::check::model::CacheRecord;
use std::fs;

#[test]
fn cargo_output_cache_requires_original_producer_binding_on_store_and_load() {
    let root = fixture();
    let mut receipt = build(root.path(), &root.path().join("target"), "cached");
    receipt.cache.eligible = true;
    let mut gate = Gate::new("cached", receipt.command.clone());
    gate.required_outputs = vec![root.path().join("target/release/lkjscript")];
    let cache_root = root.path().join("cache");
    let cache = VerificationCache::new(root.path(), &cache_root);
    cache
        .store(&gate, &receipt)
        .expect("genuine producer cache");
    let restored = root.path().join("restored");
    fs::create_dir(&restored).expect("restored logs");
    let hit = cache.load(
        &gate,
        &receipt.input_fingerprint,
        &restored.join("valid.stdout"),
        &restored.join("valid.stderr"),
    );
    assert!(
        hit.cached.is_some(),
        "genuine matching producer cache must remain reusable"
    );

    // Rewrite both the log and its ordinary identity consistently. Integrity alone
    // must not substitute for the Cargo artifact-to-output relation.
    let forged_log = root.path().join("forged.stdout");
    evidence::publish(
        &forged_log,
        b"{\"reason\":\"build-finished\",\"success\":true}\n",
    )
    .expect("owned malformed claim");
    let mut process = receipt.process.clone().expect("original process");
    process.stdout = evidence::proof(&forged_log, evidence::relative(root.path(), &forged_log))
        .expect("consistent forged log proof");
    let mut forged = receipt.clone();
    forged.process = Some(process.clone());
    forged.evidence_digest = cache::gate_evidence_digest(
        &gate.name,
        &forged.input_fingerprint,
        &process,
        &forged.outputs,
    )
    .expect("consistent digest");
    assert!(
        cache.store(&gate, &forged).is_err(),
        "store must reject an artifact-free claim"
    );

    let record_path = root.path().join(
        cache
            .record_path(&gate, &receipt.input_fingerprint)
            .expect("record path"),
    );
    let mut record: CacheRecord =
        serde_json::from_slice(&fs::read(&record_path).expect("cache record"))
            .expect("cache schema");
    let digest = process.stdout.digest.as_ref().expect("forged log digest");
    let saved = cache_root
        .join("logs")
        .join(format!("{}.log", digest.as_str()));
    evidence::publish(&saved, &fs::read(&forged_log).expect("forged bytes"))
        .expect("owned consistent cached log");
    record.process = process;
    record.evidence_digest = forged.evidence_digest;
    evidence::publish_json(&record_path, &record).expect("owned consistent cached claim");
    let rejected = cache.load(
        &gate,
        &receipt.input_fingerprint,
        &restored.join("forged.stdout"),
        &restored.join("forged.stderr"),
    );
    assert!(
        rejected.cached.is_none(),
        "restored bytes still require origin admission"
    );
    assert!(rejected.reason.starts_with("cache_corrupt:"));
    cache
        .store(&gate, &receipt)
        .expect("repair with original evidence");
    let recovered = cache.load(
        &gate,
        &receipt.input_fingerprint,
        &restored.join("recovered.stdout"),
        &restored.join("recovered.stderr"),
    );
    assert!(recovered.cached.is_some());
}
