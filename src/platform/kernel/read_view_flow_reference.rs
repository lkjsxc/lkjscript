//! Independent affine and provenance evaluation of raw canonical read views.
//!
//! The numbers below identify proof-local obligations. They are never type,
//! application, cache, transport, or runtime identities. A descriptor has an
//! explicit custody read even when it has no accessible reference fields.
use super::view_scopes::ReadPathStep;
use super::*;
use crate::platform::semantic_id::{BindingId, FieldId};

struct ScopedBody {
    binding: BindingId,
    body: ExpressionId,
}

#[derive(Clone, Debug)]
struct Value {
    ty: TypeObjectDigest,
    kind: ValueKind,
}

#[derive(Clone, Debug)]
enum ValueKind {
    Data,
    Owner,
    Read {
        origin: ReadProvenance,
        scopes: Vec<ReadScopeArgument>,
        lease: usize,
    },
    View {
        origin: ReadProvenance,
        scope: ReadScopeArgument,
        cell: usize,
        lease: usize,
        owning: bool,
    },
}

impl Value {
    fn entries(&self) -> Result<usize> {
        self.origin().map_or(Ok(1), |p| {
            p.path
                .len()
                .checked_add(self.scopes().len())
                .and_then(|n| n.checked_add(2))
                .ok_or(Failure::Capacity)
        })
    }
    fn origin(&self) -> Option<&ReadProvenance> {
        match &self.kind {
            ValueKind::Read { origin, .. } | ValueKind::View { origin, .. } => Some(origin),
            _ => None,
        }
    }
    fn lease(&self) -> Option<usize> {
        match self.kind {
            ValueKind::Read { lease, .. } | ValueKind::View { lease, .. } => Some(lease),
            _ => None,
        }
    }
    fn scopes(&self) -> &[ReadScopeArgument] {
        match &self.kind {
            ValueKind::Read { scopes, .. } => scopes,
            ValueKind::View { scope, .. } => std::slice::from_ref(scope),
            _ => &[],
        }
    }
}

#[derive(Clone)]
struct Slot {
    value: Value,
    available: bool,
}

#[derive(Clone, Default)]
struct Lease {
    guards: BTreeSet<LocalValueReference>,
    cells: BTreeSet<usize>,
}

#[derive(Clone)]
struct Cell {
    ty: TypeObjectDigest,
    origin: ReadProvenance,
    scope: ReadScopeArgument,
    // These are the cell's one set of stored cleanup obligations. Borrowing
    // a field acquires another lease; it never clones these obligations.
    fields: BTreeMap<FieldId, Value>,
    custody: Option<Value>,
    readers: usize,
}

#[derive(Clone, Default)]
struct State {
    locals: BTreeMap<LocalValueReference, Slot>,
    leases: BTreeMap<usize, Lease>,
    cells: BTreeMap<usize, Cell>,
    next: usize,
    // Initializer/adoption coordinates are lexical scope evidence. Merely
    // finding a binding somewhere in the snapshot does not activate it.
    lexical: Vec<BindingId>,
    containment: BTreeSet<(ReadScopeArgument, ReadScopeArgument)>,
}

impl State {
    fn fresh(&mut self) -> Result<usize> {
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or(Failure::Capacity)?;
        Ok(id)
    }
    fn entries(&self) -> Result<usize> {
        let mut count = self.locals.len();
        for slot in self.locals.values() {
            count = count
                .checked_add(slot.value.entries()?)
                .ok_or(Failure::Capacity)?;
        }
        for lease in self.leases.values() {
            count = count
                .checked_add(1 + lease.guards.len() + lease.cells.len())
                .ok_or(Failure::Capacity)?;
        }
        for cell in self.cells.values() {
            count = count
                .checked_add(4 + cell.fields.len() + cell.origin.path.len())
                .ok_or(Failure::Capacity)?;
            for value in cell.fields.values().chain(cell.custody.iter()) {
                count = count
                    .checked_add(value.entries()?)
                    .ok_or(Failure::Capacity)?;
            }
        }
        count
            .checked_add(self.lexical.len())
            .and_then(|n| n.checked_add(self.containment.len()))
            .ok_or(Failure::Capacity)
    }
    fn branch(&self, resolver: &mut Resolver<'_>) -> Result<Self> {
        resolver.reserve_entries(self.entries()?)?;
        Ok(self.clone())
    }
    fn acquire(&mut self, resolver: &mut Resolver<'_>, lease: Lease) -> Result<usize> {
        resolver.step(0)?;
        resolver.reserve_entries(1 + lease.guards.len() + lease.cells.len())?;
        for id in &lease.cells {
            let cell = self.cells.get_mut(id).ok_or(Failure::Authority)?;
            cell.readers = cell.readers.checked_add(1).ok_or(Failure::Capacity)?;
        }
        let id = self.fresh()?;
        self.leases.insert(id, lease);
        Ok(id)
    }
    fn retain(
        &mut self,
        resolver: &mut Resolver<'_>,
        value: &Value,
        extra_cell: Option<usize>,
    ) -> Result<Value> {
        resolver.step(0)?;
        let Some(old) = value.lease() else {
            return Err(Failure::Category);
        };
        let saved = self.leases.get(&old).ok_or(Failure::Authority)?;
        resolver.reserve_entries(saved.guards.len() + saved.cells.len() + 2)?;
        let mut lease = saved.clone();
        if let Some(id) = extra_cell {
            lease.cells.insert(id);
        }
        let id = self.acquire(resolver, lease)?;
        resolver.reserve_entries(
            value.origin().map_or(1, |p| p.path.len() + 2) + value.scopes().len(),
        )?;
        let mut result = value.clone();
        match &mut result.kind {
            ValueKind::Read { lease, .. } => *lease = id,
            ValueKind::View { lease, owning, .. } => {
                *lease = id;
                *owning = false;
            }
            _ => return Err(Failure::Category),
        }
        Ok(result)
    }
    fn release(&mut self, resolver: &mut Resolver<'_>, value: Value) -> Result<()> {
        let Some(first) = value.lease() else {
            return Ok(());
        };
        // Iterative proof cleanup mirrors obligations, not the production
        // runtime's finalizer implementation. Reserve the complete worklist.
        resolver.reserve_entries(self.leases.len().checked_add(1).ok_or(Failure::Capacity)?)?;
        let mut pending = vec![first];
        while let Some(id) = pending.pop() {
            resolver.step(0)?;
            let lease = self.leases.remove(&id).ok_or(Failure::Authority)?;
            for cell_id in lease.cells {
                let cell = self.cells.get_mut(&cell_id).ok_or(Failure::Authority)?;
                cell.readers = cell.readers.checked_sub(1).ok_or(Failure::Authority)?;
                if cell.readers == 0 {
                    let cell = self.cells.remove(&cell_id).ok_or(Failure::Authority)?;
                    for stored in cell.fields.into_values().chain(cell.custody) {
                        if let Some(id) = stored.lease() {
                            pending.push(id);
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn guarded(&self, local: LocalValueReference) -> bool {
        self.leases
            .values()
            .any(|lease| lease.guards.contains(&local))
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Demand {
    Any,
    Data,
    Owner,
    Borrow,
    View,
    Loan,
}

struct Outcome {
    state: State,
    value: Value,
}

// A summary records only proven canonical paths and field shapes. It contains
// no live cell/lease coordinate from the validating activation.
#[derive(Clone)]
struct Summary {
    ty: TypeObjectDigest,
    kind: SummaryKind,
}
#[derive(Clone)]
enum SummaryKind {
    Data,
    Owner,
    Read {
        origin: ReadProvenance,
    },
    View {
        origin: ReadProvenance,
        scope: ReadScopeArgument,
        fields: BTreeMap<FieldId, Summary>,
    },
}
impl Summary {
    fn entries(&self) -> Result<usize> {
        let mut count = 1usize;
        match &self.kind {
            SummaryKind::Read { origin } => {
                count = count
                    .checked_add(origin.path.len())
                    .ok_or(Failure::Capacity)?
            }
            SummaryKind::View { origin, fields, .. } => {
                count = count
                    .checked_add(origin.path.len() + fields.len() + 1)
                    .ok_or(Failure::Capacity)?;
                for field in fields.values() {
                    count = count
                        .checked_add(field.entries()?)
                        .ok_or(Failure::Capacity)?;
                }
            }
            _ => {}
        }
        Ok(count)
    }
}

struct Engine<'r, 's> {
    resolver: &'r mut Resolver<'s>,
    scopes: closure::Scopes,
    active: BTreeSet<DeclarationReference>,
    summaries: BTreeMap<DeclarationReference, Vec<Summary>>,
}

struct CallPlan {
    signature: view_signatures::Signature,
    environment: ReadViewEnvironment,
    function: Option<DeclarationReference>,
    method: Option<(DeclarationReference, crate::platform::semantic_id::MethodId)>,
    effect: FunctionEffect,
    arguments: Vec<ExpressionId>,
}

impl Resolver<'_> {
    /// Evaluates the actual raw body and every branch using fresh affine state.
    /// Refusal/capacity are kept distinct; no production classifier is input.
    pub(in super::super) fn validate_read_view_function(
        &mut self,
        function: DeclarationReference,
    ) -> Result<()> {
        let scopes = closure::Scopes::build(self)?;
        Engine {
            resolver: self,
            scopes,
            active: BTreeSet::new(),
            summaries: BTreeMap::new(),
        }
        .function(function)
        .map(|_| ())
    }
}

impl Engine<'_, '_> {
    fn occurrence(&self, function: DeclarationReference, key: OwnerKey) -> Occurrence {
        Occurrence {
            parameter_scope: Some(function),
            context: Context::Lexical(function),
            consumer: (function.package, key),
        }
    }
    fn check_depth(&mut self, depth: usize) -> Result<()> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(Failure::Capacity);
        }
        self.resolver.step(0)
    }
    fn expression(
        &mut self,
        function: DeclarationReference,
        id: ExpressionId,
    ) -> Result<ExpressionOperation> {
        self.resolver.step(0)?;
        if function.package != self.resolver.source.root.package_id
            || self.scopes.expression(self.resolver, id, 0)? != Some(function)
        {
            return Err(Failure::Authority);
        }
        let record = match self.resolver.source.owners.get(&OwnerKey::Expression(id)) {
            Some(OwnerRecord::Expression(e)) => e,
            _ => return Err(Failure::Missing),
        };
        if record.id != id
            || record.contract_version != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || (record.contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH
                && (record.operation.is_read_view_operation()
                    || !record.operation.read_scope_arguments().is_empty()))
        {
            return Err(Failure::Generation);
        }
        let mut count = record
            .children()
            .len()
            .checked_add(record.type_roots().len())
            .and_then(|n| n.checked_add(record.operation.read_scope_arguments().len()))
            .and_then(|n| n.checked_add(1))
            .ok_or(Failure::Capacity)?;
        if let ExpressionOperation::Let { bindings, .. } = &record.operation {
            count = count.checked_add(bindings.len()).ok_or(Failure::Capacity)?;
        }
        match &record.operation {
            ExpressionOperation::ImplementationCall {
                implementations, ..
            } => {
                for operand in implementations {
                    count = count
                        .checked_add(self.resolver.operand_entries(operand, 0)?)
                        .ok_or(Failure::Capacity)?;
                }
            }
            ExpressionOperation::MethodCall { witness, .. } => {
                count = count
                    .checked_add(self.resolver.operand_entries(witness, 0)?)
                    .ok_or(Failure::Capacity)?
            }
            _ => {}
        }
        self.resolver.reserve_entries(count)?;
        let record = match self.resolver.source.owners.get(&OwnerKey::Expression(id)) {
            Some(OwnerRecord::Expression(e)) => e,
            _ => return Err(Failure::Missing),
        };
        let roots = record.type_roots();
        let operation = record.operation.clone();
        self.resolver.admit_roots(
            &roots,
            self.occurrence(function, OwnerKey::Expression(id)),
            false,
        )?;
        Ok(operation)
    }
    fn binding(
        &mut self,
        function: DeclarationReference,
        id: BindingId,
        kind: BindingKind,
        initialized: bool,
    ) -> Result<BindingRecord> {
        self.resolver.step(0)?;
        if self.scopes.binding(self.resolver, id, 0)? != Some(function) {
            return Err(Failure::Authority);
        }
        let b = match self.resolver.source.owners.get(&OwnerKey::Binding(id)) {
            Some(OwnerRecord::Binding(b)) => b,
            _ => return Err(Failure::Missing),
        };
        if b.header.owner != OwnerKey::Binding(id)
            || b.header.kind != OwnerKind::Binding
            || b.header.contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || (b.header.contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH
                && matches!(kind, BindingKind::ReadView | BindingKind::ReadViewBorrow))
            || b.kind != kind
            || b.value.is_some() != initialized
            || (matches!(
                kind,
                BindingKind::ReadView | BindingKind::ReadViewBorrow | BindingKind::OwnedBorrow
            ) && b.declared_type.is_none())
        {
            return Err(Failure::Authority);
        }
        self.resolver.reserve_entries(1)?;
        let b = match self.resolver.source.owners.get(&OwnerKey::Binding(id)) {
            Some(OwnerRecord::Binding(b)) => b.clone(),
            _ => return Err(Failure::Missing),
        };
        if let Some(ty) = b.declared_type {
            self.resolver.admit_roots(
                &[ty],
                self.occurrence(function, OwnerKey::Binding(id)),
                false,
            )?;
        }
        Ok(b)
    }
    fn private_layout(
        &mut self,
        function: DeclarationReference,
        declaration: DeclarationReference,
    ) -> Result<()> {
        self.resolver.step(0)?;
        if function.package != declaration.package
            || declaration.package != self.resolver.source.root.package_id
        {
            return Err(Failure::Authority);
        }
        let module = |d: DeclarationReference| match self
            .resolver
            .source
            .owners
            .get(&OwnerKey::Declaration(d.declaration))
        {
            Some(OwnerRecord::Declaration(d)) => Some(d.module),
            _ => None,
        };
        if module(function).is_none() || module(function) != module(declaration) {
            return Err(Failure::Authority);
        }
        Ok(())
    }
    fn function(&mut self, function: DeclarationReference) -> Result<Vec<Summary>> {
        self.resolver.step(0)?;
        if let Some(saved) = self.summaries.get(&function) {
            let mut count = saved.len();
            for summary in saved {
                count = count
                    .checked_add(summary.entries()?)
                    .ok_or(Failure::Capacity)?;
            }
            self.resolver.reserve_entries(count)?;
            return Ok(saved.clone());
        }
        if self.active.contains(&function) {
            return Err(Failure::Unproved);
        }
        self.resolver.reserve_entries(1)?;
        self.active.insert(function);
        let signature = self.resolver.read_view_function_signature(function)?;
        let definition = match self
            .resolver
            .source
            .owners
            .get(&OwnerKey::Declaration(function.declaration))
        {
            Some(OwnerRecord::Declaration(d))
                if function.package == self.resolver.source.root.package_id
                    && matches!(
                        d.header.kind,
                        OwnerKind::PureFunction | OwnerKind::TaskFunction
                    ) =>
            {
                match &d.payload {
                    DeclarationPayload::Function(f) => f,
                    _ => return Err(Failure::Unproved),
                }
            }
            _ => return Err(Failure::Unproved),
        };
        self.resolver.reserve_entries(
            definition.parameters.len()
                + definition.read_scope_parameters.len()
                + definition.read_scope_sources.len()
                + definition.type_parameters.len()
                + definition.implementation_parameters.len()
                + definition.effect_parameters.len()
                + definition.requirement_parameters.len()
                + 1,
        )?;
        let definition = match self
            .resolver
            .source
            .owners
            .get(&OwnerKey::Declaration(function.declaration))
        {
            Some(OwnerRecord::Declaration(d)) => match &d.payload {
                DeclarationPayload::Function(f) => f.clone(),
                _ => return Err(Failure::Unproved),
            },
            _ => return Err(Failure::Unproved),
        };
        let mut state = State::default();
        self.resolver
            .reserve_entries(definition.parameters.len() * 3 + signature.source_scopes.len())?;
        for (position, id) in definition.parameters.iter().enumerate() {
            let local = LocalValueReference::FunctionParameter(*id);
            let ty = signature.parameters[position].0;
            let origin = ReadProvenance {
                anchor: SourceAnchor {
                    callable: function,
                    source: local,
                },
                path: Vec::new(),
            };
            let source_scopes = signature
                .source_scopes
                .iter()
                .filter_map(|(index, scope)| (*index == position).then_some(*scope))
                .collect::<Vec<_>>();
            let value = match signature.categories[position] {
                ReadCategory::ReadView => {
                    let scope = self
                        .resolver
                        .read_view_scope(ty)?
                        .ok_or(Failure::Category)?;
                    for child in &source_scopes {
                        self.resolver.reserve_entries(1)?;
                        state.containment.insert((*child, scope));
                    }
                    self.abstract_view(&mut state, ty, origin, scope, function, 0)?
                }
                ReadCategory::Owned if signature.parameters[position].1 == ParameterUse::Borrow => {
                    let lease = state.acquire(self.resolver, Lease::default())?;
                    Value {
                        ty,
                        kind: ValueKind::Read {
                            origin,
                            scopes: source_scopes,
                            lease,
                        },
                    }
                }
                ReadCategory::Owned => Value {
                    ty,
                    kind: ValueKind::Owner,
                },
                _ => Value {
                    ty,
                    kind: ValueKind::Data,
                },
            };
            state.locals.insert(
                local,
                Slot {
                    value,
                    available: true,
                },
            );
        }
        let demand = match signature.result.relation {
            ResultRelation::ViewFrom { .. } => Demand::View,
            ResultRelation::ReadFrom { .. } => Demand::Borrow,
            ResultRelation::Value if signature.result_category == ReadCategory::Owned => {
                Demand::Owner
            }
            ResultRelation::Value => Demand::Data,
        };
        let outcomes = self.eval(function, definition.body, state, demand, 0)?;
        self.resolver.reserve_entries(outcomes.len())?;
        let mut summaries = Vec::with_capacity(outcomes.len());
        for outcome in outcomes {
            self.validate_return(
                function,
                &definition,
                &signature,
                &outcome.state,
                &outcome.value,
            )?;
            summaries.push(self.freeze(&outcome.state, &outcome.value, 0)?);
        }
        self.active.remove(&function);
        let mut count = summaries.len() + 1;
        for summary in &summaries {
            count = count
                .checked_add(summary.entries()?)
                .ok_or(Failure::Capacity)?;
        }
        self.resolver.reserve_entries(count)?;
        self.summaries.insert(function, summaries.clone());
        Ok(summaries)
    }
    fn validate_return(
        &mut self,
        function: DeclarationReference,
        definition: &FunctionDeclaration,
        signature: &view_signatures::Signature,
        state: &State,
        value: &Value,
    ) -> Result<()> {
        self.resolver.step(0)?;
        if value.ty != signature.result.ty {
            return Err(Failure::Category);
        }
        match signature.result.relation {
            ResultRelation::Value => {
                if !matches!(value.kind, ValueKind::Data | ValueKind::Owner) {
                    return Err(Failure::Category);
                }
            }
            ResultRelation::ReadFrom { source } | ResultRelation::ViewFrom { source, .. } => {
                let local = LocalValueReference::FunctionParameter(definition.parameters[source]);
                let selected = state
                    .locals
                    .get(&local)
                    .ok_or(Failure::Authority)?
                    .value
                    .origin()
                    .ok_or(Failure::Authority)?;
                let origin = value.origin().ok_or(Failure::Category)?;
                if !origin.derives_from(selected) || origin.anchor.callable != function {
                    return Err(Failure::Authority);
                }
                match signature.result.relation {
                    ResultRelation::ReadFrom { .. } => {
                        if !matches!(value.kind, ValueKind::Read { .. }) {
                            return Err(Failure::Category);
                        }
                        if signature.categories[source] == ReadCategory::ReadView
                            && !matches!(
                                origin.path.get(selected.path.len()),
                                Some(
                                    ReadPathStep::ViewReference { .. }
                                        | ReadPathStep::MethodRead { .. }
                                )
                            )
                        {
                            return Err(Failure::Authority);
                        }
                    }
                    ResultRelation::ViewFrom { scope, .. } => {
                        let ValueKind::View {
                            cell,
                            scope: actual,
                            owning: true,
                            ..
                        } = value.kind
                        else {
                            return Err(Failure::Category);
                        };
                        if scope != actual {
                            return Err(Failure::Authority);
                        }
                        self.validate_cell_lineage(state, cell, selected, 0)?;
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }
    fn validate_cell_lineage(
        &mut self,
        state: &State,
        id: usize,
        selected: &ReadProvenance,
        depth: usize,
    ) -> Result<()> {
        self.resolver.step(depth)?;
        let cell = state.cells.get(&id).ok_or(Failure::Authority)?;
        if !cell.origin.derives_from(selected) {
            return Err(Failure::Authority);
        }
        for value in cell.fields.values().chain(cell.custody.iter()) {
            if let Some(origin) = value.origin()
                && !origin.derives_from(selected)
            {
                return Err(Failure::Authority);
            }
            if let ValueKind::View { cell, .. } = value.kind {
                self.validate_cell_lineage(state, cell, selected, depth + 1)?;
            }
        }
        Ok(())
    }
    fn freeze(&mut self, state: &State, value: &Value, depth: usize) -> Result<Summary> {
        self.resolver.step(depth)?;
        self.resolver
            .reserve_entries(1 + value.origin().map_or(0, |p| p.path.len()))?;
        let kind = match &value.kind {
            ValueKind::Data => SummaryKind::Data,
            ValueKind::Owner => SummaryKind::Owner,
            ValueKind::Read { origin, .. } => SummaryKind::Read {
                origin: origin.clone(),
            },
            ValueKind::View {
                origin,
                scope,
                cell,
                owning: true,
                ..
            } => {
                let cell = state.cells.get(cell).ok_or(Failure::Authority)?;
                self.resolver.reserve_entries(cell.fields.len())?;
                let mut fields = BTreeMap::new();
                for (id, value) in &cell.fields {
                    fields.insert(*id, self.freeze(state, value, depth + 1)?);
                }
                SummaryKind::View {
                    origin: origin.clone(),
                    scope: *scope,
                    fields,
                }
            }
            // A stored view referent is an accessible path, not an owning view
            // return. Its fields are selected again from the actual source.
            ValueKind::View { origin, .. } => SummaryKind::Read {
                origin: origin.clone(),
            },
        };
        Ok(Summary { ty: value.ty, kind })
    }

    fn view_fields(
        &mut self,
        function: DeclarationReference,
        ty: TypeObjectDigest,
    ) -> Result<(DeclarationReference, Vec<(ReadViewField, TypeObjectDigest)>)> {
        self.resolver.step(0)?;
        let TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope,
        } = self.resolver.object(ty)?.form.clone()
        else {
            return Err(Failure::Authority);
        };
        let view = self.resolver.read_view_declaration(declaration)?;
        self.resolver.admit_view_layout(declaration)?;
        if view.type_parameters.len() != arguments.len() {
            return Err(Failure::Arity);
        }
        self.resolver
            .reserve_entries(arguments.len() + view.fields.len() * 2 + 1)?;
        let binder = ReadScopeBinder::Declaration(declaration);
        let environment = ReadViewEnvironment {
            types: Environment {
                ordinary: view
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(arguments)
                    .collect(),
                source: Context::Forbidden,
                caller: self.occurrence(function, OwnerKey::Declaration(function.declaration)),
                contract_operand: None,
                witnesses: BTreeMap::new(),
            },
            source_binder: binder,
            caller_binder: ReadScopeBinder::Declaration(function),
            scopes: BTreeMap::from([(
                ReadScopeReference {
                    binder,
                    parameter: view.scope_parameter.id,
                },
                read_scope,
            )]),
        };
        let mut fields = Vec::with_capacity(view.fields.len());
        for field in view.fields {
            let actual = self.resolver.substitute_view(field.ty, &environment, 0)?;
            fields.push((field, actual));
        }
        Ok((declaration, fields))
    }
    fn cell_value(&mut self, state: &mut State, cell: Cell, owning: bool) -> Result<Value> {
        self.resolver
            .reserve_entries(4 + cell.fields.len() * 2 + cell.origin.path.len())?;
        let ty = cell.ty;
        let origin = cell.origin.clone();
        let scope = cell.scope;
        let id = state.fresh()?;
        state.cells.insert(id, cell);
        let lease = state.acquire(
            self.resolver,
            Lease {
                guards: BTreeSet::new(),
                cells: BTreeSet::from([id]),
            },
        )?;
        Ok(Value {
            ty,
            kind: ValueKind::View {
                origin,
                scope,
                cell: id,
                lease,
                owning,
            },
        })
    }
    fn abstract_view(
        &mut self,
        state: &mut State,
        ty: TypeObjectDigest,
        origin: ReadProvenance,
        scope: ReadScopeArgument,
        function: DeclarationReference,
        depth: usize,
    ) -> Result<Value> {
        self.resolver.step(depth)?;
        let mut fields = BTreeMap::new();
        if matches!(
            self.resolver.object(ty)?.form,
            TypeForm::ReadViewApplied { .. }
        ) {
            let (declaration, inventory) = self.view_fields(function, ty)?;
            self.resolver.reserve_entries(inventory.len())?;
            for (field, actual) in inventory {
                let value = match field.kind {
                    ReadViewFieldKind::Ordinary => Value {
                        ty: actual,
                        kind: ValueKind::Data,
                    },
                    ReadViewFieldKind::ReadReference { .. } => {
                        let path = origin.extend(
                            self.resolver,
                            ReadPathStep::ViewReference {
                                declaration,
                                field: field.id,
                            },
                        )?;
                        if self.resolver.read_view_category(
                            actual,
                            self.occurrence(function, OwnerKey::Declaration(function.declaration)),
                        )? == ReadCategory::ReadView
                        {
                            self.abstract_view(state, actual, path, scope, function, depth + 1)?
                        } else {
                            let lease = state.acquire(self.resolver, Lease::default())?;
                            Value {
                                ty: actual,
                                kind: ValueKind::Read {
                                    origin: path,
                                    scopes: vec![scope],
                                    lease,
                                },
                            }
                        }
                    }
                };
                fields.insert(field.id, value);
            }
        }
        self.cell_value(
            state,
            Cell {
                ty,
                origin,
                scope,
                fields,
                custody: None,
                readers: 0,
            },
            false,
        )
    }
    fn local(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        local: LocalValueReference,
        demand: Demand,
    ) -> Result<Value> {
        self.resolver.step(0)?;
        let slot = state.locals.get(&local).ok_or(Failure::Authority)?;
        if !slot.available {
            return Err(Failure::Authority);
        }
        self.resolver.reserve_entries(
            1 + slot.value.origin().map_or(0, |p| p.path.len()) + slot.value.scopes().len(),
        )?;
        let value = slot.value.clone();
        match value.kind {
            ValueKind::Data if matches!(demand, Demand::Any | Demand::Data) => Ok(value),
            ValueKind::Owner if demand == Demand::Borrow => {
                let lease = state.acquire(
                    self.resolver,
                    Lease {
                        guards: BTreeSet::from([local]),
                        cells: BTreeSet::new(),
                    },
                )?;
                Ok(Value {
                    ty: value.ty,
                    kind: ValueKind::Read {
                        origin: ReadProvenance {
                            anchor: SourceAnchor {
                                callable: function,
                                source: local,
                            },
                            path: Vec::new(),
                        },
                        scopes: Vec::new(),
                        lease,
                    },
                })
            }
            ValueKind::Owner if matches!(demand, Demand::Any | Demand::Owner) => {
                if state.guarded(local) {
                    return Err(Failure::Authority);
                }
                state
                    .locals
                    .get_mut(&local)
                    .ok_or(Failure::Authority)?
                    .available = false;
                Ok(value)
            }
            ValueKind::Read { .. } | ValueKind::View { .. } if demand == Demand::Borrow => {
                state.retain(self.resolver, &value, None)
            }
            _ => Err(Failure::Category),
        }
    }
    fn admit_scope(
        &mut self,
        function: DeclarationReference,
        state: &State,
        scope: ReadScopeArgument,
    ) -> Result<()> {
        self.resolver.admit_scope_argument(
            &scope,
            &ReadScopeBinder::Declaration(function),
            self.occurrence(function, OwnerKey::Declaration(function.declaration)),
        )?;
        if let ReadScopeArgument::Lexical { callable, binding } = scope
            && (callable != function || !state.lexical.contains(&binding))
        {
            return Err(Failure::Authority);
        }
        Ok(())
    }
    fn permits_scope(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        value: &Value,
        scope: ReadScopeArgument,
    ) -> Result<()> {
        self.admit_scope(function, state, scope)?;
        let lexical = matches!(scope, ReadScopeArgument::Lexical { .. });
        if value.scopes().is_empty() {
            // Only an owning local read has no certified incoming scope. Its
            // loan can live in the active lexical coordinate, never a formal.
            let origin = value.origin().ok_or(Failure::Category)?;
            if !lexical
                || !matches!(
                    state
                        .locals
                        .get(&origin.anchor.source)
                        .map(|s| &s.value.kind),
                    Some(ValueKind::Owner)
                )
            {
                return Err(Failure::Authority);
            }
            return Ok(());
        }
        if lexical {
            for parent in value.scopes() {
                self.resolver.reserve_entries(1)?;
                state.containment.insert((scope, *parent));
            }
        }
        let mut relations = ScopeRelations::default();
        for (child, parent) in &state.containment {
            relations.declare(self.resolver, *child, *parent)?;
        }
        for parent in value.scopes() {
            if relations.contains(self.resolver, &scope, parent)? {
                return Ok(());
            }
        }
        Err(Failure::Authority)
    }
    fn install(&mut self, state: &mut State, binding: BindingId, value: Value) -> Result<()> {
        self.resolver.reserve_entries(2)?;
        let local = LocalValueReference::LexicalBinding(binding);
        if state
            .locals
            .insert(
                local,
                Slot {
                    value,
                    available: true,
                },
            )
            .is_some()
        {
            return Err(Failure::Inventory);
        }
        Ok(())
    }
    fn exit_binding(&mut self, state: &mut State, binding: BindingId) -> Result<()> {
        let local = LocalValueReference::LexicalBinding(binding);
        let slot = state.locals.remove(&local).ok_or(Failure::Authority)?;
        let owned = matches!(slot.value.kind, ValueKind::Owner);
        if slot.available {
            state.release(self.resolver, slot.value)?;
        }
        // A local owner cannot disappear while an escaping child retains it.
        if owned && state.guarded(local) {
            return Err(Failure::Authority);
        }
        Ok(())
    }
    fn scoped(
        &mut self,
        function: DeclarationReference,
        scoped_body: ScopedBody,
        mut state: State,
        value: Value,
        demand: Demand,
        depth: usize,
    ) -> Result<Vec<Outcome>> {
        let ScopedBody { binding, body } = scoped_body;
        self.install(&mut state, binding, value)?;
        self.resolver.reserve_entries(1)?;
        state.lexical.push(binding);
        let mut outcomes = self.eval(function, body, state, demand, depth + 1)?;
        for outcome in &mut outcomes {
            if outcome.state.lexical.pop() != Some(binding) {
                return Err(Failure::Authority);
            }
            self.exit_binding(&mut outcome.state, binding)?;
        }
        Ok(outcomes)
    }
    fn one(&mut self, state: State, value: Value) -> Result<Vec<Outcome>> {
        self.resolver.reserve_entries(1)?;
        Ok(vec![Outcome { state, value }])
    }
    fn require(&mut self, value: &Value, demand: Demand) -> Result<()> {
        self.resolver.step(0)?;
        let admitted = match demand {
            Demand::Any => matches!(
                value.kind,
                ValueKind::Data | ValueKind::Owner | ValueKind::View { owning: true, .. }
            ),
            Demand::Data => matches!(value.kind, ValueKind::Data),
            Demand::Owner => matches!(value.kind, ValueKind::Owner),
            Demand::Borrow => matches!(
                value.kind,
                ValueKind::Read { .. } | ValueKind::View { owning: false, .. }
            ),
            Demand::View => matches!(value.kind, ValueKind::View { owning: true, .. }),
            Demand::Loan => matches!(
                value.kind,
                ValueKind::Read { .. } | ValueKind::View { owning: true, .. }
            ),
        };
        if admitted {
            Ok(())
        } else {
            Err(Failure::Category)
        }
    }

    fn eval(
        &mut self,
        function: DeclarationReference,
        id: ExpressionId,
        mut state: State,
        demand: Demand,
        depth: usize,
    ) -> Result<Vec<Outcome>> {
        self.check_depth(depth)?;
        let operation = self.expression(function, id)?;
        use ExpressionOperation as E;
        match operation {
            E::Local { value } => {
                let value = self.local(function, &mut state, value, demand)?;
                self.one(state, value)
            }
            E::Unit {}
            | E::Bool { .. }
            | E::I64 { .. }
            | E::F64 { .. }
            | E::Text { .. }
            | E::StaticText { .. } => {
                let form = match operation {
                    E::Unit {} => TypeForm::Unit,
                    E::Bool { .. } => TypeForm::Bool,
                    E::I64 { .. } => TypeForm::I64,
                    E::F64 { .. } => TypeForm::F64,
                    E::Text { .. } => TypeForm::Text,
                    _ => TypeForm::StaticText,
                };
                let value = Value {
                    ty: self.resolver.intern(form)?,
                    kind: ValueKind::Data,
                };
                self.require(&value, demand)?;
                self.one(state, value)
            }
            E::Let { bindings, body } => {
                self.resolver.reserve_entries(bindings.len() * 2 + 1)?;
                let mut candidates = vec![state];
                let mut seen = BTreeSet::new();
                for binding in &bindings {
                    if !seen.insert(*binding) {
                        return Err(Failure::Inventory);
                    }
                    let kind = match self
                        .resolver
                        .source
                        .owners
                        .get(&OwnerKey::Binding(*binding))
                    {
                        Some(OwnerRecord::Binding(b)) => b.kind,
                        _ => return Err(Failure::Missing),
                    };
                    if !matches!(kind, BindingKind::Let | BindingKind::ReadView) {
                        return Err(Failure::Category);
                    }
                    let b = self.binding(function, *binding, kind, true)?;
                    let mut next = Vec::new();
                    for mut candidate in candidates {
                        self.resolver.reserve_entries(1)?;
                        candidate.lexical.push(*binding);
                        let results = self.eval(
                            function,
                            b.value.ok_or(Failure::Inventory)?,
                            candidate,
                            if kind == BindingKind::ReadView {
                                Demand::View
                            } else {
                                Demand::Any
                            },
                            depth + 1,
                        )?;
                        for mut result in results {
                            if kind == BindingKind::Let
                                && matches!(result.value.kind, ValueKind::View { .. })
                            {
                                return Err(Failure::Category);
                            }
                            if b.declared_type.is_some_and(|ty| ty != result.value.ty) {
                                return Err(Failure::Category);
                            }
                            if result.state.lexical.pop() != Some(*binding) {
                                return Err(Failure::Authority);
                            }
                            self.install(&mut result.state, *binding, result.value)?;
                            self.resolver.reserve_entries(1)?;
                            next.push(result.state);
                        }
                    }
                    candidates = next;
                }
                let mut outputs = Vec::new();
                for mut candidate in candidates {
                    self.resolver.reserve_entries(bindings.len())?;
                    candidate.lexical.extend(bindings.iter().copied());
                    let results = self.eval(function, body, candidate, demand, depth + 1)?;
                    for mut result in results {
                        for binding in bindings.iter().rev() {
                            if result.state.lexical.pop() != Some(*binding) {
                                return Err(Failure::Authority);
                            }
                            self.exit_binding(&mut result.state, *binding)?;
                        }
                        self.resolver.reserve_entries(1)?;
                        outputs.push(result);
                    }
                }
                Ok(outputs)
            }
            E::If {
                condition,
                when_true,
                when_false,
            } => {
                let conditions = self.eval(function, condition, state, Demand::Data, depth + 1)?;
                let mut outputs = Vec::new();
                for condition in conditions {
                    if !matches!(
                        self.resolver.object(condition.value.ty)?.form,
                        TypeForm::Bool
                    ) {
                        return Err(Failure::Category);
                    }
                    let other = condition.state.branch(self.resolver)?;
                    for (branch, candidate) in [(when_true, condition.state), (when_false, other)] {
                        let values = self.eval(function, branch, candidate, demand, depth + 1)?;
                        self.resolver.reserve_entries(values.len())?;
                        outputs.extend(values);
                    }
                }
                Ok(outputs)
            }
            E::Sequence { items } => {
                if items.is_empty() {
                    return Err(Failure::Inventory);
                }
                self.resolver.reserve_entries(1)?;
                let mut candidates = vec![state];
                for (position, item) in items.iter().enumerate() {
                    let last = position + 1 == items.len();
                    let mut outputs = Vec::new();
                    let mut next = Vec::new();
                    for candidate in candidates {
                        for mut result in self.eval(
                            function,
                            *item,
                            candidate,
                            if last { demand } else { Demand::Any },
                            depth + 1,
                        )? {
                            self.resolver.reserve_entries(1)?;
                            if last {
                                outputs.push(result);
                            } else {
                                result.state.release(self.resolver, result.value)?;
                                next.push(result.state);
                            }
                        }
                    }
                    if last {
                        return Ok(outputs);
                    }
                    candidates = next;
                }
                unreachable!()
            }
            E::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                self.private_layout(function, declaration)?;
                let source_value = self.local(function, &mut state, source, Demand::Borrow)?;
                self.permits_scope(function, &mut state, &source_value, read_scope)?;
                let ty = self.resolver.intern(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: type_arguments,
                    read_scope,
                })?;
                self.resolver
                    .read_view_category(ty, self.occurrence(function, OwnerKey::Expression(id)))?;
                let (_, inventory) = self.view_fields(function, ty)?;
                if fields.len() != inventory.len() {
                    return Err(Failure::Inventory);
                }
                self.resolver.reserve_entries(fields.len() + 1)?;
                let mut seen = BTreeSet::new();
                let mut candidates = vec![(state, source_value, BTreeMap::new())];
                for initializer in fields {
                    if !seen.insert(initializer.field) {
                        return Err(Failure::Inventory);
                    }
                    let (field, expected) = inventory
                        .iter()
                        .find(|(f, _)| f.id == initializer.field)
                        .ok_or(Failure::Authority)?;
                    let mut next = Vec::new();
                    for (candidate, source, stored) in candidates {
                        let values = self.eval(
                            function,
                            initializer.value,
                            candidate,
                            if field.kind == ReadViewFieldKind::Ordinary {
                                Demand::Data
                            } else {
                                Demand::Borrow
                            },
                            depth + 1,
                        )?;
                        for mut value in values {
                            if value.value.ty != *expected {
                                return Err(Failure::Category);
                            }
                            if matches!(field.kind, ReadViewFieldKind::ReadReference { .. }) {
                                if !value
                                    .value
                                    .origin()
                                    .ok_or(Failure::Category)?
                                    .derives_from(source.origin().ok_or(Failure::Category)?)
                                {
                                    return Err(Failure::Authority);
                                }
                                self.permits_scope(
                                    function,
                                    &mut value.state,
                                    &value.value,
                                    read_scope,
                                )?;
                                if let ValueKind::Read { scopes, .. } = &mut value.value.kind {
                                    self.resolver.reserve_entries(1)?;
                                    *scopes = vec![read_scope];
                                }
                            }
                            self.resolver.reserve_entries(
                                stored.len() + source.origin().map_or(0, |p| p.path.len()) + 2,
                            )?;
                            let mut stored = stored.clone();
                            stored.insert(initializer.field, value.value);
                            next.push((value.state, source.clone(), stored));
                        }
                    }
                    candidates = next;
                }
                let mut outputs = Vec::new();
                for (mut candidate, source, fields) in candidates {
                    let origin = source.origin().ok_or(Failure::Category)?.clone();
                    let value = self.cell_value(
                        &mut candidate,
                        Cell {
                            ty,
                            origin,
                            scope: read_scope,
                            fields,
                            custody: Some(source),
                            readers: 0,
                        },
                        true,
                    )?;
                    self.require(&value, demand)?;
                    self.resolver.reserve_entries(1)?;
                    outputs.push(Outcome {
                        state: candidate,
                        value,
                    });
                }
                Ok(outputs)
            }
            E::MoveReadView { value } | E::DropReadView { value } => {
                let slot = state.locals.get(&value).ok_or(Failure::Authority)?;
                if !slot.available
                    || !matches!(slot.value.kind, ValueKind::View { owning: true, .. })
                {
                    return Err(Failure::Authority);
                }
                self.resolver
                    .reserve_entries(1 + slot.value.origin().map_or(0, |p| p.path.len()))?;
                let moved = slot.value.clone();
                state
                    .locals
                    .get_mut(&value)
                    .ok_or(Failure::Authority)?
                    .available = false;
                let value = if matches!(operation, E::DropReadView { .. }) {
                    state.release(self.resolver, moved)?;
                    Value {
                        ty: self.resolver.intern(TypeForm::Unit)?,
                        kind: ValueKind::Data,
                    }
                } else {
                    moved
                };
                self.require(&value, demand)?;
                self.one(state, value)
            }
            E::ReborrowReadView { value, read_scope } => {
                let source = self.local(function, &mut state, value, Demand::Borrow)?;
                if !matches!(source.kind, ValueKind::View { .. }) {
                    return Err(Failure::Category);
                }
                self.permits_scope(function, &mut state, &source, read_scope)?;
                let value = self.reborrow(function, &mut state, source, read_scope, 0)?;
                self.require(&value, demand)?;
                self.one(state, value)
            }
            E::BorrowReadView {
                value,
                binding,
                body,
            } => {
                let value = self.local(function, &mut state, value, Demand::Borrow)?;
                if !matches!(value.kind, ValueKind::View { .. }) {
                    return Err(Failure::Category);
                }
                let b = self.binding(function, binding, BindingKind::ReadViewBorrow, false)?;
                if b.declared_type != Some(value.ty) {
                    return Err(Failure::Category);
                }
                self.scoped(
                    function,
                    ScopedBody { binding, body },
                    state,
                    value,
                    demand,
                    depth,
                )
            }
            E::ReadViewDataField { value, field } => {
                let parent = self.local(function, &mut state, value, Demand::Borrow)?;
                let (mut value, kind) = self.view_field(function, &mut state, &parent, field)?;
                if kind != ReadViewFieldKind::Ordinary {
                    return Err(Failure::Category);
                }
                value.kind = ValueKind::Data;
                state.release(self.resolver, parent)?;
                self.require(&value, demand)?;
                self.one(state, value)
            }
            E::BorrowReadViewField {
                value,
                field,
                binding,
                body,
            } => {
                let parent = self.local(function, &mut state, value, Demand::Borrow)?;
                let (value, kind) = self.view_field(function, &mut state, &parent, field)?;
                if !matches!(kind, ReadViewFieldKind::ReadReference { .. }) {
                    return Err(Failure::Category);
                }
                let b = self.binding(
                    function,
                    binding,
                    if matches!(value.kind, ValueKind::View { .. }) {
                        BindingKind::ReadViewBorrow
                    } else {
                        BindingKind::OwnedBorrow
                    },
                    false,
                )?;
                if b.declared_type != Some(value.ty) {
                    return Err(Failure::Category);
                }
                let mut outcomes = self.scoped(
                    function,
                    ScopedBody { binding, body },
                    state,
                    value,
                    demand,
                    depth,
                )?;
                for outcome in &mut outcomes {
                    outcome.state.release(self.resolver, parent.clone())?;
                }
                Ok(outcomes)
            }
            E::BorrowCall {
                call,
                binding,
                body,
            } => {
                let call_operation = self.expression(function, call)?;
                if !matches!(
                    call_operation,
                    E::Call { .. } | E::ImplementationCall { .. } | E::MethodCall { .. }
                ) {
                    return Err(Failure::Authority);
                }
                self.resolver.reserve_entries(1)?;
                state.lexical.push(binding);
                let calls = self.eval(function, call, state, Demand::Loan, depth + 1)?;
                let mut outcomes = Vec::new();
                for mut call in calls {
                    if call.state.lexical.pop() != Some(binding) {
                        return Err(Failure::Authority);
                    }
                    let kind = if matches!(call.value.kind, ValueKind::View { owning: true, .. }) {
                        BindingKind::ReadView
                    } else {
                        BindingKind::OwnedBorrow
                    };
                    let b = self.binding(function, binding, kind, false)?;
                    if b.declared_type != Some(call.value.ty) {
                        return Err(Failure::Category);
                    }
                    let results = self.scoped(
                        function,
                        ScopedBody { binding, body },
                        call.state,
                        call.value,
                        demand,
                        depth,
                    )?;
                    self.resolver.reserve_entries(results.len())?;
                    outcomes.extend(results);
                }
                Ok(outcomes)
            }
            E::Call { .. } | E::ImplementationCall { .. } | E::MethodCall { .. } => {
                self.call(function, id, operation, state, demand, depth, false)
            }
            E::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => self.owned_borrow(
                function,
                source,
                binding,
                body,
                state,
                demand,
                depth,
                product_type,
                ReadPathStep::OwnedProductField(field),
            ),
            E::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            } => self.owned_borrow(
                function,
                source,
                binding,
                body,
                state,
                demand,
                depth,
                record_type,
                ReadPathStep::OwnedField(field),
            ),
            E::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let indexes = self.eval(function, index, state, Demand::Data, depth + 1)?;
                let mut outputs = Vec::new();
                for index in indexes {
                    if !matches!(self.resolver.object(index.value.ty)?.form, TypeForm::I64) {
                        return Err(Failure::Category);
                    }
                    let results = self.owned_borrow(
                        function,
                        source,
                        binding,
                        body,
                        index.state,
                        demand,
                        depth,
                        sequence_type,
                        ReadPathStep::OwnedItem(sequence_type),
                    )?;
                    self.resolver.reserve_entries(results.len())?;
                    outputs.extend(results);
                }
                Ok(outputs)
            }
            E::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let TypeForm::OwnedChoice { cases } =
                    self.resolver.object(choice_type)?.form.clone()
                else {
                    return Err(Failure::Category);
                };
                if arms.len() != cases.len() {
                    return Err(Failure::Inventory);
                }
                let sources = self.eval(function, source, state, Demand::Borrow, depth + 1)?;
                let mut outputs = Vec::new();
                for source in sources {
                    if source.value.ty != choice_type {
                        return Err(Failure::Category);
                    }
                    self.resolver.reserve_entries(arms.len())?;
                    let mut seen = BTreeSet::new();
                    for arm in &arms {
                        if !seen.insert(arm.name.clone()) {
                            return Err(Failure::Inventory);
                        }
                        let mut branch = source.state.branch(self.resolver)?;
                        let child = self.project(
                            function,
                            &mut branch,
                            &source.value,
                            &ReadPathStep::OwnedCase(arm.name.clone()),
                        )?;
                        let b =
                            self.binding(function, arm.binding, BindingKind::OwnedBorrow, false)?;
                        if b.declared_type != Some(child.ty) {
                            return Err(Failure::Category);
                        }
                        let mut results = self.scoped(
                            function,
                            ScopedBody {
                                binding: arm.binding,
                                body: arm.body,
                            },
                            branch,
                            child,
                            demand,
                            depth,
                        )?;
                        for result in &mut results {
                            result.state.release(self.resolver, source.value.clone())?;
                        }
                        self.resolver.reserve_entries(results.len())?;
                        outputs.extend(results);
                    }
                }
                Ok(outputs)
            }
            E::Parallel { left, right } => {
                self.parallel(function, left, right, state, demand, depth)
            }
            other => self.ordinary(function, id, other, state, demand, depth),
        }
    }

    fn view_field(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        parent: &Value,
        field: ReadViewFieldReference,
    ) -> Result<(Value, ReadViewFieldKind)> {
        self.view_field_inner(function, state, parent, field, true)
    }
    fn view_field_inner(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        parent: &Value,
        field: ReadViewFieldReference,
        check_access: bool,
    ) -> Result<(Value, ReadViewFieldKind)> {
        if check_access {
            self.private_layout(function, field.declaration)?;
        }
        let ValueKind::View { cell, scope, .. } = parent.kind else {
            return Err(Failure::Category);
        };
        let (declaration, inventory) = self.view_fields(function, parent.ty)?;
        if declaration != field.declaration {
            return Err(Failure::Authority);
        }
        let (declared, ty) = inventory
            .iter()
            .find(|(f, _)| f.id == field.field)
            .ok_or(Failure::Authority)?;
        let backing = state.cells.get(&cell).ok_or(Failure::Authority)?;
        if backing.ty != parent.ty || backing.scope != scope {
            return Err(Failure::Authority);
        }
        let stored = backing.fields.get(&field.field).ok_or(Failure::Authority)?;
        if stored.ty != *ty {
            return Err(Failure::Category);
        }
        self.resolver.reserve_entries(
            1 + stored.origin().map_or(0, |p| p.path.len()) + stored.scopes().len(),
        )?;
        let stored = stored.clone();
        let mut value = match declared.kind {
            ReadViewFieldKind::Ordinary if matches!(stored.kind, ValueKind::Data) => stored,
            ReadViewFieldKind::ReadReference { .. } => {
                state.retain(self.resolver, &stored, Some(cell))?
            }
            _ => return Err(Failure::Category),
        };
        // Delegated field reads carry the descriptor's admitted scope. A
        // broader hidden source scope is custody, not a widening permission.
        if let ValueKind::Read { scopes, .. } = &mut value.kind {
            self.resolver.reserve_entries(1)?;
            *scopes = vec![scope];
        }
        Ok((value, declared.kind))
    }
    fn scoped_type(
        &mut self,
        ty: TypeObjectDigest,
        scope: ReadScopeArgument,
    ) -> Result<TypeObjectDigest> {
        let form = match self.resolver.object(ty)?.form.clone() {
            TypeForm::RegionCap {
                region,
                mode: RegionMode::Shared { .. },
            } => TypeForm::RegionCap {
                region,
                mode: RegionMode::Shared { scope },
            },
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                ..
            } => TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope: scope,
            },
            TypeForm::AssociatedReadViewMember {
                contract, member, ..
            } => TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope: scope,
            },
            TypeForm::AssociatedReadViewProjection {
                scope: binder,
                parameter,
                contract,
                member,
                ..
            } => TypeForm::AssociatedReadViewProjection {
                scope: binder,
                parameter,
                contract,
                member,
                read_scope: scope,
            },
            _ => return Err(Failure::Category),
        };
        self.resolver.intern(form)
    }
    fn reborrow(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        source: Value,
        scope: ReadScopeArgument,
        depth: usize,
    ) -> Result<Value> {
        self.resolver.step(depth)?;
        let ValueKind::View { cell, .. } = source.kind else {
            return Err(Failure::Category);
        };
        let ty = self.scoped_type(source.ty, scope)?;
        let origin = source.origin().ok_or(Failure::Authority)?.clone();
        let mut fields = BTreeMap::new();
        if matches!(
            self.resolver.object(ty)?.form,
            TypeForm::ReadViewApplied { .. }
        ) {
            let (_, inventory) = self.view_fields(function, ty)?;
            for (field, actual) in inventory {
                let stored = state
                    .cells
                    .get(&cell)
                    .and_then(|cell| cell.fields.get(&field.id))
                    .ok_or(Failure::Authority)?;
                self.resolver.reserve_entries(
                    1 + stored.origin().map_or(0, |p| p.path.len()) + stored.scopes().len(),
                )?;
                let stored = stored.clone();
                let mut value = if field.kind == ReadViewFieldKind::Ordinary {
                    stored
                } else {
                    state.retain(self.resolver, &stored, Some(cell))?
                };
                match &mut value.kind {
                    ValueKind::Read { scopes, .. } => {
                        self.resolver.reserve_entries(1)?;
                        *scopes = vec![scope];
                        value.ty = actual;
                    }
                    ValueKind::View { .. } => {
                        value = self.reborrow(function, state, value, scope, depth + 1)?;
                        if let ValueKind::View { owning, .. } = &mut value.kind {
                            *owning = false;
                        }
                    }
                    ValueKind::Data => {}
                    _ => return Err(Failure::Category),
                }
                if value.ty != actual {
                    return Err(Failure::Category);
                }
                self.resolver.reserve_entries(1)?;
                fields.insert(field.id, value);
            }
        }
        self.cell_value(
            state,
            Cell {
                ty,
                origin,
                scope,
                fields,
                custody: Some(source),
                readers: 0,
            },
            true,
        )
    }
    fn project(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        source: &Value,
        step: &ReadPathStep,
    ) -> Result<Value> {
        self.project_inner(function, state, source, step, true)
    }
    fn project_inner(
        &mut self,
        function: DeclarationReference,
        state: &mut State,
        source: &Value,
        step: &ReadPathStep,
        check_access: bool,
    ) -> Result<Value> {
        self.resolver.step(0)?;
        if let ReadPathStep::ViewReference { declaration, field } = step {
            return self
                .view_field_inner(
                    function,
                    state,
                    source,
                    ReadViewFieldReference {
                        declaration: *declaration,
                        field: *field,
                    },
                    check_access,
                )
                .and_then(|(value, kind)| {
                    if matches!(kind, ReadViewFieldKind::ReadReference { .. }) {
                        Ok(value)
                    } else {
                        Err(Failure::Authority)
                    }
                });
        }
        if !matches!(source.kind, ValueKind::Read { .. })
            && !matches!(step, ReadPathStep::MethodRead { .. })
        {
            return Err(Failure::Category);
        }
        let ty = match step {
            ReadPathStep::OwnedProductField(name) => {
                match self.resolver.object(source.ty)?.form.clone() {
                    TypeForm::OwnedProduct { fields } => fields
                        .iter()
                        .find(|field| &field.name == name)
                        .map(|field| field.ty)
                        .ok_or(Failure::Authority)?,
                    _ => return Err(Failure::Category),
                }
            }
            ReadPathStep::OwnedCase(name) => match self.resolver.object(source.ty)?.form.clone() {
                TypeForm::OwnedChoice { cases } => cases
                    .iter()
                    .find(|case| &case.name == name)
                    .map(|case| case.ty)
                    .ok_or(Failure::Authority)?,
                _ => return Err(Failure::Category),
            },
            ReadPathStep::OwnedItem(expected) => match self.resolver.object(source.ty)?.form {
                TypeForm::OwnedSequence { item } if source.ty == *expected => item,
                _ => return Err(Failure::Category),
            },
            ReadPathStep::OwnedField(field) => {
                let overlay = self.resolver.overlay();
                let oracle = Oracle(&overlay, Some(function));
                let (declaration, fields) =
                    oracle.record_fields(source.ty).ok_or(Failure::Unproved)?;
                if check_access {
                    self.private_layout(function, declaration)?;
                }
                fields
                    .iter()
                    .find(|(selected, _)| selected == field)
                    .map(|(_, ty)| *ty)
                    .ok_or(Failure::Authority)?
            }
            ReadPathStep::MethodRead {
                contract,
                method,
                ty,
            } if !check_access => {
                let c = self.resolver.contract(*contract)?.clone();
                let m = c
                    .methods
                    .iter()
                    .find(|m| m.id == *method)
                    .ok_or(Failure::Authority)?;
                self.resolver.read_view_method_signature(*contract, m)?;
                if !matches!(m.result.relation, ResultRelation::ReadFrom { .. })
                    || !matches!(m.effect, FunctionEffect::Pure)
                {
                    return Err(Failure::Authority);
                }
                *ty
            }
            ReadPathStep::MethodRead { .. } => return Err(Failure::Authority),
            ReadPathStep::ViewReference { .. } => unreachable!(),
        };
        if self.resolver.read_view_category(
            ty,
            self.occurrence(function, OwnerKey::Declaration(function.declaration)),
        )? != ReadCategory::Owned
        {
            return Err(Failure::Category);
        }
        let mut value = state.retain(self.resolver, source, None)?;
        let origin = source
            .origin()
            .ok_or(Failure::Authority)?
            .extend(self.resolver, step.clone())?;
        value.ty = ty;
        match &mut value.kind {
            ValueKind::Read { origin: stored, .. } => *stored = origin,
            ValueKind::View { lease, .. } if matches!(step, ReadPathStep::MethodRead { .. }) => {
                let lease = *lease;
                self.resolver.reserve_entries(source.scopes().len())?;
                value.kind = ValueKind::Read {
                    origin,
                    scopes: source.scopes().to_vec(),
                    lease,
                };
            }
            _ => return Err(Failure::Category),
        }
        Ok(value)
    }
    #[allow(clippy::too_many_arguments)]
    fn owned_borrow(
        &mut self,
        function: DeclarationReference,
        source: ExpressionId,
        binding: BindingId,
        body: ExpressionId,
        state: State,
        demand: Demand,
        depth: usize,
        expected: TypeObjectDigest,
        step: ReadPathStep,
    ) -> Result<Vec<Outcome>> {
        let sources = self.eval(function, source, state, Demand::Borrow, depth + 1)?;
        let mut outputs = Vec::new();
        for mut source in sources {
            if source.value.ty != expected {
                return Err(Failure::Category);
            }
            let child = self.project(function, &mut source.state, &source.value, &step)?;
            let b = self.binding(function, binding, BindingKind::OwnedBorrow, false)?;
            if b.declared_type != Some(child.ty) {
                return Err(Failure::Category);
            }
            let mut results = self.scoped(
                function,
                ScopedBody { binding, body },
                source.state,
                child,
                demand,
                depth,
            )?;
            for result in &mut results {
                result.state.release(self.resolver, source.value.clone())?;
            }
            self.resolver.reserve_entries(results.len())?;
            outputs.extend(results);
        }
        Ok(outputs)
    }
    fn apply_signature(
        &mut self,
        signature: &view_signatures::Signature,
        environment: &ReadViewEnvironment,
    ) -> Result<view_signatures::Signature> {
        self.resolver
            .reserve_entries(signature.parameters.len() * 3 + signature.source_scopes.len() + 1)?;
        let mut parameters = Vec::new();
        let mut categories = Vec::new();
        let mut source_scopes = Vec::new();
        for (ty, mode) in &signature.parameters {
            let ty = self.resolver.substitute_view(*ty, environment, 0)?;
            categories.push(
                self.resolver
                    .read_view_category(ty, environment.types.caller)?,
            );
            parameters.push((ty, *mode));
        }
        for (source, scope) in &signature.source_scopes {
            source_scopes.push((
                *source,
                self.resolver.substitute_scope(
                    scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?,
            ));
        }
        let relation = match signature.result.relation {
            ResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source,
                scope: self.resolver.substitute_scope(
                    &scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?,
            },
            relation => relation,
        };
        let ty = self
            .resolver
            .substitute_view(signature.result.ty, environment, 0)?;
        let result_category = self
            .resolver
            .read_view_category(ty, environment.types.caller)?;
        Ok(view_signatures::Signature {
            binder: environment.caller_binder,
            parameters,
            categories,
            source_scopes,
            result: CallableResult { ty, relation },
            result_category,
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn named_plan(
        &mut self,
        caller: DeclarationReference,
        consumer: OwnerKey,
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        read_scope_arguments: Vec<ReadScopeArgument>,
        implementations: Vec<ImplementationOperand>,
        arguments: Vec<ExpressionId>,
        requirement_arguments: Vec<RequirementOperand>,
        effect_arguments: Vec<EffectRow>,
    ) -> Result<CallPlan> {
        self.resolver.step(0)?;
        let oracle = Oracle(self.resolver.source, Some(caller));
        let Some(f) = oracle.function(function) else {
            if !read_scope_arguments.is_empty()
                || !implementations.is_empty()
                || !requirement_arguments.is_empty()
                || !effect_arguments.is_empty()
            {
                return Err(Failure::Category);
            }
            if !oracle.application(function, &type_arguments) {
                return Err(Failure::Unproved);
            }
            let applied = oracle
                .applied_signature(function, &type_arguments)
                .ok_or(Failure::Unproved)?;
            if !oracle.legal_signature(&applied) {
                return Err(Failure::Unproved);
            }
            self.resolver
                .reserve_entries(applied.0.len() * 2 + type_arguments.len())?;
            let occurrence = self.occurrence(caller, consumer);
            let mut parameters = Vec::new();
            let mut categories = Vec::new();
            for p in applied.0 {
                self.resolver.admit_roots(&[p.ty], occurrence, false)?;
                categories.push(self.resolver.read_view_category(p.ty, occurrence)?);
                parameters.push((p.ty, p.use_mode));
            }
            self.resolver.admit_roots(&[applied.1], occurrence, false)?;
            let category = self.resolver.read_view_category(applied.1, occurrence)?;
            if category == ReadCategory::ReadView || categories.contains(&ReadCategory::ReadView) {
                return Err(Failure::Category);
            }
            let binder = ReadScopeBinder::Declaration(caller);
            return Ok(CallPlan {
                signature: view_signatures::Signature {
                    binder,
                    parameters,
                    categories,
                    source_scopes: Vec::new(),
                    result: CallableResult {
                        ty: applied.1,
                        relation: ResultRelation::Value,
                    },
                    result_category: category,
                },
                environment: ReadViewEnvironment {
                    types: Environment {
                        ordinary: BTreeMap::new(),
                        source: Context::Forbidden,
                        caller: occurrence,
                        contract_operand: None,
                        witnesses: BTreeMap::new(),
                    },
                    source_binder: binder,
                    caller_binder: binder,
                    scopes: BTreeMap::new(),
                },
                function: None,
                method: None,
                effect: if applied.2 {
                    FunctionEffect::Pure
                } else {
                    return Err(Failure::Unproved);
                },
                arguments,
            });
        };
        if f.type_parameters.len() != type_arguments.len()
            || f.implementation_parameters.len() != implementations.len()
            || f.read_scope_parameters.len() != read_scope_arguments.len()
        {
            return Err(Failure::Arity);
        }
        // Effect-polymorphic row application remains the existing separate raw
        // oracle's responsibility; it must not be guessed from an empty row.
        if !f.requirement_parameters.is_empty()
            || !f.effect_parameters.is_empty()
            || !requirement_arguments.is_empty()
            || !effect_arguments.is_empty()
        {
            return Err(Failure::Unproved);
        }
        self.resolver.reserve_entries(
            type_arguments.len()
                + implementations.len()
                + read_scope_arguments.len()
                + arguments.len(),
        )?;
        let binder = ReadScopeBinder::Declaration(function);
        let environment = ReadViewEnvironment {
            types: Environment {
                ordinary: f
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(type_arguments)
                    .collect(),
                source: Context::Lexical(function),
                caller: self.occurrence(caller, consumer),
                contract_operand: None,
                witnesses: f
                    .implementation_parameters
                    .iter()
                    .map(|p| p.id)
                    .zip(implementations)
                    .collect(),
            },
            source_binder: binder,
            caller_binder: ReadScopeBinder::Declaration(caller),
            scopes: f
                .read_scope_parameters
                .iter()
                .map(|p| ReadScopeReference {
                    binder,
                    parameter: p.id,
                })
                .zip(read_scope_arguments)
                .collect(),
        };
        self.resolver.admit_application(&environment.types)?;
        let template = self.resolver.read_view_function_signature(function)?;
        let signature = self.apply_signature(&template, &environment)?;
        Ok(CallPlan {
            signature,
            environment,
            function: Some(function),
            method: None,
            effect: f.effect,
            arguments,
        })
    }
    fn apply_operand(
        &mut self,
        operand: &ImplementationOperand,
        environment: &Environment,
        depth: usize,
    ) -> Result<ImplementationOperand> {
        self.resolver.step(depth)?;
        match operand {
            ImplementationOperand::Parameter { scope, parameter }
                if environment.source == Context::Lexical(*scope) =>
            {
                environment
                    .witnesses
                    .get(parameter)
                    .cloned()
                    .ok_or(Failure::Authority)
            }
            ImplementationOperand::Parameter { .. } => Err(Failure::Authority),
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                self.resolver
                    .reserve_entries(type_arguments.len() + implementations.len() + 1)?;
                let mut types = Vec::new();
                let mut witnesses = Vec::new();
                for ty in type_arguments {
                    types.push(self.resolver.substitute(*ty, environment, depth + 1)?);
                }
                for operand in implementations {
                    witnesses.push(self.apply_operand(operand, environment, depth + 1)?);
                }
                Ok(ImplementationOperand::Concrete {
                    implementation: *implementation,
                    type_arguments: types,
                    implementations: witnesses,
                })
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn method_plan(
        &mut self,
        caller: DeclarationReference,
        id: ExpressionId,
        witness: ImplementationOperand,
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
        scope_actuals: Vec<ReadScopeArgument>,
        arguments: Vec<ExpressionId>,
    ) -> Result<CallPlan> {
        let occurrence = self.occurrence(caller, OwnerKey::Expression(id));
        let (actual, self_type, actuals) = self.resolver.admit_operand(&witness, occurrence, 0)?;
        if actual != contract {
            return Err(Failure::Authority);
        }
        let c = self.resolver.contract(contract)?.clone();
        let mut selected = c.methods.iter().filter(|m| m.id == method);
        let m = selected.next().ok_or(Failure::Authority)?;
        if selected.next().is_some() || m.read_scope_parameters.len() != scope_actuals.len() {
            return Err(Failure::Inventory);
        }
        self.resolver
            .reserve_entries(c.type_parameters.len() + scope_actuals.len() + 1)?;
        let binder = ReadScopeBinder::ContractMethod { contract, method };
        let environment = ReadViewEnvironment {
            types: Environment {
                ordinary: std::iter::once((c.self_parameter, self_type))
                    .chain(c.type_parameters.iter().copied().zip(actuals))
                    .collect(),
                source: Context::Contract(contract),
                caller: occurrence,
                contract_operand: Some(witness.clone()),
                witnesses: BTreeMap::new(),
            },
            source_binder: binder,
            caller_binder: ReadScopeBinder::Declaration(caller),
            scopes: m
                .read_scope_parameters
                .iter()
                .map(|p| ReadScopeReference {
                    binder,
                    parameter: p.id,
                })
                .zip(scope_actuals)
                .collect(),
        };
        self.resolver.admit_application(&environment.types)?;
        let template = self.resolver.read_view_method_signature(contract, m)?;
        let signature = self.apply_signature(&template, &environment)?;
        if let ImplementationOperand::Concrete {
            implementation,
            type_arguments,
            implementations,
        } = witness
        {
            let i = self.resolver.implementation(implementation)?.clone();
            if i.contract != contract || i.methods.len() != c.methods.len() {
                return Err(Failure::Inventory);
            }
            self.resolver.reserve_entries(i.methods.len())?;
            let mut seen = BTreeSet::new();
            for mapping in &i.methods {
                if !seen.insert(mapping.method) {
                    return Err(Failure::Inventory);
                }
                let declared = c
                    .methods
                    .iter()
                    .find(|m| m.id == mapping.method)
                    .ok_or(Failure::Authority)?;
                self.resolver
                    .admit_view_method_mapping(implementation, declared, mapping)?;
            }
            let mapping = i
                .methods
                .iter()
                .find(|m| m.method == method)
                .ok_or(Failure::Inventory)?;
            self.resolver
                .reserve_entries(i.type_parameters.len() + i.implementation_parameters.len())?;
            let implementation_environment = Environment {
                ordinary: i
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(type_arguments)
                    .collect(),
                source: Context::Lexical(implementation),
                caller: occurrence,
                contract_operand: None,
                witnesses: i
                    .implementation_parameters
                    .iter()
                    .map(|p| p.id)
                    .zip(implementations)
                    .collect(),
            };
            self.resolver
                .admit_application(&implementation_environment)?;
            let mut types = Vec::new();
            let mut witnesses = Vec::new();
            let mut scopes = Vec::new();
            self.resolver.reserve_entries(
                mapping.type_arguments.len()
                    + mapping.implementations.len()
                    + mapping.read_scope_arguments.len(),
            )?;
            for ty in &mapping.type_arguments {
                types.push(
                    self.resolver
                        .substitute(*ty, &implementation_environment, 0)?,
                );
            }
            for witness in &mapping.implementations {
                witnesses.push(self.apply_operand(witness, &implementation_environment, 0)?);
            }
            for scope in &mapping.read_scope_arguments {
                scopes.push(self.resolver.substitute_scope(
                    scope,
                    &environment,
                    &binder,
                    &environment.caller_binder,
                )?);
            }
            let plan = self.named_plan(
                caller,
                OwnerKey::Expression(id),
                mapping.function,
                types,
                scopes,
                witnesses,
                arguments,
                Vec::new(),
                Vec::new(),
            )?;
            if plan.signature.parameters != signature.parameters
                || plan.signature.result != signature.result
                || plan.signature.source_scopes != signature.source_scopes
                || plan.effect != m.effect
            {
                return Err(Failure::Authority);
            }
            return Ok(plan);
        }
        Ok(CallPlan {
            signature,
            environment,
            function: None,
            method: Some((contract, method)),
            effect: m.effect.clone(),
            arguments,
        })
    }
    fn call_plan(
        &mut self,
        caller: DeclarationReference,
        id: ExpressionId,
        operation: ExpressionOperation,
    ) -> Result<CallPlan> {
        match operation {
            ExpressionOperation::Call {
                function,
                type_arguments,
                read_scope_arguments,
                arguments,
                requirement_arguments,
                effect_arguments,
            } => self.named_plan(
                caller,
                OwnerKey::Expression(id),
                function,
                type_arguments,
                read_scope_arguments,
                Vec::new(),
                arguments,
                requirement_arguments,
                effect_arguments,
            ),
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                read_scope_arguments,
                implementations,
                arguments,
                requirement_arguments,
                effect_arguments,
            } => self.named_plan(
                caller,
                OwnerKey::Expression(id),
                function,
                type_arguments,
                read_scope_arguments,
                implementations,
                arguments,
                requirement_arguments,
                effect_arguments,
            ),
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                read_scope_arguments,
                arguments,
            } => self.method_plan(
                caller,
                id,
                witness,
                contract,
                method,
                read_scope_arguments,
                arguments,
            ),
            _ => Err(Failure::Authority),
        }
    }

    fn prepare_arguments(
        &mut self,
        caller: DeclarationReference,
        plan: &CallPlan,
        state: State,
        depth: usize,
        parallel: bool,
    ) -> Result<Vec<(State, Vec<Value>)>> {
        if plan.signature.parameters.len() != plan.arguments.len() {
            return Err(Failure::Arity);
        }
        self.resolver.reserve_entries(1)?;
        let mut candidates = vec![(state, Vec::new())];
        for (position, argument) in plan.arguments.iter().enumerate() {
            let (expected, mode) = plan.signature.parameters[position];
            let mut next = Vec::new();
            for (state, prepared) in candidates {
                let values = self.eval(
                    caller,
                    *argument,
                    state,
                    match mode {
                        ParameterUse::Borrow => Demand::Borrow,
                        ParameterUse::Consume => Demand::Owner,
                        ParameterUse::Unrestricted => Demand::Data,
                    },
                    depth + 1,
                )?;
                for mut value in values {
                    if value.value.ty != expected {
                        return Err(Failure::Category);
                    }
                    for (source, scope) in &plan.signature.source_scopes {
                        if *source == position {
                            self.permits_scope(caller, &mut value.state, &value.value, *scope)?;
                        }
                    }
                    if parallel && mode == ParameterUse::Borrow {
                        let occurrence = plan.environment.types.caller;
                        match plan.signature.categories[position] {
                            ReadCategory::ReadView => {
                                if !self.resolver.read_shareable_view(expected, occurrence)? {
                                    return Err(Failure::Category);
                                }
                            }
                            ReadCategory::Owned => {
                                let overlay = self.resolver.overlay();
                                match Oracle(&overlay, Some(caller)).read_shape(
                                    expected,
                                    &BTreeMap::new(),
                                    0,
                                    &mut self.resolver.remaining,
                                ) {
                                    Some(true) => {}
                                    Some(false) => return Err(Failure::Category),
                                    None => return Err(Failure::Unproved),
                                }
                            }
                            _ => return Err(Failure::Category),
                        }
                    }
                    self.resolver.reserve_entries(prepared.len() * 2 + 1)?;
                    let mut prepared = prepared.clone();
                    prepared.push(value.value);
                    next.push((value.state, prepared));
                }
            }
            candidates = next;
        }
        Ok(candidates)
    }
    fn follow(
        &mut self,
        caller: DeclarationReference,
        state: &mut State,
        source: &Value,
        origin: &ReadProvenance,
        selected: SourceAnchor,
        environment: &ReadViewEnvironment,
    ) -> Result<Value> {
        if origin.anchor != selected {
            return Err(Failure::Authority);
        }
        let mut value = state.retain(self.resolver, source, None)?;
        for step in &origin.path {
            let step = match step {
                ReadPathStep::OwnedItem(ty) => {
                    ReadPathStep::OwnedItem(self.resolver.substitute_view(*ty, environment, 0)?)
                }
                ReadPathStep::MethodRead {
                    contract,
                    method,
                    ty,
                } => ReadPathStep::MethodRead {
                    contract: *contract,
                    method: *method,
                    ty: self.resolver.substitute_view(*ty, environment, 0)?,
                },
                step => step.clone(),
            };
            // This is replay of an already proved callee path. It grants the
            // returned read, not private field-operation authority to caller.
            let child = self.project_inner(caller, state, &value, &step, false)?;
            state.release(self.resolver, value)?;
            value = child;
        }
        Ok(value)
    }
    #[allow(clippy::too_many_arguments)]
    fn materialize(
        &mut self,
        caller: DeclarationReference,
        state: &mut State,
        summary: &Summary,
        source: Option<&Value>,
        selected: Option<SourceAnchor>,
        environment: &ReadViewEnvironment,
        depth: usize,
    ) -> Result<Value> {
        self.resolver.step(depth)?;
        let ty = self.resolver.substitute_view(summary.ty, environment, 0)?;
        match &summary.kind {
            SummaryKind::Data => Ok(Value {
                ty,
                kind: ValueKind::Data,
            }),
            SummaryKind::Owner => Ok(Value {
                ty,
                kind: ValueKind::Owner,
            }),
            SummaryKind::Read { origin } => {
                let mut value = self.follow(
                    caller,
                    state,
                    source.ok_or(Failure::Authority)?,
                    origin,
                    selected.ok_or(Failure::Authority)?,
                    environment,
                )?;
                if value.ty != ty {
                    let scope = self
                        .resolver
                        .read_view_scope(ty)?
                        .ok_or(Failure::Category)?;
                    self.permits_scope(caller, state, &value, scope)?;
                    value = self.reborrow(caller, state, value, scope, depth + 1)?;
                    if let ValueKind::View { owning, .. } = &mut value.kind {
                        *owning = false;
                    }
                    if value.ty != ty {
                        return Err(Failure::Category);
                    }
                }
                Ok(value)
            }
            SummaryKind::View {
                origin,
                scope,
                fields,
            } => {
                let custody = self.follow(
                    caller,
                    state,
                    source.ok_or(Failure::Authority)?,
                    origin,
                    selected.clone().ok_or(Failure::Authority)?,
                    environment,
                )?;
                let scope = self.resolver.substitute_scope(
                    scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?;
                self.permits_scope(caller, state, &custody, scope)?;
                self.resolver.reserve_entries(fields.len())?;
                let mut actuals = BTreeMap::new();
                for (id, field) in fields {
                    actuals.insert(
                        *id,
                        self.materialize(
                            caller,
                            state,
                            field,
                            source,
                            selected.clone(),
                            environment,
                            depth + 1,
                        )?,
                    );
                }
                let origin = custody.origin().ok_or(Failure::Authority)?.clone();
                self.cell_value(
                    state,
                    Cell {
                        ty,
                        origin,
                        scope,
                        fields: actuals,
                        custody: Some(custody),
                        readers: 0,
                    },
                    true,
                )
            }
        }
    }
    fn abstract_result(
        &mut self,
        caller: DeclarationReference,
        state: &mut State,
        plan: &CallPlan,
        arguments: &[Value],
    ) -> Result<Value> {
        let ty = plan.signature.result.ty;
        match plan.signature.result.relation {
            ResultRelation::Value => Ok(Value {
                ty,
                kind: if plan.signature.result_category == ReadCategory::Owned {
                    ValueKind::Owner
                } else {
                    ValueKind::Data
                },
            }),
            ResultRelation::ReadFrom { source } => {
                let (contract, method) = plan.method.ok_or(Failure::Authority)?;
                let input = arguments.get(source).ok_or(Failure::Authority)?;
                let origin = input.origin().ok_or(Failure::Category)?.extend(
                    self.resolver,
                    ReadPathStep::MethodRead {
                        contract,
                        method,
                        ty,
                    },
                )?;
                let retained = state.retain(self.resolver, input, None)?;
                let lease = retained.lease().ok_or(Failure::Authority)?;
                self.resolver.reserve_entries(input.scopes().len())?;
                Ok(Value {
                    ty,
                    kind: ValueKind::Read {
                        origin,
                        scopes: input.scopes().to_vec(),
                        lease,
                    },
                })
            }
            ResultRelation::ViewFrom { source, scope } => {
                let input = arguments.get(source).ok_or(Failure::Authority)?;
                self.permits_scope(caller, state, input, scope)?;
                let custody = state.retain(self.resolver, input, None)?;
                let origin = input.origin().ok_or(Failure::Authority)?.clone();
                let mut value = self.abstract_view(state, ty, origin, scope, caller, 0)?;
                let ValueKind::View { cell, owning, .. } = &mut value.kind else {
                    return Err(Failure::Category);
                };
                *owning = true;
                state.cells.get_mut(cell).ok_or(Failure::Authority)?.custody = Some(custody);
                Ok(value)
            }
        }
    }
    fn complete_call(
        &mut self,
        caller: DeclarationReference,
        plan: &CallPlan,
        state: State,
        arguments: Vec<Value>,
        demand: Demand,
    ) -> Result<Vec<Outcome>> {
        let summaries = if let Some(function) = plan.function {
            Some(self.function(function)?)
        } else {
            None
        };
        let mut outputs = Vec::new();
        if let Some(summaries) = summaries {
            let selected = plan.signature.result.relation.source().copied();
            let selected_anchor = if let Some(index) = selected {
                let f = Oracle(self.resolver.source, plan.function)
                    .function(plan.function.ok_or(Failure::Authority)?)
                    .ok_or(Failure::Authority)?;
                Some(SourceAnchor {
                    callable: plan.function.ok_or(Failure::Authority)?,
                    source: LocalValueReference::FunctionParameter(
                        *f.parameters.get(index).ok_or(Failure::Authority)?,
                    ),
                })
            } else {
                None
            };
            for summary in &summaries {
                let mut candidate = state.branch(self.resolver)?;
                let value = self.materialize(
                    caller,
                    &mut candidate,
                    summary,
                    selected.and_then(|index| arguments.get(index)),
                    selected_anchor.clone(),
                    &plan.environment,
                    0,
                )?;
                if value.ty != plan.signature.result.ty {
                    return Err(Failure::Category);
                }
                self.require(&value, demand)?;
                for argument in arguments.iter().rev() {
                    candidate.release(self.resolver, argument.clone())?;
                }
                self.resolver.reserve_entries(1)?;
                outputs.push(Outcome {
                    state: candidate,
                    value,
                });
            }
        } else {
            let mut candidate = state;
            let value = self.abstract_result(caller, &mut candidate, plan, &arguments)?;
            self.require(&value, demand)?;
            for argument in arguments.into_iter().rev() {
                candidate.release(self.resolver, argument)?;
            }
            self.resolver.reserve_entries(1)?;
            outputs.push(Outcome {
                state: candidate,
                value,
            });
        }
        Ok(outputs)
    }
    #[allow(clippy::too_many_arguments)]
    fn call(
        &mut self,
        caller: DeclarationReference,
        id: ExpressionId,
        operation: ExpressionOperation,
        state: State,
        demand: Demand,
        depth: usize,
        parallel: bool,
    ) -> Result<Vec<Outcome>> {
        let plan = self.call_plan(caller, id, operation)?;
        let caller_effect = Oracle(self.resolver.source, Some(caller))
            .function(caller)
            .ok_or(Failure::Authority)?
            .effect;
        if matches!(caller_effect, FunctionEffect::Pure)
            && !matches!(plan.effect, FunctionEffect::Pure)
        {
            return Err(Failure::Category);
        }
        if let FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = &plan.effect
        {
            let row = caller_effect.row();
            if !requirements
                .iter()
                .all(|requirement| row.requirements.contains(requirement))
                || !effect_parameters
                    .iter()
                    .all(|parameter| row.parameters.contains(parameter))
            {
                return Err(Failure::Authority);
            }
        }
        if !matches!(plan.signature.result.relation, ResultRelation::Value)
            && !matches!(demand, Demand::Loan | Demand::View | Demand::Borrow)
        {
            return Err(Failure::Category);
        }
        let candidates = self.prepare_arguments(caller, &plan, state, depth, parallel)?;
        let mut outputs = Vec::new();
        for (state, arguments) in candidates {
            let results = self.complete_call(caller, &plan, state, arguments, demand)?;
            self.resolver.reserve_entries(results.len())?;
            outputs.extend(results);
        }
        Ok(outputs)
    }
    fn parallel(
        &mut self,
        caller: DeclarationReference,
        left: ExpressionId,
        right: ExpressionId,
        state: State,
        demand: Demand,
        depth: usize,
    ) -> Result<Vec<Outcome>> {
        let caller_effect = Oracle(self.resolver.source, Some(caller))
            .function(caller)
            .ok_or(Failure::Authority)?
            .effect;
        if matches!(caller_effect, FunctionEffect::Pure) {
            return Err(Failure::Category);
        }
        let left_op = self.expression(caller, left)?;
        let right_op = self.expression(caller, right)?;
        if !matches!(
            left_op,
            ExpressionOperation::Call { .. } | ExpressionOperation::ImplementationCall { .. }
        ) || !matches!(
            right_op,
            ExpressionOperation::Call { .. } | ExpressionOperation::ImplementationCall { .. }
        ) {
            return Err(Failure::Authority);
        }
        let left = self.call_plan(caller, left, left_op)?;
        let right = self.call_plan(caller, right, right_op)?;
        for plan in [&left, &right] {
            if !matches!(&plan.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
                || !matches!(plan.signature.result.relation, ResultRelation::Value)
            {
                return Err(Failure::Category);
            }
        }
        let mut outputs = Vec::new();
        for (state, left_args) in self.prepare_arguments(caller, &left, state, depth, true)? {
            for (state, right_args) in self.prepare_arguments(caller, &right, state, depth, true)? {
                // Both footprints remain live until both children have joined.
                for left_result in
                    self.complete_call(caller, &left, state, left_args.clone(), Demand::Any)?
                {
                    for mut right_result in self.complete_call(
                        caller,
                        &right,
                        left_result.state,
                        right_args.clone(),
                        Demand::Any,
                    )? {
                        let fields = vec![
                            StructuralTypeField {
                                name: Name::new("left").map_err(|_| Failure::Inventory)?,
                                ty: left_result.value.ty,
                            },
                            StructuralTypeField {
                                name: Name::new("right").map_err(|_| Failure::Inventory)?,
                                ty: right_result.value.ty,
                            },
                        ];
                        self.resolver.reserve_entries(2)?;
                        let owned = matches!(left_result.value.kind, ValueKind::Owner)
                            || matches!(right_result.value.kind, ValueKind::Owner);
                        let ty = self.resolver.intern(if owned {
                            TypeForm::OwnedProduct { fields }
                        } else {
                            TypeForm::StructuralRecord { fields }
                        })?;
                        right_result
                            .state
                            .release(self.resolver, left_result.value.clone())?;
                        right_result
                            .state
                            .release(self.resolver, right_result.value)?;
                        let value = Value {
                            ty,
                            kind: if owned {
                                ValueKind::Owner
                            } else {
                                ValueKind::Data
                            },
                        };
                        self.require(&value, demand)?;
                        self.resolver.reserve_entries(1)?;
                        outputs.push(Outcome {
                            state: right_result.state,
                            value,
                        });
                    }
                }
            }
        }
        Ok(outputs)
    }

    /// The old raw Owned oracle is used only for a complete subtree which has
    /// no view values/operations or scoped view calls. Its boolean ambiguity
    /// is always Unproved, never a semantic source-lineage decision.
    fn ordinary(
        &mut self,
        function: DeclarationReference,
        id: ExpressionId,
        operation: ExpressionOperation,
        mut state: State,
        demand: Demand,
        depth: usize,
    ) -> Result<Vec<Outcome>> {
        if matches!(demand, Demand::Borrow | Demand::View | Demand::Loan) {
            return Err(Failure::Category);
        }
        self.no_view_subtree(function, id, &state, 0)?;
        let ty = self.ordinary_type(function, id, &state, depth)?;
        let occurrence = self.occurrence(function, OwnerKey::Expression(id));
        self.resolver.admit_roots(&[ty], occurrence, false)?;
        let category = self.resolver.read_view_category(ty, occurrence)?;
        if category == ReadCategory::ReadView {
            return Err(Failure::Category);
        }
        self.resolver
            .reserve_entries(state.locals.len() * 3 + state.leases.len() * 2)?;
        let mut rights = Rights::default();
        for (local, slot) in &state.locals {
            match slot.value.kind {
                ValueKind::Owner => {
                    rights.memory.insert(*local);
                    if slot.available {
                        rights.owned.insert(*local);
                    }
                }
                ValueKind::Read { .. } if slot.available => {
                    rights.memory.insert(*local);
                    rights.borrowed.insert(*local);
                }
                _ => {}
            }
            if state.guarded(*local) {
                rights.loans.insert(*local, 1);
            }
        }
        let overlay = self.resolver.overlay();
        let owner = category == ReadCategory::Owned;
        if Oracle(&overlay, Some(function))
            .flow(
                id,
                &mut rights,
                if owner {
                    super::super::Demand::Owner
                } else {
                    super::super::Demand::Data
                },
                depth,
            )
            .is_none()
        {
            return Err(Failure::Unproved);
        }
        for (local, slot) in &mut state.locals {
            if matches!(slot.value.kind, ValueKind::Owner) {
                slot.available = rights.owned.contains(local);
            }
        }
        let value = Value {
            ty,
            kind: if owner {
                ValueKind::Owner
            } else {
                ValueKind::Data
            },
        };
        self.require(&value, demand)?;
        let _ = operation;
        self.one(state, value)
    }
    fn no_view_subtree(
        &mut self,
        function: DeclarationReference,
        id: ExpressionId,
        state: &State,
        depth: usize,
    ) -> Result<()> {
        self.check_depth(depth)?;
        let operation = self.expression(function, id)?;
        if operation.is_read_view_operation() {
            return Err(Failure::Unproved);
        }
        if let ExpressionOperation::Local { value } = operation
            && state
                .locals
                .get(&value)
                .is_some_and(|s| matches!(s.value.kind, ValueKind::View { .. }))
        {
            return Err(Failure::Category);
        }
        if matches!(
            operation,
            ExpressionOperation::Call { .. }
                | ExpressionOperation::ImplementationCall { .. }
                | ExpressionOperation::MethodCall { .. }
        ) {
            let plan = self.call_plan(function, id, operation.clone())?;
            if !plan.environment.scopes.is_empty()
                || !matches!(plan.signature.result.relation, ResultRelation::Value)
                || plan.signature.categories.contains(&ReadCategory::ReadView)
            {
                return Err(Failure::Unproved);
            }
        }
        if let ExpressionOperation::FunctionValue {
            function: callee,
            read_scope_arguments,
            ..
        } = &operation
        {
            let f = Oracle(self.resolver.source, Some(*callee))
                .function(*callee)
                .ok_or(Failure::Unproved)?;
            if !read_scope_arguments.is_empty()
                || !f.read_scope_parameters.is_empty()
                || !matches!(f.result.relation, ResultRelation::Value)
            {
                return Err(Failure::Category);
            }
            let sig = self.resolver.read_view_function_signature(*callee)?;
            if sig.categories.contains(&ReadCategory::ReadView)
                || sig.result_category == ReadCategory::ReadView
            {
                return Err(Failure::Category);
            }
        }
        let record = match self.resolver.source.owners.get(&OwnerKey::Expression(id)) {
            Some(OwnerRecord::Expression(e)) => e,
            _ => return Err(Failure::Missing),
        };
        let children = record.children();
        self.resolver.reserve_entries(children.len())?;
        for child in children {
            self.no_view_subtree(function, child.expression, state, depth + 1)?;
        }
        if let ExpressionOperation::Let { bindings, .. } = operation {
            for binding in bindings {
                let b = self.binding(function, binding, BindingKind::Let, true)?;
                if let Some(ty) = b.declared_type
                    && self.resolver.contains_read_view(ty)?
                {
                    return Err(Failure::Category);
                }
                self.no_view_subtree(function, b.value.ok_or(Failure::Missing)?, state, depth + 1)?;
            }
        }
        Ok(())
    }
    fn ordinary_type(
        &mut self,
        function: DeclarationReference,
        id: ExpressionId,
        state: &State,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.check_depth(depth)?;
        use ExpressionOperation as E;
        let operation = self.expression(function, id)?;
        match operation {
            E::Unit {} => self.resolver.intern(TypeForm::Unit),
            E::Bool { .. } => self.resolver.intern(TypeForm::Bool),
            E::I64 { .. } => self.resolver.intern(TypeForm::I64),
            E::F64 { .. } => self.resolver.intern(TypeForm::F64),
            E::Text { .. } => self.resolver.intern(TypeForm::Text),
            E::StaticText { .. } => self.resolver.intern(TypeForm::StaticText),
            E::Local { value } => state
                .locals
                .get(&value)
                .map(|s| s.value.ty)
                .or_else(|| Oracle(self.resolver.source, Some(function)).local_type(value))
                .ok_or(Failure::Authority),
            E::SequenceEmpty { sequence_type } | E::SequencePush { sequence_type, .. } => {
                Ok(sequence_type)
            }
            E::SequenceLength { .. } => self.resolver.intern(TypeForm::I64),
            E::SequencePop { result_type, .. } | E::SequenceReplace { result_type, .. } => {
                Ok(result_type)
            }
            E::SequenceGet { sequence_type, .. } => match self.resolver.object(sequence_type)?.form
            {
                TypeForm::OwnedSequence { item } => Ok(item),
                _ => Err(Failure::Category),
            },
            E::PackOwned { product_type, .. } => Ok(product_type),
            E::PackOwnedRecord { record_type, .. } => Ok(record_type),
            E::ChooseOwned { choice_type, .. } => Ok(choice_type),
            E::List { item_type, .. } => self.resolver.intern(TypeForm::List { item: item_type }),
            E::Map {
                key_type,
                value_type,
                ..
            } => self.resolver.intern(TypeForm::Map {
                key: key_type,
                value: value_type,
            }),
            E::Constant { declaration } => match self
                .resolver
                .source
                .owners
                .get(&OwnerKey::Declaration(declaration.declaration))
            {
                Some(OwnerRecord::Declaration(d))
                    if declaration.package == self.resolver.source.root.package_id =>
                {
                    match d.payload {
                        DeclarationPayload::Constant { ty, .. } => Ok(ty),
                        _ => Err(Failure::Category),
                    }
                }
                _ => Err(Failure::Unproved),
            },
            E::Call { .. } | E::ImplementationCall { .. } | E::MethodCall { .. } => {
                Ok(self.call_plan(function, id, operation)?.signature.result.ty)
            }
            E::Let { body, .. }
            | E::UnpackOwned { body, .. }
            | E::UnpackOwnedRecord { body, .. }
            | E::BorrowOwnedField { body, .. }
            | E::BorrowOwnedRecordField { body, .. }
            | E::BorrowOwnedItem { body, .. }
            | E::BorrowCall { body, .. }
            | E::Transaction { body, .. } => self.ordinary_type(function, body, state, depth + 1),
            E::Sequence { items } => self.ordinary_type(
                function,
                *items.last().ok_or(Failure::Inventory)?,
                state,
                depth + 1,
            ),
            E::If {
                when_true,
                when_false,
                ..
            } => {
                let a = self.ordinary_type(function, when_true, state, depth + 1)?;
                if a != self.ordinary_type(function, when_false, state, depth + 1)? {
                    return Err(Failure::Category);
                }
                Ok(a)
            }
            E::Record {
                nominal_type: Some(declaration),
                type_arguments,
                ..
            } => self.resolver.intern(if type_arguments.is_empty() {
                TypeForm::Named { declaration }
            } else {
                TypeForm::Applied {
                    declaration,
                    arguments: type_arguments,
                }
            }),
            E::Record {
                nominal_type: None,
                fields,
                ..
            } => {
                self.resolver.reserve_entries(fields.len())?;
                let mut types = Vec::new();
                for field in fields {
                    let FieldSelector::Structural(name) = field.selector else {
                        return Err(Failure::Category);
                    };
                    types.push(StructuralTypeField {
                        name,
                        ty: self.ordinary_type(function, field.value, state, depth + 1)?,
                    });
                }
                types.sort_by(|a, b| a.name.cmp(&b.name));
                self.resolver
                    .intern(TypeForm::StructuralRecord { fields: types })
            }
            E::Field {
                value,
                selector: FieldSelector::Structural(name),
            } => {
                let ty = self.ordinary_type(function, value, state, depth + 1)?;
                match self.resolver.object(ty)?.form.clone() {
                    TypeForm::StructuralRecord { fields } => fields
                        .iter()
                        .find(|f| f.name == name)
                        .map(|f| f.ty)
                        .ok_or(Failure::Authority),
                    _ => Err(Failure::Unproved),
                }
            }
            E::Invoke { callee, .. } => {
                let ty = self.ordinary_type(function, callee, state, depth + 1)?;
                match self.resolver.object(ty)?.form {
                    TypeForm::Function { result, .. } | TypeForm::TaskFunction { result, .. } => {
                        Ok(result)
                    }
                    _ => Err(Failure::Category),
                }
            }
            E::FunctionValue {
                function: callee,
                type_arguments,
                ..
            } => {
                let overlay = self.resolver.overlay();
                let signature = Oracle(&overlay, Some(function))
                    .applied_signature(callee, &type_arguments)
                    .ok_or(Failure::Unproved)?;
                let parameters = signature.0.iter().map(|p| p.ty).collect();
                self.resolver.reserve_entries(signature.0.len())?;
                self.resolver.intern(if signature.2 {
                    TypeForm::Function {
                        parameters,
                        result: signature.1,
                    }
                } else {
                    return Err(Failure::Unproved);
                })
            }
            _ => Err(Failure::Unproved),
        }
    }
}

#[cfg(test)]
#[path = "read_view_flow_reference_tests.rs"]
mod tests;
