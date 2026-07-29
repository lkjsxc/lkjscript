use std::fmt;

pub const MAX_APPLICATION_PATH_BYTES: usize = 4_096;
pub const MAX_APPLICATION_PATH_SEGMENT_BYTES: usize = 255;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationPath(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationPathError {
    Empty,
    Absolute,
    EmptySegment,
    CurrentSegment,
    ParentSegment,
    PlatformPrefix,
    ContainsNul,
    TooLong,
    SegmentTooLong,
}

impl ApplicationPath {
    pub fn parse(value: impl Into<String>) -> Result<Self, ApplicationPathError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ApplicationPathError::Empty);
        }
        if value.len() > MAX_APPLICATION_PATH_BYTES {
            return Err(ApplicationPathError::TooLong);
        }
        if value.starts_with('/') || value.ends_with('/') {
            return Err(ApplicationPathError::Absolute);
        }
        if value.contains('\0') {
            return Err(ApplicationPathError::ContainsNul);
        }
        if value.contains('\\') || value.as_bytes().get(1) == Some(&b':') {
            return Err(ApplicationPathError::PlatformPrefix);
        }
        for segment in value.split('/') {
            if segment.is_empty() {
                return Err(ApplicationPathError::EmptySegment);
            }
            if segment == "." {
                return Err(ApplicationPathError::CurrentSegment);
            }
            if segment == ".." {
                return Err(ApplicationPathError::ParentSegment);
            }
            if segment.len() > MAX_APPLICATION_PATH_SEGMENT_BYTES {
                return Err(ApplicationPathError::SegmentTooLong);
            }
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}

impl fmt::Display for ApplicationPathError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "invalid application path: {self:?}")
    }
}

impl std::error::Error for ApplicationPathError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_paths_are_relative_segmented_and_normalized() {
        let path = ApplicationPath::parse("assets/ui/counter.txt");
        assert!(path.is_ok());
        assert_eq!(
            path.as_ref().map(ApplicationPath::as_str),
            Ok("assets/ui/counter.txt")
        );
        for invalid in [
            "", "/root", "root/", "a//b", "./a", "a/../b", "C:/a", "a\\b", "a\0b",
        ] {
            assert!(ApplicationPath::parse(invalid).is_err(), "{invalid}");
        }
    }
}
