//! Explicit, monomorphic meaning. Only CheckedProgram reaches a backend.
#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Unit,
    Bool,
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
}
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub name: String,
    pub kind: TypeKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Projection {
    Deref,
    Field(usize),
    VariantField(usize, usize),
    Index(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct Local {
    pub name: String,
    pub ty: usize,
    pub mutable: bool,
}
#[derive(Clone, Debug, PartialEq)]
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
        src: usize,
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
    Return {
        src: usize,
    },
    Trap {
        message: String,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub op: Op,
    pub line: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReturnSource {
    /// Reference leaves in a parameter value, after the declared dereferences.
    /// Projected sources currently require shared leaves; exclusive reborrows
    /// retain the lifetime of their outer reference.
    Parameter {
        index: usize,
        projections: Vec<Projection>,
    },
    Static,
}
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
