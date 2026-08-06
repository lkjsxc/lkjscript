#[test]
fn unique_phi_retains_one_exact_representative_without_erasing_layout() {
    let first = OwnerIdentity::instruction(3, 1);
    let second = OwnerIdentity::instruction(9, 1);
    assert_eq!(
        merge_kind(Kind::ByteVector(first), Kind::ByteVector(second)),
        Some(Kind::ByteVector(first))
    );
    assert_eq!(
        merge_kind(Kind::Bytes(first), Kind::Bytes(second)),
        Some(Kind::Bytes(first))
    );
    assert_eq!(
        merge_identity(Some(first), Some(second)),
        Some(Some(first)),
        "owner places retain the same predecessor representative",
    );
    assert_eq!(
        merge_kind(Kind::ByteVector(first), Kind::Bytes(first)),
        None
    );
}
