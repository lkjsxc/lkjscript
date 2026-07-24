use std::collections::HashSet;

use crate::image::{
    entry_metadata, frame_facts, outcome_map_entry, relocation, scalar_safepoint, source_map_entry,
    trap_map_entry, AbiVersions, ImageParts, InstallableImage, OutcomeKind, RelocationKind,
    RelocationTarget,
};
use crate::plan::{
    BlockId, BoolComparison, F64Comparison, FunctionId, FunctionPlan, I64Comparison, Instruction,
    Operation, RuntimeCallSlot, RuntimeOutcome, Terminator, TrapCode, ValueId, ValueType,
};
use crate::verify::VerifiedMachinePlan;
use crate::{EncodeError, NativeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodingConfig {
    versions: AbiVersions,
}

impl EncodingConfig {
    #[must_use]
    pub const fn new(versions: AbiVersions) -> Self {
        Self { versions }
    }

    #[must_use]
    pub const fn versions(self) -> AbiVersions {
        self.versions
    }
}

impl Default for EncodingConfig {
    fn default() -> Self {
        Self::new(AbiVersions::current())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixupTarget {
    Block(BlockId),
    Trap(TrapCode),
    StatusReturn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BranchFixup {
    displacement_offset: usize,
    target: FixupTarget,
}

struct FunctionEncoder<'a> {
    function: &'a FunctionPlan,
    signatures: &'a [(FunctionId, crate::Signature)],
    bytes: &'a mut Vec<u8>,
    relocations: &'a mut Vec<crate::Relocation>,
    safepoints: &'a mut Vec<crate::Safepoint>,
    source_map: &'a mut Vec<crate::SourceMapEntry>,
    trap_map: &'a mut Vec<crate::TrapMapEntry>,
    outcome_map: &'a mut Vec<crate::OutcomeMapEntry>,
    runtime_calls: &'a mut HashSet<RuntimeCallSlot>,
    fixups: Vec<BranchFixup>,
    block_offsets: Vec<Option<usize>>,
    trap_offsets: [Option<usize>; 3],
    status_return_offset: Option<usize>,
    frame_bytes: u32,
    maximum_code_bytes: usize,
}

pub fn encode(
    plan: VerifiedMachinePlan,
    config: EncodingConfig,
) -> Result<InstallableImage, NativeError> {
    let mut bytes = Vec::new();
    let mut entries = Vec::new();
    let mut relocations = Vec::new();
    let mut runtime_call_set = HashSet::new();
    let mut frames = Vec::new();
    let mut safepoints = Vec::new();
    let mut source_map = Vec::new();
    let mut trap_map = Vec::new();
    let mut outcome_map = Vec::new();
    let signatures: Vec<_> = plan
        .functions
        .iter()
        .map(|function| (function.id, function.signature.clone()))
        .collect();

    for function in &plan.functions {
        let start = bytes.len();
        let frame_bytes = calculate_frame_bytes(function)?;
        let outgoing_arguments = maximum_outgoing_arguments(function)?;
        let mut encoder = FunctionEncoder {
            function,
            signatures: &signatures,
            bytes: &mut bytes,
            relocations: &mut relocations,
            safepoints: &mut safepoints,
            source_map: &mut source_map,
            trap_map: &mut trap_map,
            outcome_map: &mut outcome_map,
            runtime_calls: &mut runtime_call_set,
            fixups: Vec::new(),
            block_offsets: vec![None; function.blocks.len()],
            trap_offsets: [None; 3],
            status_return_offset: None,
            frame_bytes,
            maximum_code_bytes: plan.limits.max_code_bytes(),
        };
        encoder.emit_function()?;
        let end = encoder.bytes.len();
        let start_u32 = to_u32(start)?;
        let end_u32 = to_u32(end)?;
        entries.push(entry_metadata(
            function.id,
            function.source_function,
            function.signature.clone(),
            start_u32,
            end_u32,
        ));
        frames.push(frame_facts(
            function.id,
            frame_bytes,
            to_u32(function.values.len())?,
            to_u32(function.locals.len())?,
            outgoing_arguments,
        ));
        source_map.push(source_map_entry(function.id, start_u32, end_u32, None));
    }

    let mut runtime_calls: Vec<_> = runtime_call_set.into_iter().collect();
    runtime_calls.sort_by_key(|slot| match slot {
        RuntimeCallSlot::IdentityI64V1 => 1_u8,
        RuntimeCallSlot::PollV1 => 2_u8,
        RuntimeCallSlot::EnterFunctionV1 => 3_u8,
    });

    let image = InstallableImage::new(ImageParts {
        bytes,
        entries,
        relocations,
        runtime_calls,
        frames,
        safepoints,
        source_map,
        trap_map,
        outcome_map,
        work_units: plan.work_units,
        versions: config.versions,
    })
    .map_err(NativeError::Image)?;
    if image.accounting().metadata_bytes() > plan.limits.max_metadata_bytes() {
        return Err(NativeError::Encode(EncodeError::LimitExceeded(
            "metadata bytes",
        )));
    }
    Ok(image)
}

impl FunctionEncoder<'_> {
    fn emit_function(&mut self) -> Result<(), NativeError> {
        self.emit_prologue()?;
        self.emit_parameters()?;
        let entry = self
            .function
            .entry
            .ok_or(NativeError::Encode(EncodeError::MissingEntry))?;
        self.emit_jump(FixupTarget::Block(entry))?;

        for block in &self.function.blocks {
            let index = block.id.index as usize;
            let offset = self.bytes.len();
            let slot = self
                .block_offsets
                .get_mut(index)
                .ok_or(NativeError::Encode(EncodeError::InvalidLabel))?;
            *slot = Some(offset);
            for instruction in &block.instructions {
                let start = self.bytes.len();
                self.emit_instruction(instruction)?;
                let end = self.bytes.len();
                self.source_map.push(source_map_entry(
                    self.function.id,
                    to_u32(start)?,
                    to_u32(end)?,
                    instruction.source,
                ));
            }
            let terminator = block
                .terminator
                .as_ref()
                .ok_or(NativeError::Encode(EncodeError::InvalidLabel))?;
            self.emit_terminator(terminator)?;
        }

        for trap in [
            TrapCode::I64Overflow,
            TrapCode::DivisionByZero,
            TrapCode::Explicit,
        ] {
            let offset = self.bytes.len();
            self.trap_offsets[trap_index(trap)] = Some(offset);
            self.trap_map
                .push(trap_map_entry(self.function.id, to_u32(offset)?, trap));
            self.outcome_map.push(outcome_map_entry(
                self.function.id,
                to_u32(offset)?,
                OutcomeKind::Trap(trap),
            ));
            self.emit_trap_stub(trap)?;
        }
        self.status_return_offset = Some(self.bytes.len());
        self.emit_zero_return()?;
        self.patch_fixups()?;
        self.check_code_limit()
    }

    fn emit_prologue(&mut self) -> Result<(), NativeError> {
        self.emit(&[0x55])?;
        self.emit(&[0x48, 0x89, 0xe5])?;
        self.emit(&[0x48, 0x81, 0xec])?;
        self.emit(&self.frame_bytes.to_le_bytes())?;
        self.store_integer_register(7, self.context_offset())
    }

    fn emit_parameters(&mut self) -> Result<(), NativeError> {
        let mut integer_index = 0_usize;
        let mut float_index = 0_usize;
        for (index, parameter) in self.function.signature.parameters().iter().enumerate() {
            let value = self
                .function
                .values
                .get(index)
                .ok_or(NativeError::Encode(EncodeError::InvalidValue))?
                .id;
            let offset = self.value_offset(value)?;
            match parameter {
                ValueType::I64 | ValueType::Bool => {
                    let register = [6_u8, 2_u8]
                        .get(integer_index)
                        .copied()
                        .ok_or(NativeError::Encode(EncodeError::UnsupportedSignature))?;
                    self.store_integer_register(register, offset)?;
                    integer_index += 1;
                }
                ValueType::F64 => {
                    self.store_xmm(float_index, offset)?;
                    float_index += 1;
                }
                ValueType::Unit => {
                    self.zero_rax()?;
                    self.store_rax(offset)?;
                }
            }
        }
        Ok(())
    }

    fn emit_instruction(&mut self, instruction: &Instruction) -> Result<(), NativeError> {
        match &instruction.operation {
            Operation::I64Const(value) => {
                self.load_rax_immediate(*value as u64)?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::F64Const(bits) => {
                self.load_rax_immediate(*bits)?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::BoolConst(value) => {
                self.load_rax_immediate(u64::from(*value))?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::Unit => {
                self.zero_rax()?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::I64Add(left, right) => {
                self.emit_checked_i64_binary(instruction.output, *left, *right, 0x03, None)?;
            }
            Operation::I64Sub(left, right) => {
                self.emit_checked_i64_binary(instruction.output, *left, *right, 0x2b, None)?;
            }
            Operation::I64Mul(left, right) => {
                self.emit_checked_i64_binary(
                    instruction.output,
                    *left,
                    *right,
                    0xaf,
                    Some(&[0x48, 0x0f]),
                )?;
            }
            Operation::I64Div(left, right) => {
                self.emit_checked_i64_division(instruction.output, *left, *right)?;
            }
            Operation::I64BitAnd(left, right) => {
                self.emit_i64_bitwise(instruction.output, *left, *right, 0x23)?;
            }
            Operation::I64BitOr(left, right) => {
                self.emit_i64_bitwise(instruction.output, *left, *right, 0x0b)?;
            }
            Operation::I64BitXor(left, right) => {
                self.emit_i64_bitwise(instruction.output, *left, *right, 0x33)?;
            }
            Operation::I64ToF64(value) => {
                self.emit_i64_to_f64(instruction.output, *value)?;
            }
            Operation::F64Add(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x58)?;
            }
            Operation::F64Sub(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x5c)?;
            }
            Operation::F64Mul(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x59)?;
            }
            Operation::F64Div(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x5e)?;
            }
            Operation::I64Compare(comparison, left, right) => {
                self.emit_integer_comparison(
                    instruction.output,
                    *left,
                    *right,
                    integer_condition(*comparison),
                )?;
            }
            Operation::BoolCompare(comparison, left, right) => {
                let condition = match comparison {
                    BoolComparison::Equal => 0x94,
                    BoolComparison::NotEqual => 0x95,
                };
                self.emit_integer_comparison(instruction.output, *left, *right, condition)?;
            }
            Operation::F64Compare(comparison, left, right) => {
                self.emit_f64_comparison(instruction.output, *left, *right, *comparison)?;
            }
            Operation::F64BitsEqual(left, right) => {
                self.emit_integer_comparison(instruction.output, *left, *right, 0x94)?;
            }
            Operation::BoolNot(value) => {
                self.load_rax(self.value_offset(*value)?)?;
                self.emit(&[0x48, 0x83, 0xf0, 0x01])?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::ReadLocal(local) => {
                self.load_rax(self.local_offset(*local)?)?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::WriteLocal(local, value) => {
                self.load_rax(self.value_offset(*value)?)?;
                self.store_rax(self.local_offset(*local)?)?;
                self.zero_rax()?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::Call(callee, arguments) => {
                let signature = self
                    .find_signature(*callee)
                    .ok_or(NativeError::Encode(EncodeError::InvalidCall))?
                    .clone();
                self.emit_call(
                    instruction.output,
                    &signature,
                    arguments,
                    RelocationTarget::Function(*callee),
                )?;
            }
            Operation::RuntimeCall(slot, arguments) => {
                let signature = slot.signature();
                self.runtime_calls.insert(*slot);
                self.emit_call(
                    instruction.output,
                    &signature,
                    arguments,
                    RelocationTarget::Runtime(*slot),
                )?;
            }
        }
        Ok(())
    }

    fn emit_checked_i64_binary(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        opcode: u8,
        prefix: Option<&[u8]>,
    ) -> Result<(), NativeError> {
        self.load_rax(self.value_offset(left)?)?;
        if let Some(prefix) = prefix {
            self.emit(prefix)?;
            self.emit(&[opcode, 0x85])?;
        } else {
            self.emit(&[0x48, opcode, 0x85])?;
        }
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit_conditional_jump(0x80, FixupTarget::Trap(TrapCode::I64Overflow))?;
        self.store_rax(self.value_offset(output)?)
    }

    fn emit_checked_i64_division(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
    ) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x83, 0xbd])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit(&[0x00])?;
        self.emit_conditional_jump(0x84, FixupTarget::Trap(TrapCode::DivisionByZero))?;
        self.load_rax(self.value_offset(left)?)?;
        self.emit(&[0x48, 0xb9])?;
        self.emit(&(i64::MIN as u64).to_le_bytes())?;
        self.emit(&[0x48, 0x39, 0xc8])?;
        self.emit(&[0x0f, 0x85])?;
        let normal_displacement = self.reserve_i32()?;
        self.emit(&[0x48, 0x83, 0xbd])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit(&[0xff])?;
        self.emit_conditional_jump(0x84, FixupTarget::Trap(TrapCode::I64Overflow))?;
        let normal = self.bytes.len();
        patch_relative(self.bytes, normal_displacement, normal)?;
        self.emit(&[0x48, 0x99])?;
        self.emit(&[0x48, 0xf7, 0xbd])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.store_rax(self.value_offset(output)?)
    }

    fn emit_i64_bitwise(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        opcode: u8,
    ) -> Result<(), NativeError> {
        self.load_rax(self.value_offset(left)?)?;
        self.emit(&[0x48, opcode, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.store_rax(self.value_offset(output)?)
    }

    fn emit_i64_to_f64(&mut self, output: ValueId, value: ValueId) -> Result<(), NativeError> {
        self.emit(&[0xf2, 0x48, 0x0f, 0x2a, 0x85])?;
        self.emit_displacement(self.value_offset(value)?)?;
        self.store_xmm0(self.value_offset(output)?)
    }

    fn emit_f64_binary(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        opcode: u8,
    ) -> Result<(), NativeError> {
        self.load_xmm0(self.value_offset(left)?)?;
        self.emit(&[0xf2, 0x0f, opcode, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.store_xmm0(self.value_offset(output)?)
    }

    fn emit_integer_comparison(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        condition: u8,
    ) -> Result<(), NativeError> {
        self.load_rax(self.value_offset(left)?)?;
        self.emit(&[0x48, 0x3b, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit(&[0x0f, condition, 0xc0, 0x0f, 0xb6, 0xc0])?;
        self.store_rax(self.value_offset(output)?)
    }

    fn emit_f64_comparison(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        comparison: F64Comparison,
    ) -> Result<(), NativeError> {
        self.load_xmm0(self.value_offset(left)?)?;
        self.emit(&[0x66, 0x0f, 0x2e, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        let condition = match comparison {
            F64Comparison::OrderedEqual => 0x94,
            F64Comparison::OrderedNotEqual => 0x95,
            F64Comparison::OrderedLessThan => 0x92,
            F64Comparison::OrderedLessThanOrEqual => 0x96,
            F64Comparison::OrderedGreaterThan => 0x97,
            F64Comparison::OrderedGreaterThanOrEqual => 0x93,
        };
        self.emit(&[0x0f, condition, 0xc0])?;
        self.emit(&[0x0f, 0x9b, 0xc1])?;
        self.emit(&[0x20, 0xc8, 0x0f, 0xb6, 0xc0])?;
        self.store_rax(self.value_offset(output)?)
    }

    fn emit_call(
        &mut self,
        output: ValueId,
        signature: &crate::Signature,
        arguments: &[ValueId],
        target: RelocationTarget,
    ) -> Result<(), NativeError> {
        self.load_integer_register(7, self.context_offset())?;
        let mut integer_index = 0_usize;
        let mut float_index = 0_usize;
        for (argument, argument_type) in arguments.iter().zip(signature.parameters()) {
            match argument_type {
                ValueType::I64 | ValueType::Bool => {
                    let register = [6_u8, 2_u8]
                        .get(integer_index)
                        .copied()
                        .ok_or(NativeError::Encode(EncodeError::UnsupportedSignature))?;
                    self.load_integer_register(register, self.value_offset(*argument)?)?;
                    integer_index += 1;
                }
                ValueType::F64 => {
                    self.load_xmm(float_index, self.value_offset(*argument)?)?;
                    float_index += 1;
                }
                ValueType::Unit => {}
            }
        }
        self.emit(&[0x49, 0xbb])?;
        let relocation_offset = self.bytes.len();
        self.emit(&0_u64.to_le_bytes())?;
        self.relocations.push(relocation(
            to_u32(relocation_offset)?,
            RelocationKind::Absolute64,
            target,
        ));
        let call_offset = self.bytes.len();
        self.emit(&[0x41, 0xff, 0xd3])?;
        self.safepoints
            .push(scalar_safepoint(self.function.id, to_u32(call_offset)?));
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0x83, 0x39, 0x00])?;
        self.emit_conditional_jump(0x85, FixupTarget::StatusReturn)?;
        match signature.result() {
            ValueType::I64 | ValueType::Bool => self.store_rax(self.value_offset(output)?)?,
            ValueType::F64 => self.store_xmm0(self.value_offset(output)?)?,
            ValueType::Unit => {
                self.zero_rax()?;
                self.store_rax(self.value_offset(output)?)?;
            }
        }
        Ok(())
    }

    fn emit_terminator(&mut self, terminator: &Terminator) -> Result<(), NativeError> {
        match terminator {
            Terminator::Branch(target) => self.emit_jump(FixupTarget::Block(*target)),
            Terminator::BranchIf {
                condition,
                when_true,
                when_false,
            } => {
                self.load_rax(self.value_offset(*condition)?)?;
                self.emit(&[0x48, 0x85, 0xc0])?;
                self.emit_conditional_jump(0x85, FixupTarget::Block(*when_true))?;
                self.emit_jump(FixupTarget::Block(*when_false))
            }
            Terminator::Return(value) => {
                let offset = self.bytes.len();
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    OutcomeKind::Return,
                ));
                match self.value_type(*value)? {
                    ValueType::I64 | ValueType::Bool => {
                        self.load_rax(self.value_offset(*value)?)?;
                    }
                    ValueType::F64 => {
                        self.load_xmm0(self.value_offset(*value)?)?;
                    }
                    ValueType::Unit => self.zero_rax()?,
                }
                self.emit_epilogue()
            }
            Terminator::Trap(trap) => self.emit_jump(FixupTarget::Trap(*trap)),
            Terminator::Exit(code) => {
                let offset = self.bytes.len();
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    OutcomeKind::Exit,
                ));
                self.load_rax(self.value_offset(*code)?)?;
                self.load_integer_register(1, self.context_offset())?;
                self.emit(&[0xc7, 0x01])?;
                self.emit(&2_u32.to_le_bytes())?;
                self.emit(&[0x48, 0x89, 0x41, 0x08])?;
                self.emit_zero_return()
            }
            Terminator::Outcome(outcome) => {
                let offset = self.bytes.len();
                self.outcome_map.push(outcome_map_entry(
                    self.function.id,
                    to_u32(offset)?,
                    match outcome {
                        RuntimeOutcome::DeadlineExceeded => OutcomeKind::DeadlineExceeded,
                        RuntimeOutcome::ResourceLimitExceeded => OutcomeKind::ResourceLimitExceeded,
                        RuntimeOutcome::HostFailure => OutcomeKind::HostFailure,
                    },
                ));
                let status = match outcome {
                    RuntimeOutcome::DeadlineExceeded => 3_u32,
                    RuntimeOutcome::ResourceLimitExceeded => 4_u32,
                    RuntimeOutcome::HostFailure => 5_u32,
                };
                self.load_integer_register(1, self.context_offset())?;
                self.emit(&[0xc7, 0x01])?;
                self.emit(&status.to_le_bytes())?;
                self.emit_zero_return()
            }
        }
    }

    fn emit_trap_stub(&mut self, trap: TrapCode) -> Result<(), NativeError> {
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0xc7, 0x01])?;
        self.emit(&1_u32.to_le_bytes())?;
        self.emit(&[0xc7, 0x41, 0x04])?;
        self.emit(&trap.as_u32().to_le_bytes())?;
        self.emit_zero_return()
    }

    fn emit_zero_return(&mut self) -> Result<(), NativeError> {
        self.zero_rax()?;
        self.emit(&[0x66, 0x0f, 0xef, 0xc0])?;
        self.emit_epilogue()
    }

    fn emit_epilogue(&mut self) -> Result<(), NativeError> {
        self.emit(&[0xc9, 0xc3])
    }

    fn emit_jump(&mut self, target: FixupTarget) -> Result<(), NativeError> {
        self.emit(&[0xe9])?;
        let displacement_offset = self.reserve_i32()?;
        self.fixups.push(BranchFixup {
            displacement_offset,
            target,
        });
        Ok(())
    }

    fn emit_conditional_jump(
        &mut self,
        condition: u8,
        target: FixupTarget,
    ) -> Result<(), NativeError> {
        self.emit(&[0x0f, condition])?;
        let displacement_offset = self.reserve_i32()?;
        self.fixups.push(BranchFixup {
            displacement_offset,
            target,
        });
        Ok(())
    }

    fn patch_fixups(&mut self) -> Result<(), NativeError> {
        for fixup in &self.fixups {
            let target = match fixup.target {
                FixupTarget::Block(block) => self
                    .block_offsets
                    .get(block.index as usize)
                    .copied()
                    .flatten(),
                FixupTarget::Trap(trap) => self.trap_offsets[trap_index(trap)],
                FixupTarget::StatusReturn => self.status_return_offset,
            }
            .ok_or(NativeError::Encode(EncodeError::InvalidLabel))?;
            patch_relative(self.bytes, fixup.displacement_offset, target)?;
        }
        Ok(())
    }

    fn find_signature(&self, function: FunctionId) -> Option<&crate::Signature> {
        self.signatures
            .iter()
            .find(|(function_id, _)| *function_id == function)
            .map(|(_, signature)| signature)
    }

    fn value_type(&self, value: ValueId) -> Result<ValueType, NativeError> {
        self.function
            .values
            .get(value.index as usize)
            .filter(|fact| fact.id == value)
            .map(|fact| fact.value_type)
            .ok_or(NativeError::Encode(EncodeError::InvalidValue))
    }

    fn context_offset(&self) -> i32 {
        -8
    }

    fn local_offset(&self, local: crate::LocalId) -> Result<i32, NativeError> {
        let slot = 1_usize
            .checked_add(local.index as usize)
            .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
        slot_offset(slot)
    }

    fn value_offset(&self, value: ValueId) -> Result<i32, NativeError> {
        let slot = 1_usize
            .checked_add(self.function.locals.len())
            .and_then(|base| base.checked_add(value.index as usize))
            .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
        slot_offset(slot)
    }

    fn load_rax_immediate(&mut self, value: u64) -> Result<(), NativeError> {
        self.emit(&[0x48, 0xb8])?;
        self.emit(&value.to_le_bytes())
    }

    fn zero_rax(&mut self) -> Result<(), NativeError> {
        self.emit(&[0x31, 0xc0])
    }

    fn load_rax(&mut self, offset: i32) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x8b, 0x85])?;
        self.emit_displacement(offset)
    }

    fn store_rax(&mut self, offset: i32) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x89, 0x85])?;
        self.emit_displacement(offset)
    }

    fn load_integer_register(&mut self, register: u8, offset: i32) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x8b, 0x85 | (register << 3)])?;
        self.emit_displacement(offset)
    }

    fn store_integer_register(&mut self, register: u8, offset: i32) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x89, 0x85 | (register << 3)])?;
        self.emit_displacement(offset)
    }

    fn load_xmm0(&mut self, offset: i32) -> Result<(), NativeError> {
        self.load_xmm(0, offset)
    }

    fn load_xmm(&mut self, register: usize, offset: i32) -> Result<(), NativeError> {
        let register = u8::try_from(register)
            .map_err(|_| NativeError::Encode(EncodeError::UnsupportedSignature))?;
        self.emit(&[0xf2, 0x0f, 0x10, 0x85 | (register << 3)])?;
        self.emit_displacement(offset)
    }

    fn store_xmm0(&mut self, offset: i32) -> Result<(), NativeError> {
        self.store_xmm(0, offset)
    }

    fn store_xmm(&mut self, register: usize, offset: i32) -> Result<(), NativeError> {
        let register = u8::try_from(register)
            .map_err(|_| NativeError::Encode(EncodeError::UnsupportedSignature))?;
        self.emit(&[0xf2, 0x0f, 0x11, 0x85 | (register << 3)])?;
        self.emit_displacement(offset)
    }

    fn emit_displacement(&mut self, displacement: i32) -> Result<(), NativeError> {
        self.emit(&displacement.to_le_bytes())
    }

    fn reserve_i32(&mut self) -> Result<usize, NativeError> {
        let offset = self.bytes.len();
        self.emit(&0_i32.to_le_bytes())?;
        Ok(offset)
    }

    fn emit(&mut self, bytes: &[u8]) -> Result<(), NativeError> {
        let next_len = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(NativeError::Encode(EncodeError::LimitExceeded(
                "code bytes",
            )))?;
        if next_len > self.maximum_code_bytes {
            return Err(NativeError::Encode(EncodeError::LimitExceeded(
                "code bytes",
            )));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn check_code_limit(&self) -> Result<(), NativeError> {
        if self.bytes.len() > self.maximum_code_bytes {
            return Err(NativeError::Encode(EncodeError::LimitExceeded(
                "code bytes",
            )));
        }
        Ok(())
    }
}

fn calculate_frame_bytes(function: &FunctionPlan) -> Result<u32, NativeError> {
    let slots = 1_usize
        .checked_add(function.locals.len())
        .and_then(|value| value.checked_add(function.values.len()))
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    let bytes = slots
        .checked_mul(8)
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    let aligned = bytes
        .checked_add(15)
        .map(|value| value & !15)
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    if aligned > i32::MAX as usize {
        return Err(NativeError::Encode(EncodeError::FrameTooLarge));
    }
    u32::try_from(aligned).map_err(|_| NativeError::Encode(EncodeError::FrameTooLarge))
}

fn maximum_outgoing_arguments(function: &FunctionPlan) -> Result<u8, NativeError> {
    let maximum = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.operation {
            Operation::Call(_, arguments) | Operation::RuntimeCall(_, arguments) => Some(
                arguments
                    .iter()
                    .filter(|argument| {
                        function
                            .values
                            .get(argument.index as usize)
                            .map(|fact| fact.value_type != ValueType::Unit)
                            .unwrap_or(false)
                    })
                    .count(),
            ),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    u8::try_from(maximum).map_err(|_| NativeError::Encode(EncodeError::UnsupportedSignature))
}

fn slot_offset(slot: usize) -> Result<i32, NativeError> {
    let bytes = slot
        .checked_add(1)
        .and_then(|value| value.checked_mul(8))
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    let bytes =
        i32::try_from(bytes).map_err(|_| NativeError::Encode(EncodeError::FrameTooLarge))?;
    Ok(-bytes)
}

fn patch_relative(
    bytes: &mut [u8],
    displacement_offset: usize,
    target: usize,
) -> Result<(), NativeError> {
    let after = displacement_offset
        .checked_add(4)
        .ok_or(NativeError::Encode(EncodeError::InvalidRelocation))?;
    if after > bytes.len() {
        return Err(NativeError::Encode(EncodeError::InvalidRelocation));
    }
    let target =
        i64::try_from(target).map_err(|_| NativeError::Encode(EncodeError::InvalidRelocation))?;
    let after =
        i64::try_from(after).map_err(|_| NativeError::Encode(EncodeError::InvalidRelocation))?;
    let displacement = i32::try_from(target - after)
        .map_err(|_| NativeError::Encode(EncodeError::InvalidRelocation))?;
    let end = displacement_offset + 4;
    let destination = bytes
        .get_mut(displacement_offset..end)
        .ok_or(NativeError::Encode(EncodeError::InvalidRelocation))?;
    destination.copy_from_slice(&displacement.to_le_bytes());
    Ok(())
}

fn integer_condition(comparison: I64Comparison) -> u8 {
    match comparison {
        I64Comparison::Equal => 0x94,
        I64Comparison::NotEqual => 0x95,
        I64Comparison::LessThan => 0x9c,
        I64Comparison::LessThanOrEqual => 0x9e,
        I64Comparison::GreaterThan => 0x9f,
        I64Comparison::GreaterThanOrEqual => 0x9d,
    }
}

fn trap_index(trap: TrapCode) -> usize {
    match trap {
        TrapCode::I64Overflow => 0,
        TrapCode::DivisionByZero => 1,
        TrapCode::Explicit => 2,
    }
}

fn to_u32(value: usize) -> Result<u32, NativeError> {
    u32::try_from(value).map_err(|_| NativeError::Encode(EncodeError::LimitExceeded("u32 offset")))
}
