//! Derived native scalar emission over admitted normalized meaning.
//!
//! Graph signatures select the direct-call ABI. Aggregate storage, callable
//! prefixes and opaque external calls remain boxed; every crossing is explicit.
use super::asm::{R::*, *};
use super::scalar_flow::{Representation, State};
use super::{
    Emitter, I, NormalizedCode, NormalizedFunctionBody, NormalizedProgram, Result, TypeForm,
    layout_extent, layout_offset, reserve, unsupported,
};
use crate::platform::kernel::{FunctionEffect, ParameterUse, ResultRelation};

/// Expand rewrites before representation analysis. Old interior labels retain
/// the existing emitter's meaning: they land after the complete replacement.
pub(super) fn expanded_code(
    program: &NormalizedProgram,
    code: &NormalizedCode,
) -> Result<(NormalizedCode, usize)> {
    let rewrites = crate::platform::compiler::egraph_opt::optimize(program, code);
    if rewrites.rewrites.is_empty() {
        return Ok((code.clone(), 0));
    }
    let mut positions = Vec::new();
    reserve(&mut positions, code.instructions.len())?;
    positions.resize(code.instructions.len(), 0_usize);
    let mut instructions = Vec::new();
    reserve(&mut instructions, code.instructions.len())?;
    let mut ip = 0;
    while ip < code.instructions.len() {
        positions[ip] = instructions.len();
        if let Some(rewrite) = rewrites.rewrites.get(&ip) {
            if rewrite.end <= ip || rewrite.end > code.instructions.len() {
                return Err(unsupported("native rewrite extent is inconsistent"));
            }
            reserve(&mut instructions, rewrite.instructions.len())?;
            instructions.extend(rewrite.instructions.iter().cloned());
            positions[ip + 1..rewrite.end].fill(instructions.len());
            ip = rewrite.end;
        } else {
            reserve(&mut instructions, 1)?;
            instructions.push(code.instructions[ip].clone());
            ip += 1;
        }
    }
    // An optional rewrite must not narrow the existing u32 branch domain.
    if u32::try_from(instructions.len()).is_err() {
        return Ok((code.clone(), 0));
    }
    for instruction in &mut instructions {
        match instruction {
            I::Jump(target) | I::JumpIfFalse(target) => {
                *target = remapped_target(&positions, *target)?;
            }
            I::SwitchVariant(jumps) => {
                let mut remapped = Vec::new();
                reserve(&mut remapped, jumps.len())?;
                for jump in jumps.iter() {
                    let mut jump = *jump;
                    jump.target = remapped_target(&positions, jump.target)?;
                    remapped.push(jump);
                }
                *jumps = remapped.into();
            }
            _ => {}
        }
    }
    Ok((
        NormalizedCode {
            parameter_count: code.parameter_count,
            local_count: code.local_count,
            instructions: instructions.into(),
        },
        rewrites.rewrites.len(),
    ))
}

fn remapped_target(positions: &[usize], target: u32) -> Result<u32> {
    positions
        .get(target as usize)
        .and_then(|target| u32::try_from(*target).ok())
        .ok_or_else(|| unsupported("native rewritten branch target is absent"))
}

#[derive(Clone, Copy)]
enum Input {
    I64,
    Bytes,
    List,
    Buffer(ParameterUse),
}

#[derive(Clone, Copy)]
enum Output {
    I64,
    Bool,
    Buffer,
}

impl Emitter<'_> {
    pub(super) fn operand_representation(
        state: Option<&State>,
        offset_from_top: usize,
    ) -> Result<Representation> {
        let Some(state) = state else {
            return Ok(Representation::Boxed);
        };
        state
            .operands()
            .len()
            .checked_sub(
                offset_from_top
                    .checked_add(1)
                    .ok_or_else(|| unsupported("native scalar operand offset overflowed"))?,
            )
            .and_then(|index| state.operands().get(index))
            .copied()
            .ok_or_else(|| unsupported("native scalar operand state is inconsistent"))
    }

    pub(super) fn convert_ax(&mut self, from: Representation, to: Representation) {
        match (from, to) {
            (Representation::I64, Representation::Boxed) => self.a.call(self.runtime.integer),
            (Representation::Boxed, Representation::I64) => self.a.load(Ax, Ax, 8),
            _ => {}
        }
    }

    pub(super) fn box_operands(&mut self, state: Option<&State>, count: usize) -> Result<()> {
        for offset in 0..count {
            if Self::operand_representation(state, offset)? == Representation::I64 {
                let offset = layout_offset(offset, 8, 0)?;
                self.a.load_offset(Ax, Sp, offset);
                self.a.call(self.runtime.integer);
                self.a.store_offset(Sp, offset, Ax);
            }
        }
        Ok(())
    }

    pub(super) fn direct_arguments(
        &mut self,
        target: usize,
        arguments: u32,
        state: Option<&State>,
    ) -> Result<()> {
        for parameter in 0..arguments as usize {
            let offset = arguments as usize - 1 - parameter;
            let from = Self::operand_representation(state, offset)?;
            let to = if self.raw_parameter(target, parameter) {
                Representation::I64
            } else {
                Representation::Boxed
            };
            if from != to {
                let offset = layout_offset(offset, 8, 0)?;
                self.a.load_offset(Ax, Sp, offset);
                self.convert_ax(from, to);
                self.a.store_offset(Sp, offset, Ax);
            }
        }
        Ok(())
    }

    pub(super) fn direct_result(&mut self, target: usize, optimized: bool) -> Result<()> {
        let function = self
            .program
            .functions
            .get(target)
            .ok_or_else(|| unsupported("native direct result has no signature"))?;
        let to = if optimized {
            Representation::of_type(self.program, function.result.ty)
                .ok_or_else(|| unsupported("native direct result type is absent"))?
        } else {
            Representation::Boxed
        };
        let from = if self.raw_result(target) {
            Representation::I64
        } else {
            Representation::Boxed
        };
        self.convert_ax(from, to);
        Ok(())
    }

    /// The direct ABI is stable even when optional local analysis declines.
    pub(super) fn box_fallback_parameters(&mut self, function: usize, count: u32) -> Result<()> {
        for parameter in 0..count as usize {
            if self.raw_parameter(function, parameter) {
                let offset = layout_offset(parameter, -8, -32)?;
                self.a.load_offset(Ax, Bp, offset);
                self.a.call(self.runtime.integer);
                self.a.store_offset(Bp, offset, Ax);
            }
        }
        Ok(())
    }

    /// Preserve the poll formerly reached by out-of-pool scalar allocation.
    /// Removed allocations do not increment passive request observations.
    fn scalar_result_checkpoint(&mut self) {
        let poll = self.a.label();
        let ready = self.a.label();
        self.a.group(7, Ax, -1);
        self.a.when(L, poll);
        self.a.group(7, Ax, 256);
        self.a.when(L, ready);
        self.a.bind(poll);
        self.runtime.checkpoint(&mut self.a);
        self.a.bind(ready);
    }

    fn scalar_signature(&self, target: usize, inputs: &[Input], output: Output) -> bool {
        let Some(function) = self.program.functions.get(target) else {
            return false;
        };
        function.effect == FunctionEffect::Pure
            && matches!(function.result.relation, ResultRelation::Value)
            && function.parameters.len() == inputs.len()
            && function
                .parameters
                .iter()
                .zip(inputs)
                .all(|(parameter, input)| {
                    let Some(ty) = self.program.types.get(&parameter.ty) else {
                        return false;
                    };
                    let (kind, use_mode) = match input {
                        Input::I64 => {
                            (matches!(ty.form, TypeForm::I64), ParameterUse::Unrestricted)
                        }
                        Input::Bytes => (
                            matches!(ty.form, TypeForm::Bytes),
                            ParameterUse::Unrestricted,
                        ),
                        Input::List => (
                            matches!(ty.form, TypeForm::List { .. }),
                            ParameterUse::Unrestricted,
                        ),
                        Input::Buffer(mode) => (matches!(ty.form, TypeForm::ByteBuffer), *mode),
                    };
                    kind && parameter.use_mode == use_mode
                })
            && self
                .program
                .types
                .get(&function.result.ty)
                .is_some_and(|ty| match output {
                    Output::I64 => matches!(ty.form, TypeForm::I64),
                    Output::Bool => matches!(ty.form, TypeForm::Bool),
                    Output::Buffer => matches!(ty.form, TypeForm::ByteBuffer),
                })
    }

    /// Inline only exact pure intrinsic signatures. The operand tuple remains
    /// rooted on the stack through checks, allocation and recovery safe points.
    pub(super) fn scalar_intrinsic(
        &mut self,
        target: usize,
        arguments: u32,
        state: &State,
    ) -> Result<bool> {
        let Some(function) = self.program.functions.get(target) else {
            return Ok(false);
        };
        let NormalizedFunctionBody::External(name) = &function.body else {
            return Ok(false);
        };
        let (inputs, output): (&[Input], Output) = match name.as_str() {
            "core.i64.add" | "core.i64.subtract" | "core.i64.multiply" | "core.i64.divide" => {
                (&[Input::I64, Input::I64], Output::I64)
            }
            "core.i64.equal" | "core.i64.less" | "core.i64.less-equal" => {
                (&[Input::I64, Input::I64], Output::Bool)
            }
            "core.bytes.read-i64-le" | "core.bytes.get" => {
                (&[Input::Bytes, Input::I64], Output::I64)
            }
            "core.bytes.length" => (&[Input::Bytes], Output::I64),
            "core.list.length" => (&[Input::List], Output::I64),
            "core.buffer.length" => (&[Input::Buffer(ParameterUse::Borrow)], Output::I64),
            "core.buffer.get" => (
                &[Input::I64, Input::Buffer(ParameterUse::Borrow)],
                Output::I64,
            ),
            "core.buffer.append-i64-le" => (
                &[Input::I64, Input::Buffer(ParameterUse::Consume)],
                Output::Buffer,
            ),
            _ => return Ok(false),
        };
        if arguments as usize != inputs.len() || !self.scalar_signature(target, inputs, output) {
            return Ok(false);
        }
        for (parameter, input) in inputs.iter().enumerate() {
            if !matches!(input, Input::I64)
                && Self::operand_representation(Some(state), inputs.len() - 1 - parameter)?
                    != Representation::Boxed
            {
                return Ok(false);
            }
        }
        // External entry polling precedes its arithmetic/range checks.
        self.runtime.checkpoint(&mut self.a);
        for (parameter, input) in inputs.iter().enumerate() {
            let offset = inputs.len() - 1 - parameter;
            let register = if parameter == 0 { Ax } else { Cx };
            self.a
                .load_offset(register, Sp, layout_offset(offset, 8, 0)?);
            if matches!(input, Input::I64)
                && Self::operand_representation(Some(state), offset)? == Representation::Boxed
            {
                self.a.load(register, register, 8);
            }
        }
        match name.as_str() {
            "core.bytes.read-i64-le" => self.bytes_read_i64_le_word(),
            "core.bytes.length" | "core.list.length" | "core.buffer.length" => {
                self.a.load(Ax, Ax, 8);
            }
            "core.bytes.get" => {
                self.a.load(Dx, Ax, 8);
                self.a.bin(0x39, Cx, Dx);
                self.a.when(AE, self.runtime.bytes_index);
                self.a.load(Ax, Ax, 16);
                self.a.bin(0x01, Ax, Cx);
                self.a.load_byte(Ax, Ax);
            }
            "core.buffer.get" => {
                self.a.load(Dx, Cx, 8);
                self.a.bin(0x39, Ax, Dx);
                self.a.when(AE, self.runtime.buffer_index);
                self.a.load(Cx, Cx, 16);
                self.a.bin(0x01, Cx, Ax);
                self.a.load_byte(Ax, Cx);
            }
            "core.buffer.append-i64-le" => self.buffer_append_i64_le_word(),
            _ => self.i64_binary(name.as_str()),
        }
        match output {
            Output::I64 => self.scalar_result_checkpoint(),
            Output::Bool => self.a.call(self.runtime.boolean),
            Output::Buffer => {}
        }
        self.a
            .group_wide(0, Sp, layout_extent(arguments as usize, 8, 0)?);
        self.a.push(Ax);
        Ok(true)
    }

    /// Raw Ax/Cx arithmetic shared by boxed external entries and direct lowering.
    pub(super) fn i64_binary(&mut self, name: &str) {
        match name {
            "core.i64.add" => {
                self.a.bin(0x01, Ax, Cx);
                self.a.when(O, self.runtime.overflow);
            }
            "core.i64.subtract" => {
                self.a.bin(0x29, Ax, Cx);
                self.a.when(O, self.runtime.overflow);
            }
            "core.i64.multiply" => {
                self.a.imul(Ax, Cx);
                self.a.when(O, self.runtime.overflow);
            }
            "core.i64.divide" => {
                self.a.bin(0x85, Cx, Cx);
                self.a.when(E, self.runtime.division);
                self.a.imm(Dx, i64::MIN as u64);
                self.a.bin(0x39, Ax, Dx);
                let divide = self.a.label();
                self.a.when(NE, divide);
                self.a.group(7, Cx, -1);
                self.a.when(E, self.runtime.division);
                self.a.bind(divide);
                self.a.bytes(&[0x48, 0x99]);
                self.a.unary(7, Cx);
            }
            _ => {
                self.a.bin(0x39, Ax, Cx);
                self.a.set(if name == "core.i64.equal" {
                    E
                } else if name == "core.i64.less" {
                    L
                } else {
                    LE
                });
            }
        }
    }
}
