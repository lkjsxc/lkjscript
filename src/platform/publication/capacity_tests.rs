use super::*;
use crate::platform::kernel::{SemanticRootDigest, TypeObjectDigest};
use crate::platform::semantic_id::{RepositoryId, RevisionId};

#[test]
fn complete_history_codecs_exceed_former_byte_and_edit_inventories() {
    // Every supplied digest is distinct and ordered. This isolates the history
    // representation from full-publication admission, which the large native
    // public-authoring fixture independently exercises with real type objects.
    let supplied_types: Vec<_> = (0..131_073_u64)
        .map(|index| {
            let mut bytes = [0; 32];
            bytes[..8].copy_from_slice(&index.to_be_bytes());
            TypeObjectDigest::from_bytes(bytes)
        })
        .collect();
    let repository_id = RepositoryId::migrate(b"wide-history-inventory", 0);
    let base = RevisionId::from_digest([1; 32]);
    let base_root = SemanticRootDigest::from_bytes([2; 32]);
    let result_root = SemanticRootDigest::from_bytes([3; 32]);
    let transaction = NormalizedTransaction {
        contract_version: contract::TRANSACTION_CONTRACT_VERSION,
        graph_contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
        repository_id,
        body: TransactionBody::Change {
            base,
            base_root,
            result_root,
            owners: Vec::new(),
            supplied_types: supplied_types.clone(),
            dependencies: Vec::new(),
            retirements: Vec::new(),
        },
    };
    let diff = SemanticDiff {
        contract_version: contract::SEMANTIC_DIFF_CONTRACT_VERSION,
        graph_contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
        repository_id,
        body: SemanticDiffBody::Change {
            base,
            base_root,
            result_root,
            owners: Vec::new(),
            supplied_types,
            dependencies: Vec::new(),
            retirements: Vec::new(),
        },
    };
    let (transaction_digest, transaction_bytes) = transaction.encode().unwrap();
    let (diff_digest, diff_bytes) = diff.encode().unwrap();
    assert!(transaction_bytes.len() > 4 * 1_048_576);
    assert!(diff_bytes.len() > 4 * 1_048_576);
    assert_eq!(
        NormalizedTransaction::decode(&transaction_bytes, transaction_digest).unwrap(),
        transaction
    );
    assert_eq!(
        SemanticDiff::decode(&diff_bytes, diff_digest).unwrap(),
        diff
    );
}
