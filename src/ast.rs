//! Flat syntax trees. Edges are arena indices, so dropping a deeply nested source is iterative.

pub type FileId = usize;
pub type ModuleId = usize;
pub type DeclarationId = usize;
pub type TypeId = usize;
pub type ExprId = usize;
pub type BlockId = usize;
pub type PatternId = usize;

/// Source function identities are distinct from implementation and trait owners.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FunctionId {
    pub index: usize,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImplId {
    pub index: usize,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraitId {
    pub index: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FunctionOwner {
    Free,
    Implementation(ImplId),
    Trait(TraitId),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub file: FileId,
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, code: &str, message: &str) -> Diagnostic {
        Diagnostic {
            path: String::new(),
            file: span.file,
            line: span.line,
            column: span.column,
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Path {
    pub declaration: Option<DeclarationId>,
    pub name: String,
    pub args: Vec<TypeId>,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct TypeNode {
    pub scope: ModuleId,
    pub kind: Type,
    pub span: Span,
}

#[derive(Clone)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<TypeId>,
    pub lifetime: bool,
    pub span: Span,
}

#[derive(Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeId,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub result: TypeId,
    pub body: Option<BlockId>,
    pub attrs: Vec<Attribute>,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct Struct {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<Field>,
    pub tuple: bool,
    pub unit: bool,
    pub attrs: Vec<Attribute>,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VariantStyle {
    Unit,
    Tuple,
    Named,
}

#[derive(Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub style: VariantStyle,
    pub discriminant: Option<ExprId>,
    pub span: Span,
}

#[derive(Clone)]
pub struct Enum {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<Variant>,
    pub attrs: Vec<Attribute>,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct AssociatedType {
    pub name: String,
    pub bounds: Vec<TypeId>,
    pub value: Option<TypeId>,
    pub span: Span,
}

#[derive(Clone)]
pub struct Trait {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub bounds: Vec<TypeId>,
    pub methods: Vec<usize>,
    pub associated: Vec<AssociatedType>,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct Impl {
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub target: TypeId,
    pub trait_ty: Option<TypeId>,
    pub methods: Vec<usize>,
    pub associated: Vec<AssociatedType>,
    pub span: Span,
}

#[derive(Clone)]
pub struct Use {
    pub module: String,
    pub path: String,
    pub alias: String,
    pub glob: bool,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct Module {
    pub scope: ModuleId,
    pub name: String,
    pub parent: String,
    pub items: Vec<Item>,
    pub external: bool,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct Alias {
    pub name: String,
    pub module: String,
    pub generics: Vec<GenericParam>,
    pub ty: TypeId,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone)]
pub struct Constant {
    pub name: String,
    pub module: String,
    pub ty: TypeId,
    pub value: ExprId,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone)]
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
        patterns: Vec<PatternId>,
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
    /// A source obligation introduced by let-else expansion.
    Diverge(ExprId),
    Range {
        start: Option<ExprId>,
        end: Option<ExprId>,
        inclusive: bool,
    },
}

#[derive(Clone)]
pub struct ExprNode {
    pub scope: ModuleId,
    pub kind: Expr,
    pub span: Span,
}

#[derive(Clone)]
pub struct Arm {
    pub pattern: PatternId,
    pub guard: Option<ExprId>,
    pub value: ExprId,
    pub span: Span,
}

#[derive(Clone)]
pub enum Pattern {
    Wild,
    /// An omitted tuple/tuple-variant field span, expanded after arity is known.
    Rest,
    Bind {
        name: String,
        mutable: bool,
        by_ref: bool,
        ref_mut: bool,
    },
    Path(Path),
    Literal(ExprId),
    Range {
        start: Option<ExprId>,
        end: Option<ExprId>,
        inclusive: bool,
    },
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

#[derive(Clone)]
pub struct PatternNode {
    pub scope: ModuleId,
    pub kind: Pattern,
    pub span: Span,
}

#[derive(Clone)]
pub enum Stmt {
    Let {
        pattern: PatternId,
        ty: Option<TypeId>,
        value: Option<ExprId>,
        span: Span,
    },
    Expr {
        value: ExprId,
        semicolon: bool,
    },
    Item(Item),
}

#[derive(Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub tail: Option<ExprId>,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Parent,
    Crate,
    /// An explicit restriction to an ancestor module in this program.
    Restricted(ModuleId),
    Public,
}

#[derive(Clone)]
pub struct SourceFile {
    pub path: String,
    pub crate_root: ModuleId,
}
#[derive(Clone)]
pub struct ModuleScope {
    pub path: String,
    pub parent: Option<ModuleId>,
    pub crate_root: ModuleId,
}
#[derive(Clone)]
pub struct Declaration {
    pub name: String,
    pub scope: ModuleId,
    pub item: Item,
    pub public: Visibility,
    pub span: Span,
}

#[derive(Clone, Default)]
pub struct Program {
    pub files: Vec<SourceFile>,
    pub scopes: Vec<ModuleScope>,
    pub declarations: Vec<Declaration>,
    pub resolved: bool,
    /// Populated once by resolution; later phases read this relation directly.
    pub function_owners: Vec<FunctionOwner>,
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

impl Program {
    /// The caller supplies a resolved program, whose owner table covers every function.
    pub fn function_owner(&self, function: FunctionId) -> FunctionOwner {
        self.function_owners[function.index]
    }
}
