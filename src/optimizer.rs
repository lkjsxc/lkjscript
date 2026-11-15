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
// Every operand and destination participates, including indices embedded in a
// place. Address-taking is deliberately conservative across the whole body.
struct Uses {
    first: Vec<Option<usize>>,
    last: Vec<Option<usize>>,
    addressed: Vec<bool>,
}
impl Uses {
    fn new(function: &Function) -> Self {
        let mut uses = Self {
            first: vec![None; function.locals.len()],
            last: vec![None; function.locals.len()],
            addressed: vec![false; function.locals.len()],
        };
        for (at, instruction) in function.body.iter().enumerate() {
            uses.instruction(&instruction.op, at);
        }
        uses
    }
    fn local(&mut self, local: usize, at: usize) {
        if self.first[local].is_none() { self.first[local] = Some(at); }
        self.last[local] = Some(at);
    }
    fn place(&mut self, place: &Place, at: usize) {
        self.local(place.local, at);
        for projection in &place.projections {
            if let Projection::Index(local) = projection { self.local(*local, at); }
        }
    }
    fn instruction(&mut self, op: &Op, at: usize) {
        match op {
            Op::Text { dst, .. } | Op::Int { dst, .. } | Op::Float { dst, .. }
                | Op::Unit { dst } => self.local(*dst, at),
            Op::Read { dst, place, .. } => { self.local(*dst, at); self.place(place, at); }
            Op::Borrow { dst, place, .. } | Op::SliceBorrow { dst, place, .. } => {
                self.local(*dst, at);
                self.place(place, at);
                self.addressed[place.local] = true;
            }
            Op::ViewRange { dst, place, start, end, .. } => {
                self.local(*dst, at);
                self.place(place, at);
                self.local(*start, at);
                self.local(*end, at);
                self.addressed[place.local] = true;
            }
            Op::Array { dst, elements: values } | Op::Struct { dst, fields: values }
                | Op::Variant { dst, fields: values, .. } | Op::Call { dst, args: values, .. }
                | Op::Intrinsic { dst, args: values, .. } => {
                self.local(*dst, at);
                for local in values { self.local(*local, at); }
            }
            Op::Assign { place, src } => { self.place(place, at); self.local(*src, at); }
            Op::Drop { place } | Op::DropIf { place } => self.place(place, at),
            Op::Unary { dst, src, .. } | Op::Discriminant { dst, src } => {
                self.local(*dst, at); self.local(*src, at);
            }
            Op::Binary { dst, left, right, .. } => {
                self.local(*dst, at); self.local(*left, at); self.local(*right, at);
            }
            Op::TagTest { dst, place, .. } => { self.local(*dst, at); self.place(place, at); }
            Op::If { condition } => self.local(*condition, at),
            Op::Return { src } => self.local(*src, at),
            Op::Else | Op::End | Op::Loop | Op::Break | Op::Continue
                | Op::BreakTo { .. } | Op::ContinueTo { .. } | Op::Trap { .. } => {}
        }
    }
}
fn destination(op: &Op) -> Option<usize> {
    match op {
        Op::Text { dst, .. } | Op::Int { dst, .. } | Op::Float { dst, .. }
            | Op::Unit { dst } | Op::Read { dst, .. } | Op::Borrow { dst, .. }
            | Op::SliceBorrow { dst, .. } | Op::ViewRange { dst, .. }
            | Op::Array { dst, .. } | Op::Unary { dst, .. } | Op::Binary { dst, .. }
            | Op::Call { dst, .. } | Op::Struct { dst, .. } | Op::Variant { dst, .. }
            | Op::Discriminant { dst, .. } | Op::TagTest { dst, .. }
            | Op::Intrinsic { dst, .. } => Some(*dst),
        Op::Assign { .. } | Op::Drop { .. } | Op::DropIf { .. } | Op::If { .. }
            | Op::Else | Op::End | Op::Loop | Op::Break | Op::Continue
            | Op::BreakTo { .. } | Op::ContinueTo { .. } | Op::Return { .. }
            | Op::Trap { .. } => None,
    }
}
fn place_reads(place: &Place, local: usize) -> bool {
    if place.local == local { return true; }
    for projection in &place.projections {
        if let Projection::Index(index) = projection {
            if *index == local { return true; }
        }
    }
    false
}
fn reads(op: &Op, local: usize) -> bool {
    match op {
        Op::Text { .. } | Op::Int { .. } | Op::Float { .. } | Op::Unit { .. } => false,
        Op::Read { place, .. } | Op::Borrow { place, .. } | Op::SliceBorrow { place, .. }
            | Op::Drop { place } | Op::DropIf { place } | Op::TagTest { place, .. } => place_reads(place, local),
        Op::ViewRange { place, start, end, .. } => place_reads(place, local) || *start == local || *end == local,
        Op::Array { elements: values, .. } | Op::Struct { fields: values, .. }
            | Op::Variant { fields: values, .. } | Op::Call { args: values, .. }
            | Op::Intrinsic { args: values, .. } => values.contains(&local),
        Op::Assign { place, src } => place_reads(place, local) || *src == local,
        Op::Unary { src, .. } | Op::Discriminant { src, .. } | Op::Return { src } => *src == local,
        Op::Binary { left, right, .. } => *left == local || *right == local,
        Op::If { condition } => *condition == local,
        Op::Else | Op::End | Op::Loop | Op::Break | Op::Continue
            | Op::BreakTo { .. } | Op::ContinueTo { .. } | Op::Trap { .. } => false,
    }
}
// A last textual use alone says nothing about a loop's next iteration. Require
// a fresh full definition in this same straight-line block on every execution.
fn freshly_defined(function: &Function, local: usize, call: usize) -> bool {
    let mut at = call;
    while at > 0 {
        at -= 1;
        let op = &function.body[at].op;
        if destination(op) == Some(local) { return !reads(op, local); }
        match op {
            Op::If { .. } | Op::Else | Op::End | Op::Loop | Op::Break | Op::Continue
                | Op::BreakTo { .. } | Op::ContinueTo { .. } | Op::Return { .. }
                | Op::Trap { .. } => return false,
            Op::Assign { place, .. } | Op::Drop { place } | Op::DropIf { place } => {
                if place.local == local { return false; }
            }
            _ => {}
        }
    }
    false
}
fn unique_argument(args: &[usize], index: usize) -> bool {
    for (other, argument) in args.iter().enumerate() {
        if other != index && *argument == args[index] { return false; }
    }
    true
}
fn inline_locals(
    caller: &Function, uses: &Uses, call: usize, callee: &Function,
    args: &[usize], destination: usize, shape: Shape, rewritten: &mut Function,
) -> Vec<usize> {
    let callee_uses = Uses::new(callee);
    let returned = if shape.direct && destination >= caller.params
        && uses.first[destination] == Some(call) && !uses.addressed[destination]
        && !args.contains(&destination) {
        match callee.body.last().map(|instruction: &Instruction| &instruction.op) {
            Some(Op::Return { src }) => {
                if !callee_uses.addressed[*src] { Some(*src) } else { None }
            }
            _ => None,
        }
    } else { None };
    let mut mapping = Vec::new();
    for (index, local) in callee.locals.iter().enumerate() {
        let actual = if returned == Some(index) {
            Some(destination)
        } else if index < callee.params {
            let argument = args[index];
            if argument >= caller.params && argument != destination
                && unique_argument(args, index) && !uses.addressed[argument]
                && !callee_uses.addressed[index] && uses.last[argument] == Some(call)
                && freshly_defined(caller, argument, call) {
                Some(argument)
            } else { None }
        } else { None };
        let target = match actual {
            Some(target) => {
                // Neither mapping can alter the caller's signature or contracts.
                rewritten.locals[target].mutable |= local.mutable;
                target
            }
            None => {
                let target = rewritten.locals.len();
                rewritten.locals.push(local.clone());
                target
            }
        };
        mapping.push(target);
    }
    mapping
}

fn locals(values: &[usize], mapping: &[usize]) -> Vec<usize> {
    let mut result = Vec::new();
    for value in values { result.push(mapping[*value]); }
    result
}
fn place(value: &Place, mapping: &[usize]) -> Place {
    let mut projections = Vec::new();
    for projection in &value.projections {
        projections.push(match projection {
            Projection::Deref => Projection::Deref,
            Projection::Field(field) => Projection::Field(*field),
            Projection::VariantField(variant, field) => Projection::VariantField(*variant, *field),
            Projection::Index(local) => Projection::Index(mapping[*local]),
        });
    }
    Place { local: mapping[value.local], projections }
}
fn remap(op: &Op, mapping: &[usize]) -> Op {
    match op {
        Op::Text { dst, bytes } => Op::Text { dst: mapping[*dst], bytes: bytes.clone() },
        Op::Int { dst, value } => Op::Int { dst: mapping[*dst], value: *value },
        Op::Float { dst, bits } => Op::Float { dst: mapping[*dst], bits: *bits },
        Op::Unit { dst } => Op::Unit { dst: mapping[*dst] },
        Op::Read { dst, place: source, moving } => Op::Read {
            dst: mapping[*dst], place: place(source, mapping), moving: *moving,
        },
        Op::Borrow { dst, place: source, mutable } => Op::Borrow {
            dst: mapping[*dst], place: place(source, mapping), mutable: *mutable,
        },
        Op::SliceBorrow { dst, place: source, mutable } => Op::SliceBorrow {
            dst: mapping[*dst], place: place(source, mapping), mutable: *mutable,
        },
        Op::ViewRange { dst, place: source, start, end, inclusive, mutable } => Op::ViewRange {
            dst: mapping[*dst], place: place(source, mapping), start: mapping[*start],
            end: mapping[*end], inclusive: *inclusive, mutable: *mutable,
        },
        Op::Array { dst, elements } => Op::Array { dst: mapping[*dst], elements: locals(elements, mapping) },
        Op::Assign { place: target, src } => Op::Assign { place: place(target, mapping), src: mapping[*src] },
        Op::Drop { place: target } => Op::Drop { place: place(target, mapping) },
        Op::DropIf { place: target } => Op::DropIf { place: place(target, mapping) },
        Op::Unary { dst, op, src } => Op::Unary { dst: mapping[*dst], op: *op, src: mapping[*src] },
        Op::Binary { dst, op, left, right } => Op::Binary {
            dst: mapping[*dst], op: *op, left: mapping[*left], right: mapping[*right],
        },
        Op::Call { dst, function, args } => Op::Call {
            dst: mapping[*dst], function: *function, args: locals(args, mapping),
        },
        Op::Struct { dst, fields } => Op::Struct { dst: mapping[*dst], fields: locals(fields, mapping) },
        Op::Variant { dst, variant, fields } => Op::Variant {
            dst: mapping[*dst], variant: *variant, fields: locals(fields, mapping),
        },
        Op::Discriminant { dst, src } => Op::Discriminant { dst: mapping[*dst], src: mapping[*src] },
        Op::TagTest { dst, place: source, variant } => Op::TagTest {
            dst: mapping[*dst], place: place(source, mapping), variant: *variant,
        },
        Op::Intrinsic { dst, name, args } => Op::Intrinsic {
            dst: mapping[*dst], name: *name, args: locals(args, mapping),
        },
        Op::If { condition } => Op::If { condition: mapping[*condition] },
        Op::Else => Op::Else,
        Op::End => Op::End,
        Op::Loop => Op::Loop,
        Op::Break => Op::Break,
        Op::Continue => Op::Continue,
        Op::BreakTo { depth } => Op::BreakTo { depth: *depth },
        Op::ContinueTo { depth } => Op::ContinueTo { depth: *depth },
        Op::Return { src } => Op::Return { src: mapping[*src] },
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
    args: &[usize], destination: usize, mapping: &[usize], shape: Shape,
) {
    for (index, argument) in args.iter().enumerate() {
        if mapping[index] != *argument {
            output.push(at(call, Op::Read {
                dst: mapping[index], place: Place::local(*argument), moving: true,
            }));
        }
    }
    if !shape.direct { output.push(at(call, Op::Loop)); }
    let mut controls = Vec::new();
    let mut loops = 0usize;
    for instruction in &callee.body {
        if let Op::Return { src } = instruction.op {
            if mapping[src] != destination {
                output.push(copied(callee, instruction, Op::Read {
                    dst: destination, place: Place::local(mapping[src]), moving: true,
                }));
            }
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
        output.push(copied(callee, instruction, remap(&instruction.op, mapping)));
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
    let uses = Uses::new(function);
    let mut rewritten = function.clone();
    rewritten.body.clear();
    for (at, instruction) in function.body.iter().enumerate() {
        if let Op::Call { dst, function: called, args } = &instruction.op {
            if let Some(shape) = eligible[*called] {
                let callee = &functions[*called];
                let mapping = inline_locals(
                    function, &uses, at, callee, args, *dst, shape, &mut rewritten,
                );
                append(&mut rewritten.body, callee, instruction, args, *dst, &mapping, shape);
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
