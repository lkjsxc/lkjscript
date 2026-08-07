#![allow(clippy::expect_used)]

use super::*;

fn descriptor() -> PreparedProgramDescriptor {
    PreparedProgramDescriptor {
        package_kind: PackageProvenanceKind::Locked,
        package_content: [1; 32],
        package_root: [2; 32],
        entry: [3; 32],
        module_memory_closure: [4; 32],
        memory_plan: [5; 32],
        witness_closure: [6; 32],
        semantic_ssa: [7; 32],
        native_specialization_ssa: Some([8; 32]),
        validated_bytecode: [9; 32],
        contracts: PreparedContractDigests {
            prepared_program: [10; 32],
            runtime_calls: [11; 32],
            native_layout: [12; 32],
            verified_ssa: [13; 32],
            bytecode: [14; 32],
        },
    }
}

#[test]
fn descriptor_identity_is_known_and_field_sensitive() {
    let identity = descriptor()
        .identity()
        .expect("descriptor identity")
        .bytes();
    assert_eq!(
        crate::ContractDigest::from_bytes(identity).to_hex(),
        "63cbfa520b3c7c329a8f7d90005d75632065188eb95e0e3b8fa637bde3aeb340"
    );
    let mut forged = descriptor();
    forged.entry = [17; 32];
    assert!(forged.identity().expect("forged identity") != descriptor().identity().expect("base"));
    let mut generic_only = descriptor();
    generic_only.native_specialization_ssa = None;
    assert!(
        generic_only.identity().expect("generic-only identity")
            != descriptor().identity().expect("specialized identity")
    );
}

#[test]
fn closure_rejects_reorder_missing_extra_and_zero() {
    let canonical =
        prepared_ordered_closure_digest(1, &[[1; 32], [2; 32]]).expect("canonical closure");
    assert!(prepared_ordered_closure_digest(1, &[[2; 32], [1; 32]]).is_err());
    assert!(prepared_ordered_closure_digest(1, &[]).is_err());
    assert!(prepared_ordered_closure_digest(1, &[[0; 32]]).is_err());
    assert!(
        canonical
            != prepared_ordered_closure_digest(1, &[[1; 32], [2; 32], [3; 32]])
                .expect("extra closure")
    );
}

#[test]
fn closure_accepts_more_than_former_entry_boundary() {
    let values: Vec<_> = (1_u64..=65_537)
        .map(|value| {
            let mut digest = [0; 32];
            digest[24..].copy_from_slice(&value.to_be_bytes());
            digest
        })
        .collect();
    assert_ne!(
        prepared_ordered_closure_digest(1, &values).expect("large ordered closure"),
        [0; 32]
    );
}

#[test]
fn descriptor_rejects_missing_fields() {
    let mut missing = descriptor();
    missing.contracts.bytecode = [0; 32];
    assert!(missing.identity().is_err());
}
