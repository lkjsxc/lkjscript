use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMetadata {
    pub id: u32,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductMetadata {
    pub id: ProductId,
    pub name: String,
    pub fields: Vec<ProductField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductField {
    pub name: String,
    pub ty: SsaType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumMetadata {
    pub id: EnumId,
    pub name: String,
    pub type_parameters: Vec<String>,
    pub variants: Vec<EnumVariantMetadata>,
    pub layout: EnumLayoutFacts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantMetadata {
    pub id: VariantId,
    pub name: String,
    pub physical_tag: u16,
    pub fields: Vec<EnumFieldMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumFieldMetadata {
    pub id: VariantFieldId,
    pub name: String,
    pub ty: SsaType,
    pub indirect: bool,
    pub traced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumLayoutFacts {
    pub identity: RuntimeLayoutId,
    pub recursive: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct EffectSet(u16);

impl EffectSet {
    pub const PURE: Self = Self(0);
    pub const ALLOCATES: Self = Self(1 << 0);
    pub const READS_MEMORY: Self = Self(1 << 1);
    pub const WRITES_MEMORY: Self = Self(1 << 2);
    pub const MUTATES_LOCAL: Self = Self(1 << 3);
    pub const HOST_IO: Self = Self(1 << 4);
    pub const MAY_TRAP: Self = Self(1 << 5);
    pub const MAY_EXIT: Self = Self(1 << 6);
    pub const MAY_DIVERGE: Self = Self(1 << 7);
    pub const CONSERVATIVE_CALL: Self = Self::ALLOCATES
        .union(Self::READS_MEMORY)
        .union(Self::WRITES_MEMORY)
        .union(Self::MUTATES_LOCAL)
        .union(Self::HOST_IO)
        .union(Self::MAY_TRAP)
        .union(Self::MAY_EXIT)
        .union(Self::MAY_DIVERGE);

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, effects: Self) -> bool {
        self.0 & effects.0 == effects.0
    }

    pub const fn is_pure(self) -> bool {
        self.0 == 0
    }
}
