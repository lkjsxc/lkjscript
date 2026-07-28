use serde::{Deserialize, Serialize};

use super::{ClosedBuiltinOperation, SpanRecord};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SemanticNodeKind {
    Import,
    ImportModule,
    ImportDeclarations,
    ImportDeclaration,
    Main,
    FunctionDeclaration,
    Function,
    Signature,
    SignatureInputs,
    SignatureOutput,
    Parameters,
    TypeVariables,
    Bounds,
    Bound,
    Product,
    ProductField,
    EnumDeclaration,
    EnumVariant,
    EnumVariantField,
    MarkerTrait,
    TraitImplementation,
    ContextName,
    ContextType,
    ContextFields,
    ContextVariants,
    ContextVariant,
    ContextTrait,
    ContextFor,
    UnitLiteral,
    BoolLiteral,
    I64Literal,
    F64Literal,
    StringLiteral,
    BytesLiteral,
    NameReference,
    TypedHole,
    HoleIdentity,
    HoleGoal,
    ParameterName,
    BindingName,
    MutableName,
    FieldName,
    ProductName,
    VariantName,
    TraitName,
    QuotedName,
    PlaceName,
    TypeNever,
    TypeUnit,
    TypeBool,
    TypeI64,
    TypeF64,
    TypeString,
    TypeBuffer,
    TypeBytes,
    TypeByteVector,
    TypeByteSlice,
    TypeByteSliceMut,
    TypePath,
    TypeCapability,
    CapabilityKind,
    TypeSymbol,
    TypeResource,
    TypeProduct,
    TypeEnum,
    TypeList,
    TypeOption,
    TypeResult,
    TypeVariable,
    Let,
    Bind,
    Var,
    Set,
    If,
    While,
    Loop,
    Return,
    Break,
    Continue,
    Trap,
    Exit,
    Do,
    Quote,
    ProductValue,
    ProductValueField,
    VariantValue,
    VariantValueField,
    Match,
    MatchArms,
    MatchArm,
    WildcardPattern,
    BindingPattern,
    BoolPattern,
    I64Pattern,
    VariantPattern,
    VariantFieldPattern,
    ProductPattern,
    ProductFieldPattern,
    FieldAccess,
    WithField,
    EmptyList,
    None,
    Move,
    Borrow,
    BorrowMut,
    BuiltinCall,
    UserFunctionCall,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum SemanticNodeValue {
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    F64 {
        canonical: String,
    },
    Text {
        value: String,
    },
    Bytes {
        hexadecimal: String,
    },
    SourceName {
        name: String,
    },
    ImportPath {
        path: String,
    },
    BuiltinOperation {
        operation: ClosedBuiltinOperation,
    },
    UserFunction {
        name: String,
    },
    TypedHole {
        identity: String,
        goal: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TriviaAttachment {
    Leading,
    BeforeClose,
    SourceUnitTrailing,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TriviaRecord {
    pub attachment: TriviaAttachment,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeRecord {
    pub index: u32,
    pub kind: SemanticNodeKind,
    pub value: Option<SemanticNodeValue>,
    pub source: String,
    pub span: SpanRecord,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub declaration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_identity: Option<String>,
    pub fingerprint: String,
    pub trivia: Vec<TriviaRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SemanticSubtreeRecord {
    pub node: NodeRecord,
    pub children: Vec<SemanticSubtreeRecord>,
}
