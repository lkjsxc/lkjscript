//! Exact source/scope and complete occurrence admission for descriptive read-view evidence.
//! This boundary never admits a body, field access, or live read permission.

use super::{PackageInterfaceClosure, PackageRevision, package_error};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use crate::platform::package_interface::{
    AssociatedInterfaceAdmission, AssociatedInterfaceProof, PackageInterfaceValidation,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

fn reject(message: &str) -> Diagnostic {
    package_error(
        DiagnosticClass::Semantic,
        "package_transport_read_view",
        message,
    )
}

fn capacity() -> Diagnostic {
    package_error(
        DiagnosticClass::Resource,
        "package_transport_read_view_capacity",
        "read-view transport proof exceeds its finite preparation capacity",
    )
}

fn current(revision: &PackageRevision) -> Result<(), Diagnostic> {
    if revision.graph_contract_version != contract::GRAPH_CONTRACT_VERSION
        || revision.revision.graph_contract_version != contract::GRAPH_CONTRACT_VERSION
    {
        return Err(reject(
            "read views/scopes require the exact current containing semantic revision",
        ));
    }
    Ok(())
}

fn current_owner(
    owner: &crate::platform::package_interface::PackageInterfaceOwner,
    key: OwnerKey,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    admission.current(owner, key)?;
    if owner.record.header().contract_version != contract::GRAPH_CONTRACT_VERSION
        || owner.contract_version
            != crate::platform::package_interface::PACKAGE_INTERFACE_CONTRACT_VERSION
    {
        return Err(reject(
            "new view/scope occurrence cannot use a frozen consuming interface owner",
        ));
    }
    Ok(())
}

fn scope_parameters(
    parameters: &[ReadScopeParameter],
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    if parameters.len() > contract::MAXIMUM_CHILDREN {
        return Err(capacity());
    }
    admission.charge(parameters.len())?;
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for parameter in parameters {
        if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
            return Err(reject(
                "scope parameter identities and names must be independently unique",
            ));
        }
    }
    Ok(())
}

fn scope(
    actual: &ReadScopeArgument,
    binder: &ReadScopeBinder,
    parameters: &[ReadScopeParameter],
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    admission.step()?;
    let ReadScopeArgument::Parameter(reference) = actual else {
        return Err(reject(
            "descriptive signatures cannot retain lexical activation scopes",
        ));
    };
    if &reference.binder != binder {
        return Err(reject("scope reference belongs to another exact binder"));
    }
    let mut occurrences = 0;
    for parameter in parameters {
        admission.step()?;
        occurrences += usize::from(parameter.id == reference.parameter);
    }
    if occurrences != 1 {
        return Err(reject(
            "scope reference is absent from its complete binder inventory",
        ));
    }
    Ok(())
}

fn view_scope(form: &TypeForm) -> Option<&ReadScopeArgument> {
    match form {
        TypeForm::ReadViewApplied { read_scope, .. }
        | TypeForm::AssociatedReadViewMember { read_scope, .. }
        | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Some(read_scope),
        _ => None,
    }
}

fn object<'a>(
    interface: &'a PackageInterfaceValidation,
    ty: TypeObjectDigest,
) -> Result<&'a TypeObject, Diagnostic> {
    interface
        .type_objects
        .get(&ty)
        .ok_or_else(|| reject("read-view type closure is incomplete"))
}

fn view_type(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    owner: OwnerKey,
    ty: TypeObjectDigest,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<bool, Diagnostic> {
    let form = &object(interface, ty)?.form;
    if view_scope(form).is_none() {
        return Ok(false);
    }
    current(revision)?;
    current_owner(
        interface
            .owners
            .get(&owner)
            .ok_or_else(|| reject("view occurrence owner is absent"))?,
        owner,
        admission,
    )?;
    if let TypeForm::ReadViewApplied {
        declaration,
        arguments,
        ..
    } = form
    {
        nominal_arguments(
            closure,
            revision,
            owner,
            *declaration,
            arguments,
            interface,
            admission,
        )?;
    }
    let mut resolve = |reference| closure.associated_owner(revision, reference).map(Some);
    let proof = admission.validate_form(
        revision.package,
        owner,
        form,
        &interface.owners,
        &interface.type_objects,
        &mut resolve,
    )?;
    if matches!(proof, AssociatedInterfaceProof::DeferredReadView(_)) {
        discharge_view_form(closure, revision, owner, form, interface, admission)?;
        return Ok(true);
    }
    Ok(proof == AssociatedInterfaceProof::ReadView)
}

fn discharge_view_form(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    owner: OwnerKey,
    form: &TypeForm,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    current(revision)?;
    match form {
        TypeForm::ReadViewApplied {
            declaration,
            arguments,
            ..
        } => nominal_arguments(
            closure,
            revision,
            owner,
            *declaration,
            arguments,
            interface,
            admission,
        ),
        TypeForm::AssociatedReadViewMember {
            contract, member, ..
        }
        | TypeForm::AssociatedReadViewProjection {
            contract, member, ..
        } => {
            let target = closure.require_owner(
                revision,
                contract.package,
                OwnerKey::Declaration(contract.declaration),
                &[OwnerKind::OwnedContract],
                "associated read-view family",
            )?;
            admission.current(target, OwnerKey::Declaration(contract.declaration))?;
            if target.record.header().contract_version != contract::GRAPH_CONTRACT_VERSION {
                return Err(reject(
                    "read-view family cannot use a historical member inventory",
                ));
            }
            let PackageInterfaceRecord::Declaration(declaration) = &target.record else {
                return Err(reject("family needs public contract declaration authority"));
            };
            let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &declaration.payload
            else {
                return Err(reject("family target has wrong contract kind"));
            };
            admission.charge(contract.associated_types.len())?;
            if !contract.associated_types.iter().any(|candidate| {
                candidate.id == *member
                    && matches!(candidate.bound, AssociatedTypeBound::ReadViewFamily { .. })
            }) {
                return Err(reject(
                    "deferred view family has wrong exact member kind/arity",
                ));
            }
            Ok(())
        }
        _ => Err(reject("deferred ReadView proof has no read-view form")),
    }
}

pub(super) fn discharge_occurrences(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    owner: OwnerKey,
    roots: &[TypeObjectDigest],
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    admission.charge(roots.len())?;
    let mut pending = roots.iter().map(|ty| (*ty, 0_usize)).collect::<Vec<_>>();
    while let Some((ty, depth)) = pending.pop() {
        admission.step()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = object(interface, ty)?;
        if object.form.is_associated() || view_scope(&object.form).is_some() {
            let mut resolve = |reference| closure.associated_owner(revision, reference).map(Some);
            match admission.validate_form(
                revision.package,
                owner,
                &object.form,
                &interface.owners,
                &interface.type_objects,
                &mut resolve,
            )? {
                AssociatedInterfaceProof::Deferred(_) => {
                    return Err(reject(
                        "exact interface closure left foreign Owned member authority unresolved",
                    ));
                }
                AssociatedInterfaceProof::DeferredReadView(_) => discharge_view_form(
                    closure,
                    revision,
                    owner,
                    &object.form,
                    interface,
                    admission,
                )?,
                AssociatedInterfaceProof::Owned | AssociatedInterfaceProof::ReadView => {}
            }
        }
        admission.charge(object.child_type_count())?;
        pending
            .try_reserve(object.child_type_count())
            .map_err(|_| capacity())?;
        pending.extend(object.child_types().into_iter().map(|ty| (ty, depth + 1)));
    }
    Ok(())
}

/// The direct root permission never propagates into arguments or structural children.
/// This includes nominal phantom actuals and unused first-class callable signatures.
fn occurrences(
    interface: &PackageInterfaceValidation,
    ty: TypeObjectDigest,
    root_view: bool,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    admission.step()?;
    let mut pending = Vec::new();
    pending.try_reserve_exact(1).map_err(|_| capacity())?;
    pending.push((ty, root_view, 0_usize));
    while let Some((ty, permitted, depth)) = pending.pop() {
        admission.step()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = object(interface, ty)?;
        if view_scope(&object.form).is_some() && !permitted {
            return Err(reject(
                "read view is hidden in an ordinary/Owned/container/callable/phantom occurrence",
            ));
        }
        let count = object.child_type_count();
        admission.charge(count)?;
        pending.try_reserve(count).map_err(|_| capacity())?;
        pending.extend(
            object
                .child_types()
                .into_iter()
                .map(|child| (child, false, depth + 1)),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn signature<S: Copy + Eq>(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    owner: OwnerKey,
    binder: ReadScopeBinder,
    parameters: &[(S, TypeObjectDigest, ParameterUse)],
    result: &CallableResult<S>,
    effect: &FunctionEffect,
    formals: &[ReadScopeParameter],
    sources: &[ReadScopeSource<S>],
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    scope_parameters(formals, admission)?;
    if !formals.is_empty()
        || !sources.is_empty()
        || matches!(result.relation, ResultRelation::ViewFrom { .. })
    {
        current(revision)?;
        current_owner(
            interface
                .owners
                .get(&owner)
                .ok_or_else(|| reject("signature owner is absent"))?,
            owner,
            admission,
        )?;
    }
    if sources.len() > contract::MAXIMUM_CHILDREN {
        return Err(capacity());
    }
    for (index, source) in sources.iter().enumerate() {
        admission.step()?;
        scope(&source.scope, &binder, formals, admission)?;
        for previous in &sources[..index] {
            admission.step()?;
            if previous.source == source.source && previous.scope == source.scope {
                return Err(reject("source/scope relationship is duplicated"));
            }
        }
        let (_, ty, use_mode) = parameters
            .iter()
            .find(|(id, _, _)| *id == source.source)
            .ok_or_else(|| {
                reject("source relationship names no exact ordered borrowed parameter")
            })?;
        if *use_mode != ParameterUse::Borrow {
            return Err(reject("source relationships require a borrowed parameter"));
        }
        if let Some(actual) = view_scope(&object(interface, *ty)?.form) {
            scope(actual, &binder, formals, admission)?;
            if !view_type(closure, revision, owner, *ty, interface, admission)? {
                return Err(reject(
                    "view source relationship lacks an admitted parameter scope",
                ));
            }
        } else if !closure.owned_type_at(revision, owner, *ty, admission)? {
            return Err(reject(
                "source-scope relationship requires independently admitted Owned or ReadView input",
            ));
        }
    }
    for (_, ty, use_mode) in parameters {
        admission.step()?;
        let is_view = view_type(closure, revision, owner, *ty, interface, admission)?;
        occurrences(interface, *ty, is_view, admission)?;
        if is_view {
            if *use_mode != ParameterUse::Borrow {
                return Err(reject("read-view parameters permit borrow mode only"));
            }
            scope(
                view_scope(&object(interface, *ty)?.form).expect("proved view form"),
                &binder,
                formals,
                admission,
            )?;
        }
    }
    occurrences(
        interface,
        result.ty,
        matches!(result.relation, ResultRelation::ViewFrom { .. }),
        admission,
    )?;
    let source = match &result.relation {
        ResultRelation::Value => return Ok(()),
        ResultRelation::ReadFrom { source } | ResultRelation::ViewFrom { source, .. } => *source,
    };
    if !matches!(effect, FunctionEffect::Pure) {
        return Err(reject("loan results require pure named functions/methods"));
    }
    let (_, source_ty, use_mode) = parameters
        .iter()
        .find(|(id, _, _)| *id == source)
        .ok_or_else(|| reject("result source is outside its exact complete parameter inventory"))?;
    if *use_mode != ParameterUse::Borrow {
        return Err(reject("result source requires borrow mode"));
    }
    let source_view = view_type(closure, revision, owner, *source_ty, interface, admission)?;
    if !source_view && !closure.owned_type_at(revision, owner, *source_ty, admission)? {
        return Err(reject(
            "loan result source is neither independently admitted Owned nor ReadView",
        ));
    }
    match &result.relation {
        ResultRelation::ReadFrom { .. } => {
            if !closure.owned_type_at(revision, owner, result.ty, admission)? {
                return Err(reject(
                    "ReadFrom returns an independently admitted Owned read",
                ));
            }
        }
        ResultRelation::ViewFrom {
            scope: result_scope,
            ..
        } => {
            scope(result_scope, &binder, formals, admission)?;
            if !view_type(closure, revision, owner, result.ty, interface, admission)?
                || view_scope(&object(interface, result.ty)?.form) != Some(result_scope)
            {
                return Err(reject(
                    "ViewFrom result type and scope differ from its exact result relation",
                ));
            }
            let declared = sources.iter().any(|relationship| {
                relationship.source == source && &relationship.scope == result_scope
            });
            let forwarded = source_view
                && view_scope(&object(interface, *source_ty)?.form) == Some(result_scope);
            if !declared && !forwarded {
                return Err(reject(
                    "ViewFrom lacks exact source forwarding evidence; scopes cannot widen or merge",
                ));
            }
        }
        ResultRelation::Value => unreachable!("ordinary result returned above"),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_binding(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    owner: OwnerKey,
    member: crate::platform::semantic_id::AssociatedTypeMemberId,
    formal: &ReadScopeParameter,
    ty: TypeObjectDigest,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    current(revision)?;
    current_owner(
        interface
            .owners
            .get(&owner)
            .ok_or_else(|| reject("family binding owner is absent"))?,
        owner,
        admission,
    )?;
    let OwnerKey::Declaration(declaration) = owner else {
        return Err(reject(
            "family binding has no exact implementation identity",
        ));
    };
    let TypeForm::ReadViewApplied { read_scope, .. } = &object(interface, ty)?.form else {
        return Err(reject(
            "family binding RHS must be a nominal read-view application",
        ));
    };
    scope(
        read_scope,
        &ReadScopeBinder::AssociatedBinding {
            implementation: DeclarationReference {
                package: revision.package,
                declaration,
            },
            member,
        },
        std::slice::from_ref(formal),
        admission,
    )?;
    occurrences(interface, ty, true, admission)?;
    if !view_type(closure, revision, owner, ty, interface, admission)? {
        return Err(reject(
            "family binding RHS lacks exact ReadView declaration authority",
        ));
    }
    Ok(())
}

pub(super) fn validate_revision(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    for (key, owner) in &interface.owners {
        admission.checkpoint()?;
        match &owner.record {
            PackageInterfaceRecord::Declaration(declaration)
            | PackageInterfaceRecord::RepresentationDeclaration(declaration) => {
                let OwnerKey::Declaration(id) = key else {
                    return Err(reject("declaration identity kind is incorrect"));
                };
                let reference = DeclarationReference {
                    package: revision.package,
                    declaration: *id,
                };
                match &declaration.payload {
                    PackageInterfaceDeclarationPayload::Function(function) => {
                        admission.charge(function.parameters.len())?;
                        let mut parameters = Vec::new();
                        parameters
                            .try_reserve_exact(function.parameters.len())
                            .map_err(|_| capacity())?;
                        for id in &function.parameters {
                            let Some(PackageInterfaceRecord::Parameter(parameter)) = interface
                                .owners
                                .get(&OwnerKey::Parameter(*id))
                                .map(|owner| &owner.record)
                            else {
                                return Err(reject(
                                    "function parameter is absent from its complete interface inventory",
                                ));
                            };
                            if parameter.parent != ParameterParent::Function(reference.declaration)
                            {
                                return Err(reject(
                                    "function parameter belongs to another exact binder",
                                ));
                            }
                            if view_scope(&object(interface, parameter.ty)?.form).is_some() {
                                current_owner(
                                    &interface.owners[&OwnerKey::Parameter(*id)],
                                    OwnerKey::Parameter(*id),
                                    admission,
                                )?;
                            }
                            parameters.push((*id, parameter.ty, parameter.use_mode));
                        }
                        signature(
                            closure,
                            revision,
                            *key,
                            ReadScopeBinder::Declaration(reference),
                            &parameters,
                            &function.result,
                            &function.effect,
                            &function.read_scope_parameters,
                            &function.read_scope_sources,
                            interface,
                            admission,
                        )?;
                        for witness in &function.implementation_parameters {
                            occurrences(interface, witness.self_type, false, admission)?;
                            for ty in &witness.type_arguments {
                                occurrences(interface, *ty, false, admission)?;
                            }
                        }
                    }
                    PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
                        for method in &contract.methods {
                            admission.charge(method.parameters.len())?;
                            let mut parameters = Vec::new();
                            parameters
                                .try_reserve_exact(method.parameters.len())
                                .map_err(|_| capacity())?;
                            for (index, parameter) in method.parameters.iter().enumerate() {
                                parameters.push((
                                    u32::try_from(index).map_err(|_| capacity())?,
                                    parameter.ty,
                                    parameter.use_mode,
                                ));
                            }
                            signature(
                                closure,
                                revision,
                                *key,
                                ReadScopeBinder::ContractMethod {
                                    contract: reference,
                                    method: method.id,
                                },
                                &parameters,
                                &method.result,
                                &method.effect,
                                &method.read_scope_parameters,
                                &method.read_scope_sources,
                                interface,
                                admission,
                            )?;
                        }
                    }
                    PackageInterfaceDeclarationPayload::ReadView(view) => {
                        current(revision)?;
                        current_owner(owner, *key, admission)?;
                        scope_parameters(std::slice::from_ref(&view.scope_parameter), admission)?;
                        admission.charge(view.type_parameters.len())?;
                        let mut formals = BTreeSet::new();
                        for parameter in &view.type_parameters {
                            let Some(PackageInterfaceRecord::TypeParameter(record)) = interface
                                .owners
                                .get(&OwnerKey::TypeParameter(*parameter))
                                .map(|owner| &owner.record)
                            else {
                                return Err(reject(
                                    "read-view type formal inventory is incomplete",
                                ));
                            };
                            if !formals.insert(*parameter)
                                || record.declaration != reference.declaration
                                || !(record.constraints.has_owned()
                                    || record.constraints.requires_transfer())
                            {
                                return Err(reject(
                                    "read-view formals require exact explicit Owned/ordinary-data constraints",
                                ));
                            }
                        }
                        let binder = ReadScopeBinder::Declaration(reference);
                        let declared = ReadScopeArgument::Parameter(ReadScopeReference {
                            binder,
                            parameter: view.scope_parameter.id,
                        });
                        admission.charge(view.fields.len())?;
                        let mut ids = BTreeSet::new();
                        let mut names = BTreeSet::new();
                        for field in &view.fields {
                            if !ids.insert(field.id) || !names.insert(&field.name) {
                                return Err(reject(
                                    "read-view fields require independently unique identities and names",
                                ));
                            }
                            match &field.kind {
                                ReadViewFieldKind::Ordinary => {
                                    occurrences(interface, field.ty, false, admission)?;
                                    ordinary_field(
                                        closure, revision, reference, field.ty, interface,
                                        admission,
                                    )?;
                                }
                                ReadViewFieldKind::ReadReference { read_scope } => {
                                    scope(
                                        read_scope,
                                        &binder,
                                        std::slice::from_ref(&view.scope_parameter),
                                        admission,
                                    )?;
                                    if read_scope != &declared {
                                        return Err(reject(
                                            "reference field scope differs from its declaration scope",
                                        ));
                                    }
                                    let view_referent = view_type(
                                        closure, revision, *key, field.ty, interface, admission,
                                    )?;
                                    occurrences(interface, field.ty, view_referent, admission)?;
                                    if view_referent {
                                        if !matches!(
                                            object(interface, field.ty)?.form,
                                            TypeForm::ReadViewApplied { .. }
                                        ) || view_scope(&object(interface, field.ty)?.form)
                                            != Some(&declared)
                                        {
                                            return Err(reject(
                                                "reference field requires nominal view referent at the exact declaration scope",
                                            ));
                                        }
                                    } else if !closure
                                        .owned_type_at(revision, *key, field.ty, admission)?
                                    {
                                        return Err(reject(
                                            "reference field referent is neither independently admitted Owned nor nominal ReadView",
                                        ));
                                    }
                                }
                            }
                        }
                        finite_layout(closure, revision, reference, admission)?;
                    }
                    PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                        mapped_scopes(closure, revision, implementation, interface, admission)?;
                        occurrences(interface, implementation.self_type, false, admission)?;
                        for ty in &implementation.type_arguments {
                            occurrences(interface, *ty, false, admission)?;
                        }
                        for witness in &implementation.implementation_parameters {
                            occurrences(interface, witness.self_type, false, admission)?;
                            for ty in &witness.type_arguments {
                                occurrences(interface, *ty, false, admission)?;
                            }
                        }
                        for method in &implementation.methods {
                            for ty in &method.type_arguments {
                                occurrences(interface, *ty, false, admission)?;
                            }
                            for operand in &method.implementations {
                                for operand in operand.walk() {
                                    for ty in operand.type_arguments() {
                                        occurrences(interface, *ty, false, admission)?;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        admission.reserve_roots(owner)?;
                        for ty in owner.type_roots() {
                            occurrences(interface, ty, false, admission)?;
                        }
                    }
                }
            }
            PackageInterfaceRecord::Parameter(parameter) => {
                // Named function signatures above own their direct view positions. Operations
                // and externals cannot borrow a view through a detached parameter record.
                let named = match parameter.parent {
                    ParameterParent::Function(id) => matches!(
                        interface
                            .owners
                            .get(&OwnerKey::Declaration(id))
                            .map(|owner| &owner.record),
                        Some(PackageInterfaceRecord::Declaration(
                            PackageInterfaceDeclaration {
                                payload: PackageInterfaceDeclarationPayload::Function(_),
                                ..
                            }
                        ))
                    ),
                    ParameterParent::Operation(_) => false,
                };
                if !named {
                    occurrences(interface, parameter.ty, false, admission)?;
                }
            }
            _ => {
                admission.reserve_roots(owner)?;
                for ty in owner.type_roots() {
                    occurrences(interface, ty, false, admission)?;
                }
            }
        }
    }
    Ok(())
}

fn mapped_scopes(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    implementation: &OwnedImplementation,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    let contract_owner = closure.require_owner(
        revision,
        implementation.contract.package,
        OwnerKey::Declaration(implementation.contract.declaration),
        &[OwnerKind::OwnedContract],
        "mapped method exact contract",
    )?;
    let PackageInterfaceRecord::Declaration(contract_declaration) = &contract_owner.record else {
        return Err(reject(
            "mapped method contract requires public declaration authority",
        ));
    };
    let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &contract_declaration.payload
    else {
        return Err(reject("mapped method target is not an OwnedContract"));
    };
    for mapping in &implementation.methods {
        admission.charge(contract.methods.len())?;
        let method = contract
            .methods
            .iter()
            .find(|method| method.id == mapping.method)
            .ok_or_else(|| reject("mapping names no exact complete contract method"))?;
        let binder = ReadScopeBinder::ContractMethod {
            contract: implementation.contract,
            method: method.id,
        };
        for actual in &mapping.read_scope_arguments {
            scope(actual, &binder, &method.read_scope_parameters, admission)?;
        }
        if mapping.function.package == revision.package
            && !interface
                .owners
                .contains_key(&OwnerKey::Declaration(mapping.function.declaration))
        {
            if (!mapping.read_scope_arguments.is_empty()
                || matches!(method.result.relation, ResultRelation::ViewFrom { .. }))
                && contract_declaration.header.contract_version != contract::GRAPH_CONTRACT_VERSION
            {
                return Err(reject(
                    "mapped view/scopes require exact current function and contract owners",
                ));
            }
            // Private local targets are intentionally absent from this descriptive
            // projection. Source collection independently validates every target
            // and the exact interface projection before producing AdmittedClosure.
            continue;
        }
        let target = closure.require_owner(
            revision,
            mapping.function.package,
            OwnerKey::Declaration(mapping.function.declaration),
            &[OwnerKind::PureFunction, OwnerKind::TaskFunction],
            "mapped method function",
        )?;
        let PackageInterfaceRecord::Declaration(target) = &target.record else {
            return Err(reject(
                "mapped method function requires public declaration authority",
            ));
        };
        let PackageInterfaceDeclarationPayload::Function(function) = &target.payload else {
            return Err(reject("mapped method target is not a named function"));
        };
        if mapping.read_scope_arguments.is_empty()
            && function.read_scope_parameters.is_empty()
            && function.read_scope_sources.is_empty()
            && method.read_scope_parameters.is_empty()
            && method.read_scope_sources.is_empty()
            && !matches!(function.result.relation, ResultRelation::ViewFrom { .. })
            && !matches!(method.result.relation, ResultRelation::ViewFrom { .. })
        {
            continue;
        }
        if mapping.read_scope_arguments.len() != function.read_scope_parameters.len() {
            return Err(reject(
                "mapped method does not preserve the complete target scope arity",
            ));
        }
        let substitute = |actual: ReadScopeArgument| -> Result<ReadScopeArgument, Diagnostic> {
            let ReadScopeArgument::Parameter(reference) = actual else {
                return Err(reject("mapped target signature retains a lexical scope"));
            };
            if reference.binder != ReadScopeBinder::Declaration(mapping.function) {
                return Err(reject(
                    "mapped target signature captures another function's scope",
                ));
            }
            let position = function
                .read_scope_parameters
                .iter()
                .position(|parameter| parameter.id == reference.parameter)
                .ok_or_else(|| reject("mapped target signature has an absent scope formal"))?;
            mapping
                .read_scope_arguments
                .get(position)
                .copied()
                .ok_or_else(|| reject("mapped scope actual is absent"))
        };
        if method.parameters.len() != function.parameters.len() {
            return Err(reject("mapped method parameter arity differs"));
        }
        let source = |id| -> Result<u32, Diagnostic> {
            u32::try_from(
                function
                    .parameters
                    .iter()
                    .position(|parameter| *parameter == id)
                    .ok_or_else(|| {
                        reject("mapped source is outside exact target parameter inventory")
                    })?,
            )
            .map_err(|_| capacity())
        };
        admission.charge(
            function.read_scope_sources.len()
                + function.read_scope_parameters.len()
                + method.read_scope_sources.len(),
        )?;
        if function.read_scope_sources.len() != method.read_scope_sources.len() {
            return Err(reject(
                "mapped method source-scope relationship inventory differs",
            ));
        }
        for (target, expected) in function
            .read_scope_sources
            .iter()
            .zip(&method.read_scope_sources)
        {
            if source(target.source)? != expected.source
                || substitute(target.scope)? != expected.scope
            {
                return Err(reject(
                    "mapped method source-scope relationships differ after exact substitution",
                ));
            }
        }
        let relation = match &function.result.relation {
            ResultRelation::Value => ResultRelation::Value,
            ResultRelation::ReadFrom { source: selected } => ResultRelation::ReadFrom {
                source: source(*selected)?,
            },
            ResultRelation::ViewFrom {
                source: selected,
                scope,
            } => ResultRelation::ViewFrom {
                source: source(*selected)?,
                scope: substitute(*scope)?,
            },
        };
        if relation != method.result.relation {
            return Err(reject(
                "mapped method result kind/source/scope differs after exact substitution",
            ));
        }
        if (!mapping.read_scope_arguments.is_empty()
            || matches!(relation, ResultRelation::ViewFrom { .. }))
            && (target.header.contract_version != contract::GRAPH_CONTRACT_VERSION
                || contract_declaration.header.contract_version != contract::GRAPH_CONTRACT_VERSION)
        {
            return Err(reject(
                "mapped view/scopes require exact current function and contract owners",
            ));
        }
    }
    Ok(())
}

/// Metadata has no access authority: its complete closure is immutable ordinary Transferable.
fn ordinary_field(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    declaration: DeclarationReference,
    ty: TypeObjectDigest,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    type Environment = BTreeMap<
        (
            DeclarationReference,
            crate::platform::semantic_id::TypeParameterId,
        ),
        (PackageId, DeclarationReference, TypeObjectDigest),
    >;
    let mut pending = vec![(
        revision,
        interface,
        declaration,
        ty,
        0_usize,
        Environment::new(),
    )];
    let mut visited = BTreeSet::new();
    while let Some((source, inventory, binder, ty, depth, environment)) = pending.pop() {
        admission.step()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        admission.charge(environment.len())?;
        if !visited.insert((source.package, binder.declaration, ty, environment.clone())) {
            continue;
        }
        let object = object(inventory, ty)?;
        let nominal = match &object.form {
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::List { .. }
            | TypeForm::Map { .. }
            | TypeForm::Option { .. }
            | TypeForm::Result { .. }
            | TypeForm::StructuralRecord { .. } => None,
            TypeForm::TypeParameter { parameter } => {
                if let Some((package, actual_binder, actual)) =
                    environment.get(&(binder, *parameter)).copied()
                {
                    let digest = closure
                        .packages
                        .get(&package)
                        .ok_or_else(|| reject("metadata actual source is absent"))?;
                    let source = closure
                        .revisions
                        .get(digest)
                        .ok_or_else(|| reject("metadata actual revision is absent"))?;
                    let inventory = closure
                        .interfaces
                        .get(digest)
                        .ok_or_else(|| reject("metadata actual inventory is absent"))?;
                    admission.step()?;
                    pending.try_reserve(1).map_err(|_| capacity())?;
                    pending.push((
                        source,
                        inventory,
                        actual_binder,
                        actual,
                        depth + 1,
                        environment,
                    ));
                    continue;
                }
                let Some(PackageInterfaceRecord::TypeParameter(record)) = inventory
                    .owners
                    .get(&OwnerKey::TypeParameter(*parameter))
                    .map(|owner| &owner.record)
                else {
                    return Err(reject(
                        "metadata formal has no independently admitted exact type parameter",
                    ));
                };
                if record.declaration != binder.declaration
                    || record.constraints.has_owned()
                    || !record.constraints.requires_transfer()
                {
                    return Err(reject(
                        "metadata formal lacks exact ordinary Transferable evidence",
                    ));
                }
                None
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                Some(*declaration)
            }
            _ => {
                return Err(reject(
                    "metadata closure contains a view/owner/resource/secret/callable or non-Transferable value",
                ));
            }
        };
        let count = object.child_type_count();
        admission.charge(count)?;
        pending.try_reserve(count).map_err(|_| capacity())?;
        for child in object.child_types() {
            admission.charge(environment.len())?;
            pending.push((
                source,
                inventory,
                binder,
                child,
                depth + 1,
                environment.clone(),
            ));
        }
        if let Some(reference) = nominal {
            let nominal = closure.require_owner(
                source,
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                &[OwnerKind::Record, OwnerKind::Variant],
                "read-view metadata nominal layout",
            )?;
            let target_digest = closure
                .packages
                .get(&reference.package)
                .ok_or_else(|| reject("metadata nominal revision is absent"))?;
            let target_revision = closure
                .revisions
                .get(target_digest)
                .ok_or_else(|| reject("metadata nominal revision is absent"))?;
            let target_inventory = closure
                .interfaces
                .get(target_digest)
                .ok_or_else(|| reject("metadata nominal inventory is absent"))?;
            let nominal = match &nominal.record {
                PackageInterfaceRecord::Declaration(record)
                | PackageInterfaceRecord::RepresentationDeclaration(record) => record,
                _ => return Err(reject("metadata nominal layout lacks declaration evidence")),
            };
            let formals = match &nominal.payload {
                PackageInterfaceDeclarationPayload::Record {
                    type_parameters, ..
                }
                | PackageInterfaceDeclarationPayload::Variant {
                    type_parameters, ..
                } => type_parameters,
                _ => {
                    return Err(reject(
                        "metadata nominal evidence has the wrong layout kind",
                    ));
                }
            };
            let actuals: &[TypeObjectDigest] = match &object.form {
                TypeForm::Applied { arguments, .. } => arguments,
                _ => &[],
            };
            if formals.len() != actuals.len() {
                return Err(reject(
                    "metadata nominal application has the wrong complete arity",
                ));
            }
            admission.charge(environment.len() + formals.len())?;
            let mut field_environment = environment.clone();
            for (formal, actual) in formals.iter().zip(actuals) {
                field_environment.insert((reference, *formal), (source.package, binder, *actual));
            }
            match &nominal.payload {
                PackageInterfaceDeclarationPayload::Record { fields, .. } => {
                    admission.charge(fields.len())?;
                    pending.try_reserve(fields.len()).map_err(|_| capacity())?;
                    for field in fields {
                        let Some(PackageInterfaceRecord::Field(field)) = target_inventory
                            .owners
                            .get(&OwnerKey::Field(*field))
                            .map(|owner| &owner.record)
                        else {
                            return Err(reject("metadata nominal field inventory is incomplete"));
                        };
                        admission.charge(field_environment.len())?;
                        pending.push((
                            target_revision,
                            target_inventory,
                            reference,
                            field.ty,
                            depth + 1,
                            field_environment.clone(),
                        ));
                    }
                }
                PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
                    admission.charge(cases.len())?;
                    pending.try_reserve(cases.len()).map_err(|_| capacity())?;
                    for case in cases {
                        let Some(PackageInterfaceRecord::Case(case)) = target_inventory
                            .owners
                            .get(&OwnerKey::Case(*case))
                            .map(|owner| &owner.record)
                        else {
                            return Err(reject("metadata nominal case inventory is incomplete"));
                        };
                        if let Some(ty) = case.payload {
                            admission.charge(field_environment.len())?;
                            pending.push((
                                target_revision,
                                target_inventory,
                                reference,
                                ty,
                                depth + 1,
                                field_environment.clone(),
                            ));
                        }
                    }
                }
                _ => {
                    return Err(reject(
                        "metadata nominal evidence has the wrong layout kind",
                    ));
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn nominal_arguments(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    owner: OwnerKey,
    reference: DeclarationReference,
    arguments: &[TypeObjectDigest],
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    let target = closure.require_owner(
        revision,
        reference.package,
        OwnerKey::Declaration(reference.declaration),
        &[OwnerKind::ReadView],
        "read-view application declaration",
    )?;
    if matches!(
        target.record,
        PackageInterfaceRecord::RepresentationDeclaration(_)
    ) {
        let public_signature = match interface.owners.get(&owner).map(|owner| &owner.record) {
            Some(PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                payload: PackageInterfaceDeclarationPayload::Function(_),
                ..
            })) => true,
            Some(PackageInterfaceRecord::Parameter(parameter)) => match parameter.parent {
                ParameterParent::Function(declaration) => matches!(
                    interface
                        .owners
                        .get(&OwnerKey::Declaration(declaration))
                        .map(|owner| &owner.record),
                    Some(PackageInterfaceRecord::Declaration(
                        PackageInterfaceDeclaration {
                            payload: PackageInterfaceDeclarationPayload::Function(_),
                            ..
                        }
                    ))
                ),
                ParameterParent::Operation(_) => false,
            },
            _ => false,
        };
        if public_signature {
            return Err(reject(
                "public direct view signature names private representation evidence",
            ));
        }
    }
    admission.current(target, OwnerKey::Declaration(reference.declaration))?;
    let target_digest = closure
        .packages
        .get(&reference.package)
        .ok_or_else(|| reject("read-view application revision is absent"))?;
    let target_revision = closure
        .revisions
        .get(target_digest)
        .ok_or_else(|| reject("read-view application revision is absent"))?;
    current(target_revision)?;
    let target_inventory = closure
        .interfaces
        .get(target_digest)
        .ok_or_else(|| reject("read-view application inventory is absent"))?;
    let declaration = match &target.record {
        PackageInterfaceRecord::Declaration(declaration)
        | PackageInterfaceRecord::RepresentationDeclaration(declaration) => declaration,
        _ => {
            return Err(reject(
                "read-view application lacks nominal declaration evidence",
            ));
        }
    };
    let PackageInterfaceDeclarationPayload::ReadView(view) = &declaration.payload else {
        return Err(reject(
            "read-view application has the wrong declaration kind",
        ));
    };
    if arguments.len() != view.type_parameters.len() {
        return Err(reject(
            "read-view application has the wrong complete type arity",
        ));
    }
    let caller = match interface.owners.get(&owner).map(|owner| &owner.record) {
        Some(
            PackageInterfaceRecord::Declaration(declaration)
            | PackageInterfaceRecord::RepresentationDeclaration(declaration),
        ) => {
            let OwnerKey::Declaration(id) = declaration.header.owner else {
                return Err(reject("application owner has the wrong identity kind"));
            };
            DeclarationReference {
                package: revision.package,
                declaration: id,
            }
        }
        Some(PackageInterfaceRecord::Parameter(parameter)) => {
            let ParameterParent::Function(declaration) = parameter.parent else {
                return Err(reject("view application is outside a named function"));
            };
            DeclarationReference {
                package: revision.package,
                declaration,
            }
        }
        _ => {
            return Err(reject(
                "read-view actuals have no exact enclosing declaration",
            ));
        }
    };
    for (formal, actual) in view.type_parameters.iter().zip(arguments) {
        admission.step()?;
        let Some(PackageInterfaceRecord::TypeParameter(formal)) = target_inventory
            .owners
            .get(&OwnerKey::TypeParameter(*formal))
            .map(|owner| &owner.record)
        else {
            return Err(reject(
                "read-view formal constraint inventory is incomplete",
            ));
        };
        if formal.declaration != reference.declaration {
            return Err(reject("read-view formal belongs to another nominal binder"));
        }
        occurrences(interface, *actual, false, admission)?;
        actual_parameters(caller, *actual, interface, admission)?;
        finite_actual(closure, revision, *actual, interface, admission)?;
        if formal.constraints.has_owned() {
            if !closure.owned_type_at(revision, owner, *actual, admission)? {
                return Err(reject(
                    "read-view actual lacks the formal's exact Owned bound",
                ));
            }
            if formal.constraints.requires_transfer() || formal.constraints.requires_share() {
                owned_permissions(
                    closure,
                    revision,
                    caller,
                    *actual,
                    interface,
                    formal.constraints.requires_transfer(),
                    formal.constraints.requires_share(),
                    admission,
                )?;
            }
        } else if formal.constraints.requires_transfer() {
            ordinary_field(closure, revision, caller, *actual, interface, admission)?;
        } else {
            return Err(reject(
                "read-view formal lacks explicit Owned/ordinary-data admission",
            ));
        }
    }
    Ok(())
}

fn actual_parameters(
    binder: DeclarationReference,
    ty: TypeObjectDigest,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    let mut pending = vec![(ty, 0_usize)];
    while let Some((ty, depth)) = pending.pop() {
        admission.step()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = object(interface, ty)?;
        if let TypeForm::TypeParameter { parameter } = object.form {
            let Some(PackageInterfaceRecord::TypeParameter(record)) = interface
                .owners
                .get(&OwnerKey::TypeParameter(parameter))
                .map(|owner| &owner.record)
            else {
                return Err(reject("application actual type formal is absent"));
            };
            let Some(
                PackageInterfaceRecord::Declaration(declaration)
                | PackageInterfaceRecord::RepresentationDeclaration(declaration),
            ) = interface
                .owners
                .get(&OwnerKey::Declaration(binder.declaration))
                .map(|owner| &owner.record)
            else {
                return Err(reject("application actual binder is absent"));
            };
            let listed = match &declaration.payload {
                PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
                    contract.self_parameter == parameter
                        || contract.type_parameters.contains(&parameter)
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.type_parameters.contains(&parameter)
                }
                PackageInterfaceDeclarationPayload::Function(function) => {
                    function.type_parameters.contains(&parameter)
                }
                PackageInterfaceDeclarationPayload::ReadView(view) => {
                    view.type_parameters.contains(&parameter)
                }
                _ => false,
            };
            if record.declaration != binder.declaration || !listed {
                return Err(reject(
                    "application actual captures a type formal from another exact owner",
                ));
            }
        }
        admission.charge(object.child_type_count())?;
        pending
            .try_reserve(object.child_type_count())
            .map_err(|_| capacity())?;
        pending.extend(
            object
                .child_types()
                .into_iter()
                .map(|child| (child, depth + 1)),
        );
    }
    Ok(())
}

/// Stronger formal bounds remain independent obligations; the nominal view name grants none.
#[allow(clippy::too_many_arguments)]
fn owned_permissions(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    binder: DeclarationReference,
    ty: TypeObjectDigest,
    interface: &PackageInterfaceValidation,
    transfer: bool,
    share: bool,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    type Environment = BTreeMap<
        (
            PackageId,
            crate::platform::semantic_id::DeclarationId,
            crate::platform::semantic_id::TypeParameterId,
        ),
        (PackageId, DeclarationReference, TypeObjectDigest),
    >;
    let mut pending = vec![(revision, interface, binder, ty, 0_usize, Environment::new())];
    while let Some((source, inventory, binder, ty, depth, environment)) = pending.pop() {
        admission.step()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let ty_object = object(inventory, ty)?;
        match &ty_object.form {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => {}
            TypeForm::TypeParameter { parameter } => {
                if let Some((package, actual_binder, actual)) =
                    environment.get(&(binder.package, binder.declaration, *parameter))
                {
                    let digest = closure
                        .packages
                        .get(package)
                        .ok_or_else(|| reject("view property substitution source is absent"))?;
                    let source = closure
                        .revisions
                        .get(digest)
                        .ok_or_else(|| reject("view property substitution revision is absent"))?;
                    let inventory = closure
                        .interfaces
                        .get(digest)
                        .ok_or_else(|| reject("view property substitution inventory is absent"))?;
                    admission.step()?;
                    pending.try_reserve(1).map_err(|_| capacity())?;
                    pending.push((
                        source,
                        inventory,
                        *actual_binder,
                        *actual,
                        depth + 1,
                        environment,
                    ));
                    continue;
                }
                let Some(PackageInterfaceRecord::TypeParameter(parameter)) = inventory
                    .owners
                    .get(&OwnerKey::TypeParameter(*parameter))
                    .map(|owner| &owner.record)
                else {
                    return Err(reject("stronger view bound actual formal is absent"));
                };
                if parameter.declaration != binder.declaration
                    || (transfer && !parameter.constraints.requires_transfer())
                    || (share
                        && parameter.constraints.has_owned()
                        && !parameter.constraints.requires_share())
                    || (!parameter.constraints.has_owned()
                        && !parameter.constraints.requires_transfer())
                {
                    return Err(reject(
                        "view actual does not independently prove its required transfer/share constraint",
                    ));
                }
            }
            TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. } => {
                admission.charge(ty_object.child_type_count())?;
                pending
                    .try_reserve(ty_object.child_type_count())
                    .map_err(|_| capacity())?;
                for child in ty_object.child_types() {
                    admission.charge(environment.len())?;
                    pending.push((
                        source,
                        inventory,
                        binder,
                        child,
                        depth + 1,
                        environment.clone(),
                    ));
                }
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                let target = closure.require_owner(
                    source,
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[
                        OwnerKind::Record,
                        OwnerKind::Variant,
                        OwnerKind::OwnedRecord,
                    ],
                    "view stronger-bound nominal actual",
                )?;
                let target_digest = closure
                    .packages
                    .get(&declaration.package)
                    .ok_or_else(|| reject("view actual nominal revision is absent"))?;
                let target_revision = closure
                    .revisions
                    .get(target_digest)
                    .ok_or_else(|| reject("view actual nominal revision is absent"))?;
                let target_inventory = closure
                    .interfaces
                    .get(target_digest)
                    .ok_or_else(|| reject("view actual nominal inventory is absent"))?;
                let target = match &target.record {
                    PackageInterfaceRecord::Declaration(record)
                    | PackageInterfaceRecord::RepresentationDeclaration(record) => record,
                    _ => return Err(reject("view actual nominal layout is absent")),
                };
                match &target.payload {
                    PackageInterfaceDeclarationPayload::OwnedRecord {
                        fields,
                        type_parameters,
                    } => {
                        // Check complete actuals, including unused formals. A field formal's
                        // stronger proof is supplied by its exact corresponding actual.
                        let arguments: &[TypeObjectDigest] = match &ty_object.form {
                            TypeForm::Applied { arguments, .. } => arguments,
                            _ => &[],
                        };
                        if arguments.len() != type_parameters.len() {
                            return Err(reject("view nominal actual has the wrong complete arity"));
                        }
                        admission.charge(arguments.len() + fields.len() + environment.len())?;
                        pending
                            .try_reserve(arguments.len() + fields.len())
                            .map_err(|_| capacity())?;
                        let mut field_environment = environment.clone();
                        for (formal, actual) in type_parameters.iter().zip(arguments) {
                            field_environment.insert(
                                (declaration.package, declaration.declaration, *formal),
                                (source.package, binder, *actual),
                            );
                            admission.charge(environment.len())?;
                            pending.push((
                                source,
                                inventory,
                                binder,
                                *actual,
                                depth + 1,
                                environment.clone(),
                            ));
                        }
                        for field in fields {
                            let Some(PackageInterfaceRecord::Field(field)) = target_inventory
                                .owners
                                .get(&OwnerKey::Field(*field))
                                .map(|owner| &owner.record)
                            else {
                                return Err(reject("view actual nominal fields are incomplete"));
                            };
                            admission.charge(field_environment.len())?;
                            pending.push((
                                target_revision,
                                target_inventory,
                                *declaration,
                                field.ty,
                                depth + 1,
                                field_environment.clone(),
                            ));
                        }
                    }
                    _ => ordinary_field(closure, source, binder, ty, inventory, admission)?,
                }
            }
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => {
                return Err(reject(
                    "an abstract Owned member alone proves no transfer/share permission",
                ));
            }
            _ => ordinary_field(closure, source, binder, ty, inventory, admission)?,
        }
    }
    Ok(())
}

fn finite_layout(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    root: DeclarationReference,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    enum Work {
        Type(PackageId, TypeObjectDigest, usize),
        Layout(PackageId, DeclarationReference, usize),
        Leave(DeclarationReference),
    }
    let mut pending = vec![Work::Layout(revision.package, root, 0)];
    let mut active = BTreeSet::new();
    while let Some(work) = pending.pop() {
        admission.step()?;
        match work {
            Work::Leave(reference) => {
                active.remove(&reference);
            }
            Work::Type(package, ty, depth) => {
                if depth > contract::MAXIMUM_TYPE_DEPTH {
                    return Err(capacity());
                }
                let digest = closure
                    .packages
                    .get(&package)
                    .ok_or_else(|| reject("layout package is absent"))?;
                let inventory = closure
                    .interfaces
                    .get(digest)
                    .ok_or_else(|| reject("layout type inventory is absent"))?;
                let object = object(inventory, ty)?;
                admission.charge(1 + object.child_type_count())?;
                pending
                    .try_reserve(1 + object.child_type_count())
                    .map_err(|_| capacity())?;
                pending.extend(
                    object
                        .child_types()
                        .into_iter()
                        .map(|ty| Work::Type(package, ty, depth + 1)),
                );
                match object.form {
                    TypeForm::Named { declaration }
                    | TypeForm::Applied { declaration, .. }
                    | TypeForm::ReadViewApplied { declaration, .. } => {
                        pending.push(Work::Layout(package, declaration, depth + 1))
                    }
                    _ => {}
                }
            }
            Work::Layout(package, reference, depth) => {
                if depth > contract::MAXIMUM_TYPE_DEPTH {
                    return Err(capacity());
                }
                if !active.insert(reference) {
                    return Err(reject(
                        "read-view nominal field dependency closure is recursive",
                    ));
                }
                let source_digest = closure
                    .packages
                    .get(&package)
                    .ok_or_else(|| reject("layout source package is absent"))?;
                let source = closure
                    .revisions
                    .get(source_digest)
                    .ok_or_else(|| reject("layout source revision is absent"))?;
                let owner = closure.require_owner(
                    source,
                    reference.package,
                    OwnerKey::Declaration(reference.declaration),
                    &[
                        OwnerKind::ReadView,
                        OwnerKind::OwnedRecord,
                        OwnerKind::Record,
                        OwnerKind::Variant,
                    ],
                    "complete view field dependency",
                )?;
                let target_digest = closure
                    .packages
                    .get(&reference.package)
                    .ok_or_else(|| reject("layout target package is absent"))?;
                let inventory = closure
                    .interfaces
                    .get(target_digest)
                    .ok_or_else(|| reject("layout target inventory is absent"))?;
                let declaration = match &owner.record {
                    PackageInterfaceRecord::Declaration(record)
                    | PackageInterfaceRecord::RepresentationDeclaration(record) => record,
                    _ => return Err(reject("layout target is not nominal declaration evidence")),
                };
                admission.charge(1)?;
                pending.try_reserve(1).map_err(|_| capacity())?;
                pending.push(Work::Leave(reference));
                match &declaration.payload {
                    PackageInterfaceDeclarationPayload::ReadView(view) => {
                        admission.charge(view.fields.len())?;
                        pending
                            .try_reserve(view.fields.len())
                            .map_err(|_| capacity())?;
                        pending.extend(
                            view.fields
                                .iter()
                                .map(|field| Work::Type(reference.package, field.ty, depth + 1)),
                        );
                    }
                    PackageInterfaceDeclarationPayload::Record { fields, .. }
                    | PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } => {
                        admission.charge(fields.len())?;
                        pending.try_reserve(fields.len()).map_err(|_| capacity())?;
                        for field in fields {
                            let Some(PackageInterfaceRecord::Field(field)) = inventory
                                .owners
                                .get(&OwnerKey::Field(*field))
                                .map(|owner| &owner.record)
                            else {
                                return Err(reject("complete nominal field inventory is absent"));
                            };
                            pending.push(Work::Type(reference.package, field.ty, depth + 1));
                        }
                    }
                    PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
                        admission.charge(cases.len())?;
                        pending.try_reserve(cases.len()).map_err(|_| capacity())?;
                        for case in cases {
                            let Some(PackageInterfaceRecord::Case(case)) = inventory
                                .owners
                                .get(&OwnerKey::Case(*case))
                                .map(|owner| &owner.record)
                            else {
                                return Err(reject("complete nominal case inventory is absent"));
                            };
                            if let Some(ty) = case.payload {
                                pending.push(Work::Type(reference.package, ty, depth + 1));
                            }
                        }
                    }
                    _ => return Err(reject("view layout dependency has the wrong nominal kind")),
                }
            }
        }
    }
    Ok(())
}

fn finite_actual(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    ty: TypeObjectDigest,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    let mut pending = vec![(ty, 0_usize)];
    while let Some((ty, depth)) = pending.pop() {
        admission.step()?;
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = object(interface, ty)?;
        if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } = object.form
        {
            finite_layout(closure, revision, declaration, admission)?;
        }
        admission.charge(object.child_type_count())?;
        pending
            .try_reserve(object.child_type_count())
            .map_err(|_| capacity())?;
        pending.extend(object.child_types().into_iter().map(|ty| (ty, depth + 1)));
    }
    Ok(())
}
