use std::fmt;

use crate::{ApplicationId, ApplicationIncarnationId, Lifecycle, PackageContentId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuotaKind {
    ConcurrentInvocations,
    TotalInvocations,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    InvalidManifest(&'static str),
    CapabilityNotGranted(lkjscript_core::CapabilityKind),
    UnsupportedCapability(lkjscript_core::CapabilityKind),
    ExecutionCellClassMismatch,
    ProcessCell(String),
    ApplicationNotFound(ApplicationId),
    PackageCacheFull,
    PackageNotCached(PackageContentId),
    IllegalTransition {
        from: Lifecycle,
        to: Lifecycle,
    },
    StaleIncarnation {
        requested: ApplicationIncarnationId,
        current: Option<ApplicationIncarnationId>,
    },
    QuotaExceeded(QuotaKind),
    AtLeastTwoInvocationsRequired,
    IdentifierSpaceExhausted,
    StateUnavailable,
    WorkerPanicked,
    CoordinatorAlreadyActive,
    CoordinatorLease(&'static str),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest(message) => write!(formatter, "invalid manifest: {message}"),
            Self::CapabilityNotGranted(kind) => {
                write!(
                    formatter,
                    "application capability is not granted: {}",
                    kind.as_str()
                )
            }
            Self::UnsupportedCapability(kind) => write!(
                formatter,
                "application provider is unavailable for capability: {}",
                kind.as_str()
            ),
            Self::ExecutionCellClassMismatch => {
                formatter.write_str("application manifest and installed cell class differ")
            }
            Self::ProcessCell(message) => write!(formatter, "isolated process cell: {message}"),
            Self::ApplicationNotFound(id) => {
                write!(formatter, "application {} not found", id.get())
            }
            Self::PackageCacheFull => formatter.write_str("package cache has no evictable entry"),
            Self::PackageNotCached(id) => write!(formatter, "package {id:?} is not cached"),
            Self::IllegalTransition { from, to } => {
                write!(formatter, "illegal lifecycle transition {from:?} -> {to:?}")
            }
            Self::StaleIncarnation { requested, current } => write!(
                formatter,
                "stale application incarnation {} (current: {current:?})",
                requested.incarnation()
            ),
            Self::QuotaExceeded(kind) => write!(formatter, "quota exceeded: {kind:?}"),
            Self::AtLeastTwoInvocationsRequired => {
                formatter.write_str("concurrent invocation requires at least two requests")
            }
            Self::IdentifierSpaceExhausted => formatter.write_str("identifier space exhausted"),
            Self::StateUnavailable => formatter.write_str("runtime state lock is unavailable"),
            Self::WorkerPanicked => formatter.write_str("an invocation worker panicked"),
            Self::CoordinatorAlreadyActive => {
                formatter.write_str("another lkjscriptd coordinator is active")
            }
            Self::CoordinatorLease(message) => {
                write!(formatter, "coordinator lease failed: {message}")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}
