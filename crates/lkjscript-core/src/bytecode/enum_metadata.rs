macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn bytes(self) -> [u8; 32] {
                self.0
            }

            pub fn is_resolved(self) -> bool {
                self.0 != [0; 32]
            }
        }
    };
}

stable_id!(EnumId);
stable_id!(VariantId);
stable_id!(VariantFieldId);
stable_id!(RuntimeLayoutId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumMetadata {
    pub id: EnumId,
    pub name: String,
    pub type_parameter_count: u64,
    pub layout: RuntimeLayoutId,
    pub variants: Vec<EnumVariantMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantMetadata {
    pub id: VariantId,
    pub name: String,
    pub source_order: u64,
    pub physical_tag: u64,
    pub fields: Vec<EnumFieldMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumFieldMetadata {
    pub id: VariantFieldId,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumConstructionRef {
    pub enum_id: EnumId,
    pub variant: VariantId,
    pub layout: RuntimeLayoutId,
    pub substitution_arity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumVariantRef {
    pub enum_id: EnumId,
    pub variant: VariantId,
    pub layout: RuntimeLayoutId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumFieldRef {
    pub enum_id: EnumId,
    pub variant: VariantId,
    pub field: VariantFieldId,
    pub layout: RuntimeLayoutId,
}
