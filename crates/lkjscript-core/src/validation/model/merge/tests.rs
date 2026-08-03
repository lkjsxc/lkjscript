#[test]
fn unique_phi_retains_one_exact_representative_without_erasing_layout() {
    assert_eq!(
        merge_kind(Kind::ByteVector(3), Kind::ByteVector(9)),
        Some(Kind::ByteVector(3))
    );
    assert_eq!(
        merge_kind(Kind::Bytes(4), Kind::Bytes(10)),
        Some(Kind::Bytes(4))
    );
    assert_eq!(
        merge_identity(Some(3), Some(9)),
        Some(Some(3)),
        "owner places retain the same predecessor representative",
    );
    assert_eq!(merge_kind(Kind::ByteVector(3), Kind::Bytes(3)), None);
}
