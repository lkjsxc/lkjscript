use super::*;

impl MappingPermissions {
    #[must_use]
    pub const fn readable(self) -> bool {
        self.readable
    }

    #[must_use]
    pub const fn writable(self) -> bool {
        self.writable
    }

    #[must_use]
    pub const fn executable(self) -> bool {
        self.executable
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PermissionProbeError {
    UnsupportedPlatform,
    ProcMapsUnavailable,
    MappingNotFound,
    MalformedPermissions,
}

impl fmt::Display for PermissionProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("mapping permission probe is unsupported")
            }
            Self::ProcMapsUnavailable => formatter.write_str("/proc/self/maps is unavailable"),
            Self::MappingNotFound => {
                formatter.write_str("installed mapping is absent from /proc/self/maps")
            }
            Self::MalformedPermissions => {
                formatter.write_str("installed mapping has malformed permissions")
            }
        }
    }
}

impl std::error::Error for PermissionProbeError {}
