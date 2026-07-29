use super::*;

pub(in crate::executable) fn native_stack_bounds() -> Option<(usize, usize)> {
    // pthread_attr_t is opaque here. This word-aligned buffer is larger
    // than the Linux x86-64 glibc and musl objects passed to these APIs.
    let mut attributes = [0_usize; 16];
    // SAFETY: pthread_self has no arguments; pthread_getattr_np initializes
    // the oversized aligned opaque storage for the current live thread.
    let initialized =
        unsafe { pthread_getattr_np(pthread_self(), attributes.as_mut_ptr().cast::<c_void>()) }
            == 0;
    if !initialized {
        return None;
    }
    let mut stack_address = std::ptr::null_mut();
    let mut stack_size = 0_usize;
    // SAFETY: successful pthread_getattr_np initialized the attributes;
    // both output pointers are valid for writes during this call.
    let stack_result = unsafe {
        pthread_attr_getstack(
            attributes.as_ptr().cast::<c_void>(),
            &mut stack_address,
            &mut stack_size,
        )
    };
    // SAFETY: initialized pthread attributes are destroyed exactly once.
    let destroy_result = unsafe { pthread_attr_destroy(attributes.as_mut_ptr().cast::<c_void>()) };
    if stack_result != 0 || destroy_result != 0 {
        return None;
    }
    let stack_low = stack_address as usize;
    let stack_high = stack_low.checked_add(stack_size)?;
    (stack_low < stack_high).then_some((stack_low, stack_high))
}

pub(in crate::executable) fn native_stack_reservation_fits(
    rbp: *mut u8,
    frame_bytes: usize,
    guard_bytes: usize,
    stack_low: usize,
    stack_high: usize,
) -> bool {
    let frame_base = rbp as usize;
    let Some(requested_low) = frame_base.checked_sub(frame_bytes) else {
        return false;
    };
    let Some(guarded_low) = stack_low.checked_add(guard_bytes) else {
        return false;
    };
    stack_low < stack_high
        && stack_low <= frame_base
        && frame_base < stack_high
        && guarded_low <= requested_low
}
