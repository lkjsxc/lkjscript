use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use lkjscript_core::{Error, Result};

static NEXT_NAMESPACE_NONCE: AtomicU64 = AtomicU64::new(1);

/// Opaque identity domain for one logical workspace.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceNamespace([u8; 32]);

impl WorkspaceNamespace {
    pub(super) fn fresh() -> Result<Self> {
        let nonce = NEXT_NAMESPACE_NONCE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| Error::host("workspace namespace nonce exhausted"))?;
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| Error::host(format!("workspace namespace clock failed: {error}")))?;
        let mut seed = [0_u8; 45];
        seed[..4].copy_from_slice(&std::process::id().to_be_bytes());
        seed[4..12].copy_from_slice(&nonce.to_be_bytes());
        seed[12..28].copy_from_slice(&elapsed.as_nanos().to_be_bytes());
        seed[28..36].copy_from_slice(&elapsed.as_secs().to_be_bytes());
        seed[36..45].copy_from_slice(b"workspace");
        Ok(Self(lkjscript_core::sha256(&seed)))
    }

    #[cfg(test)]
    pub(super) fn deterministic(seed: u64) -> Self {
        let mut bytes = [0_u8; 32];
        bytes[..8].copy_from_slice(&seed.to_be_bytes());
        bytes[8..].copy_from_slice(&lkjscript_core::sha256(&seed.to_be_bytes())[..24]);
        Self(bytes)
    }
}

impl fmt::Debug for WorkspaceNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WorkspaceNamespace(<opaque>)")
    }
}

/// Opaque immutable workspace revision identity.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId {
    namespace: WorkspaceNamespace,
    sequence: u64,
}

impl RevisionId {
    pub(super) const fn initial(namespace: WorkspaceNamespace) -> Self {
        Self {
            namespace,
            sequence: 1,
        }
    }

    pub(super) const fn namespace(self) -> WorkspaceNamespace {
        self.namespace
    }
}

impl fmt::Debug for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RevisionId(<opaque>)")
    }
}

macro_rules! workspace_id {
    ($name:ident) => {
        /// Opaque stable logical identity, independent of HIR vector positions.
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            namespace: WorkspaceNamespace,
            slot: u64,
            generation: u64,
        }

        impl $name {
            pub(super) const fn new(
                namespace: WorkspaceNamespace,
                slot: u64,
                generation: u64,
            ) -> Self {
                Self {
                    namespace,
                    slot,
                    generation,
                }
            }

            pub(super) const fn namespace(self) -> WorkspaceNamespace {
                self.namespace
            }

            pub(super) const fn slot(self) -> u64 {
                self.slot
            }

            pub(super) const fn generation(self) -> u64 {
                self.generation
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

workspace_id!(EntityId);
workspace_id!(NodeId);
