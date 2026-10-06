//! Complete source-closure flow, including statically selected cross-package method callbacks.
//! A finite conservative call graph first excludes unsupported recursive witness construction.
//! Exact prerequisite selections then retain independent type-growth slots/SCCs.

#[path = "prerequisite_callable_flow.rs"]
mod prerequisites;
#[path = "recursive_callable_flow.rs"]
mod recursive;

use super::{CallableFlowWork, MAXIMUM_ANALYSIS_BYTES, MAXIMUM_DIAGNOSTIC_PATH, semantic};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, DeclarationReference, ExpressionOperation,
    ImplementationOperand, KernelSnapshot, OwnedImplementation, OwnedMethodImplementation,
    OwnerKey, OwnerRecord, PackageId, TypeForm, TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::{
    ExpressionId, ImplementationParameterId, MethodId, TypeParameterId,
};
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
enum Selection {
    Unknown,
    Scheme {
        implementation: DeclarationReference,
        prerequisites: Vec<SelectionId>,
    },
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SelectionId(usize);
const UNKNOWN_SELECTION: SelectionId = SelectionId(0);
struct SelectionInfo {
    shape: Selection,
    parameters: Vec<TypeParameterId>,
    depth: usize,
    has_parameters: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Context {
    owner: DeclarationReference,
    // Implementation methods have separate contexts: choosing one method cannot invent calls
    // to other methods. All mappings are nevertheless seeded for independent admission.
    method: Option<MethodId>,
    implementations: Vec<SelectionId>,
}
#[derive(Clone)]
struct WitnessLayout {
    path: Vec<usize>,
    parameters: Vec<TypeParameterId>,
    start: usize,
}
#[derive(Clone)]
struct ContextInfo {
    key: Context,
    parameters: Vec<TypeParameterId>,
    witnesses: Vec<WitnessLayout>,
    slots: Vec<usize>,
}
impl ContextInfo {
    fn witness_slots(&self, path: &[usize]) -> &[usize] {
        let Ok(index) = self
            .witnesses
            .binary_search_by(|w| w.path.as_slice().cmp(path))
        else {
            return &[];
        };
        let witness = &self.witnesses[index];
        &self.slots[witness.start..witness.start + witness.parameters.len()]
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
    selections: Vec<SelectionInfo>,
    selection_indexes: BTreeMap<Selection, SelectionId>,
    pending: Vec<usize>,
    calls: Vec<recursive::CallSite>,
    slot_owners: Vec<(usize, usize)>,
    edges: Vec<Edge>,
}
fn implementation_parameters(
    payload: &DeclarationPayload,
) -> &[super::super::ImplementationParameter] {
    match payload {
        DeclarationPayload::Function(function) => &function.implementation_parameters,
        DeclarationPayload::OwnedImplementation(implementation) => {
            &implementation.implementation_parameters
        }
        _ => &[],
    }
}
impl<R: CallableClosureRead + ?Sized> Analysis<'_, R> {
    fn tick(&mut self) -> Result<(), Diagnostic> {
        self.steps(1)
    }
    fn steps(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        *self.work = self
            .work
            .checked_add(count)
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
    fn comparison_work(&mut self, width: usize, keys: usize) -> Result<(), Diagnostic> {
        // BTreeMap searches/insertions compare flat IDs. Conservatively charge both
        // traversals before either runs, including linear comparison of a full key.
        let levels = (usize::BITS - keys.saturating_add(1).leading_zeros()) as usize;
        let comparisons = levels
            .checked_mul(24)
            .and_then(|n| n.checked_add(2))
            .ok_or_else(super::storage_overflow)?;
        self.steps(
            width
                .saturating_add(1)
                .checked_mul(comparisons)
                .ok_or_else(super::storage_overflow)?,
        )
    }
    fn intern_selection(
        &mut self,
        implementation: DeclarationReference,
        prerequisites: Vec<SelectionId>,
    ) -> Result<SelectionId, Diagnostic> {
        self.tick()?;
        let mut depth = 0usize;
        let parameters = self.scheme_parameters(implementation)?;
        let mut has_parameters = !parameters.is_empty();
        for child in &prerequisites {
            self.tick()?;
            depth = depth.max(self.selections[child.0].depth + 1);
            has_parameters |= self.selections[child.0].has_parameters;
        }
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "exact prerequisite context exceeds bounded witness depth",
            ));
        }
        let width = prerequisites.len();
        let shape = Selection::Scheme {
            implementation,
            prerequisites,
        };
        self.comparison_work(width, self.selection_indexes.len())?;
        if let Some(id) = self.selection_indexes.get(&shape) {
            return Ok(*id);
        }
        self.reserve::<SelectionInfo>(1)?;
        self.reserve::<(Selection, SelectionId)>(1)?;
        self.reserve::<SelectionId>(width)?;
        let id = SelectionId(self.selections.len());
        self.selection_indexes.insert(shape.clone(), id);
        self.selections.push(SelectionInfo {
            shape,
            parameters,
            depth,
            has_parameters,
        });
        Ok(id)
    }
    fn selected_shape(&mut self, id: SelectionId) -> Result<Selection, Diagnostic> {
        self.tick()?;
        if let Selection::Scheme { prerequisites, .. } = &self.selections[id.0].shape {
            let width = prerequisites.len();
            self.steps(width)?;
            self.reserve::<SelectionId>(width)?;
        }
        Ok(self.selections[id.0].shape.clone())
    }
    fn witness_slots<'c>(
        &mut self,
        context: &'c ContextInfo,
        path: &[usize],
    ) -> Result<&'c [usize], Diagnostic> {
        let comparisons =
            (usize::BITS - context.witnesses.len().saturating_add(1).leading_zeros()) as usize + 1;
        self.steps(
            (path.len() + 1)
                .checked_mul(comparisons)
                .ok_or_else(super::storage_overflow)?,
        )?;
        Ok(context.witness_slots(path))
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
    fn mapping<'s>(
        &mut self,
        scheme: &'s OwnedImplementation,
        method: Option<MethodId>,
    ) -> Result<&'s OwnedMethodImplementation, Diagnostic> {
        for mapping in &scheme.methods {
            self.tick()?;
            if Some(mapping.method) == method {
                return Ok(mapping);
            }
        }
        Err(semantic(
            "kernel_callable_flow_method",
            "implementation method is absent",
        ))
    }
    fn parameter_ordinal(
        &mut self,
        payload: &DeclarationPayload,
        parameter: ImplementationParameterId,
    ) -> Result<usize, Diagnostic> {
        for (ordinal, candidate) in implementation_parameters(payload).iter().enumerate() {
            self.tick()?;
            if candidate.id == parameter {
                return Ok(ordinal);
            }
        }
        Err(semantic(
            "kernel_callable_flow_witness_scope",
            "witness parameter is absent",
        ))
    }
    fn context(&mut self, key: Context) -> Result<usize, Diagnostic> {
        self.tick()?;
        // Interned selection IDs keep comparison and key cloning independent of DAG paths.
        self.reserve::<SelectionId>(key.implementations.len())?;
        self.comparison_work(key.implementations.len(), self.indexes.len())?;
        if let Some(index) = self.indexes.get(&key) {
            return Ok(*index);
        }
        let declaration = self.declaration(key.owner)?;
        let parameter_inventory = declaration.payload.type_parameters();
        self.reserve::<TypeParameterId>(parameter_inventory.iter().count())?;
        let parameters = parameter_inventory.to_vec();
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
            }
            DeclarationPayload::OwnedImplementation(i) => {
                if i.implementation_parameters.len() != key.implementations.len() {
                    return Err(semantic(
                        "kernel_callable_flow_method",
                        "implementation context has no exact method mapping",
                    ));
                }
                self.mapping(i, key.method)?;
            }
            _ if key.method.is_some() || !key.implementations.is_empty() => {
                return Err(semantic(
                    "kernel_callable_flow_context",
                    "declaration cannot carry this callable context",
                ));
            }
            _ => {}
        }
        // Discover exact callable contexts before expanding typed witness paths.
        // Only contexts participating in a call cycle can contain a type-growth cycle.
        let total = parameters.len();
        self.reserve::<ContextInfo>(1)?;
        self.reserve::<(Context, usize)>(1)?;
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
            witnesses: Vec::new(),
            slots,
        });
        self.pending.push(index);
        Ok(index)
    }
    fn materialize_witnesses(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.steps(self.contexts[index].key.implementations.len())?;
        self.reserve::<SelectionId>(self.contexts[index].key.implementations.len())?;
        let selections = self.contexts[index].key.implementations.clone();
        let start = self.contexts[index].parameters.len();
        let mut total = start;
        let mut witnesses = Vec::new();
        for (ordinal, selected) in selections.into_iter().enumerate() {
            self.reserve::<usize>(1)?;
            self.witness_layout(selected, vec![ordinal], &mut witnesses, &mut total)?;
        }
        self.reserve::<usize>(total - start)?;
        self.reserve::<(usize, usize)>(total - start)?;
        for ordinal in start..total {
            self.tick()?;
            let slot = self.slot_owners.len();
            self.slot_owners.push((index, ordinal));
            self.contexts[index].slots.push(slot);
        }
        self.contexts[index].witnesses = witnesses;
        Ok(())
    }
    fn record_call(
        &mut self,
        from: usize,
        to: usize,
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.reserve::<recursive::CallSite>(1)?;
        self.calls.push(recursive::CallSite {
            from,
            to,
            expression,
        });
        Ok(())
    }
    fn info(&mut self, index: usize) -> Result<ContextInfo, Diagnostic> {
        // Type-provenance paths remain separate, and are metered before their copies.
        self.steps(self.contexts[index].key.implementations.len())?;
        self.steps(
            self.contexts[index].parameters.len()
                + self.contexts[index].slots.len()
                + self.contexts[index].witnesses.len(),
        )?;
        self.reserve::<ContextInfo>(1)?;
        self.reserve::<TypeParameterId>(self.contexts[index].parameters.len())?;
        self.reserve::<WitnessLayout>(self.contexts[index].witnesses.len())?;
        for ordinal in 0..self.contexts[index].witnesses.len() {
            self.steps(
                self.contexts[index].witnesses[ordinal].path.len()
                    + self.contexts[index].witnesses[ordinal].parameters.len(),
            )?;
            self.reserve::<TypeParameterId>(
                self.contexts[index].witnesses[ordinal].parameters.len(),
            )?;
            self.reserve::<usize>(self.contexts[index].witnesses[ordinal].path.len())?;
        }
        self.reserve::<usize>(self.contexts[index].slots.len())?;
        self.reserve::<SelectionId>(self.contexts[index].key.implementations.len())?;
        Ok(self.contexts[index].clone())
    }
    fn witness_layout(
        &mut self,
        selection: SelectionId,
        path: Vec<usize>,
        result: &mut Vec<WitnessLayout>,
        total: &mut usize,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        if !self.selections[selection.0].has_parameters {
            return Ok(());
        }
        let Selection::Scheme { prerequisites, .. } = self.selected_shape(selection)? else {
            return Ok(());
        };
        self.reserve::<TypeParameterId>(self.selections[selection.0].parameters.len())?;
        self.steps(self.selections[selection.0].parameters.len() + path.len())?;
        let parameters = self.selections[selection.0].parameters.clone();
        self.reserve::<WitnessLayout>(1)?;
        self.reserve::<usize>(path.len())?;
        let start = *total;
        *total = total
            .checked_add(parameters.len())
            .ok_or_else(super::storage_overflow)?;
        result.push(WitnessLayout {
            path: path.clone(),
            parameters,
            start,
        });
        for (ordinal, prerequisite) in prerequisites.iter().enumerate() {
            self.reserve::<usize>(path.len() + 1)?;
            let mut child = path.clone();
            child.push(ordinal);
            self.witness_layout(*prerequisite, child, result, total)?;
        }
        Ok(())
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
        self.argument_types(
            source,
            target_slots.len(),
            Some(target_slots),
            arguments,
            expression,
        )
    }
    fn argument_types(
        &mut self,
        source: usize,
        expected: usize,
        targets: Option<&[usize]>,
        arguments: &[TypeObjectDigest],
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        if arguments.len() != expected {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable application requires every ordered type argument",
            ));
        }
        let caller = self.contexts[source].key.owner;
        for (argument, ty) in arguments.iter().enumerate() {
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
                    if let Some(targets) = targets {
                        let from = self.contexts[source].slots[ordinal];
                        self.edge(from, targets[argument], caller, expression, argument, path)?;
                    }
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
    ) -> Result<SelectionId, Diagnostic> {
        self.selection_at(source, operand, 0)
    }
    fn selection_at(
        &mut self,
        source: usize,
        operand: &ImplementationOperand,
        depth: usize,
    ) -> Result<SelectionId, Diagnostic> {
        self.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "applied witness exceeds bounded prerequisite depth",
            ));
        }
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let DeclarationPayload::OwnedImplementation(scheme) =
                    self.declaration(*implementation)?.payload
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
                        "applied witness requires every ordered scheme argument",
                    ));
                }
                // Acyclic contexts still admit every supplied type, including phantom
                // arguments and operands in unused methods or untaken syntax.
                self.argument_types(
                    source,
                    scheme.type_parameters.len(),
                    None,
                    type_arguments,
                    None,
                )?;
                self.reserve::<SelectionId>(implementations.len())?;
                let mut prerequisites = Vec::new();
                for operand in implementations {
                    prerequisites.push(self.selection_at(source, operand, depth + 1)?);
                }
                self.intern_selection(*implementation, prerequisites)
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let owner = self.contexts[source].key.owner;
                if *scope != owner {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter escapes its exact callable scope",
                    ));
                }
                let declaration = self.declaration(owner)?;
                let ordinal = self.parameter_ordinal(&declaration.payload, *parameter)?;
                Ok(self.contexts[source].key.implementations[ordinal])
            }
        }
    }
    fn witness_arguments(
        &mut self,
        source: usize,
        target: &ContextInfo,
        target_path: &[usize],
        operand: &ImplementationOperand,
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                let slots = self.witness_slots(target, target_path)?;
                self.arguments(source, slots, type_arguments, expression)?;
                for (ordinal, operand) in implementations.iter().enumerate() {
                    self.reserve::<usize>(target_path.len() + 1)?;
                    let mut path = target_path.to_vec();
                    path.push(ordinal);
                    self.witness_arguments(source, target, &path, operand, expression)?;
                }
                Ok(())
            }
            ImplementationOperand::Parameter { parameter, .. } => {
                let caller = self.contexts[source].key.owner;
                let declaration = self.declaration(caller)?;
                let ordinal = self.parameter_ordinal(&declaration.payload, *parameter)?;
                let info = self.info(source)?;
                self.forward_witness(&info, &[ordinal], target, target_path, caller, expression)
            }
        }
    }
    fn forward_witness(
        &mut self,
        source: &ContextInfo,
        source_path: &[usize],
        target: &ContextInfo,
        target_path: &[usize],
        caller: DeclarationReference,
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        for layout in &source.witnesses {
            self.steps(source_path.len() + 1)?;
            if let Some(suffix) = layout.path.strip_prefix(source_path) {
                self.reserve::<usize>(target_path.len() + suffix.len())?;
                let mut path = target_path.to_vec();
                path.extend_from_slice(suffix);
                let from = self.witness_slots(source, &layout.path)?;
                let to = self.witness_slots(target, &path)?;
                if from.len() != to.len() {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "forwarded witness scheme arity differs",
                    ));
                }
                for (argument, (from, to)) in from.iter().zip(to).enumerate() {
                    self.edge(*from, *to, caller, expression, argument, Vec::new())?;
                }
            }
        }
        Ok(())
    }
    fn method_arguments(
        &mut self,
        source: usize,
        target: &ContextInfo,
        operand: &ImplementationOperand,
        expression: ExpressionId,
    ) -> Result<(), Diagnostic> {
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                self.arguments(
                    source,
                    &target.slots[..target.parameters.len()],
                    type_arguments,
                    Some(expression),
                )?;
                for (ordinal, operand) in implementations.iter().enumerate() {
                    if target.key.implementations[ordinal] != UNKNOWN_SELECTION {
                        self.witness_arguments(
                            source,
                            target,
                            &[ordinal],
                            operand,
                            Some(expression),
                        )?;
                    }
                }
            }
            ImplementationOperand::Parameter { parameter, .. } => {
                let info = self.info(source)?;
                let declaration = self.declaration(info.key.owner)?;
                let ordinal = self.parameter_ordinal(&declaration.payload, *parameter)?;
                let from = self.witness_slots(&info, &[ordinal])?;
                let to = &target.slots[..target.parameters.len()];
                if from.len() != to.len() {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "selected method scheme arity differs",
                    ));
                }
                for (argument, (from, to)) in from.iter().zip(to).enumerate() {
                    self.edge(
                        *from,
                        *to,
                        info.key.owner,
                        Some(expression),
                        argument,
                        Vec::new(),
                    )?;
                }
                for child in 0..target.key.implementations.len() {
                    self.forward_witness(
                        &info,
                        &[ordinal, child],
                        target,
                        &[child],
                        info.key.owner,
                        Some(expression),
                    )?;
                }
            }
        }
        Ok(())
    }
    fn application(
        &mut self,
        source: usize,
        expression: Option<ExpressionId>,
        function: DeclarationReference,
        arguments: &[TypeObjectDigest],
        operands: &[ImplementationOperand],
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.observation.applications += 1;
        self.reserve::<SelectionId>(operands.len())?;
        let mut implementations = Vec::new();
        for operand in operands {
            implementations.push(self.selection(source, operand)?);
        }
        let target = self.context(Context {
            owner: function,
            method: None,
            implementations,
        })?;
        self.argument_types(
            source,
            self.contexts[target].parameters.len(),
            None,
            arguments,
            expression,
        )?;
        self.record_call(source, target, expression)
    }
    fn connect_application(
        &mut self,
        source: usize,
        target: &ContextInfo,
        arguments: &[TypeObjectDigest],
        operands: &[ImplementationOperand],
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        self.arguments(
            source,
            &target.slots[..target.parameters.len()],
            arguments,
            expression,
        )?;
        if operands.len() != target.key.implementations.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable witness arity changed",
            ));
        }
        for (ordinal, operand) in operands.iter().enumerate() {
            if target.key.implementations[ordinal] != UNKNOWN_SELECTION {
                self.witness_arguments(source, target, &[ordinal], operand, expression)?;
            }
        }
        Ok(())
    }
    fn scan(&mut self, source: usize) -> Result<(), Diagnostic> {
        self.tick()?;
        let info = self.info(source)?;
        let declaration = self.declaration(info.key.owner)?;
        if let DeclarationPayload::OwnedImplementation(i) = &declaration.payload {
            let mapping = self.mapping(i, info.key.method)?;
            self.application(
                source,
                None,
                mapping.function,
                &mapping.type_arguments,
                &mapping.implementations,
            )?;
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
                    self.application(source, Some(expression), *function, type_arguments, &[])?;
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    ..
                } => {
                    self.application(
                        source,
                        Some(expression),
                        *function,
                        type_arguments,
                        implementations,
                    )?;
                }
                ExpressionOperation::MethodCall {
                    witness, method, ..
                } => {
                    let selection = self.selection(source, witness)?;
                    if let Selection::Scheme {
                        implementation,
                        prerequisites,
                    } = self.selected_shape(selection)?
                    {
                        let target = self.context(Context {
                            owner: implementation,
                            method: Some(*method),
                            implementations: prerequisites,
                        })?;
                        self.record_call(source, target, Some(expression))?;
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
    validate_callable_closure_with_scope(read, work, maximum_work, recursive::ProofScope::Recursive)
}

fn validate_callable_closure_with_scope<R: CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut usize,
    maximum_work: usize,
    scope: recursive::ProofScope,
) -> Result<CallableFlowWork, Diagnostic> {
    let mut analysis = Analysis {
        read,
        work,
        maximum_work,
        observation: CallableFlowWork::default(),
        contexts: Vec::new(),
        indexes: BTreeMap::new(),
        selections: Vec::new(),
        selection_indexes: BTreeMap::new(),
        pending: Vec::new(),
        calls: Vec::new(),
        slot_owners: Vec::new(),
        edges: Vec::new(),
    };
    analysis.tick()?;
    analysis.reserve::<SelectionInfo>(1)?;
    analysis.selections.push(SelectionInfo {
        shape: Selection::Unknown,
        parameters: Vec::new(),
        depth: 0,
        has_parameters: false,
    });
    prerequisites::validate(&mut analysis)?;
    read.visit_callable_owners(&mut |owner| {
        let declaration = analysis.declaration(owner)?;
        match declaration.payload {
            DeclarationPayload::OwnedImplementation(i) => {
                analysis.reserve::<SelectionId>(i.implementation_parameters.len())?;
                for mapping in i.methods {
                    analysis.reserve::<SelectionId>(i.implementation_parameters.len())?;
                    analysis.context(Context {
                        owner,
                        method: Some(mapping.method),
                        implementations: vec![UNKNOWN_SELECTION; i.implementation_parameters.len()],
                    })?;
                }
            }
            DeclarationPayload::Function(f) => {
                analysis.reserve::<SelectionId>(f.implementation_parameters.len())?;
                analysis.context(Context {
                    owner,
                    method: None,
                    implementations: vec![UNKNOWN_SELECTION; f.implementation_parameters.len()],
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
    recursive::connect(&mut analysis, scope)?;
    analysis.admit()?;
    analysis.observation.slots = analysis.slot_owners.len();
    analysis.observation.edges = analysis.edges.len();
    Ok(analysis.observation)
}

#[cfg(test)]
#[path = "composed_callable_flow_tests.rs"]
mod tests;
