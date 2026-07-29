use lkjscript_core::{StructuralBorrowKey, StructuralValueKey, Value};

#[test]
fn compact_structural_keys_preserve_the_closed_value_size() {
    assert_eq!(std::mem::size_of::<StructuralValueKey>(), 8);
    assert_eq!(std::mem::size_of::<StructuralBorrowKey>(), 8);
    assert_eq!(std::mem::size_of::<Value>(), 16);
    assert_eq!(std::mem::align_of::<Value>(), 8);
}
