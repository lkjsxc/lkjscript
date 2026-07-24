//! Deterministic fixed-point effect inference over resolved HIR call identities.

use crate::hir::{BindingStorage, EffectSet, Expr, ExprKind, Program};

pub(crate) fn infer(program: &mut Program) {
    let binding_to_function = binding_to_function(program);
    let order = stable_function_order(program);
    let call_graph = direct_call_graph(program, &binding_to_function);
    let recursive = recursive_functions(&call_graph, &order);
    let mut summaries = vec![None; program.bindings.len()];

    for &function_index in &order {
        let function = &program.functions[function_index];
        if let Some(slot) = function
            .binding
            .index()
            .and_then(|index| summaries.get_mut(index))
        {
            *slot = Some(if recursive[function_index] {
                EffectSet::MAY_DIVERGE
            } else {
                EffectSet::PURE
            });
        }
    }

    loop {
        let mut changed = false;
        for &function_index in &order {
            let binding = program.functions[function_index].binding;
            let body_effects =
                recompute_expr(&mut program.functions[function_index].body, &summaries);
            let cycle_effects = if recursive[function_index] {
                EffectSet::MAY_DIVERGE
            } else {
                EffectSet::PURE
            };
            let old = summary_for_binding(binding, &summaries);
            let updated = old.union(body_effects).union(cycle_effects);
            if !old.contains(updated) {
                if let Some(slot) = binding.index().and_then(|index| summaries.get_mut(index)) {
                    *slot = Some(updated);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    for &function_index in &order {
        let binding = program.functions[function_index].binding;
        let summary = summary_for_binding(binding, &summaries);
        recompute_expr(&mut program.functions[function_index].body, &summaries);
        program.functions[function_index].summary = summary;
    }
    recompute_expr(&mut program.main.body, &summaries);
}

fn summary_for_binding(
    binding: crate::hir::BindingId,
    summaries: &[Option<EffectSet>],
) -> EffectSet {
    binding
        .index()
        .and_then(|index| summaries.get(index))
        .copied()
        .flatten()
        .unwrap_or(EffectSet::CONSERVATIVE_CALL)
}

fn binding_to_function(program: &Program) -> Vec<Option<usize>> {
    let mut result = vec![None; program.bindings.len()];
    for (function_index, function) in program.functions.iter().enumerate() {
        if let Some(binding_index) = function.binding.index() {
            if let Some(slot) = result.get_mut(binding_index) {
                *slot = Some(function_index);
            }
        }
    }
    result
}

fn stable_function_order(program: &Program) -> Vec<usize> {
    let mut order: Vec<_> = (0..program.functions.len()).collect();
    order.sort_unstable_by_key(|&index| program.functions[index].binding.raw());
    order
}

fn direct_call_graph(program: &Program, binding_to_function: &[Option<usize>]) -> Vec<Vec<usize>> {
    let mut graph = vec![Vec::new(); program.functions.len()];
    for (function_index, function) in program.functions.iter().enumerate() {
        collect_direct_callees(
            &function.body,
            binding_to_function,
            &mut graph[function_index],
        );
        graph[function_index]
            .sort_unstable_by_key(|&callee| program.functions[callee].binding.raw());
        graph[function_index].dedup();
    }
    graph
}

fn collect_direct_callees(
    expression: &Expr,
    binding_to_function: &[Option<usize>],
    callees: &mut Vec<usize>,
) {
    match &expression.kind {
        ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitNone
        | ExprKind::LitStr(_)
        | ExprKind::Load(_)
        | ExprKind::QuoteSymbol(_) => {}
        ExprKind::Call { callee, args } => {
            if callee.storage == BindingStorage::Function {
                if let Some(function_index) = callee
                    .binding
                    .index()
                    .and_then(|index| binding_to_function.get(index))
                    .copied()
                    .flatten()
                {
                    callees.push(function_index);
                }
            }
            collect_direct_callees_slice(args, binding_to_function, callees);
        }
        ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            collect_direct_callees_slice(args, binding_to_function, callees);
            if let ExprKind::While { condition, .. } = &expression.kind {
                collect_direct_callees(condition, binding_to_function, callees);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_direct_callees(condition, binding_to_function, callees);
            collect_direct_callees(then_branch, binding_to_function, callees);
            collect_direct_callees(else_branch, binding_to_function, callees);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                collect_direct_callees(&binding.value, binding_to_function, callees);
            }
            collect_direct_callees(body, binding_to_function, callees);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            collect_direct_callees(initial, binding_to_function, callees);
            collect_direct_callees(body, binding_to_function, callees);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            collect_direct_callees(value, binding_to_function, callees);
        }
        ExprKind::ProductValue { fields, .. } => {
            collect_direct_callees_slice(fields, binding_to_function, callees);
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_direct_callees(value, binding_to_function, callees);
            collect_direct_callees(replacement, binding_to_function, callees);
        }
    }
}

fn collect_direct_callees_slice(
    expressions: &[Expr],
    binding_to_function: &[Option<usize>],
    callees: &mut Vec<usize>,
) {
    for expression in expressions {
        collect_direct_callees(expression, binding_to_function, callees);
    }
}

fn recursive_functions(graph: &[Vec<usize>], order: &[usize]) -> Vec<bool> {
    let mut recursive = vec![false; graph.len()];
    for &start in order {
        let mut visited = vec![false; graph.len()];
        let mut stack = Vec::new();
        stack.extend(graph[start].iter().rev().copied());
        while let Some(current) = stack.pop() {
            if current == start {
                recursive[start] = true;
                break;
            }
            if visited[current] {
                continue;
            }
            visited[current] = true;
            stack.extend(graph[current].iter().rev().copied());
        }
    }
    recursive
}

fn recompute_expr(expression: &mut Expr, summaries: &[Option<EffectSet>]) -> EffectSet {
    let effects = match &mut expression.kind {
        ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitNone
        | ExprKind::LitStr(_)
        | ExprKind::Load(_)
        | ExprKind::QuoteSymbol(_) => EffectSet::PURE,
        ExprKind::Call { callee, args } => {
            let callee_effects = if callee.storage == BindingStorage::Function {
                callee
                    .binding
                    .index()
                    .and_then(|index| summaries.get(index))
                    .copied()
                    .flatten()
                    .unwrap_or(EffectSet::CONSERVATIVE_CALL)
            } else {
                EffectSet::CONSERVATIVE_CALL
            };
            recompute_slice(args, summaries).union(callee_effects)
        }
        ExprKind::Operation {
            operation, args, ..
        } => recompute_slice(args, summaries).union(operation.effects()),
        ExprKind::Do(expressions) => recompute_slice(expressions, summaries),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => recompute_expr(condition, summaries)
            .union(recompute_expr(then_branch, summaries))
            .union(recompute_expr(else_branch, summaries)),
        ExprKind::While { condition, body } => recompute_expr(condition, summaries)
            .union(recompute_slice(body, summaries))
            .union(EffectSet::MAY_DIVERGE),
        ExprKind::Let { bindings, body } => bindings
            .iter_mut()
            .fold(EffectSet::PURE, |effects, binding| {
                effects.union(recompute_expr(&mut binding.value, summaries))
            })
            .union(recompute_expr(body, summaries)),
        ExprKind::MutableLocal { initial, body, .. } => {
            recompute_expr(initial, summaries).union(recompute_expr(body, summaries))
        }
        ExprKind::SetLocal { value, .. } => {
            recompute_expr(value, summaries).union(EffectSet::MUTATES_LOCAL)
        }
        ExprKind::ProductValue { fields, .. } => {
            recompute_slice(fields, summaries).union(EffectSet::ALLOCATES)
        }
        ExprKind::ProductField { value, .. } => {
            recompute_expr(value, summaries).union(EffectSet::READS_MEMORY)
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => recompute_expr(value, summaries)
            .union(recompute_expr(replacement, summaries))
            .union(EffectSet::READS_MEMORY)
            .union(EffectSet::ALLOCATES),
    };
    expression.effects = effects;
    effects
}

fn recompute_slice(expressions: &mut [Expr], summaries: &[Option<EffectSet>]) -> EffectSet {
    expressions
        .iter_mut()
        .fold(EffectSet::PURE, |effects, expression| {
            effects.union(recompute_expr(expression, summaries))
        })
}
