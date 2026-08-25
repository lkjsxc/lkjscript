//! Storage-independent commitment to one logical semantic and continuity state.

use super::{
    DependencyBinding, KernelSnapshot, OwnerBinding, RetirementBinding, SemanticRoot,
    SemanticStateDigest, dependency_map_key, encode_dependency, encode_dependency_binding,
    encode_owner, encode_owner_binding, encode_retirement, encode_retirement_binding,
    owner_map_key, retirement_map_key,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::persistent_map::{MapContentRoot, MapError, MapErrorClass};

const OWNER_SECTION: u8 = 1;
const DEPENDENCY_SECTION: u8 = 2;
const RETIREMENT_SECTION: u8 = 3;

/// Independently reconstructs the exact logical state selected by a revision from complete
/// canonical records. This full oracle does not read page summaries or physical map roots.
pub fn semantic_state_digest(snapshot: &KernelSnapshot) -> Result<SemanticStateDigest, Diagnostic> {
    snapshot.root.validate_local()?;
    let owners = MapContentRoot::from_sorted(
        snapshot
            .owners
            .iter()
            .map(|(owner, record)| {
                let (object, _) = encode_owner(record)?;
                if record.owner() != *owner {
                    return Err(state_error(
                        "kernel_state_owner_key",
                        "logical owner map key disagrees with its exact record header",
                    ));
                }
                Ok((
                    owner_map_key(*owner).to_vec(),
                    encode_owner_binding(&OwnerBinding {
                        kind: record.kind(),
                        object,
                    }),
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?,
    )
    .map_err(map_diagnostic)?;
    let dependencies = MapContentRoot::from_sorted(
        snapshot
            .dependencies
            .iter()
            .map(|(package, dependency)| {
                let (object, _) = encode_dependency(dependency)?;
                if dependency.package != *package {
                    return Err(state_error(
                        "kernel_state_dependency_key",
                        "logical dependency map key disagrees with its exact package binding",
                    ));
                }
                Ok((
                    dependency_map_key(*package).to_vec(),
                    encode_dependency_binding(&DependencyBinding { object }),
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?,
    )
    .map_err(map_diagnostic)?;
    let retirements = MapContentRoot::from_sorted(
        snapshot
            .retirements
            .iter()
            .map(|(owner, retirement)| {
                let (object, _) = encode_retirement(retirement)?;
                if retirement.owner != *owner {
                    return Err(state_error(
                        "kernel_state_retirement_key",
                        "logical retirement map key disagrees with its exact retired owner",
                    ));
                }
                Ok((
                    retirement_map_key(*owner).to_vec(),
                    encode_retirement_binding(&RetirementBinding { object }),
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?,
    )
    .map_err(map_diagnostic)?;
    bind_state(&snapshot.root, owners, dependencies, retirements)
}

/// Computes the same logical state from the authenticated content summaries retained beside the
/// physical map locators. Sparse path-copy edits update these summaries without a full scan.
pub fn semantic_state_digest_from_root(
    root: &SemanticRoot,
) -> Result<SemanticStateDigest, Diagnostic> {
    root.validate_local()?;
    bind_state(
        root,
        root.owners.content_root(),
        root.dependencies.content_root(),
        root.retirements.content_root(),
    )
}

fn bind_state(
    root: &SemanticRoot,
    owners: MapContentRoot,
    dependencies: MapContentRoot,
    retirements: MapContentRoot,
) -> Result<SemanticStateDigest, Diagnostic> {
    let mut hasher = blake3::Hasher::new_derive_key(SemanticStateDigest::DOMAIN);
    hasher.update(&super::contract::SEMANTIC_STATE_CONTRACT_VERSION.to_be_bytes());
    hasher.update(&root.graph_contract_version.to_be_bytes());
    hasher.update(&root.package_id.bytes());
    update_bytes(&mut hasher, root.package_name.as_str().as_bytes())?;
    update_section(&mut hasher, OWNER_SECTION, owners);
    update_section(&mut hasher, DEPENDENCY_SECTION, dependencies);
    update_section(&mut hasher, RETIREMENT_SECTION, retirements);

    Ok(SemanticStateDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

fn update_section(hasher: &mut blake3::Hasher, section: u8, root: MapContentRoot) {
    hasher.update(&[section]);
    hasher.update(&root.entries().to_be_bytes());
    hasher.update(&root.digest().bytes());
}

fn update_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), Diagnostic> {
    let length = u64::try_from(bytes.len()).map_err(|_| {
        state_error(
            "kernel_state_value_bytes",
            "logical state value exceeds its canonical 64-bit byte bound",
        )
    })?;
    hasher.update(&length.to_be_bytes());
    hasher.update(bytes);
    Ok(())
}

fn state_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    Diagnostic::new(
        match error.class {
            MapErrorClass::Input | MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            MapErrorClass::Resource => DiagnosticClass::Resource,
            MapErrorClass::Store => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::change::stage_full_authority;
    use crate::platform::kernel::{Name, OwnerRecord};
    use crate::platform::persistent_map::{MapRoot, PageDigest};
    use crate::platform::semantic_id::RepositoryId;
    use crate::platform::storage::memory::MemoryPackedStore;

    #[test]
    fn full_oracle_agrees_with_staged_sparse_commitment() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let expected = semantic_state_digest(&snapshot).expect("full logical-state oracle");
        let mut store = MemoryPackedStore::default();
        let staged = stage_full_authority(&snapshot, &mut store).expect("stage full authority");
        assert_eq!(
            semantic_state_digest_from_root(&staged.binding.semantic.root)
                .expect("staged logical-state commitment"),
            expected
        );
    }

    #[test]
    fn logical_state_ignores_repository_and_physical_map_layout() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let expected = semantic_state_digest(&snapshot).expect("logical state digest");
        let mut repacked = snapshot.clone();
        repacked.root.repository_id = RepositoryId::migrate(b"semantic-state-other-repository", 1);
        repacked.root.owners = MapRoot::from_parts(
            PageDigest::from_bytes([0x31; 32]),
            snapshot.root.owners.entries(),
            snapshot.root.owners.content(),
        );
        repacked.root.dependencies = MapRoot::from_parts(
            PageDigest::from_bytes([0x32; 32]),
            snapshot.root.dependencies.entries(),
            snapshot.root.dependencies.content(),
        );
        repacked.root.retirements = MapRoot::from_parts(
            PageDigest::from_bytes([0x33; 32]),
            snapshot.root.retirements.entries(),
            snapshot.root.retirements.content(),
        );
        assert_eq!(semantic_state_digest(&repacked).unwrap(), expected);
    }

    #[test]
    fn logical_state_changes_with_package_presentation_and_owner_meaning() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let expected = semantic_state_digest(&snapshot).expect("logical state digest");

        let mut renamed_package = snapshot.clone();
        renamed_package.root.package_name = Name::new("renamed_package").expect("package name");
        assert_ne!(semantic_state_digest(&renamed_package).unwrap(), expected);

        let mut renamed_owner = snapshot.clone();
        let module = renamed_owner
            .owners
            .values_mut()
            .find_map(|record| match record {
                OwnerRecord::Module(module) => Some(module),
                _ => None,
            })
            .expect("module fixture");
        module.name = Name::new("renamed_module").expect("module name");
        assert_ne!(semantic_state_digest(&renamed_owner).unwrap(), expected);
    }

    #[test]
    fn logical_state_rejects_mismatched_map_keys() {
        let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
        let Some((owner, record)) = snapshot
            .owners
            .iter()
            .next()
            .map(|(owner, record)| (*owner, record.clone()))
        else {
            panic!("fixture must contain an owner")
        };
        snapshot.owners.remove(&owner);
        let foreign = snapshot
            .owners
            .keys()
            .copied()
            .find(|candidate| *candidate != record.owner())
            .expect("foreign fixture owner");
        snapshot.owners.insert(foreign, record);
        assert_eq!(
            semantic_state_digest(&snapshot)
                .expect_err("foreign map key must reject")
                .code,
            "kernel_state_owner_key"
        );
    }
}
