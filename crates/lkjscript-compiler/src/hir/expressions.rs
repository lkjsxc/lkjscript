use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub ty: Type,
    pub effects: EffectSet,
    pub origin: SourceId,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    EmptyList,
    LitStr(String),
    LitBytes(Vec<u8>),
    Load(BindingRef),
    Move {
        place: PlaceId,
        binding: BindingRef,
    },
    Borrow {
        place: PlaceId,
        loan: LoanId,
        kind: BorrowKind,
        binding: BindingRef,
    },
    BorrowBytes {
        place: PlaceId,
        loan: LoanId,
        binding: BindingRef,
    },
    Call {
        callee: BindingRef,
        args: Vec<Expr>,
        instantiation: Option<GenericInstantiation>,
    },
    Operation {
        binding: BindingId,
        operation: Operation,
        resolved_signature: Type,
        args: Vec<Expr>,
    },
    F64FromI64Exact(Box<Expr>),
    F64FromI64Rounded(Box<Expr>),
    I64FromF64Exact(Box<Expr>),
    I64FromF64Trunc(Box<Expr>),
    Do(Vec<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    While {
        loop_id: LoopId,
        condition: Box<Expr>,
        body: Vec<Expr>,
    },
    Loop {
        loop_id: LoopId,
        result_type: Type,
        body: Vec<Expr>,
    },
    Return {
        value: Box<Expr>,
    },
    Break {
        loop_id: LoopId,
        value: Box<Expr>,
    },
    Continue {
        loop_id: LoopId,
    },
    Trap {
        value: Box<Expr>,
    },
    Exit {
        code: Box<Expr>,
    },
    Let {
        bindings: Vec<LocalDefinition>,
        body: Box<Expr>,
    },
    MutableLocal {
        binding: BindingId,
        place: PlaceId,
        slot: u8,
        initial: Box<Expr>,
        body: Box<Expr>,
    },
    SetLocal {
        target: BindingId,
        slot: u8,
        value: Box<Expr>,
    },
    ProductValue {
        product: ProductId,
        fields: Vec<Expr>,
    },
    ProductField {
        product: ProductId,
        field: u8,
        value: Box<Expr>,
    },
    WithProductField {
        product: ProductId,
        field: u8,
        value: Box<Expr>,
        replacement: Box<Expr>,
    },
    EnumValue {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: Vec<Expr>,
    },
    EnumIsVariant {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        value: Box<Expr>,
    },
    EnumField {
        enum_id: EnumId,
        variant: VariantId,
        field: VariantFieldId,
        layout: RuntimeLayoutId,
        value: Box<Expr>,
    },
    EnumUnwrap {
        enum_id: EnumId,
        variant: VariantId,
        field: VariantFieldId,
        layout: RuntimeLayoutId,
        value: Box<Expr>,
        trap: String,
    },
    MatchUnreachable {
        plan: MatchPlanId,
    },
    QuoteSymbol(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalDefinition {
    pub binding: BindingId,
    pub place: PlaceId,
    pub static_bytes: bool,
    pub slot: u8,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingRef {
    pub binding: BindingId,
    pub storage: BindingStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStorage {
    Local(u8),
    Function,
}
