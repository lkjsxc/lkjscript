//! Typed, composable host capabilities used by lkjscript services.

mod cancellation;
mod clock;
mod durable;
mod error;
mod fake;
mod local_control;
mod logging;
mod spec;

pub use cancellation::{Cancellation, CancellationToken};
pub use clock::{Clock, MonotonicTime, PortableClock, WallTime};
pub use durable::{DurableStorage, PortableDurableStorage};
pub use error::{HostError, HostResult};
pub use fake::{FakeDurableStorage, StorageFault};
#[cfg(target_os = "linux")]
pub use local_control::local_peer_principal;
pub use local_control::LocalPrincipal;
pub use logging::{LogLevel, LogRecord, Logger, PortableLogger};
pub use spec::{
    Architecture, CallingConventionId, Endianness, ExecutablePolicy, ExecutionBackendId, HostSpec,
    OperatingSystem, PointerWidth, TargetSpec,
};
