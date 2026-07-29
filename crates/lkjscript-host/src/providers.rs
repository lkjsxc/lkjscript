use std::sync::Arc;

use crate::{ApplicationPath, HostResult};

pub trait StdioProvider: Send + Sync {
    fn write(&self, bytes: &[u8]) -> HostResult<()>;
    fn flush(&self) -> HostResult<()>;
    fn read_byte(&self) -> HostResult<Option<u8>>;
}

pub trait DirectoryProvider: Send + Sync {
    fn read(&self, path: &ApplicationPath) -> HostResult<Vec<u8>>;
    fn write(&self, path: &ApplicationPath, bytes: &[u8]) -> HostResult<()>;
    fn remove(&self, path: &ApplicationPath) -> HostResult<()>;
    fn list(&self, path: Option<&ApplicationPath>) -> HostResult<Vec<String>>;
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatabaseTransactionId {
    provider: u64,
    slot: u32,
    incarnation: u64,
}

impl DatabaseTransactionId {
    pub fn new(provider: u64, slot: u32, incarnation: u64) -> Option<Self> {
        (provider != 0 && slot != 0 && incarnation != 0).then_some(Self {
            provider,
            slot,
            incarnation,
        })
    }

    pub const fn provider(self) -> u64 {
        self.provider
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn incarnation(self) -> u64 {
        self.incarnation
    }
}

pub trait DatabaseProvider: Send + Sync {
    fn begin_read(&self) -> HostResult<DatabaseTransactionId>;
    fn begin_write(&self) -> HostResult<DatabaseTransactionId>;
    fn get(&self, transaction: DatabaseTransactionId, key: &[u8]) -> HostResult<Option<Vec<u8>>>;
    fn put(
        &self,
        transaction: DatabaseTransactionId,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> HostResult<()>;
    fn delete(&self, transaction: DatabaseTransactionId, key: Vec<u8>) -> HostResult<()>;
    fn range(
        &self,
        transaction: DatabaseTransactionId,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> HostResult<Vec<(Vec<u8>, Vec<u8>)>>;
    fn commit(&self, transaction: DatabaseTransactionId) -> HostResult<()>;
    fn abort(&self, transaction: DatabaseTransactionId) -> HostResult<()>;
    fn abort_all(&self) -> HostResult<usize>;
}

#[derive(Clone, Default)]
pub struct HostEnvironment {
    pub stdio: Option<Arc<dyn StdioProvider>>,
    pub clock: Option<Arc<dyn crate::Clock>>,
    pub logger: Option<Arc<dyn crate::Logger>>,
    pub cancellation: Option<Arc<dyn crate::Cancellation>>,
    pub directory: Option<Arc<dyn DirectoryProvider>>,
    pub database: Option<Arc<dyn DatabaseProvider>>,
}

impl HostEnvironment {
    pub fn portable() -> Self {
        Self {
            stdio: Some(Arc::new(crate::PortableStdio)),
            clock: Some(Arc::new(crate::PortableClock::new())),
            logger: Some(Arc::new(crate::PortableLogger)),
            cancellation: Some(Arc::new(crate::CancellationToken::new())),
            directory: None,
            database: None,
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
            .field("directory", &self.directory.is_some())
            .field("database", &self.database.is_some())
            .finish()
    }
}
