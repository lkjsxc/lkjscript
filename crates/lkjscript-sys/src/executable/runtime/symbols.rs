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
