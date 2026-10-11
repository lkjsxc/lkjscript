//! Exact associated-output demands over the finite callable context graph.
//! A selection is a shape, never a value or a slot identity. Paths belong to an
//! exact context; even equal selections at different roots keep separate slots.
use super::*;

#[cfg(test)]
#[path = "associated_callable_flow_tests.rs"]
mod tests;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Member {
    context: usize,
    path: Vec<usize>,
    contract: DeclarationReference,
    member: AssociatedTypeMemberId,
}

#[derive(Clone)]
enum Request {
    Formals { context: usize, path: Vec<usize> },
    Member(Member),
}

#[derive(Default)]
pub(super) struct State {
    pub(super) enabled: bool,
    formals: BTreeMap<(usize, Vec<usize>), Vec<usize>>,
    members: BTreeMap<Member, usize>,
    pending: Vec<Request>,
}

pub(super) fn invalid() -> Diagnostic {
    semantic(
        "kernel_callable_associated_scope",
        "associated callable flow requires an exact lexical witness, contract member and complete projection-free binding inventory",
    )
}

impl<R: CallableClosureRead + ?Sized> Analysis<'_, R> {
    pub(super) fn unknown_parameters(
        &mut self,
        parameters: &[super::super::super::ImplementationParameter],
    ) -> Result<Vec<SelectionId>, Diagnostic> {
        self.reserve::<SelectionId>(parameters.len())?;
        let mut result = Vec::new();
        for parameter in parameters {
            self.tick()?;
            let shape = Selection::Unknown {
                contract: parameter.contract,
            };
            self.comparison_work(2, self.selection_indexes.len())?;
            let selected = if let Some(selected) = self.selection_indexes.get(&shape) {
                *selected
            } else {
                self.reserve::<SelectionInfo>(1)?;
                self.reserve::<(Selection, SelectionId)>(1)?;
                let selected = SelectionId(self.selections.len());
                self.selection_indexes.insert(shape.clone(), selected);
                self.selections.push(SelectionInfo {
                    shape,
                    parameters: Vec::new(),
                    depth: 0,
                    has_parameters: false,
                });
                selected
            };
            result.push(selected);
        }
        Ok(result)
    }

    pub(super) fn selection_contract(
        &mut self,
        selected: SelectionId,
    ) -> Result<DeclarationReference, Diagnostic> {
        match self.selected_shape(selected)? {
            Selection::Unknown { contract } => Ok(contract),
            Selection::Scheme { implementation, .. } => {
                let DeclarationPayload::OwnedImplementation(scheme) =
                    self.declaration(implementation)?.payload
                else {
                    return Err(invalid());
                };
                Ok(scheme.contract)
            }
        }
    }

    fn associated_contract(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<super::super::super::OwnedContract, Diagnostic> {
        let record = self.declaration(reference)?;
        if record.header.owner != OwnerKey::Declaration(reference.declaration)
            || record.header.kind != super::super::super::OwnerKind::OwnedContract
        {
            return Err(invalid());
        }
        let DeclarationPayload::OwnedContract(contract) = record.payload else {
            return Err(invalid());
        };
        Ok(contract)
    }

    fn associated_member(
        &mut self,
        reference: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<(), Diagnostic> {
        let contract = self.associated_contract(reference)?;
        let mut matches = 0usize;
        for candidate in &contract.associated_types {
            self.tick()?;
            matches += usize::from(candidate.id == member);
        }
        if matches != 1 {
            return Err(invalid());
        }
        Ok(())
    }

    /// The surrounding source admission proves Owned/category/generation/dependency
    /// authority. This separate growth walk still checks every stored RHS and its
    /// exact formal scope, including bindings not requested by any projection.
    pub(super) fn validate_associated_scheme(
        &mut self,
        owner: DeclarationReference,
        scheme: &OwnedImplementation,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        let contract = self.read.owner(
            scheme.contract.package,
            OwnerKey::Declaration(scheme.contract.declaration),
        )?;
        let has_members = matches!(&contract,
            Some(OwnerRecord::Declaration(DeclarationRecord {
                payload: DeclarationPayload::OwnedContract(c), ..
            })) if !c.associated_types.is_empty());
        if scheme.associated_types.is_empty() && !has_members {
            return Ok(());
        }
        self.associated.enabled = true;
        let contract = self.associated_contract(scheme.contract)?;
        if scheme.associated_types.len() != contract.associated_types.len() {
            return Err(invalid());
        }
        self.reserve::<AssociatedTypeMemberId>(scheme.associated_types.len())?;
        let mut bound = BTreeSet::new();
        for binding in &scheme.associated_types {
            self.tick()?;
            self.associated_member(scheme.contract, binding.member)?;
            if !bound.insert(binding.member) {
                return Err(invalid());
            }
            self.reserve::<(TypeObjectDigest, usize)>(1)?;
            let mut pending = vec![(binding.ty, 0usize)];
            let mut visited = BTreeMap::new();
            while let Some((ty, depth)) = pending.pop() {
                self.tick()?;
                if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                    return Err(Diagnostic::new(
                        DiagnosticClass::Resource,
                        "kernel_callable_flow_type_depth",
                        "associated binding exceeds structural type depth",
                    ));
                }
                if visited.get(&ty).is_some_and(|previous| *previous >= depth) {
                    continue;
                }
                if !visited.contains_key(&ty) {
                    self.reserve::<(TypeObjectDigest, usize)>(1)?;
                }
                visited.insert(ty, depth);
                let object = self
                    .read
                    .type_object(owner.package, ty)?
                    .ok_or_else(invalid)?;
                match object.form {
                    TypeForm::AssociatedContractMember { .. }
                    | TypeForm::AssociatedWitnessProjection { .. } => return Err(invalid()),
                    TypeForm::TypeParameter { parameter } => {
                        if !scheme.type_parameters.contains(&parameter) {
                            return Err(invalid());
                        }
                    }
                    _ => {
                        self.reserve::<TypeObjectDigest>(object.child_type_count())?;
                        let children = object.child_types();
                        self.reserve::<(TypeObjectDigest, usize)>(children.len())?;
                        pending.extend(children.into_iter().map(|child| (child, depth + 1)));
                    }
                }
            }
        }
        Ok(())
    }

    fn selected_at(&mut self, context: usize, path: &[usize]) -> Result<SelectionId, Diagnostic> {
        let Some((root, suffix)) = path.split_first() else {
            return Err(invalid());
        };
        let mut selected = *self.contexts[context]
            .key
            .implementations
            .get(*root)
            .ok_or_else(invalid)?;
        for child in suffix {
            self.tick()?;
            let Selection::Scheme { prerequisites, .. } = &self.selections[selected.0].shape else {
                return Err(invalid());
            };
            selected = *prerequisites.get(*child).ok_or_else(invalid)?;
        }
        Ok(selected)
    }

    pub(super) fn projection_slot(
        &mut self,
        context: usize,
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<usize, Diagnostic> {
        self.associated.enabled = true;
        if scope != self.contexts[context].key.owner {
            return Err(invalid());
        }
        let declaration = self.declaration(scope)?;
        let ordinal = self.parameter_ordinal(&declaration.payload, parameter)?;
        if implementation_parameters(&declaration.payload)[ordinal].contract != contract {
            return Err(invalid());
        }
        self.reserve::<usize>(1)?;
        self.member_slot(Member {
            context,
            path: vec![ordinal],
            contract,
            member,
        })
    }

    fn formal_slots(&mut self, context: usize, path: Vec<usize>) -> Result<Vec<usize>, Diagnostic> {
        self.tick()?;
        self.comparison_work(path.len() + 1, self.associated.formals.len())?;
        let key = (context, path);
        if let Some(slots) = self.associated.formals.get(&key) {
            let count = slots.len();
            self.reserve::<usize>(count)?;
            return Ok(self.associated.formals[&key].clone());
        }
        let width = if key.1.is_empty() {
            self.contexts[context].parameters.len()
        } else {
            let selected = self.selected_at(context, &key.1)?;
            if matches!(self.selections[selected.0].shape, Selection::Unknown { .. }) {
                return Err(invalid());
            }
            self.selections[selected.0].parameters.len()
        };
        self.reserve::<((usize, Vec<usize>), Vec<usize>)>(1)?;
        self.reserve::<usize>(
            key.1
                .len()
                .checked_mul(2)
                .ok_or_else(super::super::storage_overflow)?,
        )?;
        self.reserve::<usize>(
            width
                .checked_mul(2)
                .ok_or_else(super::super::storage_overflow)?,
        )?;
        self.reserve::<Request>(1)?;
        let slots = if key.1.is_empty() {
            self.contexts[context].slots[..width].to_vec()
        } else {
            self.reserve::<(usize, usize)>(width)?;
            self.reserve::<usize>(width)?;
            let mut slots = Vec::new();
            for _ in 0..width {
                slots.push(self.new_associated_slot(context)?);
            }
            slots
        };
        self.associated.pending.push(Request::Formals {
            context,
            path: key.1.clone(),
        });
        self.associated.formals.insert(key, slots.clone());
        Ok(slots)
    }

    fn new_associated_slot(&mut self, context: usize) -> Result<usize, Diagnostic> {
        self.tick()?;
        let slot = self.slot_owners.len();
        let ordinal = self.contexts[context].slots.len();
        self.slot_owners.push((context, ordinal));
        self.contexts[context].slots.push(slot);
        Ok(slot)
    }

    fn member_slot(&mut self, key: Member) -> Result<usize, Diagnostic> {
        self.tick()?;
        self.associated_member(key.contract, key.member)?;
        let selected = self.selected_at(key.context, &key.path)?;
        if self.selection_contract(selected)? != key.contract {
            return Err(invalid());
        }
        self.comparison_work(key.path.len() + 3, self.associated.members.len())?;
        if let Some(slot) = self.associated.members.get(&key) {
            return Ok(*slot);
        }
        self.reserve::<(Member, usize)>(1)?;
        self.reserve::<Request>(1)?;
        self.reserve::<usize>(
            key.path
                .len()
                .checked_mul(2)
                .ok_or_else(super::super::storage_overflow)?,
        )?;
        self.reserve::<(usize, usize)>(1)?;
        self.reserve::<usize>(1)?;
        let slot = self.new_associated_slot(key.context)?;
        self.associated.pending.push(Request::Member(key.clone()));
        self.associated.members.insert(key, slot);
        Ok(slot)
    }
}

pub(super) fn connect<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    components: &[usize],
    all: bool,
) -> Result<(), Diagnostic> {
    analysis.reserve::<Vec<recursive::Incoming>>(analysis.contexts.len())?;
    let mut incoming: Vec<Vec<recursive::Incoming>> =
        (0..analysis.contexts.len()).map(|_| Vec::new()).collect();
    for index in 0..analysis.calls.len() {
        analysis.tick()?;
        let call = analysis.calls[index];
        if !all && components[call.from] != components[call.to] {
            continue;
        }
        analysis.reserve::<recursive::Incoming>(1)?;
        let mut entry = recursive::Incoming::new(call);
        let inputs = entry.resolve(|call| recursive::load_inputs(analysis, call))?;
        // Explicit actuals are roots even when ordinary demands cannot reach them.
        // A bare lexical forwarding operand does not eagerly expand its shape DAG.
        match &*inputs {
            recursive::Inputs::Application { operands, .. } => {
                for (root, operand) in operands.iter().enumerate() {
                    analysis.reserve::<usize>(1)?;
                    seed_operand(analysis, call.to, vec![root], operand)?;
                }
            }
            recursive::Inputs::Method(operand) => {
                seed_operand(analysis, call.to, Vec::new(), operand)?;
            }
        }
        incoming[call.to].push(entry);
    }
    for (context, calls) in incoming.iter().enumerate() {
        analysis.tick()?;
        if !calls.is_empty() {
            analysis.formal_slots(context, Vec::new())?;
            if all {
                seed_all_paths(analysis, context)?;
            }
        }
    }
    while let Some(request) = analysis.associated.pending.pop() {
        analysis.tick()?;
        let context = match &request {
            Request::Formals { context, .. } => *context,
            Request::Member(member) => {
                analysis.binding_edges(member)?;
                member.context
            }
        };
        for entry in &mut incoming[context] {
            analysis.tick()?;
            let call = entry.call;
            let inputs = entry.resolve(|call| recursive::load_inputs(analysis, call))?;
            transfer(analysis, &request, call, &inputs)?;
        }
    }
    Ok(())
}

fn seed_operand<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    context: usize,
    path: Vec<usize>,
    operand: &ImplementationOperand,
) -> Result<(), Diagnostic> {
    analysis.reserve::<(&ImplementationOperand, Vec<usize>)>(1)?;
    let mut pending = vec![(operand, path)];
    while let Some((operand, path)) = pending.pop() {
        analysis.tick()?;
        if path.len() > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH + 1 {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "exact associated witness path exceeds bounded prerequisite depth",
            ));
        }
        if let ImplementationOperand::Concrete {
            implementations, ..
        } = operand
        {
            analysis.reserve::<usize>(path.len())?;
            analysis.formal_slots(context, path.clone())?;
            for (child, operand) in implementations.iter().enumerate() {
                analysis.reserve::<usize>(path.len() + 1)?;
                analysis.reserve::<(&ImplementationOperand, Vec<usize>)>(1)?;
                let mut child_path = path.clone();
                child_path.push(child);
                pending.push((operand, child_path));
            }
        }
    }
    Ok(())
}

/// The eager fixture path materializes all exact paths and outputs independently
/// of the demands used by the normal entry. It is bounded by the same meter.
fn seed_all_paths<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    context: usize,
) -> Result<(), Diagnostic> {
    let count = analysis.contexts[context].key.implementations.len();
    analysis.reserve::<(SelectionId, Vec<usize>)>(count)?;
    analysis.reserve::<usize>(count)?;
    let mut pending: Vec<_> = analysis.contexts[context]
        .key
        .implementations
        .iter()
        .enumerate()
        .map(|(root, selection)| (*selection, vec![root]))
        .collect();
    while let Some((selected, path)) = pending.pop() {
        analysis.tick()?;
        let contract = analysis.selection_contract(selected)?;
        let members = analysis.associated_contract(contract)?.associated_types;
        for member in members {
            analysis.reserve::<usize>(path.len())?;
            analysis.member_slot(Member {
                context,
                path: path.clone(),
                contract,
                member: member.id,
            })?;
        }
        if let Selection::Scheme { prerequisites, .. } = analysis.selected_shape(selected)? {
            analysis.reserve::<usize>(path.len())?;
            analysis.formal_slots(context, path.clone())?;
            for (child, selected) in prerequisites.into_iter().enumerate() {
                analysis.reserve::<(SelectionId, Vec<usize>)>(1)?;
                analysis.reserve::<usize>(path.len() + 1)?;
                let mut child_path = path.clone();
                child_path.push(child);
                pending.push((selected, child_path));
            }
        }
    }
    Ok(())
}

impl<R: CallableClosureRead + ?Sized> Analysis<'_, R> {
    fn binding_edges(&mut self, member: &Member) -> Result<(), Diagnostic> {
        let selected = self.selected_at(member.context, &member.path)?;
        let Selection::Scheme { implementation, .. } = self.selected_shape(selected)? else {
            // The identity is the exact context/path/formal and contract/member;
            // forwarding adds alias edges, never constructors or invented bindings.
            return Ok(());
        };
        let DeclarationPayload::OwnedImplementation(scheme) =
            self.declaration(implementation)?.payload
        else {
            return Err(invalid());
        };
        let binding = scheme
            .associated_types
            .iter()
            .find(|binding| binding.member == member.member)
            .ok_or_else(invalid)?;
        self.reserve::<usize>(member.path.len())?;
        let formals = self.formal_slots(member.context, member.path.clone())?;
        let to = self.associated.members[member];
        self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
        let mut pending = vec![(binding.ty, Vec::new())];
        let mut visited = BTreeSet::new();
        while let Some((ty, path)) = pending.pop() {
            self.tick()?;
            if visited.contains(&(ty, !path.is_empty())) {
                continue;
            }
            self.reserve::<(TypeObjectDigest, bool)>(1)?;
            visited.insert((ty, !path.is_empty()));
            let object = self
                .read
                .type_object(implementation.package, ty)?
                .ok_or_else(invalid)?;
            match object.form {
                TypeForm::TypeParameter { parameter } => {
                    let ordinal = scheme
                        .type_parameters
                        .iter()
                        .position(|p| *p == parameter)
                        .ok_or_else(invalid)?;
                    self.edge(formals[ordinal], to, implementation, None, ordinal, path)?;
                }
                TypeForm::AssociatedContractMember { .. }
                | TypeForm::AssociatedWitnessProjection { .. } => return Err(invalid()),
                _ => {
                    self.reserve::<TypeObjectDigest>(object.child_type_count())?;
                    for (child, ty) in object.child_types().into_iter().enumerate() {
                        if path.len() >= crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                            return Err(Diagnostic::new(
                                DiagnosticClass::Resource,
                                "kernel_callable_flow_type_depth",
                                "associated binding exceeds structural type depth",
                            ));
                        }
                        self.reserve::<usize>(path.len() + 1)?;
                        self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                        let mut child_path = path.clone();
                        child_path.push(child);
                        pending.push((ty, child_path));
                    }
                }
            }
        }
        Ok(())
    }
}

fn transfer<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    request: &Request,
    call: recursive::CallSite,
    inputs: &recursive::Inputs,
) -> Result<(), Diagnostic> {
    let path = match request {
        Request::Formals { path, .. } => path,
        Request::Member(m) => &m.path,
    };
    let (mut operand, mut suffix) = match inputs {
        recursive::Inputs::Application {
            arguments,
            operands,
        } => {
            let Some((root, suffix)) = path.split_first() else {
                let Request::Formals { context, path } = request else {
                    return Err(invalid());
                };
                analysis.reserve::<usize>(path.len())?;
                let slots = analysis.formal_slots(*context, path.clone())?;
                return analysis.arguments(call.from, &slots, arguments, call.expression);
            };
            (operands.get(*root).ok_or_else(invalid)?, suffix)
        }
        recursive::Inputs::Method(operand) => (operand, path.as_slice()),
    };
    loop {
        analysis.tick()?;
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                if let Some((child, rest)) = suffix.split_first() {
                    operand = implementations.get(*child).ok_or_else(invalid)?;
                    suffix = rest;
                } else {
                    if let Request::Formals { context, path } = request {
                        analysis.reserve::<usize>(path.len())?;
                        let slots = analysis.formal_slots(*context, path.clone())?;
                        analysis.arguments(call.from, &slots, type_arguments, call.expression)?;
                    }
                    // A concrete member is derived from its own selected formal
                    // slots by binding_edges, not from all source actuals blindly.
                    return Ok(());
                }
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let owner = analysis.contexts[call.from].key.owner;
                if *scope != owner {
                    return Err(invalid());
                }
                let declaration = analysis.declaration(owner)?;
                let ordinal = analysis.parameter_ordinal(&declaration.payload, *parameter)?;
                analysis.reserve::<usize>(suffix.len() + 1)?;
                let mut source_path = vec![ordinal];
                source_path.extend_from_slice(suffix);
                match request {
                    Request::Formals { context, path } => {
                        let from = analysis.formal_slots(call.from, source_path)?;
                        analysis.reserve::<usize>(path.len())?;
                        let to = analysis.formal_slots(*context, path.clone())?;
                        if from.len() != to.len() {
                            return Err(invalid());
                        }
                        for (argument, (from, to)) in from.into_iter().zip(to).enumerate() {
                            analysis.edge(
                                from,
                                to,
                                owner,
                                call.expression,
                                argument,
                                Vec::new(),
                            )?;
                        }
                    }
                    Request::Member(target) => {
                        let from = analysis.member_slot(Member {
                            context: call.from,
                            path: source_path,
                            contract: target.contract,
                            member: target.member,
                        })?;
                        let to = analysis.associated.members[target];
                        // Path provenance is an alias, not a type constructor.
                        analysis.reserve::<usize>(target.path.len())?;
                        analysis.push_edge(Edge {
                            from,
                            to,
                            origin: owner,
                            expression: call.expression,
                            argument: 0,
                            path: target.path.clone(),
                            expanding: false,
                        })?;
                    }
                }
                return Ok(());
            }
        }
    }
}
