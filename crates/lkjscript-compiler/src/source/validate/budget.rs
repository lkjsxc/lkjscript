use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan,
    FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES, FOUNDATION_MAX_SOURCE_FILE_BYTES,
    FOUNDATION_MAX_SOURCE_UNITS,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SourceFoundationBudget {
    source_units: u64,
    source_bytes: u64,
}

impl SourceFoundationBudget {
    pub(crate) fn check_metadata(
        &self,
        origin: &SourceOrigin,
        file_bytes: u64,
    ) -> SourceResult<()> {
        check_foundation_file_bytes(origin, file_bytes)?;
        let units = self.source_units.checked_add(1).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "source-units",
                u64::MAX,
                FOUNDATION_MAX_SOURCE_UNITS,
            )
        })?;
        if units > FOUNDATION_MAX_SOURCE_UNITS {
            return Err(foundation_resource_error(
                origin.clone(),
                "source-units",
                units,
                FOUNDATION_MAX_SOURCE_UNITS,
            ));
        }
        let bytes = self.source_bytes.checked_add(file_bytes).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                u64::MAX,
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
            )
        })?;
        if bytes > FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES {
            return Err(foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                bytes,
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
            ));
        }
        Ok(())
    }

    pub(crate) fn remaining_read_allowance(&self, origin: &SourceOrigin) -> SourceResult<u64> {
        let aggregate = FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES
            .checked_sub(self.source_bytes)
            .ok_or_else(|| {
                foundation_resource_error(
                    origin.clone(),
                    "aggregate-source-bytes",
                    self.source_bytes,
                    FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
                )
            })?;
        Ok(FOUNDATION_MAX_SOURCE_FILE_BYTES.min(aggregate))
    }

    pub(crate) fn record_read(
        &mut self,
        origin: &SourceOrigin,
        file_bytes: u64,
    ) -> SourceResult<()> {
        self.check_metadata(origin, file_bytes)?;
        self.source_units = self.source_units.checked_add(1).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "source-units",
                u64::MAX,
                FOUNDATION_MAX_SOURCE_UNITS,
            )
        })?;
        self.source_bytes = self.source_bytes.checked_add(file_bytes).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                u64::MAX,
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
            )
        })?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) const fn with_usage(source_units: u64, source_bytes: u64) -> Self {
        Self {
            source_units,
            source_bytes,
        }
    }
}

pub(crate) fn check_foundation_file_bytes(
    origin: &SourceOrigin,
    file_bytes: u64,
) -> SourceResult<()> {
    if file_bytes > FOUNDATION_MAX_SOURCE_FILE_BYTES {
        return Err(foundation_resource_error(
            origin.clone(),
            "source-file-bytes",
            file_bytes,
            FOUNDATION_MAX_SOURCE_FILE_BYTES,
        ));
    }
    Ok(())
}

pub(crate) fn foundation_resource_error(
    origin: SourceOrigin,
    category: &str,
    attempted: u64,
    limit: u64,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        format!(
            "Semantic Source Foundation V1 resource limit: category={category}; attempted={attempted}; limit={limit}"
        ),
        origin,
        SourceSpan::zero(),
    )
}
