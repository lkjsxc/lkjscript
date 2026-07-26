use serde::{Deserialize, Serialize};

use super::{ClosedBuiltinOperation, SpanRecord};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticNodeKind {
    EditionMarker,
    EditionNumber,
    Import,
    Main,
    FunctionDeclaration,
    Function,
    Signature,
    Parameters,
    TypeVariables,
    Bounds,
    Bound,
    Product,
    ProductField,
    MarkerTrait,
    TraitImplementation,
    ContextName,
    ContextType,
    ContextFields,
    ContextTrait,
    ContextFor,
    UnitLiteral,
    BoolLiteral,
    I64Literal,
    F64Literal,
    StringLiteral,
    NameReference,
    TypedHole,
    HoleIdentity,
    HoleGoal,
    ParameterName,
    BindingName,
    MutableName,
    FieldName,
    ProductName,
    TraitName,
    QuotedName,
    PlaceName,
    TypeUnit,
    TypeBool,
    TypeI64,
    TypeF64,
    TypeString,
    TypeBuffer,
    TypeSymbol,
    TypeHandle,
    TypeOwned,
    TypeRef,
    TypeRefMut,
    TypeProduct,
    TypeList,
    TypeOption,
    TypeResult,
    TypeVariable,
    ReturnArrow,
    Let,
    Bind,
    Var,
    Set,
    If,
    While,
    Do,
    Quote,
    ProductValue,
    ProductValueField,
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
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
    EditionIdentity {
        edition: u32,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub fingerprint: String,
    pub trivia: Vec<TriviaRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SemanticSubtreeRecord {
    pub node: NodeRecord,
    pub children: Vec<SemanticSubtreeRecord>,
}
