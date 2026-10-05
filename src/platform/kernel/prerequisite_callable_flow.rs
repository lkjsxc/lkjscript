//! Conservative finite preflight, before exact prerequisite contexts are enumerated.
//! Contract-matched method alternatives intentionally overapproximate selected dispatch.
use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Node {
    owner: DeclarationReference,
    method: Option<MethodId>,
}
struct CallEdge {
    from: usize,
    to: usize,
    construction: bool,
    expression: Option<ExpressionId>,
}
struct Inventory {
    nodes: Vec<Node>,
    indexes: BTreeMap<Node, usize>,
    schemes: Vec<Scheme>,
}
struct Scheme {
    owner: DeclarationReference,
    contract: DeclarationReference,
    methods: Vec<MethodId>,
}

fn node<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &mut Inventory,
    key: Node,
) -> Result<(), Diagnostic> {
    analysis.tick()?;
    analysis.reserve::<Node>(1)?;
    analysis.reserve::<(Node, usize)>(1)?;
    inventory.indexes.insert(key, inventory.nodes.len());
    inventory.nodes.push(key);
    Ok(())
}
fn edge<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &mut Vec<CallEdge>,
    from: usize,
    target: Node,
    construction: bool,
    expression: Option<ExpressionId>,
) -> Result<(), Diagnostic> {
    analysis.tick()?;
    let to = *inventory.indexes.get(&target).ok_or_else(|| {
        semantic(
            "kernel_callable_flow_target",
            "complete callable source is missing an exact callable target",
        )
    })?;
    analysis.reserve::<CallEdge>(1)?;
    edges.push(CallEdge {
        from,
        to,
        construction,
        expression,
    });
    Ok(())
}

fn constructing<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    owner: DeclarationReference,
    operands: &[ImplementationOperand],
) -> Result<bool, Diagnostic> {
    analysis.reserve::<(&ImplementationOperand, bool, usize)>(operands.len())?;
    let mut pending: Vec<_> = operands.iter().map(|operand| (operand, false, 0)).collect();
    let mut result = false;
    while let Some((operand, beneath, depth)) = pending.pop() {
        analysis.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "prerequisite operand exceeds bounded witness depth",
            ));
        }
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let DeclarationPayload::OwnedImplementation(scheme) =
                    analysis.declaration(*implementation)?.payload
                else {
                    return Err(semantic(
                        "kernel_callable_flow_witness",
                        "selected witness does not name an implementation scheme",
                    ));
                };
                if scheme.type_parameters.len() != type_arguments.len()
                    || scheme.implementation_parameters.len() != implementations.len()
                {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "applied witness requires every ordered scheme argument and prerequisite",
                    ));
                }
                analysis.reserve::<(&ImplementationOperand, bool, usize)>(implementations.len())?;
                pending.extend(
                    implementations
                        .iter()
                        .map(|operand| (operand, true, depth + 1)),
                );
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let declaration = analysis.declaration(owner)?;
                if *scope != owner {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter escapes its exact callable scope",
                    ));
                }
                analysis.parameter_ordinal(&declaration.payload, *parameter)?;
                result |= beneath;
            }
        }
    }
    Ok(result)
}

fn method_edges<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &mut Vec<CallEdge>,
    from: usize,
    operand: &ImplementationOperand,
    method: MethodId,
    expression: ExpressionId,
) -> Result<(), Diagnostic> {
    let owner = inventory.nodes[from].owner;
    let construction = constructing(analysis, owner, std::slice::from_ref(operand))?;
    match operand {
        ImplementationOperand::Concrete { implementation, .. } => {
            edge(
                analysis,
                inventory,
                edges,
                from,
                Node {
                    owner: *implementation,
                    method: Some(method),
                },
                construction,
                Some(expression),
            )?;
        }
        ImplementationOperand::Parameter { parameter, .. } => {
            let declaration = analysis.declaration(owner)?;
            let ordinal = analysis.parameter_ordinal(&declaration.payload, *parameter)?;
            let contract = implementation_parameters(&declaration.payload)[ordinal].contract;
            for scheme in &inventory.schemes {
                analysis.tick()?;
                if scheme.contract != contract {
                    continue;
                }
                let mut matches = false;
                for candidate in &scheme.methods {
                    analysis.tick()?;
                    if *candidate == method {
                        matches = true;
                        break;
                    }
                }
                if matches {
                    edge(
                        analysis,
                        inventory,
                        edges,
                        from,
                        Node {
                            owner: scheme.owner,
                            method: Some(method),
                        },
                        construction,
                        Some(expression),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn scan<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &mut Vec<CallEdge>,
    source: usize,
) -> Result<(), Diagnostic> {
    let key = inventory.nodes[source];
    let declaration = analysis.declaration(key.owner)?;
    if let DeclarationPayload::OwnedImplementation(scheme) = &declaration.payload {
        let mapping = analysis.mapping(scheme, key.method)?;
        let construction = constructing(analysis, key.owner, &mapping.implementations)?;
        return edge(
            analysis,
            inventory,
            edges,
            source,
            Node {
                owner: mapping.function,
                method: None,
            },
            construction,
            None,
        );
    }
    let roots = declaration.expression_roots();
    analysis.reserve::<ExpressionId>(roots.len())?;
    let mut pending = roots;
    if let DeclarationPayload::Component { ports, .. } = &declaration.payload {
        for port in ports {
            analysis.tick()?;
            let Some(OwnerRecord::Port(record)) = analysis
                .read
                .owner(key.owner.package, OwnerKey::Port(*port))?
            else {
                return Err(semantic(
                    "kernel_callable_flow_port",
                    "complete callable source is missing a component port",
                ));
            };
            let roots = OwnerRecord::Port(record).expression_roots();
            analysis.reserve::<ExpressionId>(roots.len())?;
            pending.extend(roots);
        }
    }
    let mut visited = BTreeSet::new();
    while let Some(expression) = pending.pop() {
        analysis.tick()?;
        if visited.contains(&expression) {
            continue;
        }
        analysis.reserve::<ExpressionId>(1)?;
        visited.insert(expression);
        let Some(OwnerRecord::Expression(record)) = analysis
            .read
            .owner(key.owner.package, OwnerKey::Expression(expression))?
        else {
            return Err(semantic(
                "kernel_callable_flow_expression",
                "complete callable source is missing body syntax",
            ));
        };
        match &record.operation {
            ExpressionOperation::Call { function, .. }
            | ExpressionOperation::FunctionValue { function, .. } => {
                edge(
                    analysis,
                    inventory,
                    edges,
                    source,
                    Node {
                        owner: *function,
                        method: None,
                    },
                    false,
                    Some(expression),
                )?;
            }
            ExpressionOperation::ImplementationCall {
                function,
                implementations,
                ..
            } => {
                let construction = constructing(analysis, key.owner, implementations)?;
                edge(
                    analysis,
                    inventory,
                    edges,
                    source,
                    Node {
                        owner: *function,
                        method: None,
                    },
                    construction,
                    Some(expression),
                )?;
            }
            ExpressionOperation::MethodCall {
                witness, method, ..
            } => {
                method_edges(
                    analysis, inventory, edges, source, witness, *method, expression,
                )?;
            }
            _ => {}
        }
        let children = record.children();
        analysis.reserve::<ExpressionId>(children.len())?;
        pending.extend(children.into_iter().map(|c| c.expression));
    }
    Ok(())
}

fn admit<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    inventory: &Inventory,
    edges: &[CallEdge],
) -> Result<(), Diagnostic> {
    let count = inventory.nodes.len();
    analysis.reserve::<Vec<usize>>(
        count
            .checked_mul(2)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    analysis.reserve::<usize>(
        count
            .checked_mul(10)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    analysis.reserve::<usize>(
        edges
            .len()
            .checked_mul(2)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    let mut outgoing = vec![Vec::new(); count];
    let mut incoming = vec![Vec::new(); count];
    for edge in edges {
        analysis.tick()?;
        outgoing[edge.from].push(edge.to);
        incoming[edge.to].push(edge.from);
    }
    let mut seen = vec![false; count];
    let mut finish = Vec::new();
    for root in 0..count {
        analysis.tick()?;
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((node, next)) = stack.last_mut() {
            analysis.tick()?;
            if let Some(target) = outgoing[*node].get(*next) {
                *next += 1;
                if !seen[*target] {
                    seen[*target] = true;
                    stack.push((*target, 0));
                }
            } else {
                finish.push(*node);
                stack.pop();
            }
        }
    }
    let mut components = vec![usize::MAX; count];
    for root in finish.into_iter().rev() {
        analysis.tick()?;
        if components[root] != usize::MAX {
            continue;
        }
        components[root] = root;
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            analysis.tick()?;
            for source in &incoming[node] {
                analysis.tick()?;
                if components[*source] == usize::MAX {
                    components[*source] = root;
                    pending.push(*source);
                }
            }
        }
    }
    for edge in edges {
        analysis.tick()?;
        if edge.construction && components[edge.from] == components[edge.to] {
            let from = inventory.nodes[edge.from];
            let to = inventory.nodes[edge.to];
            return Err(semantic(
                "kernel_callable_recursive_witness_construction",
                format!(
                    "unsupported potential recursive prerequisite witness construction: {}::{} method {:?} -> {}::{} method {:?}, expression {:?}; contract-matched alternatives share a recursive component; forward or project lexical prerequisites, or reset to closed witnesses",
                    from.owner.package,
                    from.owner.declaration,
                    from.method,
                    to.owner.package,
                    to.owner.declaration,
                    to.method,
                    edge.expression
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
) -> Result<(), Diagnostic> {
    let mut inventory = Inventory {
        nodes: Vec::new(),
        indexes: BTreeMap::new(),
        schemes: Vec::new(),
    };
    analysis.read.visit_callable_owners(&mut |owner| {
        let declaration = analysis.declaration(owner)?;
        if let DeclarationPayload::OwnedImplementation(scheme) = declaration.payload {
            for mapping in &scheme.methods {
                node(
                    analysis,
                    &mut inventory,
                    Node {
                        owner,
                        method: Some(mapping.method),
                    },
                )?;
            }
            analysis.reserve::<Scheme>(1)?;
            analysis.reserve::<MethodId>(scheme.methods.len())?;
            inventory.schemes.push(Scheme {
                owner,
                contract: scheme.contract,
                methods: scheme.methods.iter().map(|m| m.method).collect(),
            });
        } else {
            node(
                analysis,
                &mut inventory,
                Node {
                    owner,
                    method: None,
                },
            )?;
        }
        Ok(())
    })?;
    let mut edges = Vec::new();
    for source in 0..inventory.nodes.len() {
        scan(analysis, &inventory, &mut edges, source)?;
    }
    admit(analysis, &inventory, &edges)
}
