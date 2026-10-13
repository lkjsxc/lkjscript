//! Production view admission and invocation custody. Symbolic keys never mint a lease.
use super::super::read_view_storage as storage;
use super::*;
use crate::platform::kernel::{
    ReadScopeArgument, ReadScopeBinder, ReadViewFieldKind, ResultRelation,
};
use storage::{
    AdmissionRequest, FieldAdmission, FieldKind, HeldRead, ReadPathStep, SourceAnchor,
    SourceCoordinate, ViewIdentity,
};

#[cfg(test)]
#[path = "vm_read_view_admission_tests.rs"]
mod tests;

pub(super) struct Scope {
    pub(super) key: ReadScopeArgument,
    pub(super) root: BorrowSource,
    pub(super) lease: storage::RuntimeScopeLease,
    // This owner belongs to the actual source activation, never to a descriptor local.
    pub(super) owner: Option<storage::RuntimeScopeOwner>,
}

pub(super) struct PendingView {
    ty: TypeObjectDigest,
    fields: Arc<[u32]>,
    base: usize,
    source: SourceAnchor,
    lifetime: SourceAnchor,
    root: BorrowSource,
    builder: storage::ViewBuilder,
}

/// An evaluator-established exact source and target. Every storage request is checked
/// against the complete prepared declaration, including fields absent from executed paths.
pub(super) struct Admission<'a> {
    pub(super) program: &'a NormalizedProgram,
    pub(super) selected: &'a SourceAnchor,
    pub(super) target: ViewIdentity,
    pub(super) scope: ReadScopeArgument,
    pub(super) lifetime: SourceAnchor,
    nodes: std::cell::Cell<usize>,
}

fn same_lineage(source: &SourceAnchor, selected: &SourceAnchor) -> bool {
    source.program == selected.program
        && source.application == selected.application
        && source.invocation == selected.invocation
        && source.coordinate == selected.coordinate
        && source.origin == selected.origin
        && source.path.starts_with(&selected.path)
}

impl Admission<'_> {
    fn tick(&self) -> Result<(), ExecutionError> {
        let nodes = self
            .nodes
            .get()
            .checked_add(1)
            .filter(|nodes| *nodes as u64 <= super::super::value::MAXIMUM_ADMISSION_ITEMS)
            .ok_or_else(|| {
                resource_error(
                    "normalized_view_admission_work",
                    "view admission exceeds finite capacity",
                )
            })?;
        self.nodes.set(nodes);
        Ok(())
    }
    fn layout(
        &self,
        ty: TypeObjectDigest,
    ) -> Result<&super::super::prepare::NormalizedReadViewLayout, ExecutionError> {
        self.program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view has no exact closed declaration application"))
    }

    fn construction(
        &self,
        construction: &storage::ViewConstruction,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        let layout = self.layout(construction.identity.ty)?;
        if construction.identity != self.target
            || construction.source != *self.selected
            || construction.scope.key() != self.scope
            || layout.read_scope != self.scope
            || construction.scope.source() != &self.lifetime
            || construction.fields != layout.fields.len()
        {
            return Err(type_error(
                "view construction has an inexact source, scope or field inventory",
            ));
        }
        self.complete_type(construction.identity.ty, false, 0, control)
    }

    fn complete_type(
        &self,
        ty: TypeObjectDigest,
        sharing: bool,
        depth: usize,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        self.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(type_error("view field closure is too deep"));
        }
        let layout = self.layout(ty)?;
        if layout.arguments.len() != layout.type_parameters.len()
            || layout.arguments.len() != layout.type_parameter_constraints.len()
        {
            return Err(type_error(
                "view application has an incomplete phantom/type inventory",
            ));
        }
        for (argument, constraint) in layout
            .arguments
            .iter()
            .zip(layout.type_parameter_constraints.iter())
        {
            control.check()?;
            if !self.program.types.contains_key(argument)
                || self.program.read_view_instances.contains_key(argument)
                || (constraint.has_owned() && !self.program.is_owned_type(*argument))
                || (!constraint.has_owned() && !self.program.buffer_free_types.contains(argument))
                || (constraint.requires_capture_safe()
                    && !self.program.capture_safe_types.contains(argument))
            {
                return Err(type_error("view phantom actual has an invalid category"));
            }
            if constraint.requires_share() {
                transfer::admit_shareable_type(self.program, *argument, control)?;
            }
            if constraint.requires_transfer() {
                transfer::admit_type(self.program, *argument, control)?;
            }
        }
        for field in layout.fields.iter() {
            control.check()?;
            match field.kind {
                ReadViewFieldKind::Ordinary => {
                    ordinary_type(self.program, field.ty, depth + 1, control)?;
                }
                ReadViewFieldKind::ReadReference { read_scope } => {
                    if read_scope != layout.read_scope {
                        return Err(type_error("view reference field has another scope"));
                    }
                    if self.program.read_view_instances.contains_key(&field.ty) {
                        if self.layout(field.ty)?.read_scope != read_scope {
                            return Err(type_error(
                                "nested view reference has another effective read scope",
                            ));
                        }
                        self.complete_type(field.ty, sharing, depth + 1, control)?;
                    } else if self.program.is_owned_type(field.ty) {
                        if sharing {
                            transfer::admit_shareable_type(self.program, field.ty, control)?;
                        }
                    } else {
                        return Err(type_error(
                            "view reference field has no positive read category",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn field(
        &self,
        ty: TypeObjectDigest,
        field: crate::platform::semantic_id::FieldId,
    ) -> Result<&super::super::prepare::NormalizedReadViewField, ExecutionError> {
        self.layout(ty)?
            .fields
            .iter()
            .find(|candidate| candidate.reference.field == field)
            .ok_or_else(|| {
                type_error("view field is not owned by the selected private declaration")
            })
    }
}

impl storage::ReadViewAdmission for Admission<'_> {
    fn validate(
        &self,
        request: AdmissionRequest<'_>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        self.tick()?;
        if self.target.program != self.program.value_origin
            || self.selected.program != self.program.value_origin
        {
            return Err(type_error("view permit belongs to another program"));
        }
        match request {
            AdmissionRequest::Scope {
                key,
                source,
                parent,
            } => {
                if key != self.scope || source != self.selected {
                    return Err(type_error(
                        "view scope does not bind its exact selected source",
                    ));
                }
                if let Some(parent) = parent {
                    if parent.source().program != source.program {
                        return Err(type_error("view scope has unrelated source custody"));
                    }
                }
            }
            AdmissionRequest::Source {
                construction,
                loans,
            } => {
                self.construction(construction, control)?;
                let selected = loans
                    .reads()
                    .first()
                    .ok_or_else(|| type_error("view source has no actual read token"))?;
                if selected.origin != self.selected.origin || !selected.value.memory_is_borrowed() {
                    return Err(type_error("view source custody is not an exact live read"));
                }
            }
            AdmissionRequest::Subview {
                construction,
                source,
            } => {
                self.construction(construction, control)?;
                if source.source() != self.selected {
                    return Err(type_error("subview uses another designated argument"));
                }
                self.complete_type(source.identity().ty, false, 0, control)?;
            }
            AdmissionRequest::Field {
                construction,
                input,
            } => {
                self.construction(construction, control)?;
                let (id, ty, kind, source) = match input {
                    FieldAdmission::Metadata { field, ty, value } => {
                        ordinary_shape(self.program, value, ty, 0, control)?;
                        (field, ty, FieldKind::Metadata, None)
                    }
                    FieldAdmission::Owned {
                        field,
                        ty,
                        source,
                        value,
                    } => {
                        if !value.memory_is_borrowed() || !self.program.is_owned_type(ty) {
                            return Err(type_error("view field is not a borrowed Owned referent"));
                        }
                        let expected =
                            direct_memory_type(self.program, ty, &BTreeMap::new(), control)?;
                        if expected.is_none() || value.memory_form() != expected {
                            return Err(type_error(
                                "view reference field has a mislabeled Owned representation",
                            ));
                        }
                        value.memory_validate(source.origin, false)?;
                        (field, ty, FieldKind::OwnedReference, Some(source))
                    }
                    FieldAdmission::View {
                        field,
                        ty,
                        source,
                        value,
                    } => {
                        if value.identity().ty != ty
                            || value.identity().program != self.program.value_origin
                        {
                            return Err(type_error(
                                "view field has an inexact descriptor application",
                            ));
                        }
                        self.complete_type(ty, false, 0, control)?;
                        (field, ty, FieldKind::ViewReference, Some(source))
                    }
                };
                let field = self.field(construction.identity.ty, id)?;
                if field.ty != ty
                    || !matches!(
                        (field.kind, kind),
                        (ReadViewFieldKind::Ordinary, FieldKind::Metadata)
                            | (
                                ReadViewFieldKind::ReadReference { .. },
                                FieldKind::OwnedReference | FieldKind::ViewReference
                            )
                    )
                {
                    return Err(type_error(
                        "view initializer has another field category or type",
                    ));
                }
                if source.is_some_and(|source| !same_lineage(source, self.selected)) {
                    return Err(type_error(
                        "view field does not descend from the ordered selected source",
                    ));
                }
            }
            AdmissionRequest::Finish {
                construction,
                fields,
            } => {
                self.construction(construction, control)?;
                let layout = self.layout(construction.identity.ty)?;
                if fields.len() != layout.fields.len() {
                    return Err(type_error("view field inventory is incomplete"));
                }
                for field in layout.fields.iter() {
                    control.check()?;
                    let mut selected = fields
                        .iter()
                        .filter(|stored| stored.descriptor().field == field.reference.field);
                    let stored = selected
                        .next()
                        .ok_or_else(|| type_error("view field inventory is incomplete"))?;
                    if selected.next().is_some() || stored.descriptor().ty != field.ty {
                        return Err(type_error("view field inventory is not exact"));
                    }
                }
            }
            AdmissionRequest::Retain {
                construction,
                guard,
            } => {
                self.construction(construction, control)?;
                if guard.identity().program != self.program.value_origin {
                    return Err(type_error(
                        "view ancestor custody belongs to another program",
                    ));
                }
                guard.validate(control)?;
                self.complete_type(guard.identity().ty, false, 0, control)?;
            }
            AdmissionRequest::RetainProjected {
                construction,
                guard,
            } => {
                self.construction(construction, control)?;
                if guard.source().program != self.program.value_origin {
                    return Err(type_error(
                        "projected ancestor custody belongs to another program",
                    ));
                }
                guard.raw(control)?;
            }
            AdmissionRequest::RetainRead { construction, read } => {
                self.construction(construction, control)?;
                if read.origin != self.selected.origin || !read.value.memory_is_borrowed() {
                    return Err(type_error(
                        "view ancestor custody is not a live read in the selected invocation",
                    ));
                }
                read.value.memory_validate(read.origin, false)?;
                CheckedValue::validate_memory_admission(&read.value, self.program.value_origin)?;
            }
            AdmissionRequest::Reborrow { source, target } => {
                if source.source() != self.selected
                    || target.identity != self.target
                    || target.scope.key() != self.scope
                    || target.source.program != self.program.value_origin
                {
                    return Err(type_error("view reborrow has an inexact invocation target"));
                }
                let old = self.layout(source.identity().ty)?;
                let new = self.layout(target.identity.ty)?;
                if old.declaration != new.declaration
                    || old.arguments != new.arguments
                    || new.read_scope != self.scope
                {
                    return Err(type_error(
                        "view reborrow changes declaration or type actuals",
                    ));
                }
                self.complete_type(
                    target.identity.ty,
                    source.is_shared() || target.sharing,
                    0,
                    control,
                )?;
            }
            AdmissionRequest::Projection {
                source,
                field,
                destination,
                target,
            } => {
                if source.source() != self.selected
                    || destination.program != self.program.value_origin
                    || field.source.as_ref().is_none_or(|anchor| {
                        source
                            .backing_source()
                            .map_or(true, |selected| !same_lineage(anchor, selected))
                    })
                {
                    return Err(type_error(
                        "view projection has an unrelated tagged reference path",
                    ));
                }
                let declared = self.field(source.identity().ty, field.field)?;
                if !matches!(declared.kind, ReadViewFieldKind::ReadReference { .. }) {
                    return Err(type_error("view projection has another field category"));
                }
                if let Some(target) = target {
                    let old = self.layout(field.ty)?;
                    let new = self.layout(target.identity.ty)?;
                    let effective = self.layout(declared.ty)?;
                    if target.identity != self.target
                        || old.declaration != new.declaration
                        || old.arguments != new.arguments
                        || effective.declaration != new.declaration
                        || effective.arguments != new.arguments
                        || new.read_scope != target.scope.key()
                    {
                        return Err(type_error(
                            "view projection changes its exact referent family application",
                        ));
                    }
                    self.complete_type(
                        target.identity.ty,
                        source.is_shared() || target.sharing,
                        0,
                        control,
                    )?;
                } else if declared.ty != field.ty {
                    return Err(type_error(
                        "Owned view projection has another exact referent type",
                    ));
                }
            }
            AdmissionRequest::MetadataResult {
                source,
                field,
                ty,
                value,
            } => {
                if source.source() != self.selected {
                    return Err(type_error("view metadata source changed"));
                }
                let declared = self.field(source.identity().ty, field)?;
                if declared.kind != ReadViewFieldKind::Ordinary || declared.ty != ty {
                    return Err(type_error("view metadata access has another field type"));
                }
                ordinary_shape(self.program, value, ty, 0, control)?;
            }
        }
        Ok(())
    }
}

/// Keep the complete checked packet while transferring its real read leases to backing.
/// A descriptor guard and a projected Owned packet are independent parts of custody.
fn retain_checked_custody<R>(
    value: &CheckedValue,
    builder: &mut storage::ViewBuilder,
    admission: &Admission<'_>,
    origin: super::super::value::ValueOrigin,
    control: &ExecutionControl,
    reserve: &mut R,
) -> Result<(), ExecutionError>
where
    R: FnMut(u64) -> Result<(), ExecutionError>,
{
    let mut retain = |guard: &CheckedValue, reserve: &mut R| -> Result<(), ExecutionError> {
        control.check()?;
        if let Some(packet) = guard.projected_read() {
            builder.reserve_guards(1, control, reserve)?;
            builder.retain_projected(packet, admission, control)?;
        }
        match guard.raw() {
            NormalizedValue::ReadView(view) => {
                builder.reserve_guards(1, control, reserve)?;
                builder.retain_guard(view, admission, control)?;
            }
            raw if raw.memory_form().is_some() => builder.retain_read(
                HeldRead {
                    origin,
                    value: raw.memory_borrow()?,
                },
                admission,
                control,
                reserve,
            )?,
            _ => {
                return Err(type_error(
                    "view source ancestor has no actual retained read lease",
                ));
            }
        }
        Ok(())
    };
    retain(value, reserve)?;
    value.visit_ancestor_reads(control, reserve, &mut retain)
}

/// Follow checked source placements and the exact ordered borrowed-result continuation.
/// No type equality or unrelated local can stand in for the selected source.
fn visit_source_custody(
    frames: &[Frame],
    mut frame_index: usize,
    mut local: u32,
    control: &ExecutionControl,
    visit: &mut impl FnMut(&CheckedValue) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    let mut remaining = frames.iter().try_fold(0usize, |count, frame| {
        control.check()?;
        count
            .checked_add(frame.locals.len())
            .ok_or_else(|| type_error("view source path capacity overflowed"))
    })?;
    loop {
        control.check()?;
        remaining = remaining
            .checked_sub(1)
            .ok_or_else(|| type_error("view source placements contain a cycle"))?;
        let frame = &frames[frame_index];
        let source = frame
            .locals
            .get(local as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| type_error("view source custody lost its exact local"))?;
        visit(source)?;
        for scope in &frame.owned_loans {
            control.check()?;
            if scope.binding_local == local {
                if let Some(parent) = &scope.parent {
                    visit(parent)?;
                }
            }
        }
        // A real owning token already carries complete physical custody. Its
        // recorded source and placement can name a consumed earlier local after
        // an affine move; those identities are not additional live read edges.
        // Retain its checked packet and lexical parent guards before stopping.
        if source.raw().memory_form().is_some() && !source.raw().memory_is_borrowed() {
            return Ok(());
        }
        let root = source.borrow_source();
        if let Some(root) = root.filter(|root| root.frame == frame.id && root.local != local) {
            local = root.local;
            continue;
        }
        let caller = source
            .borrow_placement()
            .filter(|placement| placement.frame != frame.id)
            .or_else(|| {
                frame
                    .borrow_result_target
                    .filter(|target| {
                        root == Some(BorrowSource {
                            frame: frame.id,
                            local: target.position,
                        })
                    })
                    .map(|target| target.source)
            });
        let Some(caller) = caller else {
            return Ok(());
        };
        let mut earlier = None;
        for (index, candidate) in frames[..frame_index].iter().enumerate() {
            control.check()?;
            if candidate.id == caller.frame {
                earlier = Some(index);
                break;
            }
        }
        frame_index = earlier
            .ok_or_else(|| type_error("view source placement lost its caller activation"))?;
        local = caller.local;
    }
}

impl Machine<'_> {
    pub(super) fn extended_read_path(
        &mut self,
        parent: &CheckedValue,
        step: ReadPathStep,
    ) -> Result<Arc<[ReadPathStep]>, ExecutionError> {
        let count = parent
            .read_path()
            .len()
            .checked_add(1)
            .ok_or_else(|| type_error("read path length overflowed"))?;
        if count > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(type_error("read reference path is too deep"));
        }
        self.charge_allocation((count * std::mem::size_of::<ReadPathStep>()) as u64)?;
        let mut path = Vec::new();
        path.try_reserve_exact(count).map_err(|_| {
            resource_error(
                "normalized_view_path_storage",
                "read path reservation failed",
            )
        })?;
        path.extend_from_slice(parent.read_path());
        path.push(step);
        Ok(path.into())
    }

    pub(super) fn lend_owned_read_scope(
        &mut self,
        value: &CheckedValue,
        key: ReadScopeArgument,
        ty: TypeObjectDigest,
    ) -> Result<(storage::RuntimeScopeLease, SourceAnchor), ExecutionError> {
        if !value.raw().memory_is_borrowed() {
            return Err(type_error(
                "joined source scope requires a live Owned read token",
            ));
        }
        let placement = value
            .borrow_placement()
            .ok_or_else(|| type_error("joined source scope has no exact parent placement"))?;
        let (root, source) = self.view_source_anchor(placement.local, key)?;
        let lease = self.view_scope(key, root, &source, ty, None)?;
        let expected = self
            .frames
            .iter()
            .flat_map(|frame| frame.view_scopes.iter())
            .find(|scope| scope.key == key && scope.root == root)
            .ok_or_else(|| type_error("joined source scope lost its admitted root mapping"))?
            .lease
            .source()
            .clone();
        Ok((lease, expected))
    }

    pub(super) fn owned_read_scope_placement(
        &mut self,
        value: &CheckedValue,
        key: ReadScopeArgument,
    ) -> Result<SourceAnchor, ExecutionError> {
        let placement = value
            .borrow_placement()
            .ok_or_else(|| type_error("joined Owned source lacks exact ordered placement"))?;
        if placement.frame != self.current_frame()?.id {
            return Err(type_error(
                "joined source placement escaped its current activation",
            ));
        }
        value.raw().memory_validate(self.memory_domain, false)?;
        self.view_source_anchor(placement.local, key)
            .map(|(_, source)| source)
    }

    pub(super) fn validate_call_scope_arguments(
        &self,
        index: FunctionIndex,
        arguments: &[ReadScopeArgument],
    ) -> Result<(), ExecutionError> {
        let function = self
            .program
            .functions
            .get(index.0 as usize)
            .filter(|_| index.1 == self.program.value_origin)
            .ok_or_else(|| {
                runtime_error(
                    "normalized_function_index",
                    "normalized function index escaped the prepared table",
                )
            })?;
        if function.read_scope_parameters.len() != arguments.len()
            || function.read_scope_arguments.as_ref() != arguments
        {
            return Err(type_error(
                "call scope arguments differ from the exact prepared application",
            ));
        }
        Ok(())
    }

    fn applied_scope(
        &self,
        function: &super::super::prepare::NormalizedFunction,
        scope: ReadScopeArgument,
    ) -> Result<ReadScopeArgument, ExecutionError> {
        match scope {
            ReadScopeArgument::Parameter(reference)
                if reference.binder == ReadScopeBinder::Declaration(function.declaration) =>
            {
                let position = function
                    .read_scope_parameters
                    .iter()
                    .position(|formal| formal.id == reference.parameter)
                    .ok_or_else(|| type_error("result scope formal is undeclared"))?;
                function
                    .read_scope_arguments
                    .get(position)
                    .copied()
                    .ok_or_else(|| type_error("result scope actual is missing"))
            }
            ReadScopeArgument::Parameter(_) | ReadScopeArgument::Lexical { .. }
                if function.read_scope_arguments.contains(&scope) =>
            {
                Ok(scope)
            }
            ReadScopeArgument::Lexical { callable, .. } if callable == function.declaration => {
                Ok(scope)
            }
            _ => Err(type_error("scope escaped its exact callable binder")),
        }
    }

    pub(super) fn bind_call_view_scopes(
        &mut self,
        index: FunctionIndex,
        arguments: &[CheckedValue],
        child_frame: u64,
    ) -> Result<Vec<Scope>, ExecutionError> {
        let program = self.program;
        let function = program
            .functions
            .get(index.0 as usize)
            .filter(|_| index.1 == program.value_origin)
            .ok_or_else(|| type_error("view scope target is foreign"))?;
        if function.read_scope_parameters.len() != function.read_scope_arguments.len() {
            return Err(type_error("call omitted a declared scope actual"));
        }
        let count = function
            .read_scope_sources
            .len()
            .checked_add(function.read_scope_parameters.len())
            .ok_or_else(|| type_error("scope inventory size overflowed"))?;
        self.charge_allocation((count * std::mem::size_of::<Scope>()) as u64)?;
        let mut aliases = Vec::new();
        aliases.try_reserve_exact(count).map_err(|_| {
            resource_error(
                "normalized_view_scope_storage",
                "invocation scope inventory reservation failed",
            )
        })?;
        for relationship in function.read_scope_sources.iter() {
            self.control.check()?;
            let position = relationship.source;
            let parameter = function
                .parameters
                .get(position as usize)
                .ok_or_else(|| type_error("scope source position is absent"))?;
            let value = arguments
                .get(position as usize)
                .ok_or_else(|| type_error("scope source argument is absent"))?;
            if parameter.use_mode != ParameterUse::Borrow
                || parameter.resource_requirement.is_some()
            {
                return Err(type_error(
                    "scope source is not an exact borrowed parameter",
                ));
            }
            let key = self.applied_scope(function, relationship.scope)?;
            let lease = if let NormalizedValue::ReadView(view) = value.raw() {
                if !value.is_view_borrowed() || view.scope()?.key() != key {
                    return Err(type_error(
                        "view scope argument differs from its certified source validity",
                    ));
                }
                view.scope()?.reborrow(self.control)?
            } else if let Some(imported) = self.incoming_view_scopes.iter().find(|scope| {
                scope.key == key
                    && scope.root
                        == BorrowSource {
                            frame: child_frame,
                            local: position,
                        }
            }) {
                if !self.frames.is_empty() || !value.raw().memory_is_borrowed() {
                    return Err(type_error("joined Owned scope is outside its sealed entry"));
                }
                imported.lease.reborrow(self.control)?
            } else {
                if !value.raw().memory_is_borrowed() {
                    return Err(type_error("scope source has no real borrowed Owned token"));
                }
                let placement = value
                    .borrow_placement()
                    .ok_or_else(|| type_error("scope source lacks its exact caller position"))?;
                let (root, source) = self.view_source_anchor(placement.local, key)?;
                if root.frame != placement.frame
                    && !self.frames.iter().any(|frame| frame.id == root.frame)
                {
                    return Err(type_error(
                        "scope source caller activation is no longer live",
                    ));
                }
                self.view_scope(key, root, &source, function.result.ty, None)?
            };
            aliases.push(Scope {
                key,
                root: BorrowSource {
                    frame: child_frame,
                    local: position,
                },
                lease,
                owner: None,
            });
        }
        // Unused formals still bind an existing admitted actual scope; absence of a declared
        // source relationship grants no parameter, field or construction provenance.
        for actual in function.read_scope_arguments.iter() {
            if !aliases.iter().any(|alias| alias.key == *actual) {
                let inherited = self
                    .frames
                    .iter()
                    .rev()
                    .flat_map(|frame| frame.view_scopes.iter())
                    .find(|scope| scope.key == *actual)
                    .ok_or_else(|| {
                        type_error("unused scope actual has no admitted caller scope")
                    })?;
                aliases.push(Scope {
                    key: *actual,
                    root: BorrowSource {
                        frame: child_frame,
                        local: u32::MAX,
                    },
                    lease: inherited.lease.reborrow(self.control)?,
                    owner: None,
                });
            }
        }
        Ok(aliases)
    }

    fn view_identity(&self, ty: TypeObjectDigest) -> Result<ViewIdentity, ExecutionError> {
        if self.program.read_view_layout(ty).is_none() {
            return Err(type_error("expected a fully instantiated read-view type"));
        }
        Ok(ViewIdentity {
            ty,
            program: self.program.value_origin,
        })
    }

    fn view_source_anchor(
        &mut self,
        local: u32,
        scope: ReadScopeArgument,
    ) -> Result<(BorrowSource, SourceAnchor), ExecutionError> {
        let frame = self.frames.last().ok_or_else(|| {
            runtime_error(
                "normalized_frame_missing",
                "normalized execution has no active frame",
            )
        })?;
        let value = frame
            .locals
            .get(local as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| type_error("view source is consumed or uninitialized"))?;
        let root = value.borrow_source().unwrap_or(BorrowSource {
            frame: frame.id,
            local,
        });
        if let NormalizedValue::ReadView(view) = value.raw() {
            view.validate(self.control)?;
            return Ok((root, view.source().clone()));
        }
        value.raw().memory_validate(self.memory_domain, false)?;
        value.validate_read_custody(self.control, &mut |bytes| {
            reserve_execution_allocation(
                self.policy,
                self.shared_budget.as_deref(),
                &mut self.observation,
                bytes,
            )
        })?;
        let origin_frame = self
            .frames
            .iter()
            .find(|candidate| candidate.id == root.frame)
            .ok_or_else(|| type_error("view source activation is no longer live"))?;
        let coordinate = if root.local < origin_frame.code.parameter_count {
            SourceCoordinate::Parameter(root.local)
        } else {
            match scope {
                ReadScopeArgument::Lexical { binding, .. } => SourceCoordinate::Binding(binding),
                _ => {
                    return Err(type_error(
                        "local Owned source cannot claim a named outer scope",
                    ));
                }
            }
        };
        let application = origin_frame.application_identity;
        let invocation = origin_frame.invocation_identity;
        let shared = value.shared_read_path();
        let path = if let Some(path) = shared {
            path
        } else {
            self.charge_allocation((2 * std::mem::size_of::<usize>()) as u64)?;
            Arc::from([])
        };
        Ok((
            root,
            SourceAnchor {
                program: self.program.value_origin,
                application,
                invocation,
                coordinate,
                origin: self.memory_domain,
                path,
            },
        ))
    }

    fn view_scope(
        &mut self,
        key: ReadScopeArgument,
        root: BorrowSource,
        source: &SourceAnchor,
        ty: TypeObjectDigest,
        parent: Option<&storage::RuntimeScopeLease>,
    ) -> Result<storage::RuntimeScopeLease, ExecutionError> {
        let prospective = self.current_frame()?.lexical_view_scopes.contains(&key);
        if let Some(scope) = self
            .frames
            .iter()
            .rev()
            .flat_map(|frame| frame.view_scopes.iter())
            .find(|scope| {
                scope.key == key && scope.root == root && (!prospective || scope.owner.is_some())
            })
        {
            return scope.lease.reborrow(self.control);
        }
        if !prospective
            || !matches!(key, ReadScopeArgument::Lexical { callable, .. } if self.current_frame()?.function.is_some_and(|index| self.program.functions[index.0 as usize].declaration == callable))
        {
            return Err(type_error(
                "new read scope has no active exact lexical containment relation",
            ));
        }
        let inherited = if prospective && parent.is_none() {
            self.frames
                .iter()
                .rev()
                .flat_map(|frame| frame.view_scopes.iter())
                .find(|scope| scope.key == key && scope.root == root)
                .map(|scope| scope.lease.reborrow(self.control))
                .transpose()?
        } else {
            None
        };
        let parent = parent.or(inherited.as_ref());
        let admission = Admission {
            program: self.program,
            selected: source,
            target: ViewIdentity {
                ty,
                program: self.program.value_origin,
            },
            scope: key,
            lifetime: source.clone(),
            nodes: std::cell::Cell::new(0),
        };
        let control = self.control;
        let (owner, lease) = storage::RuntimeScopeOwner::open(
            key,
            source,
            parent,
            &admission,
            control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        let retained = lease.reborrow(control)?;
        self.charge_allocation(std::mem::size_of::<Scope>() as u64)?;
        let frame = self
            .frames
            .iter_mut()
            .find(|frame| frame.id == root.frame)
            .ok_or_else(|| type_error("view source scope lost its activation"))?;
        frame.view_scopes.try_reserve(1).map_err(|_| {
            resource_error(
                "normalized_view_scope_storage",
                "view source scope storage reservation failed",
            )
        })?;
        frame.view_scopes.push(Scope {
            key,
            root,
            lease: retained,
            owner: Some(owner),
        });
        Ok(lease)
    }

    pub(super) fn lend_read_view(
        &mut self,
        value: &CheckedValue,
        destination: SourceAnchor,
        ty: TypeObjectDigest,
        sharing: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        let NormalizedValue::ReadView(view) = value.raw() else {
            return Err(type_error("view lending requires an actual descriptor"));
        };
        let identity = self.view_identity(ty)?;
        let key = self
            .program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view target has no exact layout"))?
            .read_scope;
        // The caller supplies only a destination it has independently admitted. The actual
        // ancestor lifetime is copied from the selected live source lease.
        let scope = if key == view.scope()?.key() {
            view.scope()?.reborrow(self.control)?
        } else {
            let root = value
                .borrow_source()
                .ok_or_else(|| type_error("scope narrowing requires exact source provenance"))?;
            self.view_scope(key, root, view.source(), ty, Some(view.scope()?))?
        };
        self.charge_allocation(CheckedValue::borrow_metadata_allocation_bytes())?;
        let admission = Admission {
            program: self.program,
            selected: view.source(),
            target: identity,
            scope: key,
            lifetime: view.scope()?.source().clone(),
            nodes: std::cell::Cell::new(0),
        };
        let target = storage::ViewTarget {
            identity,
            source: destination,
            scope,
            sharing,
        };
        let reservation = storage::ReadView::reserve_descriptor(self.control, &mut |bytes| {
            self.charge_allocation(bytes)
        })?;
        let raw = view.reborrow_as(target, &admission, self.control, reservation)?;
        let mut checked = CheckedValue::read_view_controlled(
            self.program,
            NormalizedValue::ReadView(raw),
            self.control,
        )?;
        checked.copy_read_provenance_from(value);
        checked.set_view_borrowed();
        Ok(checked)
    }

    pub(super) fn borrow_view_local(&mut self, local: u32) -> Result<CheckedValue, ExecutionError> {
        self.charge_allocation(CheckedValue::borrow_metadata_allocation_bytes())?;
        let reservation = storage::ReadView::reserve_descriptor(self.control, &mut |bytes| {
            self.charge_allocation(bytes)
        })?;
        let source = self
            .current_frame()?
            .locals
            .get(local as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| type_error("view local is not live"))?;
        let NormalizedValue::ReadView(view) = source.raw() else {
            return Err(type_error("view local has another category"));
        };
        let identity = view.identity();
        let key = view.scope()?.key();
        let admission = Admission {
            program: self.program,
            selected: view.source(),
            target: identity,
            scope: key,
            lifetime: view.scope()?.source().clone(),
            nodes: std::cell::Cell::new(0),
        };
        let target = storage::ViewTarget {
            identity,
            source: view.source().clone(),
            scope: view.scope()?.reborrow(self.control)?,
            sharing: false,
        };
        let raw = view.reborrow_as(target, &admission, self.control, reservation)?;
        let mut checked = CheckedValue::read_view_controlled(
            self.program,
            NormalizedValue::ReadView(raw),
            self.control,
        )?;
        checked.copy_read_provenance_from(source);
        checked.set_view_borrowed();
        Ok(checked)
    }

    fn collect_view_source_loans(
        &mut self,
        local: u32,
    ) -> Result<storage::SourceLoans, ExecutionError> {
        let source = self
            .current_frame()?
            .locals
            .get(local as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| type_error("view source disappeared"))?;
        let selected = HeldRead {
            origin: self.memory_domain,
            value: source.raw().memory_borrow()?,
        };
        let control = self.control;
        storage::SourceLoans::new(selected, 0, control, &mut |bytes| {
            self.charge_allocation(bytes)
        })
    }

    fn retain_view_source_custody(
        &mut self,
        local: u32,
        builder: &mut storage::ViewBuilder,
        admission: &Admission<'_>,
    ) -> Result<(), ExecutionError> {
        // Only neutral validation and reservation run while frames are detached. No user
        // expression executes here, and every error restores the exact activation inventory.
        let frames = std::mem::take(&mut self.frames);
        let control = self.control;
        let origin = self.memory_domain;
        let result = (|| {
            let index = frames
                .len()
                .checked_sub(1)
                .ok_or_else(|| type_error("view source has no activation"))?;
            visit_source_custody(&frames, index, local, control, &mut |value| {
                retain_checked_custody(value, builder, admission, origin, control, &mut |bytes| {
                    self.charge_allocation(bytes)
                })
            })
        })();
        self.frames = frames;
        result
    }

    pub(super) fn begin_read_view(
        &mut self,
        ty: TypeObjectDigest,
        local: u32,
        fields: Arc<[u32]>,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_frame_type(ty, "view constructor escaped its exact application")?;
        let layout = self
            .program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view constructor has no complete private layout"))?;
        if fields.len() != layout.fields.len()
            || fields
                .iter()
                .any(|field| *field as usize >= layout.fields.len())
        {
            return Err(type_error(
                "view constructor has an incomplete field inventory",
            ));
        }
        for (position, field) in fields.iter().enumerate() {
            if fields[..position].contains(field) {
                return Err(type_error("view constructor initializes a field twice"));
            }
        }
        let key = layout.read_scope;
        let (root, source) = self.view_source_anchor(local, key)?;
        let source_is_view = matches!(
            self.current_frame()?
                .locals
                .get(local as usize)
                .and_then(Option::as_ref)
                .map(CheckedValue::raw),
            Some(NormalizedValue::ReadView(_))
        );
        let parent = match self
            .current_frame()?
            .locals
            .get(local as usize)
            .and_then(Option::as_ref)
            .map(CheckedValue::raw)
        {
            Some(NormalizedValue::ReadView(view)) => Some(view.scope()?.reborrow(self.control)?),
            _ => self
                .current_frame()?
                .locals
                .get(local as usize)
                .and_then(Option::as_ref)
                .and_then(CheckedValue::projected_read)
                .map(|packet| packet.scope()?.reborrow(self.control))
                .transpose()?,
        };
        let scope = self.view_scope(key, root, &source, ty, parent.as_ref())?;
        let lifetime = scope.source().clone();
        let construction = storage::ViewConstruction {
            identity: self.view_identity(ty)?,
            source: source.clone(),
            scope,
            fields: fields.len(),
        };
        let program = self.program;
        let control = self.control;
        let admission = Admission {
            program,
            selected: &source,
            target: construction.identity,
            scope: key,
            lifetime: lifetime.clone(),
            nodes: std::cell::Cell::new(0),
        };
        let guards = fields
            .len()
            .checked_add(1)
            .ok_or_else(|| type_error("view guard count overflowed"))?;
        let mut builder = if source_is_view {
            let borrowed = self.borrow_view_local(local)?;
            let NormalizedValue::ReadView(view) = borrowed.raw() else {
                return Err(type_error("view source changed after protection"));
            };
            storage::ViewBuilder::from_view(
                construction,
                view,
                guards,
                &admission,
                control,
                &mut |bytes| self.charge_allocation(bytes),
            )?
        } else {
            let loans = self.collect_view_source_loans(local)?;
            storage::ViewBuilder::from_owned(
                construction,
                loans,
                guards,
                &admission,
                control,
                &mut |bytes| self.charge_allocation(bytes),
            )?
        };
        self.retain_view_source_custody(local, &mut builder, &admission)?;
        self.charge_allocation(std::mem::size_of::<PendingView>() as u64)?;
        let base = self.stack.len();
        let frame = self.current_frame_mut()?;
        frame.view_builders.try_reserve(1).map_err(|_| {
            resource_error(
                "normalized_view_builder_storage",
                "view constructor stack reservation failed",
            )
        })?;
        frame.view_builders.push(PendingView {
            ty,
            fields,
            base,
            source,
            lifetime,
            root,
            builder,
        });
        Ok(())
    }

    pub(super) fn finish_read_view(
        &mut self,
        ty: TypeObjectDigest,
        fields: Arc<[u32]>,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_frame_type(ty, "view finish escaped its exact application")?;
        let pending = self
            .current_frame()?
            .view_builders
            .last()
            .ok_or_else(|| type_error("view finish has no protected source"))?;
        if pending.ty != ty
            || pending.fields != fields
            || self.stack.len() != pending.base.saturating_add(fields.len())
        {
            return Err(type_error(
                "view constructor operands or field order differ",
            ));
        }
        let values = self.pop_many(fields.len())?;
        let mut pending = self
            .current_frame_mut()?
            .view_builders
            .pop()
            .ok_or_else(|| type_error("view constructor lost its custody"))?;
        let layout = self
            .program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view finish has no exact layout"))?;
        let program = self.program;
        let control = self.control;
        let admission = Admission {
            program,
            selected: &pending.source,
            target: ViewIdentity {
                ty,
                program: program.value_origin,
            },
            scope: layout.read_scope,
            lifetime: pending.lifetime.clone(),
            nodes: std::cell::Cell::new(0),
        };
        for (ordinal, value) in fields.iter().zip(values) {
            let field = &layout.fields[*ordinal as usize];
            match field.kind {
                ReadViewFieldKind::Ordinary => {
                    if value.class(program, &mut self.observation.value_work)? != Class::Free {
                        return Err(type_error(
                            "view ordinary initializer contains affine authority",
                        ));
                    }
                    pending.builder.metadata(
                        field.reference.field,
                        field.ty,
                        value.into_raw(),
                        &admission,
                        control,
                        &mut |bytes| self.charge_allocation(bytes),
                    )?;
                }
                ReadViewFieldKind::ReadReference { .. } => {
                    if value.borrow_source() != Some(pending.root) {
                        return Err(type_error(
                            "view reference initializer selects another argument alias",
                        ));
                    }
                    if !value.read_path().starts_with(&pending.source.path) {
                        return Err(type_error(
                            "view reference initializer does not preserve its selected descendant path",
                        ));
                    }
                    retain_checked_custody(
                        &value,
                        &mut pending.builder,
                        &admission,
                        self.memory_domain,
                        control,
                        &mut |bytes| self.charge_allocation(bytes),
                    )?;
                    let mut source = pending.source.clone();
                    source.path = value
                        .shared_read_path()
                        .unwrap_or_else(|| Arc::clone(&pending.source.path));
                    match value.raw() {
                        NormalizedValue::ReadView(view) => pending.builder.view_reference(
                            field.reference.field,
                            source,
                            view,
                            &admission,
                            control,
                        )?,
                        raw if raw.memory_is_borrowed() => {
                            // Root and ancestor loans already live in the preprotected source packet.
                            pending.builder.owned_reference(
                                field.reference.field,
                                field.ty,
                                source,
                                raw.memory_borrow()?,
                                &admission,
                                control,
                            )?;
                        }
                        _ => {
                            return Err(type_error(
                                "view reference initializer has no live read referent",
                            ));
                        }
                    }
                }
            }
        }
        let raw = pending.builder.finish(&admission, control, &mut |bytes| {
            self.charge_allocation(bytes)
        })?;
        self.charge_allocation(CheckedValue::borrow_metadata_allocation_bytes())?;
        let mut value =
            CheckedValue::read_view_controlled(program, NormalizedValue::ReadView(raw), control)?;
        value.set_borrow_source(Some(pending.root));
        value.set_read_path(Arc::clone(&pending.source.path));
        self.push(value)
    }
}

impl Machine<'_> {
    pub(super) fn begin_read_view_scope(
        &mut self,
        key: ReadScopeArgument,
    ) -> Result<(), ExecutionError> {
        let frame = self.current_frame()?;
        let function = frame
            .function
            .and_then(|index| self.program.functions.get(index.0 as usize))
            .ok_or_else(|| type_error("lexical view scope requires an exact named callable"))?;
        if !matches!(key, ReadScopeArgument::Lexical { callable, .. } if callable == function.declaration)
            || frame.lexical_view_scopes.contains(&key)
        {
            return Err(type_error(
                "prospective view scope has another owner or an overlapping binding",
            ));
        }
        self.charge_allocation(std::mem::size_of::<ReadScopeArgument>() as u64)?;
        let frame = self.current_frame_mut()?;
        frame.lexical_view_scopes.try_reserve(1).map_err(|_| {
            resource_error(
                "normalized_view_scope_storage",
                "prospective view scope reservation failed",
            )
        })?;
        frame.lexical_view_scopes.push(key);
        Ok(())
    }

    pub(super) fn end_read_view_scope(
        &mut self,
        key: ReadScopeArgument,
    ) -> Result<(), ExecutionError> {
        let frame = self.current_frame()?;
        if frame.lexical_view_scopes.last() != Some(&key)
            || !frame.view_builders.is_empty()
            || frame.pending_borrow_call.is_some()
        {
            return Err(type_error(
                "lexical view scope ended before construction or a nested scope completed",
            ));
        }
        if self.stack.last().is_some_and(|value| matches!(value.raw(), NormalizedValue::ReadView(view) if view.scope().is_ok_and(|scope| scope.key() == key))) { return Err(type_error("view cannot escape its initializer's lexical source scope")); }
        let control = self.control;
        let frame = self.current_frame_mut()?;
        for value in &mut frame.locals {
            let release = value.as_ref().is_some_and(|value| match value.raw() {
                NormalizedValue::ReadView(view) => view.scope().is_ok_and(|lease| {
                    frame.view_scopes.iter().any(|scope| {
                        scope.key == key
                            && scope.owner.is_some()
                            && scope.lease.same_live_scope(lease)
                    })
                }),
                _ => false,
            });
            if release {
                drop(value.take());
            }
        }
        control.check()?;
        // Removing the owner invalidates retained child/projection leases at this exact
        // source-scope boundary, independently of when any descriptor local was dropped.
        frame
            .view_scopes
            .retain(|scope| !(scope.key == key && scope.owner.is_some()));
        frame.lexical_view_scopes.pop();
        Ok(())
    }
}

impl Machine<'_> {
    pub(super) fn borrow_read_view(
        &mut self,
        ty: TypeObjectDigest,
        local: u32,
        binding: u32,
        binding_type: TypeObjectDigest,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_frame_type(ty, "view borrow escaped its exact application")?;
        let binding_type =
            self.resolve_frame_type(binding_type, "view binding escaped its exact application")?;
        if ty != binding_type {
            return Err(type_error("view borrow has another exact view type"));
        }
        self.validate_owned_borrow_binding(local, binding)?;
        let parent = self.borrow_view_local(local)?;
        if !matches!(parent.raw(), NormalizedValue::ReadView(view) if view.identity().ty == ty) {
            return Err(type_error("view borrow source type differs"));
        }
        let child = self.borrow_view_local(local)?;
        self.start_owned_borrow(binding, parent, child)
    }

    pub(super) fn reborrow_read_view(
        &mut self,
        ty: TypeObjectDigest,
        local: u32,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_frame_type(ty, "view reborrow escaped its exact application")?;
        let parent = self.borrow_view_local(local)?;
        let source = match parent.raw() {
            NormalizedValue::ReadView(view) => view.source().clone(),
            _ => return Err(type_error("view reborrow source changed")),
        };
        let mut child = self.lend_read_view(&parent, source, ty, false)?;
        child.set_view_owned();
        self.push(child)
    }

    pub(super) fn move_read_view(
        &mut self,
        ty: TypeObjectDigest,
        local: u32,
        discard: bool,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_frame_type(ty, "view move escaped its exact application")?;
        let source = self
            .current_frame()?
            .locals
            .get(local as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| type_error("view move source is consumed"))?;
        if source.is_view_borrowed()
            || !matches!(source.raw(), NormalizedValue::ReadView(view) if view.identity().ty == ty)
        {
            return Err(type_error(
                "view move/drop requires the exact affine owning handle",
            ));
        }
        if let NormalizedValue::ReadView(view) = source.raw() {
            view.validate(self.control)?;
        }
        let child = self.current_frame_mut()?.locals[local as usize]
            .take()
            .ok_or_else(|| type_error("view move source disappeared"))?;
        if discard {
            drop(child);
            self.push_scalar(NormalizedValue::Unit)
        } else {
            self.push(child)
        }
    }

    pub(super) fn read_view_data_field(
        &mut self,
        ty: TypeObjectDigest,
        local: u32,
        ordinal: u32,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_frame_type(ty, "view field escaped its exact application")?;
        let parent = self.borrow_view_local(local)?;
        let NormalizedValue::ReadView(view) = parent.raw() else {
            return Err(type_error("view field source changed"));
        };
        if view.identity().ty != ty {
            return Err(type_error("view field has another exact application"));
        }
        let layout = self
            .program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view field layout is missing"))?;
        let field = layout
            .fields
            .get(ordinal as usize)
            .ok_or_else(|| type_error("view field ordinal is absent"))?;
        if field.kind != ReadViewFieldKind::Ordinary {
            return Err(type_error(
                "ordinary field access cannot delegate a read reference",
            ));
        }
        let admission = Admission {
            program: self.program,
            selected: view.source(),
            target: view.identity(),
            scope: layout.read_scope,
            lifetime: view.scope()?.source().clone(),
            nodes: std::cell::Cell::new(0),
        };
        let read = view.metadata(field.reference.field, &admission, self.control)?;
        let raw = read.raw(self.control)?;
        self.charge_allocation(super::super::value::projected_value_bytes(raw)?)?;
        let checked = self.admit(raw.clone(), field.ty, None, false)?;
        self.push(checked)
    }

    pub(super) fn borrow_read_view_field(
        &mut self,
        ty: TypeObjectDigest,
        local: u32,
        ordinal: u32,
        binding: u32,
        binding_type: TypeObjectDigest,
    ) -> Result<(), ExecutionError> {
        let ty =
            self.resolve_frame_type(ty, "view reference field escaped its exact application")?;
        let binding_type = self.resolve_frame_type(
            binding_type,
            "view reference binding escaped its exact application",
        )?;
        self.validate_owned_borrow_binding(local, binding)?;
        let parent = self.borrow_view_local(local)?;
        let NormalizedValue::ReadView(view) = parent.raw() else {
            return Err(type_error("view reference source changed"));
        };
        if view.identity().ty != ty {
            return Err(type_error("view reference has another exact source type"));
        }
        let layout = self
            .program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view reference layout is missing"))?;
        let field = layout
            .fields
            .get(ordinal as usize)
            .ok_or_else(|| type_error("view reference ordinal is absent"))?;
        if field.ty != binding_type
            || !matches!(field.kind, ReadViewFieldKind::ReadReference { .. })
        {
            return Err(type_error(
                "view reference binding has another type/category",
            ));
        }
        let mut destination = view.source().clone();
        self.charge_allocation(
            ((destination.path.len() + 1) * std::mem::size_of::<ReadPathStep>()) as u64,
        )?;
        let mut path = Vec::new();
        path.try_reserve_exact(destination.path.len() + 1)
            .map_err(|_| {
                resource_error(
                    "normalized_view_path_storage",
                    "view path reservation failed",
                )
            })?;
        path.extend_from_slice(&destination.path);
        path.push(ReadPathStep::ViewReference {
            ty,
            field: field.reference.field,
        });
        destination.path = path.into();
        let program = self.program;
        let control = self.control;
        let target = ViewIdentity {
            ty: binding_type,
            program: program.value_origin,
        };
        let admission = Admission {
            program,
            selected: view.source(),
            target,
            scope: layout.read_scope,
            lifetime: view.scope()?.source().clone(),
            nodes: std::cell::Cell::new(0),
        };
        let mut child = if program.read_view_layout(binding_type).is_some() {
            let target = storage::ViewTarget {
                identity: target,
                source: destination.clone(),
                scope: view.scope()?.reborrow(control)?,
                sharing: false,
            };
            let raw = view.project_view_as(
                field.reference.field,
                target,
                &admission,
                control,
                &mut |bytes| self.charge_allocation(bytes),
            )?;
            let mut checked = CheckedValue::read_view_controlled(
                program,
                NormalizedValue::ReadView(raw),
                control,
            )?;
            checked.set_view_borrowed();
            checked
        } else {
            let packet = view.project_owned(
                field.reference.field,
                destination.clone(),
                &admission,
                control,
            )?;
            if packet.ty() != field.ty {
                return Err(type_error(
                    "projected Owned packet has another exact referent type",
                ));
            }
            let raw = packet.raw(control)?.memory_borrow()?;
            self.charge_allocation(
                (std::mem::size_of::<storage::ProjectedOwnedRead>()
                    + 2 * std::mem::size_of::<usize>()) as u64,
            )?;
            let mut checked = CheckedValue::memory(program, raw)?;
            checked.retain_projected_read(Arc::new(packet));
            checked
        };
        self.charge_allocation(child.borrow_metadata_bytes())?;
        child.set_read_path(destination.path);
        self.start_owned_borrow(binding, parent, child)
    }

    pub(super) fn end_view_borrow(&mut self, binding: u32) -> Result<(), ExecutionError> {
        let frame = self.current_frame()?;
        let scope = frame
            .owned_loans
            .last()
            .filter(|scope| scope.binding_local == binding)
            .ok_or_else(|| type_error("view borrow scopes must end in lexical order"))?;
        if self.stack.len() != scope.stack_base + 1 {
            return Err(type_error("view borrow body has an inexact operand result"));
        }
        // An escaping descriptor owns its complete source/ancestor leases; the local parent
        // can close independently. A borrowed parameter cannot escape without explicit mint.
        if self
            .stack
            .last()
            .is_some_and(CheckedValue::is_view_borrowed)
        {
            return Err(type_error(
                "borrowed view requires an explicit affine reborrow before forwarding",
            ));
        }
        let frame = self.current_frame_mut()?;
        drop(frame.locals[binding as usize].take());
        drop(frame.owned_loans.pop());
        Ok(())
    }

    pub(super) fn return_view(&mut self) -> Result<(), ExecutionError> {
        let frame = self.current_frame()?;
        if !frame.view_builders.is_empty()
            || !frame.lexical_view_scopes.is_empty()
            || !frame.owned_loans.is_empty()
            || frame.pending_borrow_call.is_some()
            || self.stack.len() != frame.stack_base + 1
        {
            return Err(type_error(
                "view return has an unfinished lexical operation",
            ));
        }
        let index = frame
            .function
            .ok_or_else(|| type_error("view return requires an exact named callable"))?;
        let function = &self.program.functions[index.0 as usize];
        let ResultRelation::ViewFrom {
            source: position,
            scope,
        } = function.result.relation
        else {
            return Err(type_error("view return has another result relation"));
        };
        let scope = self.applied_scope(function, scope)?;
        let target = frame
            .borrow_result_target
            .ok_or_else(|| type_error("view return has no scoped caller"))?;
        let value = self
            .stack
            .last()
            .ok_or_else(|| type_error("view result is missing"))?;
        let NormalizedValue::ReadView(view) = value.raw() else {
            return Err(type_error("view result has another category"));
        };
        let ty = self.resolve_frame_type(
            function.result.ty,
            "view result escaped its exact application",
        )?;
        if target.position != position
            || value.is_view_borrowed()
            || value.borrow_source()
                != Some(BorrowSource {
                    frame: frame.id,
                    local: position,
                })
            || view.identity().ty != ty
            || view.scope()?.key() != scope
        {
            return Err(type_error(
                "view result does not use the exact ordered source, scope and affine handle",
            ));
        }
        view.validate(self.control)?;
        if !function.graph_function
            || !matches!(
                function.effect,
                crate::platform::kernel::FunctionEffect::Pure
            )
            || self
                .transactions
                .values()
                .any(|transaction| transaction.owner_frame == frame.id)
        {
            return Err(type_error(
                "view result has an unsupported callable or effect boundary",
            ));
        }
        self.charge_allocation(CheckedValue::borrow_metadata_allocation_bytes())?;
        let mut result = self.pop()?;
        result.set_borrow_packet(target)?;
        self.control.check()?;
        drop(self.frames.pop());
        self.push(result)
    }

    pub(super) fn adopt_view_result(
        &mut self,
        source_local: u32,
        binding: u32,
        binding_type: TypeObjectDigest,
    ) -> Result<(), ExecutionError> {
        self.validate_owned_borrow_binding(source_local, binding)?;
        let ty =
            self.resolve_frame_type(binding_type, "view adoption escaped its exact application")?;
        let frame = self.current_frame()?;
        let target = frame
            .pending_borrow_call
            .as_ref()
            .filter(|pending| {
                pending.dispatched
                    && pending.target.source
                        == BorrowSource {
                            frame: frame.id,
                            local: source_local,
                        }
            })
            .map(|pending| pending.target)
            .ok_or_else(|| type_error("view adoption has no exact waiting invocation"))?;
        self.charge_allocation(CheckedValue::borrow_metadata_allocation_bytes())?;
        let parent = self
            .current_frame_mut()?
            .pending_borrow_call
            .as_mut()
            .and_then(|pending| pending.guard.take())
            .ok_or_else(|| type_error("view adoption lost its preprotected source custody"))?;
        let mut child = self.pop()?;
        if child.take_borrow_packet() != Some(target)
            || child.is_view_borrowed()
            || !matches!(child.raw(), NormalizedValue::ReadView(view) if view.identity().ty == ty)
        {
            return Err(type_error(
                "view adoption result packet is counterfeit or has another type",
            ));
        }
        if let NormalizedValue::ReadView(view) = child.raw() {
            view.validate(self.control)?;
        }
        let key = self
            .program
            .read_view_layout(ty)
            .ok_or_else(|| type_error("view adoption has no exact layout"))?
            .read_scope;
        let destination = match parent.raw() {
            NormalizedValue::ReadView(view) => view.source().clone(),
            _ => self.view_source_anchor(source_local, key)?.1,
        };
        let mut rebound = self.lend_read_view(&child, destination.clone(), ty, false)?;
        rebound.set_view_owned();
        rebound.set_read_path(destination.path);
        child = rebound;
        child.set_borrow_source(parent.borrow_source());
        drop(parent);
        self.current_frame_mut()?.pending_borrow_call = None;
        // The adopted descriptor owns its complete backing/source leases. It
        // remains an affine local for move/drop and compiler-emitted cleanup;
        // borrowed binding scopes alone require an owned_loans entry.
        let frame = self.current_frame_mut()?;
        let destination = frame
            .locals
            .get_mut(binding as usize)
            .ok_or_else(|| type_error("view binding local is absent"))?;
        if destination.is_some() {
            return Err(type_error("view binding became occupied before adoption"));
        }
        *destination = Some(child);
        Ok(())
    }
}

fn ordinary_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    depth: usize,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    let mut ancestors = [None; crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH + 1];
    let mut nodes = 0usize;
    ordinary_type_in(program, ty, depth, control, &mut ancestors, &mut nodes)
}

fn ordinary_type_in(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    depth: usize,
    control: &ExecutionControl,
    ancestors: &mut [Option<TypeObjectDigest>;
             crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH + 1],
    nodes: &mut usize,
) -> Result<(), ExecutionError> {
    control.check()?;
    *nodes = nodes
        .checked_add(1)
        .filter(|nodes| *nodes as u64 <= super::super::value::MAXIMUM_ADMISSION_ITEMS)
        .ok_or_else(|| {
            resource_error(
                "normalized_view_admission_work",
                "view ordinary type admission exceeds finite capacity",
            )
        })?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(type_error("view ordinary field type is too deep"));
    }
    if ancestors[..depth].contains(&Some(ty)) {
        return Ok(());
    }
    ancestors[depth] = Some(ty);
    if !program.ordinary_types.contains(&ty) || program.read_view_instances.contains_key(&ty) {
        return Err(type_error("view ordinary field contains live authority"));
    }
    match &program
        .types
        .get(&ty)
        .ok_or_else(|| type_error("view field type is missing"))?
        .form
    {
        TypeForm::Unit
        | TypeForm::Bool
        | TypeForm::I64
        | TypeForm::F64
        | TypeForm::Bytes
        | TypeForm::Text
        | TypeForm::StaticText => {}
        TypeForm::List { item } | TypeForm::Option { item } => {
            ordinary_type_in(program, *item, depth + 1, control, ancestors, nodes)?
        }
        TypeForm::Map { key, value } => {
            ordinary_type_in(program, *key, depth + 1, control, ancestors, nodes)?;
            ordinary_type_in(program, *value, depth + 1, control, ancestors, nodes)?;
        }
        TypeForm::Result { ok, error } => {
            ordinary_type_in(program, *ok, depth + 1, control, ancestors, nodes)?;
            ordinary_type_in(program, *error, depth + 1, control, ancestors, nodes)?;
        }
        TypeForm::StructuralRecord { fields } => {
            for field in fields {
                ordinary_type_in(program, field.ty, depth + 1, control, ancestors, nodes)?;
            }
        }
        TypeForm::Named { .. } | TypeForm::Applied { .. } => {
            if let Some(index) = program.record_instances.get(&ty) {
                let layout = &program.records[index.0 as usize];
                for argument in layout.arguments.iter() {
                    ordinary_type_in(program, *argument, depth + 1, control, ancestors, nodes)?;
                }
                for field in layout.fields.iter() {
                    ordinary_type_in(program, field.ty, depth + 1, control, ancestors, nodes)?;
                }
            } else if let Some(index) = program.variant_instances.get(&ty) {
                let layout = &program.variants[index.0 as usize];
                for argument in layout.arguments.iter() {
                    ordinary_type_in(program, *argument, depth + 1, control, ancestors, nodes)?;
                }
                for case in layout.cases.iter() {
                    if let Some(payload) = case.payload {
                        ordinary_type_in(program, payload, depth + 1, control, ancestors, nodes)?;
                    }
                }
            } else {
                return Err(type_error(
                    "view field has no complete ordinary nominal layout",
                ));
            }
        }
        _ => {
            return Err(type_error(
                "view ordinary field has no positive data category",
            ));
        }
    }
    Ok(())
}

fn ordinary_shape(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    depth: usize,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    let mut nodes = 0usize;
    ordinary_shape_in(program, value, ty, depth, control, &mut nodes)
}

fn ordinary_shape_in(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    depth: usize,
    control: &ExecutionControl,
    nodes: &mut usize,
) -> Result<(), ExecutionError> {
    *nodes = nodes
        .checked_add(1)
        .filter(|nodes| *nodes as u64 <= super::super::value::MAXIMUM_ADMISSION_ITEMS)
        .ok_or_else(|| {
            resource_error(
                "normalized_view_admission_work",
                "view metadata admission exceeds finite capacity",
            )
        })?;
    ordinary_type(program, ty, depth, control)?;
    let object = &program.types[&ty];
    match (value, &object.form) {
        (NormalizedValue::Unit, TypeForm::Unit)
        | (NormalizedValue::Bool(_), TypeForm::Bool)
        | (NormalizedValue::I64(_), TypeForm::I64)
        | (NormalizedValue::F64(_), TypeForm::F64)
        | (NormalizedValue::Bytes(_), TypeForm::Bytes)
        | (NormalizedValue::Text(_), TypeForm::Text)
        | (NormalizedValue::StaticText(_), TypeForm::StaticText) => {}
        (NormalizedValue::Option(value), TypeForm::Option { item }) => {
            if let Some(value) = value {
                ordinary_shape_in(program, value, *item, depth + 1, control, nodes)?;
            }
        }
        (NormalizedValue::Result { success, value }, TypeForm::Result { ok, error }) => {
            ordinary_shape_in(
                program,
                value,
                if *success { *ok } else { *error },
                depth + 1,
                control,
                nodes,
            )?
        }
        (NormalizedValue::List(values), TypeForm::List { item }) => {
            for value in values.iter() {
                ordinary_shape_in(program, value, *item, depth + 1, control, nodes)?;
            }
        }
        (
            NormalizedValue::Map(values),
            TypeForm::Map {
                key: key_ty,
                value: item,
            },
        ) => {
            let key_form = &program
                .types
                .get(key_ty)
                .ok_or_else(|| type_error("view metadata map key type is absent"))?
                .form;
            for (key, value) in values.iter() {
                if !matches!(
                    (key, key_form),
                    (NormalizedMapKey::Bool(_), TypeForm::Bool)
                        | (NormalizedMapKey::I64(_), TypeForm::I64)
                        | (NormalizedMapKey::Bytes(_), TypeForm::Bytes)
                        | (
                            NormalizedMapKey::Text(_),
                            TypeForm::Text | TypeForm::StaticText
                        )
                ) {
                    return Err(type_error(
                        "view metadata map key has another declared scalar type",
                    ));
                }
                ordinary_shape_in(program, value, *item, depth + 1, control, nodes)?;
            }
        }
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
            TypeForm::StructuralRecord { fields },
        ) => {
            if values.len() != fields.len() {
                return Err(type_error("view metadata record shape differs"));
            }
            for ((name, value), field) in values.iter().zip(fields) {
                if name != &field.name {
                    return Err(type_error("view metadata field identity differs"));
                }
                ordinary_shape_in(program, value, field.ty, depth + 1, control, nodes)?;
            }
        }
        (
            NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }),
            TypeForm::Named { .. } | TypeForm::Applied { .. },
        ) => {
            if program.record_instances.get(&ty) != Some(layout) {
                return Err(type_error("view metadata nominal identity differs"));
            }
            let schema = &program.records[layout.0 as usize];
            if fields.len() != schema.fields.len() {
                return Err(type_error("view metadata field inventory differs"));
            }
            for (value, field) in fields.iter().zip(schema.fields.iter()) {
                ordinary_shape_in(program, value, field.ty, depth + 1, control, nodes)?;
            }
        }
        (
            NormalizedValue::Variant {
                layout,
                case,
                payload,
            },
            TypeForm::Named { .. } | TypeForm::Applied { .. },
        ) => {
            if program.variant_instances.get(&ty) != Some(layout) {
                return Err(type_error("view metadata variant identity differs"));
            }
            let schema = program.variants[layout.0 as usize]
                .cases
                .get(*case as usize)
                .ok_or_else(|| type_error("view metadata case is missing"))?;
            match (payload, schema.payload) {
                (None, None) => {}
                (Some(value), Some(ty)) => {
                    ordinary_shape_in(program, value, ty, depth + 1, control, nodes)?
                }
                _ => return Err(type_error("view metadata payload differs")),
            }
        }
        _ => {
            return Err(type_error(
                "view ordinary initializer shape differs from its declared data type",
            ));
        }
    }
    Ok(())
}
