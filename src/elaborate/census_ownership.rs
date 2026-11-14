//! Closed structural roots use the ordinary IR builder and checker. No missing
//! nominal layout, generic witness, or callee signature is replaced by a stub.
use super::{Compiler, Instance, PendingFunction, a, census, definition_types, methods};
use crate::diagnostics::SemanticReport;
use crate::resolve::Resolver;
use std::collections::{BTreeMap, BTreeSet};

enum Body {
    Ready(Vec<usize>),
    Blocked(String),
}
fn dependencies(
    source: &a::Program,
    names: &Resolver<'_>,
    function: usize,
    inferred: &definition_types::DefinitionTypes,
    context: &census::Context,
) -> Body {
    let definition = &source.functions[function];
    if !definition.generics.is_empty()
        || source.function_owner(a::FunctionId { index: function }) != a::FunctionOwner::Free
        || !inferred.structural()
        || !inferred.unresolved_paths().is_empty()
    {
        return Body::Blocked(
            "ownership requires complete nominal, generic, or callable contracts".to_string(),
        );
    }
    let mut named_lifetimes = context.readiness.explicit_lifetimes[definition.result];
    for parameter in &definition.params {
        if context.readiness.explicit_lifetimes[parameter.ty] {
            named_lifetimes = true;
        }
    }
    if named_lifetimes {
        return Body::Blocked(
            "named lifetime obligations require their scoped declaration checks".to_string(),
        );
    }
    let mut blocks = BTreeSet::new();
    if let Some(body) = definition.body {
        blocks.insert(body);
    }
    let mut calls = Vec::new();
    for expression in inferred.expressions() {
        let node = &source.exprs[expression];
        match &node.kind {
            a::Expr::Block(block)
            | a::Expr::Loop { body: block }
            | a::Expr::While { body: block, .. }
            | a::Expr::For { body: block, .. }
            | a::Expr::If {
                then_block: block, ..
            } => {
                blocks.insert(*block);
            }
            a::Expr::Cast { ty, .. } if context.readiness.explicit_lifetimes[*ty] => {
                return Body::Blocked(
                    "named lifetime obligations require their scoped declaration checks"
                        .to_string(),
                );
            }
            a::Expr::Closure { .. } | a::Expr::Struct { .. } => {
                return Body::Blocked(
                    "ownership requires complete nominal or callable contracts".to_string(),
                );
            }
            a::Expr::Macro { name, .. } if name != "vec" => {
                return Body::Blocked(
                    "macro expansion requires its checked source library contracts".to_string(),
                );
            }
            a::Expr::Method { receiver, name, .. } => {
                let Some(receiver) = inferred.receiver(*receiver) else {
                    return Body::Blocked("method receiver type is unavailable".to_string());
                };
                if !matches!(
                    methods::direct(receiver.primitive(), name),
                    Some(
                        methods::Direct::Wrapping
                            | methods::Direct::Slice
                            | methods::Direct::Length
                            | methods::Direct::Push
                            | methods::Direct::Output
                    )
                ) {
                    return Body::Blocked(
                        "method requires complete declaration, trait, and visibility contracts"
                            .to_string(),
                    );
                }
            }
            a::Expr::Call { callee, .. } => {
                let a::Expr::Path(path) = &source.exprs[*callee].kind else {
                    return Body::Blocked(
                        "callable value requires its checked origin contract".to_string(),
                    );
                };
                let Ok(path) = names.value_path(source.exprs[*callee].scope, path, node.span)
                else {
                    return Body::Blocked("callee name is unavailable".to_string());
                };
                if let Some(declaration) = path.declaration {
                    let a::Item::Function(callee) = source.declarations[declaration].item else {
                        return Body::Blocked(
                            "constructor requires complete nominal contracts".to_string(),
                        );
                    };
                    calls.push(callee);
                } else if !matches!(
                    path.name.as_str(),
                    "Vec::new" | "Box::new" | "drop" | "std::mem::drop"
                ) {
                    return Body::Blocked("callee signature is unavailable".to_string());
                }
            }
            // Only scalar operators bypass trait selection. The other body
            // forms are constrained through their actual source inference.
            a::Expr::Binary { left, right, .. } => {
                if !matches!(
                    inferred.receiver(*left),
                    Some(definition_types::census::Receiver::Atom(_))
                ) || !matches!(
                    inferred.receiver(*right),
                    Some(definition_types::census::Receiver::Atom(_))
                ) {
                    return Body::Blocked("operator requires complete trait contracts".to_string());
                }
            }
            _ => {}
        }
    }
    for block in blocks {
        for statement in &source.blocks[block].statements {
            if let a::Stmt::Let { ty: Some(ty), .. } = statement {
                if context.readiness.explicit_lifetimes[*ty] {
                    return Body::Blocked(
                        "named lifetime obligations require their scoped declaration checks"
                            .to_string(),
                    );
                }
            }
        }
    }
    Body::Ready(calls)
}
pub(super) fn analyze(
    source: &a::Program,
    names: Resolver<'_>,
    implementations: crate::generics::ImplementationIndex,
    context: &census::Context,
    definitions: BTreeMap<usize, definition_types::DefinitionTypes>,
    report: &mut SemanticReport,
) {
    let mut available = BTreeSet::new();
    let mut blockers = BTreeMap::new();
    let mut reverse = vec![Vec::new(); source.functions.len()];
    let mut pending = Vec::new();
    for function in 0..source.functions.len() {
        let Some(inferred) = definitions.get(&function) else {
            if source.functions[function].body.is_some() {
                blockers.insert(
                    function,
                    "source type analysis is incomplete or failed".to_string(),
                );
            }
            pending.push(function);
            continue;
        };
        if let Some(problem) = context.readiness.layout_problem(source, function) {
            blockers.insert(function, problem.message.clone());
            pending.push(function);
            continue;
        }
        match dependencies(source, &names, function, inferred, context) {
            Body::Ready(callees) => {
                available.insert(function);
                for callee in callees {
                    reverse[callee].push(function);
                }
            }
            Body::Blocked(reason) => {
                blockers.insert(function, reason);
                pending.push(function);
            }
        }
    }
    let mut head = 0;
    while head < pending.len() {
        let callee = pending[head];
        head += 1;
        for function in &reverse[callee] {
            if available.remove(function) {
                blockers.insert(
                    *function,
                    format!(
                        "callee {} has unavailable ownership contracts",
                        source.functions[callee].name
                    ),
                );
                pending.push(*function);
            }
        }
    }
    for (function, reason) in blockers {
        census::record(
            report,
            source,
            Some(function),
            "layout/ownership",
            census::blocked(source.functions[function].span, &reason),
            None,
            None,
        );
    }
    if available.is_empty() {
        return;
    }
    let mut compiler = Compiler::with_indexes(source, names, implementations);
    compiler.diagnostic_functions = Some(&available);
    compiler.definition_types = definitions;
    for source_function in &available {
        match compiler.instantiate(*source_function, BTreeMap::new()) {
            Ok(_) => {}
            Err(problem) => census::record(
                report,
                source,
                Some(*source_function),
                "ownership construction",
                problem,
                None,
                None,
            ),
        }
    }
    let mut next = 0;
    while next < compiler.pending_functions.len() {
        let (function, pending) = compiler.pending_functions[next].clone();
        let PendingFunction::Source(instance) = pending else {
            let span = a::Span {
                file: compiler.functions[function].file,
                ..a::Span::default()
            };
            census::record(
                report,
                source,
                None,
                "ownership construction",
                census::blocked(span, "derived methods require complete nominal contracts"),
                None,
                None,
            );
            compiler.functions[function].body = vec![crate::ir::Instruction {
                column: 1,
                line: 1,
                op: crate::ir::Op::Trap {
                    message: "unavailable derived body".to_string(),
                },
            }];
            next += 1;
            continue;
        };
        match compiler.compile_function(
            function,
            &Instance {
                source: instance.source,
                substitutions: instance.substitutions,
            },
        ) {
            Ok(body) => {
                compiler.functions[function] = body;
                crate::diagnostics::count(
                    &mut report.coverage,
                    "structural ownership bodies constructed",
                );
            }
            Err(problem) => {
                census::record(
                    report,
                    source,
                    Some(instance.source),
                    "ownership construction",
                    problem,
                    None,
                    None,
                );
                compiler.functions[function].body = vec![crate::ir::Instruction {
                    column: 1,
                    line: 1,
                    op: crate::ir::Op::Trap {
                        message: "known signature with unavailable body".to_string(),
                    },
                }];
            }
        }
        next += 1;
    }
    let mut program = crate::ir::Program {
        types: compiler.types,
        functions: compiler.functions,
        destructors: compiler.destructors,
    };
    let mut problems =
        crate::check::refine_return_contracts(&mut program, &compiler.return_inference);
    if let Err(errors) = crate::check::check_all(program) {
        for error in errors {
            problems.push(error);
        }
    }
    for problem in problems {
        census::record(report, source, None, "ownership", problem, None, None);
    }
}
