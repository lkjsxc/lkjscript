//! Focused producer controls. These drafts have not run under Cargo yet.
use super::*;
use crate::platform::kernel::{PackageTransportDigest, TypeForm, TypeObject, encode_type_object};
use crate::platform::package_transport::source::PackageContainer;
use crate::platform::storage::object::{ObjectDomain, ObjectKey};

const PUBLIC_EXPORT: &[u8] =
    include_bytes!("../../../../tests/fixtures/selfhost-source-public-export.lkjp");
const PUBLIC_CARRIER: &[u8] =
    include_bytes!("../../../../tests/fixtures/selfhost-source-profile3.lkjsg");

fn at<const N: usize>(bytes: &[u8], start: usize) -> Result<[u8; N], Diagnostic> {
    let end = start.checked_add(N).ok_or_else(layout_error)?;
    bytes
        .get(start..end)
        .ok_or_else(layout_error)?
        .try_into()
        .map_err(|_| layout_error())
}

fn word(bytes: &[u8], start: usize) -> Result<usize, Diagnostic> {
    usize::try_from(u32::from_le_bytes(at(bytes, start)?)).map_err(|_| extent_error())
}

fn copied(bytes: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let mut result = Output::new(bytes.len())?;
    result.append(bytes)?;
    result.finish()
}

fn public_container() -> Result<PackageContainer, Diagnostic> {
    PackageContainer::decode(
        PUBLIC_EXPORT,
        PackageTransportDigest::from_bytes(at(PUBLIC_EXPORT, 40)?),
    )
}

fn fixture<'a>(
    core: &'a [u8],
    owners: &'a [RawOwner<'a>],
    types: &'a [RawType<'a>],
    retirements: &'a [RawRetirement<'a>],
) -> RawSourceGraph<'a> {
    RawSourceGraph {
        package: [0x11; 16],
        package_name: b"raw-source-fixture",
        semantic_state: [0x22; 32],
        provenance: RawRevisionProvenance {
            claimed_revision: [0x33; 32],
            canonical_core: core,
        },
        owners,
        types,
        retirements,
    }
}

#[test]
fn source_input_current_layout_preserves_every_row_and_original_object() -> Result<(), Diagnostic> {
    // Deliberately arbitrary claims: framing success is no admission certificate.
    let owners = [
        RawOwner {
            key: [1; 17],
            kind: 2,
            digest: [0x41; 32],
            bytes: b"owner-one",
        },
        RawOwner {
            key: [2; 17],
            kind: 20,
            digest: [0x42; 32],
            bytes: b"uncalled-target",
        },
    ];
    let types = [RawType {
        digest: [0x51; 32],
        bytes: b"unused-type",
    }];
    let retirements = [RawRetirement {
        key: [3; 17],
        digest: [0x61; 32],
        bytes: b"original-retirement",
    }];
    let source = fixture(b"unchanged-raw-core", &owners, &types, &retirements);
    let bytes = carrier(&source)?;
    assert_eq!(&bytes[..8], b"LKJSG002");
    assert_eq!(word(&bytes, 8)?, bytes.len());
    assert_eq!(
        [word(&bytes, 12)?, word(&bytes, 16)?, word(&bytes, 20)?],
        [2, 1, 1]
    );
    assert_eq!(word(&bytes, 112)?, 3);
    assert_eq!(word(&bytes, 124)?, 0);
    let provenance_at = word(&bytes, 116)?;
    assert_eq!(
        provenance_at,
        SOURCE_HEADER_BYTES + source.package_name.len()
    );
    assert_eq!(
        &bytes[SOURCE_HEADER_BYTES..provenance_at],
        source.package_name
    );
    assert_eq!(
        at::<32>(&bytes, provenance_at)?,
        source.provenance.claimed_revision
    );
    let owner_at = word(&bytes, 80)?;
    assert_eq!(
        owner_at,
        provenance_at + 32 + source.provenance.canonical_core.len()
    );
    assert_eq!(
        &bytes[provenance_at + 32..owner_at],
        source.provenance.canonical_core
    );
    assert_eq!(word(&bytes, 84)?, owner_at + 2 * OWNER_ROW_BYTES);
    assert_eq!(word(&bytes, 88)?, word(&bytes, 84)? + TYPE_ROW_BYTES);
    assert_eq!(word(&bytes, 92)?, word(&bytes, 88)? + RETIREMENT_ROW_BYTES);
    for (index, owner) in source.owners.iter().enumerate() {
        let row = owner_at + index * OWNER_ROW_BYTES;
        assert_eq!(at::<17>(&bytes, row)?, owner.key);
        assert_eq!(bytes[row + 17], owner.kind);
        assert_eq!(at::<32>(&bytes, row + 18)?, owner.digest);
    }
    assert_eq!(at::<32>(&bytes, word(&bytes, 84)?)?, types[0].digest);
    assert_eq!(at::<17>(&bytes, word(&bytes, 88)?)?, retirements[0].key);
    assert_eq!(
        at::<32>(&bytes, word(&bytes, 88)? + 17)?,
        retirements[0].digest
    );
    let mut expected_at = word(&bytes, 96)?;
    assert_eq!(expected_at, word(&bytes, 92)? + 4 * SPAN_ROW_BYTES);
    for (index, object) in source.objects().enumerate() {
        let span = word(&bytes, 92)? + index * SPAN_ROW_BYTES;
        assert_eq!(word(&bytes, span)?, expected_at);
        assert_eq!(word(&bytes, span + 4)?, object.len());
        assert_eq!(&bytes[expected_at..expected_at + object.len()], object);
        expected_at += object.len();
    }
    assert_eq!(expected_at, bytes.len());
    Ok(())
}

#[test]
fn source_input_none_and_literal_finite_policies_have_distinct_encodings() -> Result<(), Diagnostic>
{
    let source = fixture(b"raw-core", &[], &[], &[]);
    for allowance in [None, Some(0), Some(9264), Some(9265), Some(u64::MAX)] {
        let bytes = carrier_with_allowance(&source, allowance)?;
        assert_eq!(word(&bytes, 100)?, usize::from(allowance.is_some()));
        let value = allowance.unwrap_or_default();
        assert_eq!(u64::from_le_bytes(at(&bytes, 104)?), value);
    }
    assert_eq!(carrier(&source)?, carrier_with_allowance(&source, None)?);
    assert_ne!(
        carrier_with_allowance(&source, Some(u64::MAX))?,
        carrier(&source)?
    );
    Ok(())
}

#[test]
fn source_input_preserves_malformed_claims_for_source_reader_negatives() -> Result<(), Diagnostic> {
    let owners = [RawOwner {
        key: [0; 17],
        kind: 255,
        digest: [0; 32],
        bytes: b"invalid owner",
    }];
    let types = [RawType {
        digest: [7; 32],
        bytes: b"invalid unused type",
    }];
    for core in [b"".as_slice(), b"noncanonical-core".as_slice()] {
        let mut source = fixture(core, &owners, &types, &[]);
        source.package = [0; 16];
        source.package_name = b"";
        source.provenance.claimed_revision = [0; 32];
        let bytes = carrier(&source)?;
        assert_eq!(at::<16>(&bytes, 32)?, [0; 16]);
        assert_eq!(word(&bytes, 120)?, 32 + core.len());
        assert_eq!(at::<32>(&bytes, word(&bytes, 116)?)?, [0; 32]);
        assert_eq!(at::<32>(&bytes, 48)?, source.semantic_state);
        assert_eq!(bytes[word(&bytes, 80)? + 17], 255);
        assert_eq!(&bytes[word(&bytes, 116)? + 32..word(&bytes, 80)?], core);
    }
    Ok(())
}

#[test]
fn source_input_request_mode_and_target_never_filter_the_carrier() -> Result<(), Diagnostic> {
    let source = carrier(&fixture(b"raw-core", &[], &[], &[]))?;
    let target = TargetId::from_bytes([0x71; 16]).ok_or_else(layout_error)?;
    let other = TargetId::from_bytes([0x72; 16]).ok_or_else(layout_error)?;
    let elf = request(&source, target, SourceOutputMode::Elf)?;
    let artifact = request(&source, target, SourceOutputMode::CanonicalArtifact)?;
    let alternate = request(&source, other, SourceOutputMode::CanonicalArtifact)?;
    assert_eq!(&elf[..8], b"LKJSRQ01");
    assert_eq!(word(&elf, 8)?, 2);
    assert_eq!(elf[12], 12);
    assert_eq!(at::<16>(&elf, 13)?, target.bytes());
    assert_eq!(at::<16>(&alternate, 13)?, other.bytes());
    assert_eq!([elf[29], artifact[29]], [0, 1]);
    for bytes in [&elf, &artifact, &alternate] {
        assert_eq!(&bytes[30..32], &[0, 0]);
        assert_eq!(&bytes[32..], source.as_slice());
    }
    // Malformed embedded framing must also reach the actual source reader.
    assert_eq!(
        &request(b"bad-source", target, SourceOutputMode::Elf)?[32..],
        b"bad-source"
    );
    Ok(())
}

#[test]
fn source_input_extent_guards_do_not_require_enormous_allocations() -> Result<(), Diagnostic> {
    assert!(source_extent(usize::MAX, 1, 1).is_err());
    assert!(source_extent(0, usize::MAX, 2).is_err());
    assert!(contiguous_extent(isize::MAX as usize, 1, 1).is_err());
    if usize::BITS >= 64 {
        let largest = usize::try_from(u32::MAX).map_err(|_| extent_error())?;
        assert_eq!(source_extent(0, largest, 1)?, largest);
        assert!(source_extent(largest, 1, 1).is_err());
        assert!(source_extent(0, largest, OWNER_ROW_BYTES).is_err());
    }
    let mut output = Output::new(2)?;
    assert!(output.append(b"too long").is_err());
    output.append(b"ok")?;
    assert_eq!(output.finish()?, b"ok");
    Ok(())
}

#[test]
fn source_input_raw_framing_has_no_former_four_or_eight_mib_object_default()
-> Result<(), Diagnostic> {
    // Raw framing control only, not proof of semantic validity or a complete
    // compiler/storage path crossing these historical independent boundaries.
    let length = 8 * 1_048_576 + 17;
    let mut object = Vec::new();
    object
        .try_reserve_exact(length)
        .map_err(|_| allocation_error())?;
    object.resize(length, 0xa5);
    let owners = [RawOwner {
        key: [1; 17],
        kind: 1,
        digest: [9; 32],
        bytes: &object,
    }];
    let bytes = carrier(&fixture(b"raw-core", &owners, &[], &[]))?;
    assert_eq!(word(&bytes, word(&bytes, 92)? + 4)?, length);
    assert_eq!(&bytes[word(&bytes, 96)?..], object.as_slice());
    assert_eq!(word(&bytes, 100)?, 0);
    Ok(())
}

#[test]
fn source_input_public_export_matches_independently_framed_current_fixture()
-> Result<(), Diagnostic> {
    let container = public_container()?;
    let exported = ExportedSource::from_container(&container)?;
    let source = exported.as_raw();
    assert_eq!(
        [
            source.owners.len(),
            source.types.len(),
            source.retirements.len()
        ],
        [351, 8, 0]
    );
    assert_eq!(source.provenance.canonical_core.len(), 86);
    let revision_key = ObjectKey::from_digest(
        ObjectDomain::PackageRevision,
        container.root.package_revision.bytes(),
    );
    let original = container
        .objects
        .get(&revision_key)
        .ok_or_else(layout_error)?;
    let exact_core = original.get(37..123).ok_or_else(layout_error)?;
    assert_eq!(source.provenance.canonical_core, exact_core);
    assert_eq!(
        source.provenance.canonical_core.as_ptr(),
        exact_core.as_ptr()
    );
    assert_eq!(carrier(&source)?, PUBLIC_CARRIER);
    for owner in source.owners {
        let key = ObjectKey::from_digest(ObjectDomain::Owner, owner.digest);
        let original = container.objects.get(&key).ok_or_else(layout_error)?;
        assert_eq!(owner.bytes.as_ptr(), original.as_ptr());
        assert_eq!(owner.bytes, original.as_slice());
    }
    Ok(())
}

#[test]
fn source_input_public_export_keeps_valid_unused_supplied_types() -> Result<(), Diagnostic> {
    let mut container = public_container()?;
    let before = carrier(&ExportedSource::from_container(&container)?.as_raw())?;
    let ty = TypeObject::new(TypeForm::Text)?;
    let (digest, bytes) = encode_type_object(&ty)?;
    let key = ObjectKey::from_digest(ObjectDomain::Type, digest.bytes());
    assert!(!container.objects.contains_key(&key));
    container.objects.insert(key, bytes);
    let exported = ExportedSource::from_container(&container)?;
    let source = exported.as_raw();
    assert_eq!(source.types.len(), 9);
    assert!(source.types.iter().any(|row| row.digest == digest.bytes()));
    let after = carrier(&source)?;
    assert_eq!(at::<32>(&before, 48)?, at::<32>(&after, 48)?);
    assert_ne!(before, after);
    Ok(())
}

#[test]
fn source_input_public_export_does_not_host_admit_an_unused_type() -> Result<(), Diagnostic> {
    let mut container = public_container()?;
    let bytes = copied(b"hash-consistent malformed unused type")?;
    let key = ObjectKey::for_bytes(ObjectDomain::Type, &bytes);
    container.objects.insert(key, bytes);
    let exported = ExportedSource::from_container(&container)?;
    let source = exported.as_raw();
    assert_eq!(source.types.len(), 9);
    let ty = source
        .types
        .iter()
        .find(|row| row.digest == key.digest.bytes())
        .ok_or_else(layout_error)?;
    assert_eq!(ty.bytes, b"hash-consistent malformed unused type");
    assert!(carrier(&source).is_ok());
    Ok(())
}

#[test]
fn source_input_public_export_rejects_missing_or_unreferenced_original_objects()
-> Result<(), Diagnostic> {
    let mut missing = public_container()?;
    let key = missing
        .objects
        .keys()
        .find(|key| key.domain == ObjectDomain::Owner)
        .copied()
        .ok_or_else(layout_error)?;
    missing.objects.remove(&key);
    assert!(ExportedSource::from_container(&missing).is_err());
    let mut additional = public_container()?;
    let bytes = copied(b"unreferenced original owner")?;
    let key = ObjectKey::for_bytes(ObjectDomain::Owner, &bytes);
    additional.objects.insert(key, bytes);
    assert!(ExportedSource::from_container(&additional).is_err());
    Ok(())
}

#[test]
fn source_input_public_export_rechecks_object_hashes() -> Result<(), Diagnostic> {
    let mut container = public_container()?;
    let bytes = container
        .objects
        .iter_mut()
        .find(|(key, _)| key.domain == ObjectDomain::Type)
        .map(|(_, bytes)| bytes)
        .ok_or_else(layout_error)?;
    let first = bytes.first_mut().ok_or_else(layout_error)?;
    *first ^= 1;
    assert!(ExportedSource::from_container(&container).is_err());
    Ok(())
}
