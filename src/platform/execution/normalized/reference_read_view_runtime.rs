//! Independently checked adapters from canonical read meaning to neutral descriptor custody.
//! No dense compiler identity or production admission certificate participates in this proof.

use super::super::read_view_storage::{
    AdmissionRequest, FieldAdmission, FieldKind, ReadViewAdmission, SourceAnchor,
};
use super::*;
use crate::platform::kernel::{ReadViewFieldKind, ValueCategory};

#[path = "reference_read_view_scopes.rs"]
mod scopes;
pub(super) use scopes::ScopeFrame;

/// A checked source permission for one operation. It is never canonical type metadata.
/// The state supplies this only after admitting the exact source local/application.
pub(super) struct RuntimeAdmission<'a> {
    schema: &'a BoundReferenceSchema,
    source: &'a SourceAnchor,
    scope: ReadScopeArgument,
    destination: Option<&'a SourceAnchor>,
    control: &'a ExecutionControl,
}

impl<'a> RuntimeAdmission<'a> {
    pub(super) fn new(
        schema: &'a BoundReferenceSchema,
        source: &'a SourceAnchor,
        scope: ReadScopeArgument,
        control: &'a ExecutionControl,
    ) -> Self {
        Self {
            schema,
            source,
            scope,
            destination: None,
            control,
        }
    }

    /// The exact ordered argument/result admission supplies both coordinates.
    /// This explicit envelope is the sole route that can rebase an invocation.
    pub(super) fn rebase(
        schema: &'a BoundReferenceSchema,
        source: &'a SourceAnchor,
        destination: &'a SourceAnchor,
        scope: ReadScopeArgument,
        control: &'a ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        if source.program != destination.program
            || source.origin != destination.origin
            || (!destination.path.is_empty() && source.path.as_ref() != destination.path.as_ref())
        {
            return Err(failure(
                "rebasing changed the actual referent or tagged source path",
            ));
        }
        Ok(Self {
            schema,
            source,
            scope,
            destination: Some(destination),
            control,
        })
    }

    pub(super) fn joined(
        schema: &'a BoundReferenceSchema,
        source: &'a SourceAnchor,
        destination: &'a SourceAnchor,
        scope: ReadScopeArgument,
        control: &'a ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        if source.program != destination.program
            || !destination.path.is_empty()
            || source.invocation == destination.invocation
        {
            return Err(failure("joined read lacks a fresh ordered child anchor"));
        }
        Ok(Self {
            schema,
            source,
            scope,
            destination: Some(destination),
            control,
        })
    }

    fn proof(&self) -> Result<&read_view_admission::Proof, ExecutionError> {
        self.schema
            .read_view_proof
            .as_deref()
            .ok_or_else(|| failure("raw canonical view admission is missing"))
    }

    fn source(&self, source: &SourceAnchor, accessible: bool) -> Result<(), ExecutionError> {
        let admitted = std::iter::once(self.source)
            .chain(self.destination)
            .any(|anchor| {
                source.program == self.schema.value_origin
                    && source.program == anchor.program
                    && source.application == anchor.application
                    && source.invocation == anchor.invocation
                    && source.coordinate == anchor.coordinate
                    && source.origin == anchor.origin
                    && source.path.starts_with(&anchor.path)
                    && (accessible || source.path.as_ref() == anchor.path.as_ref())
            });
        if !admitted {
            return Err(failure(
                "read view selects another invocation, source argument or tagged path",
            ));
        }
        Ok(())
    }

    fn layout(&self, ty: TypeObjectDigest) -> Result<&read_view_admission::Layout, ExecutionError> {
        let layout = self.proof()?.prepared_layout(ty)?;
        if layout.read_scope != self.scope {
            return Err(failure(
                "view scope differs from the independently selected source scope",
            ));
        }
        Ok(layout)
    }

    fn field(
        &self,
        view_ty: TypeObjectDigest,
        field: crate::platform::semantic_id::FieldId,
        ty: TypeObjectDigest,
        kind: FieldKind,
    ) -> Result<(), ExecutionError> {
        let layout = self.layout(view_ty)?;
        let declared = layout
            .fields
            .iter()
            .find(|candidate| candidate.id == field)
            .ok_or_else(|| {
                failure("view field is absent from its complete canonical declaration")
            })?;
        let category = self.proof()?.prepared_category(ty)?;
        let expected = match declared.kind {
            ReadViewFieldKind::Ordinary => FieldKind::Metadata,
            ReadViewFieldKind::ReadReference { read_scope } if read_scope == layout.read_scope => {
                match category {
                    ValueCategory::Owned => FieldKind::OwnedReference,
                    ValueCategory::ReadView => FieldKind::ViewReference,
                    _ => {
                        return Err(failure(
                            "view reference field has a foreign referent category",
                        ));
                    }
                }
            }
            _ => {
                return Err(failure(
                    "view field forwards a different canonical source scope",
                ));
            }
        };
        let exact_type = if kind == FieldKind::ViewReference && declared.ty != ty {
            let expected = self.proof()?.prepared_layout(declared.ty)?;
            let actual = self.proof()?.prepared_layout(ty)?;
            expected.declaration == actual.declaration && expected.arguments == actual.arguments
        } else {
            declared.ty == ty
        };
        if !exact_type || expected != kind {
            return Err(failure(
                "view field differs from its exact substituted canonical type or kind",
            ));
        }
        Ok(())
    }
}

impl ReadViewAdmission for RuntimeAdmission<'_> {
    fn validate(
        &self,
        request: AdmissionRequest<'_>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        // Meaning admission remains bound to this invocation's cancellation
        // domain even if neutral storage is given a different control object.
        self.control.check()?;
        control.check()?;
        match request {
            AdmissionRequest::Scope {
                key,
                source,
                parent,
            } => {
                self.source(source, false)?;
                if key != self.scope {
                    return Err(failure("source scope was replaced"));
                }
                if let ReadScopeArgument::Parameter(reference) = key {
                    if !self
                        .proof()?
                        .scope_parameters(reference.binder)?
                        .iter()
                        .any(|formal| formal.id == reference.parameter)
                    {
                        return Err(failure("source scope names a foreign binder formal"));
                    }
                }
                // RuntimeScopeOwner separately checks the live parent prefix. Naming a
                // symbolic parameter alone creates no permission or scope containment.
                if let Some(parent) = parent {
                    if parent.source().program != source.program {
                        return Err(failure("source scope forwards another preparation"));
                    }
                }
            }
            AdmissionRequest::Source {
                construction,
                loans,
            } => {
                self.source(&construction.source, false)?;
                let layout = self.layout(construction.identity.ty)?;
                if construction.fields != layout.fields.len() || loans.reads().is_empty() {
                    return Err(failure("source/view inventory was erased"));
                }
                for read in loans.reads() {
                    control.check()?;
                    CheckedValue::validate_memory_admission(&read.value, self.schema.value_origin)?;
                    read.value.memory_validate(read.origin, false)?;
                }
            }
            AdmissionRequest::Subview {
                construction,
                source,
            } => {
                self.source(&construction.source, false)?;
                self.source(source.source(), true)?;
                source.validate(control)?;
                if self.layout(construction.identity.ty)?.fields.len() != construction.fields {
                    return Err(failure("subview field inventory changed"));
                }
            }
            AdmissionRequest::Field {
                construction,
                input,
            } => {
                self.source(&construction.source, false)?;
                match input {
                    FieldAdmission::Metadata { field, ty, value } => {
                        self.field(construction.identity.ty, field, ty, FieldKind::Metadata)?;
                        if !self.proof()?.prepared_metadata(ty)? {
                            return Err(failure(
                                "view metadata contains authority or a nontransferable occurrence",
                            ));
                        }
                        value.reject_read_view_escape(control)?;
                        // Reference checked metadata construction admits the complete raw
                        // representation before calling this adapter.
                    }
                    FieldAdmission::Owned {
                        field,
                        ty,
                        source,
                        value,
                    } => {
                        self.field(
                            construction.identity.ty,
                            field,
                            ty,
                            FieldKind::OwnedReference,
                        )?;
                        self.source(source, true)?;
                        CheckedValue::validate_memory_admission(value, self.schema.value_origin)?;
                        value.memory_validate(source.origin, false)?;
                    }
                    FieldAdmission::View {
                        field,
                        ty,
                        source,
                        value,
                    } => {
                        self.field(
                            construction.identity.ty,
                            field,
                            ty,
                            FieldKind::ViewReference,
                        )?;
                        self.source(source, true)?;
                        self.source(value.source(), true)?;
                        value.validate(control)?;
                    }
                }
            }
            AdmissionRequest::Finish {
                construction,
                fields,
            } => {
                self.source(&construction.source, false)?;
                let layout = self.layout(construction.identity.ty)?;
                if fields.len() != layout.fields.len() {
                    return Err(failure("complete view fields were omitted"));
                }
                for (actual, declared) in fields.iter().zip(&layout.fields) {
                    control.check()?;
                    let descriptor = actual.descriptor();
                    if descriptor.field != declared.id {
                        return Err(failure("view field authored order changed"));
                    }
                    self.field(
                        construction.identity.ty,
                        descriptor.field,
                        descriptor.ty,
                        descriptor.kind,
                    )?;
                    if let Some(source) = &descriptor.source {
                        self.source(source, true)?;
                    }
                }
            }
            AdmissionRequest::Retain {
                construction,
                guard,
            } => {
                self.source(&construction.source, false)?;
                guard.validate(control)?;
                // This packet came from the independently checked ancestor
                // closure. Passive retention conveys no readable field/path;
                // old return aliases keep their original invocation seals.
                if guard.identity().program != self.schema.value_origin {
                    return Err(failure("foreign descriptor ancestor custody"));
                }
            }
            AdmissionRequest::RetainProjected {
                construction,
                guard,
            } => {
                self.source(&construction.source, false)?;
                guard.raw(control)?;
                if guard.source().program != self.schema.value_origin {
                    return Err(failure("foreign projected ancestor custody"));
                }
            }
            AdmissionRequest::RetainRead { construction, read } => {
                self.source(&construction.source, false)?;
                CheckedValue::validate_memory_admission(&read.value, self.schema.value_origin)?;
                read.value.memory_validate(read.origin, false)?;
            }
            AdmissionRequest::Reborrow { source, target } => {
                self.source(source.source(), true)?;
                self.source(&target.source, true)?;
                let original = self.proof()?.prepared_layout(source.identity().ty)?;
                let selected = self.layout(target.identity.ty)?;
                if original.declaration != selected.declaration
                    || original.arguments != selected.arguments
                {
                    return Err(failure(
                        "reborrow changed the descriptor representation or phantom actuals",
                    ));
                }
                if (target.sharing || source.is_shared())
                    && !self.proof()?.prepared_read_shareable(target.identity.ty)?
                {
                    return Err(failure(
                        "complete accessible reference closure is not ReadShareable",
                    ));
                }
            }
            AdmissionRequest::Projection {
                source,
                field,
                destination,
                target,
            } => {
                self.source(source.source(), true)?;
                self.source(destination, true)?;
                self.field(source.identity().ty, field.field, field.ty, field.kind)?;
                if field.kind == FieldKind::Metadata || field.source.is_none() {
                    return Err(failure(
                        "passive source custody is not a readable view field",
                    ));
                }
                // The path crosses this exact accessible declaration field. The contained
                // source path remains separate from CustodyOnly ancestor links.
                if destination.path.last()
                    != Some(
                        &super::super::read_view_storage::ReadPathStep::ViewReference {
                            ty: source.identity().ty,
                            field: field.field,
                        },
                    )
                {
                    return Err(failure(
                        "view projection lacks its exact tagged accessible edge",
                    ));
                }
                if let Some(target) = target {
                    let effective = self.layout(source.identity().ty)?;
                    if effective
                        .fields
                        .iter()
                        .find(|declared| declared.id == field.field)
                        .is_none_or(|declared| declared.ty != target.identity.ty)
                    {
                        return Err(failure(
                            "projection target differs from the exact substituted view referent",
                        ));
                    }
                    let actual = self.layout(target.identity.ty)?;
                    let declared = self.proof()?.prepared_layout(field.ty)?;
                    if actual.declaration != declared.declaration
                        || actual.arguments != declared.arguments
                    {
                        return Err(failure(
                            "view field projection changed its nominal referent",
                        ));
                    }
                    if (target.sharing || source.is_shared())
                        && !self.proof()?.prepared_read_shareable(target.identity.ty)?
                    {
                        return Err(failure(
                            "shared projection inherits an unshareable effective referent closure",
                        ));
                    }
                }
            }
            AdmissionRequest::MetadataResult {
                source,
                field,
                ty,
                value,
            } => {
                self.source(source.source(), true)?;
                self.field(source.identity().ty, field, ty, FieldKind::Metadata)?;
                value.reject_read_view_escape(control)?;
            }
        }
        self.control.check()?;
        control.check()
    }
}

fn failure(message: impl Into<String>) -> ExecutionError {
    reference_error("normalized_reference_read_view", message)
}

impl ReferenceState<'_> {
    fn view_identity(
        &mut self,
        declaration: DeclarationReference,
        arguments: &[TypeObjectDigest],
        scope: ReadScopeArgument,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.control.check()?;
        self.charge_items(arguments.len(), std::mem::size_of::<TypeObjectDigest>())?;
        self.charge_allocation(
            (std::mem::size_of::<crate::platform::kernel::TypeObject>() + 128) as u64,
        )?;
        let object = crate::platform::kernel::TypeObject::new(TypeForm::ReadViewApplied {
            declaration,
            arguments: arguments.to_vec(),
            read_scope: scope,
        })
        .map_err(|_| failure("invalid canonical view application"))?;
        let (ty, _) = crate::platform::kernel::encode_type_object(&object)
            .map_err(|_| failure("invalid canonical view application bytes"))?;
        if self.schema.types.get(&ty) != Some(&object) {
            return Err(failure(
                "exact scope/type view application was not independently prepared",
            ));
        }
        self.schema
            .read_view_proof
            .as_ref()
            .ok_or_else(|| failure("view proof unavailable"))?
            .prepared_layout(ty)?;
        Ok(ty)
    }

    fn view_source_anchor(
        &self,
        local: LocalValueReference,
        value: &CheckedValue,
    ) -> Result<SourceAnchor, ExecutionError> {
        if let Some(anchor) = value.source_anchor() {
            if anchor.program != self.schema.value_origin {
                return Err(failure("foreign source anchor"));
            }
            return Ok(anchor.clone());
        }
        self.read_scope_frames
            .last()
            .ok_or_else(|| failure("view source lacks a canonical invocation"))?
            .source_anchor(local, self.memory_domain)
    }

    pub(super) fn share_parallel_view(
        &mut self,
        value: &CheckedValue,
        destination: SourceAnchor,
    ) -> Result<CheckedValue, ExecutionError> {
        let NormalizedValue::ReadView(view) = value.raw() else {
            return Err(failure("joined view input is not an affine descriptor"));
        };
        let proof = self
            .schema
            .read_view_proof
            .as_ref()
            .ok_or_else(|| failure("joined view lacks independent raw proof"))?;
        if !proof.prepared_read_shareable(view.identity().ty)? {
            return Err(failure(
                "joined view has an unshareable accessible reference closure",
            ));
        }
        let schema = Arc::clone(&self.schema);
        let admission = RuntimeAdmission::joined(
            &schema,
            view.source(),
            &destination,
            view.scope()?.key(),
            self.control,
        )?;
        let borrowed = view.reborrow(destination.clone(), true, &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
        CheckedValue::view(&schema, borrowed, true, self.control)
    }

    pub(super) fn evaluate_read_view_operation(
        &mut self,
        operation: ExpressionOperation,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        use super::super::read_view_storage::{
            HeldRead, ReadPathStep, SourceLoans, ViewBuilder, ViewConstruction, ViewIdentity,
            ViewTarget,
        };
        match operation {
            ExpressionOperation::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                let type_arguments = self.resolve_type_arguments(&type_arguments)?;
                let scope = self.resolve_read_scope_arguments(&[read_scope])?[0];
                let ty = self.view_identity(declaration, &type_arguments, scope)?;
                let schema = Arc::clone(&self.schema);
                let proof = schema
                    .read_view_proof
                    .as_ref()
                    .ok_or_else(|| failure("view raw proof missing"))?;
                let layout = proof.prepared_layout(ty)?;
                let owner = self.declaration(declaration)?;
                let callable = self
                    .read_scope_frames
                    .last()
                    .and_then(ScopeFrame::declaration)
                    .ok_or_else(|| failure("constructor lacks a canonical defining callable"))?;
                if callable.package != declaration.package
                    || self.declaration(callable)?.module != owner.module
                {
                    return Err(failure(
                        "view construction lacks the exact defining module authority",
                    ));
                }
                if fields.len() != layout.fields.len()
                    || fields
                        .iter()
                        .zip(&layout.fields)
                        .any(|(field, declared)| field.field != declared.id)
                {
                    return Err(failure(
                        "constructor must preserve the complete authored view field inventory",
                    ));
                }
                let source_value = locals
                    .get(&source)
                    .ok_or_else(|| failure("view source is not one live canonical local"))?;
                let anchor = self.view_source_anchor(source, source_value)?;
                let provenance = source_value.provenance().unwrap_or(source);
                let lease = self.ensure_view_source_scope(scope, source_value, &anchor)?;
                let inherited = source_value
                    .source_scope()
                    .is_some_and(|existing| existing.same_live_scope(&lease));
                if !inherited && lease.source() != &anchor {
                    return Err(failure("constructor selects another actual source scope"));
                }
                let admission = RuntimeAdmission::new(&schema, &anchor, scope, self.control);
                let construction = ViewConstruction {
                    identity: ViewIdentity {
                        ty,
                        program: schema.value_origin,
                    },
                    source: anchor.clone(),
                    scope: lease,
                    fields: fields.len(),
                };
                let control = self.control;
                // Protect the designated source before evaluating metadata or any reference
                // initializer, even for a zero-field descriptor.
                let mut builder = match source_value.raw() {
                    NormalizedValue::ReadView(view) => {
                        view.validate(control)?;
                        let mut builder = ViewBuilder::from_view(
                            construction,
                            view,
                            fields.len(),
                            &admission,
                            control,
                            &mut |bytes| self.charge_allocation(bytes),
                        )?;
                        // The raw descriptor and the old checked return packet
                        // may have distinct ancestors. Both remain custody.
                        source_value.retain_flat_read_custody(
                            anchor.origin,
                            &mut builder,
                            &admission,
                            control,
                            &mut |bytes| self.charge_allocation(bytes),
                        )?;
                        builder
                    }
                    raw if raw.memory_form().is_some() => {
                        raw.memory_validate(self.memory_domain, false)?;
                        let selected = HeldRead {
                            origin: self.memory_domain,
                            value: raw.memory_borrow()?,
                        };
                        let (ancestors, packets) = source_value.flatten_read_custody(
                            self.memory_domain,
                            control,
                            &mut |bytes| self.charge_allocation(bytes),
                        )?;
                        let mut loans =
                            SourceLoans::new(selected, ancestors.len(), control, &mut |bytes| {
                                self.charge_allocation(bytes)
                            })?;
                        for ancestor in ancestors {
                            loans.retain_ancestor(ancestor, control)?;
                        }
                        let mut builder = ViewBuilder::from_owned(
                            construction,
                            loans,
                            packets.len().saturating_add(fields.len()),
                            &admission,
                            control,
                            &mut |bytes| self.charge_allocation(bytes),
                        )?;
                        for packet in packets {
                            builder.retain_projected(&packet, &admission, control)?;
                        }
                        builder
                    }
                    _ => {
                        return Err(failure(
                            "designated view source must be admitted Owned or a read-view",
                        ));
                    }
                };
                source_value.retain_view_custody(
                    &mut builder,
                    &admission,
                    control,
                    &mut |bytes| self.charge_allocation(bytes),
                )?;
                self.read_source_demand.push(provenance);
                let result = (|| {
                    for (initializer, field) in fields.into_iter().zip(&layout.fields) {
                        control.check()?;
                        match field.kind {
                            ReadViewFieldKind::Ordinary => {
                                let value = self.evaluate(initializer.value, locals)?;
                                if value.ownership(&schema, &mut self.observation.value_work)?
                                    != Ownership::Ordinary
                                {
                                    return Err(failure(
                                        "ordinary view metadata conceals affine authority",
                                    ));
                                }
                                let admitted = self.admit_raw(
                                    value.release(),
                                    field.ty,
                                    &BTreeMap::new(),
                                    None,
                                    false,
                                )?;
                                builder.metadata(
                                    field.id,
                                    field.ty,
                                    admitted.release(),
                                    &admission,
                                    control,
                                    &mut |bytes| self.charge_allocation(bytes),
                                )?;
                            }
                            ReadViewFieldKind::ReadReference { .. } => {
                                let value = if matches!(
                                    self.read_expression(initializer.value)?,
                                    ExpressionOperation::Local { .. }
                                ) {
                                    self.evaluate_with_use(
                                        initializer.value,
                                        locals,
                                        ParameterUse::Borrow,
                                    )?
                                } else {
                                    self.evaluate_in_context(initializer.value, locals, true)?
                                };
                                if value.provenance() != Some(provenance) {
                                    return Err(failure(
                                        "reference initializer derives from another exact source local",
                                    ));
                                }
                                let selected = value
                                    .source_anchor()
                                    .ok_or_else(|| {
                                        failure("reference initializer lacks a tagged source path")
                                    })?
                                    .clone();
                                match value.raw() {
                                    NormalizedValue::ReadView(view) => {
                                        value.retain_flat_read_custody(
                                            selected.origin,
                                            &mut builder,
                                            &admission,
                                            control,
                                            &mut |bytes| self.charge_allocation(bytes),
                                        )?;
                                        value.retain_view_custody(
                                            &mut builder,
                                            &admission,
                                            control,
                                            &mut |bytes| self.charge_allocation(bytes),
                                        )?;
                                        builder.view_reference(
                                            field.id, selected, view, &admission, control,
                                        )?;
                                    }
                                    raw if raw.memory_is_borrowed() => {
                                        let (ancestors, packets) = value.flatten_read_custody(
                                            selected.origin,
                                            control,
                                            &mut |bytes| self.charge_allocation(bytes),
                                        )?;
                                        // Field ancestors must remain protected beyond this
                                        // temporary checked value. Tokens are retained in the
                                        // source packet; projected descriptor guards go in the
                                        // backing's inaccessible custody links.
                                        for read in ancestors {
                                            builder.retain_read(
                                                read,
                                                &admission,
                                                control,
                                                &mut |bytes| self.charge_allocation(bytes),
                                            )?;
                                        }
                                        builder.reserve_guards(
                                            packets.len(),
                                            control,
                                            &mut |bytes| self.charge_allocation(bytes),
                                        )?;
                                        for packet in packets {
                                            builder
                                                .retain_projected(&packet, &admission, control)?;
                                        }
                                        value.retain_view_custody(
                                            &mut builder,
                                            &admission,
                                            control,
                                            &mut |bytes| self.charge_allocation(bytes),
                                        )?;
                                        builder.owned_reference(
                                            field.id,
                                            field.ty,
                                            selected,
                                            raw.memory_borrow()?,
                                            &admission,
                                            control,
                                        )?;
                                    }
                                    _ => {
                                        return Err(failure(
                                            "read reference initializer is not a separately accounted read",
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    let view = builder.finish(&admission, control, &mut |bytes| {
                        self.charge_allocation(bytes)
                    })?;
                    let mut value = CheckedValue::view(&schema, view, false, control)?;
                    value.set_provenance(provenance);
                    Ok(value)
                })();
                self.read_source_demand.pop();
                result
            }
            ExpressionOperation::MoveReadView { value }
            | ExpressionOperation::DropReadView { value } => {
                let drop_handle = matches!(operation, ExpressionOperation::DropReadView { .. });
                let selected = locals
                    .get(&value)
                    .ok_or_else(|| failure("view handle is absent or consumed"))?;
                if selected.is_view_borrowed()
                    || !matches!(selected.raw(), NormalizedValue::ReadView(_))
                {
                    return Err(failure(
                        "borrowed view parameters/locals cannot be moved or dropped",
                    ));
                }
                let selected = locals
                    .remove(&value)
                    .ok_or_else(|| failure("view handle disappeared"))?;
                if drop_handle {
                    drop(selected);
                    CheckedValue::primitive(&self.schema, NormalizedValue::Unit)
                } else {
                    Ok(selected)
                }
            }
            ExpressionOperation::ReborrowReadView { value, read_scope } => {
                let scope = self.resolve_read_scope_arguments(&[read_scope])?[0];
                let selected = locals
                    .get(&value)
                    .ok_or_else(|| failure("reborrow source is absent"))?;
                let NormalizedValue::ReadView(view) = selected.raw() else {
                    return Err(failure("reborrow source is not a read view"));
                };
                let anchor = self.view_source_anchor(value, selected)?;
                let schema = Arc::clone(&self.schema);
                let original = schema
                    .read_view_proof
                    .as_ref()
                    .ok_or_else(|| failure("view proof missing"))?
                    .prepared_layout(view.identity().ty)?;
                let provenance = selected.provenance();
                let lease = self.ensure_view_source_scope(scope, selected, &anchor)?;
                let ty = self.view_identity(original.declaration, &original.arguments, scope)?;
                let admission = RuntimeAdmission::new(&schema, &anchor, scope, self.control);
                let target = ViewTarget {
                    identity: ViewIdentity {
                        ty,
                        program: schema.value_origin,
                    },
                    source: anchor.clone(),
                    scope: lease,
                    sharing: false,
                };
                let view = view.reborrow_as(target, &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
                let mut result = CheckedValue::view(&schema, view, false, self.control)?;
                result.inherit_read_custody(selected)?;
                if let Some(provenance) = provenance {
                    result.set_provenance(provenance);
                }
                Ok(result)
            }
            ExpressionOperation::ReadViewDataField { value, field } => {
                self.admit_view_field_authority(field.declaration)?;
                let selected = locals
                    .get(&value)
                    .ok_or_else(|| failure("view metadata source is absent"))?;
                let NormalizedValue::ReadView(view) = selected.raw() else {
                    return Err(failure("metadata source is not a read view"));
                };
                let schema = Arc::clone(&self.schema);
                let layout = schema
                    .read_view_proof
                    .as_ref()
                    .ok_or_else(|| failure("view proof missing"))?
                    .prepared_layout(view.identity().ty)?;
                if layout.declaration != field.declaration {
                    return Err(failure("metadata field names another declaration"));
                }
                let anchor = self.view_source_anchor(value, selected)?;
                let admission =
                    RuntimeAdmission::new(&schema, &anchor, layout.read_scope, self.control);
                let metadata = view.metadata(field.field, &admission, self.control)?;
                let ty = metadata.ty();
                let raw = metadata.raw(self.control)?;
                self.charge_allocation(super::super::value::projected_value_bytes(raw)?)?;
                let raw = raw.clone();
                self.admit_raw(raw, ty, &BTreeMap::new(), None, false)
            }
            ExpressionOperation::BorrowReadView {
                value,
                binding,
                body,
            } => {
                self.binding(binding, BindingKind::ReadViewBorrow)?;
                let selected = locals
                    .get(&value)
                    .ok_or_else(|| failure("view borrow source is absent"))?;
                let schema = Arc::clone(&self.schema);
                let anchor = self.view_source_anchor(value, selected)?;
                let scope = selected
                    .source_scope()
                    .ok_or_else(|| failure("view source scope missing"))?
                    .key();
                let admission = RuntimeAdmission::new(&schema, &anchor, scope, self.control);
                let parent =
                    selected.duplicate_view(&schema, anchor.clone(), &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
                let child =
                    selected.duplicate_view(&schema, anchor.clone(), &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
                self.evaluate_borrowed_body(parent, binding, child, body, locals)
            }
            ExpressionOperation::BorrowReadViewField {
                value,
                field,
                binding,
                body,
            } => {
                self.admit_view_field_authority(field.declaration)?;
                let selected = locals
                    .get(&value)
                    .ok_or_else(|| failure("view field source is absent"))?;
                let schema = Arc::clone(&self.schema);
                let NormalizedValue::ReadView(view) = selected.raw() else {
                    return Err(failure("view field source is not a read view"));
                };
                let layout = schema
                    .read_view_proof
                    .as_ref()
                    .ok_or_else(|| failure("view proof missing"))?
                    .prepared_layout(view.identity().ty)?;
                if layout.declaration != field.declaration {
                    return Err(failure("field names another exact view declaration"));
                }
                let declared = layout
                    .fields
                    .iter()
                    .find(|f| f.id == field.field)
                    .ok_or_else(|| failure("view field is undeclared"))?;
                if !matches!(declared.kind, ReadViewFieldKind::ReadReference { .. }) {
                    return Err(failure("metadata is not an accessible source reference"));
                }
                let anchor = self.view_source_anchor(value, selected)?;
                let scope = view.scope()?.reborrow(self.control)?;
                let admission =
                    RuntimeAdmission::new(&schema, &anchor, layout.read_scope, self.control);
                let parent =
                    selected.duplicate_view(&schema, anchor.clone(), &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
                let mut path = anchor.path.to_vec();
                self.charge_items(
                    path.len().saturating_add(1),
                    std::mem::size_of::<ReadPathStep>(),
                )?;
                path.push(ReadPathStep::ViewReference {
                    ty: view.identity().ty,
                    field: field.field,
                });
                let destination = SourceAnchor {
                    path: path.into(),
                    ..anchor.clone()
                };
                let control = self.control;
                let category = schema
                    .read_view_proof
                    .as_ref()
                    .ok_or_else(|| failure("view proof missing"))?
                    .prepared_category(declared.ty)?;
                let mut child = match category {
                    ValueCategory::Owned => {
                        self.binding(binding, BindingKind::OwnedBorrow)?;
                        let packet =
                            view.project_owned(field.field, destination, &admission, control)?;
                        if packet.ty() != declared.ty {
                            return Err(failure(
                                "projected Owned packet differs from the independently declared referent type",
                            ));
                        }
                        CheckedValue::projected_read(
                            &schema,
                            packet,
                            scope,
                            control,
                            &mut |bytes| self.charge_allocation(bytes),
                        )?
                    }
                    ValueCategory::ReadView => {
                        self.binding(binding, BindingKind::ReadViewBorrow)?;
                        let target = ViewTarget {
                            identity: ViewIdentity {
                                ty: declared.ty,
                                program: schema.value_origin,
                            },
                            source: destination,
                            scope,
                            sharing: view.is_shared(),
                        };
                        let projected = view.project_view_as(
                            field.field,
                            target,
                            &admission,
                            control,
                            &mut |bytes| self.charge_allocation(bytes),
                        )?;
                        CheckedValue::view(&schema, projected, true, control)?
                    }
                    _ => return Err(failure("view reference targets a foreign value category")),
                };
                if let Some(provenance) = parent.provenance() {
                    child.set_provenance(provenance);
                }
                self.evaluate_borrowed_body(parent, binding, child, body, locals)
            }
            _ => Err(failure(
                "foreign operation at the read-view execution boundary",
            )),
        }
    }
}

impl ReferenceState<'_> {
    pub(super) fn borrow_view_argument(
        &mut self,
        local: LocalValueReference,
        value: &CheckedValue,
    ) -> Result<CheckedValue, ExecutionError> {
        let schema = Arc::clone(&self.schema);
        let anchor = self.view_source_anchor(local, value)?;
        let scope = value
            .source_scope()
            .ok_or_else(|| failure("view argument lost its actual source lease"))?
            .key();
        let admission = RuntimeAdmission::new(&schema, &anchor, scope, self.control);
        value.duplicate_view(&schema, anchor.clone(), &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)
    }
}

impl ReferenceState<'_> {
    pub(super) fn attach_owned_read_path(
        &mut self,
        parent: &CheckedValue,
        child: &mut CheckedValue,
        step: super::super::read_view_storage::ReadPathStep,
    ) -> Result<(), ExecutionError> {
        let Some(anchor) = parent.source_anchor() else {
            return Ok(());
        };
        self.charge_items(
            anchor.path.len().saturating_add(1),
            std::mem::size_of::<super::super::read_view_storage::ReadPathStep>(),
        )?;
        let mut path = anchor.path.to_vec();
        path.push(step);
        let anchor = SourceAnchor {
            path: path.into(),
            ..anchor.clone()
        };
        let scope = parent
            .source_scope()
            .map(|scope| scope.reborrow(self.control))
            .transpose()?;
        child.bind_source(anchor, scope);
        Ok(())
    }
}

impl ReferenceState<'_> {
    fn borrow_call_view_scope<S>(
        &mut self,
        relation: &ResultRelation<S>,
        binder: crate::platform::kernel::ReadScopeBinder,
        formals: &[crate::platform::kernel::ReadScopeParameter],
        actuals: &[ReadScopeArgument],
    ) -> Result<Option<ReadScopeArgument>, ExecutionError> {
        let ResultRelation::ViewFrom { scope, .. } = relation else {
            return Ok(None);
        };
        let ReadScopeArgument::Parameter(reference) = scope else {
            return Err(failure("canonical view result lacks an exact formal scope"));
        };
        if reference.binder != binder || formals.len() != actuals.len() {
            return Err(failure(
                "view result scope binder or actual inventory differs",
            ));
        }
        self.charge_items(formals.len(), 0)?;
        for (formal, actual) in formals.iter().zip(actuals) {
            self.control.check()?;
            if formal.id == reference.parameter {
                return Ok(Some(*actual));
            }
        }
        Err(failure("view result selects a foreign scope formal"))
    }
}

impl ReferenceState<'_> {
    pub(super) fn open_borrow_call_scope(
        &mut self,
        call: ExpressionId,
        binding: BindingId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<
        Option<(
            ReadScopeArgument,
            Option<super::super::read_view_storage::RuntimeScopeOwner>,
            LocalValueReference,
            Option<SourceAnchor>,
            Option<super::super::read_view_storage::RuntimeScopeLease>,
        )>,
        ExecutionError,
    > {
        let operation = self.read_expression(call)?;
        let (scopes, arguments, position, view_scope) =
            match operation {
                ExpressionOperation::Call {
                    function,
                    read_scope_arguments,
                    arguments,
                    ..
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    read_scope_arguments,
                    arguments,
                    ..
                } => {
                    let signature = self.function_signature(function)?;
                    let source =
                        signature.result.relation.source().copied().ok_or_else(|| {
                            failure("borrow call has no selected canonical source")
                        })?;
                    let position = signature
                        .parameters
                        .iter()
                        .position(|parameter| parameter.header.owner == OwnerKey::Parameter(source))
                        .ok_or_else(|| failure("borrow-call result selects a foreign parameter"))?;
                    let view_scope = self.borrow_call_view_scope(
                        &signature.result.relation,
                        crate::platform::kernel::ReadScopeBinder::Declaration(function),
                        &signature.read_scope_parameters,
                        &read_scope_arguments,
                    )?;
                    (read_scope_arguments, arguments, position, view_scope)
                }
                ExpressionOperation::MethodCall {
                    contract,
                    method,
                    read_scope_arguments,
                    arguments,
                    ..
                } => {
                    let contract_reference = contract;
                    let method_id = method;
                    let DeclarationPayload::OwnedContract(contract) =
                        self.declaration(contract)?.payload
                    else {
                        return Err(failure("borrow method lacks a contract"));
                    };
                    let method = contract
                        .methods
                        .iter()
                        .find(|candidate| candidate.id == method)
                        .ok_or_else(|| failure("borrow method is undeclared"))?;
                    let position = *method
                        .result
                        .relation
                        .source()
                        .ok_or_else(|| failure("borrow method lacks a result source"))?
                        as usize;
                    let view_scope = self.borrow_call_view_scope(
                        &method.result.relation,
                        crate::platform::kernel::ReadScopeBinder::ContractMethod {
                            contract: contract_reference,
                            method: method_id,
                        },
                        &method.read_scope_parameters,
                        &read_scope_arguments,
                    )?;
                    (read_scope_arguments, arguments, position, view_scope)
                }
                _ => {
                    return Err(failure(
                        "borrow-call requires one exact named graph invocation",
                    ));
                }
            };
        if scopes.is_empty() {
            return Ok(None);
        }
        let callable = self
            .read_scope_frames
            .last()
            .and_then(ScopeFrame::declaration)
            .ok_or_else(|| failure("borrow-call scope lacks a defining invocation"))?;
        let local_key = ReadScopeArgument::Lexical { callable, binding };
        let (key, initializer_scope) = match view_scope {
            Some(key) if key == local_key => (key, false),
            Some(
                key @ ReadScopeArgument::Lexical {
                    callable: owner, ..
                },
            ) if owner == callable
                && self
                    .read_scope_frames
                    .last()
                    .is_some_and(|frame| frame.is_prospective(key)) =>
            {
                // The exact outer initializer owns this lifetime. Its result
                // formal selects one actual; an unrelated written scope grants no source.
                (key, true)
            }
            None if scopes.contains(&local_key) => (local_key, false),
            _ => return Ok(None),
        };
        let expression = *arguments
            .get(position)
            .ok_or_else(|| failure("borrow-call lacks its source argument"))?;
        let ExpressionOperation::Local { value: local } = self.read_expression(expression)? else {
            return Err(failure("lexical view source requires one exact live local"));
        };
        let source = locals
            .get(&local)
            .ok_or_else(|| failure("lexical view source is absent"))?;
        let anchor = self.view_source_anchor(local, source)?;
        let old_anchor = source.source_anchor().cloned();
        let old_scope = source
            .source_scope()
            .map(|scope| scope.reborrow(self.control))
            .transpose()?;
        if initializer_scope {
            let lease = self.ensure_view_source_scope(key, source, &anchor)?;
            if lease.source() != &anchor {
                return Err(failure(
                    "prospective view source differs from its admitted lifetime",
                ));
            }
            locals
                .get_mut(&local)
                .ok_or_else(|| failure("lexical source disappeared"))?
                .bind_source(anchor, Some(lease));
            // ScopeFrame retains the owner until the containing initializer
            // scope closes; BorrowCall restores only its temporary source link.
            return Ok(Some((key, None, local, old_anchor, old_scope)));
        }
        let parent = source
            .source_scope()
            .map(|scope| scope.reborrow(self.control))
            .transpose()?;
        let schema = Arc::clone(&self.schema);
        let admission = RuntimeAdmission::new(&schema, &anchor, key, self.control);
        let control = self.control;
        let (owner, lease) = super::super::read_view_storage::RuntimeScopeOwner::open(
            key,
            &anchor,
            parent.as_ref(),
            &admission,
            control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        self.read_scope_frames
            .last_mut()
            .ok_or_else(|| failure("lexical scope frame disappeared"))?
            .bind_scope(key, lease.reborrow(control)?, control)?;
        locals
            .get_mut(&local)
            .ok_or_else(|| failure("lexical source disappeared"))?
            .bind_source(anchor, Some(lease));
        Ok(Some((key, Some(owner), local, old_anchor, old_scope)))
    }
}

struct OpenedSourceScope {
    key: ReadScopeArgument,
}

impl ReferenceState<'_> {
    fn open_view_initializer_scope(
        &mut self,
        binding: BindingId,
        declared: TypeObjectDigest,
        _initializer: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<Option<OpenedSourceScope>, ExecutionError> {
        let callable = self
            .read_scope_frames
            .last()
            .and_then(ScopeFrame::declaration)
            .ok_or_else(|| failure("view initializer has no exact defining invocation"))?;
        let key = ReadScopeArgument::Lexical { callable, binding };
        let annotated = match &self
            .schema
            .types
            .get(&declared)
            .ok_or_else(|| failure("view initializer annotation is absent"))?
            .form
        {
            TypeForm::ReadViewApplied { read_scope, .. }
            | TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. } => *read_scope,
            _ => return Err(failure("view initializer annotation has another category")),
        };
        if annotated != key {
            return Ok(None);
        }
        if locals.contains_key(&LocalValueReference::LexicalBinding(binding)) {
            return Err(failure(
                "prospective view binding must remain unavailable during initialization",
            ));
        }
        self.charge_items(
            2,
            std::mem::size_of::<ReadScopeArgument>() + 3 * std::mem::size_of::<usize>(),
        )?;
        self.read_scope_frames
            .last_mut()
            .ok_or_else(|| failure("prospective frame disappeared"))?
            .begin_prospective_scope(key, self.control)?;
        self.lexical_loan_scopes = self
            .lexical_loan_scopes
            .checked_add(1)
            .ok_or_else(|| failure("source scope count overflow"))?;
        Ok(Some(OpenedSourceScope { key }))
    }

    fn close_view_initializer_scope(
        &mut self,
        opened: OpenedSourceScope,
        _locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) {
        if let Some(frame) = self.read_scope_frames.last_mut() {
            frame.remove_scope(opened.key);
        }
        self.lexical_loan_scopes -= 1;
    }

    pub(super) fn evaluate_view_let(
        &mut self,
        bindings: Vec<BindingId>,
        body: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
        demand: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        self.charge_items(
            bindings.len(),
            std::mem::size_of::<LocalValueReference>() + std::mem::size_of::<OpenedSourceScope>(),
        )?;
        let mut bound = Vec::with_capacity(bindings.len());
        let mut sources = Vec::with_capacity(bindings.len());
        let result = (|| {
            for binding in bindings {
                let local = LocalValueReference::LexicalBinding(binding);
                if locals.contains_key(&local) {
                    return Err(failure("Let aliases an existing exact local"));
                }
                let view_binding = matches!(self.owner(OwnerKey::Binding(binding))?, Some(OwnerRecord::Binding(record)) if record.kind == BindingKind::ReadView);
                let record = self.binding(
                    binding,
                    if view_binding {
                        BindingKind::ReadView
                    } else {
                        BindingKind::Let
                    },
                )?;
                let expression = record
                    .value
                    .ok_or_else(|| failure("Let initializer is absent"))?;
                if view_binding {
                    let declared = record
                        .declared_type
                        .ok_or_else(|| failure("owning view Let lacks an exact declared type"))?;
                    if let Some(opened) =
                        self.open_view_initializer_scope(binding, declared, expression, locals)?
                    {
                        sources.push(opened);
                    }
                }
                let value = self.evaluate(expression, locals)?;
                if view_binding {
                    let NormalizedValue::ReadView(view) = value.raw() else {
                        return Err(failure("owning view initializer has another category"));
                    };
                    let declared = record
                        .declared_type
                        .ok_or_else(|| failure("view annotation disappeared"))?;
                    let expected = self.resolve_occurrence_types(
                        (self.active_package, OwnerKey::Binding(binding)),
                        &[declared],
                    )?[0];
                    if value.is_view_borrowed() || view.identity().ty != expected {
                        return Err(failure(
                            "view initializer changes its exact type, scope or ownership mode",
                        ));
                    }
                    view.validate(self.control)?;
                } else if matches!(value.raw(), NormalizedValue::ReadView(_)) {
                    return Err(failure("ordinary Let cannot carry a view"));
                }
                locals.insert(local, value);
                bound.push(local);
            }
            self.evaluate_in_context(body, locals, demand)
        })();
        // Discard each descriptor/borrow before closing its source lifetime,
        // including partial initialization, cancellation and quota failures.
        for local in bound.into_iter().rev() {
            locals.remove(&local);
        }
        for opened in sources.into_iter().rev() {
            self.close_view_initializer_scope(opened, locals);
        }
        result
    }
}

impl ReferenceState<'_> {
    pub(super) fn begin_borrow_call_context(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<ReadScopeArgument>, ExecutionError> {
        let Some((package, OwnerKey::Expression(expression))) = self.type_occurrence else {
            return Err(failure("borrow-call lacks its exact canonical expression"));
        };
        let Some(callable) = self
            .schema
            .read_view_proof
            .as_ref()
            .and_then(|proof| proof.lexical_function_for_binding(package, binding))
        else {
            return Ok(None);
        };
        let Some(OwnerRecord::Expression(record)) =
            self.owner_in_package(package, OwnerKey::Expression(expression))?
        else {
            return Err(failure("borrow-call expression disappeared"));
        };
        if self.declaration(callable)?.header.contract_version
            < crate::platform::kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
        {
            return Ok(None);
        }
        if !matches!(record.operation, ExpressionOperation::BorrowCall { binding: actual, .. } if actual == binding)
        {
            return Err(failure("prospective context does not name this BorrowCall"));
        }
        if self
            .read_scope_frames
            .last()
            .and_then(ScopeFrame::declaration)
            != Some(callable)
        {
            return Err(failure(
                "borrow-call scope belongs to another current callable",
            ));
        }
        let scope = ReadScopeArgument::Lexical { callable, binding };
        self.charge_items(
            2,
            std::mem::size_of::<ReadScopeArgument>() + 3 * std::mem::size_of::<usize>(),
        )?;
        self.read_scope_frames
            .last_mut()
            .ok_or_else(|| failure("borrow-call frame disappeared"))?
            .begin_prospective_scope(scope, self.control)?;
        Ok(Some(scope))
    }
}

impl ReferenceState<'_> {
    fn ensure_view_source_scope(
        &mut self,
        scope: ReadScopeArgument,
        source: &CheckedValue,
        anchor: &SourceAnchor,
    ) -> Result<super::super::read_view_storage::RuntimeScopeLease, ExecutionError> {
        let frame = self
            .read_scope_frames
            .last()
            .ok_or_else(|| failure("source scope frame is missing"))?;
        if let Ok(lease) = frame.scope(scope) {
            return lease.reborrow(self.control);
        }
        if !frame.is_prospective(scope) {
            return Err(failure(
                "source lifetime was not declared by an exact lexical context",
            ));
        }
        let schema = Arc::clone(&self.schema);
        let parent = source
            .source_scope()
            .map(|lease| lease.reborrow(self.control))
            .transpose()?;
        let admission = RuntimeAdmission::new(&schema, anchor, scope, self.control);
        let control = self.control;
        let (owner, lease) = super::super::read_view_storage::RuntimeScopeOwner::open(
            scope,
            anchor,
            parent.as_ref(),
            &admission,
            control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        let result = lease.reborrow(control)?;
        self.charge_items(
            2,
            std::mem::size_of::<ReadScopeArgument>()
                + std::mem::size_of::<super::super::read_view_storage::RuntimeScopeLease>()
                + 6 * std::mem::size_of::<usize>(),
        )?;
        self.read_scope_frames
            .last_mut()
            .ok_or_else(|| failure("source scope frame disappeared"))?
            .bind_owned_scope(scope, owner, lease, control)?;
        Ok(result)
    }
}

impl ReferenceState<'_> {
    fn admit_view_field_authority(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<(), ExecutionError> {
        let callable = self
            .read_scope_frames
            .last()
            .and_then(ScopeFrame::declaration)
            .ok_or_else(|| failure("view field operation lacks a canonical defining callable"))?;
        if callable.package != declaration.package
            || self.declaration(callable)?.module != self.declaration(declaration)?.module
        {
            return Err(failure(
                "view field operation lacks the exact defining module authority",
            ));
        }
        Ok(())
    }
}

impl ReferenceState<'_> {
    pub(super) fn begin_lexical_read_context(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<ReadScopeArgument>, ExecutionError> {
        let Some((package, OwnerKey::Expression(expression))) = self.type_occurrence else {
            return Err(failure("lexical loan lacks its exact canonical expression"));
        };
        let Some(callable) = self
            .schema
            .read_view_proof
            .as_ref()
            .and_then(|proof| proof.lexical_function_for_binding(package, binding))
        else {
            return Ok(None);
        };
        let Some(OwnerRecord::Expression(record)) =
            self.owner_in_package(package, OwnerKey::Expression(expression))?
        else {
            return Err(failure("lexical loan expression disappeared"));
        };
        if self.declaration(callable)?.header.contract_version
            < crate::platform::kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
        {
            return Ok(None);
        }
        if self
            .read_scope_frames
            .last()
            .and_then(ScopeFrame::declaration)
            != Some(callable)
        {
            return Err(failure(
                "lexical source context belongs to another current callable",
            ));
        }
        let key = ReadScopeArgument::Lexical { callable, binding };
        if self
            .read_scope_frames
            .last()
            .is_some_and(|frame| frame.is_prospective(key))
        {
            // BorrowCall already proved and opened this exact outer context;
            // its current occurrence is now the direct invocation being adopted.
            return Ok(None);
        }
        let selected = match &record.operation {
            ExpressionOperation::BorrowOwnedField {
                binding: actual, ..
            }
            | ExpressionOperation::BorrowOwnedRecordField {
                binding: actual, ..
            }
            | ExpressionOperation::BorrowOwnedItem {
                binding: actual, ..
            }
            | ExpressionOperation::BorrowReadView {
                binding: actual, ..
            }
            | ExpressionOperation::BorrowReadViewField {
                binding: actual, ..
            }
            | ExpressionOperation::BorrowCall {
                binding: actual, ..
            } => *actual == binding,
            ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                arms.iter().any(|arm| arm.binding == binding)
            }
            _ => false,
        };
        if !selected {
            return Err(failure(
                "lexical context does not name this exact loan binding",
            ));
        }
        self.charge_items(
            2,
            std::mem::size_of::<ReadScopeArgument>() + 3 * std::mem::size_of::<usize>(),
        )?;
        self.read_scope_frames
            .last_mut()
            .ok_or_else(|| failure("lexical loan frame disappeared"))?
            .begin_prospective_scope(key, self.control)?;
        Ok(Some(key))
    }
}
