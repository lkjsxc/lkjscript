use crate::error::{ErrorCode, LkError, Result};
use crate::ids::NodeId;
use crate::schema::SemanticType;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FunctionId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct BlockId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ValueId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreProgram {
    pub functions: Vec<CoreFunction>,
    pub entry: FunctionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreFunction {
    pub origin: NodeId,
    pub parameters: Vec<ValueId>,
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
    ConstUnit {
        origin: NodeId,
        result: ValueId,
    },
    ConstBool {
        origin: NodeId,
        result: ValueId,
        value: bool,
    },
    ConstI64 {
        origin: NodeId,
        result: ValueId,
        value: i64,
    },
    AddI64 {
        origin: NodeId,
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    LtI64 {
        origin: NodeId,
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    Call {
        origin: NodeId,
        result: ValueId,
        function: FunctionId,
        arguments: Vec<ValueId>,
    },
}

impl Instruction {
    pub const fn origin(&self) -> NodeId {
        match self {
            Self::ConstUnit { origin, .. }
            | Self::ConstBool { origin, .. }
            | Self::ConstI64 { origin, .. }
            | Self::AddI64 { origin, .. }
            | Self::LtI64 { origin, .. }
            | Self::Call { origin, .. } => *origin,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Terminator {
    Return {
        origin: NodeId,
        value: ValueId,
    },
    Branch {
        origin: NodeId,
        target: BlockId,
        arguments: Vec<ValueId>,
    },
    CondBranch {
        origin: NodeId,
        condition: ValueId,
        then_target: BlockId,
        then_arguments: Vec<ValueId>,
        else_target: BlockId,
        else_arguments: Vec<ValueId>,
    },
}

impl Terminator {
    pub const fn origin(&self) -> NodeId {
        match self {
            Self::Return { origin, .. }
            | Self::Branch { origin, .. }
            | Self::CondBranch { origin, .. } => *origin,
        }
    }
}

pub(crate) fn verify(program: &CoreProgram) -> Result<()> {
    let entry = function_index(program.entry)?;
    if entry >= program.functions.len() {
        return Err(invalid("program entry function is out of bounds"));
    }
    for function in &program.functions {
        verify_function(program, function)?;
    }
    Ok(())
}

fn verify_function(program: &CoreProgram, function: &CoreFunction) -> Result<()> {
    let entry = block_index(function.entry)?;
    let entry_block = function
        .blocks
        .get(entry)
        .ok_or_else(|| invalid("function entry block is out of bounds"))?;
    if entry_block.parameters != function.parameters {
        return Err(invalid(
            "entry block parameters must exactly equal function parameters",
        ));
    }
    let mut defined = vec![false; function.value_types.len()];
    for parameter in &function.parameters {
        let index = value_index(*parameter)?;
        let _ = function
            .value_types
            .get(index)
            .ok_or_else(|| invalid("function parameter value is out of bounds"))?;
        define(&mut defined, &function.value_types, *parameter, None)?;
    }
    for (block_index_value, block) in function.blocks.iter().enumerate() {
        let mut local = vec![false; function.value_types.len()];
        for parameter in &block.parameters {
            let index = value_index(*parameter)?;
            let _ = function
                .value_types
                .get(index)
                .ok_or_else(|| invalid("block parameter value is out of bounds"))?;
            if local[index] {
                return Err(invalid("block parameter is repeated"));
            }
            if block_index_value != entry {
                define(&mut defined, &function.value_types, *parameter, None)?;
            } else if !function.parameters.contains(parameter) {
                return Err(invalid("entry block contains a non-function parameter"));
            }
            local[index] = true;
        }
        for instruction in &block.instructions {
            verify_instruction(program, function, instruction, &local)?;
            let (result, expected) = match instruction {
                Instruction::ConstUnit { result, .. } => (*result, SemanticType::Unit),
                Instruction::ConstBool { result, .. } => (*result, SemanticType::Bool),
                Instruction::ConstI64 { result, .. } => (*result, SemanticType::I64),
                Instruction::AddI64 { result, .. } => (*result, SemanticType::I64),
                Instruction::LtI64 { result, .. } => (*result, SemanticType::Bool),
                Instruction::Call {
                    result,
                    function: target,
                    ..
                } => {
                    let callee = program
                        .functions
                        .get(function_index(*target)?)
                        .ok_or_else(|| invalid("call target function is out of bounds"))?;
                    (*result, callee.result)
                }
            };
            define(&mut defined, &function.value_types, result, Some(expected))?;
            local[value_index(result)?] = true;
        }
        verify_terminator(function, block, &local)?;
    }
    if defined.iter().any(|value| !*value) {
        return Err(invalid("Core IR declares a value that is never defined"));
    }
    Ok(())
}

fn verify_instruction(
    program: &CoreProgram,
    function: &CoreFunction,
    instruction: &Instruction,
    local: &[bool],
) -> Result<()> {
    match instruction {
        Instruction::ConstUnit { .. }
        | Instruction::ConstBool { .. }
        | Instruction::ConstI64 { .. } => Ok(()),
        Instruction::AddI64 { lhs, rhs, .. } | Instruction::LtI64 { lhs, rhs, .. } => {
            require_local(function, local, *lhs, SemanticType::I64)?;
            require_local(function, local, *rhs, SemanticType::I64)
        }
        Instruction::Call {
            function: target,
            arguments,
            ..
        } => {
            let callee = program
                .functions
                .get(function_index(*target)?)
                .ok_or_else(|| invalid("call target function is out of bounds"))?;
            if arguments.len() != callee.parameters.len() {
                return Err(invalid(
                    "call argument count disagrees with callee parameters",
                ));
            }
            for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                require_local(function, local, *argument, value_type(callee, *parameter)?)?;
            }
            Ok(())
        }
    }
}

fn verify_terminator(function: &CoreFunction, block: &CoreBlock, local: &[bool]) -> Result<()> {
    match &block.terminator {
        Terminator::Return { value, .. } => require_local(function, local, *value, function.result),
        Terminator::Branch {
            target, arguments, ..
        } => verify_edge(function, local, *target, arguments),
        Terminator::CondBranch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            require_local(function, local, *condition, SemanticType::Bool)?;
            verify_edge(function, local, *then_target, then_arguments)?;
            verify_edge(function, local, *else_target, else_arguments)
        }
    }
}

fn verify_edge(
    function: &CoreFunction,
    local: &[bool],
    target: BlockId,
    arguments: &[ValueId],
) -> Result<()> {
    let target_block = function
        .blocks
        .get(block_index(target)?)
        .ok_or_else(|| invalid("branch target block is out of bounds"))?;
    if arguments.len() != target_block.parameters.len() {
        return Err(invalid(
            "branch argument count disagrees with target block parameters",
        ));
    }
    for (argument, parameter) in arguments.iter().zip(&target_block.parameters) {
        require_local(
            function,
            local,
            *argument,
            value_type(function, *parameter)?,
        )?;
    }
    Ok(())
}

fn define(
    defined: &mut [bool],
    types: &[SemanticType],
    value: ValueId,
    expected: Option<SemanticType>,
) -> Result<()> {
    let index = value_index(value)?;
    let actual = types
        .get(index)
        .copied()
        .ok_or_else(|| invalid("defined value is out of bounds"))?;
    if let Some(expected) = expected
        && actual != expected
    {
        return Err(
            invalid("instruction result type disagrees with its contract")
                .with_types(expected, actual),
        );
    }
    if defined[index] {
        return Err(invalid("Core IR value is defined more than once"));
    }
    defined[index] = true;
    Ok(())
}

fn require_local(
    function: &CoreFunction,
    local: &[bool],
    value: ValueId,
    expected: SemanticType,
) -> Result<()> {
    let index = value_index(value)?;
    let actual = function
        .value_types
        .get(index)
        .copied()
        .ok_or_else(|| invalid("operand value is out of bounds"))?;
    if !local.get(index).copied().unwrap_or(false) {
        return Err(invalid("Core IR operand is not available in this block"));
    }
    if actual != expected {
        return Err(invalid("Core IR operand type disagrees with its contract")
            .with_types(expected, actual));
    }
    Ok(())
}

fn value_type(function: &CoreFunction, value: ValueId) -> Result<SemanticType> {
    function
        .value_types
        .get(value_index(value)?)
        .copied()
        .ok_or_else(|| invalid("value type index is out of bounds"))
}

fn function_index(id: FunctionId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("function index overflows host indexes"))
}
fn block_index(id: BlockId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("block index overflows host indexes"))
}
fn value_index(id: ValueId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("value index overflows host indexes"))
}
fn invalid(message: &str) -> LkError {
    LkError::new(ErrorCode::CoreIrInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WorkspaceId;

    fn node(serial: u64) -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([3; 16]), serial).expect("node")
    }
    fn valid() -> CoreProgram {
        CoreProgram {
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: vec![ValueId(0)],
                result: SemanticType::I64,
                value_types: vec![SemanticType::I64],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: vec![ValueId(0)],
                    instructions: vec![],
                    terminator: Terminator::Return {
                        origin: node(3),
                        value: ValueId(0),
                    },
                }],
            }],
        }
    }
    fn rejects(mutator: impl FnOnce(&mut CoreProgram)) {
        let mut p = valid();
        mutator(&mut p);
        assert_eq!(
            verify(&p).expect_err("invalid").code,
            ErrorCode::CoreIrInvalid
        );
    }
    fn assert_invalid(program: &CoreProgram, message: &str) {
        let error = verify(program).expect_err(message);
        assert_eq!(error.code, ErrorCode::CoreIrInvalid);
        assert_eq!(error.message, message);
    }
    fn constant(instruction: Instruction, ty: SemanticType) -> CoreProgram {
        CoreProgram {
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: vec![],
                result: ty,
                value_types: vec![ty],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: vec![],
                    instructions: vec![instruction],
                    terminator: Terminator::Return {
                        origin: node(3),
                        value: ValueId(0),
                    },
                }],
            }],
        }
    }
    fn conditional() -> CoreProgram {
        CoreProgram {
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: vec![ValueId(0), ValueId(1)],
                result: SemanticType::I64,
                value_types: vec![
                    SemanticType::Bool,
                    SemanticType::I64,
                    SemanticType::I64,
                    SemanticType::I64,
                ],
                entry: BlockId(0),
                blocks: vec![
                    CoreBlock {
                        origin: node(2),
                        parameters: vec![ValueId(0), ValueId(1)],
                        instructions: vec![],
                        terminator: Terminator::CondBranch {
                            origin: node(3),
                            condition: ValueId(0),
                            then_target: BlockId(1),
                            then_arguments: vec![ValueId(1)],
                            else_target: BlockId(2),
                            else_arguments: vec![ValueId(1)],
                        },
                    },
                    CoreBlock {
                        origin: node(4),
                        parameters: vec![ValueId(2)],
                        instructions: vec![],
                        terminator: Terminator::Return {
                            origin: node(5),
                            value: ValueId(2),
                        },
                    },
                    CoreBlock {
                        origin: node(6),
                        parameters: vec![ValueId(3)],
                        instructions: vec![],
                        terminator: Terminator::Return {
                            origin: node(7),
                            value: ValueId(3),
                        },
                    },
                ],
            }],
        }
    }
    fn call_program() -> CoreProgram {
        CoreProgram {
            entry: FunctionId(0),
            functions: vec![
                CoreFunction {
                    origin: node(1),
                    parameters: vec![ValueId(0)],
                    result: SemanticType::I64,
                    value_types: vec![SemanticType::I64, SemanticType::I64],
                    entry: BlockId(0),
                    blocks: vec![CoreBlock {
                        origin: node(2),
                        parameters: vec![ValueId(0)],
                        instructions: vec![Instruction::Call {
                            origin: node(3),
                            result: ValueId(1),
                            function: FunctionId(1),
                            arguments: vec![ValueId(0)],
                        }],
                        terminator: Terminator::Return {
                            origin: node(4),
                            value: ValueId(1),
                        },
                    }],
                },
                CoreFunction {
                    origin: node(10),
                    parameters: vec![ValueId(0)],
                    result: SemanticType::I64,
                    value_types: vec![SemanticType::I64],
                    entry: BlockId(0),
                    blocks: vec![CoreBlock {
                        origin: node(11),
                        parameters: vec![ValueId(0)],
                        instructions: vec![],
                        terminator: Terminator::Return {
                            origin: node(12),
                            value: ValueId(0),
                        },
                    }],
                },
            ],
        }
    }

    #[test]
    fn verifier_accepts_exact_parameter_contract() {
        verify(&valid()).expect("valid");
    }
    #[test]
    fn verifier_rejects_program_function_and_block_ranges() {
        rejects(|p| p.entry = FunctionId(1));
        rejects(|p| p.functions[0].entry = BlockId(1));
        rejects(|p| {
            p.functions[0].blocks[0].terminator = Terminator::Branch {
                origin: node(3),
                target: BlockId(1),
                arguments: vec![],
            }
        });
    }
    #[test]
    fn verifier_rejects_parameter_definition_and_locality_malformations() {
        rejects(|p| p.functions[0].blocks[0].parameters.clear());
        rejects(|p| {
            p.functions[0].value_types.push(SemanticType::I64);
        });
        rejects(|p| {
            p.functions[0].value_types.push(SemanticType::I64);
            p.functions[0].blocks.push(CoreBlock {
                origin: node(4),
                parameters: vec![ValueId(1)],
                instructions: vec![Instruction::AddI64 {
                    origin: node(5),
                    result: ValueId(0),
                    lhs: ValueId(1),
                    rhs: ValueId(1),
                }],
                terminator: Terminator::Return {
                    origin: node(6),
                    value: ValueId(0),
                },
            });
        });
    }
    #[test]
    fn verifier_rejects_instruction_and_call_contract_malformations() {
        rejects(|p| {
            p.functions[0].blocks[0]
                .instructions
                .push(Instruction::AddI64 {
                    origin: node(4),
                    result: ValueId(0),
                    lhs: ValueId(0),
                    rhs: ValueId(0),
                });
        });
        rejects(|p| {
            p.functions[0].blocks[0]
                .instructions
                .push(Instruction::Call {
                    origin: node(4),
                    result: ValueId(0),
                    function: FunctionId(2),
                    arguments: vec![ValueId(0)],
                });
        });
        rejects(|p| {
            p.functions[0].value_types[0] = SemanticType::Bool;
        });
    }
    #[test]
    fn verifier_rejects_call_arity_argument_and_result_types() {
        let callee = CoreFunction {
            origin: node(10),
            parameters: vec![ValueId(0)],
            result: SemanticType::I64,
            value_types: vec![SemanticType::Bool, SemanticType::I64],
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(11),
                parameters: vec![ValueId(0)],
                instructions: vec![Instruction::ConstI64 {
                    origin: node(12),
                    result: ValueId(1),
                    value: 1,
                }],
                terminator: Terminator::Return {
                    origin: node(13),
                    value: ValueId(1),
                },
            }],
        };
        let caller = CoreFunction {
            origin: node(1),
            parameters: vec![ValueId(0)],
            result: SemanticType::I64,
            value_types: vec![SemanticType::I64, SemanticType::I64],
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(2),
                parameters: vec![ValueId(0)],
                instructions: vec![Instruction::Call {
                    origin: node(3),
                    result: ValueId(1),
                    function: FunctionId(1),
                    arguments: vec![ValueId(0)],
                }],
                terminator: Terminator::Return {
                    origin: node(4),
                    value: ValueId(1),
                },
            }],
        };
        let base = CoreProgram {
            functions: vec![caller, callee],
            entry: FunctionId(0),
        };
        let mut arity = base.clone();
        let Instruction::Call { arguments, .. } = &mut arity.functions[0].blocks[0].instructions[0]
        else {
            unreachable!()
        };
        arguments.clear();
        assert_eq!(
            verify(&arity).expect_err("call arity").code,
            ErrorCode::CoreIrInvalid
        );
        assert_eq!(
            verify(&base).expect_err("call argument type").code,
            ErrorCode::CoreIrInvalid
        );
        let mut result = base;
        result.functions[0].value_types[1] = SemanticType::Bool;
        result.functions[1].value_types[0] = SemanticType::I64;
        assert_eq!(
            verify(&result).expect_err("call result type").code,
            ErrorCode::CoreIrInvalid
        );
    }

    #[test]
    fn verifier_rejects_cross_block_values_and_branch_argument_types() {
        let cross = CoreProgram {
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: vec![ValueId(0)],
                result: SemanticType::I64,
                value_types: vec![SemanticType::I64, SemanticType::I64, SemanticType::I64],
                entry: BlockId(0),
                blocks: vec![
                    CoreBlock {
                        origin: node(2),
                        parameters: vec![ValueId(0)],
                        instructions: vec![],
                        terminator: Terminator::Branch {
                            origin: node(3),
                            target: BlockId(1),
                            arguments: vec![ValueId(0)],
                        },
                    },
                    CoreBlock {
                        origin: node(4),
                        parameters: vec![ValueId(1)],
                        instructions: vec![Instruction::AddI64 {
                            origin: node(5),
                            result: ValueId(2),
                            lhs: ValueId(0),
                            rhs: ValueId(1),
                        }],
                        terminator: Terminator::Return {
                            origin: node(6),
                            value: ValueId(2),
                        },
                    },
                ],
            }],
        };
        assert_eq!(
            verify(&cross).expect_err("cross block").code,
            ErrorCode::CoreIrInvalid
        );
        let mut branch_type = cross;
        branch_type.functions[0].value_types[1] = SemanticType::Bool;
        branch_type.functions[0].blocks[1].instructions.clear();
        branch_type.functions[0].blocks[1].terminator = Terminator::Return {
            origin: node(6),
            value: ValueId(1),
        };
        assert_eq!(
            verify(&branch_type).expect_err("branch type").code,
            ErrorCode::CoreIrInvalid
        );
    }

    #[test]
    fn verifier_rejects_branch_argument_and_condition_malformations() {
        rejects(|p| {
            p.functions[0].blocks[0].terminator = Terminator::Branch {
                origin: node(3),
                target: BlockId(0),
                arguments: vec![],
            };
        });
        rejects(|p| {
            p.functions[0].blocks[0].terminator = Terminator::CondBranch {
                origin: node(3),
                condition: ValueId(0),
                then_target: BlockId(0),
                then_arguments: vec![ValueId(0)],
                else_target: BlockId(0),
                else_arguments: vec![ValueId(0)],
            };
        });
    }

    #[test]
    fn verifier_isolates_parameter_range_repetition_and_reuse_rules() {
        let mut function_range = valid();
        function_range.functions[0].parameters = vec![ValueId(1)];
        function_range.functions[0].blocks[0].parameters = vec![ValueId(1)];
        assert_invalid(&function_range, "function parameter value is out of bounds");

        let mut block_range = valid();
        block_range.functions[0].blocks.push(CoreBlock {
            origin: node(4),
            parameters: vec![ValueId(1)],
            instructions: vec![],
            terminator: Terminator::Return {
                origin: node(5),
                value: ValueId(1),
            },
        });
        assert_invalid(&block_range, "block parameter value is out of bounds");

        let mut duplicate_function = valid();
        duplicate_function.functions[0].parameters = vec![ValueId(0), ValueId(0)];
        duplicate_function.functions[0].blocks[0].parameters = vec![ValueId(0), ValueId(0)];
        assert_invalid(
            &duplicate_function,
            "Core IR value is defined more than once",
        );

        let mut repeated_block = valid();
        repeated_block.functions[0]
            .value_types
            .push(SemanticType::I64);
        repeated_block.functions[0].blocks.push(CoreBlock {
            origin: node(4),
            parameters: vec![ValueId(1), ValueId(1)],
            instructions: vec![],
            terminator: Terminator::Return {
                origin: node(5),
                value: ValueId(1),
            },
        });
        assert_invalid(&repeated_block, "block parameter is repeated");

        let mut reused_block = valid();
        reused_block.functions[0]
            .value_types
            .push(SemanticType::I64);
        for serial in [4, 6] {
            reused_block.functions[0].blocks.push(CoreBlock {
                origin: node(serial),
                parameters: vec![ValueId(1)],
                instructions: vec![],
                terminator: Terminator::Return {
                    origin: node(serial + 1),
                    value: ValueId(1),
                },
            });
        }
        assert_invalid(&reused_block, "Core IR value is defined more than once");
    }

    #[test]
    fn verifier_isolates_constant_and_instruction_result_contracts() {
        for (instruction, ty) in [
            (
                Instruction::ConstUnit {
                    origin: node(3),
                    result: ValueId(0),
                },
                SemanticType::I64,
            ),
            (
                Instruction::ConstBool {
                    origin: node(3),
                    result: ValueId(0),
                    value: true,
                },
                SemanticType::I64,
            ),
            (
                Instruction::ConstI64 {
                    origin: node(3),
                    result: ValueId(0),
                    value: 1,
                },
                SemanticType::Bool,
            ),
        ] {
            let error = verify(&constant(instruction, ty)).expect_err("constant result type");
            assert_eq!(error.code, ErrorCode::CoreIrInvalid);
            assert_eq!(
                error.message,
                "instruction result type disagrees with its contract"
            );
        }
        let out = constant(
            Instruction::ConstI64 {
                origin: node(3),
                result: ValueId(1),
                value: 1,
            },
            SemanticType::I64,
        );
        assert_invalid(&out, "defined value is out of bounds");

        let mut duplicate = constant(
            Instruction::ConstI64 {
                origin: node(3),
                result: ValueId(0),
                value: 1,
            },
            SemanticType::I64,
        );
        duplicate.functions[0].blocks[0]
            .instructions
            .push(Instruction::ConstI64 {
                origin: node(4),
                result: ValueId(0),
                value: 2,
            });
        assert_invalid(&duplicate, "Core IR value is defined more than once");

        let mut across_blocks = duplicate;
        across_blocks.functions[0].blocks[0]
            .instructions
            .truncate(1);
        across_blocks.functions[0].blocks.push(CoreBlock {
            origin: node(5),
            parameters: vec![],
            instructions: vec![Instruction::ConstI64 {
                origin: node(6),
                result: ValueId(0),
                value: 2,
            }],
            terminator: Terminator::Return {
                origin: node(7),
                value: ValueId(0),
            },
        });
        assert_invalid(&across_blocks, "Core IR value is defined more than once");
    }

    #[test]
    fn verifier_isolates_add_and_lt_operand_and_result_contracts() {
        let arithmetic = |last: Instruction, types: Vec<SemanticType>| {
            let definition = |origin, result, ty| match ty {
                SemanticType::Unit => Instruction::ConstUnit { origin, result },
                SemanticType::Bool => Instruction::ConstBool {
                    origin,
                    result,
                    value: true,
                },
                SemanticType::I64 => Instruction::ConstI64 {
                    origin,
                    result,
                    value: 1,
                },
            };
            CoreProgram {
                entry: FunctionId(0),
                functions: vec![CoreFunction {
                    origin: node(1),
                    parameters: vec![],
                    result: *types.last().expect("result type"),
                    value_types: types.clone(),
                    entry: BlockId(0),
                    blocks: vec![CoreBlock {
                        origin: node(2),
                        parameters: vec![],
                        instructions: vec![
                            definition(node(3), ValueId(0), types[0]),
                            definition(node(4), ValueId(1), types[1]),
                            last,
                        ],
                        terminator: Terminator::Return {
                            origin: node(6),
                            value: ValueId(2),
                        },
                    }],
                }],
            }
        };
        let add_out = arithmetic(
            Instruction::AddI64 {
                origin: node(5),
                result: ValueId(2),
                lhs: ValueId(9),
                rhs: ValueId(1),
            },
            vec![SemanticType::I64; 3],
        );
        assert_invalid(&add_out, "operand value is out of bounds");
        let add_type = arithmetic(
            Instruction::AddI64 {
                origin: node(5),
                result: ValueId(2),
                lhs: ValueId(0),
                rhs: ValueId(1),
            },
            vec![SemanticType::Bool, SemanticType::I64, SemanticType::I64],
        );
        assert_invalid(
            &add_type,
            "Core IR operand type disagrees with its contract",
        );
        let add_result = arithmetic(
            Instruction::AddI64 {
                origin: node(5),
                result: ValueId(2),
                lhs: ValueId(0),
                rhs: ValueId(1),
            },
            vec![SemanticType::I64, SemanticType::I64, SemanticType::Bool],
        );
        assert_invalid(
            &add_result,
            "instruction result type disagrees with its contract",
        );
        let lt_type = arithmetic(
            Instruction::LtI64 {
                origin: node(5),
                result: ValueId(2),
                lhs: ValueId(0),
                rhs: ValueId(1),
            },
            vec![SemanticType::Bool, SemanticType::I64, SemanticType::Bool],
        );
        assert_invalid(&lt_type, "Core IR operand type disagrees with its contract");
        let lt_result = arithmetic(
            Instruction::LtI64 {
                origin: node(5),
                result: ValueId(2),
                lhs: ValueId(0),
                rhs: ValueId(1),
            },
            vec![SemanticType::I64, SemanticType::I64, SemanticType::I64],
        );
        assert_invalid(
            &lt_result,
            "instruction result type disagrees with its contract",
        );
    }

    #[test]
    fn verifier_isolates_call_target_argument_and_result_rules() {
        let mut target = call_program();
        let Instruction::Call { function, .. } = &mut target.functions[0].blocks[0].instructions[0]
        else {
            unreachable!()
        };
        *function = FunctionId(9);
        assert_invalid(&target, "call target function is out of bounds");

        let mut arity = call_program();
        let Instruction::Call { arguments, .. } = &mut arity.functions[0].blocks[0].instructions[0]
        else {
            unreachable!()
        };
        arguments.clear();
        assert_invalid(
            &arity,
            "call argument count disagrees with callee parameters",
        );

        let mut out = call_program();
        let Instruction::Call { arguments, .. } = &mut out.functions[0].blocks[0].instructions[0]
        else {
            unreachable!()
        };
        arguments[0] = ValueId(9);
        assert_invalid(&out, "operand value is out of bounds");

        let mut ty = call_program();
        ty.functions[0].value_types[0] = SemanticType::Bool;
        assert_invalid(&ty, "Core IR operand type disagrees with its contract");

        let mut locality = call_program();
        locality.functions[0].value_types.push(SemanticType::I64);
        locality.functions[0].blocks.push(CoreBlock {
            origin: node(5),
            parameters: vec![ValueId(2)],
            instructions: vec![],
            terminator: Terminator::Return {
                origin: node(6),
                value: ValueId(2),
            },
        });
        let Instruction::Call { arguments, .. } =
            &mut locality.functions[0].blocks[0].instructions[0]
        else {
            unreachable!()
        };
        arguments[0] = ValueId(2);
        assert_invalid(&locality, "Core IR operand is not available in this block");

        let mut result = call_program();
        result.functions[0].value_types[1] = SemanticType::Bool;
        assert_invalid(
            &result,
            "instruction result type disagrees with its contract",
        );
    }

    #[test]
    fn verifier_isolates_return_index_locality_and_type_rules() {
        let mut out = valid();
        out.functions[0].blocks[0].terminator = Terminator::Return {
            origin: node(3),
            value: ValueId(9),
        };
        assert_invalid(&out, "operand value is out of bounds");

        let mut locality = valid();
        locality.functions[0].value_types.push(SemanticType::I64);
        locality.functions[0].blocks[0].terminator = Terminator::Return {
            origin: node(3),
            value: ValueId(1),
        };
        locality.functions[0].blocks.push(CoreBlock {
            origin: node(4),
            parameters: vec![ValueId(1)],
            instructions: vec![],
            terminator: Terminator::Return {
                origin: node(5),
                value: ValueId(1),
            },
        });
        assert_invalid(&locality, "Core IR operand is not available in this block");

        let mut ty = valid();
        ty.functions[0].value_types[0] = SemanticType::Bool;
        assert_invalid(&ty, "Core IR operand type disagrees with its contract");
    }

    #[test]
    fn verifier_isolates_branch_and_each_conditional_edge_rule() {
        let mut branch = conditional();
        branch.functions[0].blocks[0].terminator = Terminator::Branch {
            origin: node(3),
            target: BlockId(9),
            arguments: vec![],
        };
        assert_invalid(&branch, "branch target block is out of bounds");
        let mut branch = conditional();
        branch.functions[0].blocks[0].terminator = Terminator::Branch {
            origin: node(3),
            target: BlockId(1),
            arguments: vec![],
        };
        assert_invalid(
            &branch,
            "branch argument count disagrees with target block parameters",
        );
        let mut branch = conditional();
        branch.functions[0].blocks[0].terminator = Terminator::Branch {
            origin: node(3),
            target: BlockId(1),
            arguments: vec![ValueId(9)],
        };
        assert_invalid(&branch, "operand value is out of bounds");
        let mut branch = conditional();
        branch.functions[0].blocks[0].terminator = Terminator::Branch {
            origin: node(3),
            target: BlockId(1),
            arguments: vec![ValueId(2)],
        };
        assert_invalid(&branch, "Core IR operand is not available in this block");
        let mut branch = conditional();
        branch.functions[0].blocks[0].terminator = Terminator::Branch {
            origin: node(3),
            target: BlockId(1),
            arguments: vec![ValueId(0)],
        };
        assert_invalid(&branch, "Core IR operand type disagrees with its contract");

        for then_edge in [true, false] {
            let mutate = |program: &mut CoreProgram,
                          target: Option<BlockId>,
                          arguments: Option<Vec<ValueId>>| {
                let Terminator::CondBranch {
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                    ..
                } = &mut program.functions[0].blocks[0].terminator
                else {
                    unreachable!()
                };
                if then_edge {
                    if let Some(value) = target {
                        *then_target = value;
                    }
                    if let Some(value) = arguments {
                        *then_arguments = value;
                    }
                } else {
                    if let Some(value) = target {
                        *else_target = value;
                    }
                    if let Some(value) = arguments {
                        *else_arguments = value;
                    }
                }
            };
            let mut target = conditional();
            mutate(&mut target, Some(BlockId(9)), None);
            assert_invalid(&target, "branch target block is out of bounds");
            let mut arity = conditional();
            mutate(&mut arity, None, Some(vec![]));
            assert_invalid(
                &arity,
                "branch argument count disagrees with target block parameters",
            );
            let mut out = conditional();
            mutate(&mut out, None, Some(vec![ValueId(9)]));
            assert_invalid(&out, "operand value is out of bounds");
            let mut locality = conditional();
            mutate(&mut locality, None, Some(vec![ValueId(2)]));
            assert_invalid(&locality, "Core IR operand is not available in this block");
            let mut ty = conditional();
            mutate(&mut ty, None, Some(vec![ValueId(0)]));
            assert_invalid(&ty, "Core IR operand type disagrees with its contract");
        }
    }

    #[test]
    fn verifier_isolates_conditional_condition_rules() {
        let mut out = conditional();
        let Terminator::CondBranch { condition, .. } = &mut out.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *condition = ValueId(9);
        assert_invalid(&out, "operand value is out of bounds");
        let mut locality = conditional();
        let Terminator::CondBranch { condition, .. } =
            &mut locality.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *condition = ValueId(2);
        assert_invalid(&locality, "Core IR operand is not available in this block");
        let mut ty = conditional();
        let Terminator::CondBranch { condition, .. } = &mut ty.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *condition = ValueId(1);
        assert_invalid(&ty, "Core IR operand type disagrees with its contract");
    }
}
