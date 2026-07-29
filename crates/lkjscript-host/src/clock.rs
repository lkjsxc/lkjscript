use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::{HostError, HostResult};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MonotonicTime(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WallTime(pub u64);

pub trait Clock: Send + Sync {
    fn monotonic_time(&self) -> MonotonicTime;
    fn wall_time(&self) -> HostResult<WallTime>;
    fn sleep(&self, duration: std::time::Duration) -> HostResult<()>;
}

#[derive(Debug)]
pub struct PortableClock {
    origin: Instant,
}

impl PortableClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for PortableClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for PortableClock {
    fn monotonic_time(&self) -> MonotonicTime {
        let nanos = self.origin.elapsed().as_nanos();
        MonotonicTime(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    fn wall_time(&self) -> HostResult<WallTime> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| HostError::Clock(error.to_string()))?;
        Ok(WallTime(
            u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX),
        ))
    }

    fn sleep(&self, duration: std::time::Duration) -> HostResult<()> {
        std::thread::sleep(duration);
        Ok(())
    }
}
