mod descriptor;
mod encoder;

use std::fmt;

pub use descriptor::{PackageProvenanceKind, PreparedContractDigests, PreparedProgramDescriptor};
use encoder::Encoder;

pub const MAX_PREPARED_CLOSURE_ENTRIES: usize = 65_536;
pub const MAX_PREPARED_DESCRIPTOR_BYTES: usize = 4 * 1024 * 1024;
const CLOSURE_DOMAIN: &[u8] = b"lkjscript.prepared-program-closure\0canonical-binary";

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PreparedProgramIdentity([u8; 32]);

impl fmt::Debug for PreparedProgramIdentity {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("PreparedProgramIdentity(<redacted>)")
    }
}

impl PreparedProgramIdentity {
    #[doc(hidden)]
    pub const UNBOUND: Self = Self([0; 32]);

    pub fn new(bytes: [u8; 32]) -> Result<Self, PreparedProgramError> {
        nonzero(bytes)?;
        Ok(Self(bytes))
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn is_bound(self) -> bool {
        self.0 != [0; 32]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparedProgramError {
    ZeroPlatformRevision,
    ZeroDigest,
    EmptyClosure,
    ClosureOrder,
    BoundExceeded,
    AllocationFailed,
}

impl fmt::Display for PreparedProgramError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::ZeroPlatformRevision => "prepared platform revision is zero",
            Self::ZeroDigest => "prepared digest is zero",
            Self::EmptyClosure => "prepared closure is empty",
            Self::ClosureOrder => "prepared closure is not strictly ordered and unique",
            Self::BoundExceeded => "prepared descriptor bound exceeded",
            Self::AllocationFailed => "prepared descriptor allocation failed",
        })
    }
}

impl std::error::Error for PreparedProgramError {}

pub fn prepared_ordered_closure_digest(
    domain_tag: u16,
    values: &[[u8; 32]],
) -> Result<[u8; 32], PreparedProgramError> {
    if values.is_empty() {
        return Err(PreparedProgramError::EmptyClosure);
    }
    if values.len() > MAX_PREPARED_CLOSURE_ENTRIES {
        return Err(PreparedProgramError::BoundExceeded);
    }
    if !values.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(PreparedProgramError::ClosureOrder);
    }
    let mut out = Encoder::new(CLOSURE_DOMAIN)?;
    out.tag(domain_tag)?;
    out.u64(u64::try_from(values.len()).map_err(|_| PreparedProgramError::BoundExceeded)?)?;
    for value in values {
        nonzero(*value)?;
        out.fixed(value)?;
    }
    let digest = out.finish();
    nonzero(digest)?;
    Ok(digest)
}

pub(crate) fn nonzero(value: [u8; 32]) -> Result<(), PreparedProgramError> {
    if value == [0; 32] {
        Err(PreparedProgramError::ZeroDigest)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
