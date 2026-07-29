use std::mem::size_of;

use super::*;

#[test]
fn packed_words_round_trip_only_through_exact_store_layouts() {
    let mut store = store_with(30, 3, 32, 3, 3, 4);
    let vector = store.allocate_byte_vector(vec![1]).expect("vector");
    let bytes = store.allocate_bytes(vec![2]).expect("bytes");
    let path = store
        .allocate_path(vec![3].into_boxed_slice())
        .expect("path");

    assert_eq!(size_of::<UniqueKeyWord>(), size_of::<u64>());
    assert_eq!(vector.packed_word().get(), 1_u64 << u32::BITS);
    assert_eq!(
        store.import_byte_vector_key(vector.packed_word()),
        Ok(vector)
    );
    assert_eq!(store.import_bytes_key(bytes.packed_word()), Ok(bytes));
    assert_eq!(store.import_path_key(path.packed_word()), Ok(path));

    store.free_byte_vector(vector).expect("free vector");
    store.free_bytes(bytes).expect("free bytes");
    store.free_path(path).expect("free path");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn malformed_forged_stale_generation_and_layout_words_are_rejected() {
    assert_eq!(
        UniqueKeyWord::new(u64::from(u32::MAX)),
        Err(InvalidUniqueKeyWord::ZeroGeneration)
    );

    let mut store = store_with(31, 1, 8, 1, 3, 3);
    let first = store.allocate_byte_vector(vec![7]).expect("first");
    let first_word = first.packed_word();
    let forged_index = UniqueKeyWord::new((1_u64 << u32::BITS) | u64::from(u32::MAX))
        .expect("nonzero forged generation");
    assert_eq!(
        store.import_byte_vector_key(forged_index),
        Err(UniqueStoreError::StaleKey)
    );

    let forged_generation =
        UniqueKeyWord::new((u64::from(u32::MAX) << u32::BITS) | u64::from(first.raw().index))
            .expect("nonzero forged generation");
    assert_eq!(
        store.import_byte_vector_key(forged_generation),
        Err(UniqueStoreError::StaleKey)
    );
    assert_eq!(
        store.import_bytes_key(first_word),
        Err(UniqueStoreError::WrongLayout {
            expected: UniqueLayout::Bytes,
            actual: UniqueLayout::ByteVector,
        })
    );
    assert_eq!(store.byte_vector(first), Ok(&[7][..]));

    store.free_byte_vector(first).expect("free first");
    let second = store.allocate_byte_vector(vec![8]).expect("second");
    assert_ne!(first_word, second.packed_word());
    assert_eq!(
        store.import_byte_vector_key(first_word),
        Err(UniqueStoreError::StaleKey)
    );
    assert_eq!(store.stats().stale_failures, 3);
    assert_eq!(store.stats().wrong_layout_failures, 1);
    store.free_byte_vector(second).expect("free second");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn importing_binds_owner_store_and_typed_cross_store_access_fails() {
    let mut owner = store_with(32, 1, 8, 1, 1, 2);
    let mut other = store_with(33, 1, 8, 1, 1, 2);
    let key = owner.allocate_byte_vector(vec![9]).expect("owner key");
    let imported = owner
        .import_byte_vector_key(key.packed_word())
        .expect("owner import");

    assert_eq!(
        other.byte_vector(imported),
        Err(UniqueStoreError::StoreMismatch)
    );
    assert_eq!(
        other.import_byte_vector_key(key.packed_word()),
        Err(UniqueStoreError::StaleKey)
    );
    assert_eq!(owner.byte_vector(imported), Ok(&[9][..]));
    owner.free_byte_vector(imported).expect("owner release");
    assert_eq!(owner.assert_no_leaks(), Ok(()));
    assert_eq!(other.assert_no_leaks(), Ok(()));
}
