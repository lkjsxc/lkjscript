#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Any,
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Symbol,
    Proto(u32),
    Closure(u32),
    List,
    StaticBytes,
    Bytes(u32),
    BytesBorrow {
        owner: u32,
        used: bool,
    },
    ByteVector(u32),
    ByteSlice {
        owner: u32,
        mutable: bool,
        used: bool,
    },
    Path,
    Capability(crate::CapabilityKind),
    Resource {
        kind: crate::ResourceKind,
        owner: u32,
    },
    ResourceResult {
        kind: crate::ResourceKind,
        owner: u32,
    },
    Product(ProductId),
    Enum(EnumId, Option<VariantId>),
    StructuralOwner {
        representation: crate::StructuralRepresentationId,
        owner: u32,
        active_variant: Option<crate::VariantId>,
    },
    StructuralOwnerRef {
        representation: crate::StructuralRepresentationId,
        owner: u32,
        active_variant: Option<crate::VariantId>,
    },
    StructuralView {
        representation: crate::StructuralRepresentationId,
        owner: u32,
        mutable: bool,
        used: bool,
    },
    StructuralDestination {
        destination: crate::StructuralDestinationId,
        identity: u32,
    },
}

impl std::fmt::Display for Kind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Any => "any",
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Str => "string",
            Self::Symbol => "symbol",
            Self::Proto(_) => "function-prototype",
            Self::Closure(_) => "function",
            Self::List => "list",
            Self::StaticBytes => "static bytes",
            Self::Bytes(_) => "dynamic bytes",
            Self::BytesBorrow { .. } => "borrowed dynamic bytes",
            Self::ByteVector(_) => "byte-vector",
            Self::ByteSlice { mutable: false, .. } => "byte-slice",
            Self::ByteSlice { mutable: true, .. } => "byte-slice-mut",
            Self::Path => "path",
            Self::Capability(_) => "capability",
            Self::Resource { .. } => "resource",
            Self::ResourceResult { .. } => "result resource",
            Self::Product(_) => "product",
            Self::Enum(_, _) => "enum",
            Self::StructuralOwner { .. } => "structural-owner",
            Self::StructuralOwnerRef { .. } => "structural-owner-reference",
            Self::StructuralView { .. } => "structural-view",
            Self::StructuralDestination { .. } => "structural-destination",
        };
        formatter.write_str(name)?;
        match self {
            Self::Proto(id) | Self::Closure(id) => write!(formatter, " {id}"),
            Self::Capability(kind) => write!(formatter, " {}", kind.as_str()),
            Self::Resource { kind, owner } | Self::ResourceResult { kind, owner } => {
                write!(formatter, " {} owner {owner}", kind.as_str())
            }
            Self::Bytes(owner)
            | Self::BytesBorrow { owner, .. }
            | Self::ByteVector(owner)
            | Self::ByteSlice { owner, .. } => {
                write!(formatter, " owner {owner}")
            }
            Self::Product(id) => write!(formatter, " {}", id.raw()),
            Self::Enum(_, Some(_)) => formatter.write_str(" variant"),
            Self::Enum(_, None) => Ok(()),
            Self::StructuralOwner {
                representation,
                owner,
                ..
            }
            | Self::StructuralOwnerRef {
                representation,
                owner,
                ..
            }
            | Self::StructuralView {
                representation,
                owner,
                ..
            } => write!(
                formatter,
                " representation {} owner {owner}",
                representation.raw()
            ),
            Self::StructuralDestination {
                destination,
                identity,
            } => write!(
                formatter,
                " metadata {} identity {identity}",
                destination.raw()
            ),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UniquePlaceState {
    Inactive,
    Active {
        owner: Option<u32>,
        transferred: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructuralDestinationState {
    pub(super) destination: crate::StructuralDestinationId,
    pub(super) initialized: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct State {
    pub(super) stack: Vec<Kind>,
    pub(super) locals: Vec<Option<Kind>>,
    pub(super) globals: Vec<Option<Kind>>,
    pub(super) unique_places: Vec<UniquePlaceState>,
    pub(super) structural_destinations: std::collections::BTreeMap<u32, StructuralDestinationState>,
}
