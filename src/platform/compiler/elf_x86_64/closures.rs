//! Current native callable values and frame-reusing graph transfers.
//!
//! Value: [tag=6, immutable target metadata pointer, bound count, flat prefix...]
//! Target: [boxed entry, boxed-input graph body or zero, arity, locals, raw-I64 result].
//! Prefix values are in authored order; operand-stack values are reversed.
use super::asm::{Asm, Label, R::*, *};
use super::runtime::Runtime;
use super::scalar_flow::State;
use super::{Emitter, Result, layout_extent, unsupported};
use crate::platform::execution::normalized::prepare::{NormalizedCode, NormalizedFunctionBody};
use crate::platform::kernel::ParameterUse;

pub(super) const VALUE_HEADER: u64 = 24;

/// A physical cancellation interval, never an admission or iteration limit.
fn word_checkpoint(a: &mut Asm, runtime: &Runtime) {
    let ready = a.label();
    a.mov(R11, Cx);
    a.group(4, R11, 8191);
    a.when(NE, ready);
    runtime.checkpoint(a);
    a.bind(ready);
}

fn pointer_add(a: &mut Asm, runtime: &Runtime, destination: R, amount: u64) {
    a.group_wide(0, destination, amount);
    a.when(B, runtime.capacity);
}

fn pointer_subtract(a: &mut Asm, runtime: &Runtime, destination: R, amount: u64) {
    a.group_wide(5, destination, amount);
    a.when(B, runtime.capacity);
}

fn scaled_words(a: &mut Asm, runtime: &Runtime, destination: R, words: R) {
    a.mov(destination, words);
    a.shift(5, destination, 61);
    a.bin(0x85, destination, destination);
    a.when(NE, runtime.capacity);
    a.mov(destination, words);
    a.shift(4, destination, 3);
}

/// Resolve a compiler-constructed callable without changing the operand stack.
/// Bx=original SP, R12=value, R13=prefix count, R8=target, R9=full arity.
/// The ABI is not a sandbox for arbitrary pointers supplied by foreign code.
fn prepare(a: &mut Asm, runtime: &Runtime, arguments: u32, binding: bool) {
    runtime.checkpoint(a);
    a.mov(Bx, Sp);
    a.mov(Ax, Bx);
    pointer_add(a, runtime, Ax, (u64::from(arguments) + 1) * 8);
    a.mov(R12, Bx);
    pointer_add(a, runtime, R12, u64::from(arguments) * 8);
    a.load(R12, R12, 0);
    a.bin(0x85, R12, R12);
    a.when(E, runtime.callable);
    a.mov(Ax, R12);
    pointer_add(a, runtime, Ax, VALUE_HEADER);
    a.load(Ax, R12, 0);
    a.group(7, Ax, 6);
    a.when(NE, runtime.callable);
    a.load(R8, R12, 8);
    a.bin(0x85, R8, R8);
    a.when(E, runtime.callable);
    a.mov(Ax, R8);
    pointer_add(a, runtime, Ax, 40);
    a.load(R13, R12, 16);
    a.load(R9, R8, 16);
    a.mov(Ax, R13);
    pointer_add(a, runtime, Ax, u64::from(arguments));
    if binding {
        a.bin(0x39, R9, Ax);
        a.when(B, runtime.callable);
    } else {
        a.bin(0x39, Ax, R9);
        a.when(NE, runtime.callable);
    }
    // Validate the complete old prefix extent before reading any prefix value.
    scaled_words(a, runtime, Dx, R13);
    pointer_add(a, runtime, Dx, VALUE_HEADER);
    a.bin(0x01, Dx, R12);
    a.when(B, runtime.capacity);
}

/// Copy authored-order values from ascending Si into descending stack slots Di.
/// Cx counts words; both complete extents were checked before this helper.
fn prefix_to_slots(a: &mut Asm, runtime: &Runtime) {
    let next = a.label();
    let done = a.label();
    runtime.checkpoint(a);
    a.bin(0x85, Cx, Cx);
    a.when(E, done);
    a.bind(next);
    word_checkpoint(a, runtime);
    a.load(Ax, Si, 0);
    a.store(Di, 0, Ax);
    a.group(5, Cx, 1);
    a.when(E, done);
    a.group(0, Si, 8);
    a.group(5, Di, 8);
    a.jump(next);
    a.bind(done);
}

/// Copy reversed operand-stack values into an ascending immutable prefix.
fn arguments_to_prefix(a: &mut Asm, runtime: &Runtime) {
    let next = a.label();
    let done = a.label();
    runtime.checkpoint(a);
    a.bin(0x85, Cx, Cx);
    a.when(E, done);
    a.bind(next);
    word_checkpoint(a, runtime);
    a.load(Ax, Si, 0);
    a.store(Di, 0, Ax);
    a.group(5, Cx, 1);
    a.when(E, done);
    a.group(5, Si, 8);
    a.group(0, Di, 8);
    a.jump(next);
    a.bind(done);
}

pub(super) fn bind(a: &mut Asm, runtime: &Runtime, arguments: u32) {
    if arguments == 0 {
        // The complete callee was already evaluated and statically admitted.
        // Preserve its identity and prefix, without even reserving an environment.
        runtime.checkpoint(a);
        return;
    }
    prepare(a, runtime, arguments, true);
    a.mov(Di, R13);
    pointer_add(a, runtime, Di, u64::from(arguments));
    scaled_words(a, runtime, Dx, Di);
    a.mov(Di, Dx);
    pointer_add(a, runtime, Di, VALUE_HEADER);
    a.call(runtime.alloc);
    a.mov(Bx, Ax);
    a.imm(Cx, 6);
    a.store(Bx, 0, Cx);
    a.load(Cx, R12, 8);
    a.store(Bx, 8, Cx);
    a.mov(Cx, R13);
    a.group_wide(0, Cx, u64::from(arguments)); // Exact sum was checked before allocation.
    a.store(Bx, 16, Cx);

    a.lea(Si, R12, VALUE_HEADER as i32);
    a.lea(Di, Bx, VALUE_HEADER as i32);
    a.mov(Cx, R13);
    a.shift(4, Cx, 3);
    runtime.copy_bytes(a);

    // copy_bytes advances Di past the old prefix. New arguments are read in
    // authored order from the unchanged original operand stack.
    a.mov(Si, Sp);
    pointer_add(a, runtime, Si, (u64::from(arguments) - 1) * 8);
    a.imm(Cx, u64::from(arguments));
    arguments_to_prefix(a, runtime);
    a.group_wide(0, Sp, (u64::from(arguments) + 1) * 8);
    a.push(Bx);
}

pub(super) fn invoke(a: &mut Asm, runtime: &Runtime, arguments: u32) {
    prepare(a, runtime, arguments, false);
    // Reserve the whole outgoing argument window before polling/copying, so
    // helper call records cannot overwrite an argument under construction.
    scaled_words(a, runtime, Dx, R9);
    a.mov(Ax, Sp);
    a.bin(0x29, Ax, Dx);
    a.when(B, runtime.capacity);
    a.mov(Sp, Ax);

    a.mov(Cx, R13);
    a.lea(Si, R12, VALUE_HEADER as i32);
    a.mov(Di, Bx);
    // A zero prefix never dereferences Di. For a nonempty prefix the checked
    // outgoing extent proves Bx >= 8.
    let no_prefix = a.label();
    a.bin(0x85, Cx, Cx);
    a.when(E, no_prefix);
    a.group(5, Di, 8);
    prefix_to_slots(a, runtime);
    a.bind(no_prefix);

    a.mov(Si, Bx);
    a.mov(Di, Sp);
    a.imm(Cx, u64::from(arguments) * 8);
    runtime.copy_bytes(a);
    a.load(Ax, R12, 8);
    a.load(Ax, Ax, 0);
    a.bin(0x85, Ax, Ax);
    a.when(E, runtime.callable);
    a.indirect(Ax);
    // Every emitted graph/external prologue preserves Bx. The original caller
    // record is independent of the ultimate target's arity along a tail chain.
    a.mov(Sp, Bx);
    a.group_wide(0, Sp, (u64::from(arguments) + 1) * 8);
}

/// Initialize all new nonparameter slots before any runtime owner scan can read
/// them. Zero is an internal empty slot, never a source-language value.
pub(super) fn initialize_locals(
    a: &mut Asm,
    runtime: &Runtime,
    parameters: u32,
    locals: u32,
) -> Result<()> {
    if parameters > locals {
        return Err(unsupported(
            "native parameter count exceeds its local frame",
        ));
    }
    if parameters == locals {
        return Ok(());
    }
    let next = a.label();
    let done = a.label();
    a.mov(Di, Bp);
    pointer_subtract(a, runtime, Di, 32 + u64::from(parameters) * 8);
    a.imm(Cx, u64::from(locals - parameters));
    a.imm(Ax, 0);
    runtime.checkpoint(a);
    a.bind(next);
    word_checkpoint(a, runtime);
    a.store(Di, 0, Ax);
    a.group(5, Cx, 1);
    a.when(E, done);
    a.group(5, Di, 8);
    a.jump(next);
    a.bind(done);
    Ok(())
}

/// A heap-free scan of a known-initialized slot interval. ByteBuffer is the
/// current native backend's only owned value class (tag 8). Borrowed parameter
/// intervals are excluded by the emitter; consumed slots are already empty.
pub(super) fn owner_range(
    a: &mut Asm,
    runtime: &Runtime,
    start: u32,
    end: u32,
    live: Label,
) -> Result<()> {
    if start > end {
        return Err(unsupported(
            "native owner scan has an inverted slot interval",
        ));
    }
    if start == end {
        return Ok(());
    }
    let next = a.label();
    let empty = a.label();
    let done = a.label();
    a.mov(Si, Bp);
    pointer_subtract(a, runtime, Si, 32 + u64::from(start) * 8);
    a.imm(Cx, u64::from(end - start));
    runtime.checkpoint(a);
    a.bind(next);
    word_checkpoint(a, runtime);
    a.load(Ax, Si, 0);
    a.bin(0x85, Ax, Ax);
    a.when(E, empty);
    a.load(Dx, Ax, 0);
    a.group(7, Dx, 8);
    a.when(E, live);
    a.bind(empty);
    a.group(5, Cx, 1);
    a.when(E, done);
    a.group(5, Si, 8);
    a.jump(next);
    a.bind(done);
    Ok(())
}

/// Transfer after complete callsite custody/target checks, with no new CALL.
///
/// Inputs: Bx=old operand SP, R8=graph body, R9=full arity, R10=local count,
/// R12=immutable prefix base, R13=prefix count. RBP and the original saved
/// BX/R12/R13, caller RBP and return record remain fixed. R14/R15 are untouched.
pub(super) fn tail_frame(
    a: &mut Asm,
    runtime: &Runtime,
    arguments: u32,
    old_locals: u32,
    indirect: bool,
) {
    runtime.checkpoint(a);
    a.bin(0x85, R8, R8);
    a.when(E, runtime.callable);
    a.mov(Ax, R13);
    pointer_add(a, runtime, Ax, u64::from(arguments));
    a.bin(0x39, Ax, R9);
    a.when(NE, runtime.callable);
    a.bin(0x39, R10, R9);
    a.when(B, runtime.callable);

    // The terminal stack contains only its complete explicit call tuple.
    a.mov(Ax, Bx);
    pointer_add(
        a,
        runtime,
        Ax,
        (u64::from(arguments) + u64::from(indirect)) * 8,
    );
    a.mov(Dx, Bp);
    pointer_subtract(a, runtime, Dx, 24 + u64::from(old_locals) * 8);
    a.bin(0x39, Ax, Dx);
    a.when(NE, runtime.callable);

    // The immutable prefix extent is separate from every stack window.
    scaled_words(a, runtime, Ax, R13);
    a.bin(0x01, Ax, R12);
    a.when(B, runtime.capacity);

    // Compute the complete target frame before changing SP or any source slot.
    scaled_words(a, runtime, Ax, R10);
    pointer_add(a, runtime, Ax, 24);
    a.mov(Dx, Bp);
    a.bin(0x29, Dx, Ax);
    a.when(B, runtime.capacity);
    let reserved = a.label();
    a.bin(0x39, Sp, Dx);
    a.when(B, reserved);
    a.mov(Sp, Dx);
    a.bind(reserved);

    // Explicit arguments move first: copying the prefix first can overwrite
    // still-live source operands when destination locals overlap that window.
    if arguments != 0 {
        scaled_words(a, runtime, Ax, R9);
        a.mov(Di, Bp);
        pointer_subtract(a, runtime, Di, 24);
        a.bin(0x29, Di, Ax);
        a.when(B, runtime.capacity);
        a.mov(Si, Bx);
        a.imm(Cx, u64::from(arguments));
        let forward = a.label();
        let reverse = a.label();
        let copied = a.label();
        a.bin(0x39, Di, Si);
        a.when(B, forward);
        a.when(E, copied);
        pointer_add(a, runtime, Si, (u64::from(arguments) - 1) * 8);
        pointer_add(a, runtime, Di, (u64::from(arguments) - 1) * 8);
        a.bind(reverse);
        word_checkpoint(a, runtime);
        a.load(Ax, Si, 0);
        a.store(Di, 0, Ax);
        a.group(5, Cx, 1);
        a.when(E, copied);
        a.group(5, Si, 8);
        a.group(5, Di, 8);
        a.jump(reverse);
        a.bind(forward);
        word_checkpoint(a, runtime);
        a.load(Ax, Si, 0);
        a.store(Di, 0, Ax);
        a.group(5, Cx, 1);
        a.when(E, copied);
        a.group(0, Si, 8);
        a.group(0, Di, 8);
        a.jump(forward);
        a.bind(copied);
    }

    let no_prefix = a.label();
    a.mov(Cx, R13);
    a.bin(0x85, Cx, Cx);
    a.when(E, no_prefix);
    a.mov(Si, R12);
    a.mov(Di, Bp);
    pointer_subtract(a, runtime, Di, 32);
    prefix_to_slots(a, runtime);
    a.bind(no_prefix);

    // The parameter windows now survive independently of all old local slots.
    let clear = a.label();
    let cleared = a.label();
    a.mov(Cx, R10);
    a.bin(0x29, Cx, R9);
    a.bin(0x85, Cx, Cx);
    a.when(E, cleared);
    a.mov(Di, Dx);
    a.imm(Ax, 0);
    runtime.checkpoint(a);
    a.bind(clear);
    word_checkpoint(a, runtime);
    a.store(Di, 0, Ax);
    a.group(5, Cx, 1);
    a.when(E, cleared);
    a.group(0, Di, 8);
    a.jump(clear);
    a.bind(cleared);
    a.mov(Sp, Dx);
    a.jump_register(R8);
}

impl Emitter<'_> {
    fn owner_fallback(
        &mut self,
        function: usize,
        code: &NormalizedCode,
        live: Label,
        state: Option<&State>,
    ) -> Result<()> {
        let parameters = &self.program.functions[function].parameters;
        let mut start = 0;
        for index in 0..code.local_count {
            let borrowed = parameters
                .get(index as usize)
                .is_some_and(|parameter| parameter.use_mode == ParameterUse::Borrow);
            // Raw scalars and known-empty slots are never pointer candidates.
            // Boxed or uncertain slots retain the original live-owner check.
            if borrowed || state.is_some_and(|state| !state.may_own(index)) {
                owner_range(&mut self.a, &self.runtime, start, index, live)?;
                start = index + 1;
            }
        }
        owner_range(&mut self.a, &self.runtime, start, code.local_count, live)?;
        Ok(())
    }

    pub(super) fn tail_call(
        &mut self,
        function: usize,
        code: &NormalizedCode,
        target: usize,
        arguments: u32,
        state: Option<&State>,
    ) -> Result<()> {
        if let Some(body) = self.bodies.get(target).copied().flatten()
            && self.raw_result(function) == self.raw_result(target)
        {
            let target_function = self
                .program
                .functions
                .get(target)
                .ok_or_else(|| unsupported("native tail target is absent"))?;
            let NormalizedFunctionBody::Code(target_code) = &target_function.body else {
                return Err(unsupported("native graph body has no code"));
            };
            let parameters = target_function.parameter_count;
            let locals = target_code.local_count;
            if parameters != arguments || locals < parameters {
                return Err(unsupported("native tail target has an incompatible frame"));
            }
            let ordinary = self.a.label();
            self.runtime.checkpoint(&mut self.a);
            self.owner_fallback(function, code, ordinary, state)?;
            self.a.mov(Bx, Sp);
            self.a.address(R8, body);
            self.a.imm(R9, u64::from(parameters));
            self.a.imm(R10, u64::from(locals));
            self.a.imm(R12, 0);
            self.a.imm(R13, 0);
            tail_frame(
                &mut self.a,
                &self.runtime,
                arguments,
                code.local_count,
                false,
            );
            self.a.bind(ordinary);
        }
        self.a.call(self.label(target)?);
        self.a
            .group_wide(0, Sp, layout_extent(arguments as usize, 8, 0)?);
        self.direct_result(target, state.is_some())?;
        self.a.push(Ax);
        // The retained normalized continuation performs lexical cleanup/Return.
        Ok(())
    }

    pub(super) fn tail_invoke(
        &mut self,
        function: usize,
        code: &NormalizedCode,
        arguments: u32,
        state: Option<&State>,
    ) -> Result<()> {
        let ordinary = self.a.label();
        self.runtime.checkpoint(&mut self.a);
        self.owner_fallback(function, code, ordinary, state)?;
        prepare(&mut self.a, &self.runtime, arguments, false);
        // A tail hint proves a terminal value, not its machine representation.
        // The boxed ordinary wrapper remains the fallback for unlike results.
        self.a.load(Ax, R8, 32);
        self.a.group(7, Ax, i32::from(self.raw_result(function)));
        self.a.when(NE, ordinary);
        self.a.load(R10, R8, 24);
        self.a.load(R8, R8, 8);
        self.a.bin(0x85, R8, R8);
        self.a.when(E, ordinary);
        pointer_add(&mut self.a, &self.runtime, R12, VALUE_HEADER);
        tail_frame(
            &mut self.a,
            &self.runtime,
            arguments,
            code.local_count,
            true,
        );
        self.a.bind(ordinary);
        invoke(&mut self.a, &self.runtime, arguments);
        self.a.push(Ax);
        // Terminal externals add one bounded call; live-owner fallback follows
        // the original continuation instead of bypassing its cleanup.
        Ok(())
    }
}
