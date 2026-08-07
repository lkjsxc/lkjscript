use std::mem::size_of;

use super::*;

#[test]
fn opaque_words_round_trip_only_through_exact_store_layouts() {
    let mut store = store(30);
    let vector = store.allocate_byte_vector(vec![1]).expect("vector");
    let bytes = store.allocate_bytes(vec![2]).expect("bytes");
    let path = store
        .allocate_path(vec![3].into_boxed_slice())
        .expect("path");

    assert_eq!(size_of::<UniqueKeyWord>(), size_of::<u64>());
    assert_ne!(vector.opaque_word().get(), 0);
    assert_eq!(
        store.import_byte_vector_key(vector.opaque_word()),
        Ok(vector)
    );
    assert_eq!(store.import_bytes_key(bytes.opaque_word()), Ok(bytes));
    assert_eq!(store.import_path_key(path.opaque_word()), Ok(path));

    store.free_byte_vector(vector).expect("free vector");
    store.free_bytes(bytes).expect("free bytes");
    store.free_path(path).expect("free path");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn forged_stale_and_wrong_layout_opaque_words_are_rejected() {
    assert_eq!(UniqueKeyWord::new(0), Err(InvalidUniqueKeyWord::ZeroToken));

    let mut store = store(31);
    let first = store.allocate_byte_vector(vec![7]).expect("first");
    let first_word = first.opaque_word();
    let forged = UniqueKeyWord::new(u64::MAX).expect("nonzero forged token");
    assert_eq!(
        store.import_byte_vector_key(forged),
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
    assert_ne!(first_word, second.opaque_word());
    assert_eq!(
        store.import_byte_vector_key(first_word),
        Err(UniqueStoreError::StaleKey)
    );
    assert_eq!(store.stats().stale_failures, 2);
    assert_eq!(store.stats().wrong_layout_failures, 1);
    store.free_byte_vector(second).expect("free second");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn high_opaque_token_preserves_wide_slot_and_generation_identity() {
    let mut store = store(34);
    let original = store.allocate_byte_vector(vec![5]).expect("owner");
    let raw = original.raw();
    let high = UniqueKeyWord::new(u64::from(u32::MAX) + 17).expect("high token");
    store.tokens.remove(&original.opaque_word());
    let high_raw = super::super::model::RawUniqueKey {
        word: high,
        index: raw.index,
        generation: raw.generation,
        store: raw.store,
    };
    store.tokens.insert(high, high_raw);
    let imported = store
        .import_byte_vector_key(high)
        .expect("high opaque token resolves");
    assert_eq!(imported.raw().index, raw.index);
    assert_eq!(imported.raw().generation, raw.generation);
    store
        .free_byte_vector(imported)
        .expect("release high token");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn importing_binds_owner_store_and_typed_cross_store_access_fails() {
    let mut owner = store(32);
    let mut other = store(33);
    let key = owner.allocate_byte_vector(vec![9]).expect("owner key");
    let imported = owner
        .import_byte_vector_key(key.opaque_word())
        .expect("owner import");

    assert_eq!(
        other.byte_vector(imported),
        Err(UniqueStoreError::StoreMismatch)
    );
    assert_eq!(
        other.import_byte_vector_key(key.opaque_word()),
        Err(UniqueStoreError::StaleKey)
    );
    assert_eq!(owner.byte_vector(imported), Ok(&[9][..]));
    owner.free_byte_vector(imported).expect("owner release");
    assert_eq!(owner.assert_no_leaks(), Ok(()));
    assert_eq!(other.assert_no_leaks(), Ok(()));
}
