//! Typed, composable host capabilities used by lkjscript services.

mod cancellation;
mod clock;
mod directory;
mod durable;
mod error;
mod fake;
mod local_control;
mod logging;
mod path;
mod providers;
mod spec;
mod stdio;

pub use cancellation::{Cancellation, CancellationToken};
pub use clock::{Clock, MonotonicTime, PortableClock, WallTime};
pub use directory::PortableDirectory;
pub use durable::{DurableStorage, PortableDurableStorage};
pub use error::{HostError, HostResult};
pub use fake::{FakeDurableStorage, StorageFault};
#[cfg(target_os = "linux")]
pub use local_control::local_peer_principal;
pub use local_control::LocalPrincipal;
pub use logging::{LogLevel, LogRecord, Logger, PortableLogger};
pub use path::{
    ApplicationPath, ApplicationPathError, MAX_APPLICATION_PATH_BYTES,
    MAX_APPLICATION_PATH_SEGMENT_BYTES,
};
pub use providers::{
    DatabaseProvider, DatabaseTenantFactory, DatabaseTransactionId, DirectoryProvider,
    HostEnvironment, StdioProvider,
};
pub use spec::{
    Architecture, CallingConventionId, Endianness, ExecutablePolicy, OperatingSystem, PointerWidth,
    TargetSpec,
};
pub use stdio::{BufferedStdio, PortableStdio};
