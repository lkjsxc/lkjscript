//! Exact type, scope and private-layout checks for the seven read-view operations.
//!
//! These checks do not turn equal types or scopes into source provenance. The
//! complete memory walk independently protects the designated source before
//! constructor children and proves the origin of every retained reference.

use super::super::owned_associated::read_view_admission as views;
use super::super::{
    ReadScopeArgument, ReadViewField, ReadViewFieldKind, ReadViewFieldReference, RegionMode,
};
use super::*;

impl<R: ExpressionRead> ExpressionValidator<'_, '_, R> {
    pub(super) fn infer_read_view_operation(
        &mut self,
        operation: ExpressionOperation,
        context: &ExecutionContext,
        consumer_generation: u16,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.consume_work()?;
        views::require_generation(consumer_generation)?;
        if depth > MAXIMUM_EXPRESSION_DEPTH {
            return Err(type_error(
                "kernel_type_expression_depth",
                "read-view inference exceeded its structural depth bound",
            ));
        }
        let occurrence = self.type_occurrence(context, consumer_generation);
        match operation {
            ExpressionOperation::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                // This point read precedes every field expression, including a
                // metadata-only constructor. It does not infer the source from
                // a reference field or grant a read path from custody alone.
                let source_ty = self.raw_local_type(source, context, depth)?;
                let source_is_view = self.owned_read_with_associated(|read, admission| {
                    let view = super::super::memory::view_at(read, source_ty, occurrence, admission)?;
                    if !view && !super::super::memory::direct_at(read, source_ty, occurrence, admission)? {
                        return Err(type_error("kernel_read_view_source", "view construction requires an exact live Owned or read-view local source"));
                    }
                    if !view {
                        super::super::owned_contract::validate_owned_type_at(read, source_ty, occurrence, admission)?;
                    }
                    Ok(view)
                })?;
                let authority = self.scope_authority(context);
                self.owned_read_with_associated(|read, admission| {
                    authority.validate(read, read_scope, admission)
                })?;
                if source_is_view {
                    let source_scope =
                        self.require_read_view_type(source_ty, context, consumer_generation)?;
                    self.require_scope_containment(read_scope, source_scope, context)?;
                }
                let ty = self.canonical_type(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: type_arguments,
                    read_scope,
                })?;
                let layout = self.private_view_layout(ty, context, consumer_generation)?;
                if layout.fields.len() != fields.len() {
                    return Err(type_error(
                        "kernel_read_view_field",
                        "view construction must initialize its complete exact field inventory",
                    ));
                }
                self.owned_read_with_associated(|read, admission| {
                    admission.step(read)?;
                    admission.reserve::<(FieldId, ReadViewField)>(layout.fields.len())
                })?;
                let mut expected = BTreeMap::new();
                for field in layout.fields {
                    self.consume_work()?;
                    if expected.insert(field.id, field).is_some() {
                        return Err(type_error(
                            "kernel_read_view_field",
                            "view layout contains a duplicate exact field identity",
                        ));
                    }
                }
                // Follow the constructor's authored order, rather than sorting
                // evaluation by the nominal declaration's field identities.
                for initializer in fields {
                    self.consume_work()?;
                    let field = expected.remove(&initializer.field).ok_or_else(|| {
                        type_error(
                            "kernel_read_view_field",
                            "view construction repeats or names a foreign inline field",
                        )
                    })?;
                    let actual = match field.kind {
                        ReadViewFieldKind::Ordinary => {
                            self.infer(initializer.value, context, depth)?
                        }
                        ReadViewFieldKind::ReadReference {
                            read_scope: field_scope,
                        } => {
                            if field_scope != read_scope {
                                return Err(type_error(
                                    "kernel_read_view_scope",
                                    "view reference field escaped the instantiated constructor scope",
                                ));
                            }
                            self.read_view_reference_initializer_type(
                                initializer.value,
                                context,
                                depth,
                            )?
                        }
                    };
                    require_same(
                        field.ty,
                        actual,
                        "kernel_read_view_field",
                        "view field initializer",
                    )?;
                }
                if !expected.is_empty() {
                    return Err(type_error(
                        "kernel_read_view_field",
                        "view construction omits an exact inline field",
                    ));
                }
                Ok(ty)
            }
            ExpressionOperation::BorrowReadView {
                value,
                binding,
                body,
            } => {
                let ty = self.raw_local_type(value, context, depth)?;
                self.require_read_view_type(ty, context, consumer_generation)?;
                self.require_read_view_binding(binding, ty, BindingKind::ReadViewBorrow, context)?;
                let scoped = self.read_view_borrow_context(
                    context,
                    binding,
                    BindingKind::ReadViewBorrow,
                    ty,
                )?;
                self.infer(body, &scoped, depth)
            }
            ExpressionOperation::ReborrowReadView { value, read_scope } => {
                let ty = self.raw_local_type(value, context, depth)?;
                let source_scope = self.require_read_view_type(ty, context, consumer_generation)?;
                self.require_scope_containment(read_scope, source_scope, context)?;
                let form = match self.type_object(ty)?.form {
                    TypeForm::RegionCap {
                        region,
                        mode: RegionMode::Shared { .. },
                    } => TypeForm::RegionCap {
                        region,
                        mode: RegionMode::Shared { scope: read_scope },
                    },
                    TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        ..
                    } => TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        read_scope,
                    },
                    TypeForm::AssociatedReadViewMember {
                        contract, member, ..
                    } => TypeForm::AssociatedReadViewMember {
                        contract,
                        member,
                        read_scope,
                    },
                    TypeForm::AssociatedReadViewProjection {
                        scope,
                        parameter,
                        contract,
                        member,
                        ..
                    } => TypeForm::AssociatedReadViewProjection {
                        scope,
                        parameter,
                        contract,
                        member,
                        read_scope,
                    },
                    _ => {
                        return Err(type_error(
                            "kernel_read_view_source",
                            "reborrow has no positive read-view source type",
                        ));
                    }
                };
                let result = self.canonical_type(form)?;
                self.require_read_view_type(result, context, consumer_generation)?;
                Ok(result)
            }
            ExpressionOperation::MoveReadView { value } => {
                let ty =
                    self.owning_read_view_local_type(value, context, consumer_generation, depth)?;
                // Availability and the one cleanup obligation are checked by
                // the complete affine memory walk, not a mutable type context.
                Ok(ty)
            }
            ExpressionOperation::DropReadView { value } => {
                self.owning_read_view_local_type(value, context, consumer_generation, depth)?;
                self.canonical_type(TypeForm::Unit)
            }
            ExpressionOperation::ReadViewDataField { value, field } => {
                let selected =
                    self.read_view_field(value, field, context, consumer_generation, depth)?;
                if selected.kind != ReadViewFieldKind::Ordinary {
                    return Err(type_error(
                        "kernel_read_view_field",
                        "ordinary view field access cannot recover a reference-field read",
                    ));
                }
                // Complete nominal admission already proved the immutable
                // ordinary Transferable closure, including hidden actuals.
                Ok(selected.ty)
            }
            ExpressionOperation::BorrowReadViewField {
                value,
                field,
                binding,
                body,
            } => {
                let selected =
                    self.read_view_field(value, field, context, consumer_generation, depth)?;
                if !matches!(selected.kind, ReadViewFieldKind::ReadReference { .. }) {
                    return Err(type_error(
                        "kernel_read_view_field",
                        "reference borrowing requires an explicit read-reference field",
                    ));
                }
                let view = self.owned_read_with_associated(|read, admission| {
                    let view = super::super::memory::view_at(read, selected.ty, occurrence, admission)?;
                    if !view && !super::super::memory::direct_at(read, selected.ty, occurrence, admission)? {
                        return Err(type_error("kernel_read_view_field", "reference field has neither an admitted Owned nor positive read-view referent"));
                    }
                    Ok(view)
                })?;
                let kind = if view {
                    self.require_read_view_type(selected.ty, context, consumer_generation)?;
                    BindingKind::ReadViewBorrow
                } else {
                    BindingKind::OwnedBorrow
                };
                self.require_read_view_binding(binding, selected.ty, kind, context)?;
                let scoped = self.read_view_borrow_context(context, binding, kind, selected.ty)?;
                self.infer(body, &scoped, depth)
            }
            _ => Err(type_error(
                "kernel_read_view_operation",
                "read-view inference received a different operation kind",
            )),
        }
    }

    /// Explicit borrow/move/field operations point-read a local without giving
    /// the ordinary Local expression an implicit view-copy interpretation.
    pub(super) fn raw_local_type(
        &mut self,
        reference: LocalValueReference,
        context: &ExecutionContext,
        _depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.consume_work()?;
        match reference {
            LocalValueReference::FunctionParameter(parameter) => {
                let key = OwnerKey::Parameter(parameter);
                let owner = self.read.owner(key)?.ok_or_else(|| {
                    type_error(
                        "kernel_type_parameter_missing",
                        "explicit read source parameter is missing",
                    )
                })?;
                owner.validate_local()?;
                let OwnerRecord::Parameter(record) = owner else {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "explicit read source names a different owner kind",
                    ));
                };
                if record.header.owner != key || record.header.kind != OwnerKind::Parameter {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "explicit read source has a foreign parameter identity",
                    ));
                }
                let ParameterParent::Function(parent) = record.parent else {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "explicit read source names an operation parameter",
                    ));
                };
                if context.declaration != Some(parent) {
                    return Err(type_error(
                        "kernel_type_parameter_scope",
                        "explicit read source belongs to a different callable",
                    ));
                }
                self.consume_work()?;
                let declaration_owner = self
                    .read
                    .owner(OwnerKey::Declaration(parent))?
                    .ok_or_else(|| {
                        type_error(
                            "kernel_type_parameter_scope",
                            "explicit read source has no exact callable owner",
                        )
                    })?;
                declaration_owner.validate_local()?;
                let OwnerRecord::Declaration(declaration) = declaration_owner else {
                    return Err(type_error(
                        "kernel_type_parameter_scope",
                        "explicit read source has no exact callable owner",
                    ));
                };
                if declaration.header.owner != OwnerKey::Declaration(parent) {
                    return Err(type_error(
                        "kernel_type_parameter_scope",
                        "explicit read source callable has a foreign identity",
                    ));
                }
                let parameters = match declaration.payload {
                    DeclarationPayload::Function(function) => function.parameters,
                    DeclarationPayload::External(function) => function.parameters,
                    _ => {
                        return Err(type_error(
                            "kernel_type_parameter_scope",
                            "explicit read source parent is not a callable",
                        ));
                    }
                };
                let mut listed = 0usize;
                for id in parameters {
                    self.consume_work()?;
                    listed += usize::from(id == parameter);
                }
                if listed != 1 {
                    return Err(type_error(
                        "kernel_type_parameter_scope",
                        "explicit read source is outside the exact ordered parameter inventory",
                    ));
                }
                self.validate_type_occurrence(record.ty, context, record.header.contract_version)?;
                if self.type_object(record.ty)?.form.is_read_view()
                    && record.use_mode != ParameterUse::Borrow
                {
                    return Err(type_error(
                        "kernel_read_view_source",
                        "view parameters require borrow mode",
                    ));
                }
                Ok(record.ty)
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => {
                let key = OwnerKey::Binding(binding);
                let owner = self.read.owner(key)?.ok_or_else(|| {
                    type_error(
                        "kernel_type_binding_missing",
                        "explicit read source binding is missing",
                    )
                })?;
                owner.validate_local()?;
                let OwnerRecord::Binding(record) = owner else {
                    return Err(type_error(
                        "kernel_full_local_binding_domain",
                        "explicit read source names a different owner kind",
                    ));
                };
                if record.header.owner != key || record.header.kind != OwnerKind::Binding {
                    return Err(type_error(
                        "kernel_full_local_binding_domain",
                        "explicit read source has a foreign binding identity",
                    ));
                }
                let Some((kind, ty)) = context.bindings.get(&binding).copied() else {
                    return Err(type_error(
                        "kernel_full_lexical_scope",
                        "explicit read source is outside its exact lexical scope",
                    ));
                };
                let valid_domain = match reference {
                    LocalValueReference::LexicalBinding(_) => matches!(
                        kind,
                        BindingKind::Let
                            | BindingKind::Region
                            | BindingKind::OwnedUnpack
                            | BindingKind::OwnedChoicePayload
                            | BindingKind::OwnedBorrow
                            | BindingKind::ReadView
                            | BindingKind::ReadViewBorrow
                    ),
                    LocalValueReference::MatchPayload(_) => kind == BindingKind::MatchPayload,
                    _ => false,
                };
                if !valid_domain
                    || record.kind != kind
                    || record.declared_type.is_some_and(|declared| declared != ty)
                {
                    return Err(type_error(
                        "kernel_full_local_binding_domain",
                        "explicit read source binding domain, kind or annotation differs from its exact context",
                    ));
                }
                self.validate_type_occurrence(ty, context, record.header.contract_version)?;
                let view = self.type_object(ty)?.form.is_read_view();
                if view != matches!(kind, BindingKind::ReadView | BindingKind::ReadViewBorrow) {
                    return Err(type_error(
                        "kernel_read_view_source",
                        "local view category and exact binding kind disagree",
                    ));
                }
                Ok(ty)
            }
            LocalValueReference::OperationParameter(parameter) => {
                let key = OwnerKey::Parameter(parameter);
                let owner = self.read.owner(key)?.ok_or_else(|| {
                    type_error(
                        "kernel_type_parameter_missing",
                        "operation parameter is missing",
                    )
                })?;
                owner.validate_local()?;
                let OwnerRecord::Parameter(record) = owner else {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "operation-parameter reference names a different owner kind",
                    ));
                };
                if record.header.owner != key
                    || !matches!(record.parent, ParameterParent::Operation(_))
                {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "operation-parameter reference names a function parameter or foreign identity",
                    ));
                }
                Err(type_error(
                    "kernel_type_parameter_scope",
                    "operation parameter has no executable operation context",
                ))
            }
            // The ordinary transaction-marker path remains in local_type.
            LocalValueReference::TransactionBinding(_) => Err(type_error(
                "kernel_read_view_source",
                "explicit view operations require a callable parameter or live lexical value",
            )),
        }
    }

    pub(super) fn require_read_view_type(
        &mut self,
        ty: TypeObjectDigest,
        context: &ExecutionContext,
        generation: u16,
    ) -> Result<ReadScopeArgument, Diagnostic> {
        let occurrence = self.type_occurrence(context, generation);
        self.validate_type_occurrence_at(ty, occurrence)?;
        let scope = match self.type_object(ty)?.form {
            TypeForm::ReadViewApplied { read_scope, .. }
            | TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. }
            | TypeForm::RegionCap {
                mode: crate::platform::kernel::RegionMode::Shared { scope: read_scope },
                ..
            } => read_scope,
            _ => {
                return Err(type_error(
                    "kernel_read_view_source",
                    "operation requires the positive ReadView category",
                ));
            }
        };
        let authority = self.scope_authority(context);
        self.owned_read_with_associated(|read, admission| {
            if !super::super::memory::view_at(read, ty, occurrence, admission)? {
                return Err(type_error(
                    "kernel_read_view_source",
                    "read-view category admission failed",
                ));
            }
            authority.validate(read, scope, admission)
        })?;
        Ok(scope)
    }

    fn private_view_layout(
        &mut self,
        ty: TypeObjectDigest,
        context: &ExecutionContext,
        generation: u16,
    ) -> Result<views::ViewLayout, Diagnostic> {
        self.require_read_view_type(ty, context, generation)?;
        let TypeForm::ReadViewApplied { declaration, .. } = self.type_object(ty)?.form else {
            return Err(type_error(
                "kernel_read_view_field",
                "private view operations require the exact nominal declaration type",
            ));
        };
        let caller = context
            .declaration
            .map(|declaration| DeclarationReference {
                package: self.read.package_id(),
                declaration,
            })
            .ok_or_else(|| {
                type_error(
                    "kernel_read_view_field",
                    "private view operations require an exact containing declaration",
                )
            })?;
        let occurrence = self.type_occurrence(context, generation);
        let mut layout = self.owned_read_with_associated(|read, admission| {
            views::require_access(read, declaration, caller, admission)?;
            views::validate_nominal_at(read, ty, occurrence, admission)?;
            views::layout(read, ty, occurrence, admission)
        })?;
        self.adopt_applied_types(std::mem::take(&mut layout.types))?;
        Ok(layout)
    }

    fn read_view_field(
        &mut self,
        value: LocalValueReference,
        reference: ReadViewFieldReference,
        context: &ExecutionContext,
        generation: u16,
        depth: usize,
    ) -> Result<ReadViewField, Diagnostic> {
        let ty = self.raw_local_type(value, context, depth)?;
        let layout = self.private_view_layout(ty, context, generation)?;
        if reference.declaration != layout.declaration {
            return Err(type_error(
                "kernel_read_view_field",
                "view field reference names a different exact nominal declaration",
            ));
        }
        for field in layout.fields {
            self.consume_work()?;
            if field.id == reference.field {
                return Ok(field);
            }
        }
        Err(type_error(
            "kernel_read_view_field",
            "view field identity is outside its complete inline inventory",
        ))
    }

    fn require_read_view_binding(
        &mut self,
        binding: BindingId,
        ty: TypeObjectDigest,
        kind: BindingKind,
        context: &ExecutionContext,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        if context.bindings.contains_key(&binding) {
            return Err(type_error(
                "kernel_read_view_source",
                "a scoped read cannot redefine an already live exact binding",
            ));
        }
        let key = OwnerKey::Binding(binding);
        let owner = self.read.owner(key)?.ok_or_else(|| {
            type_error(
                "kernel_read_view_source",
                "scoped view operation has no exact destination binding",
            )
        })?;
        owner.validate_local()?;
        let OwnerRecord::Binding(record) = owner else {
            return Err(type_error(
                "kernel_read_view_source",
                "scoped view destination has a different owner kind",
            ));
        };
        // Owned read bindings retain their native profile. Their new lexical
        // scope is admitted by the actual enclosing Graph 34 function, while
        // positive view bindings independently require the view generation.
        if kind != BindingKind::OwnedBorrow {
            views::require_generation(record.header.contract_version)?;
        }
        if record.header.owner != key
            || record.header.kind != OwnerKind::Binding
            || record.kind != kind
            || record.value.is_some()
            || record.declared_type != Some(ty)
        {
            return Err(type_error(
                "kernel_read_view_source",
                "scoped view destination kind, identity or exact type annotation mismatch",
            ));
        }
        self.validate_type_occurrence(ty, context, record.header.contract_version)
    }

    fn read_view_borrow_context(
        &mut self,
        context: &ExecutionContext,
        binding: BindingId,
        kind: BindingKind,
        ty: TypeObjectDigest,
    ) -> Result<ExecutionContext, Diagnostic> {
        let mut scoped = self.scoped_context(context)?;
        self.consume_work()?;
        scoped.bindings.insert(binding, (kind, ty));
        self.enter_read_scope(&mut scoped, binding)?;
        Ok(scoped)
    }

    fn owning_read_view_local_type(
        &mut self,
        value: LocalValueReference,
        context: &ExecutionContext,
        generation: u16,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let LocalValueReference::LexicalBinding(binding) = value else {
            return Err(type_error(
                "kernel_read_view_source",
                "view move and drop require an owning local handle",
            ));
        };
        if !matches!(
            context.bindings.get(&binding),
            Some((BindingKind::ReadView, _))
        ) {
            return Err(type_error(
                "kernel_read_view_source",
                "borrowed view reads cannot surrender an owning handle",
            ));
        }
        let ty = self.raw_local_type(value, context, depth)?;
        self.require_read_view_type(ty, context, generation)?;
        Ok(ty)
    }

    fn read_view_reference_initializer_type(
        &mut self,
        expression: ExpressionId,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.consume_work()?;
        if depth > MAXIMUM_EXPRESSION_DEPTH {
            return Err(type_error(
                "kernel_type_expression_depth",
                "view reference initializer exceeded its structural depth bound",
            ));
        }
        let Some(OwnerRecord::Expression(record)) =
            self.read.owner(OwnerKey::Expression(expression))?
        else {
            return Err(type_error(
                "kernel_type_expression_missing",
                "view reference initializer is missing",
            ));
        };
        if record.id != expression {
            return Err(type_error(
                "kernel_read_view_field",
                "view reference initializer has a foreign expression identity",
            ));
        }
        record.validate_local()?;
        if let ExpressionOperation::Local { value } = record.operation {
            let ty = self.raw_local_type(value, context, depth)?;
            self.validate_type_occurrence(ty, context, record.contract_version)?;
            if let Some((selected, inferred)) = &mut self.selected_expression
                && *selected == expression
            {
                *inferred = Some(ty);
            }
            // The constructor establishes a read lease. This is not ordinary
            // implicit copying of the owner or an affine descriptor handle.
            Ok(ty)
        } else {
            self.infer(expression, context, depth)
        }
    }
}
