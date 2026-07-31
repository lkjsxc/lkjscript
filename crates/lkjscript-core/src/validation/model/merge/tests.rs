#[test]
fn unique_phi_merges_distinct_owner_origins_without_erasing_layout() {
    assert_eq!(
        merge_kind(Kind::ByteVector(3), Kind::ByteVector(9)),
        Some(Kind::ByteVector(u32::MAX))
    );
    assert_eq!(
        merge_kind(Kind::Bytes(4), Kind::Bytes(10)),
        Some(Kind::Bytes(u32::MAX))
    );
    assert_eq!(merge_kind(Kind::ByteVector(3), Kind::Bytes(3)), None);
}
