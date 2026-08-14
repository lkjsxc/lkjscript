use crate::error::{ErrorCode, LkError, Result};
use crate::ids::NodeId;
use crate::schema::SemanticType;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ValueId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BlockId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreProgram {
    pub function: CoreFunction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreFunction {
    pub origin: NodeId,
    pub result: SemanticType,
    pub value_types: Vec<SemanticType>,
    pub blocks: Vec<CoreBlock>,
    pub entry: BlockId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreBlock {
    pub origin: NodeId,
    pub parameters: Vec<ValueId>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Instruction {
    ConstI64 {
        origin: NodeId,
        result: ValueId,
        value: i64,
    },
    ConstBool {
        origin: NodeId,
        result: ValueId,
        value: bool,
    },
    AddI64 {
        origin: NodeId,
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Terminator {
    Return { origin: NodeId, value: ValueId },
}

pub(crate) fn verify(program: &CoreProgram) -> Result<()> {
    let function = &program.function;
    let entry =
        usize::try_from(function.entry.0).map_err(|_| invalid("entry block index overflows"))?;
    if entry >= function.blocks.len() {
        return Err(invalid("entry block index is out of bounds"));
    }
    if function.blocks.len() != 1 {
        return Err(invalid("bootstrap Core IR requires exactly one block"));
    }
    let mut defined = vec![false; function.value_types.len()];
    for block in &function.blocks {
        if !block.parameters.is_empty() {
            return Err(invalid(
                "bootstrap Core IR block parameters are unsupported",
            ));
        }
        for instruction in &block.instructions {
            match instruction {
                Instruction::ConstI64 { result, .. } => {
                    define(
                        &mut defined,
                        &function.value_types,
                        *result,
                        SemanticType::I64,
                    )?;
                }
                Instruction::ConstBool { result, .. } => {
                    define(
                        &mut defined,
                        &function.value_types,
                        *result,
                        SemanticType::Bool,
                    )?;
                }
                Instruction::AddI64 {
                    result, lhs, rhs, ..
                } => {
                    require_value(&defined, &function.value_types, *lhs, SemanticType::I64)?;
                    require_value(&defined, &function.value_types, *rhs, SemanticType::I64)?;
                    define(
                        &mut defined,
                        &function.value_types,
                        *result,
                        SemanticType::I64,
                    )?;
                }
            }
        }
        match block.terminator {
            Terminator::Return { value, .. } => {
                require_value(&defined, &function.value_types, value, function.result)?;
            }
        }
    }
    if defined.iter().any(|value| !*value) {
        return Err(invalid("Core IR declares an undefined dense value"));
    }
    Ok(())
}

fn define(
    defined: &mut [bool],
    types: &[SemanticType],
    value: ValueId,
    expected: SemanticType,
) -> Result<()> {
    let index = value_index(value)?;
    let actual = types
        .get(index)
        .copied()
        .ok_or_else(|| invalid("instruction result index is out of bounds"))?;
    if actual != expected {
        return Err(
            invalid("instruction result type disagrees with instruction contract")
                .with_types(expected, actual),
        );
    }
    if defined
        .get(index)
        .copied()
        .ok_or_else(|| invalid("instruction result index is out of bounds"))?
    {
        return Err(invalid("Core IR value is defined more than once"));
    }
    if defined[..index].iter().any(|prior| !*prior) {
        return Err(invalid(
            "Core IR value definitions are not dense and ordered",
        ));
    }
    defined[index] = true;
    Ok(())
}

fn require_value(
    defined: &[bool],
    types: &[SemanticType],
    value: ValueId,
    expected: SemanticType,
) -> Result<()> {
    let index = value_index(value)?;
    if !defined.get(index).copied().unwrap_or(false) {
        return Err(invalid(
            "Core IR operand is not dominated by its definition",
        ));
    }
    let actual = types
        .get(index)
        .copied()
        .ok_or_else(|| invalid("Core IR operand index is out of bounds"))?;
    if actual != expected {
        return Err(
            invalid("Core IR operand type disagrees with instruction contract")
                .with_types(expected, actual),
        );
    }
    Ok(())
}

fn value_index(value: ValueId) -> Result<usize> {
    usize::try_from(value.0).map_err(|_| invalid("Core IR value index overflows host indexes"))
}

fn invalid(message: &str) -> LkError {
    LkError::new(ErrorCode::CoreIrInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WorkspaceId;

    fn node(serial: u64) -> NodeId {
        match NodeId::new(WorkspaceId::from_bytes([3; 16]), serial) {
            Ok(id) => id,
            Err(error) => {
                assert!(serial != 0, "invalid test node: {error}");
                std::process::abort();
            }
        }
    }

    #[test]
    fn verifier_rejects_use_before_definition() {
        let program = CoreProgram {
            function: CoreFunction {
                origin: node(1),
                result: SemanticType::I64,
                value_types: vec![SemanticType::I64, SemanticType::I64],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::AddI64 {
                        origin: node(3),
                        result: ValueId(1),
                        lhs: ValueId(0),
                        rhs: ValueId(0),
                    }],
                    terminator: Terminator::Return {
                        origin: node(4),
                        value: ValueId(1),
                    },
                }],
            },
        };
        assert_eq!(
            verify(&program).err().map(|error| error.code),
            Some(ErrorCode::CoreIrInvalid)
        );
    }

    #[test]
    fn verifier_rejects_wrong_return_type_and_invalid_entry_block() {
        let wrong_return = CoreProgram {
            function: CoreFunction {
                origin: node(1),
                result: SemanticType::I64,
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
            verify(&wrong_return).expect_err("wrong return type").code,
            ErrorCode::CoreIrInvalid
        );

        let mut invalid_entry = wrong_return;
        invalid_entry.function.entry = BlockId(1);
        assert_eq!(
            verify(&invalid_entry)
                .expect_err("invalid entry block")
                .code,
            ErrorCode::CoreIrInvalid
        );
    }
}
