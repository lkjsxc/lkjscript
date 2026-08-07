use std::sync::Arc;

use crate::HostResult;

pub trait StdioProvider: Send + Sync {
    fn write(&self, bytes: &[u8]) -> HostResult<()>;
    fn flush(&self) -> HostResult<()>;
    fn read_byte(&self) -> HostResult<Option<u8>>;
}

#[derive(Clone, Default)]
pub struct HostEnvironment {
    pub stdio: Option<Arc<dyn StdioProvider>>,
    pub clock: Option<Arc<dyn crate::Clock>>,
    pub logger: Option<Arc<dyn crate::Logger>>,
    pub cancellation: Option<Arc<dyn crate::Cancellation>>,
}

impl HostEnvironment {
    pub fn portable() -> Self {
        Self {
            stdio: Some(Arc::new(crate::PortableStdio)),
            clock: Some(Arc::new(crate::PortableClock::new())),
            logger: Some(Arc::new(crate::PortableLogger)),
            cancellation: Some(Arc::new(crate::CancellationToken::new())),
        }
    }
}

impl std::fmt::Debug for HostEnvironment {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output
            .debug_struct("HostEnvironment")
            .field("stdio", &self.stdio.is_some())
            .field("clock", &self.clock.is_some())
            .field("logger", &self.logger.is_some())
            .field("cancellation", &self.cancellation.is_some())
            .finish()
    }
}
