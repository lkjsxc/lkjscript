//! Exact normalized transaction records over Graph 8 canonical map bindings.

use super::contract::{
    MAXIMUM_INLINE_HISTORY_EDITS, MAXIMUM_TRANSACTION_BYTES, TRANSACTION_CONTRACT_VERSION,
    TRANSACTION_ENVELOPE_DOMAIN, TRANSACTION_MAGIC,
};
use super::digest::TransactionDigest;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyObjectDigest, OwnerKey, OwnerObjectDigest, PackageId, RetirementObjectDigest,
    SemanticRootDigest, TypeObjectDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DigestEdit<D> {
    pub before: Option<D>,
    pub after: Option<D>,
}

impl<D: Copy + Eq> DigestEdit<D> {
    fn validate(self, label: &str) -> Result<(), Diagnostic> {
        if matches!((self.before, self.after), (None, None)) || self.before == self.after {
            return Err(transaction_error(
                DiagnosticClass::Corrupt,
                "publication_transaction_edit",
                format!("{label} transaction edit is empty or unchanged"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerTransactionEdit {
    pub owner: OwnerKey,
    pub objects: DigestEdit<OwnerObjectDigest>,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyTransactionEdit {
    pub package: PackageId,
    pub objects: DigestEdit<DependencyObjectDigest>,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementTransactionEdit {
    pub owner: OwnerKey,
    pub objects: DigestEdit<RetirementObjectDigest>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransactionBody {
    Bootstrap {
        result_root: SemanticRootDigest,
    },
    Change {
        base: RevisionId,
        base_root: SemanticRootDigest,
        result_root: SemanticRootDigest,
        owners: Vec<OwnerTransactionEdit>,
        type_additions: Vec<TypeObjectDigest>,
        dependencies: Vec<DependencyTransactionEdit>,
        retirements: Vec<RetirementTransactionEdit>,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedTransaction {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub body: TransactionBody,
}

impl NormalizedTransaction {
    pub fn encode(&self) -> Result<(TransactionDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            TRANSACTION_MAGIC,
            TRANSACTION_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_TRANSACTION_BYTES,
        )?;
        Ok((TransactionDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: TransactionDigest) -> Result<Self, Diagnostic> {
        require_digest(expected, bytes)?;
        let value: Self = crate::platform::packed::decode(
            bytes,
            TRANSACTION_MAGIC,
            TRANSACTION_ENVELOPE_DOMAIN,
            MAXIMUM_TRANSACTION_BYTES,
        )?;
        value.validate()?;
        if value.encode()?.1 != bytes {
            return Err(transaction_error(
                DiagnosticClass::Corrupt,
                "publication_transaction_canonical",
                "transaction object is not canonically encoded",
            ));
        }
        Ok(value)
    }

    pub fn result_root(&self) -> SemanticRootDigest {
        match self.body {
            TransactionBody::Bootstrap { result_root }
            | TransactionBody::Change { result_root, .. } => result_root,
        }
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != TRANSACTION_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(transaction_error(
                DiagnosticClass::Source,
                "publication_transaction_contract",
                "transaction uses a predecessor or foreign contract",
            ));
        }
        let TransactionBody::Change {
            base_root,
            result_root,
            owners,
            type_additions,
            dependencies,
            retirements,
            ..
        } = &self.body
        else {
            return Ok(());
        };
        if base_root == result_root {
            return Err(transaction_error(
                DiagnosticClass::Corrupt,
                "publication_transaction_no_change",
                "accepted change transaction has equal base and result roots",
            ));
        }
        let edits = owners
            .len()
            .checked_add(type_additions.len())
            .and_then(|count| count.checked_add(dependencies.len()))
            .and_then(|count| count.checked_add(retirements.len()))
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Resource,
                    "publication_transaction_edit_count",
                    "transaction edit count overflows this platform",
                )
            })?;
        if edits == 0 || edits > MAXIMUM_INLINE_HISTORY_EDITS {
            return Err(transaction_error(
                DiagnosticClass::Resource,
                "publication_transaction_edit_count",
                format!(
                    "inline transaction contains {edits} edits; current bound is {MAXIMUM_INLINE_HISTORY_EDITS}"
                ),
            ));
        }
        validate_sorted(owners, |edit| edit.owner, "owner")?;
        validate_sorted(type_additions, |digest| *digest, "type addition")?;
        validate_sorted(dependencies, |edit| edit.package, "dependency")?;
        validate_sorted(retirements, |edit| edit.owner, "retirement")?;
        for edit in owners {
            edit.objects.validate("owner")?;
        }
        for edit in dependencies {
            edit.objects.validate("dependency")?;
        }
        for edit in retirements {
            edit.objects.validate("retirement")?;
        }
        Ok(())
    }
}

fn validate_sorted<T, K: Ord>(
    values: &[T],
    key: impl Fn(&T) -> K,
    label: &str,
) -> Result<(), Diagnostic> {
    if values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        return Err(transaction_error(
            DiagnosticClass::Corrupt,
            "publication_transaction_order",
            format!("{label} transaction edits are not unique and canonically ordered"),
        ));
    }
    Ok(())
}

fn require_digest(expected: TransactionDigest, bytes: &[u8]) -> Result<(), Diagnostic> {
    if TransactionDigest::of(bytes) != expected {
        return Err(transaction_error(
            DiagnosticClass::Corrupt,
            "publication_transaction_digest",
            "transaction bytes disagree with their object digest",
        ));
    }
    Ok(())
}

fn transaction_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
