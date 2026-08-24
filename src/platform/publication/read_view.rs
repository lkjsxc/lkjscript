//! Revision-pinned, bounded reads over accepted Graph 5 authority and its committed witness.

use super::CurrentPublication;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, EncodedOwnerKey, OwnerKey, OwnerRecord, PackageId, RelationEdge,
    RelationEndpoint, RelationKind, RetirementRecord, TypeObject, TypeObjectDigest,
    decode_dependency, decode_dependency_binding, decode_owner, decode_owner_binding,
    decode_retirement, decode_retirement_binding, decode_type_object, dependency_map_key,
    owner_map_key, retirement_map_key,
};
use crate::platform::persistent_map::{MapError, MapErrorClass, MapRoot, MapWork, PersistentMap};
use crate::platform::semantic_id::RevisionId;
use crate::platform::storage::directory::PackDirectoryStore;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use crate::platform::witness::{
    NamespaceKey, OwnerSummary, OwnershipEntry, SummaryBinding, decode_forward_relation_key,
    decode_owner_summary, decode_ownership, decode_reverse_relation_key, forward_relation_prefix,
    owner_key_bytes, reverse_relation_prefix,
};

pub const MAXIMUM_RELATION_READ_ITEMS: usize = 10_000;
const RELATION_LIMIT_STOP: &str = "publication_relation_limit_stop";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RepositoryReadWork {
    pub map: MapWork,
    pub store: StoreWork,
    pub canonical_records_decoded: u64,
    pub witness_records_decoded: u64,
    pub items_returned: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionRead<T> {
    pub revision: RevisionId,
    pub value: T,
    pub work: RepositoryReadWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationRead {
    pub edges: Vec<RelationEdge>,
    pub truncated: bool,
}

/// One immutable catalog snapshot plus the exact accepted revision it was opened against.
///
/// Packs are append-only in the current Graph 5 store, so a later HEAD publication cannot alter
/// any object visible through this view. Future physical deletion must add an explicit lease
/// before it may coexist with these views.
#[derive(Debug)]
pub struct RepositoryView {
    current: CurrentPublication,
    store: PackDirectoryStore,
}

impl RepositoryView {
    pub(super) const fn new(current: CurrentPublication, store: PackDirectoryStore) -> Self {
        Self { current, store }
    }

    pub const fn revision(&self) -> RevisionId {
        self.current.head.revision
    }

    pub const fn package(&self) -> PackageId {
        self.current.semantic_root.package_id
    }

    pub const fn current(&self) -> &CurrentPublication {
        &self.current
    }

    pub fn owner(&self, owner: OwnerKey) -> Result<RevisionRead<Option<OwnerRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.semantic_root.owners,
            &owner_map_key(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_owner_binding(&binding_bytes, owner)?;
        let bytes = self.read_required_object(
            ObjectDomain::Owner,
            binding.object.bytes(),
            "accepted owner binding references a missing owner object",
            &mut work,
        )?;
        let record = decode_owner(&bytes, owner, binding.kind, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<RevisionRead<Option<TypeObject>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(bytes) =
            self.read_optional_object(ObjectDomain::Type, digest.bytes(), &mut work)?
        else {
            return Ok(self.read(None, work));
        };
        let object = decode_type_object(&bytes, digest)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(object), work))
    }

    pub fn dependency(
        &self,
        package: PackageId,
    ) -> Result<RevisionRead<Option<DependencyRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.semantic_root.dependencies,
            &dependency_map_key(package),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_dependency_binding(&binding_bytes)?;
        let bytes = self.read_required_object(
            ObjectDomain::Dependency,
            binding.object.bytes(),
            "accepted dependency binding references a missing dependency object",
            &mut work,
        )?;
        let record = decode_dependency(&bytes, &package, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<RetirementRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.semantic_root.retirements,
            &retirement_map_key(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_retirement_binding(&binding_bytes)?;
        let bytes = self.read_required_object(
            ObjectDomain::Retirement,
            binding.object.bytes(),
            "accepted retirement binding references a missing retirement object",
            &mut work,
        )?;
        let record = decode_retirement(&bytes, owner, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<RevisionRead<Option<OwnerKey>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map(
            self.current.witness.roots.namespaces,
            &key.encode(),
            &mut work,
        )?;
        let value = value.as_deref().map(EncodedOwnerKey::decode).transpose()?;
        work.witness_records_decoded = u64::from(value.is_some());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value, work))
    }

    pub fn ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<OwnershipEntry>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map(
            self.current.witness.roots.ownership,
            &owner_key_bytes(owner),
            &mut work,
        )?;
        let value = value.as_deref().map(decode_ownership).transpose()?;
        work.witness_records_decoded = u64::from(value.is_some());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value, work))
    }

    pub fn owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<OwnerSummary>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.witness.roots.owner_summaries,
            &owner_key_bytes(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = SummaryBinding::decode(&binding_bytes, owner)?;
        let bytes = self.read_required_object(
            ObjectDomain::OwnerSummary,
            binding.summary.bytes(),
            "accepted witness binding references a missing owner-summary object",
            &mut work,
        )?;
        let summary = decode_owner_summary(&bytes, binding.summary)?;
        if summary.owner != owner || summary.kind != binding.kind {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_summary_binding",
                "owner-summary object disagrees with its exact witness binding",
            ));
        }
        work.witness_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(summary), work))
    }

    pub fn outgoing_relations(
        &self,
        source: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(source, kind, maximum_items, false)
    }

    pub fn incoming_relations(
        &self,
        target: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(target, kind, maximum_items, true)
    }

    fn relations(
        &self,
        endpoint: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
        reverse: bool,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_RELATION_READ_ITEMS {
            return Err(read_error(
                DiagnosticClass::Resource,
                "publication_relation_item_budget",
                format!("relation item budget must be 1 through {MAXIMUM_RELATION_READ_ITEMS}"),
            ));
        }
        let prefix = if reverse {
            reverse_relation_prefix(endpoint, kind)
        } else {
            forward_relation_prefix(endpoint, kind)
        };
        let root = if reverse {
            self.current.witness.roots.reverse_relations
        } else {
            self.current.witness.roots.forward_relations
        };
        let mut work = RepositoryReadWork::default();
        let mut keys = Vec::with_capacity(maximum_items.min(64));
        let mut truncated = false;
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let result = PersistentMap::from_root(root).for_each_prefix(
            &reader,
            &prefix,
            &mut map_work,
            |key, value| {
                if !value.is_empty() {
                    return Err(MapError {
                        class: MapErrorClass::Corrupt,
                        code: "publication_relation_value",
                        message: "relation witness entry has a nonempty value".to_owned(),
                    });
                }
                if keys.len() == maximum_items {
                    truncated = true;
                    return Err(MapError {
                        class: MapErrorClass::Resource,
                        code: RELATION_LIMIT_STOP,
                        message: "bounded relation prefix has more matching entries".to_owned(),
                    });
                }
                keys.push(key.to_vec());
                Ok(())
            },
        );
        work.map = map_work;
        work.store = reader.work();
        if let Err(error) = result
            && error.code != RELATION_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        let mut edges = Vec::with_capacity(keys.len());
        for key_bytes in keys {
            let edge = if reverse {
                decode_reverse_relation_key(&key_bytes)?
            } else {
                decode_forward_relation_key(&key_bytes)?
            };
            let observed = if reverse { edge.target } else { edge.source };
            if observed != endpoint || kind.is_some_and(|expected| edge.kind != expected) {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_relation_prefix",
                    "decoded relation does not agree with its selected witness prefix",
                ));
            }
            edges.push(edge);
        }
        work.items_returned = edges.len() as u64;
        work.witness_records_decoded = edges.len() as u64;
        Ok(self.read(RelationRead { edges, truncated }, work))
    }

    fn lookup_map(
        &self,
        root: MapRoot,
        key: &[u8],
        work: &mut RepositoryReadWork,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let reader = ObjectPageReader::new(&self.store);
        let result = PersistentMap::from_root(root).lookup(&reader, key, &mut work.map);
        work.store = reader.work();
        result.map_err(map_diagnostic)
    }

    fn read_optional_object(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        work: &mut RepositoryReadWork,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        self.store
            .read(
                ObjectKey::from_digest(domain, digest),
                domain.maximum_bytes(),
                &mut work.store,
            )
            .map_err(store_diagnostic)
    }

    fn read_required_object(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        missing: &'static str,
        work: &mut RepositoryReadWork,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.read_optional_object(domain, digest, work)?
            .ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_read_object_missing",
                    missing,
                )
            })
    }

    fn read<T>(&self, value: T, work: RepositoryReadWork) -> RevisionRead<T> {
        RevisionRead {
            revision: self.revision(),
            value,
            work,
        }
    }
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Source,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    read_error(class, error.code, error.message)
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    read_error(class, error.code, error.message)
}

fn read_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
