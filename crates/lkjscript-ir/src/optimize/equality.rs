use super::equality_enums::exact_enum_instruction_kind_equal;
use super::equality_numeric::exact_numeric_instruction_kind_equal;
use crate::{Constant, Function, Instruction, InstructionKind, Program};

pub(crate) fn exact_program_equal(left: &Program, right: &Program) -> bool {
    left.memory == right.memory
        && left.region_products == right.region_products
        && left.sources == right.sources
        && left.products == right.products
        && left.enums == right.enums
        && left.traits == right.traits
        && left.implementations == right.implementations
        && left.main == right.main
        && left.functions.len() == right.functions.len()
        && left
            .functions
            .iter()
            .zip(&right.functions)
            .all(|(left, right)| exact_function_equal(left, right))
}

pub(crate) fn exact_function_equal(left: &Function, right: &Function) -> bool {
    left.id == right.id
        && left.name == right.name
        && left.signature == right.signature
        && left.places == right.places
        && left.failure_cleanups == right.failure_cleanups
        && left.effects == right.effects
        && left.entry == right.entry
        && left.origin == right.origin
        && left.blocks.len() == right.blocks.len()
        && left.blocks.iter().zip(&right.blocks).all(|(left, right)| {
            left.id == right.id
                && left.parameters == right.parameters
                && left.terminator == right.terminator
                && left.metadata == right.metadata
                && left.instructions.len() == right.instructions.len()
                && left
                    .instructions
                    .iter()
                    .zip(&right.instructions)
                    .all(|(left, right)| exact_instruction_equal(left, right))
        })
}

pub(crate) fn exact_instruction_equal(left: &Instruction, right: &Instruction) -> bool {
    left.id == right.id
        && left.ty == right.ty
        && left.metadata == right.metadata
        && exact_instruction_kind_equal(&left.kind, &right.kind)
}

pub(crate) fn exact_instruction_kind_equal(
    left: &InstructionKind,
    right: &InstructionKind,
) -> bool {
    match (left, right) {
        (InstructionKind::Constant(left), InstructionKind::Constant(right)) => {
            exact_constant_equal(left, right)
        }
        (InstructionKind::Copy(left), InstructionKind::Copy(right)) => left == right,
        (InstructionKind::FunctionRef(left), InstructionKind::FunctionRef(right)) => left == right,
        (
            InstructionKind::Runtime {
                operation: left_operation,
                arguments: left_arguments,
                signature: left_signature,
            },
            InstructionKind::Runtime {
                operation: right_operation,
                arguments: right_arguments,
                signature: right_signature,
            },
        ) => {
            left_operation == right_operation
                && left_arguments == right_arguments
                && left_signature == right_signature
        }
        (
            InstructionKind::Call {
                target: left_target,
                arguments: left_arguments,
                consuming: left_consuming,
                signature: left_signature,
                instantiation: left_instantiation,
            },
            InstructionKind::Call {
                target: right_target,
                arguments: right_arguments,
                consuming: right_consuming,
                signature: right_signature,
                instantiation: right_instantiation,
            },
        ) => {
            left_target == right_target
                && left_arguments == right_arguments
                && left_consuming == right_consuming
                && left_signature == right_signature
                && left_instantiation == right_instantiation
        }
        (
            InstructionKind::ProductValue {
                product: left_product,
                fields: left_fields,
            },
            InstructionKind::ProductValue {
                product: right_product,
                fields: right_fields,
            },
        ) => left_product == right_product && left_fields == right_fields,
        (
            InstructionKind::ProductField {
                product: left_product,
                field: left_field,
                value: left_value,
            },
            InstructionKind::ProductField {
                product: right_product,
                field: right_field,
                value: right_value,
            },
        ) => {
            left_product == right_product && left_field == right_field && left_value == right_value
        }
        (
            InstructionKind::WithProductField {
                product: left_product,
                field: left_field,
                value: left_value,
                replacement: left_replacement,
            },
            InstructionKind::WithProductField {
                product: right_product,
                field: right_field,
                value: right_value,
                replacement: right_replacement,
            },
        ) => {
            left_product == right_product
                && left_field == right_field
                && left_value == right_value
                && left_replacement == right_replacement
        }
        _ => {
            super::equality_ownership::exact_ownership_instruction_kind_equal(left, right)
                || exact_numeric_instruction_kind_equal(left, right)
                || exact_enum_instruction_kind_equal(left, right)
        }
    }
}

pub(crate) fn exact_constant_equal(left: &Constant, right: &Constant) -> bool {
    match (left, right) {
        (Constant::F64(left), Constant::F64(right)) => left.to_bits() == right.to_bits(),
        (Constant::Unit, Constant::Unit) | (Constant::EmptyList, Constant::EmptyList) => true,
        (Constant::Bool(left), Constant::Bool(right)) => left == right,
        (Constant::I64(left), Constant::I64(right)) => left == right,
        (Constant::Str(left), Constant::Str(right))
        | (Constant::Symbol(left), Constant::Symbol(right)) => left == right,
        (Constant::StaticBytes(left), Constant::StaticBytes(right)) => left == right,
        _ => false,
    }
}
