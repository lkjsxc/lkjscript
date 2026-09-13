//! Finite ordinary callable instantiation from explicit, simultaneous parameter flow.
//! No runtime branch, variance, nominal layout or preparation frontier changes this relation.

#[cfg(test)]
#[path = "callable_flow_tests.rs"]
mod tests;

use super::{
    DeclarationPayload, DeclarationReference, ExpressionOperation, ExpressionRead, OwnerKey,
    OwnerRecord, TypeForm, TypeObjectDigest,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{ExpressionId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

const MAXIMUM_ANALYSIS_BYTES: usize = 64 * 1_048_576;
const MAXIMUM_DIAGNOSTIC_PATH: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Slot {
    function: DeclarationReference,
    ordinal: usize,
    parameter: TypeParameterId,
}

struct Edge {
    from: usize,
    to: usize,
    expression: ExpressionId,
    argument: usize,
    argument_type: TypeObjectDigest,
    path: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CallableFlowWork {
    pub functions: usize,
    pub applications: usize,
    pub slots: usize,
    pub edges: usize,
    pub metadata_bytes: usize,
}

struct Flow<'a, R> {
    read: &'a R,
    work: &'a mut usize,
    maximum_work: usize,
    observation: CallableFlowWork,
    slots: Vec<Slot>,
    indexes: BTreeMap<Slot, usize>,
    edges: Vec<Edge>,
}

impl<R: ExpressionRead> Flow<'_, R> {
    fn tick(&mut self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        *self.work = self
            .work
            .checked_add(1)
            .filter(|next| *next <= self.maximum_work)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_work",
                    "callable parameter-flow analysis exhausted its work admission",
                )
            })?;
        Ok(())
    }

    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.observation.metadata_bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .and_then(|size| self.observation.metadata_bytes.checked_add(size))
            .filter(|size| *size <= MAXIMUM_ANALYSIS_BYTES)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_storage",
                    "callable parameter-flow analysis exceeded its metadata admission",
                )
            })?;
        Ok(())
    }

    fn slot(&mut self, slot: Slot) -> Result<usize, Diagnostic> {
        if let Some(index) = self.indexes.get(&slot) {
            return Ok(*index);
        }
        self.reserve::<(Slot, usize)>(2)?;
        let index = self.slots.len();
        self.slots.push(slot);
        self.indexes.insert(slot, index);
        Ok(index)
    }

    fn application(
        &mut self,
        caller: DeclarationReference,
        parameters: &[TypeParameterId],
        expression: ExpressionId,
        target: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.observation.applications += 1;
        // Exact package dependencies are acyclic. Foreign signature/type/constraint checks
        // belong to inference; complete supplier bodies are independently admitted at transport.
        if target.package != self.read.package_id() {
            return Ok(());
        }
        let Some(OwnerRecord::Declaration(declaration)) =
            self.read.owner(OwnerKey::Declaration(target.declaration))?
        else {
            return Err(semantic(
                "kernel_callable_flow_target",
                "callable application target is missing",
            ));
        };
        let target_parameters = match &declaration.payload {
            DeclarationPayload::Function(function) => &function.type_parameters,
            DeclarationPayload::External(function) => &function.type_parameters,
            _ => {
                return Err(semantic(
                    "kernel_callable_flow_target",
                    "application target is not callable",
                ));
            }
        };
        if target_parameters.len() != arguments.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable application requires every ordered ordinary type argument",
            ));
        }
        for (argument, (ty, target_parameter)) in
            arguments.iter().zip(target_parameters).enumerate()
        {
            self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
            let mut pending = vec![(*ty, Vec::new())];
            let mut visited = BTreeSet::new();
            while let Some((digest, path)) = pending.pop() {
                self.tick()?;
                // The same subterm at the entire argument and underneath a constructor has
                // different flow meaning. Keep both facts, retaining one exact occurrence path.
                let occurrence = (digest, !path.is_empty());
                if visited.contains(&occurrence) {
                    continue;
                }
                self.reserve::<(TypeObjectDigest, bool)>(1)?;
                visited.insert(occurrence);
                let object = self.read.type_object(digest)?.ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_type",
                        "callable application type object is missing",
                    )
                })?;
                if let TypeForm::TypeParameter { parameter } = object.form {
                    let ordinal = parameters
                        .iter()
                        .position(|value| *value == parameter)
                        .ok_or_else(|| {
                            semantic(
                                "kernel_type_parameter_scope",
                                "callable argument contains a foreign caller parameter",
                            )
                        })?;
                    let from = self.slot(Slot {
                        function: caller,
                        ordinal,
                        parameter,
                    })?;
                    let to = self.slot(Slot {
                        function: target,
                        ordinal: argument,
                        parameter: *target_parameter,
                    })?;
                    self.reserve::<Edge>(1)?;
                    self.edges.push(Edge {
                        from,
                        to,
                        expression,
                        argument,
                        argument_type: *ty,
                        path,
                    });
                } else {
                    for (index, child) in object.child_types().into_iter().enumerate() {
                        self.tick()?;
                        if path.len() >= super::contract::MAXIMUM_TYPE_DEPTH {
                            return Err(Diagnostic::new(
                                DiagnosticClass::Resource,
                                "kernel_callable_flow_type_depth",
                                "callable argument exceeds the structural type-depth contract",
                            ));
                        }
                        self.reserve::<usize>(path.len() + 1)?;
                        self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                        let mut child_path = path.clone();
                        child_path.push(index);
                        pending.push((child, child_path));
                    }
                }
            }
        }
        Ok(())
    }

    fn scan(&mut self, roots: impl IntoIterator<Item = OwnerKey>) -> Result<(), Diagnostic> {
        let mut pending = BTreeSet::new();
        for root in roots {
            self.tick()?;
            if let OwnerKey::Declaration(_) = root {
                self.reserve::<OwnerKey>(1)?;
                pending.insert(root);
            }
        }
        let mut visited = BTreeSet::new();
        while let Some(owner) = pending.pop_first() {
            self.tick()?;
            if visited.contains(&owner) {
                continue;
            }
            self.reserve::<OwnerKey>(1)?;
            visited.insert(owner);
            let OwnerKey::Declaration(id) = owner else {
                continue;
            };
            let Some(OwnerRecord::Declaration(declaration)) = self.read.owner(owner)? else {
                continue;
            };
            let parameters = declaration.payload.type_parameters().to_vec();
            let mut syntax = declaration.expression_roots();
            if syntax.is_empty() {
                continue;
            }
            self.observation.functions += usize::from(matches!(
                declaration.payload,
                DeclarationPayload::Function(_)
            ));
            self.reserve::<ExpressionId>(syntax.len())?;
            let caller = DeclarationReference {
                package: self.read.package_id(),
                declaration: id,
            };
            let mut expressions = BTreeSet::new();
            while let Some(expression) = syntax.pop() {
                self.tick()?;
                if expressions.contains(&expression) {
                    continue;
                }
                self.reserve::<ExpressionId>(1)?;
                expressions.insert(expression);
                let Some(OwnerRecord::Expression(record)) =
                    self.read.owner(OwnerKey::Expression(expression))?
                else {
                    return Err(semantic(
                        "kernel_callable_flow_expression",
                        "callable body expression is missing",
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
                        self.application(
                            caller,
                            &parameters,
                            expression,
                            *function,
                            type_arguments,
                        )?;
                        if function.package == self.read.package_id() {
                            self.reserve::<OwnerKey>(1)?;
                            pending.insert(OwnerKey::Declaration(function.declaration));
                        }
                    }
                    _ => {}
                }
                let children = record.children();
                self.reserve::<ExpressionId>(children.len())?;
                syntax.extend(children.into_iter().map(|child| child.expression));
            }
        }
        Ok(())
    }

    fn admit(&mut self) -> Result<(), Diagnostic> {
        let count = self.slots.len();
        self.reserve::<Vec<usize>>(count.checked_mul(2).ok_or_else(storage_overflow)?)?;
        self.reserve::<usize>(count.checked_mul(10).ok_or_else(storage_overflow)?)?;
        self.reserve::<usize>(
            self.edges
                .len()
                .checked_mul(2)
                .ok_or_else(storage_overflow)?,
        )?;
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for index in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[index];
            outgoing[edge.from].push(edge.to);
            incoming[edge.to].push(edge.from);
        }
        let mut seen = vec![false; count];
        let mut finish = Vec::new();
        for root in 0..count {
            self.tick()?;
            if seen[root] {
                continue;
            }
            seen[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, next)) = stack.last_mut() {
                self.tick()?;
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
        let mut component = vec![usize::MAX; count];
        let mut sizes = vec![0; count];
        for root in finish.into_iter().rev() {
            self.tick()?;
            if component[root] != usize::MAX {
                continue;
            }
            component[root] = root;
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                self.tick()?;
                sizes[root] += 1;
                for source in &incoming[node] {
                    self.tick()?;
                    if component[*source] == usize::MAX {
                        component[*source] = root;
                        pending.push(*source);
                    }
                }
            }
        }
        for index in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[index];
            if !edge.path.is_empty() && component[edge.from] == component[edge.to] {
                // The SCC decision is complete. Rendering cannot change semantic rejection into
                // resource exhaustion, even for a cycle much longer than the witness bound.
                let from = self.slots[edge.from];
                let to = self.slots[edge.to];
                let path = &edge.path[..edge.path.len().min(MAXIMUM_DIAGNOSTIC_PATH)];
                let omitted = if edge.path.len() > path.len() {
                    " (remaining constructor path omitted)"
                } else {
                    ""
                };
                return Err(semantic(
                    "kernel_callable_expansion",
                    format!(
                        "expanding callable parameter cycle: {}::{} slot {} ({}) -> {}::{} slot {} ({}); expression {}; argument {} type {}; constructor path {:?}{}; same strongly connected component ({} slots), return-path witness omitted; forward parameters without nesting on the cycle or reset to a closed type",
                        from.function.package,
                        from.function.declaration,
                        from.ordinal,
                        from.parameter,
                        to.function.package,
                        to.function.declaration,
                        to.ordinal,
                        to.parameter,
                        edge.expression,
                        edge.argument,
                        edge.argument_type,
                        path,
                        omitted,
                        sizes[component[edge.from]],
                    ),
                ));
            }
        }
        Ok(())
    }
}

fn semantic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

fn storage_overflow() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "kernel_callable_flow_storage",
        "callable flow metadata size overflowed",
    )
}

pub(crate) fn validate_callable_flow<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    work: &mut usize,
    maximum_work: usize,
) -> Result<CallableFlowWork, Diagnostic> {
    let mut flow = Flow {
        read,
        work,
        maximum_work,
        observation: CallableFlowWork::default(),
        slots: Vec::new(),
        indexes: BTreeMap::new(),
        edges: Vec::new(),
    };
    flow.scan(roots)?;
    flow.admit()?;
    flow.observation.slots = flow.slots.len();
    flow.observation.edges = flow.edges.len();
    Ok(flow.observation)
}
