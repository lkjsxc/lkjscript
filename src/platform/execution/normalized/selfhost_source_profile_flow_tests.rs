//! Canonical-source control-flow observations for the maintained compiler.
//! Tail positions and lexical scopes do not prove owned-slot liveness or cleanup.
use crate::platform::kernel;
use crate::platform::semantic_id::{BindingId, ExpressionId};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type Reference = kernel::DeclarationReference;
type CallGraph = BTreeMap<Reference, BTreeSet<Reference>>;
type ContextKey = (kernel::OwnerKey, ExpressionId, bool, Vec<BindingId>);

#[derive(Clone, Copy)]
struct Root {
    owner: kernel::OwnerKey,
    caller: Option<Reference>,
    expression: ExpressionId,
}

enum Frame {
    Enter {
        expression: ExpressionId,
        tail: bool,
        bindings: Vec<BindingId>,
    },
    Exit(ExpressionId),
}

fn expression(source: &kernel::KernelSnapshot, id: ExpressionId) -> &kernel::ExpressionRecord {
    match source.owners.get(&kernel::OwnerKey::Expression(id)) {
        Some(kernel::OwnerRecord::Expression(record)) => {
            assert_eq!(record.id, id);
            record
        }
        _ => panic!("control inventory has no exact expression {id:?}"),
    }
}

fn binding(source: &kernel::KernelSnapshot, id: BindingId) -> &kernel::BindingRecord {
    match source.owners.get(&kernel::OwnerKey::Binding(id)) {
        Some(kernel::OwnerRecord::Binding(record)) => {
            assert_eq!(record.kind, kernel::BindingKind::Let);
            record
        }
        _ => panic!("control inventory has no exact Let binding {id:?}"),
    }
}

/// Iterative Kosaraju over every supplied callable, including isolated vertices.
fn components(graph: &CallGraph) -> Vec<Vec<Reference>> {
    let mut seen = BTreeSet::new();
    let mut finished = Vec::with_capacity(graph.len());
    for start in graph.keys().copied() {
        if seen.contains(&start) {
            continue;
        }
        let mut pending = vec![(start, false)];
        while let Some((node, exiting)) = pending.pop() {
            if exiting {
                finished.push(node);
                continue;
            }
            if !seen.insert(node) {
                continue;
            }
            pending.push((node, true));
            for next in graph.get(&node).unwrap().iter().rev().copied() {
                assert!(graph.contains_key(&next), "edge has no supplied callable");
                if !seen.contains(&next) {
                    pending.push((next, false));
                }
            }
        }
    }
    assert_eq!(finished.len(), graph.len());
    let mut reversed: CallGraph = graph
        .keys()
        .copied()
        .map(|node| (node, BTreeSet::new()))
        .collect();
    for (from, destinations) in graph {
        for to in destinations {
            reversed.get_mut(to).unwrap().insert(*from);
        }
    }
    let mut assigned = BTreeSet::new();
    let mut result = Vec::new();
    for start in finished.into_iter().rev() {
        if !assigned.insert(start) {
            continue;
        }
        let mut members = Vec::new();
        let mut pending = vec![start];
        while let Some(node) = pending.pop() {
            members.push(node);
            for next in reversed.get(&node).unwrap().iter().rev().copied() {
                if assigned.insert(next) {
                    pending.push(next);
                }
            }
        }
        members.sort();
        result.push(members);
    }
    result.sort();
    assert_eq!(assigned.len(), graph.len());
    assert_eq!(result.iter().map(Vec::len).sum::<usize>(), graph.len());
    result
}

pub(super) fn inventory(source: &kernel::KernelSnapshot) -> Value {
    use kernel::ExpressionOperation as E;
    let mut roots = Vec::new();
    let mut graph = CallGraph::new();
    let mut supplied_expressions = BTreeSet::new();
    let mut supplied_calls = BTreeSet::new();
    let mut supplied_bindings = BTreeSet::new();
    for (owner, record) in &source.owners {
        assert_eq!(*owner, record.owner());
        match record {
            kernel::OwnerRecord::Declaration(record) => {
                let kernel::OwnerKey::Declaration(id) = *owner else {
                    panic!("control inventory declaration key");
                };
                let reference = Reference {
                    package: source.root.package_id,
                    declaration: id,
                };
                match &record.payload {
                    kernel::DeclarationPayload::Function(function) => {
                        graph.entry(reference).or_default();
                        roots.push(Root {
                            owner: *owner,
                            caller: Some(reference),
                            expression: function.body,
                        });
                    }
                    kernel::DeclarationPayload::External(_) => {
                        graph.entry(reference).or_default();
                    }
                    _ => {}
                }
            }
            kernel::OwnerRecord::Port(record) => {
                if let kernel::PortImplementation::Expression(id) = &record.implementation {
                    roots.push(Root {
                        owner: *owner,
                        caller: None,
                        expression: *id,
                    });
                }
            }
            kernel::OwnerRecord::Expression(record) => {
                assert!(supplied_expressions.insert(record.id));
                if matches!(&record.operation, E::Call { .. }) {
                    assert!(supplied_calls.insert(record.id));
                }
            }
            kernel::OwnerRecord::Binding(record) => {
                let kernel::OwnerKey::Binding(id) = *owner else {
                    panic!("control inventory binding key");
                };
                assert_eq!(record.kind, kernel::BindingKind::Let);
                assert!(supplied_bindings.insert(id));
            }
            _ => {}
        }
    }
    roots.sort_by_key(|root| (root.owner, root.expression));
    let mut visited_contexts = BTreeSet::<ContextKey>::new();
    let mut visited_expressions = BTreeSet::new();
    let mut visited_calls = BTreeSet::new();
    let mut visited_bindings = BTreeSet::new();
    let mut call_sites = BTreeMap::<ContextKey, Value>::new();
    let mut scopes = BTreeMap::<ContextKey, Value>::new();
    for root in &roots {
        let mut active = BTreeSet::new();
        let mut pending = vec![Frame::Enter {
            expression: root.expression,
            tail: true,
            bindings: Vec::new(),
        }];
        while let Some(frame) = pending.pop() {
            let (id, tail, lexical_bindings) = match frame {
                Frame::Exit(id) => {
                    assert!(active.remove(&id));
                    continue;
                }
                Frame::Enter {
                    expression,
                    tail,
                    bindings,
                } => (expression, tail, bindings),
            };
            // Check before contextual memoization: a Let cycle may otherwise
            // produce a different binding vector on every traversal.
            assert!(
                active.insert(id),
                "expression cycle at {id:?} in {:?}",
                root.owner
            );
            let context = (root.owner, id, tail, lexical_bindings.clone());
            if !visited_contexts.insert(context.clone()) {
                assert!(active.remove(&id));
                continue;
            }
            pending.push(Frame::Exit(id));
            visited_expressions.insert(id);
            let record = expression(source, id);
            super::assert_active_expression(&record.operation);
            if let E::Call {
                function,
                type_arguments,
                ..
            } = &record.operation
            {
                assert!(
                    graph.contains_key(function),
                    "call has no supplied callable"
                );
                let callee = super::declaration(source, *function);
                visited_calls.insert(id);
                if let Some(caller) = root.caller {
                    graph.get_mut(&caller).unwrap().insert(*function);
                }
                assert!(call_sites.insert(context.clone(), json!({
                    "root_owner": root.owner,
                    "caller": root.caller,
                    "expression": id,
                    "callee": function,
                    "callee_kind": callee.header.kind,
                    "ordered_type_arguments": type_arguments,
                    "ordered_substitution": super::substitutions(source, *function, type_arguments),
                    "syntactic_tail_position": tail,
                    "lexical_bindings": &lexical_bindings,
                })).is_none());
            }
            let mut children = Vec::new();
            match &record.operation {
                E::If {
                    condition,
                    when_true,
                    when_false,
                } => {
                    for (child, child_tail) in
                        [(*condition, false), (*when_true, tail), (*when_false, tail)]
                    {
                        children.push(Frame::Enter {
                            expression: child,
                            tail: child_tail,
                            bindings: lexical_bindings.clone(),
                        });
                    }
                }
                E::Let { bindings, body } => {
                    let mut prefix = lexical_bindings.clone();
                    let mut ordered = Vec::new();
                    for id in bindings {
                        let record = binding(source, *id);
                        let initializer = record.value.expect("Let binding has no initializer");
                        visited_bindings.insert(*id);
                        ordered.push(json!({
                            "binding": id,
                            "initializer": initializer,
                            "declared_type": record.declared_type,
                        }));
                        children.push(Frame::Enter {
                            expression: initializer,
                            tail: false,
                            bindings: prefix.clone(),
                        });
                        prefix.push(*id);
                    }
                    children.push(Frame::Enter {
                        expression: *body,
                        tail,
                        bindings: prefix,
                    });
                    let reverse: Vec<_> = bindings.iter().rev().copied().collect();
                    assert!(
                        scopes
                            .insert(
                                context.clone(),
                                json!({
                                    "root_owner": root.owner,
                                    "caller": root.caller,
                                    "expression": id,
                                    "syntactic_tail_position": tail,
                                    "outer_bindings": &lexical_bindings,
                                    "bindings": ordered,
                                    "body": body,
                                    "reverse_declaration_order": reverse,
                                })
                            )
                            .is_none()
                    );
                }
                E::Sequence { items } => {
                    let last = items.len().checked_sub(1);
                    for (index, child) in items.iter().enumerate() {
                        children.push(Frame::Enter {
                            expression: *child,
                            tail: tail && Some(index) == last,
                            bindings: lexical_bindings.clone(),
                        });
                    }
                }
                _ => {
                    // Other current-profile children are operands. children()
                    // omits Let initializers, which are handled above.
                    for child in record.children() {
                        children.push(Frame::Enter {
                            expression: child.expression,
                            tail: false,
                            bindings: lexical_bindings.clone(),
                        });
                    }
                }
            }
            pending.extend(children.into_iter().rev());
        }
        assert!(active.is_empty());
    }
    assert_eq!(
        visited_expressions, supplied_expressions,
        "complete expression coverage"
    );
    assert_eq!(
        visited_calls, supplied_calls,
        "complete direct-call coverage"
    );
    assert_eq!(
        visited_bindings, supplied_bindings,
        "complete Let-binding coverage"
    );
    let tail_contexts = call_sites.keys().filter(|key| key.2).count();
    let tail_sites: BTreeSet<_> = call_sites
        .keys()
        .filter(|key| key.2)
        .map(|key| key.1)
        .collect();
    let maximum_lexical_bindings = visited_contexts
        .iter()
        .map(|key| key.3.len())
        .max()
        .unwrap_or(0);
    let components = components(&graph);
    let recursive_components: Vec<_> = components
        .iter()
        .filter(|members| {
            members.len() > 1 || graph.get(&members[0]).unwrap().contains(&members[0])
        })
        .cloned()
        .collect();
    let recursive_declarations = recursive_components.iter().map(Vec::len).sum::<usize>();
    let edges: Vec<_> = graph
        .iter()
        .flat_map(|(caller, callees)| {
            callees
                .iter()
                .map(move |callee| json!({ "caller": caller, "callee": callee }))
        })
        .collect();
    let vertices: Vec<_> = graph.keys().copied().collect();
    let root_records: Vec<_> = roots
        .iter()
        .map(|root| {
            json!({
                "owner": root.owner, "caller": root.caller, "expression": root.expression,
            })
        })
        .collect();
    let summary = json!({
        "roots": roots.len(),
        "callable_vertices": graph.len(),
        "direct_call_edges": edges.len(),
        "expression_owners": supplied_expressions.len(),
        "expression_contexts": visited_contexts.len(),
        "call_expression_owners": supplied_calls.len(),
        "call_contexts": call_sites.len(),
        "syntactic_tail_call_expression_owners": tail_sites.len(),
        "syntactic_tail_call_contexts": tail_contexts,
        "let_binding_owners": supplied_bindings.len(),
        "let_scope_contexts": scopes.len(),
        "maximum_lexical_bindings": maximum_lexical_bindings,
        "components": components.len(),
        "recursive_components": recursive_components.len(),
        "recursive_declarations": recursive_declarations,
    });
    json!({
        "scope": "Every supplied function body and expression-backed port; complete expression, call and Let-binding owner coverage",
        "graph_semantics": "Declaration-level direct calls; exact generic substitutions remain at call sites; no normalized specialization graph",
        "tail_semantics": "Syntactic result positions; no claim that native TailCall lowering is permitted",
        "lexical_semantics": "Declared binding scopes and reverse declaration order; no owned-slot liveness or disposal inference",
        "summary": summary,
        "roots": root_records,
        "vertices": vertices,
        "edges": edges,
        "components": components,
        "recursive_components": recursive_components,
        "call_sites": call_sites.into_values().collect::<Vec<_>>(),
        "lexical_scopes": scopes.into_values().collect::<Vec<_>>(),
    })
}
