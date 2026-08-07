#![allow(unsafe_code)]

use super::*;

#[derive(Clone, Copy)]
pub(in crate::executable) struct NativeStackBounds {
    low: usize,
    high: usize,
    guard_bytes: usize,
}

pub(in crate::executable) fn native_stack_bounds() -> Option<NativeStackBounds> {
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
    let mut guard_bytes = 0_usize;
    // SAFETY: successful pthread_getattr_np initialized the attributes;
    // all output pointers are valid for writes during these calls.
    let stack_result = unsafe {
        pthread_attr_getstack(
            attributes.as_ptr().cast::<c_void>(),
            &mut stack_address,
            &mut stack_size,
        )
    };
    // SAFETY: the initialized attributes remain live until pthread_attr_destroy.
    let guard_result = unsafe {
        pthread_attr_getguardsize(attributes.as_ptr().cast::<c_void>(), &mut guard_bytes)
    };
    // SAFETY: initialized pthread attributes are destroyed exactly once.
    let destroy_result = unsafe { pthread_attr_destroy(attributes.as_mut_ptr().cast::<c_void>()) };
    if stack_result != 0 || guard_result != 0 || destroy_result != 0 {
        return None;
    }
    let low = stack_address as usize;
    let high = low.checked_add(stack_size)?;
    (low < high).then_some(NativeStackBounds {
        low,
        high,
        guard_bytes,
    })
}

pub(in crate::executable) fn native_stack_reservation_fits(
    rbp: *mut u8,
    frame_bytes: usize,
    bounds: NativeStackBounds,
) -> Result<(), NativeStackError> {
    let frame_base = rbp as usize;
    if !(bounds.low <= frame_base && frame_base < bounds.high) {
        return Err(NativeStackError::FrameOutsideThreadExtent);
    }
    let requested_low = frame_base
        .checked_sub(frame_bytes)
        .ok_or(NativeStackError::FrameArithmeticOverflow)?;
    let guarded_low = bounds
        .low
        .checked_add(bounds.guard_bytes)
        .filter(|guarded| *guarded < bounds.high)
        .ok_or(NativeStackError::FrameArithmeticOverflow)?;
    if requested_low < guarded_low {
        return Err(NativeStackError::GuardReached);
    }
    Ok(())
}

pub(in crate::executable) fn native_stack_requirement_fits(
    required_bytes: usize,
    bounds: NativeStackBounds,
) -> Result<(), NativeStackError> {
    let marker = 0_u8;
    native_stack_reservation_fits((&raw const marker).cast_mut(), required_bytes, bounds)
}
