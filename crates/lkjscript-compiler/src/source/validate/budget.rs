use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan,
    FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES, FOUNDATION_MAX_SOURCE_FILE_BYTES,
    FOUNDATION_MAX_SOURCE_UNITS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SourceFoundationBudget {
    source_units: u64,
    source_bytes: u64,
    max_source_units: u64,
    max_source_bytes: u64,
}

impl Default for SourceFoundationBudget {
    fn default() -> Self {
        Self {
            source_units: 0,
            source_bytes: 0,
            max_source_units: FOUNDATION_MAX_SOURCE_UNITS,
            max_source_bytes: FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
        }
    }
}

impl SourceFoundationBudget {
    pub(crate) const fn with_limits(max_source_units: u64, max_source_bytes: u64) -> Self {
        Self {
            source_units: 0,
            source_bytes: 0,
            max_source_units: if max_source_units < FOUNDATION_MAX_SOURCE_UNITS {
                max_source_units
            } else {
                FOUNDATION_MAX_SOURCE_UNITS
            },
            max_source_bytes: if max_source_bytes < FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES {
                max_source_bytes
            } else {
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES
            },
        }
    }

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
                self.max_source_units,
            )
        })?;
        if units > self.max_source_units {
            return Err(foundation_resource_error(
                origin.clone(),
                "source-units",
                units,
                self.max_source_units,
            ));
        }
        let bytes = self.source_bytes.checked_add(file_bytes).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                u64::MAX,
                self.max_source_bytes,
            )
        })?;
        if bytes > self.max_source_bytes {
            return Err(foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                bytes,
                self.max_source_bytes,
            ));
        }
        Ok(())
    }

    pub(crate) fn remaining_read_allowance(&self, origin: &SourceOrigin) -> SourceResult<u64> {
        let aggregate = self
            .max_source_bytes
            .checked_sub(self.source_bytes)
            .ok_or_else(|| {
                foundation_resource_error(
                    origin.clone(),
                    "aggregate-source-bytes",
                    self.source_bytes,
                    self.max_source_bytes,
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
                self.max_source_units,
            )
        })?;
        self.source_bytes = self.source_bytes.checked_add(file_bytes).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                u64::MAX,
                self.max_source_bytes,
            )
        })?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) const fn with_usage(source_units: u64, source_bytes: u64) -> Self {
        Self {
            source_units,
            source_bytes,
            max_source_units: FOUNDATION_MAX_SOURCE_UNITS,
            max_source_bytes: FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
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
