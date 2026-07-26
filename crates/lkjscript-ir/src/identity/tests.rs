#![allow(clippy::expect_used)]

use super::*;

#[test]
fn verified_program_identity_is_deterministic_and_complete_for_names() {
    let program = crate::tests::fixtures::one_block_program();
    let verified = crate::verify(program.clone()).expect("verified fixture");
    let first = verified_program_digest(&verified).expect("first identity");
    let second = verified_program_digest(&verified).expect("second identity");
    assert_eq!(first, second);

    let mut changed = program;
    changed.functions[0].name.push_str("-changed");
    let changed = crate::verify(changed).expect("changed verified fixture");
    assert_ne!(
        first,
        verified_program_digest(&changed).expect("changed identity")
    );
}
