//! Borrow original records from one current dependency-free public source export.
//!
//! Canonical object/map parsing establishes physical bindings and completeness.
//! It is not kernel source admission. In particular no owner/type decoder,
//! witness reconstruction, interface projection, or compiler lowering runs here.

use super::{
    RawOwner, RawRetirement, RawRevisionProvenance, RawSourceGraph, RawType, allocation_error,
    contiguous_extent, extent_error, source_extent, wire_count,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    PackageRevisionDigest, SemanticRoot, SemanticRootDigest, decode_root,
};
use crate::platform::package_transport::source::PackageContainer;
use crate::platform::package_transport::{
    PACKAGE_TRANSPORT_ENVELOPE_DOMAIN, PACKAGE_TRANSPORT_MAGIC, PackageRevision,
};
use crate::platform::persistent_map::{
    MapAdmission, MapError, MapErrorClass, MapRoot, MapWork, PersistentMap,
};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::storage::page_store::ObjectPageReader;
use bincode::enc::write::Writer;
use bincode::error::EncodeError;
use bincode::{Decode, Encode};

/// An owning directory over borrowed original object/core bytes. The only owned
/// payload is the small decoded root (including its package name); original
/// source objects and the exact core remain borrowed from `PackageContainer`.
pub(crate) struct ExportedSource<'a> {
    root: SemanticRoot,
    semantic_state: [u8; 32],
    provenance: RawRevisionProvenance<'a>,
    owners: Vec<RawOwner<'a>>,
    types: Vec<RawType<'a>>,
    retirements: Vec<RawRetirement<'a>>,
}

impl<'a> ExportedSource<'a> {
    /// `PackageContainer::decode` is the existing public raw container reader;
    /// callers do not call `.admit()` or supply an admitted snapshot. This method
    /// also rechecks every object it uses, so a manually assembled container is
    /// not treated as trusted semantic evidence.
    pub(crate) fn from_container(container: &'a PackageContainer) -> Result<Self, Diagnostic> {
        if container.selections.as_slice() != [container.root] {
            return Err(profile_error(
                "raw source profile 3 requires one exact root selection and no dependencies",
            ));
        }
        let revision_bytes = object(
            container,
            ObjectDomain::PackageRevision,
            container.root.package_revision.bytes(),
        )?;
        // This existing decoder checks only the canonical publication metadata,
        // its local history contract, and object/envelope hashes. No source body
        // is decoded, inferred, or admitted by this operation.
        let revision = PackageRevision::decode(revision_bytes, container.root.package_revision)?;
        if !revision.dependencies.is_empty() {
            return Err(profile_error(
                "source dependencies are outside current source profile 3",
            ));
        }
        let canonical_core = core_window(revision_bytes, &revision)?;
        let provenance = RawRevisionProvenance {
            claimed_revision: revision_digest(canonical_core)?,
            canonical_core,
        };
        let transport_bytes = object(
            container,
            ObjectDomain::PackageTransport,
            container.root.transport.bytes(),
        )?;
        let transport = transport_source_prefix(transport_bytes)?;
        if transport.package_revision != container.root.package_revision {
            return Err(binding_error(
                "export transport names a different exact package revision",
            ));
        }
        let root = decode_root(
            object(
                container,
                ObjectDomain::SemanticRoot,
                transport.semantic_root.bytes(),
            )?,
            transport.semantic_root,
        )?;
        if root.repository_id != revision.revision.repository_id
            || root.package_id != revision.package
        {
            return Err(binding_error(
                "export root and exact revision core disagree on repository or package",
            ));
        }
        if root.dependencies.entries() != 0 {
            return Err(profile_error(
                "source dependencies are outside current source profile 3",
            ));
        }
        // Visit even the empty map so an absent/corrupt dependency root cannot
        // silently disappear during extraction.
        visit_map(container, root.dependencies, |_key, _value| {
            Err(profile_error("source dependency map is not empty"))
        })?;

        let mut owners = Vec::new();
        visit_map(container, root.owners, |key, value| {
            let key: [u8; 17] = fixed(key)?;
            let binding: [u8; 33] = fixed(value)?;
            let digest = fixed(&binding[1..])?;
            let bytes = object(container, ObjectDomain::Owner, digest)?;
            push(
                &mut owners,
                RawOwner {
                    key,
                    kind: binding[0],
                    digest,
                    bytes,
                },
            )
        })?;
        let mut retirements = Vec::new();
        visit_map(container, root.retirements, |key, value| {
            let key = fixed(key)?;
            let digest = fixed(value)?;
            let bytes = object(container, ObjectDomain::Retirement, digest)?;
            push(&mut retirements, RawRetirement { key, digest, bytes })
        })?;
        let mut types = Vec::new();
        // Full container iteration, deliberately independent of type reachability.
        // Type objects supplied but unused by the selected program remain in S.
        for (key, bytes) in &container.objects {
            match key.domain {
                ObjectDomain::Type => {
                    let digest = key.digest.bytes();
                    verify_object(*key, bytes)?;
                    push(&mut types, RawType { digest, bytes })?;
                }
                ObjectDomain::Owner | ObjectDomain::Retirement => {}
                ObjectDomain::Blob | ObjectDomain::Dependency => {
                    return Err(profile_error(
                        "blob/dependency objects are outside current source profile 3",
                    ));
                }
                ObjectDomain::MapPage
                | ObjectDomain::SemanticRoot
                | ObjectDomain::PackageRevision
                | ObjectDomain::PackageTransport
                | ObjectDomain::PackageInterface => {}
                _ => {
                    return Err(inventory_error(
                        "source export contains a non-source object domain",
                    ));
                }
            }
        }
        verify_inventory(
            container,
            ObjectDomain::Owner,
            owners.iter().map(|owner| owner.digest),
        )?;
        verify_inventory(
            container,
            ObjectDomain::Retirement,
            retirements.iter().map(|retirement| retirement.digest),
        )?;
        let result = Self {
            root,
            semantic_state: revision.revision.semantic_state.bytes(),
            provenance,
            owners,
            types,
            retirements,
        };
        // Check the whole output representation before exposing the view. This
        // remains framing validation; the source reader recomputes semantic state.
        super::SourceLayout::measure(&result.as_raw())?;
        Ok(result)
    }

    pub(crate) fn as_raw(&self) -> RawSourceGraph<'_> {
        RawSourceGraph {
            package: self.root.package_id.bytes(),
            package_name: self.root.package_name.as_str().as_bytes(),
            semantic_state: self.semantic_state,
            provenance: self.provenance,
            owners: &self.owners,
            types: &self.types,
            retirements: &self.retirements,
        }
    }
}

/// The transport prefix is only a locator. Historical witness/interface fields
/// after this prefix are authenticated as raw object bytes but never decoded or
/// used as evidence, and never copied into the compiler input.
#[derive(Decode, Encode)]
struct TransportSourcePrefix {
    contract_version: u16,
    graph_contract_version: u16,
    package_revision: PackageRevisionDigest,
    semantic_root: SemanticRootDigest,
}

fn transport_source_prefix(bytes: &[u8]) -> Result<TransportSourcePrefix, Diagnostic> {
    let payload = canonical_payload(
        bytes,
        PACKAGE_TRANSPORT_MAGIC,
        PACKAGE_TRANSPORT_ENVELOPE_DOMAIN,
    )?;
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding();
    let (prefix, consumed): (TransportSourcePrefix, usize) =
        bincode::decode_from_slice(payload, configuration).map_err(|_| {
            binding_error("source transport has an invalid canonical locator prefix")
        })?;
    if prefix.contract_version
        != crate::platform::package_transport::PACKAGE_TRANSPORT_CONTRACT_VERSION
        || !crate::platform::kernel::contract::supported_graph_contract(
            prefix.graph_contract_version,
        )
    {
        return Err(binding_error(
            "source transport has a foreign current contract",
        ));
    }
    let original = payload
        .get(..consumed)
        .ok_or_else(|| binding_error("transport prefix extent"))?;
    let mut comparison = EncodingWindow {
        original: Some(original),
        length: 0,
    };
    bincode::encode_into_writer(&prefix, &mut comparison, configuration)
        .map_err(|_| binding_error("source transport locator prefix is not canonical"))?;
    if comparison.length != consumed {
        return Err(binding_error(
            "source transport locator prefix is not canonical",
        ));
    }
    Ok(prefix)
}

fn core_window<'a>(bytes: &'a [u8], revision: &PackageRevision) -> Result<&'a [u8], Diagnostic> {
    // PackageRevision::decode has already proved the complete original canonical
    // encoding. Count its prefix/core with the SAME canonical encoder, allocating
    // no second payload. Only the corresponding ORIGINAL byte window is returned.
    let prefix = encoded_length(&(
        revision.contract_version,
        revision.graph_contract_version,
        revision.package,
    ))?;
    let length = encoded_length(&revision.revision)?;
    let start = contiguous_extent(crate::platform::packed::CANONICAL_HEADER_BYTES, prefix, 1)?;
    let end = contiguous_extent(start, length, 1)?;
    bytes
        .get(start..end)
        .ok_or_else(|| binding_error("canonical revision core extent"))
}

fn encoded_length(value: &impl Encode) -> Result<usize, Diagnostic> {
    let mut counter = EncodingWindow {
        original: None,
        length: 0,
    };
    bincode::encode_into_writer(
        value,
        &mut counter,
        bincode::config::standard()
            .with_little_endian()
            .with_variable_int_encoding(),
    )
    .map_err(|_| binding_error("canonical publication field extent cannot be measured"))?;
    Ok(counter.length)
}

struct EncodingWindow<'a> {
    original: Option<&'a [u8]>,
    length: usize,
}

impl Writer for EncodingWindow<'_> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let end = self
            .length
            .checked_add(bytes.len())
            .ok_or(EncodeError::Other("encoded extent"))?;
        if let Some(original) = self.original
            && original.get(self.length..end) != Some(bytes)
        {
            return Err(EncodeError::Other("noncanonical source locator"));
        }
        self.length = end;
        Ok(())
    }
}

fn revision_digest(core: &[u8]) -> Result<[u8; 32], Diagnostic> {
    let length = u64::try_from(core.len()).map_err(|_| extent_error())?;
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-revision");
    hasher.update(&crate::platform::canonical_schema::SCHEMA_EPOCH.to_le_bytes());
    hasher.update(&length.to_be_bytes());
    hasher.update(core);
    Ok(*hasher.finalize().as_bytes())
}

fn canonical_payload<'a>(
    bytes: &'a [u8],
    magic: [u8; 8],
    domain: &str,
) -> Result<&'a [u8], Diagnostic> {
    let header_bytes = crate::platform::packed::CANONICAL_HEADER_BYTES;
    if bytes.get(..8) != Some(magic.as_slice())
        || bytes.get(8..10)
            != Some(
                crate::platform::canonical_schema::SCHEMA_EPOCH
                    .to_le_bytes()
                    .as_slice(),
            )
    {
        return Err(binding_error(
            "source locator object has a foreign canonical envelope",
        ));
    }
    let encoded_length: [u8; 8] = fixed(
        bytes
            .get(10..18)
            .ok_or_else(|| binding_error("locator length"))?,
    )?;
    let payload_length =
        usize::try_from(u64::from_le_bytes(encoded_length)).map_err(|_| extent_error())?;
    let checksum_at = contiguous_extent(header_bytes, payload_length, 1)?;
    if contiguous_extent(checksum_at, 32, 1)? != bytes.len() {
        return Err(binding_error(
            "source locator object length disagrees with its envelope",
        ));
    }
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    let checksum_length = u64::try_from(checksum_at).map_err(|_| extent_error())?;
    hasher.update(&checksum_length.to_be_bytes());
    hasher.update(&bytes[..checksum_at]);
    if bytes[checksum_at..] != *hasher.finalize().as_bytes() {
        return Err(binding_error("source locator envelope checksum disagrees"));
    }
    Ok(&bytes[header_bytes..checksum_at])
}

fn object(
    container: &PackageContainer,
    domain: ObjectDomain,
    digest: [u8; 32],
) -> Result<&[u8], Diagnostic> {
    let key = ObjectKey::from_digest(domain, digest);
    let bytes = container.objects.get(&key).ok_or_else(|| {
        inventory_error("public source export is missing an exact referenced object")
    })?;
    verify_object(key, bytes)?;
    Ok(bytes)
}

fn verify_object(key: ObjectKey, bytes: &[u8]) -> Result<(), Diagnostic> {
    source_extent(0, bytes.len(), 1)?;
    // Hash verification is independent of storage's remaining logical-domain
    // defaults. The current carrier representation itself is checked above.
    if ObjectKey::for_bytes(key.domain, bytes) != key {
        return Err(binding_error(
            "original source object bytes disagree with their domain digest",
        ));
    }
    Ok(())
}

fn visit_map(
    container: &PackageContainer,
    root: MapRoot,
    mut visitor: impl FnMut(&[u8], &[u8]) -> Result<(), Diagnostic>,
) -> Result<(), Diagnostic> {
    let count = usize::try_from(root.entries()).map_err(|_| extent_error())?;
    wire_count(count)?;
    let reader = ObjectPageReader::new(&container.objects);
    let mut work = MapWork::with_admission(MapAdmission::unbounded());
    let mut failure = None;
    let result = PersistentMap::from_root(root).for_each(&reader, &mut work, |key, value| {
        match visitor(key, value) {
            Ok(()) => Ok(()),
            Err(diagnostic) => {
                failure = Some(diagnostic);
                Err(MapError {
                    class: MapErrorClass::Input,
                    code: "compiler_source_export_visit",
                    message: String::new(),
                })
            }
        }
    });
    if let Some(diagnostic) = failure {
        return Err(diagnostic);
    }
    result.map_err(|error| {
        Diagnostic::new(
            match error.class {
                MapErrorClass::Resource => DiagnosticClass::Resource,
                MapErrorClass::Store => DiagnosticClass::Infrastructure,
                MapErrorClass::Input | MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            },
            "compiler_source_export_map",
            error.message,
        )
    })
}

fn verify_inventory(
    container: &PackageContainer,
    domain: ObjectDomain,
    digests: impl Iterator<Item = [u8; 32]>,
) -> Result<(), Diagnostic> {
    // Compare exact sets, not only counts: duplicate map references must not hide
    // a supplied original object with no directory row. Unstable array sorting
    // allocates no scratch and equality does not depend on duplicate tie order.
    let mut referenced = Vec::new();
    for digest in digests {
        push(&mut referenced, digest)?;
    }
    referenced.sort_unstable();
    if referenced.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(inventory_error(
            "source root map repeats an original object digest",
        ));
    }
    let supplied = container
        .objects
        .keys()
        .filter(|key| key.domain == domain)
        .map(|key| key.digest.bytes());
    if !referenced.into_iter().eq(supplied) {
        return Err(inventory_error(
            "source export contains original owners or retirements outside its complete root maps",
        ));
    }
    Ok(())
}

fn push<T>(output: &mut Vec<T>, value: T) -> Result<(), Diagnostic> {
    let count = output.len().checked_add(1).ok_or_else(extent_error)?;
    wire_count(count)?;
    contiguous_extent(0, count, std::mem::size_of::<T>())?;
    output.try_reserve(1).map_err(|_| allocation_error())?;
    output.push(value);
    Ok(())
}

fn fixed<const N: usize>(bytes: &[u8]) -> Result<[u8; N], Diagnostic> {
    bytes
        .try_into()
        .map_err(|_| binding_error("canonical source directory field has the wrong width"))
}

fn binding_error(message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "compiler_source_export_binding",
        message,
    )
}

fn inventory_error(message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "compiler_source_export_inventory",
        message,
    )
}

fn profile_error(message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_source_export_profile",
        message,
    )
}

#[cfg(test)]
mod inventory_tests {
    use super::*;
    use crate::platform::kernel::PackageTransportDigest;
    use crate::platform::package_transport::PackageTransportBinding;
    use std::collections::BTreeMap;

    #[test]
    fn source_input_export_set_comparison_cannot_hide_orphans_behind_duplicate_references()
    -> Result<(), Diagnostic> {
        // Isolated set comparison: these deliberately raw empty objects are not
        // claimed as a valid export or passed to any source semantic decoder.
        let root = PackageTransportBinding {
            package_revision: PackageRevisionDigest::from_bytes([1; 32]),
            transport: PackageTransportDigest::from_bytes([2; 32]),
        };
        let mut objects = BTreeMap::new();
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, [3; 32]),
            Vec::new(),
        );
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, [4; 32]),
            Vec::new(),
        );
        let container = PackageContainer {
            root,
            selections: Vec::new(),
            objects,
        };
        assert!(
            verify_inventory(
                &container,
                ObjectDomain::Owner,
                [[3; 32], [3; 32]].into_iter()
            )
            .is_err()
        );
        assert!(verify_inventory(&container, ObjectDomain::Owner, [[3; 32]].into_iter()).is_err());
        verify_inventory(
            &container,
            ObjectDomain::Owner,
            [[4; 32], [3; 32]].into_iter(),
        )?;
        Ok(())
    }
}
