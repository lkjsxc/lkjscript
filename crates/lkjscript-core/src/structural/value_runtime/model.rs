use std::num::NonZeroU32;

use super::super::{image::SemanticChildren, LayoutIdentity, SemanticTypeIdentity};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructuralKind {
    Unit,
    Bool,
    I64,
    F64,
    String,
    Path,
    Bytes,
    ByteVector,
    Product,
    Enum,
    Static,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StructuralType {
    pub layout: LayoutIdentity,
    pub semantic_type: SemanticTypeIdentity,
    pub kind: StructuralKind,
}

impl StructuralType {
    pub const fn new(
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
        kind: StructuralKind,
    ) -> Self {
        Self {
            layout,
            semantic_type,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineStructuralValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64Bits(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticStructuralLeaf {
    Function(u64),
    Symbol(u64),
    Bytes(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticPayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    String(Vec<u8>),
    Path(Vec<u8>),
    Bytes(Vec<u8>),
    ByteVector(Vec<u8>),
    Product(SemanticChildren),
    Enum {
        tag: u16,
        active_payload: SemanticChildren,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticValue {
    pub value_type: StructuralType,
    pub payload: SemanticPayload,
}

impl SemanticValue {
    pub const fn new(value_type: StructuralType, payload: SemanticPayload) -> Self {
        Self {
            value_type,
            payload,
        }
    }

    pub fn utf8(&self) -> Option<&str> {
        match &self.payload {
            SemanticPayload::String(bytes) => std::str::from_utf8(bytes).ok(),
            _ => None,
        }
    }

    pub fn path_bytes(&self) -> Option<&[u8]> {
        match &self.payload {
            SemanticPayload::Path(bytes) => Some(bytes),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticArtifactPayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    String(&'static str),
    Path(&'static [u8]),
    Bytes(&'static [u8]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticStructuralArtifact {
    pub value_type: StructuralType,
    pub payload: StaticArtifactPayload,
}

macro_rules! private_key {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            pub const fn from_word(word: u64) -> Option<Self> {
                if word >> 32 == 0 {
                    None
                } else {
                    Some(Self(word))
                }
            }

            pub const fn get(self) -> u64 {
                self.0
            }

            pub(super) const fn slot(self) -> u32 {
                self.0 as u32
            }

            pub(super) const fn generation(self) -> u32 {
                (self.0 >> 32) as u32
            }

            pub(super) const fn new(slot: u32, generation: NonZeroU32) -> Self {
                Self(((generation.get() as u64) << 32) | slot as u64)
            }
        }
    };
}

private_key!(StructuralDestinationKey);
private_key!(StructuralViewKey);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralFieldPath(Vec<u16>);

impl StructuralFieldPath {
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    pub fn new(fields: Vec<u16>) -> Self {
        Self(fields)
    }

    pub fn as_slice(&self) -> &[u16] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralProjection {
    Field {
        path: StructuralFieldPath,
        expected: StructuralType,
    },
    Utf8 {
        path: StructuralFieldPath,
        expected: StructuralType,
        start: u32,
        end: u32,
    },
}

impl StructuralProjection {
    pub(super) fn path(&self) -> &StructuralFieldPath {
        match self {
            Self::Field { path, .. } | Self::Utf8 { path, .. } => path,
        }
    }

    pub(super) const fn expected(&self) -> StructuralType {
        match self {
            Self::Field { expected, .. } | Self::Utf8 { expected, .. } => *expected,
        }
    }
}
