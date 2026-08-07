//! Strings, generation-safe resources, filesystem, SQLite, and socket host operations.

use std::cell::Cell;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_core::{
    CapabilityKind, CleanupFailures, CleanupRetentionPolicy, Error, OwnedReservation, ProviderId,
    ResourceKey, ResourceKind, ResourceOwnership, ResourceTable as CoreResourceTable,
    ResourceTableError, ResourceTableLimits, ResourceTokenParts, Result, ScopeId, Value,
};
use lkjscript_sys::OwnedFd;

const FILESYSTEM_PROVIDER: ProviderId = ProviderId::for_capability(CapabilityKind::FileSystem);
const NETWORK_PROVIDER: ProviderId = ProviderId::for_capability(CapabilityKind::Network);
const SQLITE_PROVIDER: ProviderId = ProviderId::for_capability(CapabilityKind::Sqlite);
const STDIO_PROVIDER: ProviderId = ProviderId::for_capability(CapabilityKind::Stdio);
const TERMINAL_PROVIDER: ProviderId = ProviderId::for_capability(CapabilityKind::Terminal);

static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);

fn next_scope() -> Option<ScopeId> {
    let value = NEXT_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .ok()?;
    ScopeId::new(value)
}

fn exhausted_scope() -> ScopeId {
    ScopeId::new(u64::MAX).unwrap_or_else(|| std::process::abort())
}

enum OwnedResource {
    StandardInput,
    File(OwnedFd),
    Directory(OwnedFd),
    Socket(OwnedFd),
    SqliteConnection(lkjscript_sys::SqliteConnection),
    SqliteStatement(lkjscript_sys::SqliteStatement),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceMetrics {
    resources_opened: u64,
    resources_closed: u64,
    slots_reused: u64,
    stale_key_failures: u64,
    ordinary_obligations: usize,
    emergency_obligations: usize,
    cleanup_attempts: usize,
}

#[cfg(test)]
impl ResourceMetrics {
    pub const fn resources_opened(self) -> u64 {
        self.resources_opened
    }

    pub const fn resources_closed(self) -> u64 {
        self.resources_closed
    }

    pub const fn slots_reused(self) -> u64 {
        self.slots_reused
    }

    pub const fn stale_key_failures(self) -> u64 {
        self.stale_key_failures
    }

    pub const fn ordinary_obligations(self) -> usize {
        self.ordinary_obligations
    }

    pub const fn emergency_obligations(self) -> usize {
        self.emergency_obligations
    }

    pub const fn cleanup_attempts(self) -> usize {
        self.cleanup_attempts
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceTeardown {
    ordinary_obligations: usize,
    emergency_obligations: usize,
    cleanup_attempts: usize,
    cleanup_failures: CleanupFailures,
}

pub struct ResourceTable {
    table: CoreResourceTable<OwnedResource>,
    stdin_key: ResourceKey,
    tokens: HashMap<u64, ResourceTokenParts>,
    token_by_identity: HashMap<ResourceTokenParts, u64>,
    next_token: Option<NonZeroU64>,
    metrics: Cell<ResourceMetrics>,
    limit_exceeded: bool,
    scope_exhausted: bool,
    cleanup_retention: CleanupRetentionPolicy,
}

mod files;
mod paths;
mod resource_cleanup;
mod resource_keys;
mod resource_session;
mod resource_token;
mod resources;
mod sockets;
mod sqlite_bindings;
mod sqlite_columns;
mod sqlite_lifecycle;
mod streams;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub use paths::*;
pub use sockets::SocketReceiveError;
