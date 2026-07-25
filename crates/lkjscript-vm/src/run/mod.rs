//! Bytecode interpreter over a validated immutable chunk.

mod calls;
mod dispatch;
mod ext_ops;
mod numeric;

use std::time::{Duration, Instant};

use lkjscript_core::{
    Constant, Error, ErrorClass, ExecutionConfig, ExecutionOutcome, GcConfig, GcHeap as Arena,
    HeapObj, HostError, ResourceLimitKind, Result, Trap, ValidatedChunk, Value,
};
use lkjscript_jit::{
    EngineError, EntryDecision, FunctionId, JitSession, JitStats, NativeValue, ScalarInvocation,
    ScalarSignature, TrapCode,
};

use crate::host_ext::ResourceTable;

pub(crate) struct Frame {
    pub proto: u32,
    pub ip: usize,
    pub stack_base: usize,
    pub locals_base: usize,
}

enum Stop {
    Returned(Value),
    Exited(i32),
}

pub trait RuntimeTier {
    fn observe_function_entry(&mut self, prototype: u32) -> EntryDecision;
    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature>;
    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError>;
    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u32>) -> String;
    fn record_invocation_failure(&mut self, function: FunctionId);
}

#[derive(Debug, Default)]
pub struct NoTier;

impl RuntimeTier for NoTier {
    fn observe_function_entry(&mut self, _prototype: u32) -> EntryDecision {
        EntryDecision::Interpret
    }

    fn scalar_signature(&self, _function: FunctionId) -> Option<ScalarSignature> {
        None
    }

    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        _arguments: &[NativeValue],
        _execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        Err(EngineError::new_unavailable(function))
    }

    fn trap_message(&self, _function: FunctionId, _trap: TrapCode, _site: Option<u32>) -> String {
        "native tier is unavailable".to_string()
    }

    fn record_invocation_failure(&mut self, _function: FunctionId) {}
}

impl RuntimeTier for JitSession {
    fn observe_function_entry(&mut self, prototype: u32) -> EntryDecision {
        JitSession::observe_function_entry(self, prototype)
    }

    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature> {
        JitSession::scalar_signature(self, function)
    }

    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        JitSession::invoke_scalar(self, function, arguments, execution)
    }

    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u32>) -> String {
        self.trap_message_for(function, trap, site)
    }

    fn record_invocation_failure(&mut self, function: FunctionId) {
        JitSession::record_invocation_failure(self, function);
    }
}

pub struct Vm<'a, J: RuntimeTier> {
    pub(crate) chunk: &'a ValidatedChunk,
    pub(crate) globals: Vec<Value>,
    pub(crate) stack: Vec<Value>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) arena: Arena,
    pub(crate) jit: J,
    pub(crate) exit_code: Option<i32>,
    pub(crate) args: Vec<String>,
    pub(crate) resources: ResourceTable,
    config: ExecutionConfig,
    fuel_remaining: u64,
    output_bytes: usize,
    allocation_error: Option<Error>,
    started: Instant,
}

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub fn new(
        chunk: &'a ValidatedChunk,
        jit: J,
        args: Vec<String>,
        config: ExecutionConfig,
    ) -> Self {
        Self {
            chunk,
            globals: vec![Value::INVALID; chunk.global_names().len()],
            stack: Vec::new(),
            frames: Vec::new(),
            arena: Arena::new(GcConfig {
                max_allocations: config.max_allocations,
                max_heap_bytes: config.max_heap_bytes,
                ..GcConfig::default()
            }),
            jit,
            exit_code: None,
            args,
            resources: ResourceTable::new(config.max_handles),
            fuel_remaining: config.instruction_fuel,
            output_bytes: 0,
            allocation_error: None,
            started: Instant::now(),
            config,
        }
    }

    pub fn run(mut self) -> ExecutionOutcome {
        self.run_inner()
    }

    fn run_inner(&mut self) -> ExecutionOutcome {
        let stopped = self.run_loop();
        let mut outcome = match stopped {
            Ok(Stop::Returned(value)) => {
                let arena = std::mem::take(&mut self.arena);
                match arena.into_owned(value) {
                    Ok(value) => ExecutionOutcome::Returned(value),
                    Err(error) => ExecutionOutcome::Trapped(Trap::new(format!(
                        "invalid returned VM value: {error}"
                    ))),
                }
            }
            Ok(Stop::Exited(code)) => ExecutionOutcome::Exited(code),
            Err(error) => outcome_from_error(error),
        };

        let resources = std::mem::replace(&mut self.resources, ResourceTable::new(0));
        drop(resources);
        let restore_error = crate::host_term::restore_tty().err();
        let flush_error = crate::host::flush_out().err();
        if restore_error.is_some() || flush_error.is_some() {
            let prior = outcome.summary();
            let message = match (restore_error, flush_error) {
                (Some(restore), Some(flush)) => {
                    format!("{restore}; stdout cleanup {flush}")
                }
                (Some(restore), None) => restore.to_string(),
                (None, Some(flush)) => format!("stdout cleanup {flush}"),
                (None, None) => String::new(),
            };
            outcome = ExecutionOutcome::HostFailure(HostError::during_cleanup(message, prior));
        }
        outcome
    }

    fn run_loop(&mut self) -> Result<Stop> {
        self.frames.push(Frame {
            proto: u32::MAX,
            ip: 0,
            stack_base: 0,
            locals_base: 0,
        });
        for _ in 0..self.chunk.main().locals {
            self.stack.push(Value::INVALID);
        }
        self.check_runtime_limits()?;
        loop {
            if let Some(code) = self.exit_code {
                return Ok(Stop::Exited(code));
            }
            if self.frames.is_empty() {
                return self.pop().map(Stop::Returned);
            }
            self.check_deadline()?;
            if self.fuel_remaining == 0 {
                return Err(Error::resource(
                    ResourceLimitKind::InstructionFuel,
                    "instruction fuel exhausted",
                ));
            }
            self.fuel_remaining -= 1;
            if self.arena.needs_collect() {
                self.collect();
            }
            self.step()?;
            if let Some(error) = self.allocation_error.take() {
                return Err(error);
            }
            self.check_runtime_limits()?;
        }
    }

    fn collect(&mut self) {
        let mut roots = self.globals.clone();
        roots.extend_from_slice(&self.stack);
        self.arena.collect(&roots);
    }

    fn check_runtime_limits(&mut self) -> Result<()> {
        if self.stack.len() > self.config.max_stack_values {
            return Err(Error::resource(
                ResourceLimitKind::StackValues,
                "VM stack value limit exceeded",
            ));
        }
        if self.frames.len() > self.config.max_frames {
            return Err(Error::resource(
                ResourceLimitKind::FrameDepth,
                "VM frame depth limit exceeded",
            ));
        }
        if self.arena.total_allocations() > self.config.max_allocations {
            return Err(Error::resource(
                ResourceLimitKind::Allocations,
                "VM aggregate allocation limit exceeded",
            ));
        }
        if self.arena.heap_bytes() > self.config.max_heap_bytes {
            self.collect();
            if self.arena.heap_bytes() > self.config.max_heap_bytes {
                return Err(Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "VM live heap byte limit exceeded",
                ));
            }
        }
        if self.resources.limit_exceeded()
            || self.resources.allocated_handle_slots() > self.config.max_handles
        {
            return Err(Error::resource(
                ResourceLimitKind::Handles,
                "VM handle limit exceeded",
            ));
        }
        self.check_deadline()
    }

    pub(crate) fn check_deadline(&self) -> Result<()> {
        if self
            .config
            .wall_time
            .is_some_and(|limit| self.started.elapsed() >= limit)
        {
            return Err(Error::deadline("execution wall deadline exceeded"));
        }
        Ok(())
    }

    pub(crate) fn remaining_wall_time(&self) -> Result<Option<Duration>> {
        let Some(limit) = self.config.wall_time else {
            return Ok(None);
        };
        let elapsed = self.started.elapsed();
        limit
            .checked_sub(elapsed)
            .filter(|remaining| !remaining.is_zero())
            .map(Some)
            .ok_or_else(|| Error::deadline("execution wall deadline exceeded"))
    }

    pub(crate) fn ensure_host_deadline_support(
        &self,
        operation: &str,
        hard_deadline_supported: bool,
    ) -> Result<()> {
        if self.config.require_hard_deadline
            && self.config.wall_time.is_some()
            && !hard_deadline_supported
        {
            return Err(Error::host(format!(
                "{operation}: hard wall deadline is unsupported by the current host wrapper"
            )));
        }
        Ok(())
    }

    pub(crate) fn deadline_timeout_ms(&self) -> Result<Option<i32>> {
        let Some(remaining) = self.remaining_wall_time()? else {
            return Ok(None);
        };
        let milliseconds = remaining.as_millis().max(1);
        Ok(Some(i32::try_from(milliseconds).unwrap_or(i32::MAX)))
    }

    pub(crate) fn wait_for_stdin(&self) -> Result<()> {
        let Some(timeout) = self.deadline_timeout_ms()? else {
            return Ok(());
        };
        let ready = lkjscript_sys::poll_fd(lkjscript_sys::STDIN_FD, timeout)
            .map_err(|error| Error::host(format!("read-byte poll: {error}")))?;
        if ready {
            Ok(())
        } else {
            Err(Error::deadline(
                "execution wall deadline exceeded during read-byte",
            ))
        }
    }

    pub(crate) fn record_output(&mut self, bytes: usize) -> Result<()> {
        let total = self.output_bytes.checked_add(bytes).ok_or_else(|| {
            Error::resource(
                ResourceLimitKind::OutputBytes,
                "VM output byte counter overflow",
            )
        })?;
        if total > self.config.max_output_bytes {
            return Err(Error::resource(
                ResourceLimitKind::OutputBytes,
                "VM output byte limit exceeded",
            ));
        }
        self.output_bytes = total;
        Ok(())
    }

    pub(crate) fn code_len(&self) -> Result<usize> {
        Ok(self.code()?.len())
    }

    pub(crate) fn code(&self) -> Result<&[u8]> {
        let frame = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
        if frame.proto == u32::MAX {
            Ok(&self.chunk.main().code)
        } else {
            self.chunk
                .protos()
                .get(frame.proto as usize)
                .map(|proto| proto.code.as_slice())
                .ok_or_else(|| Error::msg("frame proto index out of range"))
        }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        let (proto, ip) = {
            let frame = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
            (frame.proto, frame.ip)
        };
        let code = if proto == u32::MAX {
            &self.chunk.main().code
        } else {
            &self
                .chunk
                .protos()
                .get(proto as usize)
                .ok_or_else(|| Error::msg("frame proto index out of range"))?
                .code
        };
        let byte = *code.get(ip).ok_or_else(|| Error::msg("ip out of range"))?;
        if let Some(frame) = self.frames.last_mut() {
            frame.ip += 1;
        }
        Ok(byte)
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let low = self.read_u8()? as u16;
        let high = self.read_u8()? as u16;
        Ok(low | (high << 8))
    }

    pub(crate) fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub(crate) fn make_i64(&mut self, number: i64) -> Result<Value> {
        match Value::from_small_i64(number) {
            Some(value) => Ok(value),
            None => self.arena.alloc(HeapObj::Int(number)),
        }
    }

    pub(crate) fn as_i64(&self, value: Value) -> Result<i64> {
        if let Some(number) = value.as_small_i64() {
            return Ok(number);
        }
        match value.as_heap().and_then(|_| self.arena.get(value).ok()) {
            Some(HeapObj::Int(number)) => Ok(*number),
            _ => Err(Error::msg("expected I64")),
        }
    }

    pub(crate) fn pop(&mut self) -> Result<Value> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| Error::msg("VM stack underflow"))?;
        if value.is_invalid() {
            return Err(Error::msg("uninitialized VM value"));
        }
        Ok(value)
    }

    pub(crate) fn peek(&self) -> Result<Value> {
        let value = self
            .stack
            .last()
            .copied()
            .ok_or_else(|| Error::msg("VM stack underflow"))?;
        if value.is_invalid() {
            return Err(Error::msg("uninitialized VM value"));
        }
        Ok(value)
    }

    pub(crate) fn load_const(&mut self, id: usize) -> Result<Value> {
        match self
            .chunk
            .constants()
            .get(id)
            .ok_or_else(|| Error::msg("bad const"))?
        {
            Constant::I64(number) => self.make_i64(*number),
            Constant::F64(number) => self.arena.alloc(HeapObj::Float(*number)),
            Constant::Str(text) => self.arena.alloc(HeapObj::Str(text.clone())),
            Constant::Symbol(symbol) => self.arena.alloc(HeapObj::Symbol(symbol.clone())),
            Constant::Proto(proto) => self.make_i64(i64::from(*proto)),
        }
    }

    fn step(&mut self) -> Result<()> {
        let code_len = self.code_len()?;
        let ip = self
            .frames
            .last()
            .map(|frame| frame.ip)
            .ok_or_else(|| Error::msg("no frame"))?;
        if ip >= code_len {
            return Err(Error::msg("function ended without Return"));
        }
        let op = self.read_u8()?;
        dispatch::dispatch(self, op)
    }
}

impl<'a> Vm<'a, JitSession> {
    pub fn run_auto(mut self) -> (ExecutionOutcome, JitStats) {
        let outcome = self.run_inner();
        let stats = self.jit.stats();
        (outcome, stats)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn test_chunk() -> ValidatedChunk {
    let mut chunk = lkjscript_core::Chunk::new();
    chunk.main.emit(lkjscript_core::Op::Unit);
    chunk.main.emit(lkjscript_core::Op::Return);
    lkjscript_core::validate_chunk(chunk, &lkjscript_core::ValidationLimits::default())
        .expect("VM unit-test chunk validates")
}

fn outcome_from_error(error: Error) -> ExecutionOutcome {
    match error.class() {
        ErrorClass::Ordinary => ExecutionOutcome::Trapped(Trap::new(error.to_string())),
        ErrorClass::Deadline => ExecutionOutcome::DeadlineExceeded,
        ErrorClass::Resource(kind) => ExecutionOutcome::ResourceLimitExceeded(kind),
        ErrorClass::Host => ExecutionOutcome::HostFailure(HostError::new(error.to_string())),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::field_reassign_with_default)]
mod tests {
    use std::time::Duration;

    use lkjscript_core::{
        validate_chunk, Chunk, Constant, ExecutionConfig, ExecutionOutcome, Op, ResourceLimitKind,
        ValidationLimits,
    };

    use super::{NoTier as NullJit, Vm};

    fn validated(ops: &[Op]) -> lkjscript_core::ValidatedChunk {
        let mut chunk = Chunk::new();
        for op in ops {
            chunk.main.emit(*op);
        }
        validate(chunk)
    }

    fn validate(chunk: Chunk) -> lkjscript_core::ValidatedChunk {
        validate_chunk(chunk, &ValidationLimits::default()).expect("test chunk validates")
    }

    #[test]
    fn fuel_and_returned_values_use_structured_outcomes() {
        let chunk = validated(&[Op::Unit, Op::Return]);
        let returned = Vm::new(&chunk, NullJit, Vec::new(), ExecutionConfig::default()).run();
        assert!(matches!(returned, ExecutionOutcome::Returned(value) if value.is_unit()));

        let mut config = ExecutionConfig::default();
        config.instruction_fuel = 1;
        let exhausted = Vm::new(&chunk, NullJit, Vec::new(), config).run();
        assert_eq!(
            exhausted,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel)
        );
    }

    #[test]
    fn exit_does_not_terminate_or_contaminate_later_vms() {
        let mut exit = Chunk::new();
        let zero = exit.add_const(Constant::I64(0));
        exit.main.emit_op_u16(Op::LoadConst, zero.0);
        exit.main.emit(Op::Exit);
        let exit = validate_chunk(exit, &ValidationLimits::default()).expect("exit validates");
        assert_eq!(
            Vm::new(&exit, NullJit, Vec::new(), ExecutionConfig::default()).run(),
            ExecutionOutcome::Exited(0)
        );

        let returned = validated(&[Op::Unit, Op::Return]);
        assert!(matches!(
            Vm::new(
                &returned,
                NullJit,
                Vec::new(),
                ExecutionConfig::default()
            )
            .run(),
            ExecutionOutcome::Returned(value) if value.is_unit()
        ));
    }

    #[test]
    fn trap_does_not_contaminate_a_later_vm() {
        let mut trap = Chunk::new();
        let one = trap.add_const(Constant::I64(1));
        let zero = trap.add_const(Constant::I64(0));
        trap.main.emit_op_u16(Op::LoadConst, one.0);
        trap.main.emit_op_u16(Op::LoadConst, zero.0);
        trap.main.emit(Op::Div);
        trap.main.emit(Op::Return);
        let trap = validate(trap);
        assert!(matches!(
            Vm::new(&trap, NullJit, Vec::new(), ExecutionConfig::default()).run(),
            ExecutionOutcome::Trapped(_)
        ));

        let returned = validated(&[Op::Unit, Op::Return]);
        assert!(matches!(
            Vm::new(
                &returned,
                NullJit,
                Vec::new(),
                ExecutionConfig::default()
            )
            .run(),
            ExecutionOutcome::Returned(value) if value.is_unit()
        ));
    }

    #[test]
    fn returned_heap_values_own_their_storage() {
        let mut chunk = Chunk::new();
        let text = chunk.add_const(Constant::Str("owned".into()));
        chunk.main.emit_op_u16(Op::LoadConst, text.0);
        chunk.main.emit(Op::Return);
        let chunk = validate(chunk);
        let outcome = Vm::new(&chunk, NullJit, Vec::new(), ExecutionConfig::default()).run();
        assert!(matches!(
            outcome,
            ExecutionOutcome::Returned(value) if value.as_str() == Some("owned")
        ));
    }

    #[test]
    fn configured_stack_frame_heap_allocation_and_output_limits_stop_execution() {
        let returned = validated(&[Op::Unit, Op::Return]);

        let mut stack = ExecutionConfig::default();
        stack.max_stack_values = 0;
        assert_eq!(
            Vm::new(&returned, NullJit, Vec::new(), stack).run(),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
        );

        let mut frames = ExecutionConfig::default();
        frames.max_frames = 0;
        assert_eq!(
            Vm::new(&returned, NullJit, Vec::new(), frames).run(),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth)
        );

        let mut string = Chunk::new();
        let text = string.add_const(Constant::Str("x".into()));
        string.main.emit_op_u16(Op::LoadConst, text.0);
        string.main.emit(Op::Return);
        let string = validate(string);

        let mut heap = ExecutionConfig::default();
        heap.max_heap_bytes = 0;
        assert_eq!(
            Vm::new(&string, NullJit, Vec::new(), heap).run(),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
        );

        let mut allocations = ExecutionConfig::default();
        allocations.max_allocations = 0;
        assert_eq!(
            Vm::new(&string, NullJit, Vec::new(), allocations).run(),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        );

        let mut output_chunk = Chunk::new();
        let text = output_chunk.add_const(Constant::Str("x".into()));
        output_chunk.main.emit_op_u16(Op::LoadConst, text.0);
        output_chunk.main.emit(Op::WriteStr);
        output_chunk.main.emit(Op::Return);
        let output_chunk = validate(output_chunk);
        let mut output = ExecutionConfig::default();
        output.max_output_bytes = 0;
        assert_eq!(
            Vm::new(&output_chunk, NullJit, Vec::new(), output).run(),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::OutputBytes)
        );

        let mut hard_deadline = ExecutionConfig::default();
        hard_deadline.require_hard_deadline = true;
        assert!(matches!(
            Vm::new(&output_chunk, NullJit, Vec::new(), hard_deadline).run(),
            ExecutionOutcome::HostFailure(error)
                if error.as_str().contains("hard wall deadline is unsupported")
        ));
    }

    #[test]
    fn sha256_opcode_returns_language_results_for_valid_and_invalid_ranges() {
        let mut valid = Chunk::new();
        let zero = valid.add_const(Constant::I64(0));
        valid.main.emit_op_u16(Op::LoadConst, zero.0);
        valid.main.emit(Op::BufNew);
        valid.main.emit_op_u16(Op::LoadConst, zero.0);
        valid.main.emit_op_u16(Op::LoadConst, zero.0);
        valid.main.emit(Op::SysSha256);
        valid.main.emit(Op::IsOk);
        valid.main.emit(Op::Return);
        let valid = validate(valid);
        assert!(matches!(
            Vm::new(&valid, NullJit, Vec::new(), ExecutionConfig::default()).run(),
            ExecutionOutcome::Returned(value) if value.as_bool() == Some(true)
        ));

        let mut invalid = Chunk::new();
        let zero = invalid.add_const(Constant::I64(0));
        let one = invalid.add_const(Constant::I64(1));
        invalid.main.emit_op_u16(Op::LoadConst, zero.0);
        invalid.main.emit(Op::BufNew);
        invalid.main.emit_op_u16(Op::LoadConst, zero.0);
        invalid.main.emit_op_u16(Op::LoadConst, one.0);
        invalid.main.emit(Op::SysSha256);
        invalid.main.emit(Op::IsOk);
        invalid.main.emit(Op::Return);
        let invalid = validate(invalid);
        assert!(matches!(
            Vm::new(&invalid, NullJit, Vec::new(), ExecutionConfig::default()).run(),
            ExecutionOutcome::Returned(value) if value.as_bool() == Some(false)
        ));
    }

    #[test]
    fn configured_handle_and_wall_limits_are_structured() {
        let socket = validated(&[Op::SysSocket, Op::Return]);
        let mut handles = ExecutionConfig::default();
        handles.max_handles = 0;
        assert_eq!(
            Vm::new(&socket, NullJit, Vec::new(), handles).run(),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Handles)
        );

        let mut loop_chunk = Chunk::new();
        loop_chunk.main.emit_op_u16(Op::Jump, 0);
        let loop_chunk = validate(loop_chunk);
        let mut deadline = ExecutionConfig::default();
        deadline.wall_time = Some(Duration::ZERO);
        assert_eq!(
            Vm::new(&loop_chunk, NullJit, Vec::new(), deadline).run(),
            ExecutionOutcome::DeadlineExceeded
        );

        let mut wait = Chunk::new();
        let duration = wait.add_const(Constant::I64(50));
        wait.main.emit_op_u16(Op::LoadConst, duration.0);
        wait.main.emit(Op::SysWaitMs);
        wait.main.emit(Op::Return);
        let wait = validate(wait);
        let mut deadline = ExecutionConfig::default();
        deadline.wall_time = Some(Duration::from_millis(1));
        assert_eq!(
            Vm::new(&wait, NullJit, Vec::new(), deadline).run(),
            ExecutionOutcome::DeadlineExceeded
        );
    }
}
