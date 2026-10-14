//! Invocation-local scope substitution and live source admission for the reference tier.
//!
//! A formal map never owns the caller's source lifetime. The lexical caller retains
//! that owner across its body; this frame carries only independently admitted leases.

use super::super::super::read_view_storage::{
    RuntimeScopeLease, RuntimeScopeOwner, SourceAnchor, SourceCoordinate,
};
use super::super::super::value::ValueOrigin;
use super::super::*;
use crate::platform::kernel::{OwnerKind, ParameterParent, ReadScopeBinder, ReadScopeReference};
use std::collections::BTreeSet;

pub(in super::super) struct ScopeFrame {
    program: ValueOrigin,
    declaration: Option<DeclarationReference>,
    application: ValueOrigin,
    invocation: ValueOrigin,
    parameters: Vec<ParameterId>,
    bindings: BTreeMap<ReadScopeReference, ReadScopeArgument>,
    actuals: BTreeSet<ReadScopeArgument>,
    leases: BTreeMap<ReadScopeArgument, RuntimeScopeLease>,
    prospective: BTreeSet<ReadScopeArgument>,
    owners: BTreeMap<ReadScopeArgument, RuntimeScopeOwner>,
}

impl ScopeFrame {
    fn new(
        program: ValueOrigin,
        declaration: Option<DeclarationReference>,
    ) -> Result<Self, ExecutionError> {
        Ok(Self {
            program,
            declaration,
            application: fresh()?,
            invocation: fresh()?,
            parameters: Vec::new(),
            bindings: BTreeMap::new(),
            actuals: BTreeSet::new(),
            leases: BTreeMap::new(),
            prospective: BTreeSet::new(),
            owners: BTreeMap::new(),
        })
    }

    pub(in super::super) fn declaration(&self) -> Option<DeclarationReference> {
        self.declaration
    }
    pub(in super::super) fn parameter_ids(&self) -> &[ParameterId] {
        &self.parameters
    }
    pub(in super::super) fn source_anchor(
        &self,
        local: LocalValueReference,
        origin: ValueOrigin,
    ) -> Result<SourceAnchor, ExecutionError> {
        let coordinate = match local {
            LocalValueReference::FunctionParameter(parameter) => {
                let position = self
                    .parameters
                    .iter()
                    .position(|id| *id == parameter)
                    .ok_or_else(|| failure("source parameter is outside this exact invocation"))?;
                SourceCoordinate::Parameter(u32::try_from(position).map_err(|_| capacity())?)
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => {
                if self.declaration.is_none() {
                    return Err(failure(
                        "captured actuals cannot invent a lexical source binding",
                    ));
                }
                SourceCoordinate::Binding(binding)
            }
            LocalValueReference::OperationParameter(_)
            | LocalValueReference::TransactionBinding(_) => {
                return Err(failure(
                    "source coordinate is outside the admitted function's data locals",
                ));
            }
        };
        Ok(SourceAnchor {
            program: self.program,
            application: self.application,
            invocation: self.invocation,
            coordinate,
            origin,
            path: Arc::from([]),
        })
    }

    /// Simultaneous substitution replaces only this invocation's exact formal.
    /// Inserted caller actuals remain opaque, including equal parameter IDs.
    pub(in super::super) fn resolve(
        &self,
        scope: ReadScopeArgument,
    ) -> Result<ReadScopeArgument, ExecutionError> {
        if let ReadScopeArgument::Parameter(reference) = scope
            && let Some(actual) = self.bindings.get(&reference)
        {
            return Ok(*actual);
        }
        if self.actuals.contains(&scope) {
            return Ok(scope);
        }
        Err(failure("read scope is outside the exact active invocation"))
    }

    pub(in super::super) fn scope(
        &self,
        actual: ReadScopeArgument,
    ) -> Result<&RuntimeScopeLease, ExecutionError> {
        if !self.actuals.contains(&actual) {
            return Err(failure(
                "runtime lease lookup requires an exact resolved caller actual",
            ));
        }
        self.leases
            .get(&actual)
            .ok_or_else(|| failure("symbolic read scope has no admitted live source lease"))
    }

    /// Only the actual lexical loan creator may add a source scope. Its owner is
    /// retained by that caller, independently of this frame's formal bindings.
    pub(in super::super) fn bind_scope(
        &mut self,
        scope: ReadScopeArgument,
        lease: RuntimeScopeLease,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        let ReadScopeArgument::Lexical { callable, .. } = scope else {
            return Err(failure("a formal binding cannot create a source lifetime"));
        };
        if self.declaration != Some(callable)
            || lease.key() != scope
            || lease.source().program != self.program
            || (self.actuals.contains(&scope) && !self.prospective.contains(&scope))
            || self.leases.contains_key(&scope)
        {
            return Err(failure(
                "lexical scope does not name one fresh containing loan",
            ));
        }
        // Validate liveness before making the lease visible in the frame.
        let lease = lease.reborrow(control)?;
        self.actuals.insert(scope);
        self.leases.insert(scope, lease);
        Ok(())
    }

    pub(in super::super) fn begin_prospective_scope(
        &mut self,
        scope: ReadScopeArgument,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        let ReadScopeArgument::Lexical { callable, .. } = scope else {
            return Err(failure("prospective loan requires a lexical binding"));
        };
        if self.declaration != Some(callable)
            || self.actuals.contains(&scope)
            || !self.prospective.insert(scope)
        {
            return Err(failure(
                "prospective loan aliases another exact live binding",
            ));
        }
        self.actuals.insert(scope);
        Ok(())
    }

    pub(in super::super) fn remove_scope(&mut self, scope: ReadScopeArgument) {
        self.leases.remove(&scope);
        self.actuals.remove(&scope);
        self.prospective.remove(&scope);
        self.owners.remove(&scope);
    }

    pub(in super::super) fn is_prospective(&self, scope: ReadScopeArgument) -> bool {
        self.prospective.contains(&scope)
    }

    pub(in super::super) fn bind_owned_scope(
        &mut self,
        scope: ReadScopeArgument,
        owner: RuntimeScopeOwner,
        lease: RuntimeScopeLease,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        if !self.prospective.contains(&scope) || self.owners.contains_key(&scope) {
            return Err(failure(
                "source owner lacks one exact prospective lexical context",
            ));
        }
        self.bind_scope(scope, lease, control)?;
        self.owners.insert(scope, owner);
        Ok(())
    }
}

impl ReferenceState<'_> {
    pub(in super::super) fn resolve_read_scope_arguments(
        &mut self,
        actuals: &[ReadScopeArgument],
    ) -> Result<Vec<ReadScopeArgument>, ExecutionError> {
        self.charge_items(actuals.len(), std::mem::size_of::<ReadScopeArgument>())?;
        let mut resolved = Vec::with_capacity(actuals.len());
        for actual in actuals {
            self.control.check()?;
            self.validate_read_scope_actual(*actual)?;
            let frame = self
                .read_scope_frames
                .last()
                .ok_or_else(|| failure("read-scope actual has no active canonical caller"))?;
            resolved.push(frame.resolve(*actual)?);
        }
        Ok(resolved)
    }

    /// A scoped child receives exact actual keys and leases directly. This
    /// capture neither invents child formals nor reads a dense callable index.
    pub(in super::super) fn capture_read_scope_frame(
        &self,
        actuals: &[ReadScopeArgument],
    ) -> Result<ScopeFrame, ExecutionError> {
        self.control.check()?;
        if actuals.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
            return Err(capacity());
        }
        let mut captured = ScopeFrame::new(self.schema.value_origin, None)?;
        for actual in actuals {
            self.control.check()?;
            self.validate_read_scope_actual(*actual)?;
            let caller = self
                .read_scope_frames
                .last()
                .ok_or_else(|| failure("scoped child has no admitted canonical caller"))?;
            // Task preparation already resolved its authored scope actuals.
            // Replacing them a second time would capture the wrong invocation
            // when recursive calls permute same-binder scope parameters.
            let actual = *actual;
            if !caller.actuals.contains(&actual) {
                return Err(failure(
                    "captured scope is outside the resolved caller actual inventory",
                ));
            }
            captured.actuals.insert(actual);
            if let Some(lease) = caller.leases.get(&actual) {
                captured
                    .leases
                    .insert(actual, lease.reborrow(self.control)?);
            }
        }
        Ok(captured)
    }

    pub(in super::super) fn admit_read_scope_frame(
        &mut self,
        declaration: DeclarationReference,
        function: &mut FunctionDeclaration,
        parameters: &[ParameterRecord],
        actuals: &[ReadScopeArgument],
        arguments: &mut [CheckedValue],
    ) -> Result<ScopeFrame, ExecutionError> {
        self.control.check()?;
        if function.parameters.len() != arguments.len()
            || function.parameters.len() != parameters.len()
            || function.read_scope_parameters.len() != actuals.len()
        {
            return Err(failure("complete callable scope or argument arity differs"));
        }
        let proof = self.schema.read_view_proof.clone();
        if let Some(proof) = &proof {
            let raw = proof.declaration(declaration)?;
            let DeclarationPayload::Function(raw_function) = &raw.payload else {
                return Err(failure("scope frame names a non-function canonical owner"));
            };
            // The surrounding call admission may already have independently
            // closed result.ty; its source relation and all inventories stay raw.
            // Effects are admitted from the current canonical reader by that
            // call's allowance checks, rather than by this frozen scope proof.
            if raw_function.parameters != function.parameters
                || raw_function.type_parameters != function.type_parameters
                || raw_function.implementation_parameters != function.implementation_parameters
                || raw_function.effect_parameters != function.effect_parameters
                || raw_function.requirement_parameters != function.requirement_parameters
                || raw_function.read_scope_parameters != function.read_scope_parameters
                || raw_function.read_scope_sources != function.read_scope_sources
                || raw_function.result.relation != function.result.relation
                || raw_function.body != function.body
            {
                return Err(failure(
                    "scope frame differs from the raw canonical callable inventory",
                ));
            }
            let binder = ReadScopeBinder::Declaration(declaration);
            if proof.scope_parameters(binder)? != function.read_scope_parameters.as_slice() {
                return Err(failure("scope frame omits or imports a canonical formal"));
            }
            let sources = proof.scope_sources(binder)?;
            if sources.len() != function.read_scope_sources.len() {
                return Err(failure("scope frame omits a canonical source relationship"));
            }
            for (admitted, raw) in sources.iter().zip(&function.read_scope_sources) {
                self.control.check()?;
                let position = function
                    .parameters
                    .iter()
                    .position(|id| *id == raw.source)
                    .ok_or_else(|| {
                        failure("scope source is outside the ordered parameter inventory")
                    })?;
                if admitted.source as usize != position || admitted.scope != raw.scope {
                    return Err(failure(
                        "scope source changes its exact ordered parameter or formal",
                    ));
                }
            }
        } else if !actuals.is_empty()
            || !function.read_scope_sources.is_empty()
            || arguments
                .iter()
                .any(|value| matches!(value.raw(), NormalizedValue::ReadView(_)))
        {
            return Err(failure(
                "scope-bearing invocation lacks independent raw canonical admission",
            ));
        }

        // Call admission already read and precharged this exact ordered vector.
        // Its possibly closed type digests are not interpreted here; all raw
        // header, parent, use, resource, ownership and source gates remain below.
        self.charge_items(
            actuals.len(),
            std::mem::size_of::<(ReadScopeReference, ReadScopeArgument)>()
                + std::mem::size_of::<ReadScopeArgument>()
                + 6 * std::mem::size_of::<usize>(),
        )?;
        let mut frame = ScopeFrame::new(self.schema.value_origin, Some(declaration))?;
        for (formal, actual) in function.read_scope_parameters.iter().zip(actuals) {
            self.validate_read_scope_actual(*actual)?;
            let caller = self
                .read_scope_frames
                .last()
                .ok_or_else(|| failure("scope-bearing call has no admitted caller invocation"))?;
            // Complete call preparation supplies resolved actuals. Substitution
            // is simultaneous: never interpret an inserted actual as a new formal.
            let actual = *actual;
            if !caller.actuals.contains(&actual) {
                return Err(failure(
                    "callee scope is outside the resolved caller actual inventory",
                ));
            }
            let reference = ReadScopeReference {
                binder: ReadScopeBinder::Declaration(declaration),
                parameter: formal.id,
            };
            if frame.bindings.insert(reference, actual).is_some() {
                return Err(failure("duplicate exact callable scope formal"));
            }
            frame.actuals.insert(actual);
            if let Some(lease) = caller.leases.get(&actual) {
                frame.leases.insert(actual, lease.reborrow(self.control)?);
            }
        }

        for ((id, parameter), value) in function
            .parameters
            .iter()
            .zip(parameters)
            .zip(arguments.iter())
        {
            self.control.check()?;
            if parameter.header.owner != OwnerKey::Parameter(*id)
                || parameter.header.kind != OwnerKind::Parameter
                || parameter.parent != ParameterParent::Function(declaration.declaration)
            {
                return Err(failure("scope frame parameter has another canonical owner"));
            }
            value.ownership(&self.schema, &mut self.observation.value_work)?;
        }
        for relationship in &function.read_scope_sources {
            self.control.check()?;
            let position = function
                .parameters
                .iter()
                .position(|id| *id == relationship.source)
                .ok_or_else(|| failure("scope relationship selects a foreign parameter"))?;
            let actual = frame.resolve(relationship.scope)?;
            let argument = &arguments[position];
            if parameters[position].use_mode != ParameterUse::Borrow
                || parameters[position].resource_requirement.is_some()
                || !(argument.raw().memory_is_borrowed()
                    || (matches!(argument.raw(), NormalizedValue::ReadView(_))
                        && argument.is_view_borrowed()))
            {
                return Err(failure(
                    "scope relationship requires its exact borrowed Owned or view argument",
                ));
            }
            let lease = argument.source_scope().ok_or_else(|| {
                failure("ordered source argument has no independently admitted runtime lease")
            })?;
            if lease.key() != actual || lease.source().program != self.schema.value_origin {
                return Err(failure(
                    "source argument forwards another exact actual scope",
                ));
            }
            if let Some(existing) = frame.leases.get(&actual)
                && !existing.same_live_scope(lease)
            {
                return Err(failure(
                    "one scope formal combines independent runtime source lifetimes",
                ));
            }
            self.charge_items(
                1,
                std::mem::size_of::<(ReadScopeArgument, RuntimeScopeLease)>()
                    + 3 * std::mem::size_of::<usize>(),
            )?;
            frame.leases.insert(actual, lease.reborrow(self.control)?);
        }

        // Each borrowed argument gets its own new ordered invocation coordinate.
        // Selected origin and tagged descendant path survive forwarding intact.
        for (position, ((id, parameter), value)) in function
            .parameters
            .iter()
            .zip(parameters)
            .zip(arguments.iter_mut())
            .enumerate()
        {
            self.control.check()?;
            if parameter.use_mode != ParameterUse::Borrow
                || !(value.raw().memory_is_borrowed()
                    || matches!(value.raw(), NormalizedValue::ReadView(_)))
            {
                continue;
            }
            let selected = value.source_anchor();
            if selected.is_some_and(|source| source.program != self.schema.value_origin) {
                return Err(failure(
                    "borrowed argument source belongs to another canonical preparation",
                ));
            }
            let origin = if value.raw().memory_is_borrowed() {
                // Structured adoption changes the sealed token's read domain;
                // retained scope custody never changes that domain back.
                self.memory_domain
            } else {
                selected.map_or(self.memory_domain, |source| source.origin)
            };
            let path = selected.map_or_else(|| Arc::from([]), |source| Arc::clone(&source.path));
            if value.raw().memory_is_borrowed() {
                value.raw().memory_validate(origin, false)?;
            } else if let NormalizedValue::ReadView(view) = value.raw() {
                view.validate(self.control)?;
                if view.identity().program != self.schema.value_origin {
                    return Err(failure(
                        "view argument belongs to another canonical preparation",
                    ));
                }
            }
            let lease = value
                .source_scope()
                .map(|scope| scope.reborrow(self.control))
                .transpose()?;
            let destination = SourceAnchor {
                program: self.schema.value_origin,
                application: frame.application,
                invocation: frame.invocation,
                coordinate: SourceCoordinate::Parameter(
                    u32::try_from(position).map_err(|_| capacity())?,
                ),
                origin,
                path,
            };
            let binding = CheckedValue::reserve_source_binding(destination, lease, &mut |bytes| {
                self.charge_allocation(bytes)
            })?;
            if let NormalizedValue::ReadView(view) = value.raw() {
                let source = view.source().clone();
                let actual = value
                    .source_scope()
                    .ok_or_else(|| failure("borrowed view argument lost its actual scope lease"))?
                    .key();
                let schema = Arc::clone(&self.schema);
                let admission = super::RuntimeAdmission::rebase(
                    &schema,
                    &source,
                    binding.anchor(),
                    actual,
                    self.control,
                )?;
                value.rebind_view_source(Arc::clone(&binding), &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
            }
            value.bind_source(binding);
            value.set_provenance(LocalValueReference::FunctionParameter(*id));
        }
        // All admission checks have finished. Retain the invocation's existing
        // ordered ID storage instead of allocating a second parameter vector.
        frame.parameters = std::mem::take(&mut function.parameters);
        Ok(frame)
    }

    fn validate_read_scope_actual(&self, actual: ReadScopeArgument) -> Result<(), ExecutionError> {
        self.control.check()?;
        let proof = self
            .schema
            .read_view_proof
            .as_ref()
            .ok_or_else(|| failure("scope actual lacks independent raw canonical admission"))?;
        match actual {
            ReadScopeArgument::Parameter(reference) => {
                if !proof
                    .scope_parameters(reference.binder)?
                    .iter()
                    .any(|formal| formal.id == reference.parameter)
                {
                    return Err(failure("scope actual imports a foreign canonical formal"));
                }
            }
            ReadScopeArgument::Lexical { .. } => {
                // Lexical liveness comes only from the containing caller's
                // explicit registered loan; a raw binding lookup cannot mint it.
                self.read_scope_frames
                    .last()
                    .ok_or_else(|| failure("lexical scope actual has no containing invocation"))?
                    .resolve(actual)?;
            }
        }
        Ok(())
    }
}

fn fresh() -> Result<ValueOrigin, ExecutionError> {
    ValueOrigin::fresh().ok_or_else(capacity)
}

fn capacity() -> ExecutionError {
    reference_resource(
        "normalized_reference_read_scope_capacity",
        "finite read-scope identity or inventory capacity exhausted",
    )
}

fn failure(message: impl Into<String>) -> ExecutionError {
    reference_type_error(message)
}
