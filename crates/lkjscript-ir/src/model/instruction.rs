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
    /// Ends the lexical identity of a whole local place. Runtime cleanup remains
    /// separate from deterministic source Drop, which is not in this slice.
    PlaceEnd {
        place: PlaceId,
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
}

impl InstructionKind {
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Constant(_) | Self::PlaceEnd { .. } | Self::FunctionRef(_) => Vec::new(),
            Self::Copy(value)
            | Self::PlaceInit { value, .. }
            | Self::Move { value, .. }
            | Self::Borrow { value, .. } => vec![*value],
            Self::Runtime { arguments, .. }
            | Self::Call {
                target: CallTarget::Direct(_),
                arguments,
                ..
            } => arguments.clone(),
            Self::Call {
                target: CallTarget::Indirect(target),
                arguments,
                ..
            } => {
                let mut operands = Vec::with_capacity(arguments.len().saturating_add(1));
                operands.push(*target);
                operands.extend(arguments.iter().copied());
                operands
            }
            Self::ProductValue { fields, .. } => fields.clone(),
            Self::ProductField { value, .. } => vec![*value],
            Self::WithProductField {
                value, replacement, ..
            } => vec![*value, *replacement],
        }
    }
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
