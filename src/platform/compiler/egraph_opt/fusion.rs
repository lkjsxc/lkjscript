//! Literal and infallible identity map/fold fusion, guarded by the exact package revision.
//! Every callback is checked with a finite pure evaluator before a rewrite can
//! remove its call. Any trap, unknown operation or exhausted proof leaves the
//! original code intact, including its original trap ordering.
use super::*;
use crate::platform::kernel::{FunctionEffect, OwnerKey, OwnerRecord};

#[derive(Clone, Copy)]
enum Combinator {
    Map,
    Fold,
}
#[derive(Clone)]
enum Symbol {
    Integer(i64),
    Function(usize),
    List(Vec<i64>),
    Mapped(Vec<i64>, usize),
    Unknown,
    ListInput(usize),
    MappedInput { list_end: usize, map_end: usize },
}
#[derive(Clone, Copy)]
enum Value {
    Integer(i64),
    Bool(bool),
}

fn combinator(program: &NormalizedProgram, function: usize) -> Option<Combinator> {
    let function = program.functions.get(function)?;
    if !matches!(function.body, NormalizedFunctionBody::Code(_)) {
        return None;
    }
    let declaration = function.declaration;
    if declaration.package.to_string() != crate::platform::builtin_standard::STANDARD_PACKAGE {
        return None;
    }
    let package = program
        .artifact()
        .manifest
        .packages
        .iter()
        .find(|p| p.package == declaration.package)?;
    if package.package_revision.to_string()
        != crate::platform::builtin_standard::STANDARD_PACKAGE_REVISION
    {
        return None;
    }
    let Some(OwnerRecord::Declaration(owner)) = program
        .artifact()
        .reference_owner(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
            &mut Default::default(),
            &mut Default::default(),
        )
        .ok()?
    else {
        return None;
    };
    match owner.name.as_str() {
        "list-map" => Some(Combinator::Map),
        "list-fold-left" => Some(Combinator::Fold),
        _ => None,
    }
}

fn evaluate(
    program: &NormalizedProgram,
    function: usize,
    args: &[Value],
    remaining: &mut usize,
    depth: usize,
) -> Option<Value> {
    if depth > 8 {
        return None;
    }
    *remaining = remaining.checked_sub(1)?;
    let function = program.functions.get(function)?;
    if function.effect != FunctionEffect::Pure
        || function.parameter_count as usize != args.len()
        || !function.task_requirements.is_empty()
    {
        return None;
    }
    match &function.body {
        NormalizedFunctionBody::External(name) => {
            let [Value::Integer(a), Value::Integer(b)] = args else {
                return None;
            };
            match name.as_str() {
                "core.i64.add" => a.checked_add(*b).map(Value::Integer),
                "core.i64.subtract" => a.checked_sub(*b).map(Value::Integer),
                "core.i64.multiply" => a.checked_mul(*b).map(Value::Integer),
                "core.i64.divide" => a.checked_div(*b).map(Value::Integer),
                "core.i64.equal" => Some(Value::Bool(a == b)),
                "core.i64.less" => Some(Value::Bool(a < b)),
                "core.i64.less-equal" => Some(Value::Bool(a <= b)),
                _ => None,
            }
        }
        NormalizedFunctionBody::Code(code) => {
            if code.local_count as usize > MAX_NODES {
                return None;
            }
            let mut locals = vec![None; code.local_count as usize];
            for (local, arg) in locals.iter_mut().zip(args) {
                *local = Some(*arg);
            }
            let mut stack = Vec::new();
            let mut ip = 0;
            loop {
                *remaining = remaining.checked_sub(1)?;
                match code.instructions.get(ip)? {
                    I::I64(v) => stack.push(Value::Integer(*v)),
                    I::Bool(v) => stack.push(Value::Bool(*v)),
                    I::LoadLocal { local, .. } | I::MoveLocal(local) => {
                        stack.push((*locals.get(*local as usize)?)?)
                    }
                    I::StoreLocal(local) => *locals.get_mut(*local as usize)? = Some(stack.pop()?),
                    I::Drop => {
                        stack.pop()?;
                    }
                    I::Call {
                        function,
                        arguments,
                        ..
                    }
                    | I::TailCall {
                        function,
                        arguments,
                        ..
                    } => {
                        let start = stack.len().checked_sub(*arguments as usize)?;
                        let result = evaluate(
                            program,
                            function.0 as usize,
                            &stack[start..],
                            remaining,
                            depth + 1,
                        )?;
                        stack.truncate(start);
                        stack.push(result);
                        if matches!(code.instructions[ip], I::TailCall { .. }) {
                            return stack.pop();
                        }
                    }
                    I::Jump(target) => {
                        ip = *target as usize;
                        continue;
                    }
                    I::JumpIfFalse(target) => {
                        let Value::Bool(condition) = stack.pop()? else {
                            return None;
                        };
                        if !condition {
                            ip = *target as usize;
                            continue;
                        }
                    }
                    I::Return => {
                        let result = stack.pop()?;
                        return stack.is_empty().then_some(result);
                    }
                    _ => return None,
                }
                if stack.len() > MAX_NODES {
                    return None;
                }
                ip += 1;
            }
        }
    }
}

fn identity(program: &NormalizedProgram, index: usize) -> bool {
    let Some(function) = program.functions.get(index) else {
        return false;
    };
    if function.effect != FunctionEffect::Pure
        || !function.task_requirements.is_empty()
        || function.parameter_count != 1
    {
        return false;
    }
    let NormalizedFunctionBody::Code(code) = &function.body else {
        return false;
    };
    matches!(
        code.instructions.as_ref(),
        [I::LoadLocal { local: 0, .. } | I::MoveLocal(0), I::Return]
    )
}

pub(super) fn augment(program: &NormalizedProgram, code: &NormalizedCode, plan: &mut Plan) {
    let mut stack = Vec::<(usize, Symbol)>::new();
    let mut known = BTreeMap::new();
    let mut remaining = MAX_NODES;
    let mut entries = BTreeSet::new();
    for instruction in code.instructions.iter() {
        match instruction {
            I::Jump(t) | I::JumpIfFalse(t) => {
                entries.insert(*t as usize);
            }
            I::SwitchVariant(arms) => {
                entries.extend(arms.iter().map(|a| a.target as usize));
            }
            I::MatchOwned { cases, .. } => {
                entries.extend(cases.iter().map(|a| a.target as usize));
            }
            I::MatchBorrowedOwned { cases, .. } => {
                entries.extend(cases.iter().map(|a| a.target as usize));
            }
            _ => {}
        }
    }
    for (ip, instruction) in code.instructions.iter().enumerate() {
        if entries.contains(&ip) {
            stack.clear();
        }
        if stack.len() >= MAX_NODES {
            stack.clear();
        }
        match instruction {
            I::I64(value) => stack.push((ip, Symbol::Integer(*value))),
            I::LoadLocal { .. } | I::MoveLocal(_) => stack.push((ip, Symbol::Unknown)),
            I::FunctionValue { function, .. } => {
                stack.push((ip, Symbol::Function(function.0 as usize)))
            }
            I::List { items } if *items <= 64 && stack.len() >= *items as usize => {
                let start = stack.len() - *items as usize;
                let values: Option<Vec<_>> = stack[start..]
                    .iter()
                    .map(|(_, s)| {
                        if let Symbol::Integer(v) = s {
                            Some(*v)
                        } else {
                            None
                        }
                    })
                    .collect();
                if let Some(values) = values {
                    let first = stack.get(start).map_or(ip, |s| s.0);
                    stack.truncate(start);
                    stack.push((first, Symbol::List(values)));
                } else {
                    let first = stack.get(start).map_or(ip, |s| s.0);
                    stack.truncate(start);
                    stack.push((first, Symbol::ListInput(ip + 1)));
                }
            }
            I::Call {
                function,
                arguments,
                ..
            }
            | I::TailCall {
                function,
                arguments,
                ..
            } => {
                let kind = *known
                    .entry(function.0 as usize)
                    .or_insert_with(|| combinator(program, function.0 as usize));
                match (kind, *arguments) {
                    (Some(Combinator::Map), 2) if stack.len() >= 2 => {
                        let mapper = stack.pop();
                        let input = stack.pop();
                        match (mapper, input) {
                            (
                                Some((_, Symbol::Function(mapper))),
                                Some((start, Symbol::List(values))),
                            ) => stack.push((start, Symbol::Mapped(values, mapper))),
                            (
                                Some((_, Symbol::Function(mapper))),
                                Some((start, Symbol::ListInput(list_end))),
                            ) if identity(program, mapper) => stack.push((
                                start,
                                Symbol::MappedInput {
                                    list_end,
                                    map_end: ip + 1,
                                },
                            )),
                            _ => stack.clear(),
                        }
                    }
                    (Some(Combinator::Fold), 3) if stack.len() >= 3 => {
                        let reducer = stack.pop();
                        let initial = stack.pop();
                        let input = stack.pop();
                        if let Some((start, Symbol::MappedInput { list_end, map_end })) = &input
                            && entries.range(start + 1..=ip).next().is_none()
                        {
                            // An infallible identity mapper can be fused into the
                            // existing fold even for runtime list elements. Keep
                            // the reducer and its original failure order intact.
                            let mut instructions = code.instructions[*start..*list_end].to_vec();
                            instructions.extend_from_slice(&code.instructions[*map_end..=ip]);
                            let remove: Vec<_> = plan
                                .rewrites
                                .range(*start..=ip)
                                .map(|(&start, _)| start)
                                .collect();
                            for start in remove {
                                plan.rewrites.remove(&start);
                            }
                            plan.rewrites.insert(
                                *start,
                                Rewrite {
                                    end: ip + 1,
                                    instructions,
                                },
                            );
                            plan.fusions += 1;
                        }
                        if let (
                            Some((_, Symbol::Function(reducer))),
                            Some((_, Symbol::Integer(initial))),
                            Some((start, Symbol::Mapped(values, mapper))),
                        ) = (reducer, initial, input)
                        {
                            let mut result = Some(Value::Integer(initial));
                            for value in values {
                                result = result.and_then(|accumulator| {
                                    let item = evaluate(
                                        program,
                                        mapper,
                                        &[Value::Integer(value)],
                                        &mut remaining,
                                        0,
                                    )?;
                                    evaluate(
                                        program,
                                        reducer,
                                        &[accumulator, item],
                                        &mut remaining,
                                        0,
                                    )
                                });
                                if result.is_none() {
                                    break;
                                }
                            }
                            if let Some(Value::Integer(value)) = result {
                                // No internal branch can enter this straight-line expression.
                                let interior = entries.range(start + 1..=ip).next().is_some();
                                if !interior {
                                    let remove: Vec<_> = plan
                                        .rewrites
                                        .range(start..=ip)
                                        .map(|(&start, _)| start)
                                        .collect();
                                    for start in remove {
                                        plan.rewrites.remove(&start);
                                    }
                                    let mut instructions = vec![I::I64(value)];
                                    if matches!(instruction, I::TailCall { .. }) {
                                        instructions.push(I::Return);
                                    }
                                    plan.rewrites.insert(
                                        start,
                                        Rewrite {
                                            end: ip + 1,
                                            instructions,
                                        },
                                    );
                                    plan.fusions += 1;
                                }
                            }
                        }
                        stack.clear();
                    }
                    _ => stack.clear(),
                }
            }
            _ => stack.clear(),
        }
    }
}
