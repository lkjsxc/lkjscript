use super::*;
use std::io::Write;

pub(crate) fn fixture(tag: &str, executable: Option<&[u8]>) -> Vec<u8> {
    let mut elf = vec![0_u8; 128];
    elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    elf[16..18].copy_from_slice(&2_u16.to_le_bytes());
    elf[18..20].copy_from_slice(&62_u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
    elf[32..40].copy_from_slice(&64_u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
    elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
    elf[96..104].copy_from_slice(&128_u64.to_le_bytes());
    let executable = executable.unwrap_or(&elf);
    let digest = |bytes: &[u8]| sha256_bytes(bytes).unwrap().as_str().to_owned();
    // Recorded producer identities are independent of the current packaging tool policy.
    let value = serde_json::json!({
        "format": "lkjscript-release-content-1",
        "build": {"target_policy_sha256": "6".repeat(64), "command": ["cargo", "build", "--release", "--locked", "--bin", "lkjscript", "--target", "x86_64-unknown-linux-musl"]},
        "product": {"name": "lkjscript", "version": tag.strip_prefix('v').unwrap()},
        "source": {"repository": "lkjsxc/lkjscript", "expected_release_tag": tag, "commit_sha": "1".repeat(40), "commit_timestamp_unix_seconds": 1700000000},
        "target_triple": "x86_64-unknown-linux-musl",
        "toolchain": {"rustc": "rustc 1.80.0", "cargo": "cargo 1.80.0", "toolchain_channel": "1.80.0"},
        "cargo_lock_sha256": "2".repeat(64),
        "executable": {"archive_mode": 493, "byte_length": executable.len(), "sha256": digest(executable),
            "elf": inspect_static_elf_bytes(executable).unwrap(), "capabilities_digest": "3".repeat(64)},
        "root_license": {"archive_mode": 420, "byte_length": 7, "sha256": digest(b"license")},
        "third_party_notices": {"generator": "recorded-notice-tool", "generator_version": "0.1", "downloaded_archive_sha256": "4".repeat(64), "executable_sha256": "5".repeat(64), "invocation": ["recorded-notice-tool"], "archive_mode": 420, "byte_length": 6, "sha256": digest(b"notice")},
        "packaging": {"tar_format": "posix-ustar", "tar_version": "tar recorded", "tar_invocation": normalized_tar_invocation(1700000000), "gzip_version": "gzip recorded", "gzip_level": 9, "gzip_name_header": false, "gzip_time_header": false, "gzip_invocation": normalized_gzip_invocation(), "numeric_owner": 0, "numeric_group": 0, "source_timestamp_unix_seconds": 1700000000, "members": manifest_members()}
    });
    let manifest =
        canonical_json(&serde_json::from_value::<ReleaseManifest>(value).unwrap()).unwrap();
    encode_tar(&[
        ("lkjscript/", 0o755, b'5', b""),
        ("lkjscript/lkjscript", 0o755, b'0', executable),
        ("lkjscript/LICENSE", 0o644, b'0', b"license"),
        (
            "lkjscript/THIRD-PARTY-LICENSES.html",
            0o644,
            b'0',
            b"notice",
        ),
        ("lkjscript/RELEASE-MANIFEST.json", 0o644, b'0', &manifest),
    ])
}
fn octal(field: &mut [u8], value: u64) {
    let text = format!("{value:0width$o}\0", width = field.len() - 1);
    field.copy_from_slice(text.as_bytes());
}
pub(crate) fn checksum(header: &mut [u8]) {
    header[148..156].fill(b' ');
    let sum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
}
fn encode_tar(members: &[(&str, u32, u8, &[u8])]) -> Vec<u8> {
    let mut tar = Vec::new();
    for (name, mode, kind, payload) in members {
        let mut header = [0_u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        octal(&mut header[100..108], u64::from(*mode));
        octal(&mut header[108..116], 0);
        octal(&mut header[116..124], 0);
        octal(&mut header[124..136], payload.len() as u64);
        octal(&mut header[136..148], 1700000000);
        header[156] = *kind;
        header[257..265].copy_from_slice(b"ustar\x0000");
        checksum(&mut header);
        tar.extend(header);
        tar.extend_from_slice(payload);
        tar.resize(tar.len().next_multiple_of(512), 0);
    }
    tar.resize(tar.len() + 1024, 0);
    tar
}
pub(crate) fn gzip(tar: &[u8]) -> Vec<u8> {
    let mut output = flate2::GzBuilder::new()
        .operating_system(3)
        .mtime(0)
        .write(Vec::new(), flate2::Compression::best());
    output.write_all(tar).unwrap();
    output.finish().unwrap()
}

#[test]
fn native_recorded_producer_admission_and_payload_bytes() {
    let admitted = admit_gzip(&gzip(&fixture("v0.1.32", None))).unwrap();
    assert_eq!(
        admitted.verified.manifest.toolchain.toolchain_channel,
        "1.80.0"
    );
    let payloads = admitted.payloads().collect::<Vec<_>>();
    assert_eq!(payloads.len(), 4);
    assert_eq!(payloads[1].1, b"license");
    assert_eq!(payloads[2].1, b"notice");
}

#[test]
fn native_content_has_no_publication_event_and_preserves_executable_identity() {
    let admitted = admit_gzip(&gzip(&fixture("v0.1.32", None))).unwrap();
    assert_eq!(
        admitted.verified.manifest.publication_provenance(),
        "neutral/unverified-publication"
    );
    let executable = admitted.payloads().next().unwrap().1;
    assert_eq!(
        admitted.verified.manifest.executable.sha256,
        sha256_bytes(executable).unwrap()
    );
    let bytes = admitted.payloads().last().unwrap().1;
    let text = std::str::from_utf8(bytes).unwrap();
    assert!(text.starts_with("{\n  \"format\": \"lkjscript-release-content-1\",\n"));
    assert!(!text.contains("publication_mode"));
    assert!(!text.contains("annotated_tag_object_sha"));
    assert!(!text.contains("tagged_commit_sha"));
    assert_eq!(
        canonical_json(&decode_manifest(bytes).unwrap()).unwrap(),
        bytes
    );
}

#[test]
fn native_manifest_requires_current_format() {
    let admitted = admit_gzip(&gzip(&fixture("v0.1.32", None))).unwrap();
    let current = String::from_utf8(canonical_json(&admitted.verified.manifest).unwrap()).unwrap();
    let unsupported = current.replace(
        model::PUBLICATION_NEUTRAL_FORMAT,
        "unsupported-release-content",
    );
    assert!(decode_manifest(unsupported.as_bytes()).is_err());
}

#[test]
fn native_manifest_shape_and_canonical_json_are_complete() {
    let admitted = admit_gzip(&gzip(&fixture("v0.1.32", None))).unwrap();
    let value = serde_json::to_value(&admitted.verified.manifest).unwrap();
    for fault in 0..9 {
        let mut changed = value.clone();
        match fault {
            0 => {
                changed.as_object_mut().unwrap().remove("format");
            }
            1 => changed["unknown_field"] = "unexpected".into(),
            2 => changed["source"]["unknown_field"] = "unexpected".into(),
            3 => {
                changed.as_object_mut().unwrap().remove("build");
            }
            4 => changed["build"]["target_policy_sha256"] = "not-a-digest".into(),
            5 => changed["build"]["environment"] = serde_json::json!({}),
            6 => {
                changed["source"]
                    .as_object_mut()
                    .unwrap()
                    .remove("commit_sha");
            }
            7 => changed["source"]["repository"] = "another/project".into(),
            _ => changed["target_triple"] = "aarch64-unknown-linux-musl".into(),
        }
        let result = serde_json::from_value::<ReleaseManifest>(changed)
            .map_err(|error| error.to_string())
            .and_then(|manifest| validate_manifest(&manifest).map_err(|error| error.to_string()));
        assert!(result.is_err(), "accepted manifest shape fault {fault}");
    }
    let mut wrong_build = admitted.verified.manifest.clone();
    wrong_build.build.command.push("--all-features".to_owned());
    assert!(decode_manifest(&canonical_json(&wrong_build).unwrap()).is_err());

    let current = canonical_json(&admitted.verified.manifest).unwrap();
    assert!(decode_manifest(&[current.clone(), b"\n".to_vec()].concat()).is_err());
    let duplicate = String::from_utf8(current).unwrap().replacen(
        "  \"format\":",
        "  \"format\": \"lkjscript-release-content-1\",\n  \"format\":",
        1,
    );
    assert!(decode_manifest(duplicate.as_bytes()).is_err());
}
#[test]
fn native_recomputed_digest_does_not_admit_bad_containers() {
    let healthy = fixture("v0.1.32", None);
    let mut faults = Vec::new();
    for (offset, replacement) in [
        (0, b'/'),
        (156, b'2'),
        (157, b'x'),
        (265, b'x'),
        (345, b'x'),
        (100, b'7'),
        (111, b'1'),
        (511, b'x'),
    ] {
        let mut changed = healthy.clone();
        changed[offset] = replacement;
        checksum(&mut changed[..512]);
        faults.push(changed);
    }
    let mut changed = healthy.clone();
    changed[148] ^= 1;
    faults.push(changed);
    let mut changed = healthy.clone();
    changed[1024] = 0;
    faults.push(changed); // actual ELF differs, valid tar checksum
    let mut changed = healthy.clone();
    changed[1152] = 1;
    faults.push(changed); // nonzero payload padding
    let mut changed = healthy.clone();
    changed.extend([0_u8; 512]);
    faults.push(changed);
    faults.push(healthy[..healthy.len() - 512].to_vec());
    for (index, tar) in faults.iter().enumerate() {
        assert!(admit_gzip(&gzip(tar)).is_err(), "accepted fault {index}");
    }
    let valid = gzip(&healthy);
    for mut compressed in [
        valid[..valid.len() - 1].to_vec(),
        [valid.clone(), vec![0]].concat(),
        [valid.clone(), valid.clone()].concat(),
    ] {
        assert!(admit_gzip(&compressed).is_err());
        compressed.clear();
    }
    let mut crc = valid;
    let end = crc.len();
    crc[end - 8] ^= 1;
    assert!(admit_gzip(&crc).is_err());
    assert!(admit_gzip(&gzip(&healthy)).is_ok());
}
#[test]
fn native_allocation_and_tag_bounds_are_independent() {
    assert!(read_bounded(&mut &b"12345"[..], 4).is_err());
    for tag in ["latest", "v01.2.3", "v1.2.3-rc.1", "v1.2", "v1.2.3/../x"] {
        assert!(validate_strict_tag(tag, tag.strip_prefix('v').unwrap_or("")).is_err());
    }
    assert_eq!(parse_octal(b"0000755\0", "mode").unwrap(), 493);
    assert!(parse_octal(b"0000\0xx\0", "mode").is_err());
}

#[test]
fn native_gzip_decompression_overrun_rejects_before_container_admission() {
    // Compress a stream independently, without allocating its expanded bytes in the producer.
    let mut encoder = flate2::GzBuilder::new()
        .operating_system(3)
        .write(Vec::new(), flate2::Compression::best());
    let chunk = [0_u8; 65536];
    for _ in 0..4096 {
        encoder.write_all(&chunk).unwrap();
    }
    encoder.write_all(b"x").unwrap();
    let archive = encoder.finish().unwrap();
    assert!(archive.len() < 1024 * 1024);
    let error = admit_gzip(&archive)
        .err()
        .expect("expanded stream must reject");
    assert!(error.message.contains("byte limit"), "{error}");
}
