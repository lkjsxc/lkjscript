use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{
    BlockId, CallTarget, Constant, FunctionId, Instruction, InstructionKind, ProductId, RuntimeOp,
    StructuredOutcome, Terminator, ValueId, VerifiedProgram,
};

#[derive(Clone)]
pub struct EvalBuffer {
    id: u64,
    bytes: Rc<RefCell<Vec<u8>>>,
}

impl fmt::Debug for EvalBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let length = self.bytes.try_borrow().map_or(0, |bytes| bytes.len());
        formatter
            .debug_struct("EvalBuffer")
            .field("id", &self.id)
            .field("length", &length)
            .finish()
    }
}

impl PartialEq for EvalBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub enum EvalValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Symbol(String),
    Buf(EvalBuffer),
    Handle(u64),
    Product(ProductId, Vec<Self>),
    List(Vec<Self>),
    None,
    Some(Box<Self>),
    Ok(Box<Self>),
    Err(Box<Self>),
    Function(FunctionId),
}

impl PartialEq for EvalValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) | (Self::None, Self::None) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::I64(left), Self::I64(right)) => left == right,
            (Self::F64(left), Self::F64(right)) => left.to_bits() == right.to_bits(),
            (Self::Str(left), Self::Str(right)) | (Self::Symbol(left), Self::Symbol(right)) => {
                left == right
            }
            (Self::Buf(left), Self::Buf(right)) => left == right,
            (Self::Handle(left), Self::Handle(right)) => left == right,
            (Self::Product(left_id, left), Self::Product(right_id, right)) => {
                left_id == right_id && left == right
            }
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Some(left), Self::Some(right))
            | (Self::Ok(left), Self::Ok(right))
            | (Self::Err(left), Self::Err(right)) => left == right,
            (Self::Function(left), Self::Function(right)) => left == right,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvalOutcome {
    Returned(EvalValue),
    Exited(i64),
    Trapped(String),
    UnsupportedOperation(RuntimeOp),
    DeadlineExceeded,
    ResourceLimitExceeded(String),
    HostFailure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalConfig {
    pub fuel: u64,
    pub max_frames: usize,
    pub max_allocations: u64,
    pub max_buffer_bytes: usize,
    pub max_list_equal_steps: usize,
    pub args: Vec<String>,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            max_frames: 1_024,
            max_allocations: 1_000_000,
            max_buffer_bytes: 1_000_000,
            max_list_equal_steps: 1_000_000,
            args: Vec::new(),
        }
    }
}

pub fn evaluate(program: &VerifiedProgram, config: &EvalConfig) -> EvalOutcome {
    let mut evaluator = Evaluator {
        program,
        config,
        fuel: config.fuel,
        allocations: 0,
        next_buffer_id: 1,
    };
    match evaluator.call(program.program().main, Vec::new(), 0) {
        Ok(value) => EvalOutcome::Returned(value),
        Err(flow) => flow.outcome(),
    }
}

struct Evaluator<'a> {
    program: &'a VerifiedProgram,
    config: &'a EvalConfig,
    fuel: u64,
    allocations: u64,
    next_buffer_id: u64,
}

#[derive(Debug)]
enum Flow {
    Exit(i64),
    Trap(String),
    Unsupported(RuntimeOp),
    Deadline,
    Resource(String),
    HostFailure(String),
}

impl Flow {
    fn outcome(self) -> EvalOutcome {
        match self {
            Self::Exit(code) => EvalOutcome::Exited(code),
            Self::Trap(message) => EvalOutcome::Trapped(message),
            Self::Unsupported(operation) => EvalOutcome::UnsupportedOperation(operation),
            Self::Deadline => EvalOutcome::DeadlineExceeded,
            Self::Resource(kind) => EvalOutcome::ResourceLimitExceeded(kind),
            Self::HostFailure(message) => EvalOutcome::HostFailure(message),
        }
    }
}

impl Evaluator<'_> {
    fn call(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<EvalValue>,
        depth: usize,
    ) -> std::result::Result<EvalValue, Flow> {
        if depth >= self.config.max_frames {
            return Err(Flow::Resource("frames".into()));
        }
        let function = self
            .program
            .program()
            .functions
            .get(function_id.index().unwrap_or(usize::MAX))
            .filter(|function| function.id == function_id)
            .cloned()
            .ok_or_else(|| Flow::Trap("evaluator missing verified function".into()))?;
        let entry = function
            .blocks
            .iter()
            .find(|block| block.id == function.entry)
            .ok_or_else(|| Flow::Trap("evaluator missing verified entry block".into()))?;
        if arguments.len() != entry.parameters.len() {
            return Err(Flow::Trap("evaluator function arity mismatch".into()));
        }
        let value_count = function
            .blocks
            .iter()
            .flat_map(|block| {
                block
                    .parameters
                    .iter()
                    .map(|parameter| parameter.id)
                    .chain(block.instructions.iter().map(|instruction| instruction.id))
            })
            .filter_map(ValueId::index)
            .max()
            .map_or(Some(0), |maximum| maximum.checked_add(1))
            .ok_or_else(|| Flow::Trap("evaluator function value count overflow".into()))?;
        let mut values = vec![None; value_count];
        assign_parameters(&mut values, &entry.parameters, arguments)?;
        let mut current = function.entry;
        loop {
            self.consume_fuel()?;
            let block = function
                .blocks
                .iter()
                .find(|block| block.id == current)
                .cloned()
                .ok_or_else(|| Flow::Trap("evaluator missing verified block".into()))?;
            for instruction in &block.instructions {
                self.consume_fuel()?;
                let value = self.instruction(instruction, &values, depth)?;
                set_value(&mut values, instruction.id, value)?;
            }
            self.consume_fuel()?;
            match block.terminator {
                Terminator::Branch { target, arguments } => {
                    let arguments = values_for(&values, &arguments)?;
                    let target_block = function
                        .blocks
                        .iter()
                        .find(|block| block.id == target)
                        .ok_or_else(|| Flow::Trap("evaluator branch target is missing".into()))?;
                    assign_parameters(&mut values, &target_block.parameters, arguments)?;
                    current = target;
                }
                Terminator::ConditionalBranch {
                    condition,
                    true_target,
                    true_arguments,
                    false_target,
                    false_arguments,
                } => {
                    let condition = as_bool(value(&values, condition)?)?;
                    let (target, arguments) = if condition {
                        (true_target, true_arguments)
                    } else {
                        (false_target, false_arguments)
                    };
                    let arguments = values_for(&values, &arguments)?;
                    let target_block = function
                        .blocks
                        .iter()
                        .find(|block| block.id == target)
                        .ok_or_else(|| Flow::Trap("evaluator branch target is missing".into()))?;
                    assign_parameters(&mut values, &target_block.parameters, arguments)?;
                    current = target;
                }
                Terminator::Return(result) => return value(&values, result).cloned(),
                Terminator::Trap { message } => return Err(Flow::Trap(message)),
                Terminator::Exit { code } => {
                    return Err(Flow::Exit(as_i64(value(&values, code)?)?))
                }
                Terminator::Outcome { outcome, detail } => {
                    let detail = detail
                        .map(|value_id| as_str(value(&values, value_id)?).map(str::to_owned))
                        .transpose()?
                        .unwrap_or_default();
                    return Err(match outcome {
                        StructuredOutcome::DeadlineExceeded => Flow::Deadline,
                        StructuredOutcome::ResourceLimitExceeded => Flow::Resource(detail),
                        StructuredOutcome::HostFailure => Flow::HostFailure(detail),
                    });
                }
            }
        }
    }

    fn instruction(
        &mut self,
        instruction: &Instruction,
        values: &[Option<EvalValue>],
        depth: usize,
    ) -> std::result::Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::Constant(constant) => self.constant(constant),
            InstructionKind::Copy(source) => value(values, *source).cloned(),
            InstructionKind::FunctionRef(function) => Ok(EvalValue::Function(*function)),
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } => {
                let arguments = values_for(values, arguments)?;
                self.runtime(*operation, arguments)
            }
            InstructionKind::Call {
                target, arguments, ..
            } => {
                let target = match target {
                    CallTarget::Direct(function) => *function,
                    CallTarget::Indirect(target) => match value(values, *target)? {
                        EvalValue::Function(function) => *function,
                        _ => {
                            return Err(Flow::Trap(
                                "evaluator call target is not a function".into(),
                            ))
                        }
                    },
                };
                let arguments = values_for(values, arguments)?;
                self.call(target, arguments, depth.saturating_add(1))
            }
            InstructionKind::ProductValue { product, fields } => {
                self.allocate()?;
                Ok(EvalValue::Product(*product, values_for(values, fields)?))
            }
            InstructionKind::ProductField {
                product,
                field,
                value: product_value,
            } => match value(values, *product_value)? {
                EvalValue::Product(actual, fields) if actual == product => fields
                    .get(usize::from(*field))
                    .cloned()
                    .ok_or_else(|| Flow::Trap("product field out of bounds".into())),
                _ => Err(Flow::Trap("product field identity mismatch".into())),
            },
            InstructionKind::WithProductField {
                product,
                field,
                value: product_value,
                replacement,
            } => match value(values, *product_value)? {
                EvalValue::Product(actual, fields) if actual == product => {
                    let mut fields = fields.clone();
                    let Some(slot) = fields.get_mut(usize::from(*field)) else {
                        return Err(Flow::Trap("product replacement field out of bounds".into()));
                    };
                    *slot = value(values, *replacement)?.clone();
                    self.allocate()?;
                    Ok(EvalValue::Product(*product, fields))
                }
                _ => Err(Flow::Trap("product replacement identity mismatch".into())),
            },
        }
    }

    fn constant(&mut self, constant: &Constant) -> std::result::Result<EvalValue, Flow> {
        match constant {
            Constant::Unit => Ok(EvalValue::Unit),
            Constant::Bool(value) => Ok(EvalValue::Bool(*value)),
            Constant::I64(value) => Ok(EvalValue::I64(*value)),
            Constant::F64(value) => Ok(EvalValue::F64(*value)),
            Constant::Str(value) => {
                self.allocate()?;
                Ok(EvalValue::Str(value.clone()))
            }
            Constant::Symbol(value) => {
                self.allocate()?;
                Ok(EvalValue::Symbol(value.clone()))
            }
            Constant::EmptyList => Ok(EvalValue::List(Vec::new())),
            Constant::None => Ok(EvalValue::None),
        }
    }

    fn runtime(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Add | Op::Subtract | Op::Multiply | Op::Divide => {
                self.numeric(operation, &arguments)
            }
            Op::EqualValue => binary(&arguments, |left, right| {
                Ok(EvalValue::Bool(value_equal(left, right)?))
            }),
            Op::SameObject => binary(&arguments, |left, right| {
                let same = match (left, right) {
                    (EvalValue::Buf(left), EvalValue::Buf(right)) => left.id == right.id,
                    (EvalValue::Handle(left), EvalValue::Handle(right)) => left == right,
                    _ => return Err(Flow::Trap("same-object category mismatch".into())),
                };
                Ok(EvalValue::Bool(same))
            }),
            Op::ListEqual => binary(&arguments, |left, right| {
                let (EvalValue::List(left), EvalValue::List(right)) = (left, right) else {
                    return Err(Flow::Trap("list-equal category mismatch".into()));
                };
                if left.len().max(right.len()) > self.config.max_list_equal_steps {
                    return Err(Flow::Trap("list-equal step limit exceeded".into()));
                }
                let equal = left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| value_equal(left, right).unwrap_or(false));
                Ok(EvalValue::Bool(equal))
            }),
            Op::F64BitsEqual => binary(&arguments, |left, right| {
                Ok(EvalValue::Bool(
                    as_f64_exact(left)?.to_bits() == as_f64_exact(right)?.to_bits(),
                ))
            }),
            Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual => {
                self.compare(operation, &arguments)
            }
            Op::Not => unary(&arguments, |value| Ok(EvalValue::Bool(!as_bool(value)?))),
            Op::BitAnd | Op::BitOr | Op::BitXor => binary(&arguments, |left, right| {
                let left = as_i64(left)?;
                let right = as_i64(right)?;
                Ok(EvalValue::I64(match operation {
                    Op::BitAnd => left & right,
                    Op::BitOr => left | right,
                    Op::BitXor => left ^ right,
                    _ => return Err(Flow::Trap("invalid bit operation".into())),
                }))
            }),
            Op::Cons => binary(&arguments, |head, tail| {
                let EvalValue::List(tail) = tail else {
                    return Err(Flow::Trap("cons tail is not a list".into()));
                };
                let mut list = Vec::with_capacity(tail.len().saturating_add(1));
                list.push(head.clone());
                list.extend(tail.iter().cloned());
                self.allocate()?;
                Ok(EvalValue::List(list))
            }),
            Op::Car => unary(&arguments, |list| match list {
                EvalValue::List(items) => items
                    .first()
                    .cloned()
                    .ok_or_else(|| Flow::Trap("car of empty list".into())),
                _ => Err(Flow::Trap("car operand is not a list".into())),
            }),
            Op::Cdr => unary(&arguments, |list| match list {
                EvalValue::List(items) if !items.is_empty() => {
                    self.allocate()?;
                    Ok(EvalValue::List(items[1..].to_vec()))
                }
                EvalValue::List(_) => Err(Flow::Trap("cdr of empty list".into())),
                _ => Err(Flow::Trap("cdr operand is not a list".into())),
            }),
            Op::IsEmptyList => unary(&arguments, |list| match list {
                EvalValue::List(items) => Ok(EvalValue::Bool(items.is_empty())),
                _ => Err(Flow::Trap("empty-list? operand is not a list".into())),
            }),
            Op::EmptyStr => {
                exact_arity(&arguments, 0)?;
                self.allocate()?;
                Ok(EvalValue::Str(String::new()))
            }
            Op::ArgCount => {
                exact_arity(&arguments, 0)?;
                let count = i64::try_from(self.config.args.len())
                    .map_err(|_| Flow::Trap("argument count out of range".into()))?;
                Ok(EvalValue::I64(count))
            }
            Op::Arg => unary(&arguments, |index| {
                let index = usize::try_from(as_i64(index)?)
                    .map_err(|_| Flow::Trap("argument index out of range".into()))?;
                if let Some(argument) = self.config.args.get(index) {
                    self.allocate()?;
                    Ok(EvalValue::Some(Box::new(EvalValue::Str(argument.clone()))))
                } else {
                    Ok(EvalValue::None)
                }
            }),
            Op::BufNew => unary(&arguments, |size| {
                let size = usize::try_from(as_i64(size)?)
                    .map_err(|_| Flow::Trap("buf-new size out of range".into()))?;
                if size > self.config.max_buffer_bytes || size > 1_000_000 {
                    return Err(Flow::Trap("buf-new size out of range".into()));
                }
                self.allocate()?;
                let id = self.next_buffer_id;
                self.next_buffer_id = self.next_buffer_id.saturating_add(1);
                Ok(EvalValue::Buf(EvalBuffer {
                    id,
                    bytes: Rc::new(RefCell::new(vec![0; size])),
                }))
            }),
            Op::BufLen => unary(&arguments, |buffer| {
                let buffer = as_buffer(buffer)?;
                let length = i64::try_from(buffer.bytes.borrow().len())
                    .map_err(|_| Flow::Trap("buf-len out of range".into()))?;
                Ok(EvalValue::I64(length))
            }),
            Op::BufRef => binary(&arguments, |buffer, index| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-ref")?;
                let byte = buffer
                    .bytes
                    .borrow()
                    .get(index)
                    .copied()
                    .ok_or_else(|| Flow::Trap("buf-ref out of bounds".into()))?;
                Ok(EvalValue::I64(i64::from(byte)))
            }),
            Op::BufSet => ternary(&arguments, |buffer, index, byte| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-set")?;
                let byte = u8::try_from(as_i64(byte)?)
                    .map_err(|_| Flow::Trap("buf-set byte out of range".into()))?;
                let mut bytes = buffer.bytes.borrow_mut();
                let Some(slot) = bytes.get_mut(index) else {
                    return Err(Flow::Trap("buf-set out of bounds".into()));
                };
                *slot = byte;
                Ok(EvalValue::Unit)
            }),
            Op::BufClone => unary(&arguments, |buffer| {
                let buffer = as_buffer(buffer)?;
                self.allocate()?;
                let id = self.next_buffer_id;
                self.next_buffer_id = self.next_buffer_id.saturating_add(1);
                Ok(EvalValue::Buf(EvalBuffer {
                    id,
                    bytes: Rc::new(RefCell::new(buffer.bytes.borrow().clone())),
                }))
            }),
            Op::BufFromStr => unary(&arguments, |text| {
                self.allocate()?;
                let id = self.next_buffer_id;
                self.next_buffer_id = self.next_buffer_id.saturating_add(1);
                Ok(EvalValue::Buf(EvalBuffer {
                    id,
                    bytes: Rc::new(RefCell::new(as_str(text)?.as_bytes().to_vec())),
                }))
            }),
            Op::BufToStr => unary(&arguments, |buffer| {
                let buffer = as_buffer(buffer)?;
                self.allocate()?;
                match String::from_utf8(buffer.bytes.borrow().clone()) {
                    Ok(text) => Ok(EvalValue::Ok(Box::new(EvalValue::Str(text)))),
                    Err(_) => Ok(EvalValue::Err(Box::new(EvalValue::Str(
                        "buf-to-str invalid UTF-8".into(),
                    )))),
                }
            }),
            Op::BufSlice => ternary(&arguments, |buffer, offset, length| {
                let buffer = as_buffer(buffer)?;
                let offset = index_value(offset, "buf-slice")?;
                let length = index_value(length, "buf-slice")?;
                let Some(end) = offset.checked_add(length) else {
                    return Ok(EvalValue::Err(Box::new(EvalValue::Str(
                        "buf-slice range overflow".into(),
                    ))));
                };
                let bytes = {
                    let bytes = buffer.bytes.borrow();
                    let Some(bytes) = bytes.get(offset..end) else {
                        return Ok(EvalValue::Err(Box::new(EvalValue::Str(
                            "buf-slice out of bounds".into(),
                        ))));
                    };
                    bytes.to_vec()
                };
                self.allocate()?;
                let id = self.next_buffer_id;
                self.next_buffer_id = self.next_buffer_id.saturating_add(1);
                Ok(EvalValue::Ok(Box::new(EvalValue::Buf(EvalBuffer {
                    id,
                    bytes: Rc::new(RefCell::new(bytes)),
                }))))
            }),
            Op::BufGetU32 => binary(&arguments, |buffer, index| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-get-u32")?;
                let end = index
                    .checked_add(4)
                    .ok_or_else(|| Flow::Trap("buf-get-u32 index overflow".into()))?;
                let bytes = buffer.bytes.borrow();
                let slice = bytes
                    .get(index..end)
                    .ok_or_else(|| Flow::Trap("buf-get-u32 out of bounds".into()))?;
                let mut word = [0; 4];
                word.copy_from_slice(slice);
                Ok(EvalValue::I64(i64::from(u32::from_le_bytes(word))))
            }),
            Op::BufSetU32 => ternary(&arguments, |buffer, index, number| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-set-u32")?;
                let end = index
                    .checked_add(4)
                    .ok_or_else(|| Flow::Trap("buf-set-u32 index overflow".into()))?;
                let number = u32::try_from(as_i64(number)?)
                    .map_err(|_| Flow::Trap("buf-set-u32 value out of range".into()))?;
                let mut bytes = buffer.bytes.borrow_mut();
                let destination = bytes
                    .get_mut(index..end)
                    .ok_or_else(|| Flow::Trap("buf-set-u32 out of bounds".into()))?;
                destination.copy_from_slice(&number.to_le_bytes());
                Ok(EvalValue::Unit)
            }),
            Op::StrLen => unary(&arguments, |text| {
                let length = i64::try_from(as_str(text)?.len())
                    .map_err(|_| Flow::Trap("str-len out of range".into()))?;
                Ok(EvalValue::I64(length))
            }),
            Op::StrRef => binary(&arguments, |text, index| {
                let index = index_value(index, "str-ref")?;
                let byte = as_str(text)?
                    .as_bytes()
                    .get(index)
                    .copied()
                    .ok_or_else(|| Flow::Trap("str-ref out of bounds".into()))?;
                Ok(EvalValue::I64(i64::from(byte)))
            }),
            Op::StrAppend => binary(&arguments, |left, right| {
                let mut result = as_str(left)?.to_owned();
                result.push_str(as_str(right)?);
                self.allocate()?;
                Ok(EvalValue::Str(result))
            }),
            Op::StrSlice => ternary(&arguments, |text, start, end| {
                let start = index_value(start, "str-slice")?;
                let end = index_value(end, "str-slice")?;
                let bytes = as_str(text)?.as_bytes();
                let slice = bytes
                    .get(start..end)
                    .ok_or_else(|| Flow::Trap("str-slice out of bounds".into()))?;
                let result = std::str::from_utf8(slice)
                    .map_err(|_| Flow::Trap("str-slice splits UTF-8".into()))?;
                self.allocate()?;
                Ok(EvalValue::Str(result.to_owned()))
            }),
            Op::StrFromByte => unary(&arguments, |value| {
                let byte = u8::try_from(as_i64(value)?)
                    .map_err(|_| Flow::Trap("str-from-byte out of range".into()))?;
                self.allocate()?;
                Ok(EvalValue::Str(String::from(char::from(byte))))
            }),
            Op::StrFromI64 => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Str(as_i64(value)?.to_string()))
            }),
            Op::StrFromF64 => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Str(as_f64_exact(value)?.to_string()))
            }),
            Op::Ok => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Ok(Box::new(value.clone())))
            }),
            Op::Err => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Err(Box::new(value.clone())))
            }),
            Op::IsOk => unary(&arguments, |value| match value {
                EvalValue::Ok(_) => Ok(EvalValue::Bool(true)),
                EvalValue::Err(_) => Ok(EvalValue::Bool(false)),
                _ => Err(Flow::Trap("is-ok operand is not Result".into())),
            }),
            Op::UnwrapOk => unary(&arguments, |value| match value {
                EvalValue::Ok(value) => Ok(value.as_ref().clone()),
                EvalValue::Err(_) => Err(Flow::Trap("unwrap-ok on Err".into())),
                _ => Err(Flow::Trap("unwrap-ok operand is not Result".into())),
            }),
            Op::UnwrapErr => unary(&arguments, |value| match value {
                EvalValue::Err(value) => Ok(value.as_ref().clone()),
                EvalValue::Ok(_) => Err(Flow::Trap("unwrap-err on Ok".into())),
                _ => Err(Flow::Trap("unwrap-err operand is not Result".into())),
            }),
            Op::Some => unary(&arguments, |value| {
                self.allocate()?;
                Ok(EvalValue::Some(Box::new(value.clone())))
            }),
            Op::IsSome => unary(&arguments, |value| match value {
                EvalValue::Some(_) => Ok(EvalValue::Bool(true)),
                EvalValue::None => Ok(EvalValue::Bool(false)),
                _ => Err(Flow::Trap("is-some operand is not Option".into())),
            }),
            Op::UnwrapSome => unary(&arguments, |value| match value {
                EvalValue::Some(value) => Ok(value.as_ref().clone()),
                EvalValue::None => Err(Flow::Trap("unwrap-some on none".into())),
                _ => Err(Flow::Trap("unwrap-some operand is not Option".into())),
            }),
            Op::SysSqliteOpen
            | Op::SysSqliteClose
            | Op::SysSqliteBusyTimeout
            | Op::SysSqliteExec
            | Op::SysSqlitePrepare
            | Op::SysSqliteFinalize
            | Op::SysSqliteReset
            | Op::SysSqliteClearBindings
            | Op::SysSqliteBindNull
            | Op::SysSqliteBindI64
            | Op::SysSqliteBindF64
            | Op::SysSqliteBindText
            | Op::SysSqliteBindBytes
            | Op::SysSqliteStep
            | Op::SysSqliteColumnCount
            | Op::SysSqliteColumnType
            | Op::SysSqliteColumnI64
            | Op::SysSqliteColumnF64
            | Op::SysSqliteColumnText
            | Op::SysSqliteColumnBytes
            | Op::SysSqliteChanges
            | Op::SysSqliteLastInsertRowid
            | Op::SysSqliteExtendedResultCode
            | Op::SysSqliteBackup
            | Op::Print
            | Op::Flush
            | Op::ReadByte
            | Op::WriteByte
            | Op::WriteStr
            | Op::StdinHandle
            | Op::SysIsatty
            | Op::SysClose
            | Op::SysReadByte
            | Op::SysWriteByte
            | Op::SysReadInto
            | Op::SysWriteFrom
            | Op::SysTtyGuardSave
            | Op::SysTtyGuardClear
            | Op::SysOpenRead
            | Op::SysOpenWrite
            | Op::SysOpenAppend
            | Op::SysOpenCreateNew
            | Op::SysOpenDir
            | Op::SysFsync
            | Op::SysTruncate
            | Op::SysRename
            | Op::SysRandomFill
            | Op::SysSha256
            | Op::SysPathExists
            | Op::SysWaitMs
            | Op::SysNowMs
            | Op::SysSocket
            | Op::SysBind
            | Op::SysListen
            | Op::SysAccept
            | Op::SysRecv
            | Op::SysSend
            | Op::SysPoll
            | Op::SysTtyGet
            | Op::SysTtySet => Err(Flow::Unsupported(operation)),
        }
    }

    fn numeric(
        &self,
        operation: RuntimeOp,
        arguments: &[EvalValue],
    ) -> std::result::Result<EvalValue, Flow> {
        exact_arity(arguments, 2)?;
        let left = arguments
            .first()
            .ok_or_else(|| Flow::Trap("numeric operand missing".into()))?;
        let right = arguments
            .get(1)
            .ok_or_else(|| Flow::Trap("numeric operand missing".into()))?;
        if matches!(left, EvalValue::F64(_)) || matches!(right, EvalValue::F64(_)) {
            let left = as_numeric_f64(left)?;
            let right = as_numeric_f64(right)?;
            let result = match operation {
                RuntimeOp::Add => left + right,
                RuntimeOp::Subtract => left - right,
                RuntimeOp::Multiply => left * right,
                RuntimeOp::Divide => left / right,
                _ => return Err(Flow::Trap("invalid numeric operation".into())),
            };
            Ok(EvalValue::F64(result))
        } else {
            let left = as_i64(left)?;
            let right = as_i64(right)?;
            let result = match operation {
                RuntimeOp::Add => left.checked_add(right),
                RuntimeOp::Subtract => left.checked_sub(right),
                RuntimeOp::Multiply => left.checked_mul(right),
                RuntimeOp::Divide => left.checked_div(right),
                _ => return Err(Flow::Trap("invalid numeric operation".into())),
            }
            .ok_or_else(|| Flow::Trap("checked I64 arithmetic failed".into()))?;
            Ok(EvalValue::I64(result))
        }
    }

    fn compare(
        &self,
        operation: RuntimeOp,
        arguments: &[EvalValue],
    ) -> std::result::Result<EvalValue, Flow> {
        exact_arity(arguments, 2)?;
        let left = arguments
            .first()
            .ok_or_else(|| Flow::Trap("comparison operand missing".into()))?;
        let right = arguments
            .get(1)
            .ok_or_else(|| Flow::Trap("comparison operand missing".into()))?;
        let result = if matches!(left, EvalValue::F64(_)) || matches!(right, EvalValue::F64(_)) {
            compare_values(operation, as_numeric_f64(left)?, as_numeric_f64(right)?)?
        } else {
            compare_values(operation, as_i64(left)?, as_i64(right)?)?
        };
        Ok(EvalValue::Bool(result))
    }

    fn consume_fuel(&mut self) -> std::result::Result<(), Flow> {
        if self.fuel == 0 {
            return Err(Flow::Resource("fuel".into()));
        }
        self.fuel -= 1;
        Ok(())
    }

    fn allocate(&mut self) -> std::result::Result<(), Flow> {
        if self.allocations >= self.config.max_allocations {
            return Err(Flow::Resource("allocations".into()));
        }
        self.allocations += 1;
        Ok(())
    }
}

fn compare_values<T: PartialOrd>(
    operation: RuntimeOp,
    left: T,
    right: T,
) -> std::result::Result<bool, Flow> {
    match operation {
        RuntimeOp::Less => Ok(left < right),
        RuntimeOp::LessEqual => Ok(left <= right),
        RuntimeOp::Greater => Ok(left > right),
        RuntimeOp::GreaterEqual => Ok(left >= right),
        _ => Err(Flow::Trap("invalid comparison operation".into())),
    }
}

fn value_equal(left: &EvalValue, right: &EvalValue) -> std::result::Result<bool, Flow> {
    match (left, right) {
        (EvalValue::Unit, EvalValue::Unit) | (EvalValue::None, EvalValue::None) => Ok(true),
        (EvalValue::Bool(left), EvalValue::Bool(right)) => Ok(left == right),
        (EvalValue::I64(left), EvalValue::I64(right)) => Ok(left == right),
        (EvalValue::F64(left), EvalValue::F64(right)) => Ok(left == right),
        (EvalValue::Str(left), EvalValue::Str(right))
        | (EvalValue::Symbol(left), EvalValue::Symbol(right)) => Ok(left == right),
        (EvalValue::Some(left), EvalValue::Some(right))
        | (EvalValue::Ok(left), EvalValue::Ok(right))
        | (EvalValue::Err(left), EvalValue::Err(right)) => value_equal(left, right),
        (EvalValue::None, EvalValue::Some(_)) | (EvalValue::Some(_), EvalValue::None) => Ok(false),
        (EvalValue::Ok(_), EvalValue::Err(_)) | (EvalValue::Err(_), EvalValue::Ok(_)) => Ok(false),
        _ => Err(Flow::Trap("equal-value category mismatch".into())),
    }
}

fn unary<F>(arguments: &[EvalValue], operation: F) -> std::result::Result<EvalValue, Flow>
where
    F: FnOnce(&EvalValue) -> std::result::Result<EvalValue, Flow>,
{
    exact_arity(arguments, 1)?;
    let value = arguments
        .first()
        .ok_or_else(|| Flow::Trap("unary operand missing".into()))?;
    operation(value)
}

fn binary<F>(arguments: &[EvalValue], operation: F) -> std::result::Result<EvalValue, Flow>
where
    F: FnOnce(&EvalValue, &EvalValue) -> std::result::Result<EvalValue, Flow>,
{
    exact_arity(arguments, 2)?;
    let left = arguments
        .first()
        .ok_or_else(|| Flow::Trap("binary operand missing".into()))?;
    let right = arguments
        .get(1)
        .ok_or_else(|| Flow::Trap("binary operand missing".into()))?;
    operation(left, right)
}

fn ternary<F>(arguments: &[EvalValue], operation: F) -> std::result::Result<EvalValue, Flow>
where
    F: FnOnce(&EvalValue, &EvalValue, &EvalValue) -> std::result::Result<EvalValue, Flow>,
{
    exact_arity(arguments, 3)?;
    let first = arguments
        .first()
        .ok_or_else(|| Flow::Trap("ternary operand missing".into()))?;
    let second = arguments
        .get(1)
        .ok_or_else(|| Flow::Trap("ternary operand missing".into()))?;
    let third = arguments
        .get(2)
        .ok_or_else(|| Flow::Trap("ternary operand missing".into()))?;
    operation(first, second, third)
}

fn exact_arity(arguments: &[EvalValue], expected: usize) -> std::result::Result<(), Flow> {
    if arguments.len() == expected {
        Ok(())
    } else {
        Err(Flow::Trap("evaluator runtime arity mismatch".into()))
    }
}

fn value(values: &[Option<EvalValue>], id: ValueId) -> std::result::Result<&EvalValue, Flow> {
    values
        .get(id.index().unwrap_or(usize::MAX))
        .and_then(Option::as_ref)
        .ok_or_else(|| Flow::Trap(format!("evaluator missing SSA value {}", id.raw())))
}

fn set_value(
    values: &mut [Option<EvalValue>],
    id: ValueId,
    value: EvalValue,
) -> std::result::Result<(), Flow> {
    let Some(slot) = values.get_mut(id.index().unwrap_or(usize::MAX)) else {
        return Err(Flow::Trap("evaluator ValueId is out of range".into()));
    };
    *slot = Some(value);
    Ok(())
}

fn values_for(
    values: &[Option<EvalValue>],
    ids: &[ValueId],
) -> std::result::Result<Vec<EvalValue>, Flow> {
    ids.iter().map(|id| value(values, *id).cloned()).collect()
}

fn assign_parameters(
    values: &mut [Option<EvalValue>],
    parameters: &[crate::BlockParameter],
    arguments: Vec<EvalValue>,
) -> std::result::Result<(), Flow> {
    if parameters.len() != arguments.len() {
        return Err(Flow::Trap("evaluator block argument arity mismatch".into()));
    }
    for (parameter, argument) in parameters.iter().zip(arguments) {
        set_value(values, parameter.id, argument)?;
    }
    Ok(())
}

fn as_bool(value: &EvalValue) -> std::result::Result<bool, Flow> {
    match value {
        EvalValue::Bool(value) => Ok(*value),
        _ => Err(Flow::Trap("expected Bool".into())),
    }
}

fn as_i64(value: &EvalValue) -> std::result::Result<i64, Flow> {
    match value {
        EvalValue::I64(value) => Ok(*value),
        _ => Err(Flow::Trap("expected I64".into())),
    }
}

fn as_f64_exact(value: &EvalValue) -> std::result::Result<f64, Flow> {
    match value {
        EvalValue::F64(value) => Ok(*value),
        _ => Err(Flow::Trap("expected F64".into())),
    }
}

fn as_numeric_f64(value: &EvalValue) -> std::result::Result<f64, Flow> {
    match value {
        EvalValue::I64(value) => Ok(*value as f64),
        EvalValue::F64(value) => Ok(*value),
        _ => Err(Flow::Trap("expected numeric value".into())),
    }
}

fn as_str(value: &EvalValue) -> std::result::Result<&str, Flow> {
    match value {
        EvalValue::Str(value) => Ok(value),
        _ => Err(Flow::Trap("expected Str".into())),
    }
}

fn as_buffer(value: &EvalValue) -> std::result::Result<&EvalBuffer, Flow> {
    match value {
        EvalValue::Buf(value) => Ok(value),
        _ => Err(Flow::Trap("expected Buf".into())),
    }
}

fn index_value(value: &EvalValue, operation: &str) -> std::result::Result<usize, Flow> {
    usize::try_from(as_i64(value)?)
        .map_err(|_| Flow::Trap(format!("{operation} index out of range")))
}

#[allow(dead_code)]
fn _block_id_is_used(id: BlockId) -> BlockId {
    id
}
