use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageCodecError(pub(super) &'static str);

impl ImageCodecError {
    pub(super) const fn new(message: &'static str) -> Self {
        Self(message)
    }
}

impl fmt::Display for ImageCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ImageCodecError {}
