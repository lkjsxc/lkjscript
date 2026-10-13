//! Private read custody between joined invocations. This is never a raw value.
use super::super::super::read_view_storage::{
    ReadView, RuntimeScopeLease, SourceAnchor, SourceCoordinate,
};
use super::super::super::value::MemoryForm;
use super::{
    ExecutionControl, ExecutionError, FunctionIndex, NormalizedProgram, NormalizedValue,
    TaskApplication, TypeForm, TypeObjectDigest, ValueOrigin,
};
use std::sync::Arc;

/// Retains evaluator-owned checked values and their ancestor cleanup guards.
/// Its contents are inaccessible: a clone shares cleanup custody, not ownership.
pub(in super::super::super) struct ScopedReadCustody {
    _retained: Box<dyn Send + Sync>,
}

impl ScopedReadCustody {
    pub(in super::super::super) fn new<T: Send + Sync + 'static>(retained: T) -> Arc<Self> {
        Arc::new(Self {
            _retained: Box::new(retained),
        })
    }

    /// Evaluators reserve this wrapper and their retained value before minting it.
    pub(in super::super::super) const ALLOCATION_BYTES: u64 =
        (std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>()) as u64;
}

pub(in super::super::super) enum ScopedArgument {
    Value(NormalizedValue),
    Read {
        value: NormalizedValue,
        custody: Arc<ScopedReadCustody>,
    },
    ReadScoped {
        value: NormalizedValue,
        custody: Arc<ScopedReadCustody>,
        scopes: Vec<ScopedReadScope>,
    },
    View {
        value: NormalizedValue,
        source: SourceAnchor,
        custody: Arc<ScopedReadCustody>,
    },
}

pub(in super::super::super) struct ScopedReadScope {
    key: crate::platform::kernel::ReadScopeArgument,
    lease: RuntimeScopeLease,
    placement: SourceAnchor,
    lifetime: SourceAnchor,
}

impl ScopedReadScope {
    pub(in super::super::super) fn new(
        key: crate::platform::kernel::ReadScopeArgument,
        lease: RuntimeScopeLease,
        placement: SourceAnchor,
        lifetime: SourceAnchor,
    ) -> Result<Self, ExecutionError> {
        if key != lease.key()
            || lease.source() != &lifetime
            || placement.program != lifetime.program
        {
            return Err(super::reject());
        }
        Ok(Self {
            key,
            lease,
            placement,
            lifetime,
        })
    }
}

enum ReadKind {
    Owned {
        scopes: Vec<ScopedReadScope>,
    },
    View {
        parent: SourceAnchor,
        destination: SourceAnchor,
        scope: Box<RuntimeScopeLease>,
    },
}

/// A sealed lineage edge whose lifetime extends beyond child frame replacement.
/// Drop it only after every child operand, activation and nested child has closed.
pub(in super::super::super) struct ScopedReadLease {
    position: usize,
    program: ValueOrigin,
    source: ValueOrigin,
    destination: ValueOrigin,
    ty: TypeObjectDigest,
    function: FunctionIndex,
    application: ValueOrigin,
    types: Arc<[TypeObjectDigest]>,
    scopes: Arc<[crate::platform::kernel::ReadScopeArgument]>,
    kind: ReadKind,
    // An inert token binds the allocation without acquiring any extra rights.
    backing: NormalizedValue,
    _custody: Arc<ScopedReadCustody>,
}

impl ScopedReadLease {
    // Evaluators charge this worst-case retained storage before minting either kind.
    // The view lease moved behind a Box is counted once, outside the inline wrapper.
    pub(in super::super::super) const ALLOCATION_BYTES: u64 =
        (std::mem::size_of::<Self>() + std::mem::size_of::<RuntimeScopeLease>()) as u64;

    #[allow(
        clippy::too_many_arguments,
        reason = "exact scoped invocation and custody boundary"
    )]
    pub(in super::super::super) fn seal(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: &TaskApplication,
        position: usize,
        value: &NormalizedValue,
        custody: Arc<ScopedReadCustody>,
        control: &ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        Self::seal_with_scopes(
            program,
            source,
            destination,
            application,
            position,
            value,
            custody,
            Vec::new(),
            control,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "exact read source scopes and custody boundary"
    )]
    pub(in super::super::super) fn seal_with_scopes(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: &TaskApplication,
        position: usize,
        value: &NormalizedValue,
        custody: Arc<ScopedReadCustody>,
        scopes: Vec<ScopedReadScope>,
        control: &ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if source == destination {
            return Err(super::reject());
        }
        if !application.matches(program, control)? {
            return Err(super::reject());
        }
        let parameter = program
            .functions
            .get(application.function().0 as usize)
            .filter(|_| application.function().1 == program.value_origin)
            .and_then(|function| function.parameters.get(position))
            .ok_or_else(super::reject)?;
        if parameter.use_mode != crate::platform::kernel::ParameterUse::Borrow
            || parameter.resource_requirement.is_some()
        {
            return Err(super::reject());
        }
        let ty = application.parameter(program, position)?;
        inspect(program, source, ty, value)?;
        validate_scopes(program, source, application, position, &scopes, control)?;
        Ok(Self {
            position,
            program: program.value_origin,
            source,
            destination,
            ty,
            function: application.function(),
            application: application.application_identity(),
            types: Arc::clone(application.types()),
            scopes: Arc::clone(application.read_scope_arguments()),
            kind: ReadKind::Owned { scopes },
            backing: value.clone(),
            _custody: custody,
        })
    }

    /// View sharing is minted by the evaluator's complete canonical admission.
    /// This boundary binds its live token to exactly one child parameter and keeps
    /// the complete parent checked placement alive until child cleanup has joined.
    #[allow(
        clippy::too_many_arguments,
        reason = "exact joined view custody boundary"
    )]
    pub(in super::super::super) fn seal_view(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: &TaskApplication,
        position: usize,
        value: &NormalizedValue,
        parent: SourceAnchor,
        custody: Arc<ScopedReadCustody>,
        control: &ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if source == destination || !application.matches(program, control)? {
            return Err(super::reject());
        }
        let parameter = program
            .functions
            .get(application.function().0 as usize)
            .and_then(|function| function.parameters.get(position))
            .ok_or_else(super::reject)?;
        if parameter.use_mode != crate::platform::kernel::ParameterUse::Borrow
            || parameter.resource_requirement.is_some()
            || parent.program != program.value_origin
            || parent.origin != source
        {
            return Err(super::reject());
        }
        let ty = application.parameter(program, position)?;
        let view = inspect_view(
            program,
            destination,
            application,
            position,
            ty,
            value,
            control,
        )?;
        let scope = view.scope()?.reborrow(control)?;
        let anchor = view.source().clone();
        Ok(Self {
            position,
            program: program.value_origin,
            source,
            destination,
            ty,
            function: application.function(),
            application: application.application_identity(),
            types: Arc::clone(application.types()),
            scopes: Arc::clone(application.read_scope_arguments()),
            kind: ReadKind::View {
                parent,
                destination: anchor,
                scope: Box::new(scope),
            },
            backing: value.clone(),
            _custody: custody,
        })
    }

    pub(in super::super::super) fn position(&self) -> usize {
        self.position
    }

    pub(in super::super::super) fn imported_scope_count(
        &self,
        application: &TaskApplication,
    ) -> usize {
        match &self.kind {
            ReadKind::Owned { scopes } => scopes.len(),
            ReadKind::View { .. } => application.read_scope_source_count(self.position),
        }
    }

    pub(in super::super::super) fn visit_imported_scopes(
        &self,
        application: &TaskApplication,
        control: &ExecutionControl,
        visit: &mut impl FnMut(
            usize,
            crate::platform::kernel::ReadScopeArgument,
            RuntimeScopeLease,
        ) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        if self.application != application.application_identity() {
            return Err(super::reject());
        }
        match &self.kind {
            ReadKind::Owned { scopes } => {
                for scope in scopes {
                    visit(self.position, scope.key, scope.lease.reborrow(control)?)?;
                }
            }
            ReadKind::View { scope, .. } => {
                application.visit_read_scope_keys(self.position, &mut |key| {
                    if scope.key() != key {
                        return Err(super::reject());
                    }
                    visit(self.position, key, scope.reborrow(control)?)
                })?
            }
        }
        Ok(())
    }

    pub(in super::super::super) fn adopt(
        &self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        application: &TaskApplication,
        value: &mut NormalizedValue,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        if self.program != program.value_origin
            || self.destination != destination
            || self.function != application.function()
            || self.application != application.application_identity()
            || self.types.as_ref() != application.types().as_ref()
            || self.scopes.as_ref() != application.read_scope_arguments().as_ref()
            || self.ty != application.parameter(program, self.position)?
            || self.backing != *value
            || !application.matches(program, control)?
        {
            return Err(super::reject());
        }
        if let ReadKind::View {
            parent,
            destination: anchor,
            scope,
        } = &self.kind
        {
            let view = inspect_view(
                program,
                destination,
                application,
                self.position,
                self.ty,
                value,
                control,
            )?;
            if parent.program != self.program
                || parent.origin != self.source
                || view.source() != anchor
                || !scope.same_live_scope(view.scope()?)
            {
                return Err(super::reject());
            }
            return Ok(());
        }
        if let ReadKind::Owned { scopes } = &self.kind {
            validate_scopes(
                program,
                self.source,
                application,
                self.position,
                scopes,
                control,
            )?;
        }
        inspect(program, self.source, self.ty, value)?;
        match value {
            NormalizedValue::ByteBuffer(token) => token.adopt_scoped_read(self.source, destination),
            NormalizedValue::OwnedI64Cell(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            NormalizedValue::OwnedProduct(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            NormalizedValue::OwnedChoice(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            NormalizedValue::OwnedSequence(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            _ => Err(super::reject()),
        }
    }
}

fn validate_scopes(
    program: &NormalizedProgram,
    source: ValueOrigin,
    application: &TaskApplication,
    position: usize,
    scopes: &[ScopedReadScope],
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    if scopes.len() != application.read_scope_source_count(position) {
        return Err(super::reject());
    }
    let mut index = 0;
    application.visit_read_scope_keys(position, &mut |key| {
        let scope = scopes.get(index).ok_or_else(super::reject)?;
        index += 1;
        scope.lease.reborrow(control)?;
        if key != scope.key
            || key != scope.lease.key()
            || scope.lease.source().program != program.value_origin
            || scope.placement.program != program.value_origin
            || scope.placement.origin != source
            || scope.lease.source() != &scope.lifetime
        {
            return Err(super::reject());
        }
        Ok(())
    })
}

fn inspect_view<'a>(
    program: &NormalizedProgram,
    destination: ValueOrigin,
    application: &TaskApplication,
    position: usize,
    ty: TypeObjectDigest,
    value: &'a NormalizedValue,
    control: &ExecutionControl,
) -> Result<&'a ReadView, ExecutionError> {
    let NormalizedValue::ReadView(view) = value else {
        return Err(super::reject());
    };
    view.validate(control)?;
    if !super::admit_shareable_type(program, ty, control)? {
        return Err(super::reject());
    }
    let anchor = view.source();
    if !view.is_shared()
        || view.identity().program != program.value_origin
        || view.identity().ty != ty
        || program.read_view_layout(ty).is_none()
        || anchor.program != program.value_origin
        || anchor.application != application.application_identity()
        || anchor.invocation != destination
        || anchor.origin != destination
        || anchor.coordinate
            != SourceCoordinate::Parameter(u32::try_from(position).map_err(|_| super::reject())?)
        || !anchor.path.is_empty()
    {
        return Err(super::reject());
    }
    Ok(view)
}

/// The certificate is first-party typed construction evidence in the frozen
/// backing root. Lending checks its header, never the stored descendants.
fn inspect(
    program: &NormalizedProgram,
    source: ValueOrigin,
    ty: TypeObjectDigest,
    value: &NormalizedValue,
) -> Result<(), ExecutionError> {
    let expected = match &program.types.get(&ty).ok_or_else(super::reject)?.form {
        TypeForm::ByteBuffer => MemoryForm::ByteBuffer,
        TypeForm::OwnedI64Cell => MemoryForm::OwnedI64Cell,
        TypeForm::OwnedProduct { .. } => MemoryForm::Product(ty),
        TypeForm::Named { .. } | TypeForm::Applied { .. }
            if program.owned_record_layout(ty).is_some() =>
        {
            MemoryForm::Product(ty)
        }
        TypeForm::OwnedChoice { .. } => MemoryForm::Choice(ty),
        TypeForm::OwnedSequence { .. } => MemoryForm::Sequence(ty),
        _ => return Err(super::reject()),
    };
    if value.memory_form() != Some(expected) || !value.memory_is_borrowed() {
        return Err(super::reject());
    }
    value.memory_validate(source, false)?;
    match value {
        NormalizedValue::ByteBuffer(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedI64Cell(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedProduct(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedChoice(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedSequence(token) => token.validate_admission(program.value_origin),
        _ => Err(super::reject()),
    }
}
