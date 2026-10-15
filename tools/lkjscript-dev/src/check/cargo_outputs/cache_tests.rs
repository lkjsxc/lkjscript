use super::cargo_output_tests::{build, fixture};
use super::*;
use crate::check::cache::gate_evidence_digest;
use crate::check::model::{CACHE_CONTRACT_VERSION, CacheRecord};
use std::fs;

#[test]
fn cargo_output_cache_requires_original_producer_binding_on_store_and_load() -> Result<(), DevError>
{
    let root = fixture();
    let mut receipt = build(root.path(), &root.path().join("target"), "cached");
    receipt.cache.eligible = true;
    let mut gate = Gate::new("cached", receipt.command.clone());
    gate.required_outputs = vec![root.path().join("target/release/lkjscript")];
    let cache_root = root.path().join("cache");
    let cache = VerificationCache::new(root.path(), &cache_root);
    if receipt.status != GateStatus::Passed || receipt.cargo_origin.is_none() {
        return Err(DevError::corrupt(
            "genuine fixture lacks its cold Cargo origin",
        ));
    }
    // CHECK8 executable producers must be re-observed even with a genuine origin.
    if cache.store(&gate, &receipt).is_ok() {
        return Err(DevError::corrupt(
            "an executable producer was result-cacheable",
        ));
    }
    let process = receipt
        .process
        .clone()
        .ok_or_else(|| DevError::corrupt("original process absent"))?;
    cache.store_test_log(&process.stdout)?;
    cache.store_test_log(&process.stderr)?;
    let record_path = cache.record_path(&gate, &receipt.input_fingerprint)?;
    let original = CacheRecord {
        cache_contract_version: CACHE_CONTRACT_VERSION,
        gate: gate.name.clone(),
        input_fingerprint: receipt.input_fingerprint.clone(),
        identity_command: gate.identity_command().to_vec(),
        source_elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        process: process.clone(),
        outputs: receipt.outputs.clone(),
        cargo_origin: receipt.cargo_origin.clone(),
        evidence_digest: receipt.evidence_digest.clone(),
    };
    evidence::publish_json(&record_path, &original)?;
    let restored = root.path().join("restored");
    fs::create_dir(&restored)?;
    let probe = |label: &str| {
        cache.load(
            &gate,
            &receipt.input_fingerprint,
            &restored.join(format!("{label}.stdout")),
            &restored.join(format!("{label}.stderr")),
        )
    };
    if probe("valid").cached.is_some() {
        return Err(DevError::corrupt(
            "a cached origin replaced fresh executable selection",
        ));
    }
    // Keep the coherent artifact-free forgery. Valid origins cannot turn this
    // producer into an accepted result-cache hit, even with consistently hashed logs.
    let forged_log = root.path().join("forged.stdout");
    evidence::publish(
        &forged_log,
        b"{\"reason\":\"build-finished\",\"success\":true}\n",
    )?;
    let mut forged_process = process;
    forged_process.stdout =
        evidence::proof(&forged_log, evidence::relative(root.path(), &forged_log))?;
    let mut forged = receipt.clone();
    forged.process = Some(forged_process.clone());
    forged.evidence_digest = gate_evidence_digest(
        &gate.name,
        &forged.input_fingerprint,
        &forged_process,
        &forged.inputs,
        &forged.outputs,
        forged.cargo_origin.as_ref(),
    )?;
    if cache.store(&gate, &forged).is_ok() {
        return Err(DevError::corrupt(
            "a rehashed artifact-free producer was cacheable",
        ));
    }
    let mut record = original.clone();
    record.process = forged_process;
    record.evidence_digest = forged.evidence_digest;
    cache.store_test_log(&record.process.stdout)?;
    evidence::publish_json(&record_path, &record)?;
    if probe("forged").cached.is_some() {
        return Err(DevError::corrupt(
            "a forged producer record supplied acceptance",
        ));
    }
    evidence::publish_json(&record_path, &original)?;
    if probe("recovered").cached.is_some() {
        return Err(DevError::corrupt(
            "recovery silently waived fresh producer selection",
        ));
    }
    Ok(())
}
