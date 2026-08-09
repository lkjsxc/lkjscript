//! Localized heap-backed stack growth for recursive compiler mechanisms that have
//! not yet been converted to explicit work stacks. `stacker` allocates another
//! segment whenever recursion approaches the current segment's red zone, so
//! these tuning sizes do not impose a finite language depth.

const RED_ZONE_BYTES: usize = 96 * 1024;
const SEGMENT_BYTES: usize = 256 * 1024;

#[inline]
pub(crate) fn grow<R>(operation: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE_BYTES, SEGMENT_BYTES, operation)
}
