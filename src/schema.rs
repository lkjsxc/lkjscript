use crate::ids::NodeId;
use crate::transaction::{ExpressionKindDraft, NodeTarget};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

pub const MAXIMUM_BYTE_STRING_BYTES: usize = 64 * 1024;
pub const MAXIMUM_BYTE_STRING_ENCODED_BYTES: usize = (MAXIMUM_BYTE_STRING_BYTES / 3) * 4
    + match MAXIMUM_BYTE_STRING_BYTES % 3 {
        0 => 0,
        1 => 2,
        2 => 3,
        _ => 0,
    };
pub const MAXIMUM_BYTE_LITERAL_BYTES: usize = 4 * 1024;
pub const MAXIMUM_TRANSACTION_BYTE_LITERAL_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ByteString(Box<[u8]>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ByteStringTooLarge;

impl fmt::Display for ByteStringTooLarge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("byte string exceeds decoded byte policy")
    }
}

impl std::error::Error for ByteStringTooLarge {}

impl ByteString {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ByteStringTooLarge> {
        let bytes = bytes.into();
        if bytes.len() > MAXIMUM_BYTE_STRING_BYTES {
            return Err(ByteStringTooLarge);
        }
        Ok(Self(bytes.into_boxed_slice()))
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, ByteStringTooLarge> {
        if bytes.len() > MAXIMUM_BYTE_STRING_BYTES {
            return Err(ByteStringTooLarge);
        }
        Ok(Self(bytes.into()))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.into_vec()
    }
}

impl Serialize for ByteString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&URL_SAFE_NO_PAD.encode(&self.0))
    }
}

struct ByteStringVisitor;

fn unpadded_base64_decoded_length(encoded: usize) -> Option<usize> {
    let complete = (encoded / 4).checked_mul(3)?;
    complete.checked_add(match encoded % 4 {
        0 => 0,
        2 => 1,
        3 => 2,
        _ => return None,
    })
}

impl<'de> de::Visitor<'de> for ByteStringVisitor {
    type Value = ByteString;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("canonical unpadded URL-safe base64")
    }

    fn visit_str<E>(self, encoded: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let decoded_length = unpadded_base64_decoded_length(encoded.len())
            .ok_or_else(|| E::custom("byte string has an invalid unpadded base64 length"))?;
        if encoded.len() > MAXIMUM_BYTE_STRING_ENCODED_BYTES
            || decoded_length > MAXIMUM_BYTE_STRING_BYTES
        {
            return Err(E::custom("byte string exceeds decoded byte policy"));
        }
        let decoded = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| E::custom("byte string is not canonical unpadded URL-safe base64"))?;
        if decoded.len() > MAXIMUM_BYTE_STRING_BYTES || URL_SAFE_NO_PAD.encode(&decoded) != encoded
        {
            return Err(E::custom(
                "byte string is not canonical unpadded URL-safe base64",
            ));
        }
        Ok(ByteString(decoded.into_boxed_slice()))
    }
}

impl<'de> Deserialize<'de> for ByteString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ByteStringVisitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueStorageClass {
    ZeroCell,
    Immediate,
    FixedAggregate,
    ManagedHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimitiveDescriptor {
    pub ty: SemanticType,
    pub machine_name: &'static str,
    pub stable_tag: u8,
    pub storage_class: ValueStorageClass,
    pub physical_slot_size: u64,
    pub physical_slot_align: u64,
    pub cells: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    Unit,
    Bool,
    I64,
    Bytes,
    Nominal(NodeId),
}

impl SemanticType {
    pub const PRIMITIVES: [Self; 4] = [Self::Unit, Self::Bool, Self::I64, Self::Bytes];
    pub const ALL: [Self; 4] = Self::PRIMITIVES;

    pub const PRIMITIVE_DESCRIPTORS: [PrimitiveDescriptor; 4] = [
        PrimitiveDescriptor {
            ty: Self::Unit,
            machine_name: "unit",
            stable_tag: 1,
            storage_class: ValueStorageClass::ZeroCell,
            physical_slot_size: 0,
            physical_slot_align: 1,
            cells: 0,
        },
        PrimitiveDescriptor {
            ty: Self::Bool,
            machine_name: "bool",
            stable_tag: 2,
            storage_class: ValueStorageClass::Immediate,
            physical_slot_size: 1,
            physical_slot_align: 1,
            cells: 1,
        },
        PrimitiveDescriptor {
            ty: Self::I64,
            machine_name: "i64",
            stable_tag: 3,
            storage_class: ValueStorageClass::Immediate,
            physical_slot_size: 8,
            physical_slot_align: 8,
            cells: 1,
        },
        PrimitiveDescriptor {
            ty: Self::Bytes,
            machine_name: "bytes",
            stable_tag: 5,
            storage_class: ValueStorageClass::ManagedHandle,
            physical_slot_size: 8,
            physical_slot_align: 8,
            cells: 1,
        },
    ];

    pub const fn primitive_descriptor(self) -> Option<&'static PrimitiveDescriptor> {
        match self {
            Self::Unit => Some(&Self::PRIMITIVE_DESCRIPTORS[0]),
            Self::Bool => Some(&Self::PRIMITIVE_DESCRIPTORS[1]),
            Self::I64 => Some(&Self::PRIMITIVE_DESCRIPTORS[2]),
            Self::Bytes => Some(&Self::PRIMITIVE_DESCRIPTORS[3]),
            Self::Nominal(_) => None,
        }
    }

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::Bytes => "bytes",
            Self::Nominal(_) => "nominal",
        }
    }

    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::Unit => 1,
            Self::Bool => 2,
            Self::I64 => 3,
            Self::Bytes => 5,
            Self::Nominal(_) => 4,
        }
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Unit),
            2 => Some(Self::Bool),
            3 => Some(Self::I64),
            5 => Some(Self::Bytes),
            _ => None,
        }
    }

    pub const fn nominal_target(self) -> Option<NodeId> {
        match self {
            Self::Nominal(target) => Some(target),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeDraft {
    Unit,
    Bool,
    I64,
    Bytes,
    Nominal(NodeTarget),
}

impl From<SemanticType> for TypeDraft {
    fn from(value: SemanticType) -> Self {
        match value {
            SemanticType::Unit => Self::Unit,
            SemanticType::Bool => Self::Bool,
            SemanticType::I64 => Self::I64,
            SemanticType::Bytes => Self::Bytes,
            SemanticType::Nominal(target) => Self::Nominal(NodeTarget::Existing(target)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    WorkspaceRoot,
    Package,
    Module,
    Function,
    Parameter,
    Region,
    Block,
    Operation,
    BlockArgument,
    ProductType,
    ProductField,
    SumType,
    SumVariant,
}

impl NodeKind {
    pub const ALL: [Self; 13] = [
        Self::WorkspaceRoot,
        Self::Package,
        Self::Module,
        Self::Function,
        Self::Parameter,
        Self::Region,
        Self::Block,
        Self::Operation,
        Self::BlockArgument,
        Self::ProductType,
        Self::ProductField,
        Self::SumType,
        Self::SumVariant,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::WorkspaceRoot => "workspace_root",
            Self::Package => "package",
            Self::Module => "module",
            Self::Function => "function",
            Self::Parameter => "parameter",
            Self::Region => "region",
            Self::Block => "block",
            Self::Operation => "operation",
            Self::BlockArgument => "block_argument",
            Self::ProductType => "product_type",
            Self::ProductField => "product_field",
            Self::SumType => "sum_type",
            Self::SumVariant => "sum_variant",
        }
    }

    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::WorkspaceRoot => 1,
            Self::Package => 2,
            Self::Module => 3,
            Self::Function => 4,
            Self::Parameter => 5,
            Self::Region => 6,
            Self::Block => 7,
            Self::Operation => 8,
            Self::BlockArgument => 9,
            Self::ProductType => 10,
            Self::ProductField => 11,
            Self::SumType => 12,
            Self::SumVariant => 13,
        }
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::WorkspaceRoot),
            2 => Some(Self::Package),
            3 => Some(Self::Module),
            4 => Some(Self::Function),
            5 => Some(Self::Parameter),
            6 => Some(Self::Region),
            7 => Some(Self::Block),
            8 => Some(Self::Operation),
            9 => Some(Self::BlockArgument),
            10 => Some(Self::ProductType),
            11 => Some(Self::ProductField),
            12 => Some(Self::SumType),
            13 => Some(Self::SumVariant),
            _ => None,
        }
    }
}

pub const MINIMUM_NAME_UTF8_BYTES: usize = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NameUniquenessGroup {
    WorkspacePackages,
    PackageModules,
    ModuleTypes,
    ModuleFunctions,
    ProductFields,
    SumVariants,
    FunctionParameters,
}

impl NameUniquenessGroup {
    pub const ALL: [Self; 7] = [
        Self::WorkspacePackages,
        Self::PackageModules,
        Self::ModuleTypes,
        Self::ModuleFunctions,
        Self::ProductFields,
        Self::SumVariants,
        Self::FunctionParameters,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::WorkspacePackages => "workspace.packages",
            Self::PackageModules => "package.modules",
            Self::ModuleTypes => "module.types",
            Self::ModuleFunctions => "module.functions",
            Self::ProductFields => "product.fields",
            Self::SumVariants => "sum.variants",
            Self::FunctionParameters => "function.parameters",
        }
    }

    pub const fn owner_kind(self) -> NodeKind {
        match self {
            Self::WorkspacePackages => NodeKind::WorkspaceRoot,
            Self::PackageModules => NodeKind::Package,
            Self::ModuleTypes | Self::ModuleFunctions => NodeKind::Module,
            Self::ProductFields => NodeKind::ProductType,
            Self::SumVariants => NodeKind::SumType,
            Self::FunctionParameters => NodeKind::Function,
        }
    }

    pub const fn member_kinds(self) -> &'static [NodeKind] {
        match self {
            Self::WorkspacePackages => &[NodeKind::Package],
            Self::PackageModules => &[NodeKind::Module],
            Self::ModuleTypes => &[NodeKind::ProductType, NodeKind::SumType],
            Self::ModuleFunctions => &[NodeKind::Function],
            Self::ProductFields => &[NodeKind::ProductField],
            Self::SumVariants => &[NodeKind::SumVariant],
            Self::FunctionParameters => &[NodeKind::Parameter],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperandUse {
    Read,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionRole {
    IfThen,
    IfElse,
    ForBody,
    MatchArm(NodeId),
}

impl RegionRole {
    pub const ALL_STATIC: [Self; 3] = [Self::IfThen, Self::IfElse, Self::ForBody];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::IfThen => "then",
            Self::IfElse => "else",
            Self::ForBody => "body",
            Self::MatchArm(_) => "match_arm",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockArgumentRole {
    LoopIndex,
    LoopCarried,
    MatchPayload,
}

impl BlockArgumentRole {
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::LoopIndex => "loop_index",
            Self::LoopCarried => "loop_carried",
            Self::MatchPayload => "match_payload",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationCode {
    ConstI64,
    ConstBool,
    AddI64,
    Hole,
    Return,
    ConstUnit,
    LtI64,
    Call,
    If,
    ForI64,
    Yield,
    ConstructProduct,
    ProjectField,
    ConstructVariant,
    MatchSum,
    ConstBytes,
    BytesLen,
    BytesAt,
    BytesSlice,
    BytesEqual,
    BytesConcat,
}

impl OperationCode {
    pub const ALL: [Self; 21] = [
        Self::ConstUnit,
        Self::ConstI64,
        Self::ConstBool,
        Self::AddI64,
        Self::LtI64,
        Self::Call,
        Self::Hole,
        Self::If,
        Self::ForI64,
        Self::Return,
        Self::Yield,
        Self::ConstructProduct,
        Self::ProjectField,
        Self::ConstructVariant,
        Self::MatchSum,
        Self::ConstBytes,
        Self::BytesLen,
        Self::BytesAt,
        Self::BytesSlice,
        Self::BytesEqual,
        Self::BytesConcat,
    ];

    pub const fn stable_tag(self) -> u8 {
        self.descriptor().stable_tag
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::ConstI64),
            2 => Some(Self::ConstBool),
            3 => Some(Self::AddI64),
            4 => Some(Self::Hole),
            5 => Some(Self::Return),
            6 => Some(Self::ConstUnit),
            7 => Some(Self::LtI64),
            8 => Some(Self::Call),
            9 => Some(Self::If),
            10 => Some(Self::ForI64),
            11 => Some(Self::Yield),
            12 => Some(Self::ConstructProduct),
            13 => Some(Self::ProjectField),
            14 => Some(Self::ConstructVariant),
            15 => Some(Self::MatchSum),
            16 => Some(Self::ConstBytes),
            17 => Some(Self::BytesLen),
            18 => Some(Self::BytesAt),
            19 => Some(Self::BytesSlice),
            20 => Some(Self::BytesEqual),
            21 => Some(Self::BytesConcat),
            _ => None,
        }
    }

    pub const fn machine_name(self) -> &'static str {
        self.descriptor().machine_name
    }

    pub const fn descriptor(self) -> &'static OperationDescriptor {
        match self {
            Self::ConstUnit => &CONST_UNIT_DESCRIPTOR,
            Self::ConstI64 => &CONST_I64_DESCRIPTOR,
            Self::ConstBool => &CONST_BOOL_DESCRIPTOR,
            Self::AddI64 => &ADD_I64_DESCRIPTOR,
            Self::LtI64 => &LT_I64_DESCRIPTOR,
            Self::Call => &CALL_DESCRIPTOR,
            Self::Hole => &HOLE_DESCRIPTOR,
            Self::If => &IF_DESCRIPTOR,
            Self::ForI64 => &FOR_I64_DESCRIPTOR,
            Self::Return => &RETURN_DESCRIPTOR,
            Self::Yield => &YIELD_DESCRIPTOR,
            Self::ConstructProduct => &CONSTRUCT_PRODUCT_DESCRIPTOR,
            Self::ProjectField => &PROJECT_FIELD_DESCRIPTOR,
            Self::ConstructVariant => &CONSTRUCT_VARIANT_DESCRIPTOR,
            Self::MatchSum => &MATCH_SUM_DESCRIPTOR,
            Self::ConstBytes => &CONST_BYTES_DESCRIPTOR,
            Self::BytesLen => &BYTES_LEN_DESCRIPTOR,
            Self::BytesAt => &BYTES_AT_DESCRIPTOR,
            Self::BytesSlice => &BYTES_SLICE_DESCRIPTOR,
            Self::BytesEqual => &BYTES_EQUAL_DESCRIPTOR,
            Self::BytesConcat => &BYTES_CONCAT_DESCRIPTOR,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TypeRule {
    Fixed(SemanticType),
    PayloadExpected,
    OwnerFunctionResult,
    PayloadResult,
    PayloadCarried,
    CallTargetParameter,
    CallTargetResult,
    OwningRegionYield,
    ProductFieldType,
    ProductDeclarationResult,
    ProjectionOwner,
    ProjectedFieldResult,
    VariantPayload,
    VariantOwnerResult,
    MatchScrutinee,
    MatchResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiteralField {
    I64Value,
    BoolValue,
    ExpectedType,
    ResultType,
    CarriedType,
    PositiveStep,
    BytesValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperandArity {
    Fixed(u8),
    CallTargetParameters,
    ProductFields,
    VariantPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RegionArity {
    Fixed(u8),
    MatchVariants {
        payload_type: TypeRule,
        terminator: OperationCode,
        yield_type: TypeRule,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperandDescriptor {
    pub ty: TypeRule,
    pub use_mode: OperandUse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockArgumentDescriptor {
    pub role: BlockArgumentRole,
    pub ty: TypeRule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionDescriptor {
    pub role: RegionRole,
    pub block_arguments: &'static [BlockArgumentDescriptor],
    pub terminator: OperationCode,
    pub yield_type: TypeRule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationDescriptor {
    pub code: OperationCode,
    pub machine_name: &'static str,
    pub stable_tag: u8,
    pub operand_arity: OperandArity,
    /// Fixed operands, or the repeated per-argument prototype for dynamic calls.
    pub operands: &'static [OperandDescriptor],
    pub results: &'static [TypeRule],
    pub literal_fields: &'static [LiteralField],
    pub region_arity: RegionArity,
    pub regions: &'static [RegionDescriptor],
    pub terminator: bool,
    pub complete: bool,
}

const NO_OPERANDS: &[OperandDescriptor] = &[];
const NO_RESULTS: &[TypeRule] = &[];
const NO_LITERALS: &[LiteralField] = &[];
const NO_REGIONS: &[RegionDescriptor] = &[];
const UNIT_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::Unit)];
const I64_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::I64)];
const BOOL_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::Bool)];
const BYTES_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::Bytes)];
const PAYLOAD_RESULT: &[TypeRule] = &[TypeRule::PayloadExpected];
const STRUCTURED_RESULT: &[TypeRule] = &[TypeRule::PayloadResult];
const CARRIED_RESULT: &[TypeRule] = &[TypeRule::PayloadCarried];
const CALL_RESULT: &[TypeRule] = &[TypeRule::CallTargetResult];
const PRODUCT_RESULT: &[TypeRule] = &[TypeRule::ProductDeclarationResult];
const PROJECT_RESULT: &[TypeRule] = &[TypeRule::ProjectedFieldResult];
const VARIANT_RESULT: &[TypeRule] = &[TypeRule::VariantOwnerResult];
const MATCH_RESULT: &[TypeRule] = &[TypeRule::MatchResult];
const PRODUCT_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::ProductFieldType,
    use_mode: OperandUse::Read,
}];
const PROJECT_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::ProjectionOwner,
    use_mode: OperandUse::Read,
}];
const VARIANT_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::VariantPayload,
    use_mode: OperandUse::Read,
}];
const MATCH_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::MatchScrutinee,
    use_mode: OperandUse::Read,
}];
const I64_BINARY_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
];
const CALL_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::CallTargetParameter,
    use_mode: OperandUse::Read,
}];
const IF_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::Fixed(SemanticType::Bool),
    use_mode: OperandUse::Read,
}];
const FOR_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::PayloadCarried,
        use_mode: OperandUse::Read,
    },
];
const RETURN_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::OwnerFunctionResult,
    use_mode: OperandUse::Read,
}];
const YIELD_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::OwningRegionYield,
    use_mode: OperandUse::Read,
}];
const BYTES_UNARY_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::Fixed(SemanticType::Bytes),
    use_mode: OperandUse::Read,
}];
const BYTES_AT_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::Bytes),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
];
const BYTES_SLICE_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::Bytes),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Read,
    },
];
const BYTES_BINARY_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::Bytes),
        use_mode: OperandUse::Read,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::Bytes),
        use_mode: OperandUse::Read,
    },
];
const I64_LITERAL: &[LiteralField] = &[LiteralField::I64Value];
const BOOL_LITERAL: &[LiteralField] = &[LiteralField::BoolValue];
const BYTES_LITERAL: &[LiteralField] = &[LiteralField::BytesValue];
const EXPECTED_LITERAL: &[LiteralField] = &[LiteralField::ExpectedType];
const IF_LITERALS: &[LiteralField] = &[LiteralField::ResultType];
const FOR_LITERALS: &[LiteralField] = &[LiteralField::PositiveStep, LiteralField::CarriedType];
const NO_BLOCK_ARGUMENTS: &[BlockArgumentDescriptor] = &[];
const FOR_BLOCK_ARGUMENTS: &[BlockArgumentDescriptor] = &[
    BlockArgumentDescriptor {
        role: BlockArgumentRole::LoopIndex,
        ty: TypeRule::Fixed(SemanticType::I64),
    },
    BlockArgumentDescriptor {
        role: BlockArgumentRole::LoopCarried,
        ty: TypeRule::PayloadCarried,
    },
];
const IF_REGIONS: &[RegionDescriptor] = &[
    RegionDescriptor {
        role: RegionRole::IfThen,
        block_arguments: NO_BLOCK_ARGUMENTS,
        terminator: OperationCode::Yield,
        yield_type: TypeRule::PayloadResult,
    },
    RegionDescriptor {
        role: RegionRole::IfElse,
        block_arguments: NO_BLOCK_ARGUMENTS,
        terminator: OperationCode::Yield,
        yield_type: TypeRule::PayloadResult,
    },
];
const FOR_REGIONS: &[RegionDescriptor] = &[RegionDescriptor {
    role: RegionRole::ForBody,
    block_arguments: FOR_BLOCK_ARGUMENTS,
    terminator: OperationCode::Yield,
    yield_type: TypeRule::PayloadCarried,
}];

macro_rules! descriptor {
    ($name:ident, $code:ident, $machine:literal, $tag:literal, $arity:expr, $operands:expr, $results:expr, $literals:expr, $regions:expr, $terminator:expr, $complete:expr) => {
        static $name: OperationDescriptor = OperationDescriptor {
            code: OperationCode::$code,
            machine_name: $machine,
            stable_tag: $tag,
            operand_arity: $arity,
            operands: $operands,
            results: $results,
            literal_fields: $literals,
            region_arity: RegionArity::Fixed($regions.len() as u8),
            regions: $regions,
            terminator: $terminator,
            complete: $complete,
        };
    };
}

descriptor!(
    CONST_I64_DESCRIPTOR,
    ConstI64,
    "const_i64",
    1,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    I64_RESULT,
    I64_LITERAL,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    CONST_BOOL_DESCRIPTOR,
    ConstBool,
    "const_bool",
    2,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    BOOL_RESULT,
    BOOL_LITERAL,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    ADD_I64_DESCRIPTOR,
    AddI64,
    "add_i64",
    3,
    OperandArity::Fixed(2),
    I64_BINARY_OPERANDS,
    I64_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    HOLE_DESCRIPTOR,
    Hole,
    "hole",
    4,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    PAYLOAD_RESULT,
    EXPECTED_LITERAL,
    NO_REGIONS,
    false,
    false
);
descriptor!(
    RETURN_DESCRIPTOR,
    Return,
    "return",
    5,
    OperandArity::Fixed(1),
    RETURN_OPERANDS,
    NO_RESULTS,
    NO_LITERALS,
    NO_REGIONS,
    true,
    true
);
descriptor!(
    CONST_UNIT_DESCRIPTOR,
    ConstUnit,
    "const_unit",
    6,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    UNIT_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    LT_I64_DESCRIPTOR,
    LtI64,
    "lt_i64",
    7,
    OperandArity::Fixed(2),
    I64_BINARY_OPERANDS,
    BOOL_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    CALL_DESCRIPTOR,
    Call,
    "call",
    8,
    OperandArity::CallTargetParameters,
    CALL_OPERANDS,
    CALL_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    IF_DESCRIPTOR,
    If,
    "if",
    9,
    OperandArity::Fixed(1),
    IF_OPERANDS,
    STRUCTURED_RESULT,
    IF_LITERALS,
    IF_REGIONS,
    false,
    true
);
descriptor!(
    FOR_I64_DESCRIPTOR,
    ForI64,
    "for_i64",
    10,
    OperandArity::Fixed(3),
    FOR_OPERANDS,
    CARRIED_RESULT,
    FOR_LITERALS,
    FOR_REGIONS,
    false,
    true
);
descriptor!(
    YIELD_DESCRIPTOR,
    Yield,
    "yield",
    11,
    OperandArity::Fixed(1),
    YIELD_OPERANDS,
    NO_RESULTS,
    NO_LITERALS,
    NO_REGIONS,
    true,
    true
);
descriptor!(
    CONSTRUCT_PRODUCT_DESCRIPTOR,
    ConstructProduct,
    "construct_product",
    12,
    OperandArity::ProductFields,
    PRODUCT_OPERANDS,
    PRODUCT_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    PROJECT_FIELD_DESCRIPTOR,
    ProjectField,
    "project_field",
    13,
    OperandArity::Fixed(1),
    PROJECT_OPERANDS,
    PROJECT_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    CONSTRUCT_VARIANT_DESCRIPTOR,
    ConstructVariant,
    "construct_variant",
    14,
    OperandArity::VariantPayload,
    VARIANT_OPERANDS,
    VARIANT_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
static MATCH_SUM_DESCRIPTOR: OperationDescriptor = OperationDescriptor {
    code: OperationCode::MatchSum,
    machine_name: "match_sum",
    stable_tag: 15,
    operand_arity: OperandArity::Fixed(1),
    operands: MATCH_OPERANDS,
    results: MATCH_RESULT,
    literal_fields: NO_LITERALS,
    region_arity: RegionArity::MatchVariants {
        payload_type: TypeRule::VariantPayload,
        terminator: OperationCode::Yield,
        yield_type: TypeRule::MatchResult,
    },
    regions: NO_REGIONS,
    terminator: false,
    complete: true,
};
descriptor!(
    CONST_BYTES_DESCRIPTOR,
    ConstBytes,
    "const_bytes",
    16,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    BYTES_RESULT,
    BYTES_LITERAL,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    BYTES_LEN_DESCRIPTOR,
    BytesLen,
    "bytes_len",
    17,
    OperandArity::Fixed(1),
    BYTES_UNARY_OPERANDS,
    I64_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    BYTES_AT_DESCRIPTOR,
    BytesAt,
    "bytes_at",
    18,
    OperandArity::Fixed(2),
    BYTES_AT_OPERANDS,
    I64_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    BYTES_SLICE_DESCRIPTOR,
    BytesSlice,
    "bytes_slice",
    19,
    OperandArity::Fixed(3),
    BYTES_SLICE_OPERANDS,
    BYTES_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    BYTES_EQUAL_DESCRIPTOR,
    BytesEqual,
    "bytes_equal",
    20,
    OperandArity::Fixed(2),
    BYTES_BINARY_OPERANDS,
    BOOL_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    BYTES_CONCAT_DESCRIPTOR,
    BytesConcat,
    "bytes_concat",
    21,
    OperandArity::Fixed(2),
    BYTES_BINARY_OPERANDS,
    BYTES_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ValueRef {
    FunctionParameter(NodeId),
    BlockArgument(NodeId),
    OperationResult { operation: NodeId, output: u8 },
}

impl ValueRef {
    pub const fn referenced_node(self) -> NodeId {
        match self {
            Self::FunctionParameter(parameter) | Self::BlockArgument(parameter) => parameter,
            Self::OperationResult { operation, .. } => operation,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ValueDraft {
    FunctionParameter(NodeTarget),
    BlockArgument(NodeTarget),
    OperationResult { operation: NodeTarget, output: u8 },
    InlineExpression(Box<ExpressionKindDraft>),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductFieldValueDraft {
    pub field: NodeTarget,
    pub value: ValueDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchArmOperationDraft {
    pub variant: NodeTarget,
    pub region: NodeTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationDraft {
    ConstUnit,
    ConstI64(i64),
    ConstBool(bool),
    ConstBytes(ByteString),
    AddI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    LtI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    BytesLen {
        value: ValueDraft,
    },
    BytesAt {
        value: ValueDraft,
        index: ValueDraft,
    },
    BytesSlice {
        value: ValueDraft,
        start: ValueDraft,
        length: ValueDraft,
    },
    BytesEqual {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    BytesConcat {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    Call {
        function: NodeTarget,
        arguments: Vec<ValueDraft>,
    },
    Hole {
        expected: TypeDraft,
    },
    If {
        condition: ValueDraft,
        result: TypeDraft,
        then_region: NodeTarget,
        else_region: NodeTarget,
    },
    ForI64 {
        start: ValueDraft,
        end_exclusive: ValueDraft,
        step: i64,
        initial: ValueDraft,
        carried: TypeDraft,
        body_region: NodeTarget,
    },
    Return {
        value: ValueDraft,
    },
    Yield {
        value: ValueDraft,
    },
    ConstructProduct {
        product: NodeTarget,
        fields: Vec<ProductFieldValueDraft>,
    },
    ProjectField {
        value: ValueDraft,
        field: NodeTarget,
    },
    ConstructVariant {
        variant: NodeTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<ValueDraft>,
    },
    MatchSum {
        scrutinee: ValueDraft,
        result: TypeDraft,
        arms: Vec<MatchArmOperationDraft>,
    },
}

impl OperationDraft {
    pub const fn code(&self) -> OperationCode {
        match self {
            Self::ConstUnit => OperationCode::ConstUnit,
            Self::ConstI64(_) => OperationCode::ConstI64,
            Self::ConstBool(_) => OperationCode::ConstBool,
            Self::ConstBytes(_) => OperationCode::ConstBytes,
            Self::AddI64 { .. } => OperationCode::AddI64,
            Self::LtI64 { .. } => OperationCode::LtI64,
            Self::BytesLen { .. } => OperationCode::BytesLen,
            Self::BytesAt { .. } => OperationCode::BytesAt,
            Self::BytesSlice { .. } => OperationCode::BytesSlice,
            Self::BytesEqual { .. } => OperationCode::BytesEqual,
            Self::BytesConcat { .. } => OperationCode::BytesConcat,
            Self::Call { .. } => OperationCode::Call,
            Self::Hole { .. } => OperationCode::Hole,
            Self::If { .. } => OperationCode::If,
            Self::ForI64 { .. } => OperationCode::ForI64,
            Self::Return { .. } => OperationCode::Return,
            Self::Yield { .. } => OperationCode::Yield,
            Self::ConstructProduct { .. } => OperationCode::ConstructProduct,
            Self::ProjectField { .. } => OperationCode::ProjectField,
            Self::ConstructVariant { .. } => OperationCode::ConstructVariant,
            Self::MatchSum { .. } => OperationCode::MatchSum,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductFieldValue {
    pub field: NodeId,
    pub value: ValueRef,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchArm {
    pub variant: NodeId,
    pub region: NodeId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationKind {
    ConstUnit,
    ConstI64(i64),
    ConstBool(bool),
    ConstBytes(ByteString),
    AddI64 {
        lhs: ValueRef,
        rhs: ValueRef,
    },
    LtI64 {
        lhs: ValueRef,
        rhs: ValueRef,
    },
    BytesLen {
        value: ValueRef,
    },
    BytesAt {
        value: ValueRef,
        index: ValueRef,
    },
    BytesSlice {
        value: ValueRef,
        start: ValueRef,
        length: ValueRef,
    },
    BytesEqual {
        lhs: ValueRef,
        rhs: ValueRef,
    },
    BytesConcat {
        lhs: ValueRef,
        rhs: ValueRef,
    },
    Call {
        function: NodeId,
        arguments: Vec<ValueRef>,
    },
    Hole {
        expected: SemanticType,
    },
    If {
        condition: ValueRef,
        result: SemanticType,
        then_region: NodeId,
        else_region: NodeId,
    },
    ForI64 {
        start: ValueRef,
        end_exclusive: ValueRef,
        step: i64,
        initial: ValueRef,
        carried: SemanticType,
        body_region: NodeId,
    },
    Return {
        value: ValueRef,
    },
    Yield {
        value: ValueRef,
    },
    ConstructProduct {
        product: NodeId,
        fields: Vec<ProductFieldValue>,
    },
    ProjectField {
        value: ValueRef,
        field: NodeId,
    },
    ConstructVariant {
        variant: NodeId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<ValueRef>,
    },
    MatchSum {
        scrutinee: ValueRef,
        result: SemanticType,
        arms: Vec<MatchArm>,
    },
}

impl OperationKind {
    pub const fn code(&self) -> OperationCode {
        match self {
            Self::ConstUnit => OperationCode::ConstUnit,
            Self::ConstI64(_) => OperationCode::ConstI64,
            Self::ConstBool(_) => OperationCode::ConstBool,
            Self::ConstBytes(_) => OperationCode::ConstBytes,
            Self::AddI64 { .. } => OperationCode::AddI64,
            Self::LtI64 { .. } => OperationCode::LtI64,
            Self::BytesLen { .. } => OperationCode::BytesLen,
            Self::BytesAt { .. } => OperationCode::BytesAt,
            Self::BytesSlice { .. } => OperationCode::BytesSlice,
            Self::BytesEqual { .. } => OperationCode::BytesEqual,
            Self::BytesConcat { .. } => OperationCode::BytesConcat,
            Self::Call { .. } => OperationCode::Call,
            Self::Hole { .. } => OperationCode::Hole,
            Self::If { .. } => OperationCode::If,
            Self::ForI64 { .. } => OperationCode::ForI64,
            Self::Return { .. } => OperationCode::Return,
            Self::Yield { .. } => OperationCode::Yield,
            Self::ConstructProduct { .. } => OperationCode::ConstructProduct,
            Self::ProjectField { .. } => OperationCode::ProjectField,
            Self::ConstructVariant { .. } => OperationCode::ConstructVariant,
            Self::MatchSum { .. } => OperationCode::MatchSum,
        }
    }

    pub const fn stable_tag(&self) -> u8 {
        self.code().stable_tag()
    }
    pub const fn descriptor(&self) -> &'static OperationDescriptor {
        self.code().descriptor()
    }

    pub fn operand_count(&self) -> usize {
        match self {
            Self::Call { arguments, .. } => arguments.len(),
            Self::ConstructProduct { fields, .. } => fields.len(),
            Self::ConstructVariant { payload, .. } => usize::from(payload.is_some()),
            _ => match self.descriptor().operand_arity {
                OperandArity::Fixed(count) => usize::from(count),
                OperandArity::CallTargetParameters
                | OperandArity::ProductFields
                | OperandArity::VariantPayload => 0,
            },
        }
    }

    pub fn operand(&self, index: usize) -> Option<ValueRef> {
        match (self, index) {
            (
                Self::AddI64 { lhs, .. }
                | Self::LtI64 { lhs, .. }
                | Self::BytesEqual { lhs, .. }
                | Self::BytesConcat { lhs, .. },
                0,
            ) => Some(*lhs),
            (
                Self::AddI64 { rhs, .. }
                | Self::LtI64 { rhs, .. }
                | Self::BytesEqual { rhs, .. }
                | Self::BytesConcat { rhs, .. },
                1,
            ) => Some(*rhs),
            (Self::BytesLen { value }, 0) => Some(*value),
            (Self::BytesAt { value, .. }, 0) => Some(*value),
            (Self::BytesAt { index, .. }, 1) => Some(*index),
            (Self::BytesSlice { value, .. }, 0) => Some(*value),
            (Self::BytesSlice { start, .. }, 1) => Some(*start),
            (Self::BytesSlice { length, .. }, 2) => Some(*length),
            (Self::Call { arguments, .. }, index) => arguments.get(index).copied(),
            (Self::If { condition, .. }, 0) => Some(*condition),
            (Self::ForI64 { start, .. }, 0) => Some(*start),
            (Self::ForI64 { end_exclusive, .. }, 1) => Some(*end_exclusive),
            (Self::ForI64 { initial, .. }, 2) => Some(*initial),
            (Self::Return { value } | Self::Yield { value }, 0) => Some(*value),
            (Self::ConstructProduct { fields, .. }, index) => {
                fields.get(index).map(|field| field.value)
            }
            (Self::ProjectField { value, .. }, 0) => Some(*value),
            (Self::ConstructVariant { payload, .. }, 0) => *payload,
            (Self::MatchSum { scrutinee, .. }, 0) => Some(*scrutinee),
            _ => None,
        }
    }

    fn operand_descriptor(&self, index: usize) -> Option<&'static OperandDescriptor> {
        if index >= self.operand_count() {
            return None;
        }
        match self.descriptor().operand_arity {
            OperandArity::Fixed(_) => self.descriptor().operands.get(index),
            OperandArity::CallTargetParameters
            | OperandArity::ProductFields
            | OperandArity::VariantPayload => self.descriptor().operands.first(),
        }
    }

    pub fn operand_type(
        &self,
        index: usize,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        self.resolve_type_rule(self.operand_descriptor(index)?.ty, owner_function_result)
    }

    pub fn operand_use(&self, index: usize) -> Option<OperandUse> {
        self.operand_descriptor(index)
            .map(|operand| operand.use_mode)
    }

    pub fn result_count(&self) -> usize {
        self.descriptor().results.len()
    }

    /// Resolves node-local result rules. Call result types require a snapshot-aware helper.
    pub fn result_type(
        &self,
        index: usize,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        let rule = *self.descriptor().results.get(index)?;
        self.resolve_type_rule(rule, owner_function_result)
    }

    pub const fn is_terminator(&self) -> bool {
        self.descriptor().terminator
    }
    pub const fn is_complete(&self) -> bool {
        self.descriptor().complete
    }

    pub fn replace_operand(&mut self, index: u64, replacement: ValueRef) -> bool {
        let Ok(index) = usize::try_from(index) else {
            return false;
        };
        match (self, index) {
            (
                Self::AddI64 { lhs, .. }
                | Self::LtI64 { lhs, .. }
                | Self::BytesEqual { lhs, .. }
                | Self::BytesConcat { lhs, .. },
                0,
            ) => *lhs = replacement,
            (
                Self::AddI64 { rhs, .. }
                | Self::LtI64 { rhs, .. }
                | Self::BytesEqual { rhs, .. }
                | Self::BytesConcat { rhs, .. },
                1,
            ) => *rhs = replacement,
            (Self::BytesLen { value }, 0) => *value = replacement,
            (Self::BytesAt { value, .. }, 0) => *value = replacement,
            (Self::BytesAt { index, .. }, 1) => *index = replacement,
            (Self::BytesSlice { value, .. }, 0) => *value = replacement,
            (Self::BytesSlice { start, .. }, 1) => *start = replacement,
            (Self::BytesSlice { length, .. }, 2) => *length = replacement,
            (Self::Call { arguments, .. }, index) if index < arguments.len() => {
                arguments[index] = replacement
            }
            (Self::If { condition, .. }, 0) => *condition = replacement,
            (Self::ForI64 { start, .. }, 0) => *start = replacement,
            (Self::ForI64 { end_exclusive, .. }, 1) => *end_exclusive = replacement,
            (Self::ForI64 { initial, .. }, 2) => *initial = replacement,
            (Self::Return { value } | Self::Yield { value }, 0) => *value = replacement,
            (Self::ConstructProduct { fields, .. }, index) if index < fields.len() => {
                fields[index].value = replacement
            }
            (Self::ProjectField { value, .. }, 0) => *value = replacement,
            (
                Self::ConstructVariant {
                    payload: Some(value),
                    ..
                },
                0,
            ) => *value = replacement,
            (Self::MatchSum { scrutinee, .. }, 0) => *scrutinee = replacement,
            _ => return false,
        }
        true
    }

    pub fn definition_target_count(&self) -> usize {
        match self {
            Self::Call { .. } | Self::ProjectField { .. } | Self::ConstructVariant { .. } => 1,
            Self::ConstructProduct { fields, .. } => 1 + fields.len(),
            Self::MatchSum { arms, .. } => arms.len(),
            _ => 0,
        }
    }

    pub fn definition_target(&self, index: usize) -> Option<NodeId> {
        match (self, index) {
            (Self::Call { function, .. }, 0) => Some(*function),
            (Self::ConstructProduct { product, .. }, 0) => Some(*product),
            (Self::ConstructProduct { fields, .. }, index) => {
                fields.get(index - 1).map(|f| f.field)
            }
            (Self::ProjectField { field, .. }, 0) => Some(*field),
            (Self::ConstructVariant { variant, .. }, 0) => Some(*variant),
            (Self::MatchSum { arms, .. }, index) => arms.get(index).map(|arm| arm.variant),
            _ => None,
        }
    }

    pub fn nominal_type_target_count(&self) -> usize {
        usize::from(self.nominal_type_target(0).is_some())
    }

    pub const fn nominal_type_target(&self, index: usize) -> Option<NodeId> {
        if index != 0 {
            return None;
        }
        match self {
            Self::Hole { expected }
            | Self::If {
                result: expected, ..
            }
            | Self::ForI64 {
                carried: expected, ..
            }
            | Self::MatchSum {
                result: expected, ..
            } => expected.nominal_target(),
            _ => None,
        }
    }

    pub fn owned_region_count(&self) -> usize {
        match self {
            Self::If { .. } => 2,
            Self::ForI64 { .. } => 1,
            Self::MatchSum { arms, .. } => arms.len(),
            _ => 0,
        }
    }

    pub fn owned_region(&self, index: usize) -> Option<NodeId> {
        match (self, index) {
            (Self::If { then_region, .. }, 0) => Some(*then_region),
            (Self::If { else_region, .. }, 1) => Some(*else_region),
            (Self::ForI64 { body_region, .. }, 0) => Some(*body_region),
            (Self::MatchSum { arms, .. }, index) => arms.get(index).map(|arm| arm.region),
            _ => None,
        }
    }

    pub fn region_role(&self, region: NodeId) -> Option<RegionRole> {
        match self {
            Self::MatchSum { arms, .. } => arms
                .iter()
                .find(|arm| arm.region == region)
                .map(|arm| RegionRole::MatchArm(arm.variant)),
            _ => (0..self.owned_region_count()).find_map(|index| {
                (self.owned_region(index) == Some(region))
                    .then_some(self.descriptor().regions[index].role)
            }),
        }
    }

    const fn resolve_type_rule(
        &self,
        rule: TypeRule,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        match rule {
            TypeRule::Fixed(ty) => Some(ty),
            TypeRule::PayloadExpected => match self {
                Self::Hole { expected }
                | Self::MatchSum {
                    result: expected, ..
                } => Some(*expected),
                _ => None,
            },
            TypeRule::OwnerFunctionResult => owner_function_result,
            TypeRule::PayloadResult => match self {
                Self::If { result, .. } => Some(*result),
                _ => None,
            },
            TypeRule::PayloadCarried => match self {
                Self::ForI64 { carried, .. } => Some(*carried),
                _ => None,
            },
            TypeRule::ProductDeclarationResult => match self {
                Self::ConstructProduct { product, .. } => Some(SemanticType::Nominal(*product)),
                _ => None,
            },
            TypeRule::MatchResult => match self {
                Self::MatchSum { result, .. } => Some(*result),
                _ => None,
            },
            TypeRule::CallTargetParameter
            | TypeRule::CallTargetResult
            | TypeRule::OwningRegionYield
            | TypeRule::ProductFieldType
            | TypeRule::ProjectionOwner
            | TypeRule::ProjectedFieldResult
            | TypeRule::VariantPayload
            | TypeRule::VariantOwnerResult
            | TypeRule::MatchScrutinee => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeReferenceSlot {
    FunctionResult,
    ParameterType,
    ProductFieldType,
    SumVariantPayload,
    BlockArgumentType,
    OperationType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DirectReference {
    Definition {
        target: NodeId,
    },
    Type {
        slot: TypeReferenceSlot,
        target: NodeId,
    },
    ValueOperand {
        index: u64,
        value: ValueRef,
    },
}

impl DirectReference {
    pub const fn target(self) -> NodeId {
        match self {
            Self::Definition { target } | Self::Type { target, .. } => target,
            Self::ValueOperand { value, .. } => value.referenced_node(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Node {
    WorkspaceRoot {
        packages: Vec<NodeId>,
    },
    Package {
        owner: NodeId,
        name: String,
        modules: Vec<NodeId>,
        entry: Option<NodeId>,
    },
    Module {
        owner: NodeId,
        name: String,
        types: Vec<NodeId>,
        functions: Vec<NodeId>,
    },
    ProductType {
        owner: NodeId,
        name: String,
        fields: Vec<NodeId>,
    },
    ProductField {
        owner: NodeId,
        ordinal: u32,
        name: String,
        ty: SemanticType,
    },
    SumType {
        owner: NodeId,
        name: String,
        variants: Vec<NodeId>,
    },
    SumVariant {
        owner: NodeId,
        ordinal: u32,
        name: String,
        payload: Option<SemanticType>,
    },
    Function {
        owner: NodeId,
        name: String,
        parameters: Vec<NodeId>,
        result: SemanticType,
        body: Option<NodeId>,
    },
    Parameter {
        owner: NodeId,
        ordinal: u32,
        name: String,
        ty: SemanticType,
    },
    Region {
        owner: NodeId,
        blocks: Vec<NodeId>,
    },
    Block {
        owner: NodeId,
        arguments: Vec<NodeId>,
        operations: Vec<NodeId>,
        terminator: Option<NodeId>,
    },
    BlockArgument {
        owner: NodeId,
        ordinal: u32,
        ty: SemanticType,
    },
    Operation {
        owner: NodeId,
        operation: OperationKind,
    },
}

impl NameUniquenessGroup {
    pub fn children(self, owner: &Node) -> Option<&[NodeId]> {
        match (self, owner) {
            (Self::WorkspacePackages, Node::WorkspaceRoot { packages }) => Some(packages),
            (Self::PackageModules, Node::Package { modules, .. }) => Some(modules),
            (Self::ModuleTypes, Node::Module { types, .. }) => Some(types),
            (Self::ModuleFunctions, Node::Module { functions, .. }) => Some(functions),
            (Self::ProductFields, Node::ProductType { fields, .. }) => Some(fields),
            (Self::SumVariants, Node::SumType { variants, .. }) => Some(variants),
            (Self::FunctionParameters, Node::Function { parameters, .. }) => Some(parameters),
            _ => None,
        }
    }
}

impl Node {
    pub const fn kind(&self) -> NodeKind {
        match self {
            Self::WorkspaceRoot { .. } => NodeKind::WorkspaceRoot,
            Self::Package { .. } => NodeKind::Package,
            Self::Module { .. } => NodeKind::Module,
            Self::ProductType { .. } => NodeKind::ProductType,
            Self::ProductField { .. } => NodeKind::ProductField,
            Self::SumType { .. } => NodeKind::SumType,
            Self::SumVariant { .. } => NodeKind::SumVariant,
            Self::Function { .. } => NodeKind::Function,
            Self::Parameter { .. } => NodeKind::Parameter,
            Self::Region { .. } => NodeKind::Region,
            Self::Block { .. } => NodeKind::Block,
            Self::BlockArgument { .. } => NodeKind::BlockArgument,
            Self::Operation { .. } => NodeKind::Operation,
        }
    }

    pub const fn owner(&self) -> Option<NodeId> {
        match self {
            Self::WorkspaceRoot { .. } => None,
            Self::Package { owner, .. }
            | Self::Module { owner, .. }
            | Self::ProductType { owner, .. }
            | Self::ProductField { owner, .. }
            | Self::SumType { owner, .. }
            | Self::SumVariant { owner, .. }
            | Self::Function { owner, .. }
            | Self::Parameter { owner, .. }
            | Self::Region { owner, .. }
            | Self::Block { owner, .. }
            | Self::BlockArgument { owner, .. }
            | Self::Operation { owner, .. } => Some(*owner),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Package { name, .. }
            | Self::Module { name, .. }
            | Self::ProductType { name, .. }
            | Self::ProductField { name, .. }
            | Self::SumType { name, .. }
            | Self::SumVariant { name, .. }
            | Self::Function { name, .. }
            | Self::Parameter { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn set_name(&mut self, replacement: String) -> bool {
        match self {
            Self::Package { name, .. }
            | Self::Module { name, .. }
            | Self::ProductType { name, .. }
            | Self::ProductField { name, .. }
            | Self::SumType { name, .. }
            | Self::SumVariant { name, .. }
            | Self::Function { name, .. }
            | Self::Parameter { name, .. } => {
                *name = replacement;
                true
            }
            _ => false,
        }
    }

    pub fn owned_child_count(&self) -> usize {
        match self {
            Self::WorkspaceRoot { packages } => packages.len(),
            Self::Package { modules, .. } => modules.len(),
            Self::Module {
                types, functions, ..
            } => types.len() + functions.len(),
            Self::ProductType { fields, .. } => fields.len(),
            Self::SumType { variants, .. } => variants.len(),
            Self::ProductField { .. } | Self::SumVariant { .. } => 0,
            Self::Function {
                parameters, body, ..
            } => parameters.len() + usize::from(body.is_some()),
            Self::Parameter { .. } | Self::BlockArgument { .. } => 0,
            Self::Region { blocks, .. } => blocks.len(),
            Self::Block {
                arguments,
                operations,
                terminator,
                ..
            } => arguments.len() + operations.len() + usize::from(terminator.is_some()),
            Self::Operation { operation, .. } => operation.owned_region_count(),
        }
    }

    pub fn owned_child(&self, index: usize) -> Option<NodeId> {
        match self {
            Self::WorkspaceRoot { packages } => packages.get(index).copied(),
            Self::Package { modules, .. } => modules.get(index).copied(),
            Self::Module {
                types, functions, ..
            } => types
                .get(index)
                .copied()
                .or_else(|| functions.get(index.saturating_sub(types.len())).copied()),
            Self::ProductType { fields, .. } => fields.get(index).copied(),
            Self::SumType { variants, .. } => variants.get(index).copied(),
            Self::ProductField { .. } | Self::SumVariant { .. } => None,
            Self::Function {
                parameters, body, ..
            } => parameters
                .get(index)
                .copied()
                .or_else(|| (index == parameters.len()).then_some(*body).flatten()),
            Self::Parameter { .. } | Self::BlockArgument { .. } => None,
            Self::Region { blocks, .. } => blocks.get(index).copied(),
            Self::Block {
                arguments,
                operations,
                terminator,
                ..
            } => arguments
                .get(index)
                .copied()
                .or_else(|| {
                    operations
                        .get(index.saturating_sub(arguments.len()))
                        .copied()
                })
                .or_else(|| {
                    (index == arguments.len() + operations.len())
                        .then_some(*terminator)
                        .flatten()
                }),
            Self::Operation { operation, .. } => operation.owned_region(index),
        }
    }

    pub fn direct_reference_count(&self) -> usize {
        match self {
            Self::Package { entry, .. } => usize::from(entry.is_some()),
            Self::Function { result, .. }
            | Self::Parameter { ty: result, .. }
            | Self::ProductField { ty: result, .. }
            | Self::BlockArgument { ty: result, .. } => {
                usize::from(result.nominal_target().is_some())
            }
            Self::SumVariant { payload, .. } => {
                usize::from(payload.and_then(SemanticType::nominal_target).is_some())
            }
            Self::Operation { operation, .. } => {
                operation.operand_count()
                    + operation.definition_target_count()
                    + operation.nominal_type_target_count()
            }
            _ => 0,
        }
    }

    pub fn direct_reference(&self, index: usize) -> Option<DirectReference> {
        let type_reference = |slot, ty: SemanticType| {
            ty.nominal_target()
                .map(|target| DirectReference::Type { slot, target })
        };
        match self {
            Self::Package { entry, .. } if index == 0 => {
                entry.map(|target| DirectReference::Definition { target })
            }
            Self::Function { result, .. } if index == 0 => {
                type_reference(TypeReferenceSlot::FunctionResult, *result)
            }
            Self::Parameter { ty, .. } if index == 0 => {
                type_reference(TypeReferenceSlot::ParameterType, *ty)
            }
            Self::ProductField { ty, .. } if index == 0 => {
                type_reference(TypeReferenceSlot::ProductFieldType, *ty)
            }
            Self::SumVariant { payload, .. } if index == 0 => {
                payload.and_then(|ty| type_reference(TypeReferenceSlot::SumVariantPayload, ty))
            }
            Self::BlockArgument { ty, .. } if index == 0 => {
                type_reference(TypeReferenceSlot::BlockArgumentType, *ty)
            }
            Self::Operation { operation, .. } => {
                let mut current = index;
                if current < operation.definition_target_count() {
                    return operation
                        .definition_target(current)
                        .map(|target| DirectReference::Definition { target });
                }
                current -= operation.definition_target_count();
                if current < operation.nominal_type_target_count() {
                    return operation.nominal_type_target(current).map(|target| {
                        DirectReference::Type {
                            slot: TypeReferenceSlot::OperationType,
                            target,
                        }
                    });
                }
                current -= operation.nominal_type_target_count();
                operation.operand(current).and_then(|value| {
                    u64::try_from(current)
                        .ok()
                        .map(|index| DirectReference::ValueOperand { index, value })
                })
            }
            _ => None,
        }
    }
}

pub const fn expected_owner_kind(kind: NodeKind) -> Option<NodeKind> {
    match kind {
        NodeKind::WorkspaceRoot => None,
        NodeKind::Package => Some(NodeKind::WorkspaceRoot),
        NodeKind::Module => Some(NodeKind::Package),
        NodeKind::ProductType | NodeKind::SumType | NodeKind::Function => Some(NodeKind::Module),
        NodeKind::ProductField => Some(NodeKind::ProductType),
        NodeKind::SumVariant => Some(NodeKind::SumType),
        NodeKind::Parameter => Some(NodeKind::Function),
        NodeKind::Region => None,
        NodeKind::Block => Some(NodeKind::Region),
        NodeKind::BlockArgument => Some(NodeKind::Block),
        NodeKind::Operation => Some(NodeKind::Block),
    }
}

pub fn owner_kind_is_valid(child: NodeKind, owner: NodeKind) -> bool {
    match child {
        NodeKind::Region => matches!(owner, NodeKind::Function | NodeKind::Operation),
        _ => expected_owner_kind(child) == Some(owner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use std::collections::BTreeSet;

    fn decode_bytes(encoded: &str) -> Result<ByteString, serde_json::Error> {
        serde_json::from_value(serde_json::Value::String(encoded.to_owned()))
    }

    #[test]
    fn canonical_byte_strings_are_unique_strict_and_exact_at_the_limit() {
        for (bytes, encoded) in [
            (&b""[..], ""),
            (&b"\xff"[..], "_w"),
            (&b"\xff\xee"[..], "_-4"),
            (&b"\xff\xee\xdd"[..], "_-7d"),
        ] {
            let value = decode_bytes(encoded).expect("canonical bytes");
            assert_eq!(value.as_slice(), bytes);
            assert_eq!(serde_json::to_value(&value).unwrap(), encoded);
        }
        for malformed in ["A", "/w", "_w=", "_w==", "_ w", "_w\n", "_x"] {
            assert!(decode_bytes(malformed).is_err(), "accepted {malformed:?}");
        }

        let maximum = ByteString::new(vec![0xa5; MAXIMUM_BYTE_STRING_BYTES]).unwrap();
        let encoded = URL_SAFE_NO_PAD.encode(maximum.as_slice());
        assert_eq!(encoded.len(), MAXIMUM_BYTE_STRING_ENCODED_BYTES);
        assert_eq!(decode_bytes(&encoded).unwrap(), maximum);

        let decoded_too_large = URL_SAFE_NO_PAD.encode(vec![0; MAXIMUM_BYTE_STRING_BYTES + 1]);
        assert!(decode_bytes(&decoded_too_large).is_err());
        assert!(decode_bytes(&"A".repeat(MAXIMUM_BYTE_STRING_ENCODED_BYTES + 1)).is_err());
    }

    #[test]
    fn primitive_descriptors_own_stable_tags_storage_and_layout_facts() {
        assert_eq!(SemanticType::PRIMITIVES.len(), 4);
        assert_eq!(SemanticType::PRIMITIVE_DESCRIPTORS.len(), 4);
        let mut tags = BTreeSet::new();
        for (ty, descriptor) in SemanticType::PRIMITIVES
            .into_iter()
            .zip(SemanticType::PRIMITIVE_DESCRIPTORS)
        {
            assert_eq!(descriptor.ty, ty);
            assert!(tags.insert(descriptor.stable_tag));
            assert_eq!(ty.primitive_descriptor(), Some(&descriptor));
        }
        let bytes = SemanticType::Bytes.primitive_descriptor().unwrap();
        assert_eq!(bytes.machine_name, "bytes");
        assert_eq!(bytes.storage_class, ValueStorageClass::ManagedHandle);
        assert_eq!(
            (
                bytes.physical_slot_size,
                bytes.physical_slot_align,
                bytes.cells
            ),
            (8, 8, 1)
        );
        assert!(
            SemanticType::Nominal(
                NodeId::new(crate::ids::WorkspaceId::from_bytes([1; 16]), 1).unwrap()
            )
            .primitive_descriptor()
            .is_none()
        );
    }

    #[test]
    fn operation_descriptors_are_unique_and_structured_contracts_are_exact() {
        let mut tags = BTreeSet::new();
        let mut names = BTreeSet::new();
        for code in OperationCode::ALL {
            let descriptor = code.descriptor();
            assert_eq!(descriptor.code, code);
            assert!(tags.insert(descriptor.stable_tag));
            assert!(names.insert(descriptor.machine_name));
            assert_eq!(
                OperationCode::from_stable_tag(code.stable_tag()),
                Some(code)
            );
        }
        assert_eq!(
            OperationCode::Call.descriptor().operand_arity,
            OperandArity::CallTargetParameters
        );
        assert_eq!(
            OperationCode::If
                .descriptor()
                .regions
                .iter()
                .map(|r| r.role)
                .collect::<Vec<_>>(),
            [RegionRole::IfThen, RegionRole::IfElse]
        );
        assert_eq!(
            OperationCode::ForI64.descriptor().regions[0].block_arguments,
            FOR_BLOCK_ARGUMENTS
        );
        assert_eq!(
            OperationCode::ForI64.descriptor().regions[0].terminator,
            OperationCode::Yield
        );
        assert!(!OperationCode::Hole.descriptor().complete);
        assert!(OperationCode::Return.descriptor().terminator);
        assert!(OperationCode::Yield.descriptor().terminator);
    }
}
