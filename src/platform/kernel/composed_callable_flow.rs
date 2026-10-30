//! Complete source-closure flow, including statically selected cross-package method callbacks.
//! A finite conservative call graph first excludes unsupported recursive witness construction.
//! Exact prerequisite selections then retain independent type-growth slots/SCCs.

#[path = "associated_callable_flow.rs"]
mod associated;
#[path = "prerequisite_callable_flow.rs"]
mod prerequisites;
#[path = "recursive_callable_flow.rs"]
mod recursive;

#[cfg(test)]
fn test_validate_callable_closure_with_scope<R: CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut usize,
    maximum_work: Option<usize>,
    scope: recursive::ProofScope,
) -> Result<CallableFlowWork, Diagnostic> {
    crate::platform::kernel::test_work::observe(work, |work| {
        validate_callable_closure_with_scope(read, work, maximum_work, scope)
    })
}

#[cfg(test)]
use super::MAXIMUM_ANALYSIS_BYTES;
use super::{CallableFlowWork, MAXIMUM_DIAGNOSTIC_PATH, semantic};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::work::ValidationWork;
use crate::platform::kernel::{
    AssociatedTypeBindingValue, AssociatedTypeBound, BindingKind, DeclarationPayload,
    DeclarationRecord, DeclarationReference, ExpressionOperation, ImplementationOperand,
    KernelSnapshot, OwnedImplementation, OwnedMethodImplementation, OwnerKey, OwnerRecord,
    PackageId, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference,
    ReadScopeSource, ResultRelation, TypeForm, TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, ExpressionId, ImplementationParameterId, MethodId,
    TypeParameterId,
};
use std::collections::{BTreeMap, BTreeSet};

/// Analyzer-only exact source access. Every declaration (including private bodies and tests) is
/// enumerated. Interfaces alone cannot establish the composed callback relation.
pub(crate) trait CallableClosureRead {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic>;
    fn owner(&self, package: PackageId, owner: OwnerKey)
    -> Result<Option<OwnerRecord>, Diagnostic>;
    fn type_object(
        &self,
        package: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic>;
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        Ok(())
    }
}

struct Snapshots<'a> {
    snapshots: &'a [&'a KernelSnapshot],
    checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
}
impl CallableClosureRead for Snapshots<'_> {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for (index, snapshot) in self.snapshots.iter().enumerate() {
            self.validation_checkpoint()?;
            if self.snapshots[..index]
                .iter()
                .any(|s| s.root.package_id == snapshot.root.package_id)
            {
                return Err(semantic(
                    "kernel_callable_closure_binding",
                    "source closure has more than one revision for a package",
                ));
            }
            for owner in snapshot.owners.keys() {
                self.validation_checkpoint()?;
                if let OwnerKey::Declaration(declaration) = owner {
                    visitor(DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *declaration,
                    })?;
                }
            }
        }
        Ok(())
    }
    fn owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self
            .snapshots
            .iter()
            .find(|s| s.root.package_id == package)
            .and_then(|s| s.owners.get(&owner))
            .cloned())
    }
    fn type_object(
        &self,
        package: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self
            .snapshots
            .iter()
            .find(|s| s.root.package_id == package)
            .and_then(|s| s.types.get(&ty).or_else(|| s.dependency_types.get(&ty)))
            .cloned())
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        (self.checkpoint)()
    }
}

/// The caller binds every supplied snapshot to an already admitted exact source revision. This
/// analysis writes no summary or canonical meaning and does not acquire repository locks.
/// `None` leaves work observational; `Some` enforces the complete carried-work quota.
/// Metadata, depth, cancellation and finite specialization checks apply in either mode.
pub(crate) fn validate_snapshot_callable_closure(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut ValidationWork,
    maximum_work: Option<usize>,
) -> Result<CallableFlowWork, Diagnostic> {
    validate_callable_closure(
        &Snapshots {
            snapshots,
            checkpoint,
        },
        work,
        maximum_work,
    )
}

#[cfg(test)]
pub(crate) fn validate_snapshot_callable_closure_eager(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut ValidationWork,
    maximum_work: Option<usize>,
) -> Result<CallableFlowWork, Diagnostic> {
    validate_callable_closure_with_scope(
        &Snapshots {
            snapshots,
            checkpoint,
        },
        work,
        maximum_work,
        recursive::ProofScope::All,
    )
}

#[cfg(test)]
pub(crate) fn validate_snapshot_callable_closure_with_limits(
    snapshots: &[&KernelSnapshot],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut ValidationWork,
    maximum_work: Option<usize>,
    maximum_metadata_bytes: Option<usize>,
    eager: bool,
) -> Result<CallableFlowWork, Diagnostic> {
    validate_callable_closure_with_limits(
        &Snapshots {
            snapshots,
            checkpoint,
        },
        work,
        maximum_work,
        maximum_metadata_bytes,
        if eager {
            recursive::ProofScope::All
        } else {
            recursive::ProofScope::Recursive
        },
    )
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Selection {
    Unknown {
        contract: DeclarationReference,
    },
    Scheme {
        implementation: DeclarationReference,
        prerequisites: Vec<SelectionId>,
    },
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SelectionId(usize);
struct SelectionInfo {
    shape: Selection,
    parameters: Vec<TypeParameterId>,
    depth: usize,
    has_parameters: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Context {
    owner: DeclarationReference,
    // Implementation methods have separate contexts: choosing one method cannot invent calls
    // to other methods. All mappings are nevertheless seeded for independent admission.
    method: Option<MethodId>,
    implementations: Vec<SelectionId>,
}
#[cfg(test)]
#[derive(Clone)]
struct WitnessLayout {
    path: Vec<usize>,
    parameters: Vec<TypeParameterId>,
    start: usize,
}
#[derive(Clone)]
struct ContextInfo {
    key: Context,
    parameters: Vec<TypeParameterId>,
    #[cfg(test)]
    witnesses: Vec<WitnessLayout>,
    slots: Vec<usize>,
}
#[cfg(test)]
impl ContextInfo {
    fn witness_slots(&self, path: &[usize]) -> &[usize] {
        let Ok(index) = self
            .witnesses
            .binary_search_by(|w| w.path.as_slice().cmp(path))
        else {
            return &[];
        };
        let witness = &self.witnesses[index];
        &self.slots[witness.start..witness.start + witness.parameters.len()]
    }
}
struct Edge {
    from: usize,
    to: usize,
    origin: DeclarationReference,
    expression: Option<ExpressionId>,
    argument: usize,
    path: Vec<usize>,
    expanding: bool,
}
// Only membership/insertion is consumed. These disjoint key shapes preserve the
// full occurrence identity within one scan, without comparing known empty metadata.
type ScopedSyntaxOccurrence = (ExpressionId, BTreeSet<BindingId>, Option<BindingId>);

#[derive(Default)]
struct VisitedSyntax {
    plain: BTreeMap<ExpressionId, ()>,
    scoped: BTreeMap<ScopedSyntaxOccurrence, ()>,
}

impl VisitedSyntax {
    fn plain_key(loans: &BTreeSet<BindingId>, establishing: Option<BindingId>) -> bool {
        loans.is_empty() && establishing.is_none()
    }

    fn comparison_shape(
        &self,
        loans: &BTreeSet<BindingId>,
        establishing: Option<BindingId>,
    ) -> (usize, usize) {
        if Self::plain_key(loans, establishing) {
            (0, self.plain.len())
        } else {
            (loans.len() + 2, self.scoped.len())
        }
    }

    // One tree descent supplies both membership and insertion. Preparation must
    // reserve the new occurrence and copy its loans before membership is exposed.
    fn insert_with(
        &mut self,
        occurrence: ScopedSyntaxOccurrence,
        prepare: impl FnOnce(&ScopedSyntaxOccurrence) -> Result<BTreeSet<BindingId>, Diagnostic>,
    ) -> Result<Option<BTreeSet<BindingId>>, Diagnostic> {
        use std::collections::btree_map::Entry;

        if Self::plain_key(&occurrence.1, occurrence.2) {
            match self.plain.entry(occurrence.0) {
                Entry::Occupied(_) => Ok(None),
                Entry::Vacant(entry) => {
                    let loans = prepare(&occurrence)?;
                    entry.insert(());
                    Ok(Some(loans))
                }
            }
        } else {
            match self.scoped.entry(occurrence) {
                Entry::Occupied(_) => Ok(None),
                Entry::Vacant(entry) => {
                    let loans = prepare(entry.key())?;
                    entry.insert(());
                    Ok(Some(loans))
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct WorkContext {
    phase: &'static str,
    owner: Option<DeclarationReference>,
    context: Option<usize>,
    expression: Option<ExpressionId>,
    query: Option<(PackageId, TypeObjectDigest)>,
}

struct Analysis<'a, R: ?Sized> {
    read: &'a R,
    work: &'a mut ValidationWork,
    maximum_work: Option<usize>,
    maximum_metadata_bytes: Option<usize>,
    observation: CallableFlowWork,
    contexts: Vec<ContextInfo>,
    indexes: BTreeMap<Context, usize>,
    selections: Vec<SelectionInfo>,
    selection_indexes: BTreeMap<Selection, SelectionId>,
    pending: Vec<usize>,
    calls: Vec<recursive::CallSite>,
    slot_owners: Vec<(usize, usize)>,
    edges: Vec<Edge>,
    associated: associated::State,
    completed_view_queries: BTreeMap<(PackageId, TypeObjectDigest), bool>,
    diagnostic_context: WorkContext,
}
fn implementation_parameters(
    payload: &DeclarationPayload,
) -> &[super::super::ImplementationParameter] {
    match payload {
        DeclarationPayload::Function(function) => &function.implementation_parameters,
        DeclarationPayload::OwnedImplementation(implementation) => {
            &implementation.implementation_parameters
        }
        _ => &[],
    }
}
fn reserve_metadata<T>(
    observation: &mut CallableFlowWork,
    maximum: Option<usize>,
    count: usize,
) -> Result<(), Diagnostic> {
    reserve_metadata_product::<T>(observation, maximum, count, 1)
}

fn reserve_metadata_product<T>(
    observation: &mut CallableFlowWork,
    maximum: Option<usize>,
    count: usize,
    multiple: usize,
) -> Result<(), Diagnostic> {
    let mut observed = ValidationWork {
        consumed: observation.metadata_bytes,
        saturated: observation.metadata_saturated,
    };
    if !observed.charge_factors(
        &[
            count,
            std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>(),
            multiple,
        ],
        maximum,
    ) {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "kernel_callable_flow_storage",
            "composed callable flow exhausted its explicit metadata admission",
        ));
    }
    observation.metadata_bytes = observed.consumed;
    observation.metadata_saturated = observed.saturated;
    Ok(())
}

impl<R: CallableClosureRead + ?Sized> Analysis<'_, R> {
    fn scope_parameters(
        &mut self,
        binder: ReadScopeBinder,
    ) -> Result<Vec<ReadScopeParameter>, Diagnostic> {
        let declaration = self.declaration(binder.declaration())?;
        let generation = declaration.header.contract_version;
        let kind_matches = match &declaration.payload {
            DeclarationPayload::Function(_) => matches!(
                declaration.header.kind,
                crate::platform::kernel::OwnerKind::PureFunction
                    | crate::platform::kernel::OwnerKind::TaskFunction
            ),
            DeclarationPayload::External(_) => {
                declaration.header.kind == crate::platform::kernel::OwnerKind::External
            }
            DeclarationPayload::ReadView(_) => {
                declaration.header.kind == crate::platform::kernel::OwnerKind::ReadView
            }
            DeclarationPayload::OwnedContract(_) => {
                declaration.header.kind == crate::platform::kernel::OwnerKind::OwnedContract
            }
            DeclarationPayload::OwnedImplementation(_) => {
                declaration.header.kind == crate::platform::kernel::OwnerKind::OwnedImplementation
            }
            _ => false,
        };
        if !kind_matches {
            return Err(associated::invalid());
        }
        let parameters = match (binder, declaration.payload) {
            (ReadScopeBinder::Declaration(_), DeclarationPayload::Function(function)) => {
                function.read_scope_parameters
            }
            (ReadScopeBinder::Declaration(_), DeclarationPayload::External(_)) => Vec::new(),
            (ReadScopeBinder::Declaration(_), DeclarationPayload::ReadView(view)) => {
                self.reserve::<ReadScopeParameter>(1)?;
                vec![view.scope_parameter]
            }
            (
                ReadScopeBinder::ContractMethod { method, .. },
                DeclarationPayload::OwnedContract(contract),
            ) => {
                let mut found = None;
                for entry in contract.methods {
                    self.tick()?;
                    if entry.id == method && found.replace(entry.read_scope_parameters).is_some() {
                        return Err(associated::invalid());
                    }
                }
                found.ok_or_else(associated::invalid)?
            }
            (
                ReadScopeBinder::AssociatedMember { member, .. },
                DeclarationPayload::OwnedContract(contract),
            ) => {
                let mut found = None;
                for entry in contract.associated_types {
                    self.tick()?;
                    if entry.id == member {
                        let AssociatedTypeBound::ReadViewFamily { scope_parameter } = entry.bound
                        else {
                            return Err(associated::invalid());
                        };
                        if found.replace(scope_parameter).is_some() {
                            return Err(associated::invalid());
                        }
                    }
                }
                self.reserve::<ReadScopeParameter>(1)?;
                vec![found.ok_or_else(associated::invalid)?]
            }
            (
                ReadScopeBinder::AssociatedBinding { member, .. },
                DeclarationPayload::OwnedImplementation(implementation),
            ) => {
                let mut found = None;
                for entry in implementation.associated_types {
                    self.tick()?;
                    if entry.member == member {
                        let AssociatedTypeBindingValue::ReadViewFamily {
                            scope_parameter, ..
                        } = entry.value
                        else {
                            return Err(associated::invalid());
                        };
                        if found.replace(scope_parameter).is_some() {
                            return Err(associated::invalid());
                        }
                    }
                }
                self.reserve::<ReadScopeParameter>(1)?;
                vec![found.ok_or_else(associated::invalid)?]
            }
            _ => return Err(associated::invalid()),
        };
        if parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
            return Err(associated::invalid());
        }
        if generation != crate::platform::canonical_schema::SCHEMA_EPOCH && !parameters.is_empty() {
            return Err(associated::invalid());
        }
        self.reserve::<ReadScopeParameter>(parameters.len())?;
        self.reserve::<crate::platform::semantic_id::ReadScopeParameterId>(parameters.len())?;
        self.reserve::<crate::platform::kernel::Name>(parameters.len())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for parameter in &parameters {
            self.tick()?;
            self.reserve::<u8>(parameter.name.as_str().len())?;
            if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
                return Err(associated::invalid());
            }
        }
        Ok(parameters)
    }

    fn context_binder(&mut self, context: usize) -> Result<Option<ReadScopeBinder>, Diagnostic> {
        let key = &self.contexts[context].key;
        let owner = key.owner;
        let method = key.method;
        let declaration = self.declaration(owner)?;
        Ok(match declaration.payload {
            DeclarationPayload::Function(_) => Some(ReadScopeBinder::Declaration(owner)),
            DeclarationPayload::OwnedImplementation(implementation) => {
                Some(ReadScopeBinder::ContractMethod {
                    contract: implementation.contract,
                    method: method.ok_or_else(associated::invalid)?,
                })
            }
            _ => None,
        })
    }

    fn validate_scope(
        &mut self,
        scope: ReadScopeArgument,
        binder: Option<ReadScopeBinder>,
        loans: &BTreeSet<BindingId>,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.associated.enabled = true;
        match scope {
            ReadScopeArgument::Parameter(reference) => {
                if Some(reference.binder) != binder {
                    return Err(associated::invalid());
                }
                let parameters = self.scope_parameters(reference.binder)?;
                let mut found = false;
                for parameter in parameters {
                    self.tick()?;
                    found |= parameter.id == reference.parameter;
                }
                if !found {
                    return Err(associated::invalid());
                }
            }
            ReadScopeArgument::Lexical { callable, binding } => {
                if binder != Some(ReadScopeBinder::Declaration(callable))
                    || !loans.contains(&binding)
                {
                    return Err(associated::invalid());
                }
                let declaration = self.declaration(callable)?;
                let DeclarationPayload::Function(function) = declaration.payload else {
                    return Err(associated::invalid());
                };
                let expected_kind = match function.effect {
                    crate::platform::kernel::FunctionEffect::Pure => {
                        crate::platform::kernel::OwnerKind::PureFunction
                    }
                    crate::platform::kernel::FunctionEffect::Task { .. } => {
                        crate::platform::kernel::OwnerKind::TaskFunction
                    }
                };
                if declaration.header.contract_version
                    != crate::platform::canonical_schema::SCHEMA_EPOCH
                    || declaration.header.kind != expected_kind
                {
                    return Err(associated::invalid());
                }
                self.tick()?;
                let Some(OwnerRecord::Binding(record)) = self
                    .read
                    .owner(callable.package, OwnerKey::Binding(binding))?
                else {
                    return Err(associated::invalid());
                };
                if record.header.owner != OwnerKey::Binding(binding)
                    || record.header.contract_version
                        != crate::platform::canonical_schema::SCHEMA_EPOCH
                    || (record.value.is_some() && record.kind != BindingKind::ReadView)
                    || record.header.kind != crate::platform::kernel::OwnerKind::Binding
                    || record.declared_type.is_none()
                    || !matches!(
                        record.kind,
                        BindingKind::OwnedBorrow
                            | BindingKind::ReadView
                            | BindingKind::ReadViewBorrow
                    )
                {
                    return Err(associated::invalid());
                }
            }
        }
        Ok(())
    }

    fn context_scope_actuals(
        &mut self,
        context: usize,
    ) -> Result<Vec<ReadScopeArgument>, Diagnostic> {
        let Some(binder) = self.context_binder(context)? else {
            return Ok(Vec::new());
        };
        let declaration = self.declaration(binder.declaration())?;
        if declaration.header.contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH {
            return Ok(Vec::new());
        }
        let parameters = self.scope_parameters(binder)?;
        self.reserve::<ReadScopeArgument>(parameters.len())?;
        Ok(parameters
            .into_iter()
            .map(|parameter| {
                ReadScopeArgument::Parameter(ReadScopeReference {
                    binder,
                    parameter: parameter.id,
                })
            })
            .collect())
    }

    fn application_scopes(
        &mut self,
        source: usize,
        target: ReadScopeBinder,
        arguments: &[ReadScopeArgument],
        loans: &BTreeSet<BindingId>,
    ) -> Result<BTreeMap<ReadScopeReference, ReadScopeArgument>, Diagnostic> {
        let parameters = self.scope_parameters(target)?;
        if parameters.len() != arguments.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable scope substitution requires every ordered actual, including unused formals",
            ));
        }
        let binder = self.context_binder(source)?;
        self.reserve::<(ReadScopeReference, ReadScopeArgument)>(arguments.len())?;
        let mut scopes = BTreeMap::new();
        for (parameter, argument) in parameters.into_iter().zip(arguments) {
            self.validate_scope(*argument, binder, loans)?;
            scopes.insert(
                ReadScopeReference {
                    binder: target,
                    parameter: parameter.id,
                },
                *argument,
            );
        }
        Ok(scopes)
    }

    fn retain_call_scopes(
        &mut self,
        source: usize,
        target: usize,
        expression: Option<ExpressionId>,
        scopes: BTreeMap<ReadScopeReference, ReadScopeArgument>,
    ) -> Result<(), Diagnostic> {
        use std::collections::btree_map::Entry;

        let key = (source, target, expression);
        self.comparison_searches(scopes.len() + 3, self.associated.call_scopes.len(), 1)?;
        match self.associated.call_scopes.entry(key) {
            Entry::Occupied(entry) => {
                if entry.get() != &scopes {
                    return Err(associated::invalid());
                }
            }
            Entry::Vacant(entry) => {
                reserve_metadata::<(
                    (usize, usize, Option<ExpressionId>),
                    BTreeMap<ReadScopeReference, ReadScopeArgument>,
                )>(&mut self.observation, self.maximum_metadata_bytes, 1)?;
                entry.insert(scopes);
            }
        }
        Ok(())
    }

    fn call_scope(
        &mut self,
        call: recursive::CallSite,
        scope: ReadScopeArgument,
    ) -> Result<ReadScopeArgument, Diagnostic> {
        self.tick()?;
        let scopes = self
            .associated
            .call_scopes
            .get(&(call.from, call.to, call.expression))
            .ok_or_else(associated::invalid)?;
        // Replace only the exact callee formal once; inserted caller coordinates are opaque.
        match scope {
            ReadScopeArgument::Parameter(reference) => scopes
                .get(&reference)
                .copied()
                .ok_or_else(associated::invalid),
            ReadScopeArgument::Lexical { .. } => Ok(scope),
        }
    }

    fn scoped_inputs(
        &mut self,
        call: recursive::CallSite,
    ) -> Result<recursive::Inputs, Diagnostic> {
        self.diagnostic_context = WorkContext {
            phase: "recursive call inputs",
            owner: Some(self.contexts[call.from].key.owner),
            context: Some(call.from),
            expression: call.expression,
            query: None,
        };
        self.tick()?;
        // Scope validation is performed for every syntax placement before calls are recorded.
        // The disposable operand cache is reachable only after that complete scope admission.
        let owner = self.contexts[call.from].key.owner;
        let (binder, arguments) = if let Some(expression) = call.expression {
            self.tick()?;
            let Some(OwnerRecord::Expression(record)) = self
                .read
                .owner(owner.package, OwnerKey::Expression(expression))?
            else {
                return Err(associated::invalid());
            };
            match record.operation {
                ExpressionOperation::Call {
                    function,
                    read_scope_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    function,
                    read_scope_arguments,
                    ..
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    read_scope_arguments,
                    ..
                } => (ReadScopeBinder::Declaration(function), read_scope_arguments),
                ExpressionOperation::MethodCall {
                    contract,
                    method,
                    read_scope_arguments,
                    ..
                } => (
                    ReadScopeBinder::ContractMethod { contract, method },
                    read_scope_arguments,
                ),
                _ => return Err(associated::invalid()),
            }
        } else {
            let declaration = self.declaration(owner)?;
            let DeclarationPayload::OwnedImplementation(implementation) = declaration.payload
            else {
                return Err(associated::invalid());
            };
            let mapping = self.mapping(&implementation, self.contexts[call.from].key.method)?;
            self.reserve::<ReadScopeArgument>(mapping.read_scope_arguments.len())?;
            (
                ReadScopeBinder::Declaration(mapping.function),
                mapping.read_scope_arguments.clone(),
            )
        };
        let parameters = self.scope_parameters(binder)?;
        self.steps(parameters.len())?;
        let scopes = self
            .associated
            .call_scopes
            .get(&(call.from, call.to, call.expression))
            .ok_or_else(associated::invalid)?;
        if arguments.len() != parameters.len()
            || scopes.len() != parameters.len()
            || parameters.iter().zip(arguments).any(|(formal, actual)| {
                scopes.get(&ReadScopeReference {
                    binder,
                    parameter: formal.id,
                }) != Some(&actual)
            })
        {
            return Err(associated::invalid());
        }
        recursive::load_inputs(self, call)
    }

    fn type_contains_view(
        &mut self,
        package: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<bool, Diagnostic> {
        let previous = self.diagnostic_context;
        self.diagnostic_context.phase = "view root closure";
        self.diagnostic_context.query = Some((package, ty));
        let result = (|| {
            // Preserve the initial query checkpoint/work and metadata charge.
            self.reserve::<(PackageId, TypeObjectDigest, usize)>(1)?;
            self.tick()?;
            self.reserve::<(PackageId, TypeObjectDigest)>(1)?;
            let object = self.read.type_object(package, ty)?.ok_or_else(|| {
                semantic(
                    "kernel_callable_flow_type",
                    "complete callable source is missing a type in the view occurrence closure",
                )
            })?;
            if object.form.is_read_view() {
                return Ok(true);
            }
            // Primitive/closed leaves need no cache search. Nominals still require
            // their complete raw fields/cases, even when they have no type actuals.
            if object.child_type_count() == 0
                && !matches!(
                    object.form,
                    TypeForm::Named { .. } | TypeForm::Applied { .. }
                )
            {
                return Ok(false);
            }
            let key = (package, ty);
            // Fixed-size identity keys require no linear loan/witness comparison.
            // Account a finite B-tree search before lookup, and before insertion.
            let levels = (usize::BITS
                - self
                    .completed_view_queries
                    .len()
                    .saturating_add(1)
                    .leading_zeros()) as usize;
            self.steps(levels.saturating_mul(24).saturating_add(2))?;
            if let Some(result) = self.completed_view_queries.get(&key) {
                return Ok(*result);
            }
            let result = self.type_contains_view_uncached(package, ty, object)?;
            self.reserve::<((PackageId, TypeObjectDigest), bool)>(1)?;
            self.steps(levels.saturating_mul(24).saturating_add(2))?;
            // Only this successful complete root result is published. Internal
            // visited nodes and in-progress cyclic paths are never memo entries.
            self.completed_view_queries.insert(key, result);
            Ok(result)
        })();
        if result.is_ok() {
            self.diagnostic_context = previous;
        }
        result
    }

    fn type_contains_view_uncached(
        &mut self,
        package: PackageId,
        ty: TypeObjectDigest,
        root: TypeObject,
    ) -> Result<bool, Diagnostic> {
        let mut root = Some(root);
        let mut pending = vec![(package, ty, 0usize)];
        let mut visited = BTreeSet::new();
        while let Some((package, ty, depth)) = pending.pop() {
            if root.is_none() {
                self.tick()?;
            }
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_callable_flow_type_depth",
                    "view occurrence exceeds finite structural admission",
                ));
            }
            if visited.contains(&(package, ty)) {
                continue;
            }
            if root.is_none() {
                self.reserve::<(PackageId, TypeObjectDigest)>(1)?;
            }
            visited.insert((package, ty));
            let object = match root.take() {
                Some(root) => root,
                None => self.read.type_object(package, ty)?.ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_type",
                        "complete callable source is missing a type in the view occurrence closure",
                    )
                })?,
            };
            if object.form.is_read_view() {
                return Ok(true);
            }
            self.reserve::<(PackageId, TypeObjectDigest, usize)>(object.child_type_count())?;
            pending.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|child| (package, child, depth + 1)),
            );
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } =
                object.form
            {
                let record = self.declaration(declaration)?;
                let (fields, cases) = match record.payload {
                    DeclarationPayload::Record { fields, .. }
                    | DeclarationPayload::OwnedRecord { fields, .. } => (fields, Vec::new()),
                    DeclarationPayload::Variant { cases, .. } => (Vec::new(), cases),
                    DeclarationPayload::ReadView(_) => return Ok(true),
                    _ => continue,
                };
                for field in fields {
                    self.tick()?;
                    let Some(OwnerRecord::Field(record)) = self
                        .read
                        .owner(declaration.package, OwnerKey::Field(field))?
                    else {
                        return Err(associated::invalid());
                    };
                    if record.declaration != declaration.declaration
                        || record.header.owner != OwnerKey::Field(field)
                    {
                        return Err(associated::invalid());
                    }
                    self.reserve::<(PackageId, TypeObjectDigest, usize)>(1)?;
                    pending.push((declaration.package, record.ty, depth + 1));
                }
                for case in cases {
                    self.tick()?;
                    let Some(OwnerRecord::Case(record)) =
                        self.read.owner(declaration.package, OwnerKey::Case(case))?
                    else {
                        return Err(associated::invalid());
                    };
                    if record.declaration != declaration.declaration
                        || record.header.owner != OwnerKey::Case(case)
                    {
                        return Err(associated::invalid());
                    }
                    if let Some(ty) = record.payload {
                        self.reserve::<(PackageId, TypeObjectDigest, usize)>(1)?;
                        pending.push((declaration.package, ty, depth + 1));
                    }
                }
            }
        }
        Ok(false)
    }

    fn signature_type(
        &mut self,
        context: usize,
        ty: TypeObjectDigest,
        binder: Option<ReadScopeBinder>,
        allow_view: bool,
        loans: &BTreeSet<BindingId>,
    ) -> Result<(), Diagnostic> {
        let package = self.contexts[context].key.owner.package;
        let object = self
            .read
            .type_object(package, ty)?
            .ok_or_else(associated::invalid)?;
        if !object.form.is_read_view() {
            if self.type_contains_view(package, ty)? {
                return Err(semantic(
                    "kernel_callable_flow_category",
                    "ordinary signature closure cannot hide a ReadView",
                ));
            }
            return Ok(());
        }
        if !allow_view {
            return Err(semantic(
                "kernel_callable_flow_category",
                "ReadView signature occurrence requires an explicit view position",
            ));
        }
        self.associated.enabled = true;
        let scope = object.form.read_scope().ok_or_else(associated::invalid)?;
        self.validate_scope(scope, binder, loans)?;
        match object.form {
            TypeForm::RegionCap {
                mode: crate::platform::kernel::RegionMode::Shared { .. },
                ..
            } => {}
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                self.projection_slot(
                    context,
                    scope,
                    parameter,
                    contract,
                    member,
                    Some(read_scope),
                )?;
            }
            TypeForm::AssociatedReadViewMember {
                contract, member, ..
            } => {
                if !matches!(binder, Some(ReadScopeBinder::ContractMethod { contract: expected, .. }) if expected == contract)
                    || !matches!(
                        self.associated_member(contract, member)?,
                        AssociatedTypeBound::ReadViewFamily { .. }
                    )
                {
                    return Err(associated::invalid());
                }
            }
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                ..
            } => {
                let nominal = self.declaration(declaration)?;
                let DeclarationPayload::ReadView(view) = nominal.payload else {
                    return Err(associated::invalid());
                };
                if nominal.header.contract_version
                    != crate::platform::canonical_schema::SCHEMA_EPOCH
                    || view.type_parameters.len() != arguments.len()
                {
                    return Err(associated::invalid());
                }
                for argument in arguments {
                    self.tick()?;
                    if self.type_contains_view(package, argument)? {
                        return Err(semantic(
                            "kernel_callable_flow_category",
                            "read-view nominal type actuals must have complete ordinary or Owned closure",
                        ));
                    }
                }
            }
            _ => return Err(associated::invalid()),
        }
        Ok(())
    }

    fn signature_scopes<S: Copy + Ord>(
        &mut self,
        binder: ReadScopeBinder,
        parameters: &[ReadScopeParameter],
        sources: &[ReadScopeSource<S>],
        source_exists: impl FnMut(S) -> bool,
    ) -> Result<(), Diagnostic> {
        if parameters.is_empty() && sources.is_empty() {
            return Ok(());
        }
        self.associated.enabled = true;
        self.reserve::<ReadScopeParameter>(parameters.len())?;
        self.reserve::<ReadScopeSource<S>>(sources.len())?;
        crate::platform::kernel::validate_read_scope_inventory_at(
            binder,
            parameters,
            sources,
            source_exists,
        )?;
        for source in sources {
            self.validate_scope(source.scope, Some(binder), &BTreeSet::new())?;
        }
        let exact = self.scope_parameters(binder)?;
        if exact != parameters {
            return Err(associated::invalid());
        }
        Ok(())
    }

    fn declaration_scope_inventory(
        &mut self,
        owner: DeclarationReference,
        payload: &DeclarationPayload,
    ) -> Result<(), Diagnostic> {
        match payload {
            DeclarationPayload::Function(function) => {
                let binder = ReadScopeBinder::Declaration(owner);
                self.signature_scopes(
                    binder,
                    &function.read_scope_parameters,
                    &function.read_scope_sources,
                    |source| function.parameters.contains(&source),
                )?;
                if let ResultRelation::ViewFrom { scope, .. } = function.result.relation {
                    self.validate_scope(scope, Some(binder), &BTreeSet::new())?;
                }
            }
            DeclarationPayload::OwnedContract(contract) => {
                for member in &contract.associated_types {
                    self.tick()?;
                    if matches!(member.bound, AssociatedTypeBound::ReadViewFamily { .. }) {
                        self.associated.enabled = true;
                        self.scope_parameters(ReadScopeBinder::AssociatedMember {
                            contract: owner,
                            member: member.id,
                        })?;
                    }
                }
                for method in &contract.methods {
                    self.tick()?;
                    let binder = ReadScopeBinder::ContractMethod {
                        contract: owner,
                        method: method.id,
                    };
                    self.signature_scopes(
                        binder,
                        &method.read_scope_parameters,
                        &method.read_scope_sources,
                        |source| (source as usize) < method.parameters.len(),
                    )?;
                    if let ResultRelation::ViewFrom { scope, .. } = method.result.relation {
                        self.validate_scope(scope, Some(binder), &BTreeSet::new())?;
                    }
                }
            }
            DeclarationPayload::ReadView(view) => {
                self.associated.enabled = true;
                self.scope_parameters(ReadScopeBinder::Declaration(owner))?;
                view.validate_local_at(owner)?;
                for field in &view.fields {
                    self.tick()?;
                    if let crate::platform::kernel::ReadViewFieldKind::ReadReference {
                        read_scope,
                    } = field.kind
                    {
                        self.validate_scope(
                            read_scope,
                            Some(ReadScopeBinder::Declaration(owner)),
                            &BTreeSet::new(),
                        )?;
                    }
                }
            }
            DeclarationPayload::Record { fields, .. }
            | DeclarationPayload::OwnedRecord { fields, .. } => {
                for id in fields {
                    self.tick()?;
                    let Some(OwnerRecord::Field(field)) =
                        self.read.owner(owner.package, OwnerKey::Field(*id))?
                    else {
                        return Err(associated::invalid());
                    };
                    if field.declaration != owner.declaration
                        || field.header.owner != OwnerKey::Field(*id)
                    {
                        return Err(associated::invalid());
                    }
                    if self.type_contains_view(owner.package, field.ty)? {
                        return Err(semantic(
                            "kernel_callable_flow_category",
                            "ordinary and Owned field inventories cannot contain a ReadView, including unused fields",
                        ));
                    }
                }
            }
            DeclarationPayload::Variant { cases, .. } => {
                for id in cases {
                    self.tick()?;
                    let Some(OwnerRecord::Case(case)) =
                        self.read.owner(owner.package, OwnerKey::Case(*id))?
                    else {
                        return Err(associated::invalid());
                    };
                    if case.declaration != owner.declaration
                        || case.header.owner != OwnerKey::Case(*id)
                    {
                        return Err(associated::invalid());
                    }
                    if let Some(ty) = case.payload
                        && self.type_contains_view(owner.package, ty)?
                    {
                        return Err(semantic(
                            "kernel_callable_flow_category",
                            "ordinary variant cases cannot contain a ReadView",
                        ));
                    }
                }
            }
            DeclarationPayload::External(function) => {
                if self.type_contains_view(owner.package, function.result)? {
                    return Err(associated::invalid());
                }
                for id in &function.parameters {
                    self.tick()?;
                    let Some(OwnerRecord::Parameter(parameter)) =
                        self.read.owner(owner.package, OwnerKey::Parameter(*id))?
                    else {
                        return Err(associated::invalid());
                    };
                    if self.type_contains_view(owner.package, parameter.ty)? {
                        return Err(associated::invalid());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<(), Diagnostic> {
        self.steps(1)
    }
    fn steps(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.steps_checked(Some(count))
    }
    fn steps_checked(&mut self, count: Option<usize>) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        let admitted = match count {
            Some(count) => self.work.charge(count, self.maximum_work),
            None => self.work.charge_product(usize::MAX, 2, self.maximum_work),
        };
        if !admitted {
            let mut diagnostic = Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_work",
                "composed callable flow exhausted its work admission",
            );
            let context = self.diagnostic_context;
            let root_context = context.context.and_then(|index| self.contexts.get(index));
            let method = root_context.and_then(|info| info.key.method);
            let root_owner = root_context.map(|info| info.key.owner);
            diagnostic.notes.push(format!(
                "operation={} owner={:?} context={:?} context_owner={:?} method={:?} expression={:?} root={:?} consumed={} requested={} maximum={:?}",
                context.phase, context.owner, context.context, root_owner, method,
                context.expression, context.query, self.work.consumed, count.map_or_else(|| "overflow".to_owned(), |count| count.to_string()), self.maximum_work,
            ));
            return Err(diagnostic);
        }
        Ok(())
    }
    fn comparison_work(&mut self, width: usize, keys: usize) -> Result<(), Diagnostic> {
        self.comparison_searches(width, keys, 2)
    }
    fn comparison_searches(
        &mut self,
        width: usize,
        keys: usize,
        searches: usize,
    ) -> Result<(), Diagnostic> {
        // Conservatively charge each tree descent, including comparison of the
        // complete key. A vacant Entry inserts without a second key search.
        let levels = (usize::BITS - keys.saturating_add(1).leading_zeros()) as usize;
        let comparisons = levels
            .checked_mul(12)
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(searches));
        self.steps_checked(
            width.checked_add(1).and_then(|width| {
                comparisons.and_then(|comparisons| width.checked_mul(comparisons))
            }),
        )
    }
    fn intern_selection(
        &mut self,
        implementation: DeclarationReference,
        prerequisites: Vec<SelectionId>,
    ) -> Result<SelectionId, Diagnostic> {
        self.tick()?;
        let mut depth = 0usize;
        let parameters = self.scheme_parameters(implementation)?;
        let DeclarationPayload::OwnedImplementation(scheme) =
            self.declaration(implementation)?.payload
        else {
            return Err(associated::invalid());
        };
        for (parameter, selected) in scheme.implementation_parameters.iter().zip(&prerequisites) {
            if self.selection_contract(*selected)? != parameter.contract {
                return Err(associated::invalid());
            }
        }
        let mut has_parameters = !parameters.is_empty();
        for child in &prerequisites {
            self.tick()?;
            depth = depth.max(self.selections[child.0].depth + 1);
            has_parameters |= self.selections[child.0].has_parameters;
        }
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "exact prerequisite context exceeds bounded witness depth",
            ));
        }
        let width = prerequisites.len();
        let shape = Selection::Scheme {
            implementation,
            prerequisites,
        };
        self.comparison_work(width, self.selection_indexes.len())?;
        if let Some(id) = self.selection_indexes.get(&shape) {
            return Ok(*id);
        }
        self.reserve::<SelectionInfo>(1)?;
        self.reserve::<(Selection, SelectionId)>(1)?;
        self.reserve::<SelectionId>(width)?;
        let id = SelectionId(self.selections.len());
        self.selection_indexes.insert(shape.clone(), id);
        self.selections.push(SelectionInfo {
            shape,
            parameters,
            depth,
            has_parameters,
        });
        Ok(id)
    }
    fn selected_shape(&mut self, id: SelectionId) -> Result<Selection, Diagnostic> {
        self.tick()?;
        if let Selection::Scheme { prerequisites, .. } = &self.selections[id.0].shape {
            let width = prerequisites.len();
            self.steps(width)?;
            self.reserve::<SelectionId>(width)?;
        }
        Ok(self.selections[id.0].shape.clone())
    }
    #[cfg(test)]
    fn witness_slots<'c>(
        &mut self,
        context: &'c ContextInfo,
        path: &[usize],
    ) -> Result<&'c [usize], Diagnostic> {
        let comparisons =
            (usize::BITS - context.witnesses.len().saturating_add(1).leading_zeros()) as usize + 1;
        self.steps_checked(
            path.len()
                .checked_add(1)
                .and_then(|width| width.checked_mul(comparisons)),
        )?;
        Ok(context.witness_slots(path))
    }
    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        reserve_metadata::<T>(&mut self.observation, self.maximum_metadata_bytes, count)
    }
    fn reserve_product<T>(&mut self, count: usize, multiple: usize) -> Result<(), Diagnostic> {
        reserve_metadata_product::<T>(
            &mut self.observation,
            self.maximum_metadata_bytes,
            count,
            multiple,
        )
    }
    fn declaration(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<DeclarationRecord, Diagnostic> {
        self.diagnostic_context.owner = Some(reference);
        self.tick()?;
        match self.read.owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )? {
            Some(OwnerRecord::Declaration(record))
                if record.header.owner == OwnerKey::Declaration(reference.declaration) =>
            {
                Ok(record)
            }
            _ => Err(semantic(
                "kernel_callable_flow_target",
                "complete callable source is missing an exact declaration",
            )),
        }
    }
    fn scheme_parameters(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<Vec<TypeParameterId>, Diagnostic> {
        match self.declaration(reference)?.payload {
            DeclarationPayload::OwnedImplementation(i) => {
                self.reserve::<TypeParameterId>(i.type_parameters.len())?;
                Ok(i.type_parameters)
            }
            _ => Err(semantic(
                "kernel_callable_flow_witness",
                "selected witness does not name an implementation scheme",
            )),
        }
    }
    fn mapping<'s>(
        &mut self,
        scheme: &'s OwnedImplementation,
        method: Option<MethodId>,
    ) -> Result<&'s OwnedMethodImplementation, Diagnostic> {
        for mapping in &scheme.methods {
            self.tick()?;
            if Some(mapping.method) == method {
                return Ok(mapping);
            }
        }
        Err(semantic(
            "kernel_callable_flow_method",
            "implementation method is absent",
        ))
    }
    fn parameter_ordinal(
        &mut self,
        payload: &DeclarationPayload,
        parameter: ImplementationParameterId,
    ) -> Result<usize, Diagnostic> {
        for (ordinal, candidate) in implementation_parameters(payload).iter().enumerate() {
            self.tick()?;
            if candidate.id == parameter {
                return Ok(ordinal);
            }
        }
        Err(semantic(
            "kernel_callable_flow_witness_scope",
            "witness parameter is absent",
        ))
    }
    fn context(&mut self, key: Context) -> Result<usize, Diagnostic> {
        self.tick()?;
        // Interned selection IDs keep comparison and key cloning independent of DAG paths.
        self.reserve::<SelectionId>(key.implementations.len())?;
        self.comparison_work(key.implementations.len(), self.indexes.len())?;
        if let Some(index) = self.indexes.get(&key) {
            return Ok(*index);
        }
        let declaration = self.declaration(key.owner)?;
        let parameter_inventory = declaration.payload.type_parameters();
        self.reserve::<TypeParameterId>(parameter_inventory.iter().count())?;
        let parameters = parameter_inventory.to_vec();
        match &declaration.payload {
            DeclarationPayload::Function(f) => {
                if key.method.is_some()
                    || f.implementation_parameters.len() != key.implementations.len()
                {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "function context has a foreign method or witness arity",
                    ));
                }
                for (parameter, selected) in
                    f.implementation_parameters.iter().zip(&key.implementations)
                {
                    if self.selection_contract(*selected)? != parameter.contract {
                        return Err(associated::invalid());
                    }
                }
            }
            DeclarationPayload::OwnedImplementation(i) => {
                if i.implementation_parameters.len() != key.implementations.len() {
                    return Err(semantic(
                        "kernel_callable_flow_method",
                        "implementation context has no exact method mapping",
                    ));
                }
                for (parameter, selected) in
                    i.implementation_parameters.iter().zip(&key.implementations)
                {
                    if self.selection_contract(*selected)? != parameter.contract {
                        return Err(associated::invalid());
                    }
                }
                self.mapping(i, key.method)?;
            }
            _ if key.method.is_some() || !key.implementations.is_empty() => {
                return Err(semantic(
                    "kernel_callable_flow_context",
                    "declaration cannot carry this callable context",
                ));
            }
            _ => {}
        }
        // Discover exact callable contexts before expanding typed witness paths.
        // Only contexts participating in a call cycle can contain a type-growth cycle.
        let total = parameters.len();
        self.reserve::<ContextInfo>(1)?;
        self.reserve::<(Context, usize)>(1)?;
        self.reserve::<usize>(total)?;
        self.reserve::<(usize, usize)>(total)?;
        self.reserve::<usize>(1)?;
        let index = self.contexts.len();
        let slots = (0..total)
            .map(|ordinal| {
                let slot = self.slot_owners.len();
                self.slot_owners.push((index, ordinal));
                slot
            })
            .collect();
        self.indexes.insert(key.clone(), index);
        self.contexts.push(ContextInfo {
            key,
            parameters,
            #[cfg(test)]
            witnesses: Vec::new(),
            slots,
        });
        self.pending.push(index);
        Ok(index)
    }
    #[cfg(test)]
    fn materialize_witnesses(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.steps(self.contexts[index].key.implementations.len())?;
        self.reserve::<SelectionId>(self.contexts[index].key.implementations.len())?;
        let selections = self.contexts[index].key.implementations.clone();
        let start = self.contexts[index].parameters.len();
        let mut total = start;
        let mut witnesses = Vec::new();
        for (ordinal, selected) in selections.into_iter().enumerate() {
            self.reserve::<usize>(1)?;
            self.witness_layout(selected, vec![ordinal], &mut witnesses, &mut total)?;
        }
        self.reserve::<usize>(total - start)?;
        self.reserve::<(usize, usize)>(total - start)?;
        for ordinal in start..total {
            self.tick()?;
            let slot = self.slot_owners.len();
            self.slot_owners.push((index, ordinal));
            self.contexts[index].slots.push(slot);
        }
        self.contexts[index].witnesses = witnesses;
        Ok(())
    }
    fn record_call(
        &mut self,
        from: usize,
        to: usize,
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.reserve::<recursive::CallSite>(1)?;
        self.calls.push(recursive::CallSite {
            from,
            to,
            expression,
        });
        Ok(())
    }
    fn info(&mut self, index: usize) -> Result<ContextInfo, Diagnostic> {
        // Discovery copies flat context metadata. The eager reference additionally
        // retains separately metered witness paths only in source tests.
        self.steps(self.contexts[index].key.implementations.len())?;
        self.steps(self.contexts[index].parameters.len() + self.contexts[index].slots.len())?;
        self.reserve::<ContextInfo>(1)?;
        self.reserve::<TypeParameterId>(self.contexts[index].parameters.len())?;
        #[cfg(test)]
        {
            self.steps(self.contexts[index].witnesses.len())?;
            self.reserve::<WitnessLayout>(self.contexts[index].witnesses.len())?;
            for ordinal in 0..self.contexts[index].witnesses.len() {
                self.steps(
                    self.contexts[index].witnesses[ordinal].path.len()
                        + self.contexts[index].witnesses[ordinal].parameters.len(),
                )?;
                self.reserve::<TypeParameterId>(
                    self.contexts[index].witnesses[ordinal].parameters.len(),
                )?;
                self.reserve::<usize>(self.contexts[index].witnesses[ordinal].path.len())?;
            }
        }
        self.reserve::<usize>(self.contexts[index].slots.len())?;
        self.reserve::<SelectionId>(self.contexts[index].key.implementations.len())?;
        Ok(self.contexts[index].clone())
    }
    #[cfg(test)]
    fn witness_layout(
        &mut self,
        selection: SelectionId,
        path: Vec<usize>,
        result: &mut Vec<WitnessLayout>,
        total: &mut usize,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        if !self.selections[selection.0].has_parameters {
            return Ok(());
        }
        let Selection::Scheme { prerequisites, .. } = self.selected_shape(selection)? else {
            return Ok(());
        };
        self.reserve::<TypeParameterId>(self.selections[selection.0].parameters.len())?;
        self.steps(self.selections[selection.0].parameters.len() + path.len())?;
        let parameters = self.selections[selection.0].parameters.clone();
        self.reserve::<WitnessLayout>(1)?;
        self.reserve::<usize>(path.len())?;
        let start = *total;
        *total = total
            .checked_add(parameters.len())
            .ok_or_else(super::storage_overflow)?;
        result.push(WitnessLayout {
            path: path.clone(),
            parameters,
            start,
        });
        for (ordinal, prerequisite) in prerequisites.iter().enumerate() {
            self.reserve::<usize>(path.len() + 1)?;
            let mut child = path.clone();
            child.push(ordinal);
            self.witness_layout(*prerequisite, child, result, total)?;
        }
        Ok(())
    }
    fn edge(
        &mut self,
        from: usize,
        to: usize,
        origin: DeclarationReference,
        expression: Option<ExpressionId>,
        argument: usize,
        path: Vec<usize>,
    ) -> Result<(), Diagnostic> {
        self.push_edge(Edge {
            from,
            to,
            origin,
            expression,
            argument,
            expanding: !path.is_empty(),
            path,
        })
    }
    fn push_edge(&mut self, edge: Edge) -> Result<(), Diagnostic> {
        self.tick()?;
        self.reserve::<Edge>(1)?;
        self.edges.push(edge);
        Ok(())
    }
    fn arguments(
        &mut self,
        source: usize,
        target_slots: &[usize],
        arguments: &[TypeObjectDigest],
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        self.argument_types(
            source,
            target_slots.len(),
            Some(target_slots),
            arguments,
            expression,
        )
    }
    fn argument_types(
        &mut self,
        source: usize,
        expected: usize,
        targets: Option<&[usize]>,
        arguments: &[TypeObjectDigest],
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        if arguments.len() != expected {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable application requires every ordered type argument",
            ));
        }
        let caller = self.contexts[source].key.owner;
        for (argument, ty) in arguments.iter().enumerate() {
            if self.type_contains_view(caller.package, *ty)? {
                return Err(semantic(
                    "kernel_callable_flow_category",
                    "ReadView cannot be an ordinary type actual, including hidden or unused nominal content",
                ));
            }
            self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
            let mut pending = vec![(*ty, Vec::new())];
            let mut visited = BTreeSet::new();
            while let Some((ty, path)) = pending.pop() {
                self.tick()?;
                let occurrence = (ty, !path.is_empty());
                if visited.contains(&occurrence) {
                    continue;
                }
                self.reserve::<(TypeObjectDigest, bool)>(1)?;
                visited.insert(occurrence);
                let object = self.read.type_object(caller.package, ty)?.ok_or_else(|| {
                    semantic(
                        "kernel_callable_flow_type",
                        "complete callable source is missing an application type",
                    )
                })?;
                if object.form.is_read_view() {
                    return Err(semantic(
                        "kernel_callable_flow_category",
                        "ReadView cannot be an ordinary type actual, including a phantom or nested actual",
                    ));
                }
                if let TypeForm::AssociatedWitnessProjection {
                    scope,
                    parameter,
                    contract,
                    member,
                } = object.form
                {
                    let from =
                        self.projection_slot(source, scope, parameter, contract, member, None)?;
                    if let Some(targets) = targets {
                        self.edge(from, targets[argument], caller, expression, argument, path)?;
                    }
                } else if matches!(object.form, TypeForm::AssociatedContractMember { .. }) {
                    return Err(associated::invalid());
                } else if let TypeForm::TypeParameter { parameter } = object.form {
                    let ordinal = self.contexts[source]
                        .parameters
                        .iter()
                        .position(|p| *p == parameter)
                        .ok_or_else(|| {
                            semantic(
                                "kernel_type_parameter_scope",
                                "callable argument contains a foreign source parameter",
                            )
                        })?;
                    if let Some(targets) = targets {
                        let from = self.contexts[source].slots[ordinal];
                        self.edge(from, targets[argument], caller, expression, argument, path)?;
                    }
                } else {
                    self.reserve::<TypeObjectDigest>(object.child_type_count())?;
                    for (index, child) in object.child_types().into_iter().enumerate() {
                        self.tick()?;
                        if path.len() >= crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                            return Err(Diagnostic::new(
                                DiagnosticClass::Resource,
                                "kernel_callable_flow_type_depth",
                                "callable argument exceeds structural type depth",
                            ));
                        }
                        self.reserve::<usize>(path.len() + 1)?;
                        self.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                        let mut child_path = path.clone();
                        child_path.push(index);
                        pending.push((child, child_path));
                    }
                }
            }
        }
        Ok(())
    }
    fn selection(
        &mut self,
        source: usize,
        operand: &ImplementationOperand,
    ) -> Result<SelectionId, Diagnostic> {
        self.selection_at(source, operand, 0)
    }
    fn selection_at(
        &mut self,
        source: usize,
        operand: &ImplementationOperand,
        depth: usize,
    ) -> Result<SelectionId, Diagnostic> {
        self.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_callable_flow_witness_depth",
                "applied witness exceeds bounded prerequisite depth",
            ));
        }
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let DeclarationPayload::OwnedImplementation(scheme) =
                    self.declaration(*implementation)?.payload
                else {
                    return Err(semantic(
                        "kernel_callable_flow_witness",
                        "selected witness does not name an implementation scheme",
                    ));
                };
                if scheme.type_parameters.len() != type_arguments.len()
                    || scheme.implementation_parameters.len() != implementations.len()
                {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "applied witness requires every ordered scheme argument",
                    ));
                }
                // Acyclic contexts still admit every supplied type, including phantom
                // arguments and operands in unused methods or untaken syntax.
                self.argument_types(
                    source,
                    scheme.type_parameters.len(),
                    None,
                    type_arguments,
                    None,
                )?;
                self.reserve::<SelectionId>(implementations.len())?;
                let mut prerequisites = Vec::new();
                for operand in implementations {
                    prerequisites.push(self.selection_at(source, operand, depth + 1)?);
                }
                self.intern_selection(*implementation, prerequisites)
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let owner = self.contexts[source].key.owner;
                if *scope != owner {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter escapes its exact callable scope",
                    ));
                }
                let declaration = self.declaration(owner)?;
                let ordinal = self.parameter_ordinal(&declaration.payload, *parameter)?;
                Ok(self.contexts[source].key.implementations[ordinal])
            }
        }
    }
    #[cfg(test)]
    fn witness_arguments(
        &mut self,
        source: usize,
        target: &ContextInfo,
        target_path: &[usize],
        operand: &ImplementationOperand,
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                let slots = self.witness_slots(target, target_path)?;
                self.arguments(source, slots, type_arguments, expression)?;
                for (ordinal, operand) in implementations.iter().enumerate() {
                    self.reserve::<usize>(target_path.len() + 1)?;
                    let mut path = target_path.to_vec();
                    path.push(ordinal);
                    self.witness_arguments(source, target, &path, operand, expression)?;
                }
                Ok(())
            }
            ImplementationOperand::Parameter { parameter, .. } => {
                let caller = self.contexts[source].key.owner;
                let declaration = self.declaration(caller)?;
                let ordinal = self.parameter_ordinal(&declaration.payload, *parameter)?;
                let info = self.info(source)?;
                self.forward_witness(&info, &[ordinal], target, target_path, caller, expression)
            }
        }
    }
    #[cfg(test)]
    fn forward_witness(
        &mut self,
        source: &ContextInfo,
        source_path: &[usize],
        target: &ContextInfo,
        target_path: &[usize],
        caller: DeclarationReference,
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        for layout in &source.witnesses {
            self.steps(source_path.len() + 1)?;
            if let Some(suffix) = layout.path.strip_prefix(source_path) {
                self.reserve::<usize>(target_path.len() + suffix.len())?;
                let mut path = target_path.to_vec();
                path.extend_from_slice(suffix);
                let from = self.witness_slots(source, &layout.path)?;
                let to = self.witness_slots(target, &path)?;
                if from.len() != to.len() {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "forwarded witness scheme arity differs",
                    ));
                }
                for (argument, (from, to)) in from.iter().zip(to).enumerate() {
                    self.edge(*from, *to, caller, expression, argument, Vec::new())?;
                }
            }
        }
        Ok(())
    }
    #[cfg(test)]
    fn method_arguments(
        &mut self,
        source: usize,
        target: &ContextInfo,
        operand: &ImplementationOperand,
        expression: ExpressionId,
    ) -> Result<(), Diagnostic> {
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                self.arguments(
                    source,
                    &target.slots[..target.parameters.len()],
                    type_arguments,
                    Some(expression),
                )?;
                for (ordinal, operand) in implementations.iter().enumerate() {
                    if !matches!(
                        self.selections[target.key.implementations[ordinal].0].shape,
                        Selection::Unknown { .. }
                    ) {
                        self.witness_arguments(
                            source,
                            target,
                            &[ordinal],
                            operand,
                            Some(expression),
                        )?;
                    }
                }
            }
            ImplementationOperand::Parameter { parameter, .. } => {
                let info = self.info(source)?;
                let declaration = self.declaration(info.key.owner)?;
                let ordinal = self.parameter_ordinal(&declaration.payload, *parameter)?;
                let from = self.witness_slots(&info, &[ordinal])?;
                let to = &target.slots[..target.parameters.len()];
                if from.len() != to.len() {
                    return Err(semantic(
                        "kernel_callable_flow_arity",
                        "selected method scheme arity differs",
                    ));
                }
                for (argument, (from, to)) in from.iter().zip(to).enumerate() {
                    self.edge(
                        *from,
                        *to,
                        info.key.owner,
                        Some(expression),
                        argument,
                        Vec::new(),
                    )?;
                }
                for child in 0..target.key.implementations.len() {
                    self.forward_witness(
                        &info,
                        &[ordinal, child],
                        target,
                        &[child],
                        info.key.owner,
                        Some(expression),
                    )?;
                }
            }
        }
        Ok(())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "ordinary, witness and scope actuals retain their exact caller lexical authority"
    )]
    fn application(
        &mut self,
        source: usize,
        expression: Option<ExpressionId>,
        function: DeclarationReference,
        arguments: &[TypeObjectDigest],
        operands: &[ImplementationOperand],
        scope_arguments: &[ReadScopeArgument],
        loans: &BTreeSet<BindingId>,
    ) -> Result<(), Diagnostic> {
        self.tick()?;
        self.observation.applications += 1;
        // Admit the full scope map before a function/witness context can be reused.
        let scopes = self.application_scopes(
            source,
            ReadScopeBinder::Declaration(function),
            scope_arguments,
            loans,
        )?;
        self.reserve::<SelectionId>(operands.len())?;
        let mut implementations = Vec::new();
        for operand in operands {
            implementations.push(self.selection(source, operand)?);
        }
        let target = self.context(Context {
            owner: function,
            method: None,
            implementations,
        })?;
        self.argument_types(
            source,
            self.contexts[target].parameters.len(),
            None,
            arguments,
            expression,
        )?;
        self.retain_call_scopes(source, target, expression, scopes)?;
        self.record_call(source, target, expression)
    }
    #[cfg(test)]
    fn connect_application(
        &mut self,
        source: usize,
        target: &ContextInfo,
        arguments: &[TypeObjectDigest],
        operands: &[ImplementationOperand],
        expression: Option<ExpressionId>,
    ) -> Result<(), Diagnostic> {
        self.arguments(
            source,
            &target.slots[..target.parameters.len()],
            arguments,
            expression,
        )?;
        if operands.len() != target.key.implementations.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "callable witness arity changed",
            ));
        }
        for (ordinal, operand) in operands.iter().enumerate() {
            if !matches!(
                self.selections[target.key.implementations[ordinal].0].shape,
                Selection::Unknown { .. }
            ) {
                self.witness_arguments(source, target, &[ordinal], operand, expression)?;
            }
        }
        Ok(())
    }
    fn scan_signature(
        &mut self,
        source: usize,
        payload: &DeclarationPayload,
    ) -> Result<(), Diagnostic> {
        let owner = self.contexts[source].key.owner;
        match payload {
            DeclarationPayload::Function(function) => {
                let binder = Some(ReadScopeBinder::Declaration(owner));
                for id in &function.parameters {
                    self.tick()?;
                    let Some(OwnerRecord::Parameter(parameter)) =
                        self.read.owner(owner.package, OwnerKey::Parameter(*id))?
                    else {
                        return Err(associated::invalid());
                    };
                    if parameter.header.owner != OwnerKey::Parameter(*id)
                        || parameter.parent
                            != crate::platform::kernel::ParameterParent::Function(owner.declaration)
                    {
                        return Err(associated::invalid());
                    }
                    if parameter.header.contract_version
                        != crate::platform::canonical_schema::SCHEMA_EPOCH
                        && self.type_contains_view(owner.package, parameter.ty)?
                    {
                        return Err(associated::invalid());
                    }
                    self.signature_type(
                        source,
                        parameter.ty,
                        binder,
                        parameter.use_mode == crate::platform::kernel::ParameterUse::Borrow,
                        &BTreeSet::new(),
                    )?;
                }
                self.signature_type(
                    source,
                    function.result.ty,
                    binder,
                    matches!(function.result.relation, ResultRelation::ViewFrom { .. }),
                    &BTreeSet::new(),
                )?;
                let result = self
                    .read
                    .type_object(owner.package, function.result.ty)?
                    .ok_or_else(associated::invalid)?;
                if let Some(source_parameter) = function.result.relation.source() {
                    if function.effect != crate::platform::kernel::FunctionEffect::Pure
                        || !function.parameters.contains(source_parameter)
                    {
                        return Err(associated::invalid());
                    }
                    let Some(OwnerRecord::Parameter(source)) = self
                        .read
                        .owner(owner.package, OwnerKey::Parameter(*source_parameter))?
                    else {
                        return Err(associated::invalid());
                    };
                    if source.use_mode != crate::platform::kernel::ParameterUse::Borrow {
                        return Err(associated::invalid());
                    }
                }
                if let ResultRelation::ViewFrom { scope, .. } = function.result.relation
                    && result.form.read_scope() != Some(scope)
                {
                    return Err(associated::invalid());
                }
                if result.form.is_read_view()
                    != matches!(function.result.relation, ResultRelation::ViewFrom { .. })
                {
                    return Err(associated::invalid());
                }
            }
            DeclarationPayload::OwnedContract(contract) => {
                for method in &contract.methods {
                    self.tick()?;
                    let binder = Some(ReadScopeBinder::ContractMethod {
                        contract: owner,
                        method: method.id,
                    });
                    for parameter in &method.parameters {
                        self.tick()?;
                        self.signature_type(
                            source,
                            parameter.ty,
                            binder,
                            parameter.use_mode == crate::platform::kernel::ParameterUse::Borrow,
                            &BTreeSet::new(),
                        )?;
                    }
                    self.signature_type(
                        source,
                        method.result.ty,
                        binder,
                        matches!(method.result.relation, ResultRelation::ViewFrom { .. }),
                        &BTreeSet::new(),
                    )?;
                    let result = self
                        .read
                        .type_object(owner.package, method.result.ty)?
                        .ok_or_else(associated::invalid)?;
                    if let Some(source) = method.result.relation.source()
                        && (method.effect != crate::platform::kernel::FunctionEffect::Pure
                            || method
                                .parameters
                                .get(*source as usize)
                                .is_none_or(|parameter| {
                                    parameter.use_mode
                                        != crate::platform::kernel::ParameterUse::Borrow
                                }))
                    {
                        return Err(associated::invalid());
                    }
                    if let ResultRelation::ViewFrom { scope, .. } = method.result.relation
                        && result.form.read_scope() != Some(scope)
                    {
                        return Err(associated::invalid());
                    }
                    if result.form.is_read_view()
                        != matches!(method.result.relation, ResultRelation::ViewFrom { .. })
                    {
                        return Err(associated::invalid());
                    }
                }
            }
            DeclarationPayload::ReadView(view) => {
                let binder = Some(ReadScopeBinder::Declaration(owner));
                for field in &view.fields {
                    self.tick()?;
                    self.signature_type(
                        source,
                        field.ty,
                        binder,
                        matches!(
                            field.kind,
                            crate::platform::kernel::ReadViewFieldKind::ReadReference { .. }
                        ),
                        &BTreeSet::new(),
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn function_value(&mut self, function: DeclarationReference) -> Result<(), Diagnostic> {
        let declaration = self.declaration(function)?;
        let (parameters, result) = match declaration.payload {
            DeclarationPayload::Function(function) => {
                if !function.read_scope_parameters.is_empty()
                    || !function.read_scope_sources.is_empty()
                    || matches!(function.result.relation, ResultRelation::ViewFrom { .. })
                {
                    return Err(semantic(
                        "kernel_callable_flow_category",
                        "first-class callable values cannot carry read-scope or view contracts",
                    ));
                }
                (function.parameters, function.result.ty)
            }
            DeclarationPayload::External(function) => (function.parameters, function.result),
            _ => return Err(associated::invalid()),
        };
        if self.type_contains_view(function.package, result)? {
            return Err(associated::invalid());
        }
        for id in parameters {
            self.tick()?;
            let Some(OwnerRecord::Parameter(parameter)) =
                self.read.owner(function.package, OwnerKey::Parameter(id))?
            else {
                return Err(associated::invalid());
            };
            if self.type_contains_view(function.package, parameter.ty)? {
                return Err(semantic(
                    "kernel_callable_flow_category",
                    "first-class callable values cannot expose a ReadView parameter",
                ));
            }
        }
        Ok(())
    }

    fn loan_copy(
        &mut self,
        loans: &BTreeSet<BindingId>,
    ) -> Result<BTreeSet<BindingId>, Diagnostic> {
        self.steps(loans.len())?;
        self.reserve::<BindingId>(loans.len())?;
        Ok(loans.clone())
    }

    fn visit_syntax(
        &mut self,
        visited: &mut VisitedSyntax,
        occurrence: ScopedSyntaxOccurrence,
    ) -> Result<Option<BTreeSet<BindingId>>, Diagnostic> {
        self.diagnostic_context.phase = "syntax-key comparison";
        self.diagnostic_context.expression = Some(occurrence.0);
        self.tick()?;
        let (width, keys) = visited.comparison_shape(&occurrence.1, occurrence.2);
        self.comparison_searches(width, keys, 1)?;
        self.diagnostic_context.phase = "expression admission";
        visited.insert_with(occurrence, |occurrence| {
            if VisitedSyntax::plain_key(&occurrence.1, occurrence.2) {
                self.reserve::<ExpressionId>(1)?;
            } else {
                self.reserve::<ScopedSyntaxOccurrence>(1)?;
            }
            // Keep loan-copy work/checkpoint even for an empty loan set.
            self.loan_copy(&occurrence.1)
        })
    }

    fn scan(&mut self, source: usize) -> Result<(), Diagnostic> {
        self.diagnostic_context = WorkContext {
            phase: "callable scan",
            owner: Some(self.contexts[source].key.owner),
            context: Some(source),
            expression: None,
            query: None,
        };
        self.tick()?;
        let info = self.info(source)?;
        let declaration = self.declaration(info.key.owner)?;
        if let DeclarationPayload::OwnedImplementation(i) = &declaration.payload {
            let mapping = self.mapping(i, info.key.method)?;
            self.application(
                source,
                None,
                mapping.function,
                &mapping.type_arguments,
                &mapping.implementations,
                &mapping.read_scope_arguments,
                &BTreeSet::new(),
            )?;
            return Ok(());
        }
        self.scan_signature(source, &declaration.payload)?;
        self.observation.functions += usize::from(matches!(
            declaration.payload,
            DeclarationPayload::Function(_)
        ));
        let roots = declaration.expression_roots();
        self.reserve::<(ExpressionId, BTreeSet<BindingId>, Option<BindingId>)>(roots.len())?;
        let mut pending: Vec<_> = roots
            .into_iter()
            .map(|id| (id, BTreeSet::new(), None))
            .collect();
        if let DeclarationPayload::Component { ports, .. } = &declaration.payload {
            for port in ports {
                self.tick()?;
                let Some(OwnerRecord::Port(record)) = self
                    .read
                    .owner(info.key.owner.package, OwnerKey::Port(*port))?
                else {
                    return Err(semantic(
                        "kernel_callable_flow_port",
                        "complete callable source is missing a component port",
                    ));
                };
                let roots = OwnerRecord::Port(record).expression_roots();
                self.reserve::<(ExpressionId, BTreeSet<BindingId>, Option<BindingId>)>(
                    roots.len(),
                )?;
                pending.extend(roots.into_iter().map(|id| (id, BTreeSet::new(), None)));
            }
        }
        let source_binder = self.context_binder(source)?;
        let mut visited = VisitedSyntax::default();
        while let Some((expression, loans, establishing)) = pending.pop() {
            let Some(loans) = self.visit_syntax(&mut visited, (expression, loans, establishing))?
            else {
                continue;
            };
            let Some(OwnerRecord::Expression(record)) = self
                .read
                .owner(info.key.owner.package, OwnerKey::Expression(expression))?
            else {
                return Err(semantic(
                    "kernel_callable_flow_expression",
                    "complete callable source is missing body syntax",
                ));
            };
            if record.contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH
                && (record.operation.is_read_view_operation()
                    || !record.operation.read_scope_arguments().is_empty())
            {
                return Err(associated::invalid());
            }
            let mut metadata_loans = self.loan_copy(&loans)?;
            if let Some(binding) = establishing {
                self.reserve::<BindingId>(1)?;
                metadata_loans.insert(binding);
            }
            match &record.operation {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    read_scope_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    function,
                    type_arguments,
                    read_scope_arguments,
                    ..
                } => {
                    if matches!(record.operation, ExpressionOperation::FunctionValue { .. }) {
                        self.function_value(*function)?;
                    }
                    self.application(
                        source,
                        Some(expression),
                        *function,
                        type_arguments,
                        &[],
                        read_scope_arguments,
                        &metadata_loans,
                    )?;
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    read_scope_arguments,
                    ..
                } => {
                    self.application(
                        source,
                        Some(expression),
                        *function,
                        type_arguments,
                        implementations,
                        read_scope_arguments,
                        &metadata_loans,
                    )?;
                }
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    read_scope_arguments,
                    ..
                } => {
                    let scopes = self.application_scopes(
                        source,
                        ReadScopeBinder::ContractMethod {
                            contract: *contract,
                            method: *method,
                        },
                        read_scope_arguments,
                        &metadata_loans,
                    )?;
                    let selection = self.selection(source, witness)?;
                    if self.selection_contract(selection)? != *contract {
                        return Err(associated::invalid());
                    }
                    if let Selection::Scheme {
                        implementation,
                        prerequisites,
                    } = self.selected_shape(selection)?
                    {
                        let target = self.context(Context {
                            owner: implementation,
                            method: Some(*method),
                            implementations: prerequisites,
                        })?;
                        self.retain_call_scopes(source, target, Some(expression), scopes)?;
                        self.record_call(source, target, Some(expression))?;
                    }
                }
                ExpressionOperation::CreateReadView {
                    declaration,
                    type_arguments,
                    read_scope,
                    ..
                } => {
                    self.validate_scope(*read_scope, source_binder, &loans)?;
                    let record = self.declaration(*declaration)?;
                    let DeclarationPayload::ReadView(view) = record.payload else {
                        return Err(associated::invalid());
                    };
                    self.argument_types(
                        source,
                        view.type_parameters.len(),
                        None,
                        type_arguments,
                        Some(expression),
                    )?;
                }
                ExpressionOperation::ReborrowReadView { read_scope, .. } => {
                    self.validate_scope(*read_scope, source_binder, &loans)?;
                }
                _ => {}
            }
            if let ExpressionOperation::Let { bindings, body } = &record.operation {
                let mut completed = self.loan_copy(&loans)?;
                self.reserve::<(ExpressionId, BTreeSet<BindingId>, Option<BindingId>)>(
                    bindings.len(),
                )?;
                let mut initializers = Vec::new();
                for binding in bindings {
                    self.tick()?;
                    let Some(OwnerRecord::Binding(binding_record)) = self
                        .read
                        .owner(info.key.owner.package, OwnerKey::Binding(*binding))?
                    else {
                        return Err(associated::invalid());
                    };
                    if binding_record.header.owner != OwnerKey::Binding(*binding)
                        || !matches!(
                            binding_record.kind,
                            BindingKind::Let | BindingKind::ReadView
                        )
                    {
                        return Err(associated::invalid());
                    }
                    let value = binding_record.value.ok_or_else(associated::invalid)?;
                    let mut initializer_loans = self.loan_copy(&completed)?;
                    if binding_record.kind == BindingKind::ReadView {
                        if record.contract_version
                            != crate::platform::canonical_schema::SCHEMA_EPOCH
                            || binding_record.header.contract_version
                                != crate::platform::canonical_schema::SCHEMA_EPOCH
                        {
                            return Err(associated::invalid());
                        }
                        let ty = binding_record
                            .declared_type
                            .ok_or_else(associated::invalid)?;
                        let object = self
                            .read
                            .type_object(info.key.owner.package, ty)?
                            .ok_or_else(associated::invalid)?;
                        let declared_scope =
                            object.form.read_scope().ok_or_else(associated::invalid)?;
                        if declared_scope
                            == (ReadScopeArgument::Lexical {
                                callable: info.key.owner,
                                binding: *binding,
                            })
                        {
                            self.reserve::<BindingId>(1)?;
                            initializer_loans.insert(*binding);
                        }
                        self.signature_type(source, ty, source_binder, true, &initializer_loans)?;
                        self.reserve::<BindingId>(1)?;
                        completed.insert(*binding);
                    } else if let Some(ty) = binding_record.declared_type
                        && self.type_contains_view(info.key.owner.package, ty)?
                    {
                        return Err(semantic(
                            "kernel_callable_flow_category",
                            "ordinary let annotations cannot hide a ReadView",
                        ));
                    }
                    // Only a prospective source scope is available during initialization.
                    // This walk grants no readable local value before initializer completion.
                    initializers.push((value, initializer_loans, None));
                }
                self.reserve::<(ExpressionId, BTreeSet<BindingId>, Option<BindingId>)>(1)?;
                pending.push((*body, completed, None));
                pending.extend(initializers.into_iter().rev());
                continue;
            }
            let children = record.children();
            self.reserve::<(ExpressionId, BTreeSet<BindingId>, Option<BindingId>)>(children.len())?;
            for child in children.into_iter().rev() {
                let mut child_loans = self.loan_copy(&loans)?;
                let mut establishing = None;
                match &record.operation {
                    ExpressionOperation::BorrowCall {
                        call,
                        binding,
                        body,
                    } => {
                        if child.expression == *body {
                            self.reserve::<BindingId>(1)?;
                            child_loans.insert(*binding);
                        } else if child.expression == *call {
                            establishing = Some(*binding);
                        }
                    }
                    ExpressionOperation::BorrowOwnedField { binding, body, .. }
                    | ExpressionOperation::BorrowOwnedRecordField { binding, body, .. }
                    | ExpressionOperation::BorrowOwnedItem { binding, body, .. }
                    | ExpressionOperation::BorrowReadView { binding, body, .. }
                    | ExpressionOperation::BorrowReadViewField { binding, body, .. }
                        if child.expression == *body =>
                    {
                        self.reserve::<BindingId>(1)?;
                        child_loans.insert(*binding);
                    }
                    ExpressionOperation::MatchBorrowedOwned { arms, .. }
                        if child.role
                            == crate::platform::kernel::ExpressionChildRole::OwnedChoiceArmBody =>
                    {
                        let ordinal =
                            usize::try_from(child.ordinal).map_err(|_| associated::invalid())?;
                        let arm = arms.get(ordinal).ok_or_else(associated::invalid)?;
                        if arm.body != child.expression {
                            return Err(associated::invalid());
                        }
                        self.reserve::<BindingId>(1)?;
                        child_loans.insert(arm.binding);
                    }
                    _ => {}
                }
                pending.push((child.expression, child_loans, establishing));
            }
        }
        Ok(())
    }
    fn admit(&mut self) -> Result<(), Diagnostic> {
        let count = self.slot_owners.len();
        self.reserve_product::<Vec<usize>>(count, 2)?;
        self.reserve_product::<usize>(count, 10)?;
        self.reserve_product::<usize>(self.edges.len(), 2)?;
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for i in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[i];
            outgoing[edge.from].push(edge.to);
            incoming[edge.to].push(edge.from);
        }
        let mut seen = vec![false; count];
        let mut finish = Vec::new();
        for root in 0..count {
            self.tick()?;
            if seen[root] {
                continue;
            }
            seen[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, next)) = stack.last_mut() {
                self.tick()?;
                if let Some(target) = outgoing[*node].get(*next) {
                    *next += 1;
                    if !seen[*target] {
                        seen[*target] = true;
                        stack.push((*target, 0));
                    }
                } else {
                    finish.push(*node);
                    stack.pop();
                }
            }
        }
        let mut components = vec![usize::MAX; count];
        for root in finish.into_iter().rev() {
            self.tick()?;
            if components[root] != usize::MAX {
                continue;
            }
            components[root] = root;
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                self.tick()?;
                for source in &incoming[node] {
                    self.tick()?;
                    if components[*source] == usize::MAX {
                        components[*source] = root;
                        pending.push(*source);
                    }
                }
            }
        }
        for i in 0..self.edges.len() {
            self.tick()?;
            let edge = &self.edges[i];
            if edge.expanding && components[edge.from] == components[edge.to] {
                let (from_context, from_ordinal) = self.slot_owners[edge.from];
                let (to_context, to_ordinal) = self.slot_owners[edge.to];
                let from = &self.contexts[from_context].key;
                let to = &self.contexts[to_context].key;
                return Err(semantic(
                    "kernel_callable_expansion",
                    format!(
                        "expanding composed callable parameter cycle: {}::{} slot {} -> {}::{} slot {}; owner {}::{} expression {:?} argument {}; constructor path {:?}; same strongly connected component; forward parameters without nesting or reset to a closed type",
                        from.owner.package,
                        from.owner.declaration,
                        from_ordinal,
                        to.owner.package,
                        to.owner.declaration,
                        to_ordinal,
                        edge.origin.package,
                        edge.origin.declaration,
                        edge.expression,
                        edge.argument,
                        &edge.path[..edge.path.len().min(MAXIMUM_DIAGNOSTIC_PATH)]
                    ),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_callable_closure<R: CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut ValidationWork,
    maximum_work: Option<usize>,
) -> Result<CallableFlowWork, Diagnostic> {
    validate_callable_closure_with_scope(read, work, maximum_work, recursive::ProofScope::Recursive)
}

fn validate_callable_closure_with_scope<R: CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut ValidationWork,
    maximum_work: Option<usize>,
    scope: recursive::ProofScope,
) -> Result<CallableFlowWork, Diagnostic> {
    validate_callable_closure_with_limits(read, work, maximum_work, None, scope)
}

fn validate_callable_closure_with_limits<R: CallableClosureRead + ?Sized>(
    read: &R,
    work: &mut ValidationWork,
    maximum_work: Option<usize>,
    maximum_metadata_bytes: Option<usize>,
    scope: recursive::ProofScope,
) -> Result<CallableFlowWork, Diagnostic> {
    // The caller may enter with work already spent admitting exact supplier sources.
    // Retain only scalar phase boundaries; this is error context, never proof reuse.
    let entry_work = work.consumed;
    let mut after_prerequisites = None;
    let mut after_inventory = None;
    let mut after_scan = None;
    let mut after_recursive = None;
    let result = (|| {
        let mut analysis = Analysis {
            read,
            work: &mut *work,
            maximum_work,
            maximum_metadata_bytes,
            observation: CallableFlowWork::default(),
            contexts: Vec::new(),
            indexes: BTreeMap::new(),
            selections: Vec::new(),
            selection_indexes: BTreeMap::new(),
            pending: Vec::new(),
            calls: Vec::new(),
            slot_owners: Vec::new(),
            edges: Vec::new(),
            associated: associated::State::default(),
            completed_view_queries: BTreeMap::new(),
            diagnostic_context: WorkContext {
                phase: "initialization",
                owner: None,
                context: None,
                expression: None,
                query: None,
            },
        };
        analysis.tick()?;
        analysis.diagnostic_context.phase = "prerequisite closure";
        prerequisites::validate(&mut analysis)?;
        after_prerequisites = Some(analysis.work.consumed);
        read.visit_callable_owners(&mut |owner| {
            analysis.diagnostic_context = WorkContext {
                phase: "declaration inventory",
                owner: Some(owner),
                context: None,
                expression: None,
                query: None,
            };
            let declaration = analysis.declaration(owner)?;
            analysis.declaration_scope_inventory(owner, &declaration.payload)?;
            match declaration.payload {
                DeclarationPayload::OwnedImplementation(i) => {
                    analysis.validate_associated_scheme(owner, &i)?;
                    analysis.reserve::<SelectionId>(i.implementation_parameters.len())?;
                    let unknown = analysis.unknown_parameters(&i.implementation_parameters)?;
                    for mapping in i.methods {
                        analysis.reserve::<SelectionId>(i.implementation_parameters.len())?;
                        analysis.context(Context {
                            owner,
                            method: Some(mapping.method),
                            implementations: unknown.clone(),
                        })?;
                    }
                }
                DeclarationPayload::Function(f) => {
                    analysis.reserve::<SelectionId>(f.implementation_parameters.len())?;
                    let unknown = analysis.unknown_parameters(&f.implementation_parameters)?;
                    analysis.context(Context {
                        owner,
                        method: None,
                        implementations: unknown,
                    })?;
                }
                DeclarationPayload::OwnedContract(c) => {
                    analysis.associated.enabled |= !c.associated_types.is_empty();
                    analysis.context(Context {
                        owner,
                        method: None,
                        implementations: Vec::new(),
                    })?;
                }
                _ => {
                    analysis.context(Context {
                        owner,
                        method: None,
                        implementations: Vec::new(),
                    })?;
                }
            }
            Ok(())
        })?;
        after_inventory = Some(analysis.work.consumed);
        while let Some(context) = analysis.pending.pop() {
            analysis.scan(context)?;
        }
        after_scan = Some(analysis.work.consumed);
        analysis.diagnostic_context = WorkContext {
            phase: "recursive callable relation",
            owner: None,
            context: None,
            expression: None,
            query: None,
        };
        recursive::connect(&mut analysis, scope)?;
        after_recursive = Some(analysis.work.consumed);
        analysis.diagnostic_context.phase = "composed type SCC";
        analysis.admit()?;
        analysis.observation.slots = analysis.slot_owners.len();
        analysis.observation.edges = analysis.edges.len();
        analysis.observation.work_saturated = analysis.work.saturated;
        Ok(analysis.observation)
    })();
    result.map_err(|mut diagnostic: Diagnostic| {
        if diagnostic.code == "kernel_callable_flow_work" {
            diagnostic.notes.push(format!(
                "composed_work entry={} after_prerequisites={:?} after_inventory={:?} after_scan={:?} after_recursive={:?} consumed={}",
                entry_work, after_prerequisites, after_inventory, after_scan, after_recursive, work.consumed,
            ));
        }
        diagnostic
    })
}

#[cfg(test)]
#[path = "composed_callable_flow_tests.rs"]
mod tests;

#[cfg(test)]
mod read_view_tests {
    use super::super::read_view_tests::{Read, function, scope};
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::{
        DeclarationId, ModuleId, ParameterId, ReadScopeParameterId,
    };

    const SEED: &[u8] = b"composed-read-view-scopes";

    impl CallableClosureRead for Read {
        fn visit_callable_owners(
            &self,
            visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
        ) -> Result<(), Diagnostic> {
            for owner in self.owners.keys() {
                if let OwnerKey::Declaration(declaration) = owner {
                    visitor(DeclarationReference {
                        package: function().package,
                        declaration: *declaration,
                    })?;
                }
            }
            Ok(())
        }
        fn owner(
            &self,
            package: PackageId,
            owner: OwnerKey,
        ) -> Result<Option<OwnerRecord>, Diagnostic> {
            Ok((package == function().package)
                .then(|| self.owners.get(&owner).cloned())
                .flatten())
        }
        fn type_object(
            &self,
            package: PackageId,
            ty: TypeObjectDigest,
        ) -> Result<Option<TypeObject>, Diagnostic> {
            Ok((package == function().package)
                .then(|| self.types.get(ty).cloned())
                .flatten())
        }
    }

    fn proof(read: &Read, eager: bool) -> Result<CallableFlowWork, Diagnostic> {
        test_validate_callable_closure_with_scope(
            read,
            &mut 0,
            Some(1_000_000),
            if eager {
                recursive::ProofScope::All
            } else {
                recursive::ProofScope::Recursive
            },
        )
    }

    fn assert_foreign_binding_epoch_rejected(read: &mut Read, binding: BindingId) {
        let key = OwnerKey::Binding(binding);
        let original = read.owners[&key].clone();
        let Some(OwnerRecord::Binding(record)) = read.owners.get_mut(&key) else {
            panic!("scoped binding");
        };
        record.header.contract_version = crate::platform::canonical_schema::SCHEMA_EPOCH + 1;
        for eager in [false, true] {
            assert_eq!(
                proof(read, eager)
                    .expect_err("a foreign binding epoch grants no lexical scope")
                    .code,
                "kernel_callable_associated_scope"
            );
        }
        read.owners.insert(key, original);
    }

    fn reference(ordinal: u64) -> DeclarationReference {
        DeclarationReference {
            package: function().package,
            declaration: DeclarationId::migrate(SEED, ordinal),
        }
    }

    fn formal(owner: DeclarationReference) -> ReadScopeArgument {
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(owner),
            parameter: ReadScopeParameterId::migrate(SEED, 0),
        })
    }

    fn payload(read: &mut Read) -> &mut FunctionDeclaration {
        let Some(OwnerRecord::Declaration(record)) = read
            .owners
            .get_mut(&OwnerKey::Declaration(function().declaration))
        else {
            panic!("caller");
        };
        let DeclarationPayload::Function(function) = &mut record.payload else {
            panic!("caller function");
        };
        function
    }

    // This fixture checks symbolic source closure. Custody and runtime result adoption have
    // independent admission owners; no growth result here is used as an access certificate.
    fn view_fixture() -> (Read, DeclarationReference, TypeObjectDigest, BindingId) {
        let mut read = Read::new();
        let view = reference(1);
        let helper = reference(2);
        let cell = read.types.intern(TypeForm::OwnedI64Cell).expect("cell");
        let caller_parameter = ParameterId::migrate(SEED, 0);
        let source = ParameterId::migrate(SEED, 1);
        for (id, owner) in [(caller_parameter, function()), (source, helper)] {
            let key = OwnerKey::Parameter(id);
            read.owners.insert(
                key,
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(key, OwnerKind::Parameter),
                    parent: ParameterParent::Function(owner.declaration),
                    name: Name::new("source").expect("name"),
                    ty: cell,
                    use_mode: ParameterUse::Borrow,
                    resource_requirement: None,
                }),
            );
        }
        let caller = payload(&mut read);
        caller.parameters = vec![caller_parameter];
        caller.read_scope_sources = vec![ReadScopeSource {
            source: caller_parameter,
            scope: scope(),
        }];
        let key = OwnerKey::Declaration(view.declaration);
        read.owners.insert(
            key,
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(key, OwnerKind::ReadView),
                module: ModuleId::migrate(SEED, 0),
                name: Name::new("EmptyView").expect("name"),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: Vec::new(),
                    scope_parameter: ReadScopeParameter {
                        id: ReadScopeParameterId::migrate(SEED, 0),
                        name: Name::new("s").expect("scope name"),
                    },
                    fields: Vec::new(),
                }),
            }),
        );
        let result = read
            .types
            .intern(TypeForm::ReadViewApplied {
                declaration: view,
                arguments: Vec::new(),
                read_scope: formal(helper),
            })
            .expect("helper view");
        let body = read.expression(ExpressionOperation::CreateReadView {
            declaration: view,
            type_arguments: Vec::new(),
            read_scope: formal(helper),
            source: LocalValueReference::FunctionParameter(source),
            fields: Vec::new(),
        });
        let key = OwnerKey::Declaration(helper.declaration);
        read.owners.insert(
            key,
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(key, OwnerKind::PureFunction),
                module: ModuleId::migrate(SEED, 0),
                name: Name::new("empty").expect("name"),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::Function(FunctionDeclaration {
                    implementation_parameters: Vec::new(),
                    requirement_parameters: Vec::new(),
                    effect_parameters: Vec::new(),
                    type_parameters: Vec::new(),
                    parameters: vec![source],
                    result: CallableResult {
                        ty: result,
                        relation: ResultRelation::ViewFrom {
                            source,
                            scope: formal(helper),
                        },
                    },
                    read_scope_parameters: vec![ReadScopeParameter {
                        id: ReadScopeParameterId::migrate(SEED, 0),
                        name: Name::new("s").expect("scope name"),
                    }],
                    read_scope_sources: vec![ReadScopeSource {
                        source,
                        scope: formal(helper),
                    }],
                    effect: FunctionEffect::Pure,
                    body,
                }),
            }),
        );
        let binding = BindingId::migrate(SEED, 0);
        let lexical = ReadScopeArgument::Lexical {
            callable: function(),
            binding,
        };
        let ty = read
            .types
            .intern(TypeForm::ReadViewApplied {
                declaration: view,
                arguments: Vec::new(),
                read_scope: lexical,
            })
            .expect("caller view");
        (read, helper, ty, binding)
    }

    #[test]
    fn complete_flow_keeps_scope_forwarding_separate_from_type_expansion() {
        for expanding in [false, true] {
            let mut read = Read::new();
            let actual = if expanding {
                read.types
                    .intern(TypeForm::List {
                        item: read.parameter_type,
                    })
                    .expect("list")
            } else {
                read.parameter_type
            };
            let body = read.call(0, actual, vec![scope()]);
            read.body(body);
            for eager in [false, true] {
                let result = proof(&read, eager);
                if expanding {
                    assert_eq!(
                        result
                            .expect_err("scope forwarding cannot reset growth")
                            .code,
                        "kernel_callable_expansion"
                    );
                } else {
                    result.expect("finite explicit scope forwarding");
                }
            }
        }
    }

    #[test]
    fn complete_flow_rejects_foreign_scope_and_scoped_function_value() {
        for foreign_scope in [false, true] {
            let mut read = Read::new();
            let body = if foreign_scope {
                read.call(0, read.parameter_type, vec![formal(reference(99))])
            } else {
                read.call(1, read.parameter_type, Vec::new())
            };
            read.body(body);
            for eager in [false, true] {
                assert!(proof(&read, eager).is_err());
            }
        }
    }

    #[test]
    fn complete_flow_rejects_view_actual_even_when_parameter_is_phantom() {
        let mut read = Read::new();
        let view = read
            .types
            .intern(TypeForm::ReadViewApplied {
                declaration: reference(1),
                arguments: vec![read.parameter_type],
                read_scope: scope(),
            })
            .expect("view");
        let body = read.call(0, view, vec![scope()]);
        read.body(body);
        for eager in [false, true] {
            assert_eq!(
                proof(&read, eager)
                    .expect_err("ordinary formal cannot admit a view")
                    .code,
                "kernel_callable_flow_category"
            );
        }
    }

    #[test]
    fn complete_flow_family_demands_keep_distinct_exact_scopes() {
        let mut counts = Vec::new();
        for alias in [true, false] {
            let mut read = Read::new();
            let contract = reference(3);
            let member = AssociatedTypeMemberId::migrate(SEED, 0);
            let witness = ImplementationParameterId::migrate(SEED, 0);
            let self_parameter = TypeParameterId::migrate(SEED, 99);
            let self_type = read
                .types
                .intern(TypeForm::TypeParameter {
                    parameter: self_parameter,
                })
                .expect("contract self");
            let result_type = read.types.intern(TypeForm::I64).expect("ordinary result");
            let self_owner = OwnerKey::TypeParameter(self_parameter);
            read.owners.insert(
                self_owner,
                OwnerRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(self_owner, OwnerKind::TypeParameter),
                    declaration: contract.declaration,
                    name: Name::new("Self").expect("name"),
                    constraints: TypeParameterConstraints::Owned,
                }),
            );
            let key = OwnerKey::Declaration(contract.declaration);
            read.owners.insert(
                key,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(key, OwnerKind::OwnedContract),
                    module: ModuleId::migrate(SEED, 0),
                    name: Name::new("Views").expect("name"),
                    visibility: DeclarationVisibility::Private,
                    payload: DeclarationPayload::OwnedContract(OwnedContract {
                        self_parameter,
                        type_parameters: Vec::new(),
                        methods: vec![OwnedMethod {
                            id: MethodId::migrate(SEED, 0),
                            name: Name::new("size").expect("name"),
                            parameters: vec![OwnedMethodParameter {
                                ty: self_type,
                                use_mode: ParameterUse::Borrow,
                            }],
                            result: CallableResult {
                                ty: result_type,
                                relation: ResultRelation::Value,
                            },
                            read_scope_parameters: Vec::new(),
                            read_scope_sources: Vec::new(),
                            effect: FunctionEffect::Pure,
                        }],
                        associated_types: vec![AssociatedTypeMember {
                            id: member,
                            name: Name::new("View").expect("name"),
                            bound: AssociatedTypeBound::ReadViewFamily {
                                scope_parameter: ReadScopeParameter {
                                    id: ReadScopeParameterId::migrate(SEED, 99),
                                    name: Name::new("v").expect("name"),
                                },
                            },
                        }],
                    }),
                }),
            );
            let other = ReadScopeArgument::Parameter(ReadScopeReference {
                binder: ReadScopeBinder::Declaration(function()),
                parameter: ReadScopeParameterId::migrate(SEED, 1),
            });
            let cell = read.types.intern(TypeForm::OwnedI64Cell).expect("self");
            let mut parameters = Vec::new();
            for (ordinal, read_scope) in [scope(), if alias { scope() } else { other }]
                .into_iter()
                .enumerate()
            {
                let ty = read
                    .types
                    .intern(TypeForm::AssociatedReadViewProjection {
                        scope: function(),
                        parameter: witness,
                        contract,
                        member,
                        read_scope,
                    })
                    .expect("family projection");
                let id = ParameterId::migrate(SEED, ordinal as u64);
                let key = OwnerKey::Parameter(id);
                read.owners.insert(
                    key,
                    OwnerRecord::Parameter(ParameterRecord {
                        header: OwnerHeader::new(key, OwnerKind::Parameter),
                        parent: ParameterParent::Function(function().declaration),
                        name: Name::new(format!("view{ordinal}")).expect("name"),
                        ty,
                        use_mode: ParameterUse::Borrow,
                        resource_requirement: None,
                    }),
                );
                parameters.push(id);
            }
            let function = payload(&mut read);
            function.parameters = parameters;
            function.read_scope_parameters.push(ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, 1),
                name: Name::new("other").expect("name"),
            });
            function.implementation_parameters = vec![ImplementationParameter {
                id: witness,
                name: Name::new("W").expect("name"),
                contract,
                self_type: cell,
                type_arguments: Vec::new(),
            }];
            let normal = proof(&read, false).expect("finite family demands");
            let eager = proof(&read, true).expect("eager family demands");
            assert_eq!(normal.slots, eager.slots);
            counts.push(normal.slots);
        }
        assert_eq!(
            counts[1],
            counts[0] + 1,
            "equal witness paths retain a separate output for each exact scope"
        );
    }

    #[test]
    fn complete_flow_does_not_reuse_a_call_after_its_lexical_scope_ends() {
        let (mut read, helper, ty, binding) = view_fixture();
        let argument = read.expression(ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(ParameterId::migrate(SEED, 0)),
        });
        let call = read.expression(ExpressionOperation::Call {
            requirement_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            function: helper,
            type_arguments: Vec::new(),
            read_scope_arguments: vec![ReadScopeArgument::Lexical {
                callable: function(),
                binding,
            }],
            arguments: vec![argument],
        });
        let key = OwnerKey::Binding(binding);
        read.owners.insert(
            key,
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(key, OwnerKind::Binding),
                name: Name::new("view").expect("name"),
                kind: BindingKind::ReadView,
                value: None,
                declared_type: Some(ty),
            }),
        );
        let seven = read.expression(ExpressionOperation::I64 { value: 7 });
        let borrow = read.expression(ExpressionOperation::BorrowCall {
            call,
            binding,
            body: seven,
        });
        read.body(borrow);
        for eager in [false, true] {
            proof(&read, eager).expect("direct invocation metadata establishes its source scope");
        }
        assert_foreign_binding_epoch_rejected(&mut read, binding);
        let body = read.expression(ExpressionOperation::Sequence {
            items: vec![borrow, call],
        });
        read.body(body);
        for eager in [false, true] {
            assert_eq!(
                proof(&read, eager)
                    .expect_err("same syntax outside loan cannot reuse its admission")
                    .code,
                "kernel_callable_associated_scope"
            );
        }

        // An OwnedBorrow is a valid loan coordinate only in its exact match arm.
        let (mut read, helper, _, binding) = view_fixture();
        let parameter = ParameterId::migrate(SEED, 0);
        let cell = match &read.owners[&OwnerKey::Parameter(parameter)] {
            OwnerRecord::Parameter(record) => record.ty,
            _ => panic!("source parameter"),
        };
        let choice = read
            .types
            .intern(TypeForm::OwnedChoice {
                cases: vec![StructuralTypeField {
                    name: Name::new("item").expect("case name"),
                    ty: cell,
                }],
            })
            .expect("owned choice");
        let Some(OwnerRecord::Parameter(record)) =
            read.owners.get_mut(&OwnerKey::Parameter(parameter))
        else {
            panic!("source parameter");
        };
        record.ty = choice;
        let arm_binding = BindingId::migrate(SEED, 10);
        let scope = ReadScopeArgument::Lexical {
            callable: function(),
            binding: arm_binding,
        };
        let key = OwnerKey::Binding(arm_binding);
        read.owners.insert(
            key,
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(key, OwnerKind::Binding),
                name: Name::new("item").expect("binding name"),
                kind: BindingKind::OwnedBorrow,
                value: None,
                declared_type: Some(cell),
            }),
        );
        let ty = read
            .types
            .intern(TypeForm::ReadViewApplied {
                declaration: reference(1),
                arguments: Vec::new(),
                read_scope: scope,
            })
            .expect("arm view");
        let key = OwnerKey::Binding(binding);
        read.owners.insert(
            key,
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(key, OwnerKind::Binding),
                name: Name::new("view").expect("binding name"),
                kind: BindingKind::ReadView,
                value: None,
                declared_type: Some(ty),
            }),
        );
        let argument = read.expression(ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(arm_binding),
        });
        let call = read.expression(ExpressionOperation::Call {
            requirement_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            function: helper,
            type_arguments: Vec::new(),
            read_scope_arguments: vec![scope],
            arguments: vec![argument],
        });
        let seven = read.expression(ExpressionOperation::I64 { value: 7 });
        let body = read.expression(ExpressionOperation::BorrowCall {
            call,
            binding,
            body: seven,
        });
        let source = read.expression(ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        });
        let matched = read.expression(ExpressionOperation::MatchBorrowedOwned {
            choice_type: choice,
            source,
            arms: vec![OwnedChoiceArm {
                name: Name::new("item").expect("case name"),
                binding: arm_binding,
                body,
            }],
        });
        read.body(matched);
        for eager in [false, true] {
            proof(&read, eager).expect("exact loan in its consuming match arm");
        }
        assert_foreign_binding_epoch_rejected(&mut read, arm_binding);
        let body = read.expression(ExpressionOperation::Sequence {
            items: vec![matched, call],
        });
        read.body(body);
        for eager in [false, true] {
            assert_eq!(
                proof(&read, eager)
                    .expect_err("an arm loan cannot become ambient after its body")
                    .code,
                "kernel_callable_associated_scope"
            );
        }
    }

    #[test]
    fn complete_flow_visits_owning_view_initializer_with_exact_prospective_scope() {
        let (mut read, _, ty, binding) = view_fixture();
        let value = read.expression(ExpressionOperation::CreateReadView {
            declaration: reference(1),
            type_arguments: Vec::new(),
            read_scope: ReadScopeArgument::Lexical {
                callable: function(),
                binding,
            },
            source: LocalValueReference::FunctionParameter(ParameterId::migrate(SEED, 0)),
            fields: Vec::new(),
        });
        let key = OwnerKey::Binding(binding);
        read.owners.insert(
            key,
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(key, OwnerKind::Binding),
                name: Name::new("view").expect("name"),
                kind: BindingKind::ReadView,
                value: Some(value),
                declared_type: Some(ty),
            }),
        );
        let seven = read.expression(ExpressionOperation::I64 { value: 7 });
        let body = read.expression(ExpressionOperation::Let {
            bindings: vec![binding],
            body: seven,
        });
        read.body(body);
        for eager in [false, true] {
            proof(&read, eager).expect("owning initializer exact source scope");
        }
        assert_foreign_binding_epoch_rejected(&mut read, binding);
        let body = read.expression(ExpressionOperation::Sequence {
            items: vec![body, value],
        });
        read.body(body);
        for eager in [false, true] {
            assert_eq!(
                proof(&read, eager)
                    .expect_err("initializer scope cannot become ambient")
                    .code,
                "kernel_callable_associated_scope"
            );
        }
    }

    #[test]
    fn complete_flow_checks_unused_view_result_scope_without_type_actuals() {
        let (mut read, helper, _, _) = view_fixture();
        let Some(OwnerRecord::Declaration(declaration)) = read
            .owners
            .get_mut(&OwnerKey::Declaration(helper.declaration))
        else {
            panic!("helper");
        };
        let DeclarationPayload::Function(function) = &mut declaration.payload else {
            panic!("helper function");
        };
        let parameter = ReadScopeParameterId::migrate(SEED, 1);
        function.read_scope_parameters.push(ReadScopeParameter {
            id: parameter,
            name: Name::new("other").expect("name"),
        });
        let ResultRelation::ViewFrom { scope, .. } = &mut function.result.relation else {
            panic!("view result");
        };
        *scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(helper),
            parameter,
        });
        for eager in [false, true] {
            assert_eq!(
                proof(&read, eager)
                    .expect_err("unused result relation cannot swap its exact scope")
                    .code,
                "kernel_callable_associated_scope"
            );
        }
    }
}
