use std::fmt;

use crate::{ApplicationGenerationId, ApplicationId, Lifecycle, PackageContentId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuotaKind {
    ConcurrentInvocations,
    TotalInvocations,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    InvalidManifest(&'static str),
    UnsafeCapabilities,
    ApplicationNotFound(ApplicationId),
    PackageCacheFull,
    PackageNotCached(PackageContentId),
    IllegalTransition {
        from: Lifecycle,
        to: Lifecycle,
    },
    StaleGeneration {
        requested: ApplicationGenerationId,
        current: Option<ApplicationGenerationId>,
    },
    QuotaExceeded(QuotaKind),
    AtLeastTwoInvocationsRequired,
    IdentifierSpaceExhausted,
    StateUnavailable,
    WorkerPanicked,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest(message) => write!(formatter, "invalid manifest: {message}"),
            Self::UnsafeCapabilities => formatter
                .write_str("in-process applications must require and receive no capabilities"),
            Self::ApplicationNotFound(id) => {
                write!(formatter, "application {} not found", id.get())
            }
            Self::PackageCacheFull => formatter.write_str("package cache has no evictable entry"),
            Self::PackageNotCached(id) => write!(formatter, "package {id:?} is not cached"),
            Self::IllegalTransition { from, to } => {
                write!(formatter, "illegal lifecycle transition {from:?} -> {to:?}")
            }
            Self::StaleGeneration { requested, current } => write!(
                formatter,
                "stale generation {} (current: {current:?})",
                requested.generation()
            ),
            Self::QuotaExceeded(kind) => write!(formatter, "quota exceeded: {kind:?}"),
            Self::AtLeastTwoInvocationsRequired => {
                formatter.write_str("concurrent invocation requires at least two requests")
            }
            Self::IdentifierSpaceExhausted => formatter.write_str("identifier space exhausted"),
            Self::StateUnavailable => formatter.write_str("runtime state lock is unavailable"),
            Self::WorkerPanicked => formatter.write_str("an invocation worker panicked"),
        }
    }
}

impl std::error::Error for RuntimeError {}
