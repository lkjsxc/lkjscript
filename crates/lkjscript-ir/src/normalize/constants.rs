use std::collections::HashMap;

use crate::normalize::*;
use crate::{Constant, EffectSet, FailureBehavior, InstructionKind, RuntimeOp, VerifiedProgram};

pub fn constant_fold_and_propagate(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let mut constants = HashMap::new();
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let replacement = match &instruction.kind {
                    InstructionKind::Constant(constant) => Some(constant.clone()),
                    InstructionKind::Copy(source) => constants.get(source).cloned(),
                    InstructionKind::Runtime {
                        operation,
                        arguments,
                        ..
                    } => {
                        let arguments: Option<Vec<Constant>> = arguments
                            .iter()
                            .map(|argument| constants.get(argument).cloned())
                            .collect();
                        arguments.and_then(|arguments| fold_runtime(*operation, &arguments))
                    }
                    _ => None,
                };
                if let Some(constant) = replacement {
                    instruction.kind = InstructionKind::Constant(constant.clone());
                    instruction.metadata.effects = EffectSet::PURE;
                    instruction.metadata.failure = FailureBehavior::None;
                    instruction.metadata.frame_state = None;
                    constants.insert(instruction.id, constant);
                }
            }
        }
    }
    finish(program)
}

pub(crate) fn fold_runtime(operation: RuntimeOp, arguments: &[Constant]) -> Option<Constant> {
    use RuntimeOp as Op;
    match (operation, arguments) {
        (Op::Add, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_add(*right).map(Constant::I64)
        }
        (Op::Subtract, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_sub(*right).map(Constant::I64)
        }
        (Op::Multiply, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_mul(*right).map(Constant::I64)
        }
        (Op::Divide, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_div(*right).map(Constant::I64)
        }
        (Op::BitAnd, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::I64(left & right))
        }
        (Op::BitOr, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::I64(left | right))
        }
        (Op::BitXor, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::I64(left ^ right))
        }
        (Op::Not, [Constant::Bool(value)]) => Some(Constant::Bool(!value)),
        (Op::Less, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left < right))
        }
        (Op::LessEqual, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left <= right))
        }
        (Op::Greater, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left > right))
        }
        (Op::GreaterEqual, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left >= right))
        }
        (Op::EqualValue, [left, right]) => fold_equal(left, right).map(Constant::Bool),
        (Op::IsEmptyList, [Constant::EmptyList]) => Some(Constant::Bool(true)),
        _ => None,
    }
}

pub(crate) fn fold_equal(left: &Constant, right: &Constant) -> Option<bool> {
    match (left, right) {
        (Constant::Unit, Constant::Unit) => Some(true),
        (Constant::Bool(left), Constant::Bool(right)) => Some(left == right),
        (Constant::I64(left), Constant::I64(right)) => Some(left == right),
        (Constant::Str(left), Constant::Str(right))
        | (Constant::Symbol(left), Constant::Symbol(right)) => Some(left == right),
        _ => None,
    }
}
