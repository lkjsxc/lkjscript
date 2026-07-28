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
pub struct OpInfo {
    pub operand_width: usize,
    pub stack: StackEffect,
    pub control: ControlFlow,
}

impl Op {
    pub const fn info(self) -> OpInfo {
        OpInfo {
            operand_width: operand_width(self),
            stack: stack_effect(self),
            control: control_flow(self),
        }
    }

    pub const fn operand_width(self) -> usize {
        operand_width(self)
    }
}

const fn operand_width(op: Op) -> usize {
    match op {
        Op::LoadConst
        | Op::LoadGlobal
        | Op::StoreGlobal
        | Op::Jump
        | Op::JumpIfFalse
        | Op::MakeClosure
        | Op::MakeProduct
        | Op::LoadProductField
        | Op::WithProductField
        | Op::MakeEnum
        | Op::IsEnumVariant
        | Op::LoadEnumField
        | Op::ByteVectorPlaceInit
        | Op::ByteVectorMove
        | Op::ByteVectorDropPlace => 2,
        Op::LoadLocal
        | Op::StoreLocal
        | Op::Call
        | Op::ByteVectorBorrow
        | Op::ByteVectorBorrowMut
        | Op::StoreUniqueLocal
        | Op::StoreViewLocal
        | Op::TakeUniqueLocal
        | Op::LoadViewLocal
        | Op::EndBorrowLocal
        | Op::ByteVectorPlaceEnd => 1,
        _ => 0,
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
