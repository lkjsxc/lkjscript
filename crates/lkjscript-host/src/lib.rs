//! Typed, composable host capabilities used by lkjscript services.

mod cancellation;
mod clock;
mod error;
mod logging;
mod providers;
mod spec;
mod stdio;

pub use cancellation::{Cancellation, CancellationToken};
pub use clock::{Clock, MonotonicTime, PortableClock, WallTime};
pub use error::{HostError, HostResult};
pub use logging::{LogLevel, LogRecord, Logger, PortableLogger};
pub use providers::{HostEnvironment, StdioProvider};
pub use spec::{
    Architecture, CallingConventionId, Endianness, ExecutablePolicy, OperatingSystem, PointerWidth,
    TargetSpec,
};
pub use stdio::{BufferedStdio, PortableStdio};
