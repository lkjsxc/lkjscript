//! Deterministic fixed-point effect inference over resolved HIR call identities.

use crate::hir::{BindingStorage, EffectSet, Expr, ExprKind, Program};

mod call_graph;
mod facts;

use call_graph::{
    binding_to_function, direct_call_graph, recursive_functions, stable_function_order,
    summary_for_binding,
};
use facts::recompute_expr;

pub(crate) fn infer(program: &mut Program) {
    infer_parts(
        program.bindings.len(),
        &mut program.functions,
        Some(&mut program.main.body),
    );
}

pub(crate) fn infer_partial(
    binding_count: usize,
    functions: &mut [crate::hir::Function],
    main: Option<&mut Expr>,
) {
    infer_parts(binding_count, functions, main);
}

fn infer_parts(
    binding_count: usize,
    functions: &mut [crate::hir::Function],
    main: Option<&mut Expr>,
) {
    let binding_to_function = binding_to_function(binding_count, functions);
    let order = stable_function_order(functions);
    let call_graph = direct_call_graph(functions, &binding_to_function);
    let recursive = recursive_functions(&call_graph, &order);
    let mut summaries = vec![None; binding_count];

    for &function_index in &order {
        let function = &functions[function_index];
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
            let binding = functions[function_index].binding;
            let body_effects = recompute_expr(&mut functions[function_index].body, &summaries);
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
        let binding = functions[function_index].binding;
        let summary = summary_for_binding(binding, &summaries);
        recompute_expr(&mut functions[function_index].body, &summaries);
        functions[function_index].summary = summary;
    }
    if let Some(main) = main {
        recompute_expr(main, &summaries);
    }
}
