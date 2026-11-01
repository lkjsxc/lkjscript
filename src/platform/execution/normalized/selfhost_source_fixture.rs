//! Fixture-only provenance and raw-object preparation. All carrier/request bytes
//! are emitted by the production compiler::selfhost_source_input owner.
//!
//! Exact maintained source uses a public export. Mutated or independently authored
//! snapshot fixtures explicitly use synthetic history; this file never supplies
//! synthetic history to the production public-export path.

use super::*;
use crate::platform::compiler::selfhost_source_input::{
    self as source_input, ExportedSource, RawOwner, RawRetirement, RawRevisionProvenance,
    RawSourceGraph, RawType, SourceOutputMode,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::package_transport::source::PackageContainer;
use std::borrow::Cow;

/// Existing tests return byte buffers. Fail with the original diagnostic before
/// the fallback arm; production operations keep their ordinary Result contract.
pub(super) fn fixture_bytes(result: Result<Vec<u8>, Diagnostic>) -> Vec<u8> {
    assert!(
        result.is_ok(),
        "source fixture construction failed: {result:?}"
    );
    result.unwrap_or_default()
}

pub(super) fn packet(source: &KernelSnapshot) -> Vec<u8> {
    if std::ptr::eq(source, &compiler().0) {
        static MAINTAINED: OnceLock<Vec<u8>> = OnceLock::new();
        let bytes = MAINTAINED.get_or_init(|| fixture_bytes(maintained_public_packet(source)));
        return fixture_bytes(copy_bytes(bytes));
    }
    fixture_bytes(synthetic_fixture_packet(source, None, None, None, None))
}

pub(super) fn packet_with_fixture_commitment(
    source: &KernelSnapshot,
    state: kernel::SemanticStateDigest,
) -> Vec<u8> {
    fixture_bytes(synthetic_fixture_packet(
        source,
        Some(state),
        None,
        None,
        None,
    ))
}

pub(super) fn raw_object_packet(
    source: &KernelSnapshot,
    owner_override: Option<(OwnerKey, &[u8])>,
    extra_type: Option<&[u8]>,
) -> Vec<u8> {
    fixture_bytes(synthetic_fixture_packet(
        source,
        None,
        owner_override,
        extra_type,
        None,
    ))
}

/// This path deliberately does not validate, derive, or repair supplied claims.
/// Use it for invalid core/state/revision fixtures that must reach source readers.
pub(super) fn raw_claim_packet(
    source: &KernelSnapshot,
    state: kernel::SemanticStateDigest,
    provenance: RawRevisionProvenance<'_>,
) -> Result<Vec<u8>, Diagnostic> {
    synthetic_fixture_packet(source, Some(state), None, None, Some(provenance))
}

pub(super) fn request(carrier: &[u8], target: crate::platform::semantic_id::TargetId) -> Vec<u8> {
    fixture_bytes(source_input::request(
        carrier,
        target,
        SourceOutputMode::Elf,
    ))
}

fn maintained_public_packet(source: &KernelSnapshot) -> Result<Vec<u8>, Diagnostic> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let repository = GraphRepository::open(&root)?;
    let view = repository.view_current()?;
    let export = view.export_package_transport()?;
    let container = PackageContainer::decode(&export.container, export.transport_digest)?;
    let exported = ExportedSource::from_container(&container)?;
    let raw = exported.as_raw();
    // The compiler snapshot and exact export must be the same complete source,
    // including types omitted from semantic state. This check is an independent
    // fixture expectation; none of its results are supplied to source admission.
    if raw.package != source.root.package_id.bytes()
        || raw.package_name != source.root.package_name.as_str().as_bytes()
        || raw.semantic_state != kernel::semantic_state_digest(source)?.bytes()
        || raw.owners.len() != source.owners.len()
        || raw.types.len() != source.types.len()
        || raw.retirements.len() != source.retirements.len()
    {
        return Err(fixture_error(
            "maintained compiler snapshot and exact public export differ",
        ));
    }
    for ((key, record), row) in source.owners.iter().zip(raw.owners) {
        let (digest, bytes) = kernel::encode_owner(record)?;
        if row.key != kernel::owner_map_key(*key)
            || row.kind != record.kind().tag()
            || row.digest != digest.bytes()
            || row.bytes != bytes.as_slice()
        {
            return Err(fixture_error(
                "maintained compiler owner inventory changed during export",
            ));
        }
    }
    for ((key, record), row) in source.types.iter().zip(raw.types) {
        let (digest, bytes) = kernel::encode_type_object(record)?;
        if row.digest != key.bytes()
            || row.digest != digest.bytes()
            || row.bytes != bytes.as_slice()
        {
            return Err(fixture_error(
                "maintained compiler supplied type inventory changed during export",
            ));
        }
    }
    for ((key, record), row) in source.retirements.iter().zip(raw.retirements) {
        let (digest, bytes) = kernel::encode_retirement(record)?;
        if row.key != kernel::retirement_map_key(*key)
            || row.digest != digest.bytes()
            || row.bytes != bytes.as_slice()
        {
            return Err(fixture_error(
                "maintained compiler retirement inventory changed during export",
            ));
        }
    }
    source_input::carrier(&raw)
}

struct FixtureOwner<'a> {
    key: [u8; 17],
    kind: u8,
    digest: [u8; 32],
    bytes: Cow<'a, [u8]>,
}

struct FixtureType<'a> {
    digest: [u8; 32],
    bytes: Cow<'a, [u8]>,
}

struct FixtureRetirement {
    key: [u8; 17],
    digest: [u8; 32],
    bytes: Vec<u8>,
}

fn synthetic_fixture_packet(
    source: &KernelSnapshot,
    supplied_state: Option<kernel::SemanticStateDigest>,
    owner_override: Option<(OwnerKey, &[u8])>,
    extra_type: Option<&[u8]>,
    supplied_provenance: Option<RawRevisionProvenance<'_>>,
) -> Result<Vec<u8>, Diagnostic> {
    if !source.dependencies.is_empty()
        || !source.dependency_types.is_empty()
        || !source.blobs.is_empty()
    {
        return Err(fixture_error(
            "source fixture is outside the dependency-free carrier profile",
        ));
    }
    if let Some((owner, _)) = owner_override
        && !source.owners.contains_key(&owner)
    {
        return Err(fixture_error("raw fixture override names an absent owner"));
    }
    let mut owners = Vec::new();
    for (key, record) in &source.owners {
        let bytes = match owner_override {
            Some((changed, bytes)) if changed == *key => Cow::Borrowed(bytes),
            _ => Cow::Owned(kernel::encode_owner(record)?.1),
        };
        let digest = kernel::OwnerObjectDigest::of(&bytes).bytes();
        push(
            &mut owners,
            FixtureOwner {
                key: kernel::owner_map_key(*key),
                kind: record.kind().tag(),
                digest,
                bytes,
            },
        )?;
    }
    let mut types = Vec::new();
    for (key, object) in &source.types {
        let (digest, bytes) = kernel::encode_type_object(object)?;
        if *key != digest {
            return Err(fixture_error(
                "ordinary fixture type claim differs from its canonical object",
            ));
        }
        push(
            &mut types,
            FixtureType {
                digest: digest.bytes(),
                bytes: Cow::Owned(bytes),
            },
        )?;
    }
    if let Some(bytes) = extra_type {
        let digest = kernel::TypeObjectDigest::of(bytes).bytes();
        if let Some(existing) = types.iter_mut().find(|row| row.digest == digest) {
            existing.bytes = Cow::Borrowed(bytes);
        } else {
            push(
                &mut types,
                FixtureType {
                    digest,
                    bytes: Cow::Borrowed(bytes),
                },
            )?;
            types.sort_unstable_by_key(|row| row.digest);
        }
    }
    let mut retirements = Vec::new();
    for (key, record) in &source.retirements {
        let (digest, bytes) = kernel::encode_retirement(record)?;
        push(
            &mut retirements,
            FixtureRetirement {
                key: kernel::retirement_map_key(*key),
                digest: digest.bytes(),
                bytes,
            },
        )?;
    }
    let state = match supplied_state {
        Some(state) => state,
        None if owner_override.is_some() || extra_type.is_some() => {
            directory_state(source, &owners, &retirements)?
        }
        None => kernel::semantic_state_digest(source)?,
    };
    let synthetic_core;
    let provenance = match supplied_provenance {
        Some(provenance) => provenance,
        None => {
            // Explicit synthetic fixture history. Real exported history always
            // takes maintained_public_packet/ExportedSource and never this arm.
            let core = crate::platform::publication::RevisionCore {
                contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
                graph_contract_version: kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: source.root.repository_id,
                parents: Vec::new(),
                semantic_state: state,
            };
            let mut writer = FixtureEncoding(Vec::new());
            bincode::encode_into_writer(
                &core,
                &mut writer,
                bincode::config::standard()
                    .with_little_endian()
                    .with_variable_int_encoding(),
            )
            .map_err(|_| fixture_error("synthetic fixture core encoding failed"))?;
            synthetic_core = writer.0;
            RawRevisionProvenance {
                claimed_revision: core.revision_id()?.bytes(),
                canonical_core: &synthetic_core,
            }
        }
    };
    let mut raw_owners = Vec::new();
    for row in &owners {
        push(
            &mut raw_owners,
            RawOwner {
                key: row.key,
                kind: row.kind,
                digest: row.digest,
                bytes: &row.bytes,
            },
        )?;
    }
    let mut raw_types = Vec::new();
    for row in &types {
        push(
            &mut raw_types,
            RawType {
                digest: row.digest,
                bytes: &row.bytes,
            },
        )?;
    }
    let mut raw_retirements = Vec::new();
    for row in &retirements {
        push(
            &mut raw_retirements,
            RawRetirement {
                key: row.key,
                digest: row.digest,
                bytes: &row.bytes,
            },
        )?;
    }
    source_input::carrier(&RawSourceGraph {
        package: source.root.package_id.bytes(),
        package_name: source.root.package_name.as_str().as_bytes(),
        semantic_state: state.bytes(),
        provenance,
        owners: &raw_owners,
        types: &raw_types,
        retirements: &raw_retirements,
    })
}

fn directory_state(
    source: &KernelSnapshot,
    owners: &[FixtureOwner<'_>],
    retirements: &[FixtureRetirement],
) -> Result<kernel::SemanticStateDigest, Diagnostic> {
    use crate::platform::persistent_map::{MapContentRoot, MapRoot};
    let mut owner_rows = Vec::new();
    for owner in owners {
        let mut value = [0_u8; 33];
        value[0] = owner.kind;
        value[1..].copy_from_slice(&owner.digest);
        push(
            &mut owner_rows,
            (copy_bytes(&owner.key)?, copy_bytes(&value)?),
        )?;
    }
    let mut retirement_rows = Vec::new();
    for retirement in retirements {
        push(
            &mut retirement_rows,
            (
                copy_bytes(&retirement.key)?,
                copy_bytes(&retirement.digest)?,
            ),
        )?;
    }
    let owner_root =
        MapContentRoot::from_sorted(owner_rows).map_err(|error| fixture_error(&error.message))?;
    let retirement_root = MapContentRoot::from_sorted(retirement_rows)
        .map_err(|error| fixture_error(&error.message))?;
    let mut root = source.root.clone();
    root.owners = MapRoot::from_parts(
        root.owners.page(),
        owner_root.entries(),
        owner_root.digest(),
    );
    root.retirements = MapRoot::from_parts(
        root.retirements.page(),
        retirement_root.entries(),
        retirement_root.digest(),
    );
    kernel::semantic_state_digest_from_root(&root)
}

struct FixtureEncoding(Vec<u8>);

impl bincode::enc::write::Writer for FixtureEncoding {
    fn write(&mut self, bytes: &[u8]) -> Result<(), bincode::error::EncodeError> {
        self.0
            .try_reserve(bytes.len())
            .map_err(|_| bincode::error::EncodeError::Other("fixture core allocation"))?;
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes.len())
        .map_err(|_| fixture_error("fixture byte allocation"))?;
    output.extend_from_slice(bytes);
    Ok(output)
}

fn push<T>(output: &mut Vec<T>, value: T) -> Result<(), Diagnostic> {
    output
        .try_reserve(1)
        .map_err(|_| fixture_error("fixture directory allocation"))?;
    output.push(value);
    Ok(())
}

fn fixture_error(message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "selfhost_source_fixture",
        message,
    )
}

#[test]
fn selfhost_source_fixture_literal_core_claims_reach_source_rejection() -> Result<(), Diagnostic> {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
    ))
    .map_err(|message| fixture_error(&message))?;
    let state = kernel::semantic_state_digest(&source)?;
    let valid = synthetic_fixture_packet(&source, None, None, None, None)?;
    assert_eq!(
        super::result("read-carrier", &valid),
        (0, copy_bytes(&state.bytes())?)
    );
    let bad_core = b"deliberately noncanonical fixture core";
    let invalid = raw_claim_packet(
        &source,
        state,
        RawRevisionProvenance {
            claimed_revision: [0; 32],
            canonical_core: bad_core,
        },
    )?;
    assert_eq!(super::result("read-carrier", &invalid), (12, Vec::new()));
    Ok(())
}
