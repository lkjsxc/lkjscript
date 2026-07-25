use super::fixtures::*;
use crate::*;

#[test]
fn ownership_cfg_rejects_duplicate_calls_aliases_and_missing_provenance() {
    let callee = ownership_callee();
    let caller = duplicate_call_caller();
    let mut duplicate_call = one_block_program();
    duplicate_call.functions.extend([callee.clone(), caller]);
    let error = verify(duplicate_call).expect_err("duplicate affine call arguments must fail");
    assert!(
        error.to_string().contains("duplicates one affine"),
        "{error}"
    );

    let implicit_call = implicit_call_caller();
    let mut implicit_call_program = one_block_program();
    implicit_call_program
        .functions
        .extend([callee, implicit_call]);
    let error = verify(implicit_call_program)
        .expect_err("Owned entry call argument requires explicit Move");
    assert!(error.to_string().contains("explicit Move"), "{error}");

    let aliased_places = aliased_places();
    let error = verify(ownership_program(aliased_places))
        .expect_err("one owner value cannot initialize two PlaceIds");
    assert!(error.to_string().contains("multiple PlaceIds"), "{error}");

    let mut missing_entry = owned_branch_function(true);
    missing_entry.blocks[0].parameters[0].owner_place = None;
    assert!(verify(ownership_program(missing_entry)).is_err());

    let missing_local = missing_local_provenance();
    assert!(verify(ownership_program(missing_local)).is_err());
}
