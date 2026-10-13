//! Independent admission of descriptive read-view signatures and nominal layouts.
//!
//! An interface carries symbolic scopes only. Naming a declaration never imports
//! its scope binders, and this module grants no construction or field authority.

use super::{
    MAXIMUM_PACKAGE_INTERFACE_VALIDATION_WORK, PACKAGE_INTERFACE_CONTRACT_VERSION,
    PackageInterfaceOwner, RepresentationBudget, interface_error,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    AssociatedTypeBindingValue, AssociatedTypeBound, CallableResult, DeclarationReference,
    FunctionEffect, OwnerKey, OwnerKind, PackageId, PackageInterfaceDeclaration,
    PackageInterfaceDeclarationPayload, PackageInterfaceRecord, ParameterParent, ParameterUse,
    ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference, ReadScopeSource,
    ReadViewDeclaration, ReadViewFieldKind, ResultRelation, TypeForm, TypeObject, TypeObjectDigest,
    TypeParameterConstraints,
};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

fn reject(message: &str) -> Diagnostic {
    interface_error(
        DiagnosticClass::Semantic,
        "package_interface_read_view",
        message,
    )
}

fn capacity() -> Diagnostic {
    interface_error(
        DiagnosticClass::Resource,
        "package_interface_read_view_capacity",
        "read-view interface admission exceeds its finite preparation capacity",
    )
}

fn current(owner: &PackageInterfaceOwner, key: OwnerKey) -> Result<(), Diagnostic> {
    if owner.owner() != key
        || owner.contract_version != PACKAGE_INTERFACE_CONTRACT_VERSION
        || owner.record.header().contract_version
            != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
    {
        return Err(reject(
            "read views and scopes require their exact current interface and graph owners",
        ));
    }
    Ok(())
}

fn declaration(owner: &PackageInterfaceOwner) -> Result<&PackageInterfaceDeclaration, Diagnostic> {
    match &owner.record {
        PackageInterfaceRecord::Declaration(record)
        | PackageInterfaceRecord::RepresentationDeclaration(record) => Ok(record),
        _ => Err(reject(
            "read-view declaration identity has a foreign record kind",
        )),
    }
}

fn source_declaration(
    package: PackageId,
    source: OwnerKey,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<DeclarationReference, Diagnostic> {
    let owner = owners
        .get(&source)
        .ok_or_else(|| reject("read-view consuming owner is absent"))?;
    let id = match (&owner.record, source) {
        (
            PackageInterfaceRecord::Declaration(_)
            | PackageInterfaceRecord::RepresentationDeclaration(_),
            OwnerKey::Declaration(id),
        ) => id,
        (PackageInterfaceRecord::Parameter(parameter), OwnerKey::Parameter(id)) => {
            let ParameterParent::Function(parent) = parameter.parent else {
                return Err(reject(
                    "an operation parameter cannot import a named read-scope binder",
                ));
            };
            let parent_owner = owners
                .get(&OwnerKey::Declaration(parent))
                .ok_or_else(|| reject("read-view parameter parent is absent"))?;
            let PackageInterfaceDeclarationPayload::Function(signature) =
                &declaration(parent_owner)?.payload
            else {
                return Err(reject(
                    "read-view parameter parent is not a named function signature",
                ));
            };
            if owner.owner() != source
                || owner.kind() != OwnerKind::Parameter
                || signature
                    .parameters
                    .iter()
                    .filter(|candidate| **candidate == id)
                    .count()
                    != 1
            {
                return Err(reject(
                    "read-view parameter is outside its exact complete function inventory",
                ));
            }
            parent
        }
        _ => {
            return Err(reject(
                "this interface owner cannot contain a read scope or view",
            ));
        }
    };
    Ok(DeclarationReference {
        package,
        declaration: id,
    })
}

fn binder_declaration(binder: &ReadScopeBinder) -> DeclarationReference {
    match binder {
        ReadScopeBinder::Declaration(reference)
        | ReadScopeBinder::ContractMethod {
            contract: reference,
            ..
        }
        | ReadScopeBinder::AssociatedMember {
            contract: reference,
            ..
        }
        | ReadScopeBinder::AssociatedBinding {
            implementation: reference,
            ..
        } => *reference,
    }
}

fn scope_inventory<'a>(
    package: PackageId,
    binder: &ReadScopeBinder,
    owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<&'a [ReadScopeParameter], Diagnostic> {
    let reference = binder_declaration(binder);
    if reference.package != package {
        return Err(reject(
            "a descriptive occurrence cannot import a foreign read-scope binder",
        ));
    }
    let key = OwnerKey::Declaration(reference.declaration);
    let owner = owners
        .get(&key)
        .ok_or_else(|| reject("read-scope binder declaration is absent"))?;
    current(owner, key)?;
    let record = declaration(owner)?;
    let parameters = match (binder, &record.payload) {
        (
            ReadScopeBinder::Declaration(_),
            PackageInterfaceDeclarationPayload::Function(function),
        ) if record.header.kind
            == match &function.effect {
                FunctionEffect::Pure => OwnerKind::PureFunction,
                FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
            } =>
        {
            function.read_scope_parameters.as_slice()
        }
        (ReadScopeBinder::Declaration(_), PackageInterfaceDeclarationPayload::ReadView(view))
            if record.header.kind == OwnerKind::ReadView =>
        {
            std::slice::from_ref(&view.scope_parameter)
        }
        (
            ReadScopeBinder::ContractMethod { method, .. },
            PackageInterfaceDeclarationPayload::OwnedContract(contract),
        ) if record.header.kind == OwnerKind::OwnedContract => {
            let mut selected = contract
                .methods
                .iter()
                .filter(|candidate| candidate.id == *method);
            let method = selected
                .next()
                .ok_or_else(|| reject("read-scope method is absent from its exact contract"))?;
            if selected.next().is_some() {
                return Err(reject("read-scope method identity is duplicated"));
            }
            method.read_scope_parameters.as_slice()
        }
        (
            ReadScopeBinder::AssociatedMember { member, .. },
            PackageInterfaceDeclarationPayload::OwnedContract(contract),
        ) if record.header.kind == OwnerKind::OwnedContract => {
            let mut selected = contract
                .associated_types
                .iter()
                .filter(|candidate| candidate.id == *member);
            let member = selected
                .next()
                .ok_or_else(|| reject("scope family member is absent from its exact contract"))?;
            if selected.next().is_some() {
                return Err(reject("scope family member identity is duplicated"));
            }
            let AssociatedTypeBound::ReadViewFamily { scope_parameter } = &member.bound else {
                return Err(reject("an Owned member has no read-scope formal"));
            };
            std::slice::from_ref(scope_parameter)
        }
        (
            ReadScopeBinder::AssociatedBinding { member, .. },
            PackageInterfaceDeclarationPayload::OwnedImplementation(implementation),
        ) if record.header.kind == OwnerKind::OwnedImplementation => {
            let mut selected = implementation
                .associated_types
                .iter()
                .filter(|candidate| candidate.member == *member);
            let binding = selected.next().ok_or_else(|| {
                reject("scope family binding is absent from its exact implementation")
            })?;
            if selected.next().is_some() {
                return Err(reject("scope family binding identity is duplicated"));
            }
            let AssociatedTypeBindingValue::ReadViewFamily {
                scope_parameter, ..
            } = &binding.value
            else {
                return Err(reject("an Owned binding has no read-scope formal"));
            };
            std::slice::from_ref(scope_parameter)
        }
        _ => {
            return Err(reject(
                "scope binder kind disagrees with its exact declaration inventory",
            ));
        }
    };
    validate_scope_inventory(parameters)?;
    Ok(parameters)
}

fn validate_scope_inventory(parameters: &[ReadScopeParameter]) -> Result<(), Diagnostic> {
    if parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
        return Err(capacity());
    }
    let mut budget = RepresentationBudget::default();
    budget.reserve::<crate::platform::semantic_id::ReadScopeParameterId>(parameters.len())?;
    budget.reserve::<&crate::platform::kernel::Name>(parameters.len())?;
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for parameter in parameters {
        if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
            return Err(reject(
                "scope formal identities and names must be independently unique",
            ));
        }
    }
    Ok(())
}

/// Admit the exact owning binder of a descriptive occurrence. Inline contexts
/// are narrowed further by `validate_scope_argument_in_binder` at their owner.
pub(crate) fn validate_scope_argument(
    package: PackageId,
    source: OwnerKey,
    scope: &ReadScopeArgument,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<(), Diagnostic> {
    let ReadScopeArgument::Parameter(reference) = scope else {
        return Err(reject(
            "lexical activation scopes cannot appear in implementation-free interfaces",
        ));
    };
    validate_scope_argument_in_binder(package, source, scope, &reference.binder, owners)
}

pub(crate) fn validate_scope_argument_in_binder(
    package: PackageId,
    source: OwnerKey,
    scope: &ReadScopeArgument,
    binder: &ReadScopeBinder,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<(), Diagnostic> {
    let source_reference = source_declaration(package, source, owners)?;
    if binder_declaration(binder) != source_reference {
        return Err(reject("read scope escaped its exact consuming declaration"));
    }
    current(
        owners
            .get(&source)
            .ok_or_else(|| reject("read-scope source owner is absent"))?,
        source,
    )?;
    let ReadScopeArgument::Parameter(reference) = scope else {
        return Err(reject(
            "lexical activation scopes cannot appear in implementation-free interfaces",
        ));
    };
    if &reference.binder != binder {
        return Err(reject(
            "read scope belongs to another exact method, member, binding, or declaration",
        ));
    }
    let parameters = scope_inventory(package, binder, owners)?;
    if parameters
        .iter()
        .filter(|parameter| parameter.id == reference.parameter)
        .count()
        != 1
    {
        return Err(reject(
            "read scope is absent from its exact complete formal inventory",
        ));
    }
    Ok(())
}

#[derive(Clone)]
struct Context {
    source: OwnerKey,
    reference: DeclarationReference,
    binder: Option<ReadScopeBinder>,
    finite_layout: bool,
    actuals: BTreeMap<TypeParameterId, Actual>,
}

#[derive(Clone)]
struct Actual {
    ty: TypeObjectDigest,
    context: Rc<Context>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Category {
    Ordinary,
    Owned,
    ReadView,
    Other,
    Deferred,
}

#[derive(Clone, Copy)]
struct Properties {
    category: Category,
    transfer: bool,
    share: bool,
    capture: bool,
}

impl Properties {
    const fn new(category: Category) -> Self {
        Self {
            category,
            transfer: false,
            share: false,
            capture: false,
        }
    }
    const fn ordinary() -> Self {
        Self {
            category: Category::Ordinary,
            transfer: true,
            share: true,
            capture: true,
        }
    }
    const fn deferred() -> Self {
        Self::new(Category::Deferred)
    }
}

struct Admission<'a, 't, 'r> {
    package: PackageId,
    owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &'t BTreeMap<TypeObjectDigest, TypeObject>,
    resolve: &'r mut dyn FnMut(
        DeclarationReference,
    ) -> Result<Option<&'a PackageInterfaceOwner>, Diagnostic>,
    work: usize,
    storage: RepresentationBudget,
    deferred: bool,
    visit: &'r mut dyn FnMut(u64) -> Result<(), Diagnostic>,
}

impl<'a, 't> Admission<'a, 't, '_> {
    fn step(&mut self, depth: usize) -> Result<(), Diagnostic> {
        self.charge(1, depth)
    }

    fn charge(&mut self, count: usize, depth: usize) -> Result<(), Diagnostic> {
        (self.visit)(u64::try_from(count).map_err(|_| capacity())?)?;
        self.work = self
            .work
            .checked_add(count)
            .filter(|work| *work <= MAXIMUM_PACKAGE_INTERFACE_VALIDATION_WORK)
            .ok_or_else(capacity)?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        Ok(())
    }

    fn object(&self, ty: TypeObjectDigest) -> Result<&'t TypeObject, Diagnostic> {
        self.types
            .get(&ty)
            .ok_or_else(|| reject("read-view interface type closure is incomplete"))
    }

    fn owner(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<Option<&'a PackageInterfaceOwner>, Diagnostic> {
        self.step(0)?;
        if reference.package == self.package {
            self.owners
                .get(&OwnerKey::Declaration(reference.declaration))
                .map(Some)
                .ok_or_else(|| reject("local read-view nominal declaration is absent"))
        } else {
            let owner = (self.resolve)(reference)?;
            self.deferred = true;
            Ok(owner)
        }
    }

    fn context(
        &self,
        source: OwnerKey,
        binder: Option<ReadScopeBinder>,
    ) -> Result<Context, Diagnostic> {
        Ok(Context {
            source,
            reference: source_declaration(self.package, source, self.owners)?,
            binder,
            finite_layout: false,
            actuals: BTreeMap::new(),
        })
    }

    fn formal_occurrences(
        &self,
        reference: DeclarationReference,
        parameter: TypeParameterId,
    ) -> Result<(usize, usize), Diagnostic> {
        if reference.package != self.package {
            return Err(reject(
                "foreign type formals need their exact defining interface",
            ));
        }
        let record = declaration(
            self.owners
                .get(&OwnerKey::Declaration(reference.declaration))
                .ok_or_else(|| reject("type-formal declaration is absent"))?,
        )?;
        let ids = match &record.payload {
            PackageInterfaceDeclarationPayload::Function(function) => &function.type_parameters,
            PackageInterfaceDeclarationPayload::External(function) => &function.type_parameters,
            PackageInterfaceDeclarationPayload::Record {
                type_parameters, ..
            }
            | PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters, ..
            }
            | PackageInterfaceDeclarationPayload::Variant {
                type_parameters, ..
            } => type_parameters,
            PackageInterfaceDeclarationPayload::ReadView(view) => &view.type_parameters,
            PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                &implementation.type_parameters
            }
            PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
                return Ok((
                    usize::from(contract.self_parameter == parameter)
                        + contract
                            .type_parameters
                            .iter()
                            .filter(|id| **id == parameter)
                            .count(),
                    contract
                        .type_parameters
                        .len()
                        .checked_add(1)
                        .ok_or_else(capacity)?,
                ));
            }
            _ => return Ok((0, 0)),
        };
        Ok((ids.iter().filter(|id| **id == parameter).count(), ids.len()))
    }

    fn formal(
        &mut self,
        context: &Context,
        parameter: TypeParameterId,
        depth: usize,
    ) -> Result<TypeParameterConstraints, Diagnostic> {
        self.step(depth)?;
        let (occurrences, count) = self.formal_occurrences(context.reference, parameter)?;
        self.charge(count, depth)?;
        if occurrences != 1 {
            return Err(reject(
                "type parameter is outside its exact complete defining inventory",
            ));
        }
        let key = OwnerKey::TypeParameter(parameter);
        let owner = self
            .owners
            .get(&key)
            .ok_or_else(|| reject("read-view type formal owner is absent"))?;
        let PackageInterfaceRecord::TypeParameter(record) = &owner.record else {
            return Err(reject("read-view type formal has a foreign owner kind"));
        };
        if owner.owner() != key
            || record.header.kind != OwnerKind::TypeParameter
            || record.declaration != context.reference.declaration
        {
            return Err(reject("type formal belongs to another exact declaration"));
        }
        Ok(record.constraints)
    }

    fn scope(
        &mut self,
        context: &Context,
        actual: &ReadScopeArgument,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.step(depth)?;
        if let ReadScopeArgument::Parameter(reference) = actual {
            self.charge(
                scope_inventory(self.package, &reference.binder, self.owners)?.len(),
                depth,
            )?;
        }
        if let Some(binder) = &context.binder {
            validate_scope_argument_in_binder(
                self.package,
                context.source,
                actual,
                binder,
                self.owners,
            )
        } else {
            validate_scope_argument(self.package, context.source, actual, self.owners)
        }
    }

    fn constraints(
        &mut self,
        context: &Context,
        ty: TypeObjectDigest,
        constraints: TypeParameterConstraints,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        let properties = self.occurrence(context, ty, false, false, &mut BTreeSet::new(), depth)?;
        if properties.category == Category::Deferred {
            return Ok(());
        }
        if constraints.has_owned() {
            if properties.category != Category::Owned {
                return Err(reject(
                    "Owned type actual cannot be a read view or ordinary value",
                ));
            }
        } else if matches!(properties.category, Category::Owned | Category::ReadView) {
            return Err(reject(
                "ordinary type actual cannot contain a read view or owner",
            ));
        }
        if constraints.requires_transfer() && !properties.transfer
            || constraints.requires_share() && !properties.share
            || constraints.requires_capture_safe() && !properties.capture
        {
            return Err(reject(
                "type actual does not prove every explicit formal bound",
            ));
        }
        Ok(())
    }

    fn arguments(
        &mut self,
        context: &Context,
        reference: DeclarationReference,
        parameters: &[TypeParameterId],
        arguments: &[TypeObjectDigest],
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.step(depth)?;
        if parameters.len() != arguments.len() {
            return Err(reject(
                "read-view nominal application has the wrong exact ordered arity",
            ));
        }
        if parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
            return Err(capacity());
        }
        let defining = Context {
            source: OwnerKey::Declaration(reference.declaration),
            reference,
            binder: None,
            finite_layout: context.finite_layout,
            actuals: BTreeMap::new(),
        };
        for (parameter, argument) in parameters.iter().zip(arguments) {
            self.step(depth)?;
            // Every argument is visited, including phantom and unused actuals.
            let constraints = self.formal(&defining, *parameter, depth)?;
            self.constraints(context, *argument, constraints, depth + 1)?;
        }
        Ok(())
    }

    fn defining_context(
        &mut self,
        caller: &Context,
        reference: DeclarationReference,
        parameters: &[TypeParameterId],
        arguments: &[TypeObjectDigest],
        binder: Option<ReadScopeBinder>,
    ) -> Result<Context, Diagnostic> {
        self.storage.reserve::<Context>(1)?;
        self.storage
            .reserve::<(TypeParameterId, Actual)>(parameters.len())?;
        self.storage
            .reserve::<(TypeParameterId, Actual)>(caller.actuals.len())?;
        let caller = Rc::new(caller.clone());
        let mut actuals = BTreeMap::new();
        for (parameter, ty) in parameters.iter().zip(arguments) {
            self.step(0)?;
            if actuals
                .insert(
                    *parameter,
                    Actual {
                        ty: *ty,
                        context: Rc::clone(&caller),
                    },
                )
                .is_some()
            {
                return Err(reject("nominal declaration repeats a type formal identity"));
            }
        }
        Ok(Context {
            source: OwnerKey::Declaration(reference.declaration),
            reference,
            binder,
            finite_layout: caller.finite_layout,
            actuals,
        })
    }

    fn associated(
        &mut self,
        context: &Context,
        form: &TypeForm,
        depth: usize,
    ) -> Result<Properties, Diagnostic> {
        self.step(depth)?;
        let (contract_reference, member, view, lexical) = match form {
            TypeForm::AssociatedContractMember { contract, member } => {
                (*contract, *member, false, None)
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                self.scope(context, read_scope, depth)?;
                (*contract, *member, true, None)
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                if *scope != context.reference {
                    return Err(reject(
                        "associated projection escaped its exact lexical binder",
                    ));
                }
                (*contract, *member, false, Some(*parameter))
            }
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                if *scope != context.reference {
                    return Err(reject("view projection escaped its exact lexical binder"));
                }
                self.scope(context, read_scope, depth)?;
                (*contract, *member, true, Some(*parameter))
            }
            _ => {
                return Err(reject(
                    "associated admission requires an associated type form",
                ));
            }
        };
        if let Some(parameter) = lexical {
            let record = declaration(
                self.owners
                    .get(&OwnerKey::Declaration(context.reference.declaration))
                    .ok_or_else(|| reject("associated projection binder is absent"))?,
            )?;
            let parameters = match &record.payload {
                PackageInterfaceDeclarationPayload::Function(function) => {
                    &function.implementation_parameters
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                    &implementation.implementation_parameters
                }
                _ => {
                    return Err(reject(
                        "associated projection binder is not a function or implementation",
                    ));
                }
            };
            let mut selected = parameters
                .iter()
                .filter(|candidate| candidate.id == parameter);
            let witness = selected
                .next()
                .ok_or_else(|| reject("associated projection has no exact witness formal"))?;
            if selected.next().is_some() || witness.contract != contract_reference {
                return Err(reject(
                    "associated projection differs from its exact witness contract",
                ));
            }
            self.occurrence(
                context,
                witness.self_type,
                false,
                false,
                &mut BTreeSet::new(),
                depth + 1,
            )?;
            for ty in &witness.type_arguments {
                self.occurrence(context, *ty, false, false, &mut BTreeSet::new(), depth + 1)?;
            }
        } else if contract_reference != context.reference
            || !matches!(context.binder, Some(ReadScopeBinder::ContractMethod { .. }))
        {
            return Err(reject(
                "contract member occurrence escaped its exact method signature",
            ));
        }
        let Some(owner) = self.owner(contract_reference)? else {
            return Ok(if view {
                Properties::new(Category::ReadView)
            } else {
                Properties::deferred()
            });
        };
        if view {
            current(owner, OwnerKey::Declaration(contract_reference.declaration))?;
        }
        let record = declaration(owner)?;
        let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &record.payload else {
            return Err(reject(
                "associated member does not name an exact Owned contract",
            ));
        };
        if record.header.kind != OwnerKind::OwnedContract {
            return Err(reject("associated contract has a foreign owner kind"));
        }
        let mut selected = contract
            .associated_types
            .iter()
            .filter(|candidate| candidate.id == member);
        let member_record = selected.next().ok_or_else(|| {
            reject("associated member is absent from its complete contract inventory")
        })?;
        if selected.next().is_some() {
            return Err(reject(
                "associated member identity is duplicated in its exact contract",
            ));
        }
        let bound_view = matches!(
            &member_record.bound,
            AssociatedTypeBound::ReadViewFamily { .. }
        );
        if view != bound_view {
            return Err(reject(
                "associated member form disagrees with its exact kind and arity",
            ));
        }
        Ok(Properties::new(if view {
            Category::ReadView
        } else {
            Category::Owned
        }))
    }

    fn occurrence(
        &mut self,
        context: &Context,
        ty: TypeObjectDigest,
        view_allowed: bool,
        projection_free: bool,
        nominal_path: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<Properties, Diagnostic> {
        self.step(depth)?;
        let form = &self.object(ty)?.form;
        if let TypeForm::TypeParameter { parameter } = form
            && let Some(actual) = context.actuals.get(parameter)
        {
            return self.occurrence(
                &actual.context,
                actual.ty,
                view_allowed,
                projection_free,
                nominal_path,
                depth + 1,
            );
        }
        let view = matches!(
            form,
            TypeForm::ReadViewApplied { .. }
                | TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
        );
        if view && !view_allowed {
            return Err(reject(
                "read view escaped through a container, ordinary/Owned field, callable, or phantom argument",
            ));
        }
        if projection_free && form.is_associated() {
            return Err(reject(
                "stored associated binding closure contains an associated member or projection",
            ));
        }
        self.storage
            .reserve::<TypeObjectDigest>(self.object(ty)?.child_type_count())?;
        let children = self.object(ty)?.child_types();
        let mut child_properties = Vec::new();
        self.storage.reserve::<Properties>(children.len())?;
        child_properties
            .try_reserve_exact(children.len())
            .map_err(|_| capacity())?;
        for child in children {
            child_properties.push(self.occurrence(
                context,
                child,
                false,
                projection_free,
                nominal_path,
                depth + 1,
            )?);
        }
        let all_transfer = child_properties.iter().all(|property| property.transfer);
        let all_share = child_properties.iter().all(|property| property.share);
        let all_capture = child_properties.iter().all(|property| property.capture);
        let any_deferred = child_properties
            .iter()
            .any(|property| property.category == Category::Deferred);
        let properties = match form {
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText => Properties::ordinary(),
            TypeForm::TypeParameter { parameter } => {
                let constraints = self.formal(context, *parameter, depth)?;
                Properties {
                    category: if constraints.has_owned() {
                        Category::Owned
                    } else {
                        Category::Ordinary
                    },
                    transfer: constraints.requires_transfer(),
                    share: constraints.requires_share()
                        || !constraints.has_owned() && constraints.requires_transfer(),
                    capture: constraints.proves_capture_safe(),
                }
            }
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Properties {
                category: Category::Owned,
                transfer: true,
                share: true,
                capture: false,
            },
            TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. } => {
                if child_properties.iter().any(|property| {
                    !matches!(
                        property.category,
                        Category::Owned | Category::Ordinary | Category::Deferred
                    ) || property.category == Category::Ordinary && !property.transfer
                }) {
                    return Err(reject(
                        "Owned layout child must be admitted Owned or ordinary Transferable data",
                    ));
                }
                if any_deferred {
                    return Ok(Properties::deferred());
                }
                Properties {
                    category: Category::Owned,
                    transfer: all_transfer,
                    share: all_share,
                    capture: false,
                }
            }
            TypeForm::StructuralRecord { .. }
            | TypeForm::List { .. }
            | TypeForm::Map { .. }
            | TypeForm::Option { .. }
            | TypeForm::Result { .. } => Properties {
                category: if any_deferred {
                    Category::Deferred
                } else {
                    Category::Ordinary
                },
                transfer: all_transfer,
                share: all_share,
                capture: all_capture,
            },
            TypeForm::Named {
                declaration: reference,
            }
            | TypeForm::Applied {
                declaration: reference,
                ..
            } => self.nominal(
                context,
                *reference,
                match form {
                    TypeForm::Applied { arguments, .. } => arguments,
                    _ => &[],
                },
                nominal_path,
                // Entering the declaration helper is not a representation edge. Its fields
                // and actual type arguments each charge their real child edge below.
                depth,
            )?,
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                self.scope(context, read_scope, depth)?;
                self.view_application(context, *declaration, arguments, nominal_path, depth + 1)?;
                Properties::new(Category::ReadView)
            }
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => {
                self.associated(context, form, depth + 1)?
            }
            TypeForm::Secret
            | TypeForm::CapabilityResource { .. }
            | TypeForm::Stream { .. }
            | TypeForm::Function { .. }
            | TypeForm::TaskFunction { .. } => Properties::new(Category::Other),
        };
        Ok(properties)
    }

    fn nominal(
        &mut self,
        context: &Context,
        reference: DeclarationReference,
        arguments: &[TypeObjectDigest],
        path: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<Properties, Diagnostic> {
        self.step(depth)?;
        let Some(owner) = self.owner(reference)? else {
            return Ok(Properties::deferred());
        };
        let record = declaration(owner)?;
        let (parameters, owned) = match &record.payload {
            PackageInterfaceDeclarationPayload::Record {
                type_parameters, ..
            }
            | PackageInterfaceDeclarationPayload::Variant {
                type_parameters, ..
            } => (type_parameters, false),
            PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters, ..
            } => (type_parameters, true),
            _ => {
                return Err(reject(
                    "nominal type reference does not name an ordinary or Owned nominal layout",
                ));
            }
        };
        if reference.package != self.package {
            if parameters.len() != arguments.len() {
                return Err(reject(
                    "foreign nominal application has the wrong complete arity",
                ));
            }
            return Ok(Properties::deferred());
        }
        self.arguments(context, reference, parameters, arguments, depth)?;
        if !path.insert(reference) {
            if context.finite_layout || owned {
                return Err(reject(
                    "read-view nominal field dependency closure is recursive",
                ));
            }
            return Ok(Properties::ordinary());
        }
        self.storage.reserve::<DeclarationReference>(1)?;
        let roots = self.nominal_fields(reference)?;
        let defining = self.defining_context(context, reference, parameters, arguments, None)?;
        let mut transfer = true;
        let mut share = true;
        let mut capture = !owned;
        let mut deferred = false;
        for ty in roots {
            let properties = self.occurrence(&defining, ty, false, false, path, depth + 1)?;
            if context.finite_layout
                && owned
                && !(matches!(properties.category, Category::Owned | Category::Deferred)
                    || properties.category == Category::Ordinary && properties.transfer)
            {
                return Err(reject(
                    "Owned read-reference nominal layout contains a non-Owned/non-Transferable field",
                ));
            }
            deferred |= properties.category == Category::Deferred;
            transfer &= properties.transfer;
            share &= properties.share;
            capture &= properties.capture;
        }
        path.remove(&reference);
        if deferred {
            return Ok(Properties::deferred());
        }
        Ok(Properties {
            category: if owned {
                Category::Owned
            } else {
                Category::Ordinary
            },
            transfer,
            share,
            capture,
        })
    }

    fn nominal_fields(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        let record = declaration(
            self.owners
                .get(&OwnerKey::Declaration(reference.declaration))
                .ok_or_else(|| reject("nominal field declaration is absent"))?,
        )?;
        let mut roots = Vec::new();
        match &record.payload {
            PackageInterfaceDeclarationPayload::Record { fields, .. }
            | PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } => {
                self.storage.reserve::<TypeObjectDigest>(fields.len())?;
                roots
                    .try_reserve_exact(fields.len())
                    .map_err(|_| capacity())?;
                for id in fields {
                    self.step(0)?;
                    let key = OwnerKey::Field(*id);
                    let owner = self
                        .owners
                        .get(&key)
                        .ok_or_else(|| reject("nominal field closure is incomplete"))?;
                    let PackageInterfaceRecord::Field(field) = &owner.record else {
                        return Err(reject("nominal field owner has a foreign kind"));
                    };
                    if owner.owner() != key || field.declaration != reference.declaration {
                        return Err(reject("nominal field belongs to another declaration"));
                    }
                    roots.push(field.ty);
                }
            }
            PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
                self.storage.reserve::<TypeObjectDigest>(cases.len())?;
                roots
                    .try_reserve_exact(cases.len())
                    .map_err(|_| capacity())?;
                for id in cases {
                    self.step(0)?;
                    let key = OwnerKey::Case(*id);
                    let owner = self
                        .owners
                        .get(&key)
                        .ok_or_else(|| reject("nominal case closure is incomplete"))?;
                    let PackageInterfaceRecord::Case(case) = &owner.record else {
                        return Err(reject("nominal case owner has a foreign kind"));
                    };
                    if owner.owner() != key || case.declaration != reference.declaration {
                        return Err(reject("nominal case belongs to another declaration"));
                    }
                    roots.extend(case.payload);
                }
            }
            _ => {
                return Err(reject(
                    "nominal field closure has a foreign declaration kind",
                ));
            }
        }
        Ok(roots)
    }

    fn view_application(
        &mut self,
        context: &Context,
        reference: DeclarationReference,
        arguments: &[TypeObjectDigest],
        path: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.step(depth)?;
        let Some(owner) = self.owner(reference)? else {
            return Ok(());
        };
        current(owner, OwnerKey::Declaration(reference.declaration))?;
        if matches!(
            &owner.record,
            PackageInterfaceRecord::RepresentationDeclaration(_)
        ) {
            let source = declaration(
                self.owners
                    .get(&OwnerKey::Declaration(context.reference.declaration))
                    .ok_or_else(|| reject("private read-view occurrence source is absent"))?,
            )?;
            if !matches!(
                &source.payload,
                PackageInterfaceDeclarationPayload::ReadView(_)
                    | PackageInterfaceDeclarationPayload::OwnedImplementation(_)
            ) {
                return Err(reject(
                    "a public named signature cannot directly name a private nominal read view",
                ));
            }
        }
        let record = declaration(owner)?;
        let PackageInterfaceDeclarationPayload::ReadView(view) = &record.payload else {
            return Err(reject(
                "read-view application does not name an exact nominal read-view declaration",
            ));
        };
        if record.header.kind != OwnerKind::ReadView
            || view.type_parameters.len() != arguments.len()
        {
            return Err(reject(
                "read-view application disagrees with its exact nominal kind or ordered arity",
            ));
        }
        if reference.package != self.package {
            // The declaration-only resolver cannot import its defining type-formal
            // owners or layout objects. The complete transport must discharge them.
            self.deferred = true;
            return Ok(());
        }
        self.arguments(context, reference, &view.type_parameters, arguments, depth)?;
        if !path.insert(reference) {
            return Err(reject(
                "nominal read-view dependency closure must be finite and acyclic",
            ));
        }
        self.storage.reserve::<DeclarationReference>(1)?;
        self.storage
            .reserve::<(TypeParameterId, Actual)>(context.actuals.len())?;
        let finite = Context {
            finite_layout: true,
            ..context.clone()
        };
        for argument in arguments {
            self.occurrence(&finite, *argument, false, false, path, depth + 1)?;
        }
        let defining = self.defining_context(
            &finite,
            reference,
            &view.type_parameters,
            arguments,
            Some(ReadScopeBinder::Declaration(reference)),
        )?;
        self.view_declaration(reference, view, Some(defining), path, depth + 1)?;
        path.remove(&reference);
        Ok(())
    }

    fn view_declaration(
        &mut self,
        reference: DeclarationReference,
        view: &ReadViewDeclaration,
        defining: Option<Context>,
        path: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.step(depth)?;
        let key = OwnerKey::Declaration(reference.declaration);
        current(
            self.owners
                .get(&key)
                .ok_or_else(|| reject("read-view declaration owner is absent"))?,
            key,
        )?;
        if view.fields.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
            || view.type_parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(capacity());
        }
        let binder = ReadScopeBinder::Declaration(reference);
        scope_inventory(self.package, &binder, self.owners)?;
        let context = match defining {
            Some(context) => Context {
                finite_layout: true,
                ..context
            },
            None => Context {
                source: key,
                reference,
                binder: Some(binder),
                finite_layout: true,
                actuals: BTreeMap::new(),
            },
        };
        let scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder,
            parameter: view.scope_parameter.id,
        });
        let mut names = BTreeSet::new();
        let mut ids = BTreeSet::new();
        self.storage
            .reserve::<TypeParameterId>(view.type_parameters.len())?;
        for parameter in &view.type_parameters {
            self.step(depth)?;
            if !ids.insert(*parameter) {
                return Err(reject("read-view type formal identity is duplicated"));
            }
            let constraints = self.formal(&context, *parameter, depth)?;
            if !(constraints.has_owned()
                || matches!(
                    constraints,
                    TypeParameterConstraints::Transferable
                        | TypeParameterConstraints::CaptureSafeTransferable
                ))
            {
                return Err(reject(
                    "read-view type formals require explicit Owned or ordinary Transferable bounds",
                ));
            }
            let owner = self
                .owners
                .get(&OwnerKey::TypeParameter(*parameter))
                .ok_or_else(|| reject("read-view type formal is absent"))?;
            current(owner, OwnerKey::TypeParameter(*parameter))?;
            let PackageInterfaceRecord::TypeParameter(parameter) = &owner.record else {
                return Err(reject("read-view type formal has a foreign kind"));
            };
            if !names.insert(parameter.name.as_str()) {
                return Err(reject("read-view type formal names must be distinct"));
            }
        }
        for (index, field) in view.fields.iter().enumerate() {
            self.step(depth)?;
            for previous in &view.fields[..index] {
                self.step(depth)?;
                if previous.id == field.id || previous.name == field.name {
                    return Err(reject(
                        "read-view field identities and names must be independently unique",
                    ));
                }
            }
            match &field.kind {
                ReadViewFieldKind::Ordinary => {
                    self.metadata(&context, field.ty, path, depth + 1)?
                }
                ReadViewFieldKind::ReadReference { read_scope } => {
                    self.scope(&context, read_scope, depth)?;
                    if read_scope != &scope {
                        return Err(reject(
                            "read-reference field scope differs from its exact declaration formal",
                        ));
                    }
                    let properties =
                        self.occurrence(&context, field.ty, true, false, path, depth + 1)?;
                    if properties.category == Category::ReadView {
                        let TypeForm::ReadViewApplied { read_scope, .. } =
                            &self.object(field.ty)?.form
                        else {
                            return Err(reject("a read-reference view referent must be nominal"));
                        };
                        if read_scope != &scope {
                            return Err(reject("read-reference view referent has another scope"));
                        }
                    } else if !matches!(properties.category, Category::Owned | Category::Deferred) {
                        return Err(reject(
                            "read-reference field requires independently admitted Owned or nominal ReadView referent",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn metadata(
        &mut self,
        context: &Context,
        ty: TypeObjectDigest,
        path: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.step(depth)?;
        let form = &self.object(ty)?.form;
        if let TypeForm::TypeParameter { parameter } = form
            && let Some(actual) = context.actuals.get(parameter)
        {
            return self.metadata(&actual.context, actual.ty, path, depth + 1);
        }
        self.storage
            .reserve::<TypeObjectDigest>(self.object(ty)?.child_type_count())?;
        let children = self.object(ty)?.child_types();
        for child in children {
            self.metadata(context, child, path, depth + 1)?;
        }
        match form {
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::StructuralRecord { .. }
            | TypeForm::List { .. }
            | TypeForm::Map { .. }
            | TypeForm::Option { .. }
            | TypeForm::Result { .. } => {}
            TypeForm::TypeParameter { parameter } => {
                let constraints = self.formal(context, *parameter, depth)?;
                if constraints.has_owned() || !constraints.requires_transfer() {
                    return Err(reject(
                        "ordinary view metadata formal lacks ordinary Transferable evidence",
                    ));
                }
            }
            TypeForm::Named {
                declaration: reference,
            }
            | TypeForm::Applied {
                declaration: reference,
                ..
            } => {
                let properties = self.nominal(
                    context,
                    *reference,
                    match form {
                        TypeForm::Applied { arguments, .. } => arguments,
                        _ => &[],
                    },
                    path,
                    depth,
                )?;
                if properties.category == Category::Deferred {
                    return Ok(());
                }
                if properties.category != Category::Ordinary || !properties.transfer {
                    return Err(reject(
                        "ordinary view metadata nominal closure is not ordinary Transferable data",
                    ));
                }
                let record = declaration(
                    self.owners
                        .get(&OwnerKey::Declaration(reference.declaration))
                        .ok_or_else(|| reject("metadata nominal declaration is absent"))?,
                )?;
                let parameters = match &record.payload {
                    PackageInterfaceDeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::Variant {
                        type_parameters, ..
                    } => type_parameters,
                    _ => return Err(reject("metadata nominal layout is not ordinary")),
                };
                let arguments = match form {
                    TypeForm::Applied { arguments, .. } => arguments.as_slice(),
                    _ => &[],
                };
                let defining =
                    self.defining_context(context, *reference, parameters, arguments, None)?;
                for field in self.nominal_fields(*reference)? {
                    self.metadata(&defining, field, path, depth + 1)?;
                }
            }
            _ => {
                return Err(reject(
                    "ordinary view metadata contains a view, owner, resource, secret, callable, or non-Transferable value",
                ));
            }
        }
        Ok(())
    }

    fn mapping_scopes(
        &mut self,
        contract_reference: DeclarationReference,
        method_id: crate::platform::semantic_id::MethodId,
        target: DeclarationReference,
        actuals: &[ReadScopeArgument],
    ) -> Result<(), Diagnostic> {
        self.charge(actuals.len(), 0)?;
        if actuals.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
            return Err(capacity());
        }
        let binder = ReadScopeBinder::ContractMethod {
            contract: contract_reference,
            method: method_id,
        };
        // Mapping actuals are universally interpreted in the exact implemented
        // contract method. They are not scopes imported into the implementation.
        for actual in actuals {
            let ReadScopeArgument::Parameter(reference) = actual else {
                return Err(reject(
                    "mapped method scopes cannot retain lexical activation loans",
                ));
            };
            if reference.binder != binder {
                return Err(reject(
                    "mapped method scope belongs to another exact contract method",
                ));
            }
        }
        if let Some(owner) = self.owner(contract_reference)? {
            let record = declaration(owner)?;
            let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &record.payload
            else {
                return Err(reject(
                    "mapped method scope authority is not an Owned contract",
                ));
            };
            let mut selected = contract
                .methods
                .iter()
                .filter(|method| method.id == method_id);
            let method = selected.next().ok_or_else(|| {
                reject("mapped method is absent from its complete contract inventory")
            })?;
            if selected.next().is_some() {
                return Err(reject("mapped contract method identity is duplicated"));
            }
            self.charge(method.read_scope_parameters.len(), 0)?;
            validate_scope_inventory(&method.read_scope_parameters)?;
            if !method.read_scope_parameters.is_empty() || !actuals.is_empty() {
                current(owner, OwnerKey::Declaration(contract_reference.declaration))?;
            }
            for actual in actuals {
                let ReadScopeArgument::Parameter(reference) = actual else {
                    unreachable!("lexical scopes rejected above");
                };
                if method
                    .read_scope_parameters
                    .iter()
                    .filter(|formal| formal.id == reference.parameter)
                    .count()
                    != 1
                {
                    return Err(reject(
                        "mapped method scope is absent from its exact universal formal inventory",
                    ));
                }
                self.charge(method.read_scope_parameters.len(), 0)?;
            }
        }
        if target.package == self.package
            && !self
                .owners
                .contains_key(&OwnerKey::Declaration(target.declaration))
        {
            // A private mapped function is absent from the public projection.
            // Complete canonical source admission must discharge its signature;
            // this descriptive interface grants no callable or naming authority.
            self.step(0)?;
            self.deferred = true;
            return Ok(());
        }
        if let Some(owner) = self.owner(target)? {
            let record = declaration(owner)?;
            let PackageInterfaceDeclarationPayload::Function(function) = &record.payload else {
                return Err(reject(
                    "mapped method target is not a named function signature",
                ));
            };
            self.charge(function.read_scope_parameters.len(), 0)?;
            validate_scope_inventory(&function.read_scope_parameters)?;
            if actuals.len() != function.read_scope_parameters.len() {
                return Err(reject(
                    "mapped method scope actuals differ from the target's complete ordered formal inventory",
                ));
            }
            if !actuals.is_empty() {
                current(owner, OwnerKey::Declaration(target.declaration))?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn signature<S: Copy + Eq>(
        &mut self,
        context: &Context,
        parameters: &[(S, TypeObjectDigest, ParameterUse)],
        result: &CallableResult<S>,
        effect: &FunctionEffect,
        formals: &[ReadScopeParameter],
        sources: &[ReadScopeSource<S>],
    ) -> Result<(), Diagnostic> {
        validate_scope_inventory(formals)?;
        if parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
            || sources.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(capacity());
        }
        if !formals.is_empty()
            || !sources.is_empty()
            || matches!(result.relation, ResultRelation::ViewFrom { .. })
        {
            current(
                self.owners
                    .get(&context.source)
                    .ok_or_else(|| reject("scoped signature owner is absent"))?,
                context.source,
            )?;
        }
        for (index, (id, ty, use_mode)) in parameters.iter().enumerate() {
            self.step(0)?;
            self.charge(index, 0)?;
            if parameters[..index]
                .iter()
                .any(|(previous, _, _)| previous == id)
            {
                return Err(reject("scoped signature parameter identity is duplicated"));
            }
            let view = matches!(
                self.object(*ty)?.form,
                TypeForm::ReadViewApplied { .. }
                    | TypeForm::AssociatedReadViewMember { .. }
                    | TypeForm::AssociatedReadViewProjection { .. }
            );
            self.occurrence(context, *ty, view, false, &mut BTreeSet::new(), 0)?;
            if view && *use_mode != ParameterUse::Borrow {
                return Err(reject("read-view parameters permit borrow use only"));
            }
        }
        for (index, source) in sources.iter().enumerate() {
            self.step(0)?;
            self.charge(index, 0)?;
            if sources[..index]
                .iter()
                .any(|previous| previous.source == source.source && previous.scope == source.scope)
            {
                return Err(reject(
                    "source relationship inventory repeats an exact parameter and scope pair",
                ));
            }
            self.scope(context, &source.scope, 0)?;
            let (_, ty, use_mode) = parameters
                .iter()
                .find(|(id, _, _)| *id == source.source)
                .ok_or_else(|| {
                    reject("source relationship is outside its exact complete parameter inventory")
                })?;
            if *use_mode != ParameterUse::Borrow {
                return Err(reject("source relationships require a borrowed parameter"));
            }
            let properties = self.occurrence(context, *ty, true, false, &mut BTreeSet::new(), 0)?;
            if !matches!(
                properties.category,
                Category::Owned | Category::ReadView | Category::Deferred
            ) {
                return Err(reject(
                    "source scope relationship requires an admitted Owned or ReadView parameter",
                ));
            }
            if properties.category == Category::ReadView
                && view_scope(&self.object(*ty)?.form) != Some(&source.scope)
            {
                return Err(reject(
                    "view parameter source relationship differs from its certified type scope",
                ));
            }
        }
        let view_result = matches!(result.relation, ResultRelation::ViewFrom { .. });
        let result_properties = self.occurrence(
            context,
            result.ty,
            view_result,
            false,
            &mut BTreeSet::new(),
            0,
        )?;
        let source = match &result.relation {
            ResultRelation::Value => return Ok(()),
            ResultRelation::ReadFrom { source } | ResultRelation::ViewFrom { source, .. } => {
                *source
            }
        };
        if !matches!(effect, FunctionEffect::Pure) {
            return Err(reject(
                "source-tied read and view results require a pure named signature",
            ));
        }
        let (_, source_ty, use_mode) = parameters
            .iter()
            .find(|(id, _, _)| *id == source)
            .ok_or_else(|| {
                reject("result source is outside its exact ordered parameter inventory")
            })?;
        if *use_mode != ParameterUse::Borrow {
            return Err(reject(
                "source-tied result requires a borrowed source parameter",
            ));
        }
        let source_properties =
            self.occurrence(context, *source_ty, true, false, &mut BTreeSet::new(), 0)?;
        if !matches!(
            source_properties.category,
            Category::Owned | Category::ReadView | Category::Deferred
        ) {
            return Err(reject(
                "source-tied result has no admitted Owned or ReadView source",
            ));
        }
        match &result.relation {
            ResultRelation::ReadFrom { .. } => {
                if !matches!(
                    result_properties.category,
                    Category::Owned | Category::Deferred
                ) {
                    return Err(reject(
                        "ReadFrom result must be an independently admitted Owned read",
                    ));
                }
            }
            ResultRelation::ViewFrom { scope, .. } => {
                self.scope(context, scope, 0)?;
                if result_properties.category != Category::ReadView
                    || view_scope(&self.object(result.ty)?.form) != Some(scope)
                {
                    return Err(reject(
                        "ViewFrom relation requires a ReadView type at its exact declared result scope",
                    ));
                }
                let certified = if source_properties.category == Category::ReadView {
                    view_scope(&self.object(*source_ty)?.form) == Some(scope)
                } else {
                    sources.iter().any(|relationship| {
                        relationship.source == source && &relationship.scope == scope
                    })
                };
                if !certified {
                    return Err(reject("view result lacks exact source forwarding evidence"));
                }
            }
            ResultRelation::Value => unreachable!("value relation returned above"),
        }
        Ok(())
    }
}

fn view_scope(form: &TypeForm) -> Option<&ReadScopeArgument> {
    match form {
        TypeForm::ReadViewApplied { read_scope, .. }
        | TypeForm::AssociatedReadViewMember { read_scope, .. }
        | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Some(read_scope),
        _ => None,
    }
}

pub(crate) fn validate_declaration_metered(
    package: PackageId,
    source: OwnerKey,
    payload: &PackageInterfaceDeclarationPayload,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<(), Diagnostic> {
    validate_declaration_resolved_metered(
        package,
        source,
        payload,
        owners,
        types,
        &mut |_| Ok(None),
        visit,
    )
    .map(|_| ())
}

/// Returns whether foreign formal/layout or local private mapped-function obligations
/// still need the selected dependency closure and complete canonical source admission.
/// Deferred evidence grants no readiness or access permission.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_declaration_resolved_metered<'a>(
    package: PackageId,
    source: OwnerKey,
    payload: &PackageInterfaceDeclarationPayload,
    owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    resolve: &mut impl FnMut(
        DeclarationReference,
    ) -> Result<Option<&'a PackageInterfaceOwner>, Diagnostic>,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<bool, Diagnostic> {
    let mut admission = Admission {
        package,
        owners,
        types,
        resolve,
        work: 0,
        storage: RepresentationBudget::default(),
        deferred: false,
        visit,
    };
    let OwnerKey::Declaration(_) = source else {
        return Err(reject(
            "declaration admission requires its exact declaration owner",
        ));
    };
    let actual = declaration(
        owners
            .get(&source)
            .ok_or_else(|| reject("declaration interface owner is absent"))?,
    )?;
    if actual.header.owner != source || &actual.payload != payload {
        return Err(reject(
            "declaration admission payload differs from its exact interface owner",
        ));
    }
    let reference = source_declaration(package, source, owners)?;
    let context = admission.context(source, None)?;
    match payload {
        PackageInterfaceDeclarationPayload::ReadView(view) => {
            let mut path = BTreeSet::new();
            path.insert(reference);
            admission.storage.reserve::<DeclarationReference>(1)?;
            admission.view_declaration(reference, view, None, &mut path, 0)?;
        }
        PackageInterfaceDeclarationPayload::Function(signature) => {
            let mut parameters = Vec::new();
            admission.storage.reserve::<(
                crate::platform::semantic_id::ParameterId,
                TypeObjectDigest,
                ParameterUse,
            )>(signature.parameters.len())?;
            parameters
                .try_reserve_exact(signature.parameters.len())
                .map_err(|_| capacity())?;
            for id in &signature.parameters {
                admission.step(0)?;
                let owner = owners
                    .get(&OwnerKey::Parameter(*id))
                    .ok_or_else(|| reject("function parameter closure is incomplete"))?;
                let PackageInterfaceRecord::Parameter(parameter) = &owner.record else {
                    return Err(reject("function parameter has a foreign record kind"));
                };
                if owner.owner() != OwnerKey::Parameter(*id)
                    || parameter.parent != ParameterParent::Function(reference.declaration)
                {
                    return Err(reject(
                        "function parameter belongs to another exact declaration",
                    ));
                }
                parameters.push((*id, parameter.ty, parameter.use_mode));
            }
            let context = Context {
                binder: Some(ReadScopeBinder::Declaration(reference)),
                ..context
            };
            admission.signature(
                &context,
                &parameters,
                &signature.result,
                &signature.effect,
                &signature.read_scope_parameters,
                &signature.read_scope_sources,
            )?;
            for witness in &signature.implementation_parameters {
                admission.occurrence(
                    &context,
                    witness.self_type,
                    false,
                    false,
                    &mut BTreeSet::new(),
                    0,
                )?;
                for ty in &witness.type_arguments {
                    admission.occurrence(&context, *ty, false, false, &mut BTreeSet::new(), 0)?;
                }
            }
        }
        PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
            for (index, member) in contract.associated_types.iter().enumerate() {
                admission.step(0)?;
                admission.charge(index, 0)?;
                if contract.associated_types[..index]
                    .iter()
                    .any(|previous| previous.id == member.id || previous.name == member.name)
                {
                    return Err(reject(
                        "associated member identities and names must be independently unique",
                    ));
                }
                if let AssociatedTypeBound::ReadViewFamily { scope_parameter } = &member.bound {
                    current(
                        owners
                            .get(&source)
                            .ok_or_else(|| reject("family contract owner is absent"))?,
                        source,
                    )?;
                    validate_scope_inventory(std::slice::from_ref(scope_parameter))?;
                    scope_inventory(
                        package,
                        &ReadScopeBinder::AssociatedMember {
                            contract: reference,
                            member: member.id,
                        },
                        owners,
                    )?;
                }
            }
            for method in &contract.methods {
                admission
                    .storage
                    .reserve::<(u32, TypeObjectDigest, ParameterUse)>(method.parameters.len())?;
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
                let context = Context {
                    binder: Some(ReadScopeBinder::ContractMethod {
                        contract: reference,
                        method: method.id,
                    }),
                    ..context.clone()
                };
                admission.signature(
                    &context,
                    &parameters,
                    &method.result,
                    &method.effect,
                    &method.read_scope_parameters,
                    &method.read_scope_sources,
                )?;
            }
        }
        PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
            if implementation.associated_types.len()
                > crate::platform::kernel::contract::MAXIMUM_CHILDREN
            {
                return Err(capacity());
            }
            for pair in implementation.associated_types.windows(2) {
                admission.step(0)?;
                if pair[0].member >= pair[1].member {
                    return Err(reject(
                        "associated binding identities require a complete strictly ordered inventory",
                    ));
                }
            }
            admission.occurrence(
                &context,
                implementation.self_type,
                false,
                false,
                &mut BTreeSet::new(),
                0,
            )?;
            for ty in &implementation.type_arguments {
                admission.occurrence(&context, *ty, false, false, &mut BTreeSet::new(), 0)?;
            }
            for binding in &implementation.associated_types {
                match &binding.value {
                    AssociatedTypeBindingValue::Owned { ty } => {
                        let property = admission.occurrence(
                            &context,
                            *ty,
                            false,
                            true,
                            &mut BTreeSet::new(),
                            0,
                        )?;
                        if !matches!(property.category, Category::Owned | Category::Deferred) {
                            return Err(reject(
                                "Owned binding RHS is not an independently admitted Owned type",
                            ));
                        }
                    }
                    AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter,
                        ty,
                    } => {
                        let binder = ReadScopeBinder::AssociatedBinding {
                            implementation: reference,
                            member: binding.member,
                        };
                        let context = Context {
                            binder: Some(binder),
                            ..context.clone()
                        };
                        scope_inventory(package, &binder, owners)?;
                        let declared = ReadScopeArgument::Parameter(ReadScopeReference {
                            binder,
                            parameter: scope_parameter.id,
                        });
                        let TypeForm::ReadViewApplied { read_scope, .. } =
                            &admission.object(*ty)?.form
                        else {
                            return Err(reject(
                                "family binding RHS must be a nominal read-view application",
                            ));
                        };
                        if read_scope != &declared {
                            return Err(reject(
                                "family binding RHS does not apply its exact scope formal",
                            ));
                        }
                        admission.occurrence(&context, *ty, true, true, &mut BTreeSet::new(), 0)?;
                    }
                }
            }
            for witness in &implementation.implementation_parameters {
                admission.occurrence(
                    &context,
                    witness.self_type,
                    false,
                    false,
                    &mut BTreeSet::new(),
                    0,
                )?;
                for ty in &witness.type_arguments {
                    admission.occurrence(&context, *ty, false, false, &mut BTreeSet::new(), 0)?;
                }
            }
            for method in &implementation.methods {
                for ty in &method.type_arguments {
                    admission.occurrence(&context, *ty, false, false, &mut BTreeSet::new(), 0)?;
                }
                admission.mapping_scopes(
                    implementation.contract,
                    method.method,
                    method.function,
                    &method.read_scope_arguments,
                )?;
                for operand in &method.implementations {
                    for operand in operand.walk() {
                        for ty in operand.type_arguments() {
                            admission.occurrence(
                                &context,
                                *ty,
                                false,
                                false,
                                &mut BTreeSet::new(),
                                0,
                            )?;
                        }
                    }
                }
            }
        }
        PackageInterfaceDeclarationPayload::Record { .. }
        | PackageInterfaceDeclarationPayload::OwnedRecord { .. }
        | PackageInterfaceDeclarationPayload::Variant { .. } => {
            for ty in admission.nominal_fields(reference)? {
                admission.occurrence(&context, ty, false, false, &mut BTreeSet::new(), 0)?;
            }
        }
        PackageInterfaceDeclarationPayload::External(signature) => {
            admission.occurrence(
                &context,
                signature.result,
                false,
                false,
                &mut BTreeSet::new(),
                0,
            )?;
            for id in &signature.parameters {
                admission.step(0)?;
                let owner = owners
                    .get(&OwnerKey::Parameter(*id))
                    .ok_or_else(|| reject("external parameter closure is incomplete"))?;
                let PackageInterfaceRecord::Parameter(parameter) = &owner.record else {
                    return Err(reject("external parameter owner has a foreign kind"));
                };
                if parameter.parent != ParameterParent::Function(reference.declaration) {
                    return Err(reject(
                        "external parameter belongs to another exact declaration",
                    ));
                }
                admission.occurrence(
                    &context,
                    parameter.ty,
                    false,
                    false,
                    &mut BTreeSet::new(),
                    0,
                )?;
            }
        }
        PackageInterfaceDeclarationPayload::Interface { operations } => {
            for id in operations {
                admission.step(0)?;
                let owner = owners
                    .get(&OwnerKey::Operation(*id))
                    .ok_or_else(|| reject("interface operation closure is incomplete"))?;
                let PackageInterfaceRecord::Operation(operation) = &owner.record else {
                    return Err(reject("interface operation owner has a foreign kind"));
                };
                if operation.declaration != reference.declaration {
                    return Err(reject(
                        "interface operation belongs to another exact declaration",
                    ));
                }
                admission.occurrence(
                    &context,
                    operation.result,
                    false,
                    false,
                    &mut BTreeSet::new(),
                    0,
                )?;
                for parameter_id in &operation.parameters {
                    admission.step(0)?;
                    let owner = owners
                        .get(&OwnerKey::Parameter(*parameter_id))
                        .ok_or_else(|| reject("operation parameter closure is incomplete"))?;
                    let PackageInterfaceRecord::Parameter(parameter) = &owner.record else {
                        return Err(reject("operation parameter owner has a foreign kind"));
                    };
                    if parameter.parent != ParameterParent::Operation(*id) {
                        return Err(reject(
                            "operation parameter belongs to another exact operation",
                        ));
                    }
                    admission.occurrence(
                        &context,
                        parameter.ty,
                        false,
                        false,
                        &mut BTreeSet::new(),
                        0,
                    )?;
                }
            }
        }
        PackageInterfaceDeclarationPayload::Component { ports, .. } => {
            for id in ports {
                admission.step(0)?;
                let owner = owners
                    .get(&OwnerKey::Port(*id))
                    .ok_or_else(|| reject("component port closure is incomplete"))?;
                let PackageInterfaceRecord::Port(port) = &owner.record else {
                    return Err(reject("component port owner has a foreign kind"));
                };
                if port.declaration != reference.declaration {
                    return Err(reject(
                        "component port belongs to another exact declaration",
                    ));
                }
                admission.occurrence(
                    &context,
                    port.function_type,
                    false,
                    false,
                    &mut BTreeSet::new(),
                    0,
                )?;
            }
        }
        PackageInterfaceDeclarationPayload::Constant { ty } => {
            admission.occurrence(&context, *ty, false, false, &mut BTreeSet::new(), 0)?;
        }
    }
    Ok(admission.deferred)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_view_application_resolved_metered<'a>(
    package: PackageId,
    source: OwnerKey,
    reference: DeclarationReference,
    arguments: &[TypeObjectDigest],
    read_scope: &ReadScopeArgument,
    owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    resolve: &mut impl FnMut(
        DeclarationReference,
    ) -> Result<Option<&'a PackageInterfaceOwner>, Diagnostic>,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<bool, Diagnostic> {
    let mut admission = Admission {
        package,
        owners,
        types,
        resolve,
        work: 0,
        storage: RepresentationBudget::default(),
        deferred: false,
        visit,
    };
    let context = admission.context(source, None)?;
    admission.scope(&context, read_scope, 0)?;
    // Unlike visible fields, type actuals never inherit direct view permission.
    for ty in arguments {
        admission.occurrence(&context, *ty, false, false, &mut BTreeSet::new(), 0)?;
    }
    admission.view_application(&context, reference, arguments, &mut BTreeSet::new(), 0)?;
    Ok(admission.deferred)
}
