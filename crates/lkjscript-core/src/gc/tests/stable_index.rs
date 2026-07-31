#![allow(clippy::expect_used)]

use super::*;

#[test]
fn stable_index_boundary_model_rejects_before_duplicate_u32_handles() {
    let last_valid_slot_count = u32::MAX as usize;
    assert!(stable_index_available(last_valid_slot_count));
    if let Some(exhausted_slot_count) = last_valid_slot_count.checked_add(1) {
        assert!(!stable_index_available(exhausted_slot_count));
    }
}
