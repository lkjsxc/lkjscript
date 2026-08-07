use std::fmt;

use crate::source::SourceOrigin;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeclarationKind {
    Main,
    Function,
    Product,
    Enum,
    Trait,
    Implementation,
}

impl DeclarationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Function => "function",
            Self::Product => "product",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Implementation => "implementation",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationKey {
    pub(crate) digest: [u8; 32],
    pub(crate) exact_identity: Vec<u8>,
}

impl DeclarationKey {
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    pub fn to_hex(&self) -> String {
        crate::source::identity::hex(&self.digest)
    }
}

impl fmt::Display for DeclarationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&crate::source::identity::hex(&self.digest))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationSummary {
    pub(crate) key: DeclarationKey,
    pub(crate) kind: DeclarationKind,
    pub(crate) name: String,
    pub(crate) origin: SourceOrigin,
}

impl DeclarationSummary {
    pub fn key(&self) -> &DeclarationKey {
        &self.key
    }

    pub const fn kind(&self) -> DeclarationKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn origin(&self) -> &SourceOrigin {
        &self.origin
    }
}
