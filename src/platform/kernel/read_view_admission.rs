//! Exact scope and nominal view admission. Custody is never a readable path.
//! The session is borrowed from associated admission, including on cache hits.
use super::super::*;
use super::{AssociatedTypeContext, OwnedAssociatedAdmission, OwnedTypeOccurrence};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{BindingId, FieldId, ReadScopeParameterId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_read_view", message)
}

pub(crate) fn require_generation(generation: u16) -> Result<(), Diagnostic> {
    if generation != crate::platform::canonical_schema::SCHEMA_EPOCH {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "kernel_read_view_generation",
            "read-view meaning requires the current schema epoch at its actual consuming owner",
        ));
    }
    Ok(())
}

/// Local lexical loans are supplied by the actual containing expression walk.
/// A binding owner alone has no parent and cannot certify a lexical scope.
#[derive(Clone, Copy)]
pub(crate) struct ScopeAuthority<'a> {
    pub(crate) binder: Option<ReadScopeBinder>,
    pub(crate) lexical: Option<DeclarationReference>,
    pub(crate) live_loans: &'a BTreeSet<BindingId>,
}

impl ScopeAuthority<'_> {
    pub(crate) fn validate(
        self,
        read: &(impl ExpressionRead + ?Sized),
        argument: ReadScopeArgument,
        admission: &OwnedAssociatedAdmission,
    ) -> Result<(), Diagnostic> {
        admission.step(read)?;
        if let Some(callable) = self.lexical {
            if self.binder != Some(ReadScopeBinder::Declaration(callable)) {
                return Err(reject(
                    "lexical scope authority disagrees with its exact callable binder",
                ));
            }
            let (header, payload) = declaration(read, callable, admission)?;
            let PackageInterfaceDeclarationPayload::Function(function) = payload else {
                return Err(reject(
                    "lexical scope authority requires an exact named graph function",
                ));
            };
            let expected = match function.effect {
                FunctionEffect::Pure => OwnerKind::PureFunction,
                FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
            };
            if header.kind != expected {
                return Err(reject("scope authority function kind/header mismatch"));
            }
        }
        match argument {
            ReadScopeArgument::Parameter(reference) => {
                if self.binder != Some(reference.binder) {
                    return Err(reject("read scope escaped its exact owning binder"));
                }
                let parameters = scope_parameters(read, reference.binder, admission)?;
                let mut found = false;
                for parameter in parameters {
                    admission.step(read)?;
                    found |= parameter.id == reference.parameter;
                }
                if !found {
                    return Err(reject(
                        "read scope is absent from its exact formal inventory",
                    ));
                }
            }
            ReadScopeArgument::Lexical { callable, binding } => {
                if self.lexical != Some(callable)
                    || callable.package != read.package_id()
                    || !self.live_loans.contains(&binding)
                {
                    return Err(reject("lexical read scope has no live containing loan"));
                }
                let (header, payload) = declaration(read, callable, admission)?;
                let PackageInterfaceDeclarationPayload::Function(function) = payload else {
                    return Err(reject(
                        "lexical read scopes require an exact named graph function",
                    ));
                };
                let expected = match function.effect {
                    FunctionEffect::Pure => OwnerKind::PureFunction,
                    FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
                };
                if header.kind != expected {
                    return Err(reject("lexical read-scope function kind/header mismatch"));
                }
                admission.step(read)?;
                let Some(OwnerRecord::Binding(record)) = read.owner(OwnerKey::Binding(binding))?
                else {
                    return Err(reject("missing exact lexical read-scope binding"));
                };
                if record.header.owner != OwnerKey::Binding(binding)
                    || record.header.kind != OwnerKind::Binding
                    || (record.value.is_some() && record.kind != BindingKind::ReadView)
                    || !matches!(
                        record.kind,
                        BindingKind::OwnedBorrow
                            | BindingKind::ReadViewBorrow
                            | BindingKind::ReadView
                    )
                {
                    return Err(reject(
                        "lexical read scope is not an exact admitted loan binding",
                    ));
                }
                let owner = OwnerRecord::Binding(record);
                owner.validate_local()?;
                let OwnerRecord::Binding(record) = owner else {
                    unreachable!("binding wrapper");
                };
                if record.kind != BindingKind::OwnedBorrow {
                    require_generation(record.header.contract_version)?;
                }
                lexical_owner(
                    read,
                    callable,
                    binding,
                    AssociatedTypeContext::Lexical(callable),
                    admission,
                )?;
            }
        }
        Ok(())
    }
}

/// Certify only exact body ownership of a lexical loan binder. The actual flow
/// checks its enclosing/live scope independently; neither a table nor a Binding
/// point read is sufficient for that second obligation.
pub(crate) fn lexical_owner(
    read: &(impl ExpressionRead + ?Sized),
    callable: DeclarationReference,
    binding: BindingId,
    context: AssociatedTypeContext,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    admission.step(read)?;
    if context != AssociatedTypeContext::Lexical(callable) || callable.package != read.package_id()
    {
        return Err(reject(
            "lexical view scope escaped its exact local callable",
        ));
    }
    let Some(OwnerRecord::Declaration(record)) =
        read.owner(OwnerKey::Declaration(callable.declaration))?
    else {
        return Err(reject("missing lexical scope callable"));
    };
    require_generation(record.header.contract_version)?;
    let owner = OwnerRecord::Declaration(record);
    owner.validate_local()?;
    let OwnerRecord::Declaration(record) = owner else {
        unreachable!("declaration wrapper");
    };
    if record.header.owner != OwnerKey::Declaration(callable.declaration) {
        return Err(reject("foreign lexical scope callable identity"));
    }
    let DeclarationPayload::Function(function) = record.payload else {
        return Err(reject("lexical view scope is not a graph function"));
    };
    let Some(OwnerRecord::Binding(record)) = read.owner(OwnerKey::Binding(binding))? else {
        return Err(reject("missing lexical scope binding"));
    };
    if record.header.owner != OwnerKey::Binding(binding)
        || (record.value.is_some() && record.kind != BindingKind::ReadView)
        || record.declared_type.is_none()
        || !matches!(
            record.kind,
            BindingKind::OwnedBorrow | BindingKind::ReadView | BindingKind::ReadViewBorrow
        )
    {
        return Err(reject("lexical scope is not an exact typed loan binding"));
    }
    let owner = OwnerRecord::Binding(record);
    owner.validate_local()?;
    let OwnerRecord::Binding(record) = owner else {
        unreachable!("binding wrapper");
    };
    if record.kind != BindingKind::OwnedBorrow {
        require_generation(record.header.contract_version)?;
    }
    admission.reserve::<crate::platform::semantic_id::ExpressionId>(1)?;
    let mut pending = vec![function.body];
    let mut seen = BTreeSet::new();
    let mut found = 0usize;
    while let Some(id) = pending.pop() {
        admission.step(read)?;
        if seen.contains(&id) {
            continue;
        }
        admission.reserve::<crate::platform::semantic_id::ExpressionId>(1)?;
        seen.insert(id);
        let Some(OwnerRecord::Expression(expression)) = read.owner(OwnerKey::Expression(id))?
        else {
            return Err(reject("missing exact lexical scope body occurrence"));
        };
        if expression.id != id {
            return Err(reject("foreign lexical body occurrence identity"));
        }
        match &expression.operation {
            ExpressionOperation::BorrowCall {
                binding: declared, ..
            }
            | ExpressionOperation::BorrowOwnedField {
                binding: declared, ..
            }
            | ExpressionOperation::BorrowOwnedRecordField {
                binding: declared, ..
            }
            | ExpressionOperation::BorrowOwnedItem {
                binding: declared, ..
            }
            | ExpressionOperation::BorrowReadView {
                binding: declared, ..
            }
            | ExpressionOperation::BorrowReadViewField {
                binding: declared, ..
            } => {
                admission.step(read)?;
                if *declared == binding {
                    expression.validate_local()?;
                    found += 1;
                }
            }
            ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                for arm in arms {
                    admission.step(read)?;
                    if arm.binding == binding {
                        expression.validate_local()?;
                        found += 1;
                    }
                }
            }
            ExpressionOperation::Let { bindings, .. } => {
                for id in bindings {
                    admission.step(read)?;
                    let Some(OwnerRecord::Binding(record)) = read.owner(OwnerKey::Binding(*id))?
                    else {
                        return Err(reject("missing lexical walk initializer binding"));
                    };
                    if *id == binding
                        && record.kind == BindingKind::ReadView
                        && record.value.is_some()
                    {
                        require_generation(expression.contract_version)?;
                        let ty = record.declared_type.ok_or_else(|| {
                            reject("view initializer scope requires its exact type")
                        })?;
                        let object = read
                            .type_object(ty)?
                            .ok_or_else(|| reject("missing exact owning view initializer type"))?;
                        if !direct(&object.form) {
                            return Err(reject(
                                "owning initializer scope requires the positive view category",
                            ));
                        }
                        found += 1;
                    }
                    if let Some(value) = record.value {
                        admission.reserve::<crate::platform::semantic_id::ExpressionId>(1)?;
                        pending.push(value);
                    }
                }
            }
            _ => {}
        }
        expression.visit_children(|child| {
            admission.step(read)?;
            admission.reserve::<crate::platform::semantic_id::ExpressionId>(1)?;
            pending.push(child);
            Ok(())
        })?;
    }
    if found != 1 {
        return Err(reject(
            "lexical scope requires one exact containing loan occurrence",
        ));
    }
    Ok(())
}

fn declaration(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    admission: &OwnedAssociatedAdmission,
) -> Result<(OwnerHeader, PackageInterfaceDeclarationPayload), Diagnostic> {
    admission.step(read)?;
    let key = OwnerKey::Declaration(reference.declaration);
    let (header, payload) = if reference.package == read.package_id() {
        let Some(OwnerRecord::Declaration(record)) = read.owner(key)? else {
            return Err(reject("missing exact read-scope declaration"));
        };
        let record = super::local_declaration(record)?;
        (record.header, record.payload)
    } else {
        if !read.has_dependency(reference.package)? {
            return Err(reject(
                "read-scope authority is outside the authored import closure",
            ));
        }
        let Some(PackageInterfaceRecord::Declaration(record)) =
            read.package_interface_owner(reference.package, key)?
        else {
            return Err(reject("missing exact imported read-scope declaration"));
        };
        (record.header, record.payload)
    };
    if header.owner != key {
        return Err(reject(
            "read-scope declaration has a foreign owner identity",
        ));
    }
    require_generation(header.contract_version)?;
    Ok((header, payload))
}

pub(crate) fn scope_parameters(
    read: &(impl ExpressionRead + ?Sized),
    binder: ReadScopeBinder,
    admission: &OwnedAssociatedAdmission,
) -> Result<Vec<ReadScopeParameter>, Diagnostic> {
    let reference = match binder {
        ReadScopeBinder::Declaration(reference) => reference,
        ReadScopeBinder::ContractMethod { contract, .. }
        | ReadScopeBinder::AssociatedMember { contract, .. } => contract,
        ReadScopeBinder::AssociatedBinding { implementation, .. } => implementation,
    };
    let (header, payload) = declaration(read, reference, admission)?;
    let parameters = match (binder, payload) {
        (
            ReadScopeBinder::Declaration(_),
            PackageInterfaceDeclarationPayload::Function(function),
        ) if matches!(
            header.kind,
            OwnerKind::PureFunction | OwnerKind::TaskFunction
        ) =>
        {
            let expected = match function.effect {
                FunctionEffect::Pure => OwnerKind::PureFunction,
                FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
            };
            if header.kind != expected {
                return Err(reject(
                    "scope formal inventory function kind/header mismatch",
                ));
            }
            function.read_scope_parameters
        }
        (ReadScopeBinder::Declaration(_), PackageInterfaceDeclarationPayload::ReadView(view))
            if header.kind == OwnerKind::ReadView =>
        {
            admission.reserve::<ReadScopeParameter>(1)?;
            vec![view.scope_parameter]
        }
        (
            ReadScopeBinder::ContractMethod { method, .. },
            PackageInterfaceDeclarationPayload::OwnedContract(contract),
        ) if header.kind == OwnerKind::OwnedContract => {
            let mut found = None;
            for entry in contract.methods {
                admission.step(read)?;
                if entry.id == method {
                    if found.is_some() {
                        return Err(reject("duplicate exact contract method"));
                    }
                    found = Some(entry.read_scope_parameters);
                }
            }
            found.ok_or_else(|| reject("read scope names no exact contract method"))?
        }
        (
            ReadScopeBinder::AssociatedMember { member, .. },
            PackageInterfaceDeclarationPayload::OwnedContract(contract),
        ) if header.kind == OwnerKind::OwnedContract => {
            let mut found = None;
            for entry in contract.associated_types {
                admission.step(read)?;
                if entry.id == member {
                    if found.is_some() {
                        return Err(reject("duplicate exact associated member"));
                    }
                    let AssociatedTypeBound::ReadViewFamily { scope_parameter } = entry.bound
                    else {
                        return Err(reject("Owned member has no read-scope formal"));
                    };
                    admission.reserve::<ReadScopeParameter>(1)?;
                    found = Some(vec![scope_parameter]);
                }
            }
            found.ok_or_else(|| reject("read scope names no exact associated member"))?
        }
        (
            ReadScopeBinder::AssociatedBinding { member, .. },
            PackageInterfaceDeclarationPayload::OwnedImplementation(implementation),
        ) if header.kind == OwnerKind::OwnedImplementation => {
            let mut found = None;
            for entry in implementation.associated_types {
                admission.step(read)?;
                if entry.member == member {
                    if found.is_some() {
                        return Err(reject("duplicate exact associated binding"));
                    }
                    let AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter, ..
                    } = entry.value
                    else {
                        return Err(reject("Owned binding has no read-scope formal"));
                    };
                    admission.reserve::<ReadScopeParameter>(1)?;
                    found = Some(vec![scope_parameter]);
                }
            }
            found.ok_or_else(|| reject("read scope names no exact associated binding"))?
        }
        _ => {
            return Err(reject(
                "read-scope binder disagrees with its canonical owner kind",
            ));
        }
    };
    if parameters.len() > contract::MAXIMUM_CHILDREN {
        return Err(reject("read-scope inventory exceeds its bound"));
    }
    admission.reserve::<ReadScopeParameterId>(parameters.len())?;
    admission.reserve::<&Name>(parameters.len())?;
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for parameter in &parameters {
        admission.step(read)?;
        if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
            return Err(reject(
                "read scopes require distinct exact identities and names",
            ));
        }
    }
    Ok(parameters)
}

/// Simultaneous substitution replaces a callee formal once. Inserted caller
/// arguments stay opaque, including equal IDs owned by a different binder.
pub(crate) fn scope_bindings(
    read: &(impl ExpressionRead + ?Sized),
    binder: ReadScopeBinder,
    actuals: &[ReadScopeArgument],
    caller: ScopeAuthority<'_>,
    admission: &OwnedAssociatedAdmission,
) -> Result<BTreeMap<ReadScopeReference, ReadScopeArgument>, Diagnostic> {
    let parameters = scope_parameters(read, binder, admission)?;
    if parameters.len() != actuals.len() {
        return Err(reject("read-scope application arity mismatch"));
    }
    admission.reserve::<(ReadScopeReference, ReadScopeArgument)>(actuals.len())?;
    let mut bindings = BTreeMap::new();
    for (formal, actual) in parameters.iter().zip(actuals) {
        caller.validate(read, *actual, admission)?;
        admission.step(read)?;
        let reference = ReadScopeReference {
            binder,
            parameter: formal.id,
        };
        if bindings.insert(reference, *actual).is_some() {
            return Err(reject("duplicate exact read-scope substitution formal"));
        }
    }
    Ok(bindings)
}

pub(crate) fn substitute_scope(
    scope: ReadScopeArgument,
    bindings: &BTreeMap<ReadScopeReference, ReadScopeArgument>,
) -> Result<ReadScopeArgument, Diagnostic> {
    match scope {
        ReadScopeArgument::Parameter(reference) => bindings
            .get(&reference)
            .copied()
            .ok_or_else(|| reject("unbound exact read-scope template")),
        ReadScopeArgument::Lexical { .. } => {
            Err(reject("stored signature cannot capture a lexical loan"))
        }
    }
}

/// Containment follows the exact enclosing loan order, independently of source
/// ancestry. A same-scoped or same-typed source never grants readable rights.
pub(crate) fn prove_reborrow_scope(
    read: &(impl ExpressionRead + ?Sized),
    parent: ReadScopeArgument,
    child: ReadScopeArgument,
    authority: ScopeAuthority<'_>,
    order: &[BindingId],
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    authority.validate(read, parent, admission)?;
    authority.validate(read, child, admission)?;
    if parent == child {
        return Ok(());
    }
    let ReadScopeArgument::Lexical {
        callable,
        binding: child_binding,
    } = child
    else {
        return Err(reject(
            "read scope cannot widen or infer a relationship between formals",
        ));
    };
    if authority.lexical != Some(callable) {
        return Err(reject("foreign lexical child scope"));
    }
    let mut parent_position = None;
    let mut child_position = None;
    let mut seen = BTreeSet::new();
    admission.reserve::<BindingId>(order.len())?;
    for (position, binding) in order.iter().enumerate() {
        admission.step(read)?;
        if !authority.live_loans.contains(binding) || !seen.insert(*binding) {
            return Err(reject(
                "scope order is not the exact live enclosing loan path",
            ));
        }
        if *binding == child_binding {
            child_position = Some(position);
        }
        if matches!(parent, ReadScopeArgument::Lexical { binding: parent_binding, .. } if parent_binding == *binding)
        {
            parent_position = Some(position);
        }
    }
    let child_position =
        child_position.ok_or_else(|| reject("child scope is outside the enclosing loan order"))?;
    let proven = match parent {
        ReadScopeArgument::Parameter(reference) => authority.binder == Some(reference.binder),
        ReadScopeArgument::Lexical {
            callable: owner, ..
        } => owner == callable && parent_position.is_some_and(|position| position < child_position),
    };
    if !proven {
        return Err(reject(
            "reborrow scope has no explicit nested containment proof",
        ));
    }
    Ok(())
}

pub(crate) fn binder(context: AssociatedTypeContext) -> Option<ReadScopeBinder> {
    match context {
        AssociatedTypeContext::Lexical(scope)
        | AssociatedTypeContext::ReadViewDeclaration(scope) => {
            Some(ReadScopeBinder::Declaration(scope))
        }
        AssociatedTypeContext::ContractMethod { contract, method } => {
            Some(ReadScopeBinder::ContractMethod { contract, method })
        }
        AssociatedTypeContext::AssociatedBinding {
            implementation,
            member,
        } => Some(ReadScopeBinder::AssociatedBinding {
            implementation,
            member,
        }),
        AssociatedTypeContext::ContractSignature(_) | AssociatedTypeContext::Forbidden => None,
    }
}

pub(crate) fn lexical(context: AssociatedTypeContext) -> Option<DeclarationReference> {
    match context {
        AssociatedTypeContext::Lexical(scope) => Some(scope),
        _ => None,
    }
}

pub(crate) fn direct(form: &TypeForm) -> bool {
    form.is_read_view()
}

pub(crate) struct ViewLayout {
    pub(crate) declaration: DeclarationReference,
    pub(crate) scope: ReadScopeArgument,
    pub(crate) fields: Vec<ReadViewField>,
    pub(crate) actuals: Vec<TypeObjectDigest>,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
}

/// Representation reads prove a private layout's properties. They provide no
/// constructor, field-access, lexical binder or public naming authority.
fn view_definition(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    admission: &OwnedAssociatedAdmission,
) -> Result<(OwnerHeader, ReadViewDeclaration), Diagnostic> {
    admission.step(read)?;
    let key = OwnerKey::Declaration(reference.declaration);
    let (header, view) = if reference.package == read.package_id() {
        let Some(OwnerRecord::Declaration(record)) = read.owner(key)? else {
            return Err(reject("missing exact view layout declaration"));
        };
        let DeclarationPayload::ReadView(view) = record.payload else {
            return Err(reject("view type names a different declaration kind"));
        };
        (record.header, view)
    } else {
        if !read.has_representation_dependency(reference.package)? {
            return Err(reject(
                "view layout is outside the exact representation closure",
            ));
        }
        let Some(
            PackageInterfaceRecord::Declaration(record)
            | PackageInterfaceRecord::RepresentationDeclaration(record),
        ) = read.package_representation_owner(reference.package, key)?
        else {
            return Err(reject("missing exact imported view layout"));
        };
        let PackageInterfaceDeclarationPayload::ReadView(view) = record.payload else {
            return Err(reject(
                "imported view type names a different declaration kind",
            ));
        };
        (record.header, view)
    };
    if header.owner != key || header.kind != OwnerKind::ReadView {
        return Err(reject("view layout owner identity or kind mismatch"));
    }
    require_generation(header.contract_version)?;
    if view.type_parameters.len() > contract::MAXIMUM_CHILDREN
        || view.fields.len() > contract::MAXIMUM_CHILDREN
    {
        return Err(reject("view declaration inventory exceeds its bound"));
    }
    admission.reserve::<TypeParameterId>(view.type_parameters.len())?;
    admission.reserve::<FieldId>(view.fields.len())?;
    admission.reserve::<&Name>(view.fields.len())?;
    let mut parameters = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for id in &view.type_parameters {
        admission.step(read)?;
        if !parameters.insert(*id) {
            return Err(reject("duplicate exact view type formal"));
        }
    }
    let formal = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(reference),
        parameter: view.scope_parameter.id,
    });
    for field in &view.fields {
        admission.step(read)?;
        if !ids.insert(field.id) || !names.insert(&field.name) {
            return Err(reject("duplicate exact view field identity or name"));
        }
        if let ReadViewFieldKind::ReadReference { read_scope } = field.kind
            && read_scope != formal
        {
            return Err(reject(
                "view field escaped its exact declaration scope formal",
            ));
        }
    }
    Ok((header, view))
}

pub(crate) fn require_access(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    caller: DeclarationReference,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    view_definition(read, reference, admission)?;
    if caller.package != read.package_id() || reference.package != caller.package {
        return Err(reject(
            "view fields belong to their private defining module",
        ));
    }
    let module = |reference: DeclarationReference| -> Result<_, Diagnostic> {
        admission.step(read)?;
        let Some(OwnerRecord::Declaration(record)) =
            read.owner(OwnerKey::Declaration(reference.declaration))?
        else {
            return Err(reject("missing exact view access owner"));
        };
        if record.header.owner != OwnerKey::Declaration(reference.declaration) {
            return Err(reject("foreign view access owner"));
        }
        Ok(record.module)
    };
    if module(reference)? != module(caller)? {
        return Err(reject(
            "view fields belong to their private defining module",
        ));
    }
    Ok(())
}

/// Every actual is checked, including phantom actuals. There is no generic view
/// argument category in this increment, even when a field never mentions it.
pub(crate) fn layout(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    admission: &OwnedAssociatedAdmission,
) -> Result<ViewLayout, Diagnostic> {
    admission.step(read)?;
    require_generation(occurrence.consumer_generation)?;
    let Some(TypeObject {
        form:
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            },
        ..
    }) = read.type_object(ty)?
    else {
        return Err(reject(
            "view layout requires an exact nominal read-view application",
        ));
    };
    layout_form(
        read,
        declaration,
        arguments,
        read_scope,
        occurrence,
        admission,
    )
}

fn layout_form(
    read: &(impl ExpressionRead + ?Sized),
    declaration: DeclarationReference,
    arguments: Vec<TypeObjectDigest>,
    read_scope: ReadScopeArgument,
    occurrence: OwnedTypeOccurrence,
    admission: &OwnedAssociatedAdmission,
) -> Result<ViewLayout, Diagnostic> {
    let (_, view) = view_definition(read, declaration, admission)?;
    if view.type_parameters.len() != arguments.len() {
        return Err(reject("view ordinary argument arity mismatch"));
    }
    admission.reserve::<(TypeParameterId, TypeObjectDigest)>(arguments.len())?;
    let mut ordinary = BTreeMap::new();
    for (id, actual) in view.type_parameters.iter().zip(&arguments) {
        admission.step(read)?;
        let parameter = super::super::owned_record::parameter(read, declaration, *id)?;
        if parameter.header.owner != OwnerKey::TypeParameter(*id)
            || parameter.declaration != declaration.declaration
        {
            return Err(reject(
                "view type formal escapes its exact declaration inventory",
            ));
        }
        if !parameter.constraints.has_owned() && !parameter.constraints.requires_transfer() {
            return Err(reject(
                "view formals require explicit Owned or ordinary Transferable constraints",
            ));
        }
        if contains(read, *actual, admission)? {
            return Err(reject(
                "view cannot occur in an ordinary or Owned phantom argument",
            ));
        }
        super::super::owned_contract::validate_type_argument_constraints_at(
            read,
            parameter.constraints,
            *actual,
            occurrence,
            admission,
        )?;
        if ordinary.insert(*id, *actual).is_some() {
            return Err(reject("duplicate exact view type formal"));
        }
    }
    let formal = ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: view.scope_parameter.id,
    };
    admission.reserve::<(ReadScopeReference, ReadScopeArgument)>(1)?;
    let scopes = BTreeMap::from([(formal, read_scope)]);
    let mut applied =
        super::super::parallel_types::ScopedAppliedTypes::new(read, &ordinary, &scopes, admission);
    admission.reserve::<ReadViewField>(view.fields.len())?;
    let mut fields = Vec::with_capacity(view.fields.len());
    for mut field in view.fields {
        admission.step(read)?;
        admission.reserve_bytes(field.name.as_str().len())?;
        field.ty = applied.substitute(field.ty, 0)?;
        if let ReadViewFieldKind::ReadReference { read_scope: scope } = &mut field.kind {
            *scope = substitute_scope(*scope, &scopes)?;
            if *scope != read_scope {
                return Err(reject("view reference field changes its declaration scope"));
            }
        }
        fields.push(field);
    }
    Ok(ViewLayout {
        declaration,
        scope: read_scope,
        fields,
        actuals: arguments,
        types: applied.into_types(),
    })
}

pub(crate) fn validate_nominal_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let mut active = BTreeSet::new();
    validate_nominal_inner(read, ty, occurrence, admission, &mut active, 0, false)
}

/// Unused declarations still admit every raw field, actual, scope and nominal
/// dependency. Identity actuals preserve exact defining formal authority.
pub(crate) fn validate_declaration_at(
    read: &(impl ExpressionRead + ?Sized),
    declaration: DeclarationReference,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let (header, view) = view_definition(read, declaration, admission)?;
    admission.reserve::<(TypeObjectDigest, TypeObject)>(view.type_parameters.len() + 1)?;
    admission.reserve::<TypeObjectDigest>(view.type_parameters.len())?;
    let mut types = BTreeMap::new();
    let mut arguments = Vec::with_capacity(view.type_parameters.len());
    for id in &view.type_parameters {
        admission.step(read)?;
        let object = TypeObject::new(TypeForm::TypeParameter { parameter: *id })?;
        let digest = encode_type_object(&object)?.0;
        arguments.push(digest);
        types.insert(digest, object);
    }
    let scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: view.scope_parameter.id,
    });
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration,
        arguments,
        read_scope: scope,
    })?;
    let digest = encode_type_object(&object)?.0;
    types.insert(digest, object);
    let overlay = super::super::owned_contract::AppliedTypeRead {
        read,
        types: &types,
    };
    validate_nominal_at(
        &overlay,
        digest,
        OwnedTypeOccurrence {
            parameters: super::TypeParameterContext::Authored(declaration),
            associated: AssociatedTypeContext::ReadViewDeclaration(declaration),
            consumer_generation: header.contract_version,
        },
        admission,
    )
}

pub(crate) fn read_shareable_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    admission.step(read)?;
    if let Some(TypeObject {
        form:
            TypeForm::RegionCap {
                region,
                mode: RegionMode::Shared { .. },
            },
        ..
    }) = read.type_object(ty)?
    {
        return super::super::region::require_identity(read, region, occurrence.parameters);
    }
    let mut active = BTreeSet::new();
    validate_nominal_inner(read, ty, occurrence, admission, &mut active, 0, true)
}

fn validate_nominal_inner(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    admission: &OwnedAssociatedAdmission,
    active: &mut BTreeSet<DeclarationReference>,
    depth: usize,
    shareable: bool,
) -> Result<(), Diagnostic> {
    admission.step(read)?;
    if depth > contract::MAXIMUM_TYPE_DEPTH {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "kernel_read_view_depth",
            "view layout exceeds finite admission",
        ));
    }
    let layout = layout(read, ty, occurrence, admission)?;
    if active.contains(&layout.declaration) {
        return Err(reject("recursive nominal view layout is not admitted"));
    }
    admission.reserve::<DeclarationReference>(1)?;
    active.insert(layout.declaration);
    let overlay = super::super::owned_contract::AppliedTypeRead {
        read,
        types: &layout.types,
    };
    // Keep recursive and cross-helper admission at one reader type while the
    // same overlay retains derived types and forwards every admission hook.
    let read: &dyn ExpressionRead = &overlay;
    // Sharing covers phantom actuals independently of reference-field access.
    // The selected custody source is not an implicit phantom type argument.
    if shareable {
        for actual in &layout.actuals {
            admission.step(read)?;
            if super::super::memory::direct_at(read, *actual, occurrence, admission)? {
                super::super::share::admit_at(read, *actual, occurrence, admission)?;
            } else if super::super::transfer::admit_at(read, *actual, occurrence, admission)? {
                return Err(reject(
                    "ordinary view actual cannot acquire Owned sharing authority",
                ));
            }
        }
    }
    for field in &layout.fields {
        admission.step(read)?;
        let object = read
            .type_object(field.ty)?
            .ok_or_else(|| reject("missing instantiated view field type"))?;
        match field.kind {
            ReadViewFieldKind::Ordinary => {
                if direct(&object.form)
                    || super::super::memory::contains_at(read, field.ty, occurrence, admission)?
                    || super::super::transfer::admit_at(read, field.ty, occurrence, admission)?
                {
                    return Err(reject(
                        "view metadata must have complete ordinary Transferable closure",
                    ));
                }
            }
            ReadViewFieldKind::ReadReference { read_scope } => {
                if read_scope != layout.scope {
                    return Err(reject(
                        "reference field escaped its instantiated view scope",
                    ));
                }
                if matches!(object.form, TypeForm::ReadViewApplied { .. }) {
                    validate_nominal_inner(
                        read,
                        field.ty,
                        occurrence,
                        admission,
                        active,
                        depth + 1,
                        shareable,
                    )?;
                } else if super::super::memory::direct_at(read, field.ty, occurrence, admission)? {
                    super::super::owned_contract::validate_owned_type_at(
                        read, field.ty, occurrence, admission,
                    )?;
                    if shareable {
                        super::super::share::admit_at(read, field.ty, occurrence, admission)?;
                    }
                } else {
                    return Err(reject(
                        "view reference requires an independently admitted Owned or nominal view referent",
                    ));
                }
            }
        }
    }
    active.remove(&layout.declaration);
    Ok(())
}

/// Complete syntactic closure, including nominal layouts and unused actuals.
/// A separate category proof decides whether a root is allowed in its position.
pub(crate) fn contains(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    admission: &OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    admission.reserve::<(TypeObjectDigest, usize)>(1)?;
    let mut pending = vec![(ty, 0usize)];
    let mut seen = BTreeSet::new();
    while let Some((current, depth)) = pending.pop() {
        admission.step(read)?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_read_view_depth",
                "view closure exceeds finite type admission",
            ));
        }
        if seen.contains(&current) {
            continue;
        }
        admission.reserve::<TypeObjectDigest>(1)?;
        seen.insert(current);
        let object = read
            .type_object(current)?
            .ok_or_else(|| reject("missing exact view closure type"))?;
        if direct(&object.form) {
            return Ok(true);
        }
        admission.reserve::<TypeObjectDigest>(object.child_type_count())?;
        for child in object.child_types() {
            admission.step(read)?;
            admission.reserve::<(TypeObjectDigest, usize)>(1)?;
            pending.push((child, depth + 1));
        }
        if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } = object.form
        {
            let property = super::super::owned_record::property_definition(read, declaration)?;
            for id in property.fields {
                admission.step(read)?;
                let field = super::super::owned_record::field(
                    read,
                    FieldReference {
                        package: declaration.package,
                        field: id,
                    },
                )?;
                if field.declaration != declaration.declaration {
                    return Err(reject("foreign nominal field in view containment proof"));
                }
                admission.reserve::<(TypeObjectDigest, usize)>(1)?;
                pending.push((field.ty, depth + 1));
            }
            for id in property.cases {
                admission.step(read)?;
                let case = if declaration.package == read.package_id() {
                    match read.owner(OwnerKey::Case(id))? {
                        Some(OwnerRecord::Case(case)) => case,
                        _ => return Err(reject("missing exact view containment case")),
                    }
                } else {
                    match read
                        .package_representation_owner(declaration.package, OwnerKey::Case(id))?
                    {
                        Some(PackageInterfaceRecord::Case(case)) => case,
                        _ => return Err(reject("missing exact imported view containment case")),
                    }
                };
                if case.header.owner != OwnerKey::Case(id)
                    || case.declaration != declaration.declaration
                {
                    return Err(reject("foreign nominal case in view containment proof"));
                }
                if let Some(ty) = case.payload {
                    admission.reserve::<(TypeObjectDigest, usize)>(1)?;
                    pending.push((ty, depth + 1));
                }
            }
        }
    }
    Ok(false)
}

pub(crate) fn validate_result<S: Copy + Eq>(
    read: &(impl ExpressionRead + ?Sized),
    result: &CallableResult<S>,
    sources: &[(S, TypeObjectDigest, ParameterUse)],
    scope_sources: &[ReadScopeSource<S>],
    pure: bool,
    occurrence: OwnedTypeOccurrence,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    admission.step(read)?;
    let object = read
        .type_object(result.ty)?
        .ok_or_else(|| reject("missing exact callable result type"))?;
    let view = direct(&object.form);
    match &result.relation {
        ResultRelation::Value => {
            if view || contains(read, result.ty, admission)? {
                return Err(reject("Value result cannot carry a read view"));
            }
        }
        ResultRelation::ReadFrom { source } | ResultRelation::ViewFrom { source, .. } => {
            // Both callers separately admit and classify the exact result occurrence.
            // Owned is a source-tied relation requirement; Value needs the complete
            // view-absence proof in its own arm without repeating that classification.
            let owned = super::super::memory::direct_at(read, result.ty, occurrence, admission)?;
            if !pure || (!owned && !view) {
                return Err(reject(
                    "source-tied results require a pure named function and an exact Owned or view result",
                ));
            }
            let mut found = None;
            for (id, ty, usage) in sources {
                admission.step(read)?;
                if id == source {
                    if found.is_some() {
                        return Err(reject("duplicate exact result source"));
                    }
                    found = Some((*ty, *usage));
                }
            }
            let (source_ty, usage) = found.ok_or_else(|| {
                reject("result source is outside its complete ordered parameter inventory")
            })?;
            let source_object = read
                .type_object(source_ty)?
                .ok_or_else(|| reject("missing exact callable source type"))?;
            let source_view = direct(&source_object.form);
            if usage != ParameterUse::Borrow
                || (!source_view
                    && !super::super::memory::direct_at(read, source_ty, occurrence, admission)?)
            {
                return Err(reject(
                    "source-tied result needs an exact borrowed Owned or view source",
                ));
            }
            match &result.relation {
                ResultRelation::ReadFrom { .. } if !owned || view => {
                    return Err(reject("ReadFrom result must remain Owned"));
                }
                ResultRelation::ViewFrom { scope, .. } => {
                    require_generation(occurrence.consumer_generation)?;
                    if !view {
                        return Err(reject(
                            "ViewFrom result must have the positive ReadView category",
                        ));
                    }
                    let result_scope = match object.form {
                        TypeForm::ReadViewApplied { read_scope, .. }
                        | TypeForm::AssociatedReadViewMember { read_scope, .. }
                        | TypeForm::AssociatedReadViewProjection { read_scope, .. }
                        | TypeForm::RegionCap {
                            mode: crate::platform::kernel::RegionMode::Shared { scope: read_scope },
                            ..
                        } => read_scope,
                        _ => return Err(reject("ViewFrom has no view scope")),
                    };
                    if result_scope != *scope {
                        return Err(reject("result type and ViewFrom scopes disagree"));
                    }
                    let inherited = match source_object.form {
                        TypeForm::ReadViewApplied { read_scope, .. }
                        | TypeForm::AssociatedReadViewMember { read_scope, .. }
                        | TypeForm::AssociatedReadViewProjection { read_scope, .. }
                        | TypeForm::RegionCap {
                            mode: crate::platform::kernel::RegionMode::Shared { scope: read_scope },
                            ..
                        } => Some(read_scope),
                        _ => None,
                    };
                    let mut explicit = false;
                    for entry in scope_sources {
                        admission.step(read)?;
                        explicit |= entry.source == *source && entry.scope == *scope;
                    }
                    if inherited != Some(*scope) && !explicit {
                        return Err(reject(
                            "ViewFrom lacks an exact source-scope forwarding relationship",
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
