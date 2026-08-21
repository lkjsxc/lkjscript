use super::capability::BoundTransaction;
use super::{BoundCapabilities, Instruction, PreparedProgram};
use crate::platform::semantic::OwnerId;
use crate::platform::value::{MapKey, Value};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionFailureClass {
    Trap,
    Capability,
    PossibleVisibility,
    Resource,
    Cancelled,
    Infrastructure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionError {
    pub class: ExecutionFailureClass,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub possibly_visible: bool,
}

impl ExecutionError {
    pub fn new(
        class: ExecutionFailureClass,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            code: code.into(),
            message: message.into(),
            retryable: false,
            possibly_visible: class == ExecutionFailureClass::PossibleVisibility,
        }
    }

    pub fn resource(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ExecutionFailureClass::Resource, code, message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPolicy {
    pub instruction_fuel: u64,
    pub maximum_call_depth: usize,
    pub maximum_value_stack: usize,
}

impl Default for RunPolicy {
    fn default() -> Self {
        Self {
            instruction_fuel: 10_000_000,
            maximum_call_depth: 4_096,
            maximum_value_stack: 1_000_000,
        }
    }
}

/// Runtime-owned cancellation and deadline state. It is never representable as a language value
/// and therefore cannot cross a durable boundary.
#[derive(Clone, Debug)]
pub struct ExecutionControl {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl ExecutionControl {
    pub fn uncancelled() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
        }
    }

    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: Some(deadline),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn check(&self) -> Result<(), ExecutionError> {
        if self.is_cancelled() {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "execution_cancelled",
                "execution was cancelled by its owning task scope",
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "execution_deadline",
                "execution exceeded its operational deadline",
            ));
        }
        Ok(())
    }
}

impl Default for ExecutionControl {
    fn default() -> Self {
        Self::uncancelled()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunObservation {
    pub instructions: u64,
    pub calls: u64,
    pub intrinsic_calls: u64,
    pub maximum_call_depth: usize,
    pub maximum_value_stack: usize,
    pub production_tier: &'static str,
}

pub struct Vm<'a> {
    program: &'a PreparedProgram,
    policy: RunPolicy,
}

impl<'a> Vm<'a> {
    pub fn new(program: &'a PreparedProgram, policy: RunPolicy) -> Self {
        Self { program, policy }
    }

    pub fn invoke(
        &self,
        function: &OwnerId,
        arguments: Vec<Value>,
    ) -> Result<(Value, RunObservation), ExecutionError> {
        self.invoke_inner(function, arguments, None, &ExecutionControl::uncancelled())
    }

    pub fn invoke_with_capabilities(
        &self,
        function: &OwnerId,
        arguments: Vec<Value>,
        capabilities: &BoundCapabilities,
    ) -> Result<(Value, RunObservation), ExecutionError> {
        self.invoke_inner(
            function,
            arguments,
            Some(capabilities),
            &ExecutionControl::uncancelled(),
        )
    }

    pub fn invoke_controlled(
        &self,
        function: &OwnerId,
        arguments: Vec<Value>,
        capabilities: Option<&BoundCapabilities>,
        control: &ExecutionControl,
    ) -> Result<(Value, RunObservation), ExecutionError> {
        self.invoke_inner(function, arguments, capabilities, control)
    }

    fn invoke_inner(
        &self,
        function: &OwnerId,
        arguments: Vec<Value>,
        capabilities: Option<&BoundCapabilities>,
        control: &ExecutionControl,
    ) -> Result<(Value, RunObservation), ExecutionError> {
        control.check()?;
        let mut machine = Machine {
            program: self.program,
            policy: self.policy,
            fuel: self.policy.instruction_fuel,
            stack: Vec::new(),
            frames: Vec::new(),
            observation: RunObservation {
                instructions: 0,
                calls: 0,
                intrinsic_calls: 0,
                maximum_call_depth: 0,
                maximum_value_stack: 0,
                production_tier: "bytecode_v1",
            },
        };
        let mut transactions: BTreeMap<String, BoundTransaction> = BTreeMap::new();
        machine.call(function, arguments)?;
        loop {
            control.check()?;
            if machine.fuel == 0 {
                return Err(ExecutionError::resource(
                    "execution_fuel",
                    "instruction fuel was exhausted",
                ));
            }
            machine.fuel -= 1;
            machine.observation.instructions = machine.observation.instructions.saturating_add(1);
            let instruction = {
                let frame = machine.frames.last_mut().ok_or_else(|| {
                    ExecutionError::new(
                        ExecutionFailureClass::Infrastructure,
                        "vm_frame_missing",
                        "bytecode machine lost its active frame",
                    )
                })?;
                let instruction = frame
                    .instructions
                    .get(frame.instruction)
                    .cloned()
                    .ok_or_else(|| {
                        ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "vm_instruction_missing",
                            "bytecode instruction pointer escaped its function",
                        )
                    })?;
                frame.instruction = frame.instruction.saturating_add(1);
                instruction
            };
            match instruction {
                Instruction::Unit => machine.push(Value::Unit)?,
                Instruction::Bool(value) => machine.push(Value::Bool(value))?,
                Instruction::I64(value) => machine.push(Value::I64(value))?,
                Instruction::Text(value) => machine.push(Value::text(value))?,
                Instruction::StaticText(value) => machine.push(Value::static_text(value))?,
                Instruction::Function(owner) => machine.push(Value::Function(owner))?,
                Instruction::LoadLocal(local) => {
                    let value = machine
                        .frames
                        .last()
                        .and_then(|frame| frame.locals.get(local))
                        .and_then(Clone::clone)
                        .ok_or_else(|| {
                            ExecutionError::new(
                                ExecutionFailureClass::Infrastructure,
                                "vm_local_uninitialized",
                                "compiled code read an uninitialized local",
                            )
                        })?;
                    machine.push(value)?;
                }
                Instruction::StoreLocal(local) => {
                    let value = machine.pop()?;
                    let frame = machine.frames.last_mut().ok_or_else(|| {
                        ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "vm_frame_missing",
                            "bytecode machine lost its active frame",
                        )
                    })?;
                    let destination = frame.locals.get_mut(local).ok_or_else(|| {
                        ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "vm_local_missing",
                            "compiled local index is out of range",
                        )
                    })?;
                    *destination = Some(value);
                }
                Instruction::Drop => {
                    machine.pop()?;
                }
                Instruction::JumpIfFalse(target) => {
                    let Value::Bool(condition) = machine.pop()? else {
                        return Err(runtime_type("if condition is not boolean"));
                    };
                    if !condition {
                        machine.jump(target)?;
                    }
                }
                Instruction::Jump(target) => machine.jump(target)?,
                Instruction::Call {
                    function,
                    arguments,
                } => {
                    let arguments = machine.pop_many(arguments)?;
                    machine.call(&function, arguments)?;
                }
                Instruction::Record { owner, fields } => {
                    let values = machine.pop_many(fields.len())?;
                    machine.push(Value::record(owner, fields.into_iter().zip(values)))?;
                }
                Instruction::Variant {
                    owner,
                    case,
                    has_payload,
                } => {
                    let payload = if has_payload {
                        Some(machine.pop()?)
                    } else {
                        None
                    };
                    machine.push(Value::variant(owner, case, payload))?;
                }
                Instruction::Field(field) => {
                    let value = machine.pop()?;
                    let field = value.field(&field).cloned().ok_or_else(|| {
                        runtime_type("field selection received a foreign record value")
                    })?;
                    machine.push(field)?;
                }
                Instruction::List(length) => {
                    let items = machine.pop_many(length)?;
                    machine.push(Value::List(Arc::new(items)))?;
                }
                Instruction::Map(length) => {
                    let entries = machine.pop_many(length.saturating_mul(2))?;
                    let mut map = BTreeMap::new();
                    for pair in entries.chunks_exact(2) {
                        let key = MapKey::from_value(pair[0].clone()).map_err(|error| {
                            ExecutionError::new(
                                ExecutionFailureClass::Infrastructure,
                                error.code,
                                error.message,
                            )
                        })?;
                        if map.insert(key, pair[1].clone()).is_some() {
                            return Err(ExecutionError::new(
                                ExecutionFailureClass::Trap,
                                "map_duplicate_key",
                                "map expression contains a duplicate key",
                            ));
                        }
                    }
                    machine.push(Value::Map(Arc::new(map)))?;
                }
                Instruction::SwitchVariant(jumps) => {
                    let Value::Variant { case, payload, .. } = machine.pop()? else {
                        return Err(runtime_type("match received a foreign non-variant value"));
                    };
                    let jump = jumps.iter().find(|jump| jump.case == case).ok_or_else(|| {
                        ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "vm_match_case",
                            "validated exhaustive match omitted a runtime case",
                        )
                    })?;
                    match (jump.binding_local, payload) {
                        (Some(local), Some(payload)) => {
                            let frame = machine.frames.last_mut().ok_or_else(|| {
                                ExecutionError::new(
                                    ExecutionFailureClass::Infrastructure,
                                    "vm_frame_missing",
                                    "bytecode machine lost its active frame",
                                )
                            })?;
                            let destination = frame.locals.get_mut(local).ok_or_else(|| {
                                ExecutionError::new(
                                    ExecutionFailureClass::Infrastructure,
                                    "vm_match_local",
                                    "match binding local is out of range",
                                )
                            })?;
                            *destination = Some(*payload);
                        }
                        (None, None) => {}
                        _ => {
                            return Err(ExecutionError::new(
                                ExecutionFailureClass::Infrastructure,
                                "vm_match_payload",
                                "runtime variant payload disagrees with validated match",
                            ));
                        }
                    }
                    machine.jump(jump.target)?;
                }
                Instruction::Perform {
                    capability,
                    operation,
                    arguments,
                } => {
                    let arguments = machine.pop_many(arguments)?;
                    let value = if let Some(transaction) = transactions.get_mut(&capability) {
                        transaction.call(&operation, arguments)?
                    } else {
                        capabilities
                            .ok_or_else(capabilities_unbound)?
                            .call_controlled(&capability, &operation, arguments, control)?
                    };
                    machine.push(value)?;
                }
                Instruction::BeginTransaction {
                    capability,
                    binding,
                } => {
                    if transactions.contains_key(&binding) {
                        return Err(ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "capability_transaction_binding",
                            "transaction binding is already active",
                        ));
                    }
                    let transaction = capabilities
                        .ok_or_else(capabilities_unbound)?
                        .begin_transaction_controlled(&capability, control)?;
                    transactions.insert(binding, transaction);
                }
                Instruction::CommitTransaction { binding } => {
                    let transaction = transactions.remove(&binding).ok_or_else(|| {
                        ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "capability_transaction_missing",
                            "transaction commit has no active scope",
                        )
                    })?;
                    transaction.commit()?;
                }
                Instruction::Return => {
                    let result = machine.pop()?;
                    machine.frames.pop();
                    if machine.frames.is_empty() {
                        if !transactions.is_empty() {
                            return Err(ExecutionError::new(
                                ExecutionFailureClass::Infrastructure,
                                "capability_transaction_leak",
                                "function returned with an active transaction scope",
                            ));
                        }
                        if !machine.stack.is_empty() {
                            return Err(ExecutionError::new(
                                ExecutionFailureClass::Infrastructure,
                                "vm_stack_residue",
                                "bytecode execution left values beneath its result",
                            ));
                        }
                        return Ok((result, machine.observation));
                    }
                    machine.push(result)?;
                }
            }
        }
    }
}

struct CallFrame {
    instructions: Arc<Vec<Instruction>>,
    instruction: usize,
    locals: Vec<Option<Value>>,
    stack_base: usize,
}

struct Machine<'a> {
    program: &'a PreparedProgram,
    policy: RunPolicy,
    fuel: u64,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    observation: RunObservation,
}

impl Machine<'_> {
    fn call(&mut self, owner: &OwnerId, arguments: Vec<Value>) -> Result<(), ExecutionError> {
        let function = self.program.function(owner).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_function_missing",
                format!("prepared function '{}' is absent", owner.diagnostic_name()),
            )
        })?;
        if arguments.len() != function.signature.parameters.len() {
            return Err(runtime_type("function argument count is foreign"));
        }
        self.observation.calls = self.observation.calls.saturating_add(1);
        if let Some(implementation) = &function.external_implementation {
            self.observation.intrinsic_calls = self.observation.intrinsic_calls.saturating_add(1);
            let result =
                self.program
                    .call_intrinsic(implementation, &function.signature, arguments)?;
            return self.push(result);
        }
        if self.frames.len() >= self.policy.maximum_call_depth {
            return Err(ExecutionError::resource(
                "execution_call_depth",
                "maximum call depth was exceeded",
            ));
        }
        let compiled = function.compiled.as_ref().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_function_code",
                "prepared source function has no bytecode",
            )
        })?;
        let mut locals = vec![None; compiled.local_count];
        for (index, argument) in arguments.into_iter().enumerate() {
            let destination = locals.get_mut(index).ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "vm_parameter_local",
                    "compiled function has fewer locals than parameters",
                )
            })?;
            *destination = Some(argument);
        }
        self.frames.push(CallFrame {
            instructions: Arc::new(compiled.instructions.clone()),
            instruction: 0,
            locals,
            stack_base: self.stack.len(),
        });
        self.observation.maximum_call_depth =
            self.observation.maximum_call_depth.max(self.frames.len());
        Ok(())
    }

    fn push(&mut self, value: Value) -> Result<(), ExecutionError> {
        if self.stack.len() >= self.policy.maximum_value_stack {
            return Err(ExecutionError::resource(
                "execution_value_stack",
                "maximum value stack was exceeded",
            ));
        }
        self.stack.push(value);
        self.observation.maximum_value_stack =
            self.observation.maximum_value_stack.max(self.stack.len());
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, ExecutionError> {
        let base = self.frames.last().map_or(0, |frame| frame.stack_base);
        if self.stack.len() <= base {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_stack_underflow",
                "compiled instruction consumed beneath its frame stack",
            ));
        }
        self.stack.pop().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_stack_underflow",
                "compiled instruction consumed an empty stack",
            )
        })
    }

    fn pop_many(&mut self, count: usize) -> Result<Vec<Value>, ExecutionError> {
        let base = self.frames.last().map_or(0, |frame| frame.stack_base);
        if self.stack.len().saturating_sub(base) < count {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_stack_underflow",
                "compiled instruction consumed too many values",
            ));
        }
        let start = self.stack.len() - count;
        Ok(self.stack.split_off(start))
    }

    fn jump(&mut self, target: usize) -> Result<(), ExecutionError> {
        let frame = self.frames.last_mut().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_frame_missing",
                "bytecode machine lost its active frame",
            )
        })?;
        if target >= frame.instructions.len() {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "vm_jump_target",
                "compiled jump target is outside its function",
            ));
        }
        frame.instruction = target;
        Ok(())
    }
}

fn runtime_type(message: &'static str) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "vm_runtime_type",
        message,
    )
}

fn capabilities_unbound() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "capability_unbound",
        "effectful execution requires bound deployment grants",
    )
}
