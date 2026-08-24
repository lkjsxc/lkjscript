//! Exact revision-committed lookup records for accepted idempotent publications.

use super::contract::{
    IDEMPOTENCY_BINDING_CONTRACT_VERSION, IDEMPOTENCY_BINDING_ENVELOPE_DOMAIN,
    IDEMPOTENCY_BINDING_MAGIC, MAXIMUM_IDEMPOTENCY_BINDING_BYTES, MAXIMUM_IDEMPOTENCY_KEY_BYTES,
};
use super::{
    AcceptedBinding, HeadRecord, PublicationReceipt, ReceiptObjectDigest, TransactionDigest,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::persistent_map::{
    MapEdit, MapError, MapErrorClass, MapRoot, MapWork, PageStore, PersistentMap,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use bincode::{Decode, Encode};

const IDEMPOTENCY_KEY_TAG: u8 = 1;

/// One accepted publication named by a repository-wide idempotency key. The persistent history
/// map contains only ancestors of the revision that commits its root; the current revision's own
/// receipt remains directly available and is added by its first accepted child.
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct IdempotencyBinding {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub key: String,
    pub base: RevisionId,
    pub transaction: TransactionDigest,
    pub result: HeadRecord,
    pub receipt: ReceiptObjectDigest,
}

impl IdempotencyBinding {
    pub fn from_accepted(
        accepted: AcceptedBinding,
        receipt: &PublicationReceipt,
    ) -> Result<Option<Self>, Diagnostic> {
        let Some(key) = receipt.idempotency_key.clone() else {
            return Ok(None);
        };
        let [base] = receipt.bases.as_slice() else {
            return Err(idempotency_error(
                DiagnosticClass::Corrupt,
                "publication_idempotency_receipt_base",
                "accepted idempotency receipt does not have one exact base",
            ));
        };
        let binding = Self {
            contract_version: IDEMPOTENCY_BINDING_CONTRACT_VERSION,
            repository_id: accepted.head.repository_id,
            key,
            base: *base,
            transaction: receipt.transaction,
            result: accepted.head,
            receipt: accepted.receipt,
        };
        binding.validate()?;
        if receipt.repository_id != binding.repository_id
            || receipt.result != binding.result.revision
            || accepted.receipt != binding.receipt
        {
            return Err(idempotency_error(
                DiagnosticClass::Corrupt,
                "publication_idempotency_receipt_binding",
                "accepted idempotency receipt disagrees with its exact publication binding",
            ));
        }
        Ok(Some(binding))
    }

    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        crate::platform::packed::encode(
            IDEMPOTENCY_BINDING_MAGIC,
            IDEMPOTENCY_BINDING_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_IDEMPOTENCY_BINDING_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = crate::platform::packed::decode(
            bytes,
            IDEMPOTENCY_BINDING_MAGIC,
            IDEMPOTENCY_BINDING_ENVELOPE_DOMAIN,
            MAXIMUM_IDEMPOTENCY_BINDING_BYTES,
        )?;
        value.validate()?;
        if value.encode()? != bytes {
            return Err(idempotency_error(
                DiagnosticClass::Corrupt,
                "publication_idempotency_canonical",
                "idempotency binding is not canonically encoded",
            ));
        }
        Ok(value)
    }

    pub fn matches_prepared(&self, receipt: &PublicationReceipt) -> bool {
        receipt.repository_id == self.repository_id
            && receipt.idempotency_key.as_deref() == Some(self.key.as_str())
            && receipt.bases.as_slice() == [self.base]
            && receipt.transaction == self.transaction
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != IDEMPOTENCY_BINDING_CONTRACT_VERSION {
            return Err(idempotency_error(
                DiagnosticClass::Source,
                "publication_idempotency_contract",
                "idempotency binding uses a predecessor or foreign contract",
            ));
        }
        validate_idempotency_key(&self.key)?;
        self.result.encode()?;
        if self.repository_id != self.result.repository_id || self.base == self.result.revision {
            return Err(idempotency_error(
                DiagnosticClass::Corrupt,
                "publication_idempotency_result",
                "idempotency binding result, repository, record, or base is inconsistent",
            ));
        }
        Ok(())
    }
}

pub fn validate_idempotency_key(key: &str) -> Result<(), Diagnostic> {
    if !idempotency_key_is_valid(key) {
        return Err(idempotency_error(
            DiagnosticClass::Source,
            "publication_idempotency_key",
            "idempotency key must contain 1 through 128 portable identifier bytes",
        ));
    }
    Ok(())
}

pub fn idempotency_key_is_valid(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAXIMUM_IDEMPOTENCY_KEY_BYTES
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub fn empty_idempotency_history<P: PageStore + ?Sized>(
    pages: &mut P,
    work: &mut MapWork,
) -> Result<MapRoot, Diagnostic> {
    PersistentMap::empty(pages, work)
        .map(PersistentMap::root)
        .map_err(map_diagnostic)
}

pub fn advance_idempotency_history<P: PageStore + ?Sized>(
    root: MapRoot,
    binding: Option<&IdempotencyBinding>,
    pages: &mut P,
    work: &mut MapWork,
) -> Result<MapRoot, Diagnostic> {
    let Some(binding) = binding else {
        return Ok(root);
    };
    let key = idempotency_map_key(&binding.key)?;
    let map = PersistentMap::from_root(root);
    if map
        .lookup(pages, &key, work)
        .map_err(map_diagnostic)?
        .is_some()
    {
        return Err(idempotency_error(
            DiagnosticClass::Corrupt,
            "publication_idempotency_duplicate",
            "accepted history binds one idempotency key more than once",
        ));
    }
    let edit = MapEdit {
        key,
        before: None,
        after: Some(binding.encode()?),
    };
    let (updated, outcome) = map
        .apply_sorted_edits(pages, &[edit], work)
        .map_err(map_diagnostic)?;
    if outcome.inserted != 1
        || outcome.replaced != 0
        || outcome.removed != 0
        || outcome.unchanged != 0
    {
        return Err(idempotency_error(
            DiagnosticClass::Corrupt,
            "publication_idempotency_update",
            "idempotency history update did not insert exactly one binding",
        ));
    }
    Ok(updated.root())
}

pub fn lookup_idempotency_history<P: PageStore + ?Sized>(
    root: MapRoot,
    key: &str,
    pages: &P,
    work: &mut MapWork,
) -> Result<Option<IdempotencyBinding>, Diagnostic> {
    let encoded_key = idempotency_map_key(key)?;
    let Some(bytes) = PersistentMap::from_root(root)
        .lookup(pages, &encoded_key, work)
        .map_err(map_diagnostic)?
    else {
        return Ok(None);
    };
    let binding = IdempotencyBinding::decode(&bytes)?;
    if binding.key != key {
        return Err(idempotency_error(
            DiagnosticClass::Corrupt,
            "publication_idempotency_key_binding",
            "idempotency map key disagrees with its decoded binding key",
        ));
    }
    Ok(Some(binding))
}

fn idempotency_map_key(key: &str) -> Result<Vec<u8>, Diagnostic> {
    validate_idempotency_key(key)?;
    let length = u16::try_from(key.len()).map_err(|_| {
        idempotency_error(
            DiagnosticClass::Resource,
            "publication_idempotency_key_length",
            "idempotency key length exceeds its canonical map-key encoding",
        )
    })?;
    let mut bytes = Vec::with_capacity(3 + key.len());
    bytes.push(IDEMPOTENCY_KEY_TAG);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(key.as_bytes());
    Ok(bytes)
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Source,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    idempotency_error(class, error.code, error.message)
}

fn idempotency_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::persistent_map::MemoryPageStore;
    use crate::platform::semantic_id::RepositoryId;

    fn identity_bytes(ordinal: u64, tag: u8) -> [u8; 32] {
        let mut bytes = [tag; 32];
        bytes[..8].copy_from_slice(&ordinal.to_be_bytes());
        bytes
    }

    fn binding(key: &str, ordinal: u64) -> IdempotencyBinding {
        let repository_id = RepositoryId::migrate(b"idempotency-binding", 0);
        let base = RevisionId::from_digest(identity_bytes(ordinal, 1));
        let result = RevisionId::from_digest(identity_bytes(ordinal, 2));
        let revision_record =
            super::super::RevisionObjectDigest::from_bytes(identity_bytes(ordinal, 3));
        IdempotencyBinding {
            contract_version: IDEMPOTENCY_BINDING_CONTRACT_VERSION,
            repository_id,
            key: key.to_owned(),
            base,
            transaction: TransactionDigest::from_bytes(identity_bytes(ordinal, 4)),
            result: HeadRecord {
                contract_version: super::super::contract::REVISION_CONTRACT_VERSION,
                graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id,
                revision: result,
                record: revision_record,
            },
            receipt: ReceiptObjectDigest::from_bytes(identity_bytes(ordinal, 5)),
        }
    }

    #[test]
    fn idempotency_history_is_canonical_exact_and_strict() {
        let mut pages = MemoryPageStore::default();
        let mut work = MapWork::default();
        let empty = empty_idempotency_history(&mut pages, &mut work).unwrap();
        let first = binding("request-1", 0);
        let second = binding("request-2", 8);
        let root = advance_idempotency_history(empty, Some(&first), &mut pages, &mut work).unwrap();
        let root = advance_idempotency_history(root, Some(&second), &mut pages, &mut work).unwrap();
        assert_eq!(root.entries(), 2);
        assert_eq!(
            lookup_idempotency_history(root, "request-1", &pages, &mut work).unwrap(),
            Some(first.clone())
        );
        assert_eq!(
            lookup_idempotency_history(root, "request-2", &pages, &mut work).unwrap(),
            Some(second)
        );
        assert!(
            lookup_idempotency_history(root, "absent", &pages, &mut work)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            advance_idempotency_history(root, Some(&first), &mut pages, &mut work)
                .unwrap_err()
                .code,
            "publication_idempotency_duplicate"
        );

        let wrong_edit = MapEdit {
            key: idempotency_map_key("wrong-key").unwrap(),
            before: None,
            after: Some(first.encode().unwrap()),
        };
        let (wrong_map, _) = PersistentMap::from_root(root)
            .apply_sorted_edits(&mut pages, &[wrong_edit], &mut work)
            .unwrap();
        assert_eq!(
            lookup_idempotency_history(wrong_map.root(), "wrong-key", &pages, &mut work)
                .unwrap_err()
                .code,
            "publication_idempotency_key_binding"
        );

        let mut predecessor = first.encode().unwrap();
        predecessor[..8].copy_from_slice(b"LKJIDEM0");
        assert_eq!(
            IdempotencyBinding::decode(&predecessor).unwrap_err().code,
            "packed_contract"
        );
    }

    #[test]
    fn idempotency_lookup_work_is_bounded_below_history_size() {
        let mut pages = MemoryPageStore::default();
        let mut construction_work = MapWork::default();
        let mut root = empty_idempotency_history(&mut pages, &mut construction_work).unwrap();
        for ordinal in 0..512 {
            let binding = binding(&format!("request-{ordinal:04}"), ordinal);
            root = advance_idempotency_history(
                root,
                Some(&binding),
                &mut pages,
                &mut construction_work,
            )
            .unwrap();
        }
        assert_eq!(root.entries(), 512);

        let mut lookup_work = MapWork::default();
        assert_eq!(
            lookup_idempotency_history(root, "request-0256", &pages, &mut lookup_work)
                .unwrap()
                .unwrap()
                .key,
            "request-0256"
        );
        assert!(lookup_work.pages_read > 0 && lookup_work.pages_read < 32);
        assert!(lookup_work.entries_visited < 64);
        assert!(lookup_work.entries_visited < root.entries());
    }
}
