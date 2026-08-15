use crate::compile;
use crate::core_ir::{self, BlockId, CoreProgram, FunctionId, Instruction, Terminator, ValueId};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::schema::{Node, SemanticType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const MAX_RUN_ARGUMENTS: usize = 1_024;
pub const MAX_RUN_FUEL: u64 = 10_000_000;
pub const MAX_RUN_FRAMES: u32 = 100_000;
pub const MAX_RUN_LIVE_VALUE_SLOTS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeValue {
    Unit,
    Bool(bool),
    I64(i64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeValueCode {
    Unit,
    Bool,
    I64,
}

impl RuntimeValueCode {
    pub const ALL: [Self; 3] = [Self::Unit, Self::Bool, Self::I64];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::Unit => 1,
            Self::Bool => 2,
            Self::I64 => 3,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Unit),
            2 => Some(Self::Bool),
            3 => Some(Self::I64),
            _ => None,
        }
    }
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
        }
    }
}

impl RuntimeValue {
    pub(crate) const fn code(self) -> RuntimeValueCode {
        match self {
            Self::Unit => RuntimeValueCode::Unit,
            Self::Bool(_) => RuntimeValueCode::Bool,
            Self::I64(_) => RuntimeValueCode::I64,
        }
    }
    const fn semantic_type(self) -> SemanticType {
        match self {
            Self::Unit => SemanticType::Unit,
            Self::Bool(_) => SemanticType::Bool,
            Self::I64(_) => SemanticType::I64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPolicy {
    pub fuel: u64,
    pub maximum_frames: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub value: RuntimeValue,
    pub compile_nanoseconds: u64,
    pub execute_nanoseconds: u64,
}

pub(crate) fn compile_and_run(
    snapshot: &Snapshot,
    entry: NodeId,
    arguments: &[RuntimeValue],
    policy: RunPolicy,
) -> Result<RunResult> {
    validate_policy(policy)?;
    validate_invocation(snapshot, entry, arguments)?;
    let compile_started = Instant::now();
    let program = compile::compile(snapshot, entry)?;
    let compile_nanoseconds = nanos(compile_started.elapsed().as_nanos());
    let execute_started = Instant::now();
    let value = interpret(&program, arguments, policy)?;
    let execute_nanoseconds = nanos(execute_started.elapsed().as_nanos());
    Ok(RunResult {
        value,
        compile_nanoseconds,
        execute_nanoseconds,
    })
}

fn validate_policy(policy: RunPolicy) -> Result<()> {
    if policy.fuel == 0 || policy.fuel > MAX_RUN_FUEL {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "run fuel must be positive and within the runtime policy",
        ));
    }
    if policy.maximum_frames == 0 || policy.maximum_frames > MAX_RUN_FRAMES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "run maximum_frames must be positive and within the runtime policy",
        ));
    }
    Ok(())
}

fn validate_invocation(
    snapshot: &Snapshot,
    entry: NodeId,
    arguments: &[RuntimeValue],
) -> Result<()> {
    if arguments.len() > MAX_RUN_ARGUMENTS {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "run argument count exceeds the invocation boundary",
        )
        .for_node(entry));
    }
    let Node::Function { parameters, .. } = snapshot.node(entry)? else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "run entry must be a function").for_node(entry),
        );
    };
    if arguments.len() != parameters.len() {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "run argument count disagrees with entry parameters",
        )
        .for_node(entry)
        .with_related(parameters.iter().copied()));
    }
    for (argument, parameter) in arguments.iter().zip(parameters) {
        let Node::Parameter { ty, .. } = snapshot.node(*parameter)? else {
            return Err(LkError::new(
                ErrorCode::CoreIrInvalid,
                "entry parameter slot is not a parameter",
            )
            .for_node(*parameter));
        };
        let actual = argument.semantic_type();
        if actual != *ty {
            return Err(LkError::new(
                ErrorCode::RunArgumentMismatch,
                "run argument type disagrees with entry parameter",
            )
            .for_node(*parameter)
            .with_types(*ty, actual));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Continuation {
    result: ValueId,
}
struct Frame {
    function: FunctionId,
    block: BlockId,
    instruction: usize,
    values: Vec<Option<RuntimeValue>>,
    continuation: Option<Continuation>,
}

fn interpret(
    program: &CoreProgram,
    arguments: &[RuntimeValue],
    policy: RunPolicy,
) -> Result<RuntimeValue> {
    core_ir::verify(program)?;
    validate_policy(policy)?;
    let mut fuel = policy.fuel;
    let entry_origin = program
        .functions
        .get(index(program.entry.0, "entry function")?)
        .ok_or_else(|| invalid_ir("entry function is out of bounds"))?
        .origin;
    let mut live_value_slots = function_value_slots(program, program.entry)?;
    if live_value_slots > MAX_RUN_LIVE_VALUE_SLOTS {
        return Err(LkError::new(
            ErrorCode::ExecutionFrameExhausted,
            "execution live frame-value-slot policy exhausted before entry",
        )
        .for_node(entry_origin));
    }
    let mut frames = vec![new_frame(program, program.entry, arguments, None)?];
    loop {
        let frame_index = frames.len().checked_sub(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::CoreIrInvalid,
                "interpreter frame stack became empty",
            )
        })?;
        let function_id = frames[frame_index].function;
        let block_id = frames[frame_index].block;
        let function = program
            .functions
            .get(index(function_id.0, "function")?)
            .ok_or_else(|| invalid_ir("runtime function is out of bounds"))?;
        let block = function
            .blocks
            .get(index(block_id.0, "block")?)
            .ok_or_else(|| invalid_ir("runtime block is out of bounds"))?;
        if frames[frame_index].instruction < block.instructions.len() {
            let instruction = block.instructions[frames[frame_index].instruction].clone();
            consume_fuel(&mut fuel, instruction.origin())?;
            match instruction {
                Instruction::ConstUnit { result, .. } => {
                    write_value(&mut frames[frame_index], result, RuntimeValue::Unit)?
                }
                Instruction::ConstBool { result, value, .. } => {
                    write_value(&mut frames[frame_index], result, RuntimeValue::Bool(value))?
                }
                Instruction::ConstI64 { result, value, .. } => {
                    write_value(&mut frames[frame_index], result, RuntimeValue::I64(value))?
                }
                Instruction::AddI64 {
                    origin,
                    result,
                    lhs,
                    rhs,
                } => {
                    let lhs = require_i64(&frames[frame_index], lhs)?;
                    let rhs = require_i64(&frames[frame_index], rhs)?;
                    let value = lhs.checked_add(rhs).ok_or_else(|| {
                        LkError::new(ErrorCode::RuntimeTrap, "i64 addition overflowed")
                            .for_node(origin)
                    })?;
                    write_value(&mut frames[frame_index], result, RuntimeValue::I64(value))?;
                }
                Instruction::LtI64 {
                    result, lhs, rhs, ..
                } => {
                    let value = require_i64(&frames[frame_index], lhs)?
                        < require_i64(&frames[frame_index], rhs)?;
                    write_value(&mut frames[frame_index], result, RuntimeValue::Bool(value))?;
                }
                Instruction::Call {
                    origin,
                    result,
                    function: callee,
                    arguments,
                } => {
                    let values = arguments
                        .iter()
                        .map(|value| read_value(&frames[frame_index], *value))
                        .collect::<Result<Vec<_>>>()?;
                    frames[frame_index].instruction += 1;
                    if frames.len()
                        >= usize::try_from(policy.maximum_frames)
                            .map_err(|_| invalid_ir("frame policy overflows host indexes"))?
                    {
                        return Err(LkError::new(
                            ErrorCode::ExecutionFrameExhausted,
                            "execution frame policy exhausted before call",
                        )
                        .for_node(origin));
                    }
                    let callee_slots = function_value_slots(program, callee)?;
                    if live_value_slots
                        .checked_add(callee_slots)
                        .is_none_or(|total| total > MAX_RUN_LIVE_VALUE_SLOTS)
                    {
                        return Err(LkError::new(
                            ErrorCode::ExecutionFrameExhausted,
                            "execution live frame-value-slot policy exhausted before call",
                        )
                        .for_node(origin));
                    }
                    frames.push(new_frame(
                        program,
                        callee,
                        &values,
                        Some(Continuation { result }),
                    )?);
                    live_value_slots += callee_slots;
                    continue;
                }
            }
            frames[frame_index].instruction += 1;
            continue;
        }
        let terminator = block.terminator.clone();
        consume_fuel(&mut fuel, terminator.origin())?;
        match terminator {
            Terminator::Return { value, .. } => {
                let returned = read_value(&frames[frame_index], value)?;
                let continuation = frames[frame_index].continuation;
                let released_slots = frames[frame_index].values.len();
                frames.pop();
                live_value_slots = live_value_slots
                    .checked_sub(released_slots)
                    .ok_or_else(|| invalid_ir("live frame-value-slot accounting underflow"))?;
                if let Some(continuation) = continuation {
                    let caller = frames
                        .last_mut()
                        .ok_or_else(|| invalid_ir("call return has no caller frame"))?;
                    write_value(caller, continuation.result, returned)?;
                } else {
                    if !frames.is_empty() {
                        return Err(invalid_ir("entry return left unexpected frames"));
                    }
                    return Ok(returned);
                }
            }
            Terminator::Branch {
                target, arguments, ..
            } => {
                let values = arguments
                    .iter()
                    .map(|value| read_value(&frames[frame_index], *value))
                    .collect::<Result<Vec<_>>>()?;
                enter_block(program, &mut frames[frame_index], target, &values)?;
            }
            Terminator::CondBranch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                let condition = require_bool(&frames[frame_index], condition)?;
                let (target, arguments) = if condition {
                    (then_target, then_arguments)
                } else {
                    (else_target, else_arguments)
                };
                let values = arguments
                    .iter()
                    .map(|value| read_value(&frames[frame_index], *value))
                    .collect::<Result<Vec<_>>>()?;
                enter_block(program, &mut frames[frame_index], target, &values)?;
            }
        }
    }
}

fn function_value_slots(program: &CoreProgram, function_id: FunctionId) -> Result<usize> {
    Ok(program
        .functions
        .get(index(function_id.0, "function")?)
        .ok_or_else(|| invalid_ir("runtime function is out of bounds"))?
        .value_types
        .len())
}

fn new_frame(
    program: &CoreProgram,
    function_id: FunctionId,
    arguments: &[RuntimeValue],
    continuation: Option<Continuation>,
) -> Result<Frame> {
    let function = program
        .functions
        .get(index(function_id.0, "function")?)
        .ok_or_else(|| invalid_ir("callee function is out of bounds"))?;
    if arguments.len() != function.parameters.len() {
        return Err(invalid_ir(
            "runtime call argument count disagrees with verified function",
        ));
    }
    for (parameter, argument) in function.parameters.iter().zip(arguments) {
        let expected = function
            .value_types
            .get(index(parameter.0, "parameter value")?)
            .copied()
            .ok_or_else(|| invalid_ir("runtime parameter value is out of bounds"))?;
        if argument.semantic_type() != expected {
            return Err(invalid_ir(
                "runtime argument type disagrees with Core parameter type",
            ));
        }
    }
    let mut frame = Frame {
        function: function_id,
        block: function.entry,
        instruction: 0,
        values: vec![None; function.value_types.len()],
        continuation,
    };
    bind_parameters(&mut frame, &function.parameters, arguments)?;
    Ok(frame)
}
fn enter_block(
    program: &CoreProgram,
    frame: &mut Frame,
    target: BlockId,
    arguments: &[RuntimeValue],
) -> Result<()> {
    let function = program
        .functions
        .get(index(frame.function.0, "function")?)
        .ok_or_else(|| invalid_ir("runtime function is out of bounds"))?;
    let block = function
        .blocks
        .get(index(target.0, "block")?)
        .ok_or_else(|| invalid_ir("branch target is out of bounds"))?;
    if arguments.len() != block.parameters.len() {
        return Err(invalid_ir(
            "runtime branch argument count disagrees with verified block",
        ));
    }
    frame.values.fill(None);
    frame.block = target;
    frame.instruction = 0;
    bind_parameters(frame, &block.parameters, arguments)
}
fn bind_parameters(
    frame: &mut Frame,
    parameters: &[ValueId],
    arguments: &[RuntimeValue],
) -> Result<()> {
    for (parameter, argument) in parameters.iter().zip(arguments) {
        write_value(frame, *parameter, *argument)?;
    }
    Ok(())
}
fn write_value(frame: &mut Frame, id: ValueId, value: RuntimeValue) -> Result<()> {
    let slot = frame
        .values
        .get_mut(index(id.0, "value")?)
        .ok_or_else(|| invalid_ir("runtime result value is out of bounds"))?;
    if slot.replace(value).is_some() {
        return Err(invalid_ir("runtime value was defined twice in one block"));
    }
    Ok(())
}
fn read_value(frame: &Frame, id: ValueId) -> Result<RuntimeValue> {
    frame
        .values
        .get(index(id.0, "value")?)
        .copied()
        .flatten()
        .ok_or_else(|| invalid_ir("runtime operand is unavailable in this block"))
}
fn require_i64(frame: &Frame, id: ValueId) -> Result<i64> {
    match read_value(frame, id)? {
        RuntimeValue::I64(value) => Ok(value),
        _ => Err(invalid_ir(
            "verified i64 value has a non-i64 runtime representation",
        )),
    }
}
fn require_bool(frame: &Frame, id: ValueId) -> Result<bool> {
    match read_value(frame, id)? {
        RuntimeValue::Bool(value) => Ok(value),
        _ => Err(invalid_ir(
            "verified bool value has a non-bool runtime representation",
        )),
    }
}
fn consume_fuel(fuel: &mut u64, origin: NodeId) -> Result<()> {
    if *fuel == 0 {
        return Err(LkError::new(
            ErrorCode::ExecutionFuelExhausted,
            "execution fuel exhausted",
        )
        .for_node(origin));
    }
    *fuel -= 1;
    Ok(())
}
fn index(value: u32, category: &str) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| invalid_ir(format!("runtime {category} index overflows host indexes")))
}
fn invalid_ir(message: impl Into<String>) -> LkError {
    LkError::new(ErrorCode::CoreIrInvalid, message)
}
fn nanos(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_ir::{CoreBlock, CoreFunction};
    use crate::ids::WorkspaceId;
    fn node(serial: u64) -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([0x51; 16]), serial).expect("node")
    }
    fn policy() -> RunPolicy {
        RunPolicy {
            fuel: 100,
            maximum_frames: 10,
        }
    }

    #[test]
    fn explicit_frames_execute_calls_and_remain_usable_after_trap() {
        let callee = CoreFunction {
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
        };
        let caller = CoreFunction {
            origin: node(1),
            parameters: vec![],
            result: SemanticType::I64,
            value_types: vec![SemanticType::I64, SemanticType::I64],
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(2),
                parameters: vec![],
                instructions: vec![
                    Instruction::ConstI64 {
                        origin: node(3),
                        result: ValueId(0),
                        value: 7,
                    },
                    Instruction::Call {
                        origin: node(4),
                        result: ValueId(1),
                        function: FunctionId(1),
                        arguments: vec![ValueId(0)],
                    },
                ],
                terminator: Terminator::Return {
                    origin: node(5),
                    value: ValueId(1),
                },
            }],
        };
        let program = CoreProgram {
            functions: vec![caller, callee],
            entry: FunctionId(0),
        };
        assert_eq!(
            interpret(&program, &[], policy()).expect("call"),
            RuntimeValue::I64(7)
        );
        let exhausted = interpret(
            &program,
            &[],
            RunPolicy {
                fuel: 1,
                maximum_frames: 10,
            },
        )
        .expect_err("fuel");
        assert_eq!(exhausted.code, ErrorCode::ExecutionFuelExhausted);
        assert_eq!(
            interpret(&program, &[], policy()).expect("later call"),
            RuntimeValue::I64(7)
        );
    }

    #[test]
    fn live_frame_value_slots_exhaust_before_allocation_and_later_execution_is_usable() {
        let broad_count = MAX_RUN_LIVE_VALUE_SLOTS + 1;
        let broad_parameters = (0..broad_count)
            .map(|index| ValueId(u32::try_from(index).expect("broad parameter")))
            .collect::<Vec<_>>();
        let broad_entry = CoreProgram {
            functions: vec![CoreFunction {
                origin: node(10),
                parameters: broad_parameters.clone(),
                result: SemanticType::Unit,
                value_types: vec![SemanticType::Unit; broad_count],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(11),
                    parameters: broad_parameters,
                    instructions: Vec::new(),
                    terminator: Terminator::Return {
                        origin: node(12),
                        value: ValueId(0),
                    },
                }],
            }],
            entry: FunctionId(0),
        };
        let entry_exhausted = interpret(
            &broad_entry,
            &vec![RuntimeValue::Unit; broad_count],
            RunPolicy {
                fuel: MAX_RUN_FUEL,
                maximum_frames: MAX_RUN_FRAMES,
            },
        )
        .expect_err("entry live value slots");
        assert_eq!(entry_exhausted.code, ErrorCode::ExecutionFrameExhausted);
        assert_eq!(entry_exhausted.target, Some(node(10)));

        const VALUES_PER_FRAME: usize = 1_025;
        let mut instructions = (0..VALUES_PER_FRAME - 1)
            .map(|index| Instruction::ConstUnit {
                origin: node(20),
                result: ValueId(u32::try_from(index).expect("value")),
            })
            .collect::<Vec<_>>();
        instructions.push(Instruction::Call {
            origin: node(21),
            result: ValueId(u32::try_from(VALUES_PER_FRAME - 1).expect("call result")),
            function: FunctionId(0),
            arguments: Vec::new(),
        });
        let recursive = CoreProgram {
            functions: vec![CoreFunction {
                origin: node(19),
                parameters: Vec::new(),
                result: SemanticType::Unit,
                value_types: vec![SemanticType::Unit; VALUES_PER_FRAME],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(20),
                    parameters: Vec::new(),
                    instructions,
                    terminator: Terminator::Return {
                        origin: node(22),
                        value: ValueId(u32::try_from(VALUES_PER_FRAME - 1).expect("return")),
                    },
                }],
            }],
            entry: FunctionId(0),
        };
        let exhausted = interpret(
            &recursive,
            &[],
            RunPolicy {
                fuel: MAX_RUN_FUEL,
                maximum_frames: MAX_RUN_FRAMES,
            },
        )
        .expect_err("live value slots");
        assert_eq!(exhausted.code, ErrorCode::ExecutionFrameExhausted);
        assert_eq!(exhausted.target, Some(node(21)));
        assert!(exhausted.message.contains("frame-value-slot"));

        let later = CoreProgram {
            functions: vec![CoreFunction {
                origin: node(30),
                parameters: Vec::new(),
                result: SemanticType::Unit,
                value_types: vec![SemanticType::Unit],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(31),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::ConstUnit {
                        origin: node(32),
                        result: ValueId(0),
                    }],
                    terminator: Terminator::Return {
                        origin: node(33),
                        value: ValueId(0),
                    },
                }],
            }],
            entry: FunctionId(0),
        };
        assert_eq!(
            interpret(&later, &[], policy()).expect("later run"),
            RuntimeValue::Unit
        );
    }

    #[test]
    fn interpreter_verifies_before_execution_and_rechecks_core_argument_types() {
        let mut malformed = CoreProgram {
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: vec![],
                result: SemanticType::I64,
                value_types: vec![SemanticType::I64],
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: vec![],
                    instructions: vec![],
                    terminator: Terminator::Return {
                        origin: node(3),
                        value: ValueId(0),
                    },
                }],
            }],
            entry: FunctionId(0),
        };
        let error = interpret(
            &malformed,
            &[],
            RunPolicy {
                fuel: 0,
                maximum_frames: 0,
            },
        )
        .expect_err("verification precedes policy and execution");
        assert_eq!(error.code, ErrorCode::CoreIrInvalid);
        assert_eq!(
            error.message,
            "Core IR operand is not available in this block"
        );

        malformed.functions[0].parameters = vec![ValueId(0)];
        malformed.functions[0].blocks[0].parameters = vec![ValueId(0)];
        let error = interpret(&malformed, &[RuntimeValue::Bool(true)], policy())
            .expect_err("private Core argument type check");
        assert_eq!(error.code, ErrorCode::CoreIrInvalid);
        assert_eq!(
            error.message,
            "runtime argument type disagrees with Core parameter type"
        );
    }

    #[test]
    fn checked_add_traps_with_origin() {
        let function = CoreFunction {
            origin: node(1),
            parameters: vec![],
            result: SemanticType::I64,
            value_types: vec![SemanticType::I64, SemanticType::I64, SemanticType::I64],
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(2),
                parameters: vec![],
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
        };
        let error = interpret(
            &CoreProgram {
                functions: vec![function],
                entry: FunctionId(0),
            },
            &[],
            policy(),
        )
        .expect_err("overflow");
        assert_eq!(error.code, ErrorCode::RuntimeTrap);
        assert_eq!(error.target, Some(node(5)));
    }
}
