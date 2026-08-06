use super::{model::Op, stack::stack_effect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackEffect {
    Fixed {
        required: usize,
        pops: usize,
        pushes: usize,
    },
    Call,
    MakeProduct,
    MakeEnum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Next,
    Jump,
    Branch,
    Return,
    Exit,
    Trap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandLayout {
    None,
    U16,
    Index,
    PlaceLocal,
}

impl OperandLayout {
    pub const fn byte_width(self) -> usize {
        match self {
            Self::None => 0,
            Self::U16 => 2,
            Self::Index => 8,
            Self::PlaceLocal => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpInfo {
    pub operand_layout: OperandLayout,
    pub stack: StackEffect,
    pub control: ControlFlow,
}

impl Op {
    pub const fn info(self) -> OpInfo {
        OpInfo {
            operand_layout: operand_layout(self),
            stack: stack_effect(self),
            control: control_flow(self),
        }
    }

    pub const fn operand_layout(self) -> OperandLayout {
        operand_layout(self)
    }

    pub const fn operand_width(self) -> usize {
        operand_layout(self).byte_width()
    }
}

const fn operand_layout(op: Op) -> OperandLayout {
    match op {
        Op::Car
        | Op::MakeProduct
        | Op::LoadProductField
        | Op::WithProductField
        | Op::MakeEnum
        | Op::IsEnumVariant
        | Op::LoadEnumField
        | Op::StructuralBorrow
        | Op::StructuralBorrowMut
        | Op::StructuralPublish
        | Op::StructuralDestinationCreate
        | Op::StructuralDestinationFieldInit
        | Op::StructuralDestinationFinish
        | Op::StructuralDestinationAbort
        | Op::StructuralAggregateFieldBorrow
        | Op::StructuralAggregateFieldCopy
        | Op::StructuralAggregateTag
        | Op::StructuralAggregateConsumePayload
        | Op::StructuralStringUtf8View
        | Op::StructuralCopy => OperandLayout::U16,
        Op::Jump
        | Op::JumpIfFalse
        | Op::LoadConst
        | Op::LoadLocal
        | Op::LoadGlobal
        | Op::StoreGlobal
        | Op::MakeClosure
        | Op::StoreLocal
        | Op::Call
        | Op::ByteVectorBorrow
        | Op::ByteVectorBorrowMut
        | Op::BytesBorrow
        | Op::StoreUniqueLocal
        | Op::StoreViewLocal
        | Op::TakeUniqueLocal
        | Op::LoadViewLocal
        | Op::EndBorrowLocal
        | Op::StoreStructuralLocal
        | Op::TakeStructuralLocal
        | Op::LoadStructuralViewLocal
        | Op::EndStructuralBorrowLocal
        | Op::LoadStructuralOwnerLocal
        | Op::StructuralPlaceEnd
        | Op::MemoryWitnessIndependentOwner
        | Op::MemoryWitnessCompare
        | Op::MemoryWitnessDispose
        | Op::ByteVectorPlaceEnd
        | Op::BytesPlaceEnd => OperandLayout::Index,
        Op::ByteVectorPlaceInit
        | Op::ByteVectorMove
        | Op::ByteVectorDropPlace
        | Op::BytesPlaceInit
        | Op::BytesMove
        | Op::BytesDropPlace
        | Op::StructuralPlaceInit
        | Op::StructuralMove
        | Op::StructuralDropPlace => OperandLayout::PlaceLocal,
        _ => OperandLayout::None,
    }
}

const fn control_flow(op: Op) -> ControlFlow {
    match op {
        Op::Jump => ControlFlow::Jump,
        Op::JumpIfFalse => ControlFlow::Branch,
        Op::Return => ControlFlow::Return,
        Op::Exit => ControlFlow::Exit,
        Op::Trap => ControlFlow::Trap,
        _ => ControlFlow::Next,
    }
}
