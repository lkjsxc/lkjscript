use super::*;

pub(super) fn summary_for_binding(
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

pub(super) fn binding_to_function(program: &Program) -> Vec<Option<usize>> {
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

pub(super) fn stable_function_order(program: &Program) -> Vec<usize> {
    let mut order: Vec<_> = (0..program.functions.len()).collect();
    order.sort_unstable_by_key(|&index| program.functions[index].binding.raw());
    order
}

pub(super) fn direct_call_graph(
    program: &Program,
    binding_to_function: &[Option<usize>],
) -> Vec<Vec<usize>> {
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

pub(super) fn collect_direct_callees(
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
        | ExprKind::LitStr(_)
        | ExprKind::LitBytes(_)
        | ExprKind::Load(_)
        | ExprKind::Move { .. }
        | ExprKind::Borrow { .. }
        | ExprKind::BorrowBytes { .. }
        | ExprKind::MatchUnreachable { .. }
        | ExprKind::QuoteSymbol(_) => {}
        ExprKind::Call { callee, args, .. } => {
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
        | ExprKind::Loop { body: args, .. }
        | ExprKind::While { body: args, .. } => {
            collect_direct_callees_slice(args, binding_to_function, callees);
            if let ExprKind::While { condition, .. } = &expression.kind {
                collect_direct_callees(condition, binding_to_function, callees);
            }
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => {
            collect_direct_callees(value, binding_to_function, callees);
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
        ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => {
            collect_direct_callees(value, binding_to_function, callees);
        }
        ExprKind::ProductValue { fields, .. } | ExprKind::EnumValue { fields, .. } => {
            collect_direct_callees_slice(fields, binding_to_function, callees);
        }
        ExprKind::Continue { .. } => {}
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_direct_callees(value, binding_to_function, callees);
            collect_direct_callees(replacement, binding_to_function, callees);
        }
    }
}

pub(super) fn collect_direct_callees_slice(
    expressions: &[Expr],
    binding_to_function: &[Option<usize>],
    callees: &mut Vec<usize>,
) {
    for expression in expressions {
        collect_direct_callees(expression, binding_to_function, callees);
    }
}

pub(super) fn recursive_functions(graph: &[Vec<usize>], order: &[usize]) -> Vec<bool> {
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
