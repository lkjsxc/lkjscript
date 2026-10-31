//! Raw authoritative source projection. This module cannot read normalized IR,
//! compiled units, flow certificates, or a host validation witness.
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{self, KernelSnapshot, OwnerKey};
use crate::platform::publication::RevisionCore;
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use std::collections::BTreeMap;

const HEADER: usize = 160;
const MAX_INPUT: usize = 8 * 1_048_576;
const MAX_RECORDS: usize = 100_000;

fn unsupported(detail: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_source_input_unsupported",
        detail,
    )
}

fn word(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
}

/// Serializes one complete local source snapshot, including unused owners.
/// Initial source inspection supports Owner v14 and Type v10. Blob-backed values
/// and imported packages remain explicit unsupported boundaries in this slice.
/// It deliberately does not call validate_full, build_clean, or a linker.
pub(crate) fn snapshot(
    source: &KernelSnapshot,
    revision: &RevisionCore,
    entry: OwnerKey,
    selector: u32,
) -> Result<Vec<u8>, Diagnostic> {
    if source.owners.is_empty()
        || source.owners.len() > MAX_RECORDS
        || source.retirements.len() > MAX_RECORDS
        || !source.dependencies.is_empty()
        || !source.dependency_types.is_empty()
        || !source.dependency_interfaces.is_empty()
        || !source.representation_dependencies.is_empty()
        || !source.blobs.is_empty()
        || selector > 1
        || !matches!(entry, OwnerKey::Declaration(_))
    {
        return Err(unsupported(
            "source inventory or selected output is outside the initial raw profile",
        ));
    }
    let root = source
        .owners
        .keys()
        .position(|key| *key == entry)
        .ok_or_else(|| unsupported("entry declaration is absent from the source inventory"))?;
    let mut owners = Vec::with_capacity(source.owners.len() * 50);
    let mut objects = BTreeMap::new();
    for (key, record) in &source.owners {
        let (digest, bytes) = kernel::encode_owner(record)?;
        owners.extend_from_slice(&kernel::owner_map_key(*key));
        owners.extend_from_slice(&kernel::encode_owner_binding(&kernel::OwnerBinding {
            kind: record.kind(),
            object: digest,
        }));
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        );
    }
    for (key, record) in &source.types {
        let (_, bytes) = kernel::encode_type_object(record)?;
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::Type, key.bytes()),
            bytes,
        );
    }
    if objects.len() > MAX_RECORDS {
        return Err(unsupported(
            "source object directory exceeds its fixed bound",
        ));
    }
    let mut retirements = Vec::with_capacity(source.retirements.len() * 49);
    for (key, record) in &source.retirements {
        let (digest, _) = kernel::encode_retirement(record)?;
        retirements.extend_from_slice(&kernel::retirement_map_key(*key));
        retirements.extend_from_slice(&digest.bytes());
    }
    // These are source identity fields, not semantic proofs. The lkjscript reader
    // recomputes all logical roots and binds the canonical revision independently.
    let core = bincode::encode_to_vec(revision, bincode::config::standard())
        .map_err(|error| unsupported(&format!("revision encoding: {error}")))?;
    let mut metadata = vec![0; 112];
    metadata[..16].copy_from_slice(&source.root.repository_id.bytes());
    metadata[16..32].copy_from_slice(&source.root.package_id.bytes());
    metadata[32..64].copy_from_slice(&revision.semantic_state.bytes());
    metadata[64..96].copy_from_slice(&revision.revision_id()?.bytes());
    let name = source.root.package_name.as_str().as_bytes();
    word(&mut metadata, 96, name.len());
    word(&mut metadata, 100, core.len());
    metadata.extend_from_slice(name);
    metadata.extend_from_slice(&core);
    let owner_at = HEADER;
    let dependency_at = owner_at + owners.len();
    let retirement_at = dependency_at;
    let object_at = retirement_at + retirements.len();
    let metadata_at = object_at + objects.len() * 48;
    let payload_at = metadata_at + metadata.len();
    let object_bytes = objects
        .values()
        .try_fold(0usize, |total, bytes| total.checked_add(bytes.len()))
        .ok_or_else(|| unsupported("source object bytes overflow"))?;
    let total = payload_at
        .checked_add(object_bytes)
        .filter(|total| *total <= MAX_INPUT)
        .ok_or_else(|| unsupported("complete raw source input exceeds 8 MiB"))?;
    let mut out = vec![0; payload_at];
    out[..8].copy_from_slice(b"LKJCI001");
    word(&mut out, 8, 5);
    word(&mut out, 12, root);
    word(&mut out, 16, total);
    word(&mut out, 24, source.owners.len());
    let fuel =
        4096 + 128 * total + 64 * (source.owners.len() + objects.len() + source.retirements.len());
    word(&mut out, 32, fuel);
    for (at, offset, count) in [
        (72, owner_at, source.owners.len()),
        (80, object_at, objects.len()),
        (88, metadata_at, metadata.len()),
        (96, dependency_at, 0),
        (104, retirement_at, source.retirements.len()),
    ] {
        word(&mut out, at, offset);
        word(&mut out, at + 4, count);
    }
    word(&mut out, 112, selector as usize);
    out[owner_at..dependency_at].copy_from_slice(&owners);
    out[retirement_at..object_at].copy_from_slice(&retirements);
    out[metadata_at..payload_at].copy_from_slice(&metadata);
    for (index, (key, bytes)) in objects.into_iter().enumerate() {
        let row = object_at + index * 48;
        word(&mut out, row, usize::from(key.domain.tag()));
        let offset = out.len();
        word(&mut out, row + 8, offset);
        word(&mut out, row + 12, bytes.len());
        out[row + 16..row + 48].copy_from_slice(&key.digest.bytes());
        out.extend_from_slice(&bytes);
    }
    let mut hash = blake3::Hasher::new();
    hash.update(&out[..40]);
    hash.update(&out[72..]);
    out[40..72].copy_from_slice(hash.finalize().as_bytes());
    Ok(out)
}
