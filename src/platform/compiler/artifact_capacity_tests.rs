//! Wire/framing controls complement public graph-to-artifact capacity coverage.
use super::*;

fn fixture() -> EncodedArtifact {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let created = memory_repository(&snapshot).unwrap();
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    link_artifact(&created.repository, compilation.manifest_digest, &[])
        .unwrap()
        .artifact
}

#[test]
fn artifact_manifest_codec_crosses_former_package_and_payload_limits() {
    let mut manifest = fixture().manifest;
    let mut template = manifest.packages[0].clone();
    template.runtime_owners.clear();
    // Exercise the manifest wire inventory independently of the later closure proof.
    // This synthetic inventory does not claim an admitted executable artifact.
    manifest.packages = (1_u32..=16_385)
        .map(|serial| {
            let mut package = template.clone();
            package.package = format!("pkg_{serial:032x}").parse().unwrap();
            package
        })
        .collect();
    manifest.root_package = manifest.packages[0].package;
    let (digest, bytes) = manifest.encode().expect("wide manifest encodes");
    assert!(manifest.packages.len() > 10_000);
    assert!(bytes.len() > 4 * 1024 * 1024);
    let decoded = ArtifactManifest::decode(&bytes, digest).expect("wide manifest decodes");
    assert!(decoded == manifest);
}

#[test]
fn artifact_bundle_declared_lengths_use_checked_input_framing() {
    let encoded = fixture();
    let bytes = encoded.bytes;
    let manifest_length = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
    let segment_position = super::super::artifact::BUNDLE_HEADER_BYTES
        .checked_add(usize::try_from(manifest_length).unwrap())
        .unwrap();
    for (offset, allowed_codes) in [
        (
            12,
            [
                "artifact_bundle_manifest_bounds",
                "artifact_bundle_manifest_length",
            ],
        ),
        (
            20,
            [
                "artifact_bundle_segment_count",
                "artifact_bundle_segment_count",
            ],
        ),
        (
            segment_position,
            [
                "artifact_bundle_segment_bounds",
                "artifact_bundle_segment_length",
            ],
        ),
    ] {
        let mut malformed = bytes.clone();
        malformed[offset..offset + 8].copy_from_slice(&u64::MAX.to_be_bytes());
        let error = load_artifact(&malformed)
            .expect_err("declared extent cannot wrap or allocate beyond input");
        assert!(allowed_codes.contains(&error.code.as_str()), "{error}");
    }
}
