use std::{convert::Infallible, fmt, num::NonZeroU64};

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

#[derive(Clone)]
pub enum SemanticPayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    String(Vec<u8>),
    Path(Vec<u8>),
    Bytes(Vec<u8>),
    ByteVector(Vec<u8>),
    Product(SemanticChildren),
    Enum {
        tag: u64,
        active_payload: SemanticChildren,
    },
}

impl fmt::Debug for SemanticPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inline(value) => formatter.debug_tuple("Inline").field(value).finish(),
            Self::Static(value) => formatter.debug_tuple("Static").field(value).finish(),
            Self::String(bytes) => formatter
                .debug_struct("String")
                .field("byte_count", &bytes.len())
                .finish(),
            Self::Path(bytes) => formatter
                .debug_struct("Path")
                .field("byte_count", &bytes.len())
                .finish(),
            Self::Bytes(bytes) => formatter
                .debug_struct("Bytes")
                .field("byte_count", &bytes.len())
                .finish(),
            Self::ByteVector(bytes) => formatter
                .debug_struct("ByteVector")
                .field("byte_count", &bytes.len())
                .finish(),
            Self::Product(fields) => formatter
                .debug_struct("Product")
                .field("field_count", &fields.len())
                .finish_non_exhaustive(),
            Self::Enum {
                tag,
                active_payload,
            } => formatter
                .debug_struct("Enum")
                .field("tag", tag)
                .field("field_count", &active_payload.len())
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for SemanticPayload {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Inline(left), Self::Inline(right)) => left == right,
            (Self::Static(left), Self::Static(right)) => left == right,
            (Self::String(left), Self::String(right))
            | (Self::Path(left), Self::Path(right))
            | (Self::Bytes(left), Self::Bytes(right))
            | (Self::ByteVector(left), Self::ByteVector(right)) => left == right,
            (Self::Product(left), Self::Product(right)) => left == right,
            (
                Self::Enum {
                    tag: left_tag,
                    active_payload: left,
                },
                Self::Enum {
                    tag: right_tag,
                    active_payload: right,
                },
            ) => left_tag == right_tag && left == right,
            _ => false,
        }
    }
}

impl Eq for SemanticPayload {}

#[derive(Clone)]
pub struct SemanticValue {
    pub value_type: StructuralType,
    pub payload: SemanticPayload,
}

impl fmt::Debug for SemanticValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticValue")
            .field("value_type", &self.value_type)
            .field("payload", &self.payload)
            .finish()
    }
}

impl PartialEq for SemanticValue {
    fn eq(&self, other: &Self) -> bool {
        Self::slices_equal(std::slice::from_ref(self), std::slice::from_ref(other))
    }
}

impl Eq for SemanticValue {}

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

    /// Complete stack-safe equality for the owned, acyclic semantic value tree.
    pub fn try_equal(&self, other: &Self) -> crate::Result<bool> {
        values_equal_with(
            std::slice::from_ref(self),
            std::slice::from_ref(other),
            |pending, additional| {
                pending
                    .try_reserve(additional)
                    .map_err(|_| crate::Error::host("semantic equality work allocation failed"))
            },
        )
    }

    pub(crate) fn slices_equal(left: &[Self], right: &[Self]) -> bool {
        values_equal_with(left, right, |pending, additional| {
            pending.reserve(additional);
            Ok::<(), Infallible>(())
        })
        .unwrap_or_else(|never| match never {})
    }
}

fn values_equal_with<'a, Failure>(
    left: &'a [SemanticValue],
    right: &'a [SemanticValue],
    mut reserve: impl FnMut(
        &mut Vec<(&'a SemanticValue, &'a SemanticValue)>,
        usize,
    ) -> Result<(), Failure>,
) -> Result<bool, Failure> {
    if left.len() != right.len() {
        return Ok(false);
    }
    let mut pending = Vec::new();
    reserve(&mut pending, left.len())?;
    pending.extend(left.iter().zip(right).rev());
    while let Some((left, right)) = pending.pop() {
        if left.value_type != right.value_type {
            return Ok(false);
        }
        use SemanticPayload as Payload;
        let children = match (&left.payload, &right.payload) {
            (Payload::Inline(left), Payload::Inline(right)) if left == right => None,
            (Payload::Static(left), Payload::Static(right)) if left == right => None,
            (Payload::String(left), Payload::String(right)) if left == right => None,
            (Payload::Path(left), Payload::Path(right)) if left == right => None,
            (Payload::Bytes(left), Payload::Bytes(right)) if left == right => None,
            (Payload::ByteVector(left), Payload::ByteVector(right)) if left == right => None,
            (Payload::Product(left), Payload::Product(right)) => {
                Some((left.as_slice(), right.as_slice()))
            }
            (
                Payload::Enum {
                    tag: left_tag,
                    active_payload: left,
                },
                Payload::Enum {
                    tag: right_tag,
                    active_payload: right,
                },
            ) if left_tag == right_tag => Some((left.as_slice(), right.as_slice())),
            _ => return Ok(false),
        };
        let Some((left, right)) = children else {
            continue;
        };
        if left.len() != right.len() {
            return Ok(false);
        }
        reserve(&mut pending, left.len())?;
        pending.extend(left.iter().zip(right).rev());
    }
    Ok(true)
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
        #[repr(transparent)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub const fn from_word(word: u64) -> Option<Self> {
                match NonZeroU64::new(word) {
                    Some(word) => Some(Self(word)),
                    None => None,
                }
            }

            pub const fn get(self) -> u64 {
                self.0.get()
            }

            pub(super) const fn from_token(token: NonZeroU64) -> Self {
                Self(token)
            }
        }
    };
}

private_key!(StructuralDestinationKey);
private_key!(StructuralViewKey);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralFieldPath(Vec<usize>);

impl StructuralFieldPath {
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    pub fn new(fields: Vec<usize>) -> Self {
        Self(fields)
    }

    pub fn as_slice(&self) -> &[usize] {
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
        start: u64,
        end: u64,
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
