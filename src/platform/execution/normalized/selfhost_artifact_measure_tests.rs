//! Explicit same-input measurements of the complete artifact compiler.
use super::*;

#[test]
#[ignore = "explicit pinned compiler artifact measurement"]
fn selfhost_artifact_measure_pinned_compiler_bundle() {
    use std::io::Write;

    let path = std::env::var_os("LKJSCRIPT_SELFHOST_PINNED_ARTIFACT")
        .expect("select a strict artifact exported from the pinned authoritative compiler");
    let artifact_bytes = std::fs::read(path).unwrap();
    let pinned = crate::platform::compiler::load_artifact(&artifact_bytes).unwrap();
    let input = selfhost_artifact_input::bundle(&pinned).unwrap();
    let (code, emitted, observations) = profiled_result("compile-elf", &input);
    assert_eq!(code, 0);
    let loaded = crate::platform::compiler::load_artifact(&emitted).unwrap();
    let preserved = |domain| {
        matches!(
            domain,
            ObjectDomain::CompilerUnit
                | ObjectDomain::Owner
                | ObjectDomain::Type
                | ObjectDomain::Blob
                | ObjectDomain::PackageRevision
                | ObjectDomain::PackageInterface
        )
    };
    let expected = pinned
        .objects
        .iter()
        .filter(|(key, _)| preserved(key.domain))
        .collect::<BTreeMap<_, _>>();
    let actual = loaded
        .objects
        .iter()
        .filter(|(key, _)| preserved(key.domain))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual, expected,
        "emission changed pinned canonical units or source provenance"
    );
    let units = expected
        .keys()
        .filter(|key| key.domain == ObjectDomain::CompilerUnit)
        .count();
    assert!(units > 0);
    let identity = |bytes: &[u8]| {
        serde_json::json!({
            "bytes": bytes.len(),
            "sha256": crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes)),
        })
    };
    let output_sha256 = crate::platform::semantic_id::encode_hex(&Sha256::digest(&emitted));
    if let Ok(expected) = std::env::var("LKJSCRIPT_SELFHOST_PINNED_OUTPUT_SHA256") {
        assert_eq!(output_sha256, expected, "pinned output bytes changed");
    }
    let (source, _) = compiler();
    let report = serde_json::json!({
        "schema": "lkjscript.selfhost.pinned-artifact-measurement.v1",
        "compiler_semantic_state": crate::platform::kernel::semantic_state_digest(source).unwrap().to_string(),
        "pinned_host_artifact": identity(&artifact_bytes),
        "mode1_input": identity(&input),
        "artifact_output": identity(&emitted),
        "canonical_units": units,
        "preserved_objects": expected.len(),
        "four_evaluator_equality": true,
        "strict_loader": "passed",
        "observations": observations,
    });
    let mut encoded = serde_json::to_vec_pretty(&report).unwrap();
    encoded.push(b'\n');
    if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_ARTIFACT_MEASUREMENT_OUTPUT") {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        output.write_all(&encoded).unwrap();
        output.sync_all().unwrap();
    }
    println!("{}", String::from_utf8(encoded).unwrap());
}
