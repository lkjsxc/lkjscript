//! Untrusted IR-to-IR inlining. This module grants no checked-program authority.
use crate::ir::{Function, Instruction, Op, Place, Program, Projection};

/// The measured default is selected separately from the transformation itself.
pub fn default_limit() -> usize { 0 }

pub struct Rewrite {
    pub program: Option<Program>,
    pub calls: usize,
    pub functions: usize,
}
#[derive(Clone, Copy)]
struct Shape {
    direct: bool,
}

fn shape(function: &Function, limit: usize) -> Option<Shape> {
    let mut returns = 0usize;
    for instruction in &function.body {
        match instruction.op {
            Op::Call { .. } => return None,
            Op::Return { .. } => returns += 1,
            _ => {}
        }
    }
    if returns == 0 { return None; }
    let direct = returns == 1
        && matches!(function.body.last().map(|instruction: &Instruction| &instruction.op), Some(Op::Return { .. }));
    let mut cost = function.params.checked_add(function.body.len())?;
    if !direct {
        cost = cost.checked_add(returns)?.checked_add(2)?;
    }
    if cost > limit { return None; }
    Some(Shape { direct })
}
fn locals(values: &[usize], base: usize) -> Vec<usize> {
    let mut result = Vec::new();
    for value in values { result.push(base + *value); }
    result
}
fn place(value: &Place, base: usize) -> Place {
    let mut projections = Vec::new();
    for projection in &value.projections {
        projections.push(match projection {
            Projection::Deref => Projection::Deref,
            Projection::Field(field) => Projection::Field(*field),
            Projection::VariantField(variant, field) => Projection::VariantField(*variant, *field),
            Projection::Index(local) => Projection::Index(base + *local),
        });
    }
    Place { local: base + value.local, projections }
}
fn remap(op: &Op, base: usize) -> Op {
    match op {
        Op::Text { dst, bytes } => Op::Text { dst: base + *dst, bytes: bytes.clone() },
        Op::Int { dst, value } => Op::Int { dst: base + *dst, value: *value },
        Op::Float { dst, bits } => Op::Float { dst: base + *dst, bits: *bits },
        Op::Unit { dst } => Op::Unit { dst: base + *dst },
        Op::Read { dst, place: source, moving } => Op::Read {
            dst: base + *dst, place: place(source, base), moving: *moving,
        },
        Op::Borrow { dst, place: source, mutable } => Op::Borrow {
            dst: base + *dst, place: place(source, base), mutable: *mutable,
        },
        Op::SliceBorrow { dst, place: source, mutable } => Op::SliceBorrow {
            dst: base + *dst, place: place(source, base), mutable: *mutable,
        },
        Op::ViewRange { dst, place: source, start, end, inclusive, mutable } => Op::ViewRange {
            dst: base + *dst, place: place(source, base), start: base + *start,
            end: base + *end, inclusive: *inclusive, mutable: *mutable,
        },
        Op::Array { dst, elements } => Op::Array { dst: base + *dst, elements: locals(elements, base) },
        Op::Assign { place: target, src } => Op::Assign { place: place(target, base), src: base + *src },
        Op::Drop { place: target } => Op::Drop { place: place(target, base) },
        Op::DropIf { place: target } => Op::DropIf { place: place(target, base) },
        Op::Unary { dst, op, src } => Op::Unary { dst: base + *dst, op: *op, src: base + *src },
        Op::Binary { dst, op, left, right } => Op::Binary {
            dst: base + *dst, op: *op, left: base + *left, right: base + *right,
        },
        Op::Call { dst, function, args } => Op::Call {
            dst: base + *dst, function: *function, args: locals(args, base),
        },
        Op::Struct { dst, fields } => Op::Struct { dst: base + *dst, fields: locals(fields, base) },
        Op::Variant { dst, variant, fields } => Op::Variant {
            dst: base + *dst, variant: *variant, fields: locals(fields, base),
        },
        Op::Discriminant { dst, src } => Op::Discriminant { dst: base + *dst, src: base + *src },
        Op::TagTest { dst, place: source, variant } => Op::TagTest {
            dst: base + *dst, place: place(source, base), variant: *variant,
        },
        Op::Intrinsic { dst, name, args } => Op::Intrinsic {
            dst: base + *dst, name: *name, args: locals(args, base),
        },
        Op::If { condition } => Op::If { condition: base + *condition },
        Op::Else => Op::Else,
        Op::End => Op::End,
        Op::Loop => Op::Loop,
        Op::Break => Op::Break,
        Op::Continue => Op::Continue,
        Op::BreakTo { depth } => Op::BreakTo { depth: *depth },
        Op::ContinueTo { depth } => Op::ContinueTo { depth: *depth },
        Op::Return { src } => Op::Return { src: base + *src },
        Op::Trap { message } => Op::Trap { message: message.clone() },
    }
}
fn at(call: &Instruction, op: Op) -> Instruction {
    Instruction { op, file: call.file, line: call.line, column: call.column }
}
fn copied(callee: &Function, instruction: &Instruction, op: Op) -> Instruction {
    Instruction {
        op, file: Some(instruction.file.unwrap_or(callee.file)),
        line: instruction.line, column: instruction.column,
    }
}
fn append(
    output: &mut Vec<Instruction>, callee: &Function, call: &Instruction,
    args: &[usize], destination: usize, base: usize, shape: Shape,
) {
    for (index, argument) in args.iter().enumerate() {
        output.push(at(call, Op::Read {
            dst: base + index, place: Place::local(*argument), moving: true,
        }));
    }
    if !shape.direct { output.push(at(call, Op::Loop)); }
    let mut controls = Vec::new();
    let mut loops = 0usize;
    for instruction in &callee.body {
        if let Op::Return { src } = instruction.op {
            output.push(copied(callee, instruction, Op::Read {
                dst: destination, place: Place::local(base + src), moving: true,
            }));
            if !shape.direct {
                output.push(copied(callee, instruction, Op::BreakTo { depth: loops }));
            }
            continue;
        }
        match instruction.op {
            Op::Loop => { loops += 1; controls.push(true); }
            Op::If { .. } => controls.push(false),
            Op::End => {
                if let Some(is_loop) = controls.pop() {
                    if is_loop { loops -= 1; }
                }
            }
            _ => {}
        }
        output.push(copied(callee, instruction, remap(&instruction.op, base)));
    }
    if !shape.direct { output.push(at(call, Op::End)); }
}
fn rewrite(
    function: &Function, functions: &[Function], eligible: &[Option<Shape>],
) -> Option<(Function, usize)> {
    let mut calls = 0usize;
    for instruction in &function.body {
        if let Op::Call { function, .. } = instruction.op {
            if eligible[function].is_some() { calls += 1; }
        }
    }
    if calls == 0 { return None; }
    let mut rewritten = function.clone();
    rewritten.body.clear();
    for instruction in &function.body {
        if let Op::Call { dst, function, args } = &instruction.op {
            if let Some(shape) = eligible[*function] {
                let callee = &functions[*function];
                let base = rewritten.locals.len();
                rewritten.locals.extend_from_slice(&callee.locals);
                append(&mut rewritten.body, callee, instruction, args, *dst, base, shape);
                continue;
            }
        }
        rewritten.body.push(instruction.clone());
    }
    Some((rewritten, calls))
}

/// Threshold zero performs no transformation. Other thresholds bound each
/// expanded call's IR instruction count, including parameter/result transfers.
pub fn inline(program: &Program, threshold: usize) -> Rewrite {
    let mut report = Rewrite { program: None, calls: 0, functions: 0 };
    if threshold == 0 { return report; }
    let count = program.functions.len();
    let mut callers = vec![Vec::new(); count];
    let mut remaining = vec![0usize; count];
    for (index, function) in program.functions.iter().enumerate() {
        for instruction in &function.body {
            if let Op::Call { function, .. } = instruction.op {
                if function >= count { return report; }
                callers[function].push(index);
                remaining[index] += 1;
            }
        }
    }
    let mut ready = Vec::new();
    let mut possible = false;
    for index in 0..count {
        if remaining[index] == 0 {
            ready.push(index);
            if !callers[index].is_empty() && shape(&program.functions[index], threshold).is_some() {
                possible = true;
            }
        }
    }
    if !possible { return report; }
    let mut candidate = program.clone();
    let mut eligible: Vec<Option<Shape>> = vec![None; count];
    let mut processed = vec![false; count];
    let mut cursor = 0usize;
    while cursor < ready.len() {
        let index = ready[cursor];
        cursor += 1;
        if let Some((function, calls)) = rewrite(&candidate.functions[index], &candidate.functions, &eligible) {
            candidate.functions[index] = function;
            report.calls += calls;
            report.functions += 1;
        }
        eligible[index] = shape(&candidate.functions[index], threshold);
        processed[index] = true;
        for caller in &callers[index] {
            remaining[*caller] -= 1;
            if remaining[*caller] == 0 { ready.push(*caller); }
        }
    }
    // A cycle cannot become an eligible leaf. Its callers can still use leaves
    // already processed outside that cycle, once, without new graph machinery.
    for index in 0..count {
        if !processed[index] {
            if let Some((function, calls)) = rewrite(&candidate.functions[index], &candidate.functions, &eligible) {
                candidate.functions[index] = function;
                report.calls += calls;
                report.functions += 1;
            }
        }
    }
    if report.calls != 0 { report.program = Some(candidate); }
    report
}
