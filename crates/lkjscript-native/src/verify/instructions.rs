mod failure;
mod runtime;
mod structural;
use failure::verify_failure_cleanup;
use runtime::verify_runtime_slot;
use structural::verify_structural_call;

use super::*;

pub(super) fn verify_instruction(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    signatures: &[(FunctionId, Signature)],
    available_values: &[bool],
    initialized_locals: &[bool],
) -> Result<(), VerificationError> {
    for operand in instruction.operation.operands() {
        require_available(function, operand, available_values)?;
        if observed_local_alias(function, operand)
            && !matches!(
                &instruction.operation,
                Operation::StructuralCall(descriptor, _)
                    if descriptor.operation().is_observation()
                        || matches!(descriptor.operation(), StructuralOperation::Borrow { .. })
            )
            && !matches!(&instruction.operation, Operation::HeapCall(_, _))
        {
            return Err(VerificationError::TypeMismatch(
                "observed structural local use",
            ));
        }
    }
    verify_failure_cleanup(function, instruction, initialized_locals)?;
    match &instruction.operation {
        Operation::I64Const(_) => require_output(instruction, ValueType::I64, "I64 constant"),
        Operation::F64Const(_) => require_output(instruction, ValueType::F64, "F64 constant"),
        Operation::BoolConst(_) => require_output(instruction, ValueType::Bool, "Bool constant"),
        Operation::Unit => require_output(instruction, ValueType::Unit, "Unit constant"),
        Operation::MemoryWitnessLocator(_) => require_output(
            instruction,
            ValueType::MemoryWitnessLocator,
            "memory witness locator",
        ),
        Operation::StaticBytesConst(_) => {
            require_output(instruction, ValueType::StaticBytes, "static bytes constant")
        }
        Operation::StaticStringConst(_, value_type) => require_output(
            instruction,
            ValueType::StaticString(*value_type),
            "static string constant",
        ),
        Operation::I64Add(left, right)
        | Operation::I64Sub(left, right)
        | Operation::I64Mul(left, right)
        | Operation::I64Div(left, right)
        | Operation::I64BitAnd(left, right)
        | Operation::I64BitOr(left, right)
        | Operation::I64BitXor(left, right) => {
            require_types(function, [*left, *right], ValueType::I64, "I64 arithmetic")?;
            require_output(instruction, ValueType::I64, "I64 arithmetic")
        }
        Operation::I64ToF64(value) => {
            require_types(function, [*value], ValueType::I64, "I64 to F64 conversion")?;
            require_output(instruction, ValueType::F64, "I64 to F64 conversion")
        }
        Operation::F64Add(left, right)
        | Operation::F64Sub(left, right)
        | Operation::F64Mul(left, right)
        | Operation::F64Div(left, right) => {
            require_types(function, [*left, *right], ValueType::F64, "F64 arithmetic")?;
            require_output(instruction, ValueType::F64, "F64 arithmetic")
        }
        Operation::I64Compare(_, left, right) => {
            require_types(function, [*left, *right], ValueType::I64, "I64 comparison")?;
            require_output(instruction, ValueType::Bool, "I64 comparison")
        }
        Operation::F64Compare(_, left, right) | Operation::F64BitsEqual(left, right) => {
            require_types(function, [*left, *right], ValueType::F64, "F64 comparison")?;
            require_output(instruction, ValueType::Bool, "F64 comparison")
        }
        Operation::BoolCompare(_, left, right) => {
            require_types(
                function,
                [*left, *right],
                ValueType::Bool,
                "Bool comparison",
            )?;
            require_output(instruction, ValueType::Bool, "Bool comparison")
        }
        Operation::BoolNot(value) => {
            require_types(function, [*value], ValueType::Bool, "Bool not")?;
            require_output(instruction, ValueType::Bool, "Bool not")
        }
        Operation::ReadLocal(local) | Operation::ObserveLocal(local) => {
            let index = local_index(function, *local)?;
            if !initialized_locals.get(index).copied().unwrap_or(false) {
                return Err(VerificationError::LocalNotInitialized(*local));
            }
            let value_type = function.locals[index].value_type;
            let observable = match value_type {
                ValueType::StructuralOwner(_) | ValueType::StructuralKey => true,
                ValueType::StructuralView(view) => !view.exclusive(),
                _ => false,
            };
            if matches!(instruction.operation, Operation::ObserveLocal(_)) && !observable {
                return Err(VerificationError::TypeMismatch(
                    "observed local is not a structural owner, key, or shared view",
                ));
            }
            require_output(instruction, value_type, "local read")
        }
        Operation::WriteLocal(local, value) => {
            let local_type = function.locals[local_index(function, *local)?].value_type;
            if value_type(function, *value)? != local_type {
                return Err(VerificationError::TypeMismatch("local write"));
            }
            require_output(instruction, ValueType::Unit, "local write")
        }
        Operation::Call(callee, arguments) => {
            let signature = signatures
                .iter()
                .find(|(function_id, _)| function_id == callee)
                .map(|(_, signature)| signature)
                .ok_or(VerificationError::InvalidCall(*callee))?;
            verify_arguments(function, arguments, signature, "compiled call")?;
            require_output(instruction, signature.result(), "compiled call")
        }
        Operation::RuntimeCall(slot, arguments) => {
            verify_runtime_slot(*slot)?;
            let signature = slot
                .plan_signature()
                .ok_or(VerificationError::TypeMismatch(
                    "encoder-owned runtime call",
                ))?;
            verify_arguments(function, arguments, &signature, "runtime call")?;
            require_output(instruction, signature.result(), "runtime call")
        }
        Operation::StructuralCall(descriptor, arguments) => {
            verify_structural_call(function, instruction, descriptor, arguments)
        }
        Operation::HeapCall(descriptor, arguments) => {
            if !descriptor.canonical_facts_are_valid()
                || descriptor.input_types().len() != arguments.len()
                || Some(descriptor.input_types().len()) != descriptor.operation().expected_arity()
                || arguments
                    .iter()
                    .zip(descriptor.input_types())
                    .any(|(argument, expected)| {
                        value_type(function, *argument).ok() != Some(*expected)
                    })
            {
                return Err(VerificationError::TypeMismatch("heap runtime call"));
            }
            require_output(instruction, descriptor.result_type(), "heap runtime call")
        }
    }
}

fn observed_local_alias(function: &FunctionPlan, value: ValueId) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            instruction.output == value
                && matches!(instruction.operation, Operation::ObserveLocal(_))
        })
    })
}
