//! Flat syntax trees. Edges are arena indices, so dropping a deeply nested source is iterative.

pub type TypeId = usize;
pub type ExprId = usize;
pub type BlockId = usize;
pub type PatternId = usize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, code: &str, message: &str) -> Diagnostic {
        Diagnostic {
            line: span.line,
            column: span.column,
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub name: String,
    pub args: Vec<TypeId>,
}

#[derive(Clone, Debug)]
pub enum Type {
    Infer,
    Path(Path),
    Ref {
        mutable: bool,
        lifetime: String,
        inner: TypeId,
    },
    Slice(TypeId),
    Array {
        inner: TypeId,
        len: String,
    },
    Tuple(Vec<TypeId>),
    Function {
        params: Vec<TypeId>,
        result: TypeId,
    },
    Never,
}

#[derive(Clone, Debug)]
pub struct TypeNode {
    pub kind: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<TypeId>,
    pub lifetime: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeId,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub result: TypeId,
    pub body: Option<BlockId>,
    pub attrs: Vec<Attribute>,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<Field>,
    pub tuple: bool,
    pub attrs: Vec<Attribute>,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub named: bool,
    pub discriminant: Option<ExprId>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<Variant>,
    pub attrs: Vec<Attribute>,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AssociatedType {
    pub name: String,
    pub bounds: Vec<TypeId>,
    pub value: Option<TypeId>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Trait {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub bounds: Vec<TypeId>,
    pub methods: Vec<usize>,
    pub associated: Vec<AssociatedType>,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Impl {
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub target: TypeId,
    pub trait_ty: Option<TypeId>,
    pub methods: Vec<usize>,
    pub associated: Vec<AssociatedType>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Use {
    pub module: String,
    pub path: String,
    pub alias: String,
    pub glob: bool,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub parent: String,
    pub items: Vec<Item>,
    pub external: bool,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Alias {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub ty: TypeId,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Constant {
    pub name: String,
    pub module: String,
    pub ty: TypeId,
    pub value: ExprId,
    pub public: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Item {
    Function(usize),
    Struct(usize),
    Enum(usize),
    Trait(usize),
    Impl(usize),
    Use(usize),
    Module(usize),
    Alias(usize),
    Constant(usize),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Int(String),
    Float(String),
    Bool(bool),
    String(String),
    Char(char),
    Bytes(Vec<u8>),
    Path(Path),
    Tuple(Vec<ExprId>),
    Array(Vec<ExprId>),
    Repeat {
        value: ExprId,
        count: ExprId,
    },
    Unary {
        op: String,
        value: ExprId,
    },
    Binary {
        op: String,
        left: ExprId,
        right: ExprId,
    },
    Assign {
        op: String,
        place: ExprId,
        value: ExprId,
    },
    Borrow {
        mutable: bool,
        value: ExprId,
    },
    Cast {
        value: ExprId,
        ty: TypeId,
    },
    Call {
        callee: ExprId,
        args: Vec<ExprId>,
    },
    Method {
        receiver: ExprId,
        name: String,
        types: Vec<TypeId>,
        args: Vec<ExprId>,
    },
    Field {
        value: ExprId,
        name: String,
    },
    Index {
        value: ExprId,
        index: ExprId,
    },
    Struct {
        path: Path,
        fields: Vec<(String, ExprId)>,
        base: Option<ExprId>,
    },
    Block(BlockId),
    If {
        condition: ExprId,
        then_block: BlockId,
        otherwise: Option<ExprId>,
    },
    IfLet {
        pattern: PatternId,
        value: ExprId,
        then_block: BlockId,
        otherwise: Option<ExprId>,
    },
    While {
        condition: ExprId,
        body: BlockId,
    },
    WhileLet {
        pattern: PatternId,
        value: ExprId,
        body: BlockId,
    },
    Loop {
        body: BlockId,
    },
    For {
        pattern: PatternId,
        iterator: ExprId,
        body: BlockId,
    },
    Match {
        value: ExprId,
        arms: Vec<Arm>,
    },
    Closure {
        moved: bool,
        params: Vec<Param>,
        result: Option<TypeId>,
        body: ExprId,
    },
    Return(Option<ExprId>),
    Break(Option<ExprId>),
    Continue,
    Macro {
        name: String,
        args: Vec<ExprId>,
        separator: String,
    },
    Try(ExprId),
    Range {
        start: Option<ExprId>,
        end: Option<ExprId>,
        inclusive: bool,
    },
}

#[derive(Clone, Debug)]
pub struct ExprNode {
    pub kind: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pattern: PatternId,
    pub guard: Option<ExprId>,
    pub value: ExprId,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Wild,
    Bind {
        name: String,
        mutable: bool,
        by_ref: bool,
        ref_mut: bool,
    },
    Path(Path),
    Literal(ExprId),
    Tuple(Vec<PatternId>),
    Variant {
        path: Path,
        fields: Vec<PatternId>,
    },
    Struct {
        path: Path,
        fields: Vec<(String, PatternId)>,
        rest: bool,
    },
    Ref {
        mutable: bool,
        inner: PatternId,
    },
    Or(Vec<PatternId>),
}

#[derive(Clone, Debug)]
pub struct PatternNode {
    pub kind: Pattern,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        pattern: PatternId,
        ty: Option<TypeId>,
        value: Option<ExprId>,
        span: Span,
    },
    Expr(ExprId),
    Item(Item),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub tail: Option<ExprId>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub types: Vec<TypeNode>,
    pub exprs: Vec<ExprNode>,
    pub patterns: Vec<PatternNode>,
    pub blocks: Vec<Block>,
    pub functions: Vec<Function>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
    pub uses: Vec<Use>,
    pub modules: Vec<Module>,
    pub aliases: Vec<Alias>,
    pub constants: Vec<Constant>,
    pub items: Vec<Item>,
    pub attrs: Vec<Attribute>,
}
