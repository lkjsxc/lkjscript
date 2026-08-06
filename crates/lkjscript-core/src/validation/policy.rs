use crate::{Error, Result};

/// Byte policy applied only when a caller validates an untrusted bytecode artifact.
///
/// Trusted compiler output must use [`Self::Unrestricted`]. A limited policy
/// controls one coarse observed resource and does not redefine bytecode
/// well-formedness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationPolicy {
    Unrestricted,
    Limited { max_total_bytes: usize },
}

impl ValidationPolicy {
    pub(super) fn check_total_bytes(self, total_bytes: usize) -> Result<()> {
        match self {
            Self::Unrestricted => Ok(()),
            Self::Limited { max_total_bytes } if total_bytes > max_total_bytes => {
                Err(Error::bytecode_policy(format!(
                    "bytecode artifact has {total_bytes} total encoded bytes, policy allows {max_total_bytes}"
                )))
            }
            Self::Limited { .. } => Ok(()),
        }
    }
}
