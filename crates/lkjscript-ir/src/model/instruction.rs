use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Safepoint {
    None,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureBehavior {
    None,
    Trap,
    StructuredOutcome,
    TrapOrOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameLocal {
    pub binding: BindingId,
    pub slot: u16,
    pub value: ValueId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameState {
    /// Stable semantic position linked to an exact bytecode offset after emission.
    pub bytecode_position: u32,
    pub locals: Vec<FrameLocal>,
    pub operand_stack: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionMetadata {
    pub origin: Origin,
    pub effects: EffectSet,
    pub safepoint: Safepoint,
    pub failure: FailureBehavior,
    pub frame_state: Option<FrameState>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallTarget {
    Direct(FunctionId),
    Indirect(ValueId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    Constant(Constant),
    Copy(ValueId),
    /// Establishes one SSA value as the current owner of a whole local place.
    /// This is an ownership fact only; it is not a user-visible store or Drop.
    PlaceInit {
        place: PlaceId,
        value: ValueId,
    },
    /// Ends the lexical identity of a whole local place after its obligation is
    /// transferred or discharged.
    PlaceEnd {
        place: PlaceId,
    },
    EndBorrow {
        place: PlaceId,
        loan: LoanId,
        value: ValueId,
    },
    Drop {
        place: PlaceId,
        value: ValueId,
        glue: DropGlueIdentity,
        kind: DropEventKind,
    },
    Move {
        place: PlaceId,
        value: ValueId,
    },
    Borrow {
        place: PlaceId,
        loan: LoanId,
        kind: BorrowKind,
        value: ValueId,
    },
    FunctionRef(FunctionId),
    Runtime {
        operation: RuntimeOp,
        arguments: Vec<ValueId>,
        signature: Signature,
    },
    F64FromI64Exact {
        value: ValueId,
    },
    F64FromI64Rounded {
        value: ValueId,
    },
    I64FromF64Exact {
        value: ValueId,
    },
    I64FromF64Trunc {
        value: ValueId,
    },
    Call {
        target: CallTarget,
        arguments: Vec<ValueId>,
        signature: Signature,
        instantiation: Option<GenericInstantiation>,
    },
    ProductValue {
        product: ProductId,
        fields: Vec<ValueId>,
    },
    ProductField {
        product: ProductId,
        field: u8,
        value: ValueId,
    },
    WithProductField {
        product: ProductId,
        field: u8,
        value: ValueId,
        replacement: ValueId,
    },
    EnumValue {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: Vec<ValueId>,
    },
    EnumIsVariant {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        value: ValueId,
    },
    EnumField {
        enum_id: EnumId,
        variant: VariantId,
        field: VariantFieldId,
        layout: RuntimeLayoutId,
        value: ValueId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub id: ValueId,
    pub ty: SsaType,
    pub kind: InstructionKind,
    pub metadata: InstructionMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockParameter {
    pub id: ValueId,
    pub ty: SsaType,
    /// Exact current-owner transport for the initial ownership slice. `None`
    /// denotes an ordinary value or an unplaced transferred affine value.
    pub owner_place: Option<PlaceId>,
    pub origin: Origin,
}
