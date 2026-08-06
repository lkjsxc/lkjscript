mod measure;

pub(crate) use measure::measure;

/// Coarse byte policy for an untrusted Semantic Source request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BoundaryPolicy {
    pub source_bytes: u64,
    pub response_bytes: usize,
}

impl Default for BoundaryPolicy {
    fn default() -> Self {
        Self {
            source_bytes: 16 * 1024 * 1024,
            response_bytes: crate::semantic::codec::MAX_OUTPUT_BYTES,
        }
    }
}
