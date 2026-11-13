//! Explicit, monomorphic meaning. Only CheckedProgram reaches a backend.
#[derive(Clone, PartialEq)]
pub enum TypeKind {
    Unit,
    Bool,
    /// A Unicode scalar value: 0..=0x10ffff excluding 0xd800..=0xdfff.
    Char,
    Int {
        bits: u8,
        signed: bool,
    },
    F64,
    Struct {
        fields: Vec<Field>,
        copy: bool,
    },
    Enum {
        variants: Vec<Variant>,
        copy: bool,
    },
    Ref {
        inner: usize,
        mutable: bool,
    },
    Slice {
        inner: usize,
        mutable: bool,
    },
    Array {
        inner: usize,
        len: usize,
    },
    Vec {
        inner: usize,
    },
    Box {
        inner: usize,
    },
    /// Unsized immutable UTF-8 text; stored values use a shared Ref.
    Str,
    World,
    /// Affine authority to write standard output, derived from World.
    Stdout,
}
#[derive(Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: usize,
}
#[derive(Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}
#[derive(Clone, PartialEq)]
pub struct Type {
    pub name: String,
    pub kind: TypeKind,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Projection {
    Deref,
    Field(usize),
    VariantField(usize, usize),
    Index(usize),
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub local: usize,
    pub projections: Vec<Projection>,
}
impl Place {
    pub fn local(local: usize) -> Place {
        Place {
            local,
            projections: Vec::new(),
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct Local {
    pub name: String,
    pub ty: usize,
    pub mutable: bool,
}
#[derive(Clone, PartialEq)]
pub enum Op {
    /// Immutable UTF-8 bytes with module-long storage. The checker validates bytes.
    Text {
        dst: usize,
        bytes: Vec<u8>,
    },
    Int {
        dst: usize,
        value: i64,
    },
    Float {
        dst: usize,
        bits: u64,
    },
    Unit {
        dst: usize,
    },
    Read {
        dst: usize,
        place: Place,
        moving: bool,
    },
    Borrow {
        dst: usize,
        place: Place,
        mutable: bool,
    },
    SliceBorrow {
        dst: usize,
        place: Place,
        mutable: bool,
    },
    Array {
        dst: usize,
        elements: Vec<usize>,
    },
    Assign {
        place: Place,
        src: usize,
    },
    Drop {
        place: Place,
    },
    /// Destroy this place only when its own storage is initialized. Every
    /// projected ancestor must still exist; this does not authorize a hole
    /// through a reference or an indexed container.
    DropIf {
        place: Place,
    },
    Unary {
        dst: usize,
        op: String,
        src: usize,
    },
    Binary {
        dst: usize,
        op: String,
        left: usize,
        right: usize,
    },
    Call {
        dst: usize,
        function: usize,
        args: Vec<usize>,
    },
    Struct {
        dst: usize,
        fields: Vec<usize>,
    },
    Variant {
        dst: usize,
        variant: usize,
        fields: Vec<usize>,
    },
    Discriminant {
        dst: usize,
        src: usize,
    },
    TagTest {
        dst: usize,
        place: Place,
        variant: usize,
    },
    Intrinsic {
        dst: usize,
        name: String,
        args: Vec<usize>,
    },
    If {
        condition: usize,
    },
    Else,
    End,
    Loop,
    Break,
    Continue,
    /// Select an enclosing lexical loop, counting only loops (zero is nearest).
    BreakTo {
        depth: usize,
    },
    ContinueTo {
        depth: usize,
    },
    Return {
        src: usize,
    },
    Trap {
        message: String,
    },
}
#[derive(Clone, PartialEq)]
pub struct Instruction {
    pub op: Op,
    pub line: usize,
    pub column: usize,
}
/// An input selector names one stored reference leaf in a parameter value.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputLeaf {
    pub index: usize,
    pub projections: Vec<Projection>,
}
/// Every result reference leaf selects a declared input alternative or static storage.
#[derive(Clone, PartialEq, Eq)]
pub enum LeafOrigin {
    Inputs {
        inputs: Vec<InputLeaf>,
        /// Exact preserves an input referent; bounded permits a descendant.
        exact: bool,
    },
    Static,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ReturnLeaf {
    pub projections: Vec<Projection>,
    pub origin: LeafOrigin,
}
#[derive(Clone, PartialEq, Eq)]
pub enum ReturnSource {
    /// A complete, unique contract for every reference leaf in the result.
    Leaves(Vec<ReturnLeaf>),
    /// Reference leaves in a parameter value, after the declared dereferences.
    /// Projected sources currently require shared leaves; exclusive reborrows
    /// retain the lifetime of their outer reference.
    Parameter {
        index: usize,
        projections: Vec<Projection>,
    },
    Static,
}
#[derive(Clone, PartialEq)]
pub struct Function {
    pub file: usize,
    pub name: String,
    pub params: usize,
    pub result: usize,
    pub return_source: Option<ReturnSource>,
    pub locals: Vec<Local>,
    pub body: Vec<Instruction>,
    pub test: bool,
}
#[derive(Clone, PartialEq)]
pub struct Program {
    pub types: Vec<Type>,
    pub functions: Vec<Function>,
    pub destructors: Vec<(usize, usize)>,
}
pub struct CheckedProgram {
    pub(crate) program: Program,
}
impl CheckedProgram {
    pub fn program(&self) -> &Program {
        &self.program
    }
}
pub const UNIT: usize = 0;
pub const BOOL: usize = 1;
pub const I32: usize = 2;
pub const I64: usize = 3;
pub const U64: usize = 4;
pub const U8: usize = 5;
pub const F64: usize = 6;
pub fn primitives() -> Vec<Type> {
    vec![
        Type {
            name: "()".to_string(),
            kind: TypeKind::Unit,
        },
        Type {
            name: "bool".to_string(),
            kind: TypeKind::Bool,
        },
        Type {
            name: "i32".to_string(),
            kind: TypeKind::Int {
                bits: 32,
                signed: true,
            },
        },
        Type {
            name: "i64".to_string(),
            kind: TypeKind::Int {
                bits: 64,
                signed: true,
            },
        },
        Type {
            name: "usize".to_string(),
            kind: TypeKind::Int {
                bits: 64,
                signed: false,
            },
        },
        Type {
            name: "u8".to_string(),
            kind: TypeKind::Int {
                bits: 8,
                signed: false,
            },
        },
        Type {
            name: "f64".to_string(),
            kind: TypeKind::F64,
        },
    ]
}
