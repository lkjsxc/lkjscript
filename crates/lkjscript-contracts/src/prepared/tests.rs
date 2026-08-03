#![allow(clippy::expect_used)]

use super::*;

fn descriptor() -> PreparedProgramDescriptor {
    PreparedProgramDescriptor {
        platform_revision: 17,
        package_kind: PackageProvenanceKind::Locked,
        package_content: [1; 32],
        package_root: [2; 32],
        entry: [3; 32],
        module_memory_closure: [4; 32],
        memory_plan: [5; 32],
        witness_closure: [6; 32],
        semantic_ssa: [7; 32],
        native_lowerable_ssa: [8; 32],
        validated_bytecode: [9; 32],
        contracts: PreparedContractDigests {
            prepared_program: [10; 32],
            runtime_calls: [11; 32],
            native_layout: [12; 32],
            verified_ssa: [13; 32],
            bytecode: [14; 32],
            runtime_control: [15; 32],
            process_outcome_codec: [16; 32],
        },
        resource_profile: [17; 32],
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
        "e43cff66ff8765b8f832f5793a9a91b5fba3d1757dac52246ab5a03b2dde9c71"
    );
    let mut forged = descriptor();
    forged.entry = [17; 32];
    assert!(forged.identity().expect("forged identity") != descriptor().identity().expect("base"));
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
fn descriptor_rejects_missing_fields() {
    let mut missing = descriptor();
    missing.contracts.bytecode = [0; 32];
    assert!(missing.identity().is_err());
    missing = descriptor();
    missing.platform_revision = 0;
    assert!(missing.identity().is_err());
}
