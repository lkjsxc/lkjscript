use super::*;

pub(in crate::executable) fn runtime_symbol(
    domain: NativeExecutionDomain,
    slot: RuntimeCallSlot,
) -> Option<usize> {
    let symbol = match (domain, slot) {
        (_, RuntimeCallSlot::IdentityI64) => runtime_identity_i64 as *const () as usize,
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::Poll) => {
            runtime_island_poll as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::EnterFunction) => {
            runtime_island_enter as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::TakeRejectedEntry) => {
            runtime_island_take_rejected_entry as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StdinHandle) => {
            runtime_island_stdin as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteVectorNew) => {
            runtime_island_byte_vector_new as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteVectorMove) => {
            runtime_island_byte_vector_move as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteVectorBorrowShared) => {
            runtime_island_borrow_shared as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteVectorBorrowExclusive) => {
            runtime_island_borrow_exclusive as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceLength) => {
            runtime_island_byte_slice_length as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceByteAt) => {
            runtime_island_byte_slice_byte_at as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceReadU32Le) => {
            runtime_island_byte_slice_read_u32_little_endian as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceMutSetByte) => {
            runtime_island_byte_slice_mut_set_byte as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceMutWriteU32Le) => {
            runtime_island_byte_slice_mut_write_u32_little_endian as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceEnd) => {
            runtime_island_byte_slice_end as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteSliceMutEnd) => {
            runtime_island_byte_slice_mut_end as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ByteVectorDrop) => {
            runtime_island_byte_vector_drop as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StaticBytesLength) => {
            runtime_island_static_bytes_length as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StaticBytesByteAt) => {
            runtime_island_static_bytes_byte_at as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StaticBytesClone) => {
            runtime_island_static_bytes_clone as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StaticBytesCopySlice) => {
            runtime_island_static_bytes_copy_slice as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StaticBytesThaw) => {
            runtime_island_static_bytes_thaw as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesMove) => {
            runtime_island_bytes_move as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesBorrowShared) => {
            runtime_island_bytes_borrow as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesLength) => {
            runtime_island_bytes_length as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesByteAt) => {
            runtime_island_bytes_byte_at as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesClone) => {
            runtime_island_bytes_clone as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesCopySlice) => {
            runtime_island_bytes_copy_slice as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesEndBorrow) => {
            runtime_island_bytes_end as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::BytesDrop) => {
            runtime_island_bytes_drop as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::FreezeByteVector) => {
            runtime_island_freeze_byte_vector as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ThawBytes) => {
            runtime_island_thaw_bytes as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::StructuralDispatch) => {
            runtime_island_structural_dispatch as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::ReserveFrame) => {
            runtime_island_reserve as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::RegisterFrame) => {
            runtime_island_register as *const () as usize
        }
        (NativeExecutionDomain::CollectorFree, RuntimeCallSlot::UnregisterFrame) => {
            runtime_island_unregister as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::Poll) => {
            runtime_poll as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::EnterFunction) => {
            runtime_enter_function as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::CollectReference) => {
            runtime_collect_reference as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::HeapDispatch) => {
            runtime_heap_dispatch as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::ReserveFrame) => {
            runtime_reserve_frame as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::RegisterFrame) => {
            runtime_register_frame as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::PublishSafepoint) => {
            runtime_publish_safepoint as *const () as usize
        }
        (NativeExecutionDomain::LegacyHeap, RuntimeCallSlot::UnregisterFrame) => {
            runtime_unregister_frame as *const () as usize
        }
        _ => return None,
    };
    Some(symbol)
}
