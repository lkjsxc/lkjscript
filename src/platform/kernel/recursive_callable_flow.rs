//! A type-slot cycle projects to a cycle of exact callable contexts. Discover that
//! finite graph first; acyclic edges cannot participate in an expanding slot SCC.
//! This is pruning, not witness unification: recursive contexts keep every path.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct CallSite {
    pub from: usize,
    pub to: usize,
    pub expression: Option<ExpressionId>,
}
#[derive(Clone, Copy)]
pub(super) enum ProofScope {
    Recursive,
    // Small-fixture reference: retain the predecessor's complete path expansion.
    #[cfg(test)]
    All,
}
impl ProofScope {
    fn relevant(self, components: &[usize], call: CallSite) -> bool {
        match self {
            Self::Recursive => components[call.from] == components[call.to],
            #[cfg(test)]
            Self::All => true,
        }
    }
}

pub(super) fn connect<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    scope: ProofScope,
) -> Result<(), Diagnostic> {
    let components = components(analysis)?;
    analysis.reserve::<bool>(analysis.contexts.len())?;
    let mut needed = vec![false; analysis.contexts.len()];
    for index in 0..analysis.calls.len() {
        analysis.tick()?;
        let call = analysis.calls[index];
        if scope.relevant(&components, call) {
            needed[call.from] = true;
            needed[call.to] = true;
        }
    }
    for (index, needed) in needed.into_iter().enumerate() {
        analysis.tick()?;
        if needed {
            analysis.materialize_witnesses(index)?;
        }
    }
    for index in 0..analysis.calls.len() {
        analysis.tick()?;
        let call = analysis.calls[index];
        if scope.relevant(&components, call) {
            connect_site(analysis, call)?;
        }
    }
    Ok(())
}

fn components<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
) -> Result<Vec<usize>, Diagnostic> {
    let count = analysis.contexts.len();
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
        analysis
            .calls
            .len()
            .checked_mul(2)
            .ok_or_else(super::super::storage_overflow)?,
    )?;
    let mut outgoing = vec![Vec::new(); count];
    let mut incoming = vec![Vec::new(); count];
    for index in 0..analysis.calls.len() {
        analysis.tick()?;
        let call = analysis.calls[index];
        outgoing[call.from].push(call.to);
        incoming[call.to].push(call.from);
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
    Ok(components)
}

fn connect_site<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    call: CallSite,
) -> Result<(), Diagnostic> {
    let target = analysis.info(call.to)?;
    let owner = analysis.contexts[call.from].key.owner;
    if let Some(expression) = call.expression {
        let Some(OwnerRecord::Expression(record)) = analysis
            .read
            .owner(owner.package, OwnerKey::Expression(expression))?
        else {
            return Err(semantic(
                "kernel_callable_flow_expression",
                "recorded callable syntax is missing",
            ));
        };
        match &record.operation {
            ExpressionOperation::Call {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                same_target(&target, *function, None)?;
                analysis.connect_application(
                    call.from,
                    &target,
                    type_arguments,
                    &[],
                    Some(expression),
                )
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                implementations,
                ..
            } => {
                same_target(&target, *function, None)?;
                analysis.connect_application(
                    call.from,
                    &target,
                    type_arguments,
                    implementations,
                    Some(expression),
                )
            }
            ExpressionOperation::MethodCall {
                witness, method, ..
            } => {
                let selected = analysis.selection(call.from, witness)?;
                let Selection::Scheme {
                    implementation,
                    prerequisites,
                } = analysis.selected_shape(selected)?
                else {
                    return Err(semantic(
                        "kernel_callable_flow_context",
                        "recorded method selection is unresolved",
                    ));
                };
                same_target(&target, implementation, Some(*method))?;
                if prerequisites != target.key.implementations {
                    return Err(semantic(
                        "kernel_callable_flow_context",
                        "recorded method prerequisites changed",
                    ));
                }
                analysis.method_arguments(call.from, &target, witness, expression)
            }
            _ => Err(semantic(
                "kernel_callable_flow_context",
                "recorded expression is not callable",
            )),
        }
    } else {
        let declaration = analysis.declaration(owner)?;
        let DeclarationPayload::OwnedImplementation(scheme) = declaration.payload else {
            return Err(semantic(
                "kernel_callable_flow_context",
                "recorded method mapping is absent",
            ));
        };
        let mapping = analysis.mapping(&scheme, analysis.contexts[call.from].key.method)?;
        same_target(&target, mapping.function, None)?;
        analysis.connect_application(
            call.from,
            &target,
            &mapping.type_arguments,
            &mapping.implementations,
            None,
        )
    }
}
fn same_target(
    target: &ContextInfo,
    owner: DeclarationReference,
    method: Option<MethodId>,
) -> Result<(), Diagnostic> {
    if target.key.owner != owner || target.key.method != method {
        return Err(semantic(
            "kernel_callable_flow_context",
            "recorded exact callable target changed",
        ));
    }
    Ok(())
}
