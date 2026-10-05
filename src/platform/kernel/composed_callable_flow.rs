//! Complete source-closure flow, including statically selected cross-package method callbacks.
//! Context keys retain only finite declaration selections; type growth is decided by slots/SCCs.

use super::{CallableFlowWork, MAXIMUM_ANALYSIS_BYTES, MAXIMUM_DIAGNOSTIC_PATH, semantic};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, DeclarationReference, ExpressionOperation,
    ImplementationOperand, KernelSnapshot, OwnerKey, OwnerRecord, PackageId, TypeForm, TypeObject,
    TypeObjectDigest,
};
use crate::platform::semantic_id::{ExpressionId, MethodId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

/// Analyzer-only exact source access. Every declaration (including private bodies and tests) is
/// enumerated. Interfaces alone cannot establish the composed callback relation.
pub(crate) trait CallableClosureRead {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic>;
    fn owner(&self, package: PackageId, owner: OwnerKey)
    -> Result<Option<OwnerRecord>, Diagnostic>;
    fn type_object(
        &self,
        package: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic>;
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        Ok(())
    }
}

struct Snapshots<'a> {
    snapshots: &'a [&'a KernelSnapshot],
    checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
}
impl CallableClosureRead for Snapshots<'_> {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for (index, snapshot) in self.snapshots.iter().enumerate() {
            self.validation_checkpoint()?;
            if self.snapshots[..index]
                .iter()
                .any(|s| s.root.package_id == snapshot.root.package_id)
            {
                return Err(semantic(
                    "kernel_callable_closure_binding",
                    "source closure has more than one revision for a package",
                ));
            }
            for owner in snapshot.owners.keys() {
                self.validation_checkpoint()?;
                if let OwnerKey::Declaration(declaration) = owner {
                    visitor(DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *declaration,
                    })?;
                }
            }
        }
        Ok(())
    }
    fn owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self
            .snapshots
            .iter()
            .find(|s| s.root.package_id == package)
            .and_then(|s| s.owners.get(&owner))
            .cloned())
    }
    fn type_object(
        &self,
        package: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self
            .snapshots
            .iter()
            .find(|s| s.root.package_id == package)
            .and_then(|s| s.types.get(&ty).or_else(|| s.dependency_types.get(&ty)))
            .cloned())
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        (self.checkpoint)()
    }
}

/// The caller binds every supplied snapshot to an already admitted exact source revision. This
/// analysis writes no summary or canonical meaning and does not acquire repository locks.
pub(crate) fn validate_snapshot_callable_closure(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut usize,
    maximum_work: usize,
) -> Result<CallableFlowWork, Diagnostic> {
    validate_callable_closure(
        &Snapshots {
            snapshots,
            checkpoint,
        },
        work,
        maximum_work,
    )
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Context {
    owner: DeclarationReference,
    // Implementation methods have separate contexts: choosing one method cannot invent calls
    // to other methods. All mappings are nevertheless seeded for independent admission.
    method: Option<MethodId>,
    implementations: Vec<Option<DeclarationReference>>,
}
#[derive(Clone)]
struct ContextInfo {
    key: Context,
    parameters: Vec<TypeParameterId>,
    witness_parameters: Vec<Vec<TypeParameterId>>,
    slots: Vec<usize>,
}
impl ContextInfo {
    fn witness_slots(&self, ordinal: usize) -> &[usize] {
        let start = self.parameters.len()
            + self.witness_parameters[..ordinal]
                .iter()
                .map(Vec::len)
                .sum::<usize>();
        &self.slots[start..start + self.witness_parameters[ordinal].len()]
    }
}
struct Edge {
    from: usize,
    to: usize,
    origin: DeclarationReference,
    expression: Option<ExpressionId>,
    argument: usize,
    path: Vec<usize>,
}
struct Analysis<'a, R: ?Sized> {
    read: &'a R,
    work: &'a mut usize,
    maximum_work: usize,
    observation: CallableFlowWork,
    contexts: Vec<ContextInfo>,
    indexes: BTreeMap<Context, usize>,
    pending: Vec<usize>,
    slot_owners: Vec<(usize, usize)>,
    edges: Vec<Edge>,
}
impl<R: CallableClosureRead + ?Sized> Analysis<'_, R> {
    fn tick(&mut self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        *self.work = self
            .work
            .checked_add(1)
            .filter(|n| *n <= self.maximum_work)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_work",
                    "composed callable flow exhausted its work admission",
                )
            })?;
        Ok(())
    }
    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.observation.metadata_bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .and_then(|n| self.observation.metadata_bytes.checked_add(n))
            .filter(|n| *n <= MAXIMUM_ANALYSIS_BYTES)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_storage",
                    "composed callable flow exhausted its metadata admission",
                )
            })?;
        Ok(())
    }
    fn declaration(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<DeclarationRecord, Diagnostic> {
        self.tick()?;
        match self.read.owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )? {
            Some(OwnerRecord::Declaration(record)) => Ok(record),
            _ => Err(semantic(
                "kernel_callable_flow_target",
                "complete callable source is missing an exact declaration",
            )),
        }
    }
    fn scheme_parameters(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<Vec<TypeParameterId>, Diagnostic> {
        match self.declaration(reference)?.payload {
            DeclarationPayload::OwnedImplementation(i) => {
                self.reserve::<TypeParameterId>(i.type_parameters.len())?;
                Ok(i.type_parameters)
            }
            _ => Err(semantic(
                "kernel_callable_flow_witness",
                "selected witness does not name an implementation scheme",
            )),
        }
    }
    fn context(&mut self, key: Context) -> Result<usize, Diagnostic> {
        self.tick()?;
        if let Some(index) = self.indexes.get(&key) {
            return Ok(*index);
        }
        let declaration = self.declaration(key.owner)?;
        let parameter_inventory = declaration.payload.type_parameters();
        self.reserve::<TypeParameterId>(parameter_inventory.iter().count())?;
        let parameters = parameter_inventory.to_vec();
        let mut witness_parameters = Vec::new();
        match &declaration.payload {
            DeclarationPayload::Function(f) => {
                if key.method.is_some()
                    || f.implementation_parameters.len() != key.implementations.len()
                {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "function context has a foreign method or witness arity",
                    ));
                }
                self.reserve::<Vec<TypeParameterId>>(key.implementations.len())?;
                for selected in &key.implementations {
                    witness_parameters.push(match selected {
                        Some(i) => self.scheme_parameters(*i)?,
                        None => Vec::new(),
                    });
                }
            }
            DeclarationPayload::OwnedImplementation(i) => {
                if !key.implementations.is_empty()
                    || !i.methods.iter().any(|m| Some(m.method) == key.method)
                {
                    return Err(semantic(
                        "kernel_callable_flow_method",
                        "implementation context has no exact method mapping",
                    ));
                }
            }
            _ if key.method.is_some() || !key.implementations.is_empty() => {
                return Err(semantic(
                    "kernel_callable_flow_context",
                    "declaration cannot carry this callable context",
                ));
            }
            _ => {}
        }
        let total = witness_parameters
            .iter()
            .try_fold(parameters.len(), |n, p| n.checked_add(p.len()))
            .ok_or_else(|| {
                semantic(
                    "kernel_callable_flow_arity",
                    "callable slot arity overflowed",
                )
            })?;
        self.reserve::<ContextInfo>(1)?;
        self.reserve::<(Context, usize)>(1)?;
        self.reserve::<Option<DeclarationReference>>(
            key.implementations
                .len()
                .checked_mul(2)
                .ok_or_else(super::storage_overflow)?,
        )?;
        self.reserve::<usize>(total)?;
        self.reserve::<(usize, usize)>(total)?;
        self.reserve::<usize>(1)?;
        let index = self.contexts.len();
        let slots = (0..total)
            .map(|ordinal| {
                let slot = self.slot_owners.len();
                self.slot_owners.push((index, ordinal));
                slot
            })
            .collect();
        self.indexes.insert(key.clone(), index);
        self.contexts.push(ContextInfo {
            key,
            parameters,
            witness_parameters,
            slots,
        });
        self.pending.push(index);
        Ok(index)
    }
    fn info(&mut self, index: usize) -> Result<ContextInfo, Diagnostic> {
        self.reserve::<ContextInfo>(1)?;
        self.reserve::<TypeParameterId>(self.contexts[index].parameters.len())?;
        self.reserve::<Vec<TypeParameterId>>(self.contexts[index].witness_parameters.len())?;
        let witnesses = self.contexts[index]
            .witness_parameters
            .iter()
            .map(Vec::len)
            .sum();
        self.reserve::<TypeParameterId>(witnesses)?;
        self.reserve::<usize>(self.contexts[index].slots.len())?;
        self.reserve::<Option<DeclarationReference>>(
            self.contexts[index].key.implementations.len(),
        )?;
        Ok(self.contexts[index].clone())
    }
    fn edge(
        &mut self,
        from: usize,
        to: usize,
        origin: DeclarationReference,
        expression: Option<ExpressionId>,
        argument: usize,
        path: Vec<usize>,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.reserve::<Edge>(1)?;
        self.edges.push(Edge {
            from,
            to,
            origin,
            expression,
            argument,
            path,
        });
        Ok(())
    }
    fn arguments(
        &mut self,
        source: usize,
        target_slots: &[usize],
        arguments: &[TypeObjectDigest],
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        if arguments.len() != target_slots.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable application requires every ordered type argument",
            ));
        }
        let caller = self.contexts[source].key.owner;
        for (argument, (ty, target)) in arguments.iter().zip(target_slots).enumerate() {
            self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
            let mut pending = vec![(*ty, Vec::new())];
            let mut visited = BTreeSet::new();
            while let Some((ty, path)) = pending.pop() {
                self.tick()?;
                let occurrence = (ty, !path.is_empty());
                if visited.contains(&occurrence) {
                    continue;
                }
                self.reserve::<(TypeObjectDigest, bool)>(1)?;
                visited.insert(occurrence);
                let object = self.read.type_object(caller.package, ty)?.ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_type",
                        "complete callable source is missing an application type",
                    )
                })?;
                if let TypeForm::TypeParameter { parameter } = object.form {
                    let ordinal = self.contexts[source]
                        .parameters
                        .iter()
                        .position(|p| *p == parameter)
                        .ok_or_else(|| {
                            semantic(
                                "kernel_type_parameter_scope",
                                "callable argument contains a foreign source parameter",
                            )
                        })?;
                    let from = self.contexts[source].slots[ordinal];
                    self.edge(from, *target, caller, expression, argument, path)?;
                } else {
                    for (index, child) in object.child_types().into_iter().enumerate() {
                        self.tick()?;
                        if path.len() >= crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                            return Err(Diagnostic::new(
                                DiagnosticClass::Resource,
                                "kernel_callable_flow_type_depth",
                                "callable argument exceeds structural type depth",
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
    fn selection(
        &mut self,
        source: usize,
        operand: &ImplementationOperand,
    ) -> Result<Option<DeclarationReference>, Diagnostic> {
        self.tick()?;
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
            } => {
                if self.scheme_parameters(*implementation)?.len() != type_arguments.len() {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "applied witness requires every ordered scheme argument",
                    ));
                }
                Ok(Some(*implementation))
            }
            ImplementationOperand::Parameter {
                function,
                parameter,
            } => {
                let owner = self.contexts[source].key.owner;
                if *function != owner {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter escapes its exact callable scope",
                    ));
                }
                let DeclarationPayload::Function(f) = self.declaration(owner)?.payload else {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter has no function",
                    ));
                };
                let ordinal = f
                    .implementation_parameters
                    .iter()
                    .position(|p| p.id == *parameter)
                    .ok_or_else(|| {
                        semantic(
                            "kernel_callable_flow_witness_scope",
                            "witness parameter is absent",
                        )
                    })?;
                Ok(self.contexts[source].key.implementations[ordinal])
            }
        }
    }
    fn witness_arguments(
        &mut self,
        source: usize,
        target_slots: &[usize],
        operand: &ImplementationOperand,
        expression: ExpressionId,
    ) -> Result<(), Diagnostic> {
        match operand {
            ImplementationOperand::Concrete { type_arguments, .. } => {
                self.arguments(source, target_slots, type_arguments, Some(expression))
            }
            ImplementationOperand::Parameter { parameter, .. } => {
                let caller = self.contexts[source].key.owner;
                let DeclarationPayload::Function(f) = self.declaration(caller)?.payload else {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter has no function",
                    ));
                };
                let ordinal = f
                    .implementation_parameters
                    .iter()
                    .position(|p| p.id == *parameter)
                    .ok_or_else(|| {
                        semantic(
                            "kernel_callable_flow_witness_scope",
                            "witness parameter is absent",
                        )
                    })?;
                self.reserve::<usize>(target_slots.len())?;
                let slots = self.contexts[source].witness_slots(ordinal).to_vec();
                if slots.len() != target_slots.len() {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "forwarded witness scheme arity differs",
                    ));
                }
                for (argument, (from, to)) in slots.into_iter().zip(target_slots).enumerate() {
                    self.edge(from, *to, caller, Some(expression), argument, Vec::new())?;
                }
                Ok(())
            }
        }
    }
    fn application(
        &mut self,
        source: usize,
        expression: ExpressionId,
        function: DeclarationReference,
        arguments: &[TypeObjectDigest],
        operands: &[ImplementationOperand],
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.observation.applications += 1;
        self.reserve::<Option<DeclarationReference>>(operands.len())?;
        let mut implementations = Vec::new();
        for operand in operands {
            implementations.push(self.selection(source, operand)?);
        }
        let target = self.context(Context {
            owner: function,
            method: None,
            implementations,
        })?;
        let info = self.info(target)?;
        self.arguments(
            source,
            &info.slots[..info.parameters.len()],
            arguments,
            Some(expression),
        )?;
        for (ordinal, operand) in operands.iter().enumerate() {
            // An unresolved symbolic witness has no declaration/type-argument slots yet. Every
            // concrete downstream selection creates its separate fully known context.
            if info.key.implementations[ordinal].is_some() {
                self.witness_arguments(source, info.witness_slots(ordinal), operand, expression)?;
            }
        }
        Ok(())
    }
    fn scan(&mut self, source: usize) -> Result<(), Diagnostic> {
        self.tick()?;
        let info = self.info(source)?;
        let declaration = self.declaration(info.key.owner)?;
        if let DeclarationPayload::OwnedImplementation(i) = &declaration.payload {
            let mapping = i
                .methods
                .iter()
                .find(|m| Some(m.method) == info.key.method)
                .ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_method",
                        "implementation method is absent",
                    )
                })?;
            let target = self.context(Context {
                owner: mapping.function,
                method: None,
                implementations: Vec::new(),
            })?;
            self.reserve::<usize>(self.contexts[target].parameters.len())?;
            let slots =
                self.contexts[target].slots[..self.contexts[target].parameters.len()].to_vec();
            self.arguments(source, &slots, &mapping.type_arguments, None)?;
            return Ok(());
        }
        self.observation.functions += usize::from(matches!(
            declaration.payload,
            DeclarationPayload::Function(_)
        ));
        let mut pending = declaration.expression_roots();
        self.reserve::<ExpressionId>(pending.len())?;
        if let DeclarationPayload::Component { ports, .. } = &declaration.payload {
            for port in ports {
                self.tick()?;
                let Some(OwnerRecord::Port(record)) = self
                    .read
                    .owner(info.key.owner.package, OwnerKey::Port(*port))?
                else {
                    return Err(semantic(
                        "kernel_callable_flow_port",
                        "complete callable source is missing a component port",
                    ));
                };
                let roots = OwnerRecord::Port(record).expression_roots();
                self.reserve::<ExpressionId>(roots.len())?;
                pending.extend(roots);
            }
        }
        let mut visited = BTreeSet::new();
        while let Some(expression) = pending.pop() {
            self.tick()?;
            if visited.contains(&expression) {
                continue;
            }
            self.reserve::<ExpressionId>(1)?;
            visited.insert(expression);
            let Some(OwnerRecord::Expression(record)) = self
                .read
                .owner(info.key.owner.package, OwnerKey::Expression(expression))?
            else {
                return Err(semantic(
                    "kernel_callable_flow_expression",
                    "complete callable source is missing body syntax",
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
                    self.application(source, expression, *function, type_arguments, &[])?;
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    ..
                } => {
                    self.application(
                        source,
                        expression,
                        *function,
                        type_arguments,
                        implementations,
                    )?;
                }
                ExpressionOperation::MethodCall {
                    witness, method, ..
                } => {
                    if let Some(implementation) = self.selection(source, witness)? {
                        let target = self.context(Context {
                            owner: implementation,
                            method: Some(*method),
                            implementations: Vec::new(),
                        })?;
                        self.reserve::<usize>(self.contexts[target].slots.len())?;
                        let slots = self.contexts[target].slots.clone();
                        self.witness_arguments(source, &slots, witness, expression)?;
                    }
                }
                _ => {}
            }
            let children = record.children();
            self.reserve::<ExpressionId>(children.len())?;
            pending.extend(children.into_iter().map(|c| c.expression));
        }
        Ok(())
    }
    fn admit(&mut self) -> Result<(), Diagnostic> {
        let count = self.slot_owners.len();
        self.reserve::<Vec<usize>>(count.checked_mul(2).ok_or_else(super::storage_overflow)?)?;
        self.reserve::<usize>(count.checked_mul(10).ok_or_else(super::storage_overflow)?)?;
        self.reserve::<usize>(
            self.edges
                .len()
                .checked_mul(2)
                .ok_or_else(super::storage_overflow)?,
        )?;
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for i in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[i];
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
        let mut components = vec![usize::MAX; count];
        for root in finish.into_iter().rev() {
            self.tick()?;
            if components[root] != usize::MAX {
                continue;
            }
            components[root] = root;
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                self.tick()?;
                for source in &incoming[node] {
                    self.tick()?;
                    if components[*source] == usize::MAX {
                        components[*source] = root;
                        pending.push(*source);
                    }
                }
            }
        }
        for i in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[i];
            if !edge.path.is_empty() && components[edge.from] == components[edge.to] {
                let (from_context, from_ordinal) = self.slot_owners[edge.from];
                let (to_context, to_ordinal) = self.slot_owners[edge.to];
                let from = &self.contexts[from_context].key;
                let to = &self.contexts[to_context].key;
                return Err(semantic(
                    "kernel_callable_expansion",
                    format!(
                        "expanding composed callable parameter cycle: {}::{} slot {} -> {}::{} slot {}; owner {}::{} expression {:?} argument {}; constructor path {:?}; same strongly connected component; forward parameters without nesting or reset to a closed type",
                        from.owner.package,
                        from.owner.declaration,
                        from_ordinal,
                        to.owner.package,
                        to.owner.declaration,
                        to_ordinal,
                        edge.origin.package,
                        edge.origin.declaration,
                        edge.expression,
                        edge.argument,
                        &edge.path[..edge.path.len().min(MAXIMUM_DIAGNOSTIC_PATH)]
                    ),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_callable_closure<R: CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut usize,
    maximum_work: usize,
) -> Result<CallableFlowWork, Diagnostic> {
    let mut analysis = Analysis {
        read,
        work,
        maximum_work,
        observation: CallableFlowWork::default(),
        contexts: Vec::new(),
        indexes: BTreeMap::new(),
        pending: Vec::new(),
        slot_owners: Vec::new(),
        edges: Vec::new(),
    };
    analysis.tick()?;
    read.visit_callable_owners(&mut |owner| {
        let declaration = analysis.declaration(owner)?;
        match declaration.payload {
            DeclarationPayload::OwnedImplementation(i) => {
                for mapping in i.methods {
                    analysis.context(Context {
                        owner,
                        method: Some(mapping.method),
                        implementations: Vec::new(),
                    })?;
                }
            }
            DeclarationPayload::Function(f) => {
                analysis
                    .reserve::<Option<DeclarationReference>>(f.implementation_parameters.len())?;
                analysis.context(Context {
                    owner,
                    method: None,
                    implementations: vec![None; f.implementation_parameters.len()],
                })?;
            }
            _ => {
                analysis.context(Context {
                    owner,
                    method: None,
                    implementations: Vec::new(),
                })?;
            }
        }
        Ok(())
    })?;
    while let Some(context) = analysis.pending.pop() {
        analysis.scan(context)?;
    }
    analysis.admit()?;
    analysis.observation.slots = analysis.slot_owners.len();
    analysis.observation.edges = analysis.edges.len();
    Ok(analysis.observation)
}

#[cfg(test)]
#[path = "composed_callable_flow_tests.rs"]
mod tests;
