//! Linux monotonic clock and sleep via owned libc externs.

use crate::fd::{errno, FdError};

const CLOCK_MONOTONIC: i32 = 1;

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

extern "C" {
    fn clock_gettime(clock_id: i32, tp: *mut Timespec) -> i32;
    fn nanosleep(req: *const Timespec, rem: *mut Timespec) -> i32;
}

pub fn now_ms_monotonic() -> Result<i64, FdError> {
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { clock_gettime(CLOCK_MONOTONIC, &mut ts) };
    if rc < 0 {
        return Err(FdError(errno()));
    }
    Ok(ts.tv_sec.saturating_mul(1000) + ts.tv_nsec / 1_000_000)
}

pub fn sleep_ms(ms: u64) -> Result<(), FdError> {
    let mut req = Timespec {
        tv_sec: (ms / 1000) as i64,
        tv_nsec: ((ms % 1000) * 1_000_000) as i64,
    };
    loop {
        let mut rem = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let rc = unsafe { nanosleep(&req, &mut rem) };
        if rc == 0 {
            return Ok(());
        }
        let err = errno();
        // EINTR = 4: retry with remainder
        if err == 4 {
            req = rem;
            continue;
        }
        return Err(FdError(err));
    }
}
