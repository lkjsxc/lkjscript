//! The complete unsafe Linux affinity ABI boundary.

use crate::fd::errno;

extern "C" {
    fn getpid() -> i32;
    fn sched_getaffinity(pid: i32, cpusetsize: usize, mask: *mut u8) -> i32;
    fn sched_setaffinity(pid: i32, cpusetsize: usize, mask: *const u8) -> i32;
    fn sched_getscheduler(pid: i32) -> i32;
}

pub(crate) fn process_id() -> i32 {
    // SAFETY: getpid has no pointer arguments or preconditions.
    unsafe { getpid() }
}

pub(crate) fn get_affinity(pid: i32, mask: &mut [u8]) -> Result<(), i32> {
    // SAFETY: `mask` is writable for exactly `mask.len()` bytes for the call.
    let result = unsafe { sched_getaffinity(pid, mask.len(), mask.as_mut_ptr()) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub(crate) fn set_affinity(pid: i32, mask: &[u8]) -> Result<(), i32> {
    // SAFETY: `mask` is readable for exactly `mask.len()` bytes for the call.
    let result = unsafe { sched_setaffinity(pid, mask.len(), mask.as_ptr()) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub(crate) fn scheduler(pid: i32) -> Result<i32, i32> {
    // SAFETY: sched_getscheduler has no pointer arguments; any pid is handled by Linux.
    let result = unsafe { sched_getscheduler(pid) };
    if result >= 0 {
        Ok(result)
    } else {
        Err(errno())
    }
}
