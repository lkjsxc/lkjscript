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
    LitNone,
    LitStr(String),
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
    Do(Vec<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    While {
        condition: Box<Expr>,
        body: Vec<Expr>,
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
    QuoteSymbol(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalDefinition {
    pub binding: BindingId,
    pub place: PlaceId,
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
