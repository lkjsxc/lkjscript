use crate::source::{DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan};

/// Aggregate source-byte admission at a loader or Semantic Source boundary.
///
/// Trusted local entry points select `Unrestricted`. `Limited` is reserved for
/// an explicit untrusted request policy and never changes language validity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceBytePolicy {
    Unrestricted,
    Limited { max_aggregate_source_bytes: u64 },
}

impl SourceBytePolicy {
    pub(crate) const fn limited(max_aggregate_source_bytes: u64) -> Self {
        Self::Limited {
            max_aggregate_source_bytes,
        }
    }

    pub(crate) fn remaining_read_allowance(
        self,
        origin: &SourceOrigin,
        completed_source_bytes: u64,
    ) -> SourceResult<Option<u64>> {
        match self {
            Self::Unrestricted => Ok(None),
            Self::Limited {
                max_aggregate_source_bytes,
            } => max_aggregate_source_bytes
                .checked_sub(completed_source_bytes)
                .map(Some)
                .ok_or_else(|| {
                    source_byte_policy_error(
                        origin.clone(),
                        completed_source_bytes,
                        max_aggregate_source_bytes,
                    )
                }),
        }
    }

    /// Account one complete source unit with checked aggregate arithmetic.
    ///
    /// Loader callers publish the returned total only after all reader
    /// consistency checks pass, so a failed read consumes no allowance.
    pub(crate) fn account_source_bytes(
        self,
        origin: &SourceOrigin,
        completed_source_bytes: u64,
        file_bytes: u64,
    ) -> SourceResult<u64> {
        let total = completed_source_bytes
            .checked_add(file_bytes)
            .ok_or_else(|| {
                SourceDiagnostic::host(
                    origin.clone(),
                    "aggregate source byte accounting overflowed its u64 representation",
                )
            })?;
        self.check_total(origin, total)?;
        Ok(total)
    }

    pub(crate) fn check_total(
        self,
        origin: &SourceOrigin,
        aggregate_source_bytes: u64,
    ) -> SourceResult<()> {
        if let Self::Limited {
            max_aggregate_source_bytes,
        } = self
        {
            if aggregate_source_bytes > max_aggregate_source_bytes {
                return Err(source_byte_policy_error(
                    origin.clone(),
                    aggregate_source_bytes,
                    max_aggregate_source_bytes,
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn source_byte_policy_error(
    origin: SourceOrigin,
    attempted: u64,
    limit: u64,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        format!(
            "Semantic Source resource limit: category=aggregate-source-bytes; attempted={attempted}; limit={limit}"
        ),
        origin,
        SourceSpan::zero(),
    )
}
