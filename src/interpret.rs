use crate::compile;
use crate::core_ir::{self, CoreProgram, Instruction, Terminator, ValueId};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeValue {
    Unit,
    Bool(bool),
    I64(i64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    pub value: RuntimeValue,
    pub compile_nanoseconds: u64,
    pub execute_nanoseconds: u64,
}

pub(crate) fn compile_and_run(snapshot: &Snapshot, entry: NodeId) -> Result<RunResult> {
    let compile_started = Instant::now();
    let program = compile::compile(snapshot, entry)?;
    let compile_nanoseconds = nanos(compile_started.elapsed().as_nanos());
    let execute_started = Instant::now();
    let value = interpret(&program)?;
    let execute_nanoseconds = nanos(execute_started.elapsed().as_nanos());
    Ok(RunResult {
        value,
        compile_nanoseconds,
        execute_nanoseconds,
    })
}

fn interpret(program: &CoreProgram) -> Result<RuntimeValue> {
    core_ir::verify(program)?;
    let function = &program.function;
    let block_index = usize::try_from(function.entry.0).map_err(|_| {
        LkError::new(
            ErrorCode::CoreIrInvalid,
            "entry block index overflows host indexes",
        )
    })?;
    let block = function
        .blocks
        .get(block_index)
        .ok_or_else(|| LkError::new(ErrorCode::CoreIrInvalid, "entry block does not exist"))?;
    let mut values = Vec::with_capacity(function.value_types.len());
    for instruction in &block.instructions {
        match *instruction {
            Instruction::ConstI64 { result, value, .. } => {
                push_value(&mut values, result, RuntimeValue::I64(value))?;
            }
            Instruction::ConstBool { result, value, .. } => {
                push_value(&mut values, result, RuntimeValue::Bool(value))?;
            }
            Instruction::AddI64 {
                origin,
                result,
                lhs,
                rhs,
            } => {
                let lhs = require_i64(&values, lhs)?;
                let rhs = require_i64(&values, rhs)?;
                let value = lhs.checked_add(rhs).ok_or_else(|| {
                    LkError::new(ErrorCode::RuntimeTrap, "i64 addition overflowed").for_node(origin)
                })?;
                push_value(&mut values, result, RuntimeValue::I64(value))?;
            }
        }
    }
    match block.terminator {
        Terminator::Return { value, .. } => value_at(&values, value),
    }
}

fn push_value(values: &mut Vec<RuntimeValue>, id: ValueId, value: RuntimeValue) -> Result<()> {
    let expected = u32::try_from(values.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "runtime value count exceeds Core IR representation",
        )
    })?;
    if id.0 != expected {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "interpreter received a non-dense Core IR result",
        ));
    }
    values.push(value);
    Ok(())
}

fn value_at(values: &[RuntimeValue], id: ValueId) -> Result<RuntimeValue> {
    let index = usize::try_from(id.0).map_err(|_| {
        LkError::new(
            ErrorCode::CoreIrInvalid,
            "runtime value index overflows host indexes",
        )
    })?;
    values.get(index).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::CoreIrInvalid,
            "runtime value index is out of bounds",
        )
    })
}

fn require_i64(values: &[RuntimeValue], id: ValueId) -> Result<i64> {
    match value_at(values, id)? {
        RuntimeValue::I64(value) => Ok(value),
        RuntimeValue::Unit | RuntimeValue::Bool(_) => Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "Core IR i64 operand has a non-i64 runtime representation",
        )),
    }
}

fn nanos(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_ir::{BlockId, CoreBlock, CoreFunction, Instruction, Terminator};
    use crate::ids::WorkspaceId;
    use crate::schema::SemanticType;

    fn node(serial: u64) -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([0x51; 16]), serial).expect("test node")
    }

    #[test]
    fn interpreter_returns_bool_through_verified_core_ir() {
        let program = CoreProgram {
            function: CoreFunction {
                origin: node(1),
                result: SemanticType::Bool,
                value_types: vec![SemanticType::Bool],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::ConstBool {
                        origin: node(3),
                        result: ValueId(0),
                        value: true,
                    }],
                    terminator: Terminator::Return {
                        origin: node(4),
                        value: ValueId(0),
                    },
                }],
            },
        };
        assert_eq!(
            interpret(&program).expect("interpret bool"),
            RuntimeValue::Bool(true)
        );
    }

    #[test]
    fn checked_i64_overflow_is_a_structured_runtime_trap() {
        let program = CoreProgram {
            function: CoreFunction {
                origin: node(1),
                result: SemanticType::I64,
                value_types: vec![SemanticType::I64, SemanticType::I64, SemanticType::I64],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: Vec::new(),
                    instructions: vec![
                        Instruction::ConstI64 {
                            origin: node(3),
                            result: ValueId(0),
                            value: i64::MAX,
                        },
                        Instruction::ConstI64 {
                            origin: node(4),
                            result: ValueId(1),
                            value: 1,
                        },
                        Instruction::AddI64 {
                            origin: node(5),
                            result: ValueId(2),
                            lhs: ValueId(0),
                            rhs: ValueId(1),
                        },
                    ],
                    terminator: Terminator::Return {
                        origin: node(6),
                        value: ValueId(2),
                    },
                }],
            },
        };
        let error = interpret(&program).expect_err("overflow must trap");
        assert_eq!(error.code, ErrorCode::RuntimeTrap);
        assert_eq!(error.target, Some(node(5)));
    }
}
