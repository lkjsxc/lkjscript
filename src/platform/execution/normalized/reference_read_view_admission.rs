//! Raw-canonical admission for the reference interpreter's source-tied views.
//!
//! This owner reads records, rather than compiler indexes, property certificates,
//! or production view admission. Symbolic scope identity and accessible reference
//! closure are proved separately from the invocation's live custody protocol.

use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    self, AssociatedTypeBindingValue, AssociatedTypeBound, BindingKind, CallableResult,
    DeclarationPayload, DeclarationRecord, DeclarationReference, ExpressionOperation,
    FunctionDeclaration, FunctionEffect, ImplementationOperand, KernelSnapshot,
    LocalValueReference, OwnerKey, OwnerKind, OwnerRecord, PackageId, ParameterParent,
    ParameterUse, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference,
    ReadScopeSource, ReadViewDeclaration, ReadViewFieldKind, ResultRelation, StructuralTypeField,
    TypeForm, TypeObject, TypeObjectDigest, TypeParameterConstraints, ValueCategory,
    encode_type_object,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, ExpressionId, FieldId, ImplementationParameterId,
    TypeParameterId,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use crate::platform::kernel::Name;

type Result<T> = std::result::Result<T, ExecutionError>;

fn invalid(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_read_view_admission",
        message,
    )
}

/// Complete independently instantiated runtime layout. The shared `Proof::types`
/// owns its full type closure. Raw test queries additionally expose the selected
/// digest, authored names, and freshly derived objects for independent controls.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    #[cfg(test)]
    pub ty: TypeObjectDigest,
    pub declaration: DeclarationReference,
    pub arguments: Vec<TypeObjectDigest>,
    pub read_scope: ReadScopeArgument,
    pub fields: Vec<Field>,
    #[cfg(test)]
    pub types: BTreeMap<TypeObjectDigest, TypeObject>,
}

#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub id: FieldId,
    #[cfg(test)]
    pub name: Name,
    pub ty: TypeObjectDigest,
    pub kind: ReadViewFieldKind,
}

#[derive(Clone, Debug)]
pub(crate) struct Proof {
    owners: BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    scopes: BTreeMap<ReadScopeBinder, Vec<ReadScopeParameter>>,
    sources: BTreeMap<ReadScopeBinder, Vec<ReadScopeSource<u32>>>,
    dependencies: BTreeMap<PackageId, BTreeSet<PackageId>>,
    lexical_owners: BTreeMap<(PackageId, BindingId), DeclarationReference>,
    view_functions: BTreeSet<DeclarationReference>,
    schemas: BTreeMap<DeclarationReference, Schema>,
    prepared_layouts: BTreeMap<TypeObjectDigest, Layout>,
    prepared_facts: BTreeMap<TypeObjectDigest, Facts>,
    #[cfg(test)]
    control: ExecutionControl,
}

/// Bounds finite proof preparation, not execution or termination.
struct Budget<'a> {
    control: &'a ExecutionControl,
    remaining: usize,
    entries: usize,
}

impl<'a> Budget<'a> {
    fn new(control: &'a ExecutionControl) -> Self {
        Self {
            control,
            remaining: kernel::contract::MAXIMUM_VALIDATION_WORK,
            entries: 0,
        }
    }
    fn step(&mut self, depth: usize) -> Result<()> {
        self.control.check()?;
        if depth > kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(invalid(
                "canonical type/layout exceeds the semantic depth boundary",
            ));
        }
        self.remaining = self.remaining.checked_sub(1).ok_or_else(|| {
            ExecutionError::resource(
                "reference_read_view_proof_work",
                "finite canonical view proof capacity exhausted",
            )
        })?;
        Ok(())
    }
    fn reserve(&mut self, entries: usize) -> Result<()> {
        self.control.check()?;
        self.entries = self
            .entries
            .checked_add(entries)
            .filter(|n| *n <= kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "reference_read_view_proof_storage",
                    "finite canonical view proof storage exhausted",
                )
            })?;
        Ok(())
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Context {
    owner: Option<DeclarationReference>,
    /// Property expansion is separate from an authored occurrence's imports.
    structural_properties: bool,
    binder: Option<ReadScopeBinder>,
    /// Activation identity is separate from the component's lexical owner.
    /// This scalar imports no named-function or view-scope authority.
    port: Option<kernel::PortReference>,
    lexical: BTreeSet<BindingId>,
}

impl Context {
    fn declaration(owner: DeclarationReference) -> Self {
        Self {
            owner: Some(owner),
            structural_properties: false,
            binder: Some(ReadScopeBinder::Declaration(owner)),
            port: None,
            lexical: BTreeSet::new(),
        }
    }
    fn binder(binder: ReadScopeBinder) -> Self {
        Self {
            owner: Some(binder.declaration()),
            structural_properties: false,
            binder: Some(binder),
            port: None,
            lexical: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Facts {
    category: ValueCategory,
    metadata: bool,
    transfer: bool,
    share: bool,
}

type FactsKey = (TypeObjectDigest, Context);

/// Private per-root conjunction estimates. Inner estimates never become prepared
/// facts: every reachable sibling and every bound must pass at convergence.
/// Owner, binder and lexical actual authority stay fixed. Independently checked
/// physical field edges use a separate structural-property key, never a naming grant.
#[derive(Default)]
struct FactsState {
    active: BTreeSet<FactsKey>,
    estimates: BTreeMap<FactsKey, Facts>,
    observed: BTreeMap<FactsKey, Facts>,
    metadata: BTreeSet<FactsKey>,
    transfer: BTreeSet<FactsKey>,
    share: BTreeSet<FactsKey>,
}

#[derive(Clone, Debug)]
struct Schema {
    category: ValueCategory,
    edges: BTreeSet<DeclarationReference>,
}

impl Facts {
    fn ordinary() -> Self {
        Self {
            category: ValueCategory::OrdinaryData,
            metadata: true,
            transfer: true,
            share: true,
        }
    }
    fn owned(transfer: bool, share: bool) -> Self {
        Self {
            category: ValueCategory::Owned,
            metadata: false,
            transfer,
            share,
        }
    }
}

#[derive(Clone, Default)]
struct Substitution {
    types: BTreeMap<TypeParameterId, TypeObjectDigest>,
    effects: BTreeMap<kernel::EffectParameterReference, kernel::EffectRow>,
    requirements: kernel::RequirementSubstitution,
    scopes: BTreeMap<ReadScopeReference, ReadScopeArgument>,
    members: BTreeMap<
        (DeclarationReference, AssociatedTypeMemberId),
        (TypeObjectDigest, Option<ReadScopeReference>),
    >,
    witnesses: BTreeMap<(DeclarationReference, ImplementationParameterId), ImplementationOperand>,
}

#[derive(Clone, Copy, PartialEq)]
enum SignatureStage {
    Effects,
    Types,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Local {
    ty: TypeObjectDigest,
    source: LocalValueReference,
    scopes: BTreeSet<ReadScopeArgument>,
    read: bool,
    live: bool,
}
type Locals = BTreeMap<LocalValueReference, Local>;

#[derive(Clone, Debug, Default)]
struct Value {
    ty: Option<TypeObjectDigest>,
    source: Option<LocalValueReference>,
    scopes: BTreeSet<ReadScopeArgument>,
    read: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Use {
    Value,
    Borrow,
    Consume,
    Return,
    Reference,
}

#[derive(Default)]
struct Syntax {
    active: BTreeSet<(PackageId, ExpressionId)>,
    reached: BTreeSet<(PackageId, ExpressionId)>,
    bindings: BTreeSet<(PackageId, BindingId)>,
    lexical_owners: BTreeMap<(PackageId, BindingId), DeclarationReference>,
    view_functions: BTreeSet<DeclarationReference>,
    joined_calls: BTreeSet<(PackageId, ExpressionId)>,
    pending_call: Option<(PackageId, ExpressionId, BindingId)>,
}

struct Resolver<'a, 'control> {
    proof: &'a Proof,
    input: &'a BTreeMap<TypeObjectDigest, TypeObject>,
    derived: BTreeMap<TypeObjectDigest, TypeObject>,
    discovered: Vec<TypeObjectDigest>,
    budget: Budget<'control>,
}

impl Proof {
    #[cfg(test)]
    pub(crate) fn reconstruct(
        snapshots: &[&KernelSnapshot],
        control: &ExecutionControl,
    ) -> Result<Self> {
        let (mut proof, budget) = Self::reconstruct_raw(snapshots, control, Budget::new(control))?;
        proof.prepare_complete_closure(budget)?;
        Ok(proof)
    }

    fn reconstruct_raw<'control>(
        snapshots: &[&KernelSnapshot],
        control: &'control ExecutionControl,
        mut budget: Budget<'control>,
    ) -> Result<(Self, Budget<'control>)> {
        control.check()?;
        let mut proof = Self {
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            scopes: BTreeMap::new(),
            sources: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            lexical_owners: BTreeMap::new(),
            view_functions: BTreeSet::new(),
            schemas: BTreeMap::new(),
            prepared_layouts: BTreeMap::new(),
            prepared_facts: BTreeMap::new(),
            #[cfg(test)]
            control: control.clone(),
        };
        for snapshot in snapshots {
            budget.step(0)?;
            if snapshot.root.graph_contract_version
                < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
                && (snapshot
                    .types
                    .values()
                    .any(|object| object.form.is_read_view())
                    || snapshot.owners.values().any(raw_view_metadata))
            {
                return Err(invalid(
                    "containing canonical revision advertises a predecessor view profile",
                ));
            }
            if proof.dependencies.contains_key(&snapshot.root.package_id) {
                return Err(invalid("duplicate canonical package snapshot"));
            }
            budget.reserve(
                snapshot
                    .owners
                    .len()
                    .checked_add(snapshot.types.len())
                    .and_then(|n| n.checked_add(snapshot.dependency_types.len()))
                    .ok_or_else(|| {
                        ExecutionError::resource(
                            "reference_read_view_proof_storage",
                            "inventory overflow",
                        )
                    })?,
            )?;
            proof.dependencies.insert(
                snapshot.root.package_id,
                snapshot.dependencies.keys().copied().collect(),
            );
            for (key, record) in &snapshot.owners {
                budget.step(0)?;
                if raw_view_metadata(record)
                    && record.header().contract_version
                        < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
                {
                    return Err(invalid(
                        "new view metadata or binding uses a predecessor owner generation",
                    ));
                }
                if record.header().owner != *key {
                    return Err(invalid("raw owner key does not equal its record identity"));
                }
                proof
                    .owners
                    .insert((snapshot.root.package_id, *key), record.clone());
            }
            for (ty, object) in snapshot.types.iter().chain(&snapshot.dependency_types) {
                budget.step(0)?;
                if encode_type_object(object)
                    .map_err(|_| invalid("noncanonical type object"))?
                    .0
                    != *ty
                {
                    return Err(invalid("raw type digest differs from its canonical object"));
                }
                if let Some(previous) = proof.types.insert(*ty, object.clone())
                    && previous != *object
                {
                    return Err(invalid(
                        "conflicting canonical objects for one type identity",
                    ));
                }
            }
        }
        proof.reconstruct_scopes(&mut budget)?;
        let (schemas, next_budget) = {
            let mut resolver = proof.resolver_with_budget(&proof.types, budget);
            let schemas = resolver.schema_graph()?;
            resolver.check_schema_graph(&schemas)?;
            (schemas, resolver.budget)
        };
        proof.schemas = schemas;
        budget = next_budget;
        let (lexical_owners, view_functions, budget) = {
            let mut resolver = proof.resolver_with_budget(&proof.types, budget);
            // Every stored object has complete syntactic closure, even if no
            // callable or visible field currently selects it.
            for ty in proof.types.keys() {
                resolver.raw_closure(*ty, 0)?;
            }
            resolver.admit_inventory()?;
            let (lexical_owners, view_functions) = resolver.admit_all_expressions()?;
            (lexical_owners, view_functions, resolver.budget)
        };
        proof.lexical_owners = lexical_owners;
        proof.view_functions = view_functions;
        Ok((proof, budget))
    }

    /// Additional raw objects are admitted structurally in their exact symbolic
    /// context. A prepared producer's property tables are never proof inputs.
    pub(crate) fn reconstruct_with_types(
        snapshots: &[&KernelSnapshot],
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        control: &ExecutionControl,
    ) -> Result<Self> {
        let (mut proof, mut budget) =
            Self::reconstruct_raw(snapshots, control, Budget::new(control))?;
        budget.reserve(types.len())?;
        for (ty, object) in types {
            budget.step(0)?;
            if encode_type_object(object)
                .map_err(|_| invalid("derived raw type object has invalid syntax"))?
                .0
                != *ty
            {
                return Err(invalid(
                    "derived raw type object changes its canonical identity",
                ));
            }
            if let Some(existing) = proof.types.insert(*ty, object.clone())
                && existing != *object
            {
                return Err(invalid(
                    "derived raw type conflicts with source canonical object",
                ));
            }
        }
        proof.prepare_complete_closure(budget)?;
        Ok(proof)
    }

    /// The single preparation budget follows both raw admission and every new
    /// independently substituted field object. No cache entry is visible until
    /// that entire finite closure has completed successfully.
    fn prepare_complete_closure(&mut self, mut budget: Budget<'_>) -> Result<()> {
        budget.reserve(self.types.len().checked_mul(3).ok_or_else(|| {
            ExecutionError::resource(
                "reference_read_view_proof_storage",
                "prepared inventory overflow",
            )
        })?)?;
        let mut pending = Vec::new();
        pending.try_reserve_exact(self.types.len()).map_err(|_| {
            ExecutionError::resource(
                "reference_read_view_proof_storage",
                "cannot reserve prepared type worklist",
            )
        })?;
        pending.extend(self.types.keys().copied());
        let mut scheduled: BTreeSet<_> = self.types.keys().copied().collect();
        let mut checked = BTreeSet::new();
        let mut facts = BTreeMap::new();
        let mut layouts = BTreeMap::new();
        let derived = {
            let mut resolver = self.resolver_with_budget(&self.types, budget);
            while let Some(ty) = pending.pop() {
                resolver.budget.step(0)?;
                if !checked.insert(ty) {
                    continue;
                }
                resolver.raw_closure(ty, 0)?;
                let context = resolver.property_context_for(ty)?;
                let independent = resolver.facts(ty, &context, 0)?;
                resolver.budget.reserve(1)?;
                facts.insert(ty, independent);
                if matches!(resolver.object(ty)?.form, TypeForm::ReadViewApplied { .. }) {
                    let layout = resolver.layout(ty, &context, 0)?;
                    resolver.budget.reserve(1)?;
                    layouts.insert(ty, layout);
                }
                for discovered in std::mem::take(&mut resolver.discovered) {
                    resolver.budget.step(0)?;
                    if !scheduled.contains(&discovered) {
                        resolver.budget.reserve(3)?;
                        pending.try_reserve(1).map_err(|_| {
                            ExecutionError::resource(
                                "reference_read_view_proof_storage",
                                "cannot reserve derived field worklist",
                            )
                        })?;
                        scheduled.insert(discovered);
                        pending.push(discovered);
                    }
                }
            }
            resolver.budget.reserve(resolver.derived.len())?;
            resolver.derived
        };
        self.types.extend(derived);
        self.prepared_facts = facts;
        self.prepared_layouts = layouts;
        Ok(())
    }

    /// Successful lookups borrow immutable preparation output and allocate no
    /// types, scope maps, fields, layout copies, or proof work at runtime.
    pub(crate) fn prepared_layout(&self, ty: TypeObjectDigest) -> Result<&Layout> {
        self.prepared_layouts
            .get(&ty)
            .ok_or_else(|| invalid("exact nominal view layout was not independently prepared"))
    }
    pub(crate) fn prepared_category(&self, ty: TypeObjectDigest) -> Result<ValueCategory> {
        Ok(self.prepared_fact(ty)?.category)
    }
    pub(crate) fn prepared_metadata(&self, ty: TypeObjectDigest) -> Result<bool> {
        let facts = self.prepared_fact(ty)?;
        Ok(facts.category == ValueCategory::OrdinaryData && facts.metadata && facts.transfer)
    }
    pub(crate) fn prepared_read_shareable(&self, ty: TypeObjectDigest) -> Result<bool> {
        let facts = self.prepared_fact(ty)?;
        if facts.category != ValueCategory::ReadView {
            return Err(invalid(
                "prepared ReadShareable requires a positive view category",
            ));
        }
        Ok(facts.share)
    }
    #[cfg(test)]
    pub(crate) fn prepared_transferable(&self, ty: TypeObjectDigest) -> Result<bool> {
        Ok(self.prepared_fact(ty)?.transfer)
    }
    fn prepared_fact(&self, ty: TypeObjectDigest) -> Result<&Facts> {
        self.prepared_facts
            .get(&ty)
            .ok_or_else(|| invalid("exact type facts were not independently prepared"))
    }

    pub(crate) fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject> {
        &self.types
    }
    /// Only these exact named bodies may replace the predecessor's scope-blind
    /// borrower proof. Complete declaration and every body path passed here.
    pub(crate) fn admits_function_flow(&self, reference: DeclarationReference) -> bool {
        self.view_functions.contains(&reference)
    }
    /// Only a named graph function owns a canonical lexical read scope. Bindings
    /// traversed in constants, tests, and port expressions retain their ordinary
    /// local semantics and never inherit an enclosing runtime function's binder.
    pub(crate) fn lexical_function_for_binding(
        &self,
        package: PackageId,
        binding: BindingId,
    ) -> Option<DeclarationReference> {
        let owner = self.lexical_owners.get(&(package, binding)).copied()?;
        if owner.package != package {
            return None;
        }
        match self
            .owners
            .get(&(package, OwnerKey::Declaration(owner.declaration)))
        {
            Some(OwnerRecord::Declaration(declaration))
                if matches!(declaration.payload, DeclarationPayload::Function(_)) =>
            {
                Some(owner)
            }
            _ => None,
        }
    }
    pub(crate) fn scope_parameters(
        &self,
        binder: ReadScopeBinder,
    ) -> Result<&[ReadScopeParameter]> {
        self.scopes
            .get(&binder)
            .map(Vec::as_slice)
            .ok_or_else(|| invalid("unknown exact scope binder"))
    }
    pub(crate) fn scope_sources(&self, binder: ReadScopeBinder) -> Result<&[ReadScopeSource<u32>]> {
        self.sources
            .get(&binder)
            .map(Vec::as_slice)
            .ok_or_else(|| invalid("unknown exact source inventory"))
    }
    pub(crate) fn declaration(
        &self,
        reference: DeclarationReference,
    ) -> Result<&DeclarationRecord> {
        match self.owners.get(&(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )) {
            Some(OwnerRecord::Declaration(record)) => Ok(record),
            _ => Err(invalid("exact canonical declaration is absent")),
        }
    }
    #[cfg(test)]
    pub(crate) fn layout(&self, ty: TypeObjectDigest) -> Result<Layout> {
        self.layout_in(ty, &self.types)
    }
    #[cfg(test)]
    pub(crate) fn layout_in(
        &self,
        ty: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<Layout> {
        self.layout_in_with_control(ty, types, &self.control)
    }
    #[cfg(test)]
    pub(crate) fn layout_in_with_control(
        &self,
        ty: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        control: &ExecutionControl,
    ) -> Result<Layout> {
        let mut resolver = self.resolver_with_control(types, control);
        let context = resolver.property_context_for(ty)?;
        resolver.facts(ty, &context, 0)?;
        let mut layout = resolver.layout(ty, &context, 0)?;
        layout.types = resolver.derived;
        Ok(layout)
    }
    #[cfg(test)]
    pub(crate) fn category(&self, ty: TypeObjectDigest) -> Result<ValueCategory> {
        self.category_in(ty, &self.types)
    }
    #[cfg(test)]
    pub(crate) fn category_in(
        &self,
        ty: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<ValueCategory> {
        self.category_in_with_control(ty, types, &self.control)
    }
    #[cfg(test)]
    pub(crate) fn category_in_with_control(
        &self,
        ty: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        control: &ExecutionControl,
    ) -> Result<ValueCategory> {
        let mut resolver = self.resolver_with_control(types, control);
        let context = resolver.property_context_for(ty)?;
        Ok(resolver.facts(ty, &context, 0)?.category)
    }
    #[cfg(test)]
    pub(crate) fn read_shareable(&self, ty: TypeObjectDigest) -> Result<bool> {
        self.read_shareable_in(ty, &self.types)
    }
    #[cfg(test)]
    pub(crate) fn read_shareable_in(
        &self,
        ty: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<bool> {
        self.read_shareable_in_with_control(ty, types, &self.control)
    }
    #[cfg(test)]
    pub(crate) fn read_shareable_in_with_control(
        &self,
        ty: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        control: &ExecutionControl,
    ) -> Result<bool> {
        let mut resolver = self.resolver_with_control(types, control);
        let context = resolver.property_context_for(ty)?;
        let facts = resolver.facts(ty, &context, 0)?;
        if facts.category != ValueCategory::ReadView {
            return Err(invalid("ReadShareable requires a positive view category"));
        }
        Ok(facts.share)
    }
    #[cfg(test)]
    fn resolver_with_control<'a, 'control>(
        &'a self,
        input: &'a BTreeMap<TypeObjectDigest, TypeObject>,
        control: &'control ExecutionControl,
    ) -> Resolver<'a, 'control> {
        self.resolver_with_budget(input, Budget::new(control))
    }
    fn resolver_with_budget<'a, 'control>(
        &'a self,
        input: &'a BTreeMap<TypeObjectDigest, TypeObject>,
        budget: Budget<'control>,
    ) -> Resolver<'a, 'control> {
        Resolver {
            proof: self,
            input,
            derived: BTreeMap::new(),
            discovered: Vec::new(),
            budget,
        }
    }
    fn add_scopes(
        &mut self,
        binder: ReadScopeBinder,
        parameters: &[ReadScopeParameter],
        sources: Vec<ReadScopeSource<u32>>,
        budget: &mut Budget<'_>,
    ) -> Result<()> {
        budget.reserve(
            parameters
                .len()
                .checked_add(sources.len())
                .ok_or_else(|| invalid("scope inventory overflow"))?,
        )?;
        if parameters.len() > kernel::contract::MAXIMUM_CHILDREN
            || sources.len() > kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(invalid("scope inventory exceeds canonical bound"));
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        for parameter in parameters {
            budget.step(0)?;
            if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
                return Err(invalid("duplicate scope identity or name"));
            }
        }
        for source in &sources {
            budget.step(0)?;
            if !pairs.insert((source.source, source.scope)) {
                return Err(invalid("duplicate scope source relationship"));
            }
            match source.scope {
                ReadScopeArgument::Parameter(reference)
                    if reference.binder == binder && ids.contains(&reference.parameter) => {}
                _ => {
                    return Err(invalid(
                        "source relationship imports a foreign scope binder",
                    ));
                }
            }
        }
        if self.scopes.insert(binder, parameters.to_vec()).is_some() {
            return Err(invalid("duplicate scope binder"));
        }
        self.sources.insert(binder, sources);
        Ok(())
    }
    fn reconstruct_scopes(&mut self, budget: &mut Budget<'_>) -> Result<()> {
        let declarations: Vec<_> = self
            .owners
            .iter()
            .filter_map(|((package, key), record)| {
                if let (OwnerKey::Declaration(id), OwnerRecord::Declaration(record)) = (key, record)
                {
                    Some((
                        DeclarationReference {
                            package: *package,
                            declaration: *id,
                        },
                        record.payload.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        budget.reserve(declarations.len())?;
        for (reference, payload) in declarations {
            budget.step(0)?;
            match payload {
                DeclarationPayload::ReadView(view) => self.add_scopes(
                    ReadScopeBinder::Declaration(reference),
                    &[view.scope_parameter],
                    Vec::new(),
                    budget,
                )?,
                DeclarationPayload::Function(function) => {
                    let mut sources = Vec::new();
                    budget.reserve(function.read_scope_sources.len())?;
                    for relationship in &function.read_scope_sources {
                        let source = function
                            .parameters
                            .iter()
                            .position(|p| *p == relationship.source)
                            .ok_or_else(|| {
                                invalid(
                                    "function source is outside its ordered parameter inventory",
                                )
                            })?;
                        sources.push(ReadScopeSource {
                            source: u32::try_from(source)
                                .map_err(|_| invalid("source position overflow"))?,
                            scope: relationship.scope,
                        });
                    }
                    self.add_scopes(
                        ReadScopeBinder::Declaration(reference),
                        &function.read_scope_parameters,
                        sources,
                        budget,
                    )?;
                }
                DeclarationPayload::OwnedContract(contract) => {
                    for method in &contract.methods {
                        if method
                            .read_scope_sources
                            .iter()
                            .any(|s| s.source as usize >= method.parameters.len())
                        {
                            return Err(invalid(
                                "method source is outside its ordered parameter inventory",
                            ));
                        }
                        self.add_scopes(
                            ReadScopeBinder::ContractMethod {
                                contract: reference,
                                method: method.id,
                            },
                            &method.read_scope_parameters,
                            method.read_scope_sources.clone(),
                            budget,
                        )?;
                    }
                    for member in &contract.associated_types {
                        if let AssociatedTypeBound::ReadViewFamily { scope_parameter } =
                            &member.bound
                        {
                            self.add_scopes(
                                ReadScopeBinder::AssociatedMember {
                                    contract: reference,
                                    member: member.id,
                                },
                                std::slice::from_ref(scope_parameter),
                                Vec::new(),
                                budget,
                            )?;
                        }
                    }
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    for binding in &implementation.associated_types {
                        if let AssociatedTypeBindingValue::ReadViewFamily {
                            scope_parameter, ..
                        } = &binding.value
                        {
                            self.add_scopes(
                                ReadScopeBinder::AssociatedBinding {
                                    implementation: reference,
                                    member: binding.member,
                                },
                                std::slice::from_ref(scope_parameter),
                                Vec::new(),
                                budget,
                            )?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// Passing the raw context, locals, and complete syntax inventory explicitly
// keeps this proof independent of interpreter/compiler ambient state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
impl Resolver<'_, '_> {
    fn object(&mut self, ty: TypeObjectDigest) -> Result<TypeObject> {
        self.budget.step(0)?;
        let object = self
            .derived
            .get(&ty)
            .or_else(|| self.input.get(&ty))
            .or_else(|| self.proof.types.get(&ty))
            .ok_or_else(|| invalid("missing canonical type object"))?;
        if encode_type_object(object)
            .map_err(|_| invalid("invalid canonical type syntax"))?
            .0
            != ty
        {
            return Err(invalid(
                "type supplied to layout differs from its canonical digest",
            ));
        }
        Ok(object.clone())
    }
    fn intern(&mut self, form: TypeForm) -> Result<TypeObjectDigest> {
        self.budget.reserve(1)?;
        let object =
            TypeObject::new(form).map_err(|_| invalid("invalid independently substituted type"))?;
        let ty = encode_type_object(&object)
            .map_err(|_| invalid("cannot encode independent type substitution"))?
            .0;
        if let Some(existing) = self
            .derived
            .get(&ty)
            .or_else(|| self.input.get(&ty))
            .or_else(|| self.proof.types.get(&ty))
        {
            if existing != &object {
                return Err(invalid(
                    "independent type substitution conflicts with an exact canonical object",
                ));
            }
            return Ok(ty);
        }
        self.budget.reserve(2)?;
        self.discovered.try_reserve(1).map_err(|_| {
            ExecutionError::resource(
                "reference_read_view_proof_storage",
                "cannot reserve independent derived type inventory",
            )
        })?;
        self.derived.insert(ty, object);
        self.discovered.push(ty);
        Ok(ty)
    }
    fn owner(&mut self, package: PackageId, key: OwnerKey) -> Result<OwnerRecord> {
        self.budget.step(0)?;
        self.proof
            .owners
            .get(&(package, key))
            .cloned()
            .ok_or_else(|| invalid("missing exact canonical owner"))
    }
    fn declaration(&mut self, reference: DeclarationReference) -> Result<DeclarationRecord> {
        self.budget.step(0)?;
        self.proof.declaration(reference).cloned()
    }
    fn check_reference(&self, context: &Context, reference: DeclarationReference) -> Result<()> {
        if !context.structural_properties
            && let Some(owner) = context.owner
            && owner.package != reference.package
            && !self
                .proof
                .dependencies
                .get(&owner.package)
                .is_some_and(|d| d.contains(&reference.package))
        {
            return Err(invalid(
                "reference is outside its exact package dependency inventory",
            ));
        }
        Ok(())
    }
    fn property_context(&mut self, context: &Context) -> Result<Context> {
        self.budget.reserve(context.lexical.len())?;
        for _ in &context.lexical {
            self.budget.step(0)?;
        }
        let mut properties = context.clone();
        properties.structural_properties = true;
        Ok(properties)
    }
    fn property_context_for(&mut self, ty: TypeObjectDigest) -> Result<Context> {
        let mut context = self.context_for(ty)?;
        if let TypeForm::ReadViewApplied {
            arguments,
            read_scope: ReadScopeArgument::Parameter(_),
            ..
        } = self.object(ty)?.form
        {
            // A lifetime binder does not author the nominal carrier. Ordinary
            // symbolic actuals retain their exact independent formal owner.
            context.owner = None;
            for argument in arguments {
                let actual = self.context_for(argument)?;
                if let Some(owner) = actual.owner {
                    if context.owner.is_some_and(|previous| previous != owner) {
                        return Err(invalid(
                            "property actuals import another exact formal owner",
                        ));
                    }
                    context.owner = Some(owner);
                }
            }
        }
        self.property_context(&context)
    }
    fn scope(&mut self, actual: ReadScopeArgument, context: &Context) -> Result<()> {
        self.budget.step(0)?;
        match actual {
            ReadScopeArgument::Parameter(reference) => {
                if self
                    .proof
                    .declaration(reference.binder.declaration())?
                    .header
                    .contract_version
                    < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
                {
                    return Err(invalid(
                        "formal read scope is owned by a predecessor declaration generation",
                    ));
                }
                if context
                    .binder
                    .is_some_and(|binder| binder != reference.binder)
                {
                    return Err(invalid(
                        "scope occurrence imports another declaration's binder",
                    ));
                }
                if !self
                    .proof
                    .scope_parameters(reference.binder)?
                    .iter()
                    .any(|p| p.id == reference.parameter)
                {
                    return Err(invalid(
                        "scope formal is absent from its exact owner's inventory",
                    ));
                }
            }
            ReadScopeArgument::Lexical { callable, binding } => {
                self.named_view_function(&Context::declaration(callable))?;
                if !matches!(
                    self.proof.declaration(callable)?.payload,
                    DeclarationPayload::Function(_)
                ) {
                    return Err(invalid(
                        "canonical lexical read scope is not owned by a named graph function",
                    ));
                }
                if context.owner != Some(callable) || !context.lexical.contains(&binding) {
                    return Err(invalid(
                        "lexical scope is outside its defining callable and loan body",
                    ));
                }
                if !matches!(self.proof.owners.get(&(callable.package, OwnerKey::Binding(binding))),
                    Some(OwnerRecord::Binding(record)) if matches!(record.kind, BindingKind::OwnedBorrow | BindingKind::ReadView | BindingKind::ReadViewBorrow))
                    || self
                        .proof
                        .lexical_owners
                        .get(&(callable.package, binding))
                        .is_some_and(|owner| *owner != callable)
                {
                    return Err(invalid(
                        "lexical scope binding belongs to another callable or is absent",
                    ));
                }
            }
        }
        Ok(())
    }
    fn named_view_function(&self, context: &Context) -> Result<()> {
        let owner = context
            .owner
            .ok_or_else(|| invalid("view scope has no named function owner"))?;
        let declaration = self.proof.declaration(owner)?;
        if !matches!(declaration.payload, DeclarationPayload::Function(_))
            || declaration.header.contract_version
                < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
        {
            return Err(invalid(
                "view lexical authority requires the exact enclosing Function34",
            ));
        }
        Ok(())
    }
    fn context_for(&mut self, ty: TypeObjectDigest) -> Result<Context> {
        self.context_for_at(ty, 0)
    }
    fn context_for_at(&mut self, ty: TypeObjectDigest, depth: usize) -> Result<Context> {
        self.budget.step(depth)?;
        Ok(match self.object(ty)?.form {
            TypeForm::ReadViewApplied {
                read_scope: ReadScopeArgument::Parameter(scope),
                ..
            }
            | TypeForm::AssociatedReadViewMember {
                read_scope: ReadScopeArgument::Parameter(scope),
                ..
            }
            | TypeForm::AssociatedReadViewProjection {
                read_scope: ReadScopeArgument::Parameter(scope),
                ..
            } => Context::binder(scope.binder),
            TypeForm::ReadViewApplied {
                read_scope: ReadScopeArgument::Lexical { callable, binding },
                ..
            }
            | TypeForm::AssociatedReadViewProjection {
                read_scope: ReadScopeArgument::Lexical { callable, binding },
                ..
            } => {
                if self.proof.lexical_owners.get(&(callable.package, binding)) != Some(&callable) {
                    return Err(invalid(
                        "nominal instance has an unproved lexical callable/loan binding",
                    ));
                }
                let mut context = Context::declaration(callable);
                self.budget.reserve(1)?;
                context.lexical.insert(binding);
                context
            }
            TypeForm::AssociatedWitnessProjection { scope, .. } => Context::declaration(scope),
            TypeForm::AssociatedContractMember { contract, .. } => Context::declaration(contract),
            TypeForm::TypeParameter { parameter } => {
                let mut selected = None;
                for ((package, key), record) in &self.proof.owners {
                    self.budget.step(depth)?;
                    if *key == OwnerKey::TypeParameter(parameter)
                        && let OwnerRecord::TypeParameter(record) = record
                    {
                        if selected.is_some() {
                            return Err(invalid(
                                "ambiguous type formal requires exact owner context",
                            ));
                        }
                        selected = Some(DeclarationReference {
                            package: *package,
                            declaration: record.declaration,
                        });
                    }
                }
                Context::declaration(
                    selected.ok_or_else(|| invalid("unbound canonical type formal"))?,
                )
            }
            TypeForm::TaskFunction { effect, .. } => {
                let object = self.object(ty)?;
                let mut found = Context::default();
                for child in object.child_types() {
                    let child = self.context_for_at(child, depth + 1)?;
                    if let Some(owner) = child.owner {
                        if found.owner.is_some_and(|previous| previous != owner) {
                            return Err(invalid("task type imports another ordinary binder"));
                        }
                        found = child;
                    }
                }
                for parameter in effect.parameters {
                    let OwnerRecord::EffectParameter(record) = self.owner(
                        parameter.package,
                        OwnerKey::EffectParameter(parameter.parameter),
                    )?
                    else {
                        return Err(invalid("task type effect formal record absent"));
                    };
                    let owner = DeclarationReference {
                        package: parameter.package,
                        declaration: record.declaration,
                    };
                    if found.owner.is_some_and(|previous| previous != owner) {
                        return Err(invalid("task type imports another effect binder"));
                    }
                    found = Context::declaration(owner);
                }
                for requirement in effect.requirements {
                    if let kernel::RequirementOperand::Parameter(parameter) = requirement {
                        let OwnerRecord::RequirementParameter(record) = self.owner(
                            parameter.package,
                            OwnerKey::RequirementParameter(parameter.parameter),
                        )?
                        else {
                            return Err(invalid("task type requirement formal record absent"));
                        };
                        let owner = DeclarationReference {
                            package: parameter.package,
                            declaration: record.declaration,
                        };
                        if found.owner.is_some_and(|previous| previous != owner) {
                            return Err(invalid("task type imports another requirement binder"));
                        }
                        found = Context::declaration(owner);
                    }
                }
                found
            }
            _ => {
                let object = self.object(ty)?;
                let mut found = Context::default();
                for child in object.child_types() {
                    let context = self.context_for_at(child, depth + 1)?;
                    if context.owner.is_some() {
                        found = context;
                        break;
                    }
                }
                found
            }
        })
    }
    fn parameter(
        &mut self,
        parameter: TypeParameterId,
        context: &Context,
    ) -> Result<TypeParameterConstraints> {
        let owner = context
            .owner
            .ok_or_else(|| invalid("type formal has no exact occurrence owner"))?;
        let declaration = self.declaration(owner)?;
        if !declaration.payload.type_parameters().contains(&parameter) {
            return Err(invalid("foreign or unused unlisted type formal"));
        }
        match self.owner(owner.package, OwnerKey::TypeParameter(parameter))? {
            OwnerRecord::TypeParameter(record)
                if record.declaration == owner.declaration
                    && record.header.kind == OwnerKind::TypeParameter =>
            {
                Ok(record.constraints)
            }
            _ => Err(invalid("type formal's record names another declaration")),
        }
    }
    fn requirement_constraint(
        &mut self,
        operand: kernel::RequirementOperand,
        context: &Context,
    ) -> Result<kernel::RequirementConstraint> {
        self.budget.step(0)?;
        let (constraint, metadata_owner) = match operand {
            kernel::RequirementOperand::Parameter(reference) => {
                let owner = context
                    .owner
                    .ok_or_else(|| invalid("requirement formal has no exact caller owner"))?;
                let OwnerRecord::RequirementParameter(record) = self.owner(
                    reference.package,
                    OwnerKey::RequirementParameter(reference.parameter),
                )?
                else {
                    return Err(invalid("requirement formal record absent"));
                };
                let declaration = self.declaration(owner)?;
                if reference.package != owner.package
                    || record.declaration != owner.declaration
                    || record.header.kind != OwnerKind::RequirementParameter
                    || record.header.contract_version
                        < kernel::contract::REQUIREMENT_GRAPH_CONTRACT_VERSION
                    || !matches!(&declaration.payload, DeclarationPayload::Function(f) if f.requirement_parameters.contains(&reference.parameter))
                {
                    return Err(invalid(
                        "requirement formal imports another exact function binder",
                    ));
                }
                (record.constraint, owner)
            }
            kernel::RequirementOperand::Concrete(reference) => {
                if let Some(owner) = context.owner
                    && owner.package != reference.package
                    && !self
                        .proof
                        .dependencies
                        .get(&owner.package)
                        .is_some_and(|d| d.contains(&reference.package))
                {
                    return Err(invalid(
                        "requirement atom lies outside caller dependency inventory",
                    ));
                }
                let OwnerRecord::Requirement(record) = self.owner(
                    reference.package,
                    OwnerKey::Requirement(reference.requirement),
                )?
                else {
                    return Err(invalid("concrete requirement record absent"));
                };
                let owner = DeclarationReference {
                    package: reference.package,
                    declaration: record.declaration,
                };
                let listed = match self.declaration(owner)?.payload {
                    DeclarationPayload::Component { requirements, .. } => {
                        requirements.contains(&reference.requirement)
                    }
                    DeclarationPayload::Function(FunctionDeclaration {
                        effect: FunctionEffect::Task { requirements, .. },
                        ..
                    }) => requirements.contains(&kernel::RequirementOperand::Concrete(reference)),
                    _ => false,
                };
                if record.header.kind != OwnerKind::Requirement || !listed {
                    return Err(invalid(
                        "requirement atom has no exact component/function inventory owner",
                    ));
                }
                if record.limits.len() > kernel::contract::MAXIMUM_RESOURCE_LIMITS
                    || record
                        .limits
                        .windows(2)
                        .any(|pair| pair[0].name >= pair[1].name)
                    || record.limits.iter().any(|limit| limit.maximum == 0)
                {
                    return Err(invalid(
                        "requirement resource limits have an invalid complete inventory",
                    ));
                }
                for _ in &record.limits {
                    self.budget.step(0)?;
                }
                (
                    kernel::RequirementConstraint {
                        interface: record.interface,
                        operations: record.operations,
                    },
                    owner,
                )
            }
        };
        constraint
            .validate()
            .map_err(|_| invalid("requirement constraint has noncanonical operation inventory"))?;
        self.check_reference(&Context::declaration(metadata_owner), constraint.interface)?;
        let DeclarationPayload::Interface { operations } =
            self.declaration(constraint.interface)?.payload
        else {
            return Err(invalid(
                "requirement constraint selects a non-interface declaration",
            ));
        };
        for operation in &constraint.operations {
            self.budget.step(0)?;
            let OwnerRecord::Operation(record) =
                self.owner(operation.package, OwnerKey::Operation(operation.operation))?
            else {
                return Err(invalid("requirement constraint operation record absent"));
            };
            if operation.package != constraint.interface.package
                || record.declaration != constraint.interface.declaration
                || record.header.kind != OwnerKind::Operation
                || !operations.contains(&operation.operation)
            {
                return Err(invalid(
                    "requirement constraint operation imports another interface",
                ));
            }
        }
        Ok(constraint)
    }
    fn effect_parameter(
        &mut self,
        reference: kernel::EffectParameterReference,
        context: &Context,
    ) -> Result<()> {
        self.budget.step(0)?;
        let owner = context
            .owner
            .ok_or_else(|| invalid("effect formal has no exact caller owner"))?;
        let OwnerRecord::EffectParameter(record) = self.owner(
            reference.package,
            OwnerKey::EffectParameter(reference.parameter),
        )?
        else {
            return Err(invalid("effect formal record absent"));
        };
        if reference.package != owner.package
            || record.declaration != owner.declaration
            || record.header.kind != OwnerKind::EffectParameter
            || !matches!(self.declaration(owner)?.payload, DeclarationPayload::Function(ref f) if f.effect_parameters.contains(&reference.parameter))
        {
            return Err(invalid(
                "effect formal imports another exact function binder",
            ));
        }
        Ok(())
    }
    fn admit_effect(&mut self, row: &kernel::EffectRow, context: &Context) -> Result<()> {
        row.validate()
            .map_err(|_| invalid("effect row inventory is not canonical"))?;
        for operand in &row.requirements {
            self.requirement_constraint(*operand, context)?;
        }
        for parameter in &row.parameters {
            self.effect_parameter(*parameter, context)?;
        }
        Ok(())
    }
    fn effect_row(&mut self, effect: &FunctionEffect, depth: usize) -> Result<kernel::EffectRow> {
        if let FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = effect
        {
            self.budget
                .reserve(requirements.len() + effect_parameters.len())?;
            for _ in requirements
                .iter()
                .map(|_| ())
                .chain(effect_parameters.iter().map(|_| ()))
            {
                self.budget.step(depth)?;
            }
        } else {
            self.budget.step(depth)?;
        }
        Ok(effect.row())
    }
    fn function_rows(
        &mut self,
        reference: DeclarationReference,
        function: &FunctionDeclaration,
    ) -> Result<()> {
        if !function.requirement_parameters.is_empty()
            && self.declaration(reference)?.header.contract_version
                < kernel::contract::REQUIREMENT_GRAPH_CONTRACT_VERSION
        {
            return Err(invalid(
                "requirement binder uses a predecessor function generation",
            ));
        }
        self.budget
            .reserve(function.effect_parameters.len() + function.requirement_parameters.len())?;
        for _ in function
            .effect_parameters
            .iter()
            .map(|_| ())
            .chain(function.requirement_parameters.iter().map(|_| ()))
        {
            self.budget.step(0)?;
        }
        unique(&function.effect_parameters, "function effect formals")?;
        unique(
            &function.requirement_parameters,
            "function requirement formals",
        )?;
        let context = Context::declaration(reference);
        let mut names = BTreeSet::new();
        self.budget
            .reserve(function.effect_parameters.len() + function.requirement_parameters.len())?;
        for parameter in &function.effect_parameters {
            self.effect_parameter(
                kernel::EffectParameterReference {
                    package: reference.package,
                    parameter: *parameter,
                },
                &context,
            )?;
            let OwnerRecord::EffectParameter(record) =
                self.owner(reference.package, OwnerKey::EffectParameter(*parameter))?
            else {
                unreachable!()
            };
            if !names.insert(record.name) {
                return Err(invalid("duplicate effect formal name"));
            }
        }
        names.clear();
        for parameter in &function.requirement_parameters {
            self.requirement_constraint(
                kernel::RequirementOperand::Parameter(kernel::RequirementParameterReference {
                    package: reference.package,
                    parameter: *parameter,
                }),
                &context,
            )?;
            let OwnerRecord::RequirementParameter(record) = self.owner(
                reference.package,
                OwnerKey::RequirementParameter(*parameter),
            )?
            else {
                unreachable!()
            };
            if !names.insert(record.name) {
                return Err(invalid("duplicate requirement formal name"));
            }
        }
        let row = self.effect_row(&function.effect, 0)?;
        self.admit_effect(&row, &context)
    }
    fn bind_rows(
        &mut self,
        reference: DeclarationReference,
        function: &FunctionDeclaration,
        effects: &[kernel::EffectRow],
        requirements: &[kernel::RequirementOperand],
        context: &Context,
        substitution: &mut Substitution,
    ) -> Result<()> {
        self.function_rows(reference, function)?;
        if effects.len() != function.effect_parameters.len()
            || requirements.len() != function.requirement_parameters.len()
        {
            return Err(invalid(
                "call effect/requirement actuals omit unused ordered formals or change arity",
            ));
        }
        self.budget.reserve(effects.len() + requirements.len())?;
        for (parameter, actual) in function.requirement_parameters.iter().zip(requirements) {
            let formal = kernel::RequirementParameterReference {
                package: reference.package,
                parameter: *parameter,
            };
            let required = self.requirement_constraint(
                kernel::RequirementOperand::Parameter(formal),
                &Context::declaration(reference),
            )?;
            let actual_constraint = self.requirement_constraint(*actual, context)?;
            if !actual_constraint
                .entails(&required)
                .map_err(|_| invalid("invalid requirement entailment operands"))?
            {
                return Err(invalid(
                    "requirement actual omits an exact interface or required operation",
                ));
            }
            substitution.requirements.insert(formal, *actual);
        }
        for (parameter, actual) in function.effect_parameters.iter().zip(effects) {
            self.admit_effect(actual, context)?;
            self.budget
                .reserve(actual.requirements.len() + actual.parameters.len())?;
            for _ in actual
                .requirements
                .iter()
                .map(|_| ())
                .chain(actual.parameters.iter().map(|_| ()))
            {
                self.budget.step(0)?;
            }
            substitution.effects.insert(
                kernel::EffectParameterReference {
                    package: reference.package,
                    parameter: *parameter,
                },
                actual.clone(),
            );
        }
        Ok(())
    }
    /// Source requirements are replaced before source effect parameters. Whole
    /// caller rows are inserted opaque, so recursive E/R permutations cannot
    /// capture an inserted caller atom even when its ID also names a callee key.
    fn substitute_effect(
        &mut self,
        row: &kernel::EffectRow,
        substitution: &Substitution,
        depth: usize,
    ) -> Result<kernel::EffectRow> {
        self.budget.step(depth)?;
        row.validate()
            .map_err(|_| invalid("source effect row is not canonical"))?;
        let mut count = row.requirements.len();
        for parameter in &row.parameters {
            self.budget.step(depth)?;
            if let Some(actual) = substitution.effects.get(parameter) {
                actual
                    .validate()
                    .map_err(|_| invalid("effect actual row is not canonical"))?;
            }
            count = count
                .checked_add(substitution.effects.get(parameter).map_or(1, |actual| {
                    actual.requirements.len() + actual.parameters.len()
                }))
                .ok_or_else(|| {
                    ExecutionError::resource(
                        "reference_effect_substitution",
                        "row occurrence count overflow",
                    )
                })?;
        }
        self.budget.reserve(count)?;
        let mut result = kernel::EffectRow::default();
        for operand in &row.requirements {
            self.budget.step(depth)?;
            result.requirements.push(match operand {
                kernel::RequirementOperand::Parameter(formal) => substitution
                    .requirements
                    .get(formal)
                    .copied()
                    .unwrap_or(*operand),
                _ => *operand,
            });
        }
        for parameter in &row.parameters {
            self.budget.step(depth)?;
            if let Some(actual) = substitution.effects.get(parameter) {
                for operand in &actual.requirements {
                    self.budget.step(depth)?;
                    result.requirements.push(*operand);
                }
                for parameter in &actual.parameters {
                    self.budget.step(depth)?;
                    result.parameters.push(*parameter);
                }
            } else {
                result.parameters.push(*parameter);
            }
        }
        result
            .normalize()
            .map_err(|_| invalid("substituted effect union exceeds canonical finite row"))?;
        Ok(result)
    }

    fn caller_row(&mut self, context: &Context) -> Result<(bool, kernel::EffectRow)> {
        let owner = context
            .owner
            .ok_or_else(|| invalid("activation has no exact canonical owner"))?;
        if let Some(reference) = context.port {
            let OwnerRecord::Port(port) =
                self.owner(reference.package, OwnerKey::Port(reference.port))?
            else {
                return Err(invalid("port activation record absent"));
            };
            if reference.package != owner.package || port.declaration != owner.declaration {
                return Err(invalid("port activation imports another component owner"));
            }
            if port.header.kind != OwnerKind::Port
                || !matches!(self.declaration(owner)?.payload, DeclarationPayload::Component { ref ports, .. } if ports.contains(&reference.port))
            {
                return Err(invalid(
                    "port activation is absent from the exact component inventory",
                ));
            }
            return Ok(match self.object(port.function_type)?.form {
                TypeForm::Function { .. } => (false, kernel::EffectRow::default()),
                TypeForm::TaskFunction { effect, .. } if effect.is_closed() => (true, effect),
                _ => {
                    return Err(invalid(
                        "port activation is not an exact closed pure/task signature",
                    ));
                }
            });
        }
        Ok(match self.declaration(owner)?.payload {
            DeclarationPayload::Function(function) => (
                matches!(function.effect, FunctionEffect::Task { .. }),
                self.effect_row(&function.effect, 0)?,
            ),
            _ => (false, kernel::EffectRow::default()),
        })
    }
    fn admit_invocation(
        &mut self,
        task: bool,
        row: &kernel::EffectRow,
        context: &Context,
    ) -> Result<()> {
        self.admit_effect(row, context)?;
        if !task {
            return Ok(());
        }
        let (caller_task, available) = self.caller_row(context)?;
        if !caller_task {
            return Err(invalid(
                "pure caller invokes a task, including a task with an empty row",
            ));
        }
        self.row_coverage(row, &available, context)
    }
    fn row_coverage(
        &mut self,
        row: &kernel::EffectRow,
        available: &kernel::EffectRow,
        context: &Context,
    ) -> Result<()> {
        self.admit_effect(row, context)?;
        self.admit_effect(available, context)?;
        for parameter in &row.parameters {
            self.budget.step(0)?;
            if available.parameters.binary_search(parameter).is_err() {
                return Err(invalid(
                    "task invocation exceeds exact caller symbolic effect allowance",
                ));
            }
        }
        for required in &row.requirements {
            self.budget.step(0)?;
            if available.requirements.binary_search(required).is_ok() {
                continue;
            }
            let mut covered = false;
            for allowance in &available.requirements {
                self.budget.step(0)?;
                if let (
                    kernel::RequirementOperand::Concrete(required),
                    kernel::RequirementOperand::Concrete(allowance),
                ) = (required, allowance)
                {
                    let OwnerRecord::Requirement(candidate) = self.owner(
                        required.package,
                        OwnerKey::Requirement(required.requirement),
                    )?
                    else {
                        return Err(invalid("required atom record absent"));
                    };
                    let OwnerRecord::Requirement(slot) = self.owner(
                        allowance.package,
                        OwnerKey::Requirement(allowance.requirement),
                    )?
                    else {
                        return Err(invalid("allowance atom record absent"));
                    };
                    let mut limits = true;
                    for required in &candidate.limits {
                        let mut matched = false;
                        for available in &slot.limits {
                            self.budget.step(0)?;
                            matched |= available.name == required.name
                                && available.unit == required.unit
                                && available.maximum <= required.maximum;
                        }
                        limits &= matched;
                    }
                    let mut operations = true;
                    for operation in &candidate.operations {
                        self.budget.step(0)?;
                        operations &= slot.operations.binary_search(operation).is_ok();
                    }
                    covered |= required.package == allowance.package
                        && candidate.name == slot.name
                        && candidate.interface == slot.interface
                        && operations
                        && limits;
                }
            }
            if !covered {
                return Err(invalid(
                    "task invocation exceeds exact caller requirement allowance",
                ));
            }
        }
        Ok(())
    }
    fn admit_port(
        &mut self,
        reference: kernel::PortReference,
        context: &Context,
    ) -> Result<TypeObjectDigest> {
        let OwnerRecord::Port(port) =
            self.owner(reference.package, OwnerKey::Port(reference.port))?
        else {
            return Err(invalid("port record absent"));
        };
        let owner = context
            .owner
            .ok_or_else(|| invalid("port has no component context"))?;
        let DeclarationPayload::Component {
            requirements,
            ports,
        } = self.declaration(owner)?.payload
        else {
            return Err(invalid("port parent is not a component"));
        };
        if reference.package != owner.package
            || port.declaration != owner.declaration
            || !ports.contains(&reference.port)
        {
            return Err(invalid("port changes exact owning component inventory"));
        }
        self.facts(port.function_type, context, 0)?;
        let (_, required) = self.caller_row(context)?;
        self.budget
            .reserve(requirements.len() + required.requirements.len())?;
        let mut available = kernel::EffectRow::default();
        for requirement in requirements {
            self.budget.step(0)?;
            available
                .requirements
                .push(kernel::RequirementOperand::Concrete(
                    kernel::RequirementReference {
                        package: owner.package,
                        requirement,
                    },
                ));
        }
        for requirement in &required.requirements {
            self.budget.step(0)?;
            if matches!(requirement, kernel::RequirementOperand::Concrete(reference) if reference.package != owner.package)
            {
                available.requirements.push(*requirement);
            }
        }
        available
            .normalize()
            .map_err(|_| invalid("port component allowance has noncanonical finite row"))?;
        self.row_coverage(&required, &available, context)?;
        Ok(port.function_type)
    }
    fn bind_types(
        &mut self,
        reference: DeclarationReference,
        arguments: &[TypeObjectDigest],
        context: &Context,
    ) -> Result<Substitution> {
        self.check_reference(context, reference)?;
        let parameters = self
            .declaration(reference)?
            .payload
            .type_parameters()
            .to_vec();
        if parameters.len() != arguments.len() {
            return Err(invalid(
                "nominal/callable argument arity differs from complete formal inventory",
            ));
        }
        self.budget.reserve(parameters.len())?;
        let mut substitution = Substitution::default();
        // Only external call admission uses independent argument roots. Inner
        // facts/layout use bind_types_at and retain one state and combined depth.
        for (parameter, actual) in parameters.iter().zip(arguments) {
            let constraints = self.parameter(*parameter, &Context::declaration(reference))?;
            let facts = self.prove_facts(*actual, context, 0)?.0;
            if facts.category == ValueCategory::ReadView
                || constraints.has_owned() != (facts.category == ValueCategory::Owned)
                || (!constraints.has_owned()
                    && !matches!(
                        facts.category,
                        ValueCategory::OrdinaryData | ValueCategory::Callable
                    ))
                || (constraints.requires_transfer() && !facts.transfer)
                || (constraints.requires_share() && !facts.share)
                || (constraints.requires_capture_safe()
                    && !self.capture_safe(*actual, context, &mut BTreeSet::new(), 0)?)
            {
                return Err(invalid(
                    "type actual, including phantom actual, violates its exact ordinary/Owned constraint",
                ));
            }
            if substitution.types.insert(*parameter, *actual).is_some() {
                return Err(invalid("duplicate type formal"));
            }
        }
        Ok(substitution)
    }
    fn bind_types_at(
        &mut self,
        reference: DeclarationReference,
        arguments: &[TypeObjectDigest],
        context: &Context,
        state: &mut FactsState,
        depth: usize,
    ) -> Result<Substitution> {
        self.budget.step(depth)?;
        self.check_reference(context, reference)?;
        let declaration = self.declaration(reference)?;
        let parameters = declaration.payload.type_parameters().to_vec();
        if parameters.len() != arguments.len() {
            return Err(invalid(
                "nominal/callable argument arity differs from complete formal inventory",
            ));
        }
        self.budget.reserve(parameters.len())?;
        let mut substitution = Substitution::default();
        for (parameter, actual) in parameters.iter().zip(arguments) {
            let constraints = self.parameter(*parameter, &Context::declaration(reference))?;
            let facts = self.facts_walk(*actual, context, state, depth + 1)?;
            if facts.category == ValueCategory::ReadView
                || constraints.has_owned() != (facts.category == ValueCategory::Owned)
                || (!constraints.has_owned()
                    && !matches!(
                        facts.category,
                        ValueCategory::OrdinaryData | ValueCategory::Callable
                    ))
            {
                return Err(invalid(
                    "type actual, including phantom actual, violates its exact ordinary/Owned constraint",
                ));
            }
            if constraints.requires_capture_safe()
                && !self.capture_safe(*actual, context, &mut BTreeSet::new(), depth + 1)?
            {
                return Err(invalid(
                    "type actual violates its complete CaptureSafe constraint",
                ));
            }
            if constraints.requires_transfer() {
                let key = self.facts_key(*actual, context, depth)?;
                self.facts_search(&key, state.transfer.len(), depth)?;
                if !state.transfer.contains(&key) {
                    self.budget.reserve(1)?;
                    self.facts_search(&key, state.transfer.len(), depth)?;
                    state.transfer.insert(key);
                }
            }
            if constraints.requires_share() {
                let key = self.facts_key(*actual, context, depth)?;
                self.facts_search(&key, state.share.len(), depth)?;
                if !state.share.contains(&key) {
                    self.budget.reserve(1)?;
                    self.facts_search(&key, state.share.len(), depth)?;
                    state.share.insert(key);
                }
            }
            if substitution.types.insert(*parameter, *actual).is_some() {
                return Err(invalid("duplicate type formal"));
            }
        }
        Ok(substitution)
    }
    fn member(
        &mut self,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        view: bool,
    ) -> Result<()> {
        let declaration = self.declaration(contract)?;
        let DeclarationPayload::OwnedContract(contract) = declaration.payload else {
            return Err(invalid("associated owner is not an Owned contract"));
        };
        let found = contract
            .associated_types
            .iter()
            .find(|m| m.id == member)
            .ok_or_else(|| invalid("associated member absent from exact contract"))?;
        if view != matches!(found.bound, AssociatedTypeBound::ReadViewFamily { .. }) {
            return Err(invalid("associated member kind/arity swap"));
        }
        Ok(())
    }
    fn projection(
        &mut self,
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
        contract: DeclarationReference,
        context: &Context,
    ) -> Result<()> {
        if context.owner != Some(scope) {
            return Err(invalid(
                "associated projection imports another witness binder",
            ));
        }
        let declaration = self.declaration(scope)?;
        let parameters = match &declaration.payload {
            DeclarationPayload::Function(f) => &f.implementation_parameters,
            DeclarationPayload::OwnedImplementation(i) => &i.implementation_parameters,
            _ => {
                return Err(invalid(
                    "associated projection requires a lexical witness inventory",
                ));
            }
        };
        if !parameters
            .iter()
            .any(|p| p.id == parameter && p.contract == contract)
        {
            return Err(invalid(
                "associated projection changes exact witness/member authority",
            ));
        }
        Ok(())
    }
    /// A Context owns its complete lexical inventory. Charge every copied entry
    /// and finish the cancellable work prefix before allocating any such copy.
    /// Map/set nodes are reserved separately only at their new-entry sites.
    fn facts_key(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        depth: usize,
    ) -> Result<FactsKey> {
        self.budget.reserve(context.lexical.len())?;
        for _ in &context.lexical {
            self.budget.step(depth)?;
        }
        Ok((ty, context.clone()))
    }
    /// Property/source keys retain distinct occurrence provenance in this root state.
    /// Distinct digests or symbolic/provenance prefixes can finish comparisons
    /// early. Matching keys may inspect the whole lexical inventory; charge that
    /// prefix before lookup.
    fn facts_search(&mut self, key: &FactsKey, entries: usize, depth: usize) -> Result<()> {
        self.budget.step(depth)?;
        if entries != 0 {
            for _ in &key.1.lexical {
                self.budget.step(depth)?;
            }
        }
        Ok(())
    }
    fn facts(&mut self, ty: TypeObjectDigest, context: &Context, depth: usize) -> Result<Facts> {
        Ok(self.prove_facts(ty, context, depth)?.0)
    }
    fn prove_facts(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        depth: usize,
    ) -> Result<(Facts, FactsState)> {
        self.check_view_actual_closure(ty, context, depth)?;
        // Complete preparation proves every input and discovered object as an
        // exact root before any facts/layout cache is published. Check this
        // root once, outside convergence, rather than rescan every shared child.
        self.require_resource_container_free(ty, context, depth)?;
        let mut state = FactsState::default();
        loop {
            self.budget.step(depth)?;
            state.observed.clear();
            self.facts_walk(ty, context, &mut state, depth)?;
            let mut changed = false;
            for (key, observed) in &state.observed {
                self.budget.step(depth)?;
                self.facts_search(key, state.estimates.len(), depth)?;
                if let Some(previous) = state.estimates.get_mut(key) {
                    if previous.category != observed.category {
                        return Err(invalid("recursive facts changed value category"));
                    }
                    let next = Facts {
                        category: observed.category,
                        metadata: previous.metadata && observed.metadata,
                        transfer: previous.transfer && observed.transfer,
                        share: previous.share && observed.share,
                    };
                    changed |= *previous != next;
                    *previous = next;
                } else {
                    self.budget.reserve(1)?;
                    let copied = self.facts_key(key.0, &key.1, depth)?;
                    self.facts_search(&copied, state.estimates.len(), depth)?;
                    state.estimates.insert(copied, *observed);
                    changed = true;
                }
            }
            if !changed {
                self.check_facts_obligations(&state)?;
                let root = self.facts_key(ty, context, depth)?;
                self.facts_search(&root, state.estimates.len(), depth)?;
                let facts = state
                    .estimates
                    .get(&root)
                    .copied()
                    .ok_or_else(|| invalid("converged facts have no exact root estimate"))?;
                return Ok((facts, state));
            }
        }
    }
    fn check_facts_obligations(&mut self, state: &FactsState) -> Result<()> {
        for (keys, property) in [
            (&state.metadata, 0),
            (&state.transfer, 1),
            (&state.share, 2),
        ] {
            for key in keys {
                self.budget.step(0)?;
                self.facts_search(key, state.estimates.len(), 0)?;
                let facts = state
                    .estimates
                    .get(key)
                    .ok_or_else(|| invalid("property obligation has no complete root estimate"))?;
                if !match property {
                    0 => facts.metadata,
                    1 => facts.transfer,
                    _ => facts.share,
                } {
                    return Err(invalid(
                        "complete ordinary metadata/Transfer/Share obligation failed",
                    ));
                }
            }
        }
        Ok(())
    }
    fn facts_seed(&mut self, form: &TypeForm, context: &Context) -> Result<Facts> {
        Ok(match form {
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedSequence { .. }
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => Facts::owned(true, true),
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                if matches!(
                    self.declaration(*declaration)?.payload,
                    DeclarationPayload::OwnedRecord { .. }
                ) {
                    Facts::owned(true, true)
                } else {
                    Facts::ordinary()
                }
            }
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => Facts {
                category: ValueCategory::ReadView,
                metadata: false,
                transfer: false,
                share: true,
            },
            TypeForm::TypeParameter { parameter } => {
                let c = self.parameter(*parameter, context)?;
                if c.has_owned() {
                    Facts::owned(c.requires_transfer(), c.requires_share())
                } else {
                    Facts {
                        category: ValueCategory::OrdinaryData,
                        metadata: c.requires_transfer(),
                        transfer: c.requires_transfer(),
                        share: c.requires_transfer(),
                    }
                }
            }
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. } => Facts {
                category: ValueCategory::Callable,
                metadata: false,
                transfer: false,
                share: false,
            },
            TypeForm::CapabilityResource { .. } => Facts {
                category: ValueCategory::Capability,
                metadata: false,
                transfer: false,
                share: false,
            },
            TypeForm::Secret | TypeForm::Stream { .. } => Facts {
                category: ValueCategory::OrdinaryData,
                metadata: false,
                transfer: false,
                share: false,
            },
            _ => Facts::ordinary(),
        })
    }
    fn facts_walk(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        state: &mut FactsState,
        depth: usize,
    ) -> Result<Facts> {
        self.budget.step(depth)?;
        let form = self.object(ty)?.form;
        let key = self.facts_key(ty, context, depth)?;
        self.facts_search(&key, state.active.len(), depth)?;
        if state.active.contains(&key) {
            let seed = self.facts_seed(&form, context)?;
            if matches!(
                seed.category,
                ValueCategory::Owned | ValueCategory::ReadView
            ) {
                return Err(invalid("cyclic instantiated Owned/view closure"));
            }
            self.facts_search(&key, state.estimates.len(), depth)?;
            return Ok(state.estimates.get(&key).copied().unwrap_or(seed));
        }
        self.budget.reserve(1)?;
        let active_key = self.facts_key(ty, context, depth)?;
        self.facts_search(&active_key, state.active.len(), depth)?;
        state.active.insert(active_key);
        let facts = match form.clone() {
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText => Facts::ordinary(),
            TypeForm::Secret | TypeForm::Stream { .. } => {
                if let TypeForm::Stream { item } = form {
                    self.ordinary_child(item, context, state, depth + 1)?;
                }
                Facts {
                    category: ValueCategory::OrdinaryData,
                    metadata: false,
                    transfer: false,
                    share: false,
                }
            }
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Facts::owned(true, true),
            TypeForm::TypeParameter { parameter } => {
                let constraints = self.parameter(parameter, context)?;
                if constraints.has_owned() {
                    Facts::owned(
                        constraints.requires_transfer(),
                        constraints.requires_share(),
                    )
                } else {
                    Facts {
                        category: ValueCategory::OrdinaryData,
                        metadata: constraints.requires_transfer(),
                        transfer: constraints.requires_transfer(),
                        share: constraints.requires_transfer(),
                    }
                }
            }
            TypeForm::CapabilityResource { interface } => {
                self.check_reference(context, interface)?;
                if !matches!(
                    self.declaration(interface)?.payload,
                    DeclarationPayload::Interface { .. }
                ) {
                    return Err(invalid("capability type changes nominal authority"));
                }
                Facts {
                    category: ValueCategory::Capability,
                    metadata: false,
                    transfer: false,
                    share: false,
                }
            }
            TypeForm::AssociatedContractMember { contract, member } => {
                if context.owner != Some(contract) {
                    return Err(invalid("associated member escapes its contract"));
                }
                self.member(contract, member, false)?;
                Facts::owned(false, false)
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                self.projection(scope, parameter, contract, context)?;
                self.member(contract, member, false)?;
                Facts::owned(false, false)
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                if context.owner != Some(contract)
                    || !matches!(context.binder, Some(ReadScopeBinder::ContractMethod { contract: c, .. }) if c == contract)
                {
                    return Err(invalid(
                        "abstract view occurrence escapes its exact contract method",
                    ));
                }
                self.member(contract, member, true)?;
                self.scope(read_scope, context)?;
                Facts {
                    category: ValueCategory::ReadView,
                    metadata: false,
                    transfer: false,
                    share: false,
                }
            }
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                self.projection(scope, parameter, contract, context)?;
                self.member(contract, member, true)?;
                self.scope(read_scope, context)?;
                Facts {
                    category: ValueCategory::ReadView,
                    metadata: false,
                    transfer: false,
                    share: false,
                }
            }
            TypeForm::ReadViewApplied { .. } => {
                self.budget.reserve(1)?;
                let layout = self.layout_at(ty, context, state, depth)?;
                let mut share = true;
                // Phantom actuals remain part of the independently shared closure.
                for argument in &layout.arguments {
                    share &= self.facts_walk(*argument, context, state, depth + 1)?.share;
                }
                let field_context = self.property_context(context)?;
                for field in &layout.fields {
                    let facts = self.facts_walk(field.ty, &field_context, state, depth + 1)?;
                    match field.kind {
                        ReadViewFieldKind::Ordinary
                            if facts.category == ValueCategory::OrdinaryData =>
                        {
                            let key = self.facts_key(field.ty, &field_context, depth)?;
                            self.facts_search(&key, state.metadata.len(), depth)?;
                            if !state.metadata.contains(&key) {
                                self.budget.reserve(1)?;
                                self.facts_search(&key, state.metadata.len(), depth)?;
                                state.metadata.insert(key);
                            }
                        }
                        ReadViewFieldKind::ReadReference { read_scope }
                            if read_scope == layout.read_scope
                                && matches!(
                                    facts.category,
                                    ValueCategory::Owned | ValueCategory::ReadView
                                ) =>
                        {
                            if facts.category == ValueCategory::ReadView
                                && self.view_scope(field.ty)? != layout.read_scope
                            {
                                return Err(invalid("reference view has another scope actual"));
                            }
                        }
                        _ => {
                            return Err(invalid(
                                "view field violates ordinary/reference category or exact scope",
                            ));
                        }
                    }
                    share &= facts.share;
                }
                // The designated source's passive CustodyOnly guard is not a
                // reference edge and contributes no source-payload sharing grant.
                Facts {
                    category: ValueCategory::ReadView,
                    metadata: false,
                    transfer: false,
                    share,
                }
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                self.budget.reserve(1)?;
                let arguments = match &form {
                    TypeForm::Applied { arguments, .. } => arguments.clone(),
                    _ => Vec::new(),
                };
                let substitution =
                    self.bind_types_at(declaration, &arguments, context, state, depth)?;
                let record = self.declaration(declaration)?;
                let mut children = Vec::new();
                let owned = match record.payload {
                    DeclarationPayload::Record { fields, .. }
                    | DeclarationPayload::OwnedRecord { fields, .. } => {
                        let owned = record.header.kind == OwnerKind::OwnedRecord;
                        for field in fields {
                            let OwnerRecord::Field(field) =
                                self.owner(declaration.package, OwnerKey::Field(field))?
                            else {
                                return Err(invalid("nominal field inventory is incomplete"));
                            };
                            if field.declaration != declaration.declaration {
                                return Err(invalid("nominal field names another declaration"));
                            }
                            children.push(self.substitute(field.ty, &substitution, depth + 1)?);
                        }
                        owned
                    }
                    DeclarationPayload::Variant { cases, .. } => {
                        for case in cases {
                            let OwnerRecord::Case(case) =
                                self.owner(declaration.package, OwnerKey::Case(case))?
                            else {
                                return Err(invalid("nominal case inventory is incomplete"));
                            };
                            if case.declaration != declaration.declaration {
                                return Err(invalid("nominal case names another declaration"));
                            }
                            if let Some(ty) = case.payload {
                                children.push(self.substitute(ty, &substitution, depth + 1)?);
                            }
                        }
                        false
                    }
                    _ => {
                        return Err(invalid(
                            "ordinary/Owned nominal form selects a non-layout declaration",
                        ));
                    }
                };
                let mut metadata = !owned;
                let mut transfer = true;
                let mut share = true;
                let field_context = self.property_context(context)?;
                for (position, child) in arguments.iter().chain(&children).enumerate() {
                    let occurrence = if position < arguments.len() {
                        context
                    } else {
                        &field_context
                    };
                    let f = self.facts_walk(*child, occurrence, state, depth + 1)?;
                    if f.category == ValueCategory::ReadView
                        || (!owned && f.category == ValueCategory::Owned)
                    {
                        return Err(invalid(
                            "hidden view/owner in ordinary or Owned nominal closure",
                        ));
                    }
                    if owned
                        && !matches!(
                            f.category,
                            ValueCategory::Owned | ValueCategory::OrdinaryData
                        )
                    {
                        return Err(invalid(
                            "Owned nominal closure contains capability/callable",
                        ));
                    }
                    metadata &= f.metadata;
                    transfer &= f.transfer;
                    share &= f.share;
                }
                Facts {
                    category: if owned {
                        ValueCategory::Owned
                    } else {
                        ValueCategory::OrdinaryData
                    },
                    metadata,
                    transfer,
                    share,
                }
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                if let TypeForm::TaskFunction { effect, .. } = &form {
                    self.admit_effect(effect, context)?;
                }
                for child in parameters.iter().chain(std::iter::once(&result)) {
                    if self.facts_walk(*child, context, state, depth + 1)?.category
                        == ValueCategory::ReadView
                    {
                        return Err(invalid("first-class callable signature contains a view"));
                    }
                }
                Facts {
                    category: ValueCategory::Callable,
                    metadata: false,
                    transfer: false,
                    share: false,
                }
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                let owned = !matches!(&form, TypeForm::StructuralRecord { .. });
                let mut metadata = !owned;
                let mut transfer = true;
                let mut share = true;
                for field in fields {
                    let f = self.facts_walk(field.ty, context, state, depth + 1)?;
                    if f.category == ValueCategory::ReadView
                        || (!owned && f.category == ValueCategory::Owned)
                        || (owned
                            && !matches!(
                                f.category,
                                ValueCategory::Owned | ValueCategory::OrdinaryData
                            ))
                    {
                        return Err(invalid("view/capability/callable hidden in an aggregate"));
                    }
                    metadata &= f.metadata;
                    transfer &= f.transfer;
                    share &= f.share;
                }
                Facts {
                    category: if owned {
                        ValueCategory::Owned
                    } else {
                        ValueCategory::OrdinaryData
                    },
                    metadata,
                    transfer,
                    share,
                }
            }
            TypeForm::OwnedSequence { item } => {
                let f = self.facts_walk(item, context, state, depth + 1)?;
                if !matches!(
                    f.category,
                    ValueCategory::Owned | ValueCategory::OrdinaryData
                ) {
                    return Err(invalid("Owned sequence hides a view/callable/capability"));
                }
                Facts::owned(f.transfer, f.share)
            }
            TypeForm::List { item } | TypeForm::Option { item } => {
                self.ordinary_child(item, context, state, depth + 1)?
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                let a = self.ordinary_child(key, context, state, depth + 1)?;
                let b = self.ordinary_child(value, context, state, depth + 1)?;
                Facts {
                    category: ValueCategory::OrdinaryData,
                    metadata: a.metadata && b.metadata,
                    transfer: a.transfer && b.transfer,
                    share: a.share && b.share,
                }
            }
        };
        self.facts_search(&key, state.active.len(), depth)?;
        state.active.remove(&key);
        self.facts_search(&key, state.observed.len(), depth)?;
        if let Some(previous) = state.observed.get_mut(&key) {
            if previous.category != facts.category {
                return Err(invalid("same exact context changed category"));
            }
            previous.metadata &= facts.metadata;
            previous.transfer &= facts.transfer;
            previous.share &= facts.share;
        } else {
            self.budget.reserve(1)?;
            self.facts_search(&key, state.observed.len(), depth)?;
            state.observed.insert(key, facts);
        }
        Ok(facts)
    }
    fn ordinary_child(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        state: &mut FactsState,
        depth: usize,
    ) -> Result<Facts> {
        let facts = self.facts_walk(ty, context, state, depth)?;
        if matches!(
            facts.category,
            ValueCategory::Owned | ValueCategory::ReadView
        ) {
            return Err(invalid("ordinary container contains affine type"));
        }
        Ok(Facts {
            category: ValueCategory::OrdinaryData,
            ..facts
        })
    }
    fn view_scope(&mut self, ty: TypeObjectDigest) -> Result<ReadScopeArgument> {
        match self.object(ty)?.form {
            TypeForm::ReadViewApplied { read_scope, .. }
            | TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Ok(read_scope),
            _ => Err(invalid("type has no read-view scope")),
        }
    }
    fn layout(&mut self, ty: TypeObjectDigest, context: &Context, depth: usize) -> Result<Layout> {
        let (_, mut state) = self.prove_facts(ty, context, depth)?;
        let layout = self.layout_at(ty, context, &mut state, depth)?;
        self.check_facts_obligations(&state)?;
        Ok(layout)
    }
    fn layout_at(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        state: &mut FactsState,
        depth: usize,
    ) -> Result<Layout> {
        self.budget.step(depth)?;
        let TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope,
        } = self.object(ty)?.form
        else {
            return Err(invalid(
                "layout lookup requires the exact nominal read-view instance",
            ));
        };
        self.scope(read_scope, context)?;
        let record = self.declaration(declaration)?;
        let DeclarationPayload::ReadView(view) = record.payload else {
            return Err(invalid("view instance changes declaration kind"));
        };
        if record.header.kind != OwnerKind::ReadView
            || record.header.contract_version < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
        {
            return Err(invalid(
                "nominal view declaration uses unsupported category/generation",
            ));
        }
        self.view_inventory(declaration, &view)?;
        let mut substitution =
            self.bind_types_at(declaration, &arguments, context, state, depth)?;
        let formal = ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration),
            parameter: view.scope_parameter.id,
        };
        substitution.scopes.insert(formal, read_scope);
        self.budget.reserve(view.fields.len())?;
        let mut fields = Vec::new();
        for field in view.fields {
            let ty = self.substitute(field.ty, &substitution, depth + 1)?;
            let kind = match field.kind {
                ReadViewFieldKind::Ordinary => ReadViewFieldKind::Ordinary,
                ReadViewFieldKind::ReadReference { read_scope } => {
                    ReadViewFieldKind::ReadReference {
                        read_scope: substitute_scope(read_scope, &substitution),
                    }
                }
            };
            fields.push(Field {
                id: field.id,
                #[cfg(test)]
                name: field.name,
                ty,
                kind,
            });
        }
        Ok(Layout {
            #[cfg(test)]
            ty,
            declaration,
            arguments,
            read_scope,
            fields,
            #[cfg(test)]
            types: BTreeMap::new(),
        })
    }
    fn view_inventory(
        &mut self,
        declaration: DeclarationReference,
        view: &ReadViewDeclaration,
    ) -> Result<()> {
        if view.fields.len() > kernel::contract::MAXIMUM_CHILDREN {
            return Err(invalid("view field inventory exceeds canonical bound"));
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration),
            parameter: view.scope_parameter.id,
        });
        for field in &view.fields {
            self.budget.step(0)?;
            if !ids.insert(field.id) || !names.insert(&field.name) {
                return Err(invalid("duplicate inline view field identity/name"));
            }
            if let ReadViewFieldKind::ReadReference { read_scope } = field.kind
                && read_scope != scope
            {
                return Err(invalid(
                    "view field reference does not name exact declaration scope",
                ));
            }
        }
        for parameter in &view.type_parameters {
            let c = self.parameter(*parameter, &Context::declaration(declaration))?;
            if !c.has_owned() && !c.requires_transfer() {
                return Err(invalid(
                    "view type formal lacks explicit Owned or ordinary Transferable constraint",
                ));
            }
        }
        Ok(())
    }
    fn substitute(
        &mut self,
        ty: TypeObjectDigest,
        substitution: &Substitution,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        if substitution.effects.is_empty() && substitution.requirements.is_empty() {
            return self.substitute_at(ty, substitution, depth, SignatureStage::Types);
        }
        let source = self.substitute_at(ty, substitution, depth, SignatureStage::Effects)?;
        self.substitute_at(source, substitution, depth, SignatureStage::Types)
    }
    fn substitute_at(
        &mut self,
        ty: TypeObjectDigest,
        substitution: &Substitution,
        depth: usize,
        stage: SignatureStage,
    ) -> Result<TypeObjectDigest> {
        self.budget.step(depth)?;
        let original = self.object(ty)?.form;
        if stage == SignatureStage::Effects
            && (matches!(original, TypeForm::TypeParameter { .. }) || original.is_associated())
        {
            return Ok(ty);
        }
        let form = match original.clone() {
            TypeForm::TypeParameter { parameter } => {
                return Ok(substitution.types.get(&parameter).copied().unwrap_or(ty));
            }
            TypeForm::AssociatedContractMember { contract, member } => {
                if let Some((template, None)) = substitution.members.get(&(contract, member)) {
                    // A selected member is an admitted caller instance, not a
                    // callee template to which this application runs again.
                    return Ok(*template);
                }
                original.clone()
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                if let Some(operand) = substitution.witnesses.get(&(scope, parameter)) {
                    return self.select_member(operand, contract, member, None, depth + 1);
                }
                original.clone()
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                let actual = substitute_scope(read_scope, substitution);
                if let Some((template, Some(formal))) =
                    substitution.members.get(&(contract, member))
                {
                    self.budget.reserve(1)?;
                    let mut rhs = Substitution::default();
                    rhs.scopes.insert(*formal, actual);
                    return self.substitute_at(*template, &rhs, depth + 1, SignatureStage::Types);
                }
                TypeForm::AssociatedReadViewMember {
                    contract,
                    member,
                    read_scope: actual,
                }
            }
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                let actual = substitute_scope(read_scope, substitution);
                if let Some(operand) = substitution.witnesses.get(&(scope, parameter)) {
                    return self.select_member(operand, contract, member, Some(actual), depth + 1);
                }
                TypeForm::AssociatedReadViewProjection {
                    scope,
                    parameter,
                    contract,
                    member,
                    read_scope: actual,
                }
            }
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                let arguments =
                    self.substitute_many_at(&arguments, substitution, depth + 1, stage)?;
                TypeForm::ReadViewApplied {
                    declaration,
                    arguments,
                    read_scope: if stage == SignatureStage::Types {
                        substitute_scope(read_scope, substitution)
                    } else {
                        read_scope
                    },
                }
            }
            TypeForm::Applied {
                declaration,
                arguments,
            } => TypeForm::Applied {
                declaration,
                arguments: self.substitute_many_at(&arguments, substitution, depth + 1, stage)?,
            },
            TypeForm::List { item } => TypeForm::List {
                item: self.substitute_at(item, substitution, depth + 1, stage)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: self.substitute_at(item, substitution, depth + 1, stage)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: self.substitute_at(item, substitution, depth + 1, stage)?,
            },
            TypeForm::OwnedSequence { item } => TypeForm::OwnedSequence {
                item: self.substitute_at(item, substitution, depth + 1, stage)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: self.substitute_at(key, substitution, depth + 1, stage)?,
                value: self.substitute_at(value, substitution, depth + 1, stage)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: self.substitute_at(ok, substitution, depth + 1, stage)?,
                error: self.substitute_at(error, substitution, depth + 1, stage)?,
            },
            TypeForm::StructuralRecord { fields } => TypeForm::StructuralRecord {
                fields: self.substitute_fields_at(fields, substitution, depth + 1, stage)?,
            },
            TypeForm::OwnedProduct { fields } => TypeForm::OwnedProduct {
                fields: self.substitute_fields_at(fields, substitution, depth + 1, stage)?,
            },
            TypeForm::OwnedChoice { cases } => TypeForm::OwnedChoice {
                cases: self.substitute_fields_at(cases, substitution, depth + 1, stage)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: self.substitute_many_at(&parameters, substitution, depth + 1, stage)?,
                result: self.substitute_at(result, substitution, depth + 1, stage)?,
            },
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => {
                // Close only this source row before ordinary/witness actuals
                // enter the signature. Inserted actual types remain opaque.
                let effect = if stage == SignatureStage::Effects {
                    self.substitute_effect(&effect, substitution, depth + 1)?
                } else {
                    effect
                };
                TypeForm::TaskFunction {
                    parameters: self.substitute_many_at(
                        &parameters,
                        substitution,
                        depth + 1,
                        stage,
                    )?,
                    result: self.substitute_at(result, substitution, depth + 1, stage)?,
                    effect,
                }
            }
            _ => original.clone(),
        };
        if form == original {
            Ok(ty)
        } else {
            self.intern(form)
        }
    }
    fn substitute_many(
        &mut self,
        types: &[TypeObjectDigest],
        substitution: &Substitution,
        depth: usize,
    ) -> Result<Vec<TypeObjectDigest>> {
        self.budget.reserve(types.len())?;
        types
            .iter()
            .map(|ty| self.substitute(*ty, substitution, depth))
            .collect()
    }
    fn substitute_many_at(
        &mut self,
        types: &[TypeObjectDigest],
        substitution: &Substitution,
        depth: usize,
        stage: SignatureStage,
    ) -> Result<Vec<TypeObjectDigest>> {
        self.budget.reserve(types.len())?;
        types
            .iter()
            .map(|ty| self.substitute_at(*ty, substitution, depth, stage))
            .collect()
    }
    fn substitute_fields_at(
        &mut self,
        fields: Vec<StructuralTypeField>,
        substitution: &Substitution,
        depth: usize,
        stage: SignatureStage,
    ) -> Result<Vec<StructuralTypeField>> {
        self.budget.reserve(fields.len())?;
        fields
            .into_iter()
            .map(|field| {
                Ok(StructuralTypeField {
                    name: field.name,
                    ty: self.substitute_at(field.ty, substitution, depth, stage)?,
                })
            })
            .collect()
    }

    /// Declaration schemas are checked independently of instantiated digests.
    /// Own formals are opaque; actuals, even phantom actuals, retain their edges.
    fn schema_graph(&mut self) -> Result<BTreeMap<DeclarationReference, Schema>> {
        self.budget.reserve(self.proof.owners.len())?;
        let declarations: Vec<_> = self
            .proof
            .owners
            .iter()
            .filter_map(|((package, key), owner)| {
                let (OwnerKey::Declaration(id), OwnerRecord::Declaration(record)) = (key, owner)
                else {
                    return None;
                };
                Some((
                    DeclarationReference {
                        package: *package,
                        declaration: *id,
                    },
                    record.clone(),
                ))
            })
            .collect();
        let mut graph = BTreeMap::new();
        for (reference, record) in declarations {
            self.budget.step(0)?;
            let mut roots = Vec::new();
            let category = match record.payload {
                DeclarationPayload::Record { fields, .. }
                | DeclarationPayload::OwnedRecord { fields, .. } => {
                    self.budget.reserve(fields.len())?;
                    for id in fields {
                        let OwnerRecord::Field(field) =
                            self.owner(reference.package, OwnerKey::Field(id))?
                        else {
                            return Err(invalid("missing schema field"));
                        };
                        if field.declaration != reference.declaration {
                            return Err(invalid("schema field names another owner"));
                        }
                        roots.push(field.ty);
                    }
                    if record.header.kind == OwnerKind::OwnedRecord {
                        ValueCategory::Owned
                    } else {
                        ValueCategory::OrdinaryData
                    }
                }
                DeclarationPayload::Variant { cases, .. } => {
                    self.budget.reserve(cases.len())?;
                    for id in cases {
                        let OwnerRecord::Case(case) =
                            self.owner(reference.package, OwnerKey::Case(id))?
                        else {
                            return Err(invalid("missing schema case"));
                        };
                        if case.declaration != reference.declaration {
                            return Err(invalid("schema case names another owner"));
                        }
                        if let Some(ty) = case.payload {
                            roots.push(ty);
                        }
                    }
                    ValueCategory::OrdinaryData
                }
                DeclarationPayload::ReadView(view) => {
                    self.budget.reserve(view.fields.len())?;
                    roots.extend(view.fields.into_iter().map(|field| field.ty));
                    ValueCategory::ReadView
                }
                _ => continue,
            };
            let context = Context::declaration(reference);
            let mut edges = BTreeSet::new();
            let mut visited = BTreeSet::new();
            for ty in roots {
                self.collect_schema_edges(ty, &context, &mut visited, &mut edges, 0)?;
            }
            self.budget.reserve(1)?;
            graph.insert(reference, Schema { category, edges });
        }
        Ok(graph)
    }
    fn collect_schema_edges(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        visited: &mut BTreeSet<TypeObjectDigest>,
        edges: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<bool> {
        self.budget.step(depth)?;
        if visited.contains(&ty) {
            return Ok(false);
        }
        self.budget.reserve(1)?;
        visited.insert(ty);
        let object = self.object(ty)?;
        let mut view = matches!(
            object.form,
            TypeForm::ReadViewApplied { .. }
                | TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
        );
        match &object.form {
            TypeForm::TypeParameter { parameter } => {
                self.parameter(*parameter, context)?;
            }
            TypeForm::Named { declaration }
            | TypeForm::Applied { declaration, .. }
            | TypeForm::ReadViewApplied { declaration, .. } => {
                self.check_reference(context, *declaration)?;
                if !edges.contains(declaration) {
                    self.budget.reserve(1)?;
                    edges.insert(*declaration);
                }
                let expected = self
                    .declaration(*declaration)?
                    .payload
                    .type_parameters()
                    .len();
                let actual = match &object.form {
                    TypeForm::Applied { arguments, .. }
                    | TypeForm::ReadViewApplied { arguments, .. } => arguments.len(),
                    _ => 0,
                };
                if expected != actual {
                    return Err(invalid("schema nominal actual inventory has wrong arity"));
                }
            }
            _ => {}
        }
        for child in object.child_types() {
            view |= self.collect_schema_edges(child, context, visited, edges, depth + 1)?;
        }
        Ok(view)
    }
    fn check_schema_graph(&mut self, graph: &BTreeMap<DeclarationReference, Schema>) -> Result<()> {
        for (root, schema) in graph {
            self.budget.step(0)?;
            for edge in &schema.edges {
                if !graph.contains_key(edge) {
                    return Err(invalid("nominal schema selects a non-layout declaration"));
                }
            }
            if schema.category == ValueCategory::Owned {
                self.schema_reaches_self(*root, graph)?;
            }
            if schema.category == ValueCategory::ReadView {
                self.schema_acyclic(*root, graph, &mut BTreeSet::new(), &mut BTreeSet::new(), 0)?;
            }
        }
        Ok(())
    }
    fn schema_reaches_self(
        &mut self,
        root: DeclarationReference,
        graph: &BTreeMap<DeclarationReference, Schema>,
    ) -> Result<()> {
        self.budget.reserve(graph[&root].edges.len())?;
        let mut pending: Vec<_> = graph[&root].edges.iter().copied().collect();
        let mut visited = BTreeSet::new();
        while let Some(next) = pending.pop() {
            self.budget.step(0)?;
            if next == root {
                return Err(invalid("cyclic raw Owned declaration schema"));
            }
            if visited.contains(&next) {
                continue;
            }
            self.budget.reserve(1)?;
            visited.insert(next);
            let schema = graph
                .get(&next)
                .ok_or_else(|| invalid("missing referenced nominal schema"))?;
            self.budget.reserve(schema.edges.len())?;
            pending.extend(schema.edges.iter().copied());
        }
        Ok(())
    }
    fn schema_acyclic(
        &mut self,
        root: DeclarationReference,
        graph: &BTreeMap<DeclarationReference, Schema>,
        active: &mut BTreeSet<DeclarationReference>,
        checked: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<()> {
        self.budget.step(depth)?;
        if active.contains(&root) {
            return Err(invalid(
                "cyclic raw view referent/phantom declaration closure",
            ));
        }
        if checked.contains(&root) {
            return Ok(());
        }
        self.budget.reserve(2)?;
        active.insert(root);
        let schema = graph
            .get(&root)
            .ok_or_else(|| invalid("missing referenced nominal schema"))?;
        for edge in &schema.edges {
            self.schema_acyclic(*edge, graph, active, checked, depth + 1)?;
        }
        active.remove(&root);
        checked.insert(root);
        Ok(())
    }
    fn check_view_actual_closure(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        depth: usize,
    ) -> Result<()> {
        let mut edges = BTreeSet::new();
        let view =
            self.collect_schema_edges(ty, context, &mut BTreeSet::new(), &mut edges, depth)?;
        if view {
            let mut active = BTreeSet::new();
            let mut checked = BTreeSet::new();
            let proof = self.proof;
            for edge in edges {
                self.schema_acyclic(edge, &proof.schemas, &mut active, &mut checked, depth)?;
            }
        }
        Ok(())
    }

    fn raw_closure(&mut self, ty: TypeObjectDigest, depth: usize) -> Result<()> {
        self.budget.reserve(1)?;
        let mut pending = vec![(ty, depth)];
        let mut visited = BTreeSet::new();
        while let Some((ty, depth)) = pending.pop() {
            self.budget.step(depth)?;
            if visited.contains(&ty) {
                continue;
            }
            self.budget.reserve(1)?;
            visited.insert(ty);
            let object = self.object(ty)?;
            let children = object.child_types();
            self.budget.reserve(children.len())?;
            pending.extend(children.into_iter().map(|child| (child, depth + 1)));
            match object.form.clone() {
                TypeForm::Named { declaration }
                | TypeForm::Applied { declaration, .. }
                | TypeForm::ReadViewApplied { declaration, .. } => {
                    let declaration = self.declaration(declaration)?;
                    match declaration.payload {
                        DeclarationPayload::Record { fields, .. }
                        | DeclarationPayload::OwnedRecord { fields, .. } => {
                            for field in fields {
                                let OwnerRecord::Field(field) = self.owner(
                                    match &object.form {
                                        TypeForm::Named { declaration }
                                        | TypeForm::Applied { declaration, .. } => {
                                            declaration.package
                                        }
                                        _ => unreachable!(),
                                    },
                                    OwnerKey::Field(field),
                                )?
                                else {
                                    return Err(invalid("missing nominal field"));
                                };
                                self.budget.reserve(1)?;
                                pending.push((field.ty, 0));
                            }
                        }
                        DeclarationPayload::Variant { cases, .. } => {
                            let package = match &object.form {
                                TypeForm::Named { declaration }
                                | TypeForm::Applied { declaration, .. } => declaration.package,
                                _ => unreachable!(),
                            };
                            for case in cases {
                                let OwnerRecord::Case(case) =
                                    self.owner(package, OwnerKey::Case(case))?
                                else {
                                    return Err(invalid("missing nominal case"));
                                };
                                if let Some(payload) = case.payload {
                                    self.budget.reserve(1)?;
                                    pending.push((payload, 0));
                                }
                            }
                        }
                        DeclarationPayload::ReadView(view) => {
                            for field in view.fields {
                                self.budget.reserve(1)?;
                                pending.push((field.ty, 0));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn signature(
        &mut self,
        context: &Context,
        parameters: &[(TypeObjectDigest, ParameterUse)],
        result: CallableResult<u32>,
        effect: &FunctionEffect,
    ) -> Result<()> {
        let mut owned_suffix = false;
        let mut capability_suffix = false;
        for (position, (ty, use_mode)) in parameters.iter().enumerate() {
            let facts = self.facts(*ty, context, 0)?;
            let refuse = |message: &str| {
                let owner = context.owner.map_or_else(
                    || "unowned".to_owned(),
                    |owner| format!("{}/{}", owner.package, owner.declaration),
                );
                invalid(format!(
                    "{message}; owner={owner}; position={position}; type={ty}; category={:?}; use={use_mode:?}",
                    facts.category,
                ))
            };
            match facts.category {
                ValueCategory::ReadView => {
                    let owner = context
                        .owner
                        .ok_or_else(|| invalid("view signature has no exact owner"))?;
                    if self.proof.declaration(owner)?.header.contract_version
                        < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
                    {
                        return Err(invalid(
                            "view signature requires its exact owner generation34",
                        ));
                    }
                    if capability_suffix {
                        return Err(refuse(
                            "view signature parameter follows the capability suffix",
                        ));
                    }
                    if owned_suffix {
                        return Err(refuse("view signature parameter follows the Owned suffix"));
                    }
                    if *use_mode != ParameterUse::Borrow {
                        return Err(refuse("view signature parameter is not borrowed"));
                    }
                }
                ValueCategory::Owned => {
                    if capability_suffix {
                        return Err(refuse(
                            "Owned signature parameter follows the capability suffix",
                        ));
                    }
                    owned_suffix = true;
                    if *use_mode == ParameterUse::Unrestricted {
                        return Err(refuse("Owned signature parameter is unrestricted"));
                    }
                }
                ValueCategory::Capability => {
                    if !matches!(context.binder, Some(ReadScopeBinder::Declaration(owner))
                    if matches!(&self.proof.declaration(owner)?.payload,
                        DeclarationPayload::Function(FunctionDeclaration {
                            effect: FunctionEffect::Task { .. }, ..
                        })))
                    {
                        return Err(refuse(
                            "capability parameter requires exact named task function",
                        ));
                    }
                    if *use_mode == ParameterUse::Unrestricted {
                        return Err(refuse("capability parameter must borrow or consume"));
                    }
                    capability_suffix = true;
                }
                _ if owned_suffix
                    || capability_suffix
                    || *use_mode != ParameterUse::Unrestricted =>
                {
                    return Err(refuse(
                        "signature ordinary/affine parameter order or use changed",
                    ));
                }
                _ => {}
            }
        }
        let result_facts = self.facts(result.ty, context, 0)?;
        if result_facts.category == ValueCategory::ReadView {
            let owner = context
                .owner
                .ok_or_else(|| invalid("view result signature owner absent"))?;
            if self.proof.declaration(owner)?.header.contract_version
                < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
            {
                return Err(invalid(
                    "view result signature requires its exact owner generation34",
                ));
            }
        }
        match result.relation {
            ResultRelation::Value => {
                if result_facts.category == ValueCategory::ReadView {
                    return Err(invalid("Value result carries a read view"));
                }
            }
            ResultRelation::ReadFrom { source } | ResultRelation::ViewFrom { source, .. } => {
                let (source_ty, use_mode) = parameters.get(source as usize).ok_or_else(|| {
                    invalid("result source is outside exact ordered parameter inventory")
                })?;
                let source_facts = self.facts(*source_ty, context, 0)?;
                if *effect != FunctionEffect::Pure
                    || *use_mode != ParameterUse::Borrow
                    || !matches!(
                        source_facts.category,
                        ValueCategory::Owned | ValueCategory::ReadView
                    )
                {
                    return Err(invalid(
                        "loan result requires a pure named callable's borrowed affine source",
                    ));
                }
                match result.relation {
                    ResultRelation::ReadFrom { .. }
                        if result_facts.category == ValueCategory::Owned => {}
                    ResultRelation::ViewFrom { scope, .. }
                        if result_facts.category == ValueCategory::ReadView =>
                    {
                        self.scope(scope, context)?;
                        if self.view_scope(result.ty)? != scope {
                            return Err(invalid(
                                "ViewFrom result scope differs from its complete type identity",
                            ));
                        }
                        let binder = context
                            .binder
                            .ok_or_else(|| invalid("view result lacks exact binder"))?;
                        if !self
                            .proof
                            .scope_sources(binder)?
                            .iter()
                            .any(|s| s.source == source && s.scope == scope)
                        {
                            return Err(invalid(
                                "ViewFrom result lacks exact source/scope relationship",
                            ));
                        }
                    }
                    _ => {
                        return Err(invalid(
                            "result kind changes its positive Owned/view category",
                        ));
                    }
                }
            }
        }
        if let Some(binder) = context.binder
            && let Some(sources) = self.proof.sources.get(&binder)
        {
            for relationship in sources {
                let (ty, use_mode) = parameters
                    .get(relationship.source as usize)
                    .ok_or_else(|| invalid("scope source position absent"))?;
                let facts = self.facts(*ty, context, 0)?;
                if *use_mode != ParameterUse::Borrow
                    || !matches!(
                        facts.category,
                        ValueCategory::Owned | ValueCategory::ReadView
                    )
                {
                    return Err(invalid(
                        "scope relationship names an unborrowed or ordinary parameter",
                    ));
                }
                if facts.category == ValueCategory::ReadView
                    && self.view_scope(*ty)? != relationship.scope
                {
                    return Err(invalid(
                        "view parameter source inventory changes its carried scope",
                    ));
                }
            }
        }
        Ok(())
    }
    fn function_signature(
        &mut self,
        reference: DeclarationReference,
        function: &FunctionDeclaration,
    ) -> Result<(Vec<(TypeObjectDigest, ParameterUse)>, CallableResult<u32>)> {
        self.function_rows(reference, function)?;
        unique(&function.parameters, "function parameters")?;
        let mut parameters = Vec::new();
        let mut names = BTreeSet::new();
        let context = Context::declaration(reference);
        self.budget.reserve(function.parameters.len())?;
        for (position, parameter) in function.parameters.iter().enumerate() {
            self.budget.step(0)?;
            let OwnerRecord::Parameter(record) =
                self.owner(reference.package, OwnerKey::Parameter(*parameter))?
            else {
                return Err(invalid("function parameter inventory is incomplete"));
            };
            if record.header.kind != OwnerKind::Parameter
                || record.parent != ParameterParent::Function(reference.declaration)
                || !names.insert(record.name)
            {
                return Err(invalid(
                    "function parameter belongs to another callable or duplicates its name",
                ));
            }
            // This scalar read selects the direct form only. The complete raw
            // closure below independently checks the same exact type object.
            let interface = match &self
                .derived
                .get(&record.ty)
                .or_else(|| self.input.get(&record.ty))
                .or_else(|| self.proof.types.get(&record.ty))
                .ok_or_else(|| invalid("function parameter canonical type absent"))?
                .form
            {
                TypeForm::CapabilityResource { interface } => Some(*interface),
                _ => None,
            };
            let free = self.resource_free(record.ty, &context, 0)?;
            let refuse = |message: &str| {
                invalid(format!(
                    "{message}; owner={}/{}; parameter={parameter}; position={position}; type={}; use={:?}",
                    reference.package, reference.declaration, record.ty, record.use_mode,
                ))
            };
            if let Some(interface) = interface {
                if !matches!(function.effect, FunctionEffect::Task { .. }) {
                    return Err(refuse(
                        "capability parameter requires exact named task function",
                    ));
                }
                if record.use_mode == ParameterUse::Unrestricted {
                    return Err(refuse("capability parameter must borrow or consume"));
                }
                if !function.requirement_parameters.is_empty() {
                    return Err(refuse(
                        "resource-bearing function cannot declare requirement formals",
                    ));
                }
                let requirement = record.resource_requirement.ok_or_else(|| {
                    refuse("capability parameter lacks exact concrete requirement binding")
                })?;
                let operand = kernel::RequirementOperand::Concrete(requirement);
                let mut listed = false;
                if let FunctionEffect::Task { requirements, .. } = &function.effect {
                    for candidate in requirements {
                        self.budget.step(0)?;
                        if *candidate == operand {
                            listed = true;
                            break;
                        }
                    }
                }
                if !listed {
                    return Err(refuse(
                        "capability parameter binding is absent from exact function effect",
                    ));
                }
                let constraint = self.requirement_constraint(operand, &context)?;
                if constraint.interface != interface {
                    return Err(refuse(
                        "capability parameter interface differs from exact requirement",
                    ));
                }
            } else {
                if record.resource_requirement.is_some() {
                    return Err(refuse("nonresource parameter binds a resource requirement"));
                }
                if !free {
                    return Err(refuse(
                        "function parameter contains a forbidden capability container",
                    ));
                }
            }
            parameters.push((record.ty, record.use_mode));
        }
        if !self.resource_free(function.result.ty, &context, 0)? {
            return Err(invalid(format!(
                "function result contains a capability resource; owner={}/{}; type={}",
                reference.package, reference.declaration, function.result.ty,
            )));
        }
        let result = CallableResult {
            ty: function.result.ty,
            relation: function
                .result
                .relation
                .try_map_source(|source| {
                    function
                        .parameters
                        .iter()
                        .position(|p| *p == source)
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| {
                            crate::platform::diagnostic::Diagnostic::new(
                                crate::platform::diagnostic::DiagnosticClass::Semantic,
                                "reference_read_view_source",
                                "source parameter is not in this callable",
                            )
                        })
                })
                .map_err(|_| invalid("borrowed function result names another callable's source"))?,
        };
        Ok((parameters, result))
    }
    fn admit_inventory(&mut self) -> Result<()> {
        let declarations: Vec<_> = self
            .proof
            .owners
            .iter()
            .filter_map(|((package, key), record)| {
                if let (OwnerKey::Declaration(id), OwnerRecord::Declaration(record)) = (key, record)
                {
                    Some((
                        DeclarationReference {
                            package: *package,
                            declaration: *id,
                        },
                        record.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        self.budget.reserve(declarations.len())?;
        for (reference, declaration) in declarations {
            self.budget.step(0)?;
            let expected_kind = match &declaration.payload {
                DeclarationPayload::ReadView(_) => OwnerKind::ReadView,
                DeclarationPayload::Function(function) => match function.effect {
                    FunctionEffect::Pure => OwnerKind::PureFunction,
                    FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
                },
                DeclarationPayload::External(_) => OwnerKind::External,
                DeclarationPayload::OwnedContract(_) => OwnerKind::OwnedContract,
                DeclarationPayload::OwnedImplementation(_) => OwnerKind::OwnedImplementation,
                DeclarationPayload::Record { .. } => OwnerKind::Record,
                DeclarationPayload::OwnedRecord { .. } => OwnerKind::OwnedRecord,
                DeclarationPayload::Variant { .. } => OwnerKind::Variant,
                _ => declaration.header.kind,
            };
            if declaration.header.kind != expected_kind {
                return Err(invalid(
                    "declaration header changes the positive callable/category kind",
                ));
            }
            let context = Context::declaration(reference);
            let parameters = declaration.payload.type_parameters().to_vec();
            unique(&parameters, "declaration type parameters")?;
            let mut type_arguments = Vec::new();
            let mut names = BTreeSet::new();
            for parameter in parameters {
                self.parameter(parameter, &context)?;
                let OwnerRecord::TypeParameter(record) =
                    self.owner(reference.package, OwnerKey::TypeParameter(parameter))?
                else {
                    return Err(invalid("missing type formal"));
                };
                if !names.insert(record.name) {
                    return Err(invalid("duplicate type formal name"));
                }
                type_arguments.push(self.intern(TypeForm::TypeParameter { parameter })?);
            }
            match declaration.payload {
                DeclarationPayload::ReadView(view) => {
                    self.view_inventory(reference, &view)?;
                    // Inline fields have no separate OwnerRecord occurrence. Keep
                    // their defining imports strict before property preparation.
                    // Stored Field/Case roots are checked later in this same full
                    // inventory pass; no prepared cache is exposed before it ends.
                    for field in &view.fields {
                        self.facts(field.ty, &context, 0)?;
                    }
                    let read_scope = ReadScopeArgument::Parameter(ReadScopeReference {
                        binder: ReadScopeBinder::Declaration(reference),
                        parameter: view.scope_parameter.id,
                    });
                    let ty = self.intern(TypeForm::ReadViewApplied {
                        declaration: reference,
                        arguments: type_arguments,
                        read_scope,
                    })?;
                    self.facts(ty, &context, 0)?;
                }
                DeclarationPayload::Function(function) => {
                    let (parameters, result) = self.function_signature(reference, &function)?;
                    self.signature(&context, &parameters, result, &function.effect)?;
                    self.prerequisites(reference, &function.implementation_parameters, &context)?;
                }
                DeclarationPayload::External(function) => {
                    for parameter in function.parameters {
                        let OwnerRecord::Parameter(parameter) =
                            self.owner(reference.package, OwnerKey::Parameter(parameter))?
                        else {
                            return Err(invalid("external parameter absent"));
                        };
                        if self.facts(parameter.ty, &context, 0)?.category
                            == ValueCategory::ReadView
                        {
                            return Err(invalid("external boundary accepts a view"));
                        }
                    }
                    if self.facts(function.result, &context, 0)?.category == ValueCategory::ReadView
                    {
                        return Err(invalid("external boundary returns a view"));
                    }
                }
                DeclarationPayload::OwnedContract(contract) => {
                    let mut methods = BTreeSet::new();
                    let mut names = BTreeSet::new();
                    if contract.methods.is_empty() {
                        return Err(invalid("contract method inventory is empty"));
                    }
                    for method in &contract.methods {
                        self.budget.step(0)?;
                        if !methods.insert(method.id) || !names.insert(&method.name) {
                            return Err(invalid("duplicate contract method identity/name"));
                        }
                        let context = Context::binder(ReadScopeBinder::ContractMethod {
                            contract: reference,
                            method: method.id,
                        });
                        let parameters: Vec<_> = method
                            .parameters
                            .iter()
                            .map(|p| (p.ty, p.use_mode))
                            .collect();
                        self.signature(&context, &parameters, method.result, &method.effect)?;
                    }
                    let mut members = BTreeSet::new();
                    let mut names = BTreeSet::new();
                    for member in &contract.associated_types {
                        if !members.insert(member.id) || !names.insert(&member.name) {
                            return Err(invalid("duplicate associated member identity/name"));
                        }
                    }
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    self.implementation_inventory(reference, &implementation)?
                }
                DeclarationPayload::Record { fields, .. }
                | DeclarationPayload::OwnedRecord { fields, .. } => {
                    unique(&fields, "nominal field inventory")?;
                    let form = if type_arguments.is_empty() {
                        TypeForm::Named {
                            declaration: reference,
                        }
                    } else {
                        TypeForm::Applied {
                            declaration: reference,
                            arguments: type_arguments,
                        }
                    };
                    let ty = self.intern(form)?;
                    self.facts(ty, &context, 0)?;
                }
                DeclarationPayload::Variant { cases, .. } => {
                    unique(&cases, "nominal case inventory")?;
                    let form = if type_arguments.is_empty() {
                        TypeForm::Named {
                            declaration: reference,
                        }
                    } else {
                        TypeForm::Applied {
                            declaration: reference,
                            arguments: type_arguments,
                        }
                    };
                    let ty = self.intern(form)?;
                    self.facts(ty, &context, 0)?;
                }
                DeclarationPayload::Constant { ty, .. }
                    if self.facts(ty, &context, 0)?.category == ValueCategory::ReadView =>
                {
                    return Err(invalid("constant stores a view"));
                }
                _ => {}
            }
        }
        let records: Vec<_> = self
            .proof
            .owners
            .iter()
            .map(|(key, record)| (*key, record.clone()))
            .collect();
        for ((package, key), record) in records {
            match record {
                OwnerRecord::TypeParameter(parameter) => {
                    let OwnerKey::TypeParameter(id) = key else {
                        return Err(invalid("type formal key changes identity domain"));
                    };
                    self.parameter(
                        id,
                        &Context::declaration(DeclarationReference {
                            package,
                            declaration: parameter.declaration,
                        }),
                    )?;
                }
                OwnerRecord::Field(field) => {
                    let OwnerKey::Field(id) = key else {
                        return Err(invalid("field key changes identity domain"));
                    };
                    let reference = DeclarationReference {
                        package,
                        declaration: field.declaration,
                    };
                    let declaration = self.declaration(reference)?;
                    let fields = match declaration.payload {
                        DeclarationPayload::Record { fields, .. }
                        | DeclarationPayload::OwnedRecord { fields, .. } => fields,
                        _ => {
                            return Err(invalid(
                                "ordinary Field owner is attached to a non-record/view declaration",
                            ));
                        }
                    };
                    if field.header.kind != OwnerKind::Field || !fields.contains(&id) {
                        return Err(invalid(
                            "unused field is missing from its complete declaration inventory",
                        ));
                    }
                    if self
                        .facts(field.ty, &Context::declaration(reference), 0)?
                        .category
                        == ValueCategory::ReadView
                    {
                        return Err(invalid("unused ordinary/Owned Field owner contains a view"));
                    }
                }
                OwnerRecord::Case(case) => {
                    let OwnerKey::Case(id) = key else {
                        return Err(invalid("case key changes identity domain"));
                    };
                    let reference = DeclarationReference {
                        package,
                        declaration: case.declaration,
                    };
                    let DeclarationPayload::Variant { cases, .. } =
                        self.declaration(reference)?.payload
                    else {
                        return Err(invalid("case owner attached to a non-variant"));
                    };
                    if !cases.contains(&id) {
                        return Err(invalid(
                            "unused case is missing from its complete declaration inventory",
                        ));
                    }
                    if let Some(ty) = case.payload
                        && self
                            .facts(ty, &Context::declaration(reference), 0)?
                            .category
                            == ValueCategory::ReadView
                    {
                        return Err(invalid("unused ordinary case payload contains a view"));
                    }
                }
                OwnerRecord::Parameter(parameter) => {
                    let OwnerKey::Parameter(id) = key else {
                        return Err(invalid("parameter key changes identity domain"));
                    };
                    let included = match parameter.parent {
                        ParameterParent::Function(declaration) => match self
                            .declaration(DeclarationReference {
                                package,
                                declaration,
                            })?
                            .payload
                        {
                            DeclarationPayload::Function(function) => {
                                function.parameters.contains(&id)
                            }
                            DeclarationPayload::External(function) => {
                                function.parameters.contains(&id)
                            }
                            _ => false,
                        },
                        ParameterParent::Operation(operation) => match self
                            .owner(package, OwnerKey::Operation(operation))?
                        {
                            OwnerRecord::Operation(operation) => operation.parameters.contains(&id),
                            _ => false,
                        },
                    };
                    if !included {
                        return Err(invalid(
                            "unused parameter is missing from its complete exact parent inventory",
                        ));
                    }
                }
                OwnerRecord::Operation(operation) => {
                    let context = Context::declaration(DeclarationReference {
                        package,
                        declaration: operation.declaration,
                    });
                    if self.facts(operation.result, &context, 0)?.category
                        == ValueCategory::ReadView
                    {
                        return Err(invalid("capability operation result carries a view"));
                    }
                    for parameter in operation.parameters {
                        let OwnerRecord::Parameter(parameter) =
                            self.owner(package, OwnerKey::Parameter(parameter))?
                        else {
                            return Err(invalid("operation parameter missing"));
                        };
                        if self.facts(parameter.ty, &context, 0)?.category
                            == ValueCategory::ReadView
                        {
                            return Err(invalid("capability operation parameter carries a view"));
                        }
                    }
                }
                OwnerRecord::Port(port) => {
                    let context = Context::declaration(DeclarationReference {
                        package,
                        declaration: port.declaration,
                    });
                    self.facts(port.function_type, &context, 0)?;
                    if let kernel::PortImplementation::Function(function) = port.implementation
                        && let DeclarationPayload::Function(function) =
                            self.declaration(function)?.payload
                    {
                        if function.result.relation != ResultRelation::Value {
                            return Err(invalid(
                                "component entrypoint names a borrowed-result function",
                            ));
                        }
                        if function.parameters.iter().any(|parameter| matches!(self.proof.owners.get(&(package, OwnerKey::Parameter(*parameter))), Some(OwnerRecord::Parameter(p)) if self.proof.types.get(&p.ty).is_some_and(|t| t.form.is_read_view()))) {
                                return Err(invalid("component entrypoint accepts a view"));
                            }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn prerequisites(
        &mut self,
        owner: DeclarationReference,
        prerequisites: &[kernel::ImplementationParameter],
        context: &Context,
    ) -> Result<()> {
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        if prerequisites.len() > kernel::contract::MAXIMUM_CHILDREN {
            return Err(invalid("prerequisite inventory exceeds bound"));
        }
        for prerequisite in prerequisites {
            self.budget.step(0)?;
            if !ids.insert(prerequisite.id) || !names.insert(&prerequisite.name) {
                return Err(invalid("duplicate prerequisite identity/name"));
            }
            self.contract_application(
                prerequisite.contract,
                prerequisite.self_type,
                &prerequisite.type_arguments,
                context,
            )?;
        }
        let _ = owner;
        Ok(())
    }
    fn contract_application(
        &mut self,
        contract: DeclarationReference,
        self_type: TypeObjectDigest,
        arguments: &[TypeObjectDigest],
        context: &Context,
    ) -> Result<Substitution> {
        let declaration = self.declaration(contract)?;
        let DeclarationPayload::OwnedContract(contract_record) = declaration.payload else {
            return Err(invalid("witness contract is not an Owned contract"));
        };
        if self.facts(self_type, context, 0)?.category != ValueCategory::Owned {
            return Err(invalid("contract Self is not independently Owned"));
        }
        if arguments.len() != contract_record.type_parameters.len() {
            return Err(invalid("contract argument arity changed"));
        }
        let actuals: Vec<_> = std::iter::once(self_type)
            .chain(arguments.iter().copied())
            .collect();
        self.bind_types(contract, &actuals, context)
    }
    fn operand(
        &mut self,
        operand: &ImplementationOperand,
        context: &Context,
        depth: usize,
    ) -> Result<(
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    )> {
        self.budget.step(depth)?;
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if context.owner != Some(*scope) {
                    return Err(invalid("witness operand imports a foreign lexical binder"));
                }
                let declaration = self.declaration(*scope)?;
                let parameters = match declaration.payload {
                    DeclarationPayload::Function(f) => f.implementation_parameters,
                    DeclarationPayload::OwnedImplementation(i) => i.implementation_parameters,
                    _ => return Err(invalid("witness has no lexical inventory")),
                };
                let p = parameters
                    .iter()
                    .find(|p| p.id == *parameter)
                    .ok_or_else(|| invalid("unused witness formal is absent"))?;
                self.contract_application(p.contract, p.self_type, &p.type_arguments, context)?;
                Ok((p.contract, p.self_type, p.type_arguments.clone()))
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let declaration = self.declaration(*implementation)?;
                let DeclarationPayload::OwnedImplementation(record) = declaration.payload else {
                    return Err(invalid(
                        "concrete witness selects nonimplementation declaration",
                    ));
                };
                let mut substitution = self.bind_types(*implementation, type_arguments, context)?;
                if implementations.len() != record.implementation_parameters.len() {
                    return Err(invalid("concrete witness omits unused prerequisites"));
                }
                for (formal, actual) in record.implementation_parameters.iter().zip(implementations)
                {
                    let got = self.operand(actual, context, depth + 1)?;
                    let expected_self =
                        self.substitute(formal.self_type, &substitution, depth + 1)?;
                    let expected_arguments =
                        self.substitute_many(&formal.type_arguments, &substitution, depth + 1)?;
                    if got != (formal.contract, expected_self, expected_arguments) {
                        return Err(invalid(
                            "prerequisite witness application changed exact contract/Self/actuals",
                        ));
                    }
                    substitution.witnesses.insert(
                        (*implementation, formal.id),
                        self.copy_operand(actual, depth + 1)?,
                    );
                }
                let self_type = self.substitute(record.self_type, &substitution, depth + 1)?;
                let arguments =
                    self.substitute_many(&record.type_arguments, &substitution, depth + 1)?;
                self.contract_application(record.contract, self_type, &arguments, context)?;
                Ok((record.contract, self_type, arguments))
            }
        }
    }
    fn select_member(
        &mut self,
        operand: &ImplementationOperand,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        read_scope: Option<ReadScopeArgument>,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.budget.step(depth)?;
        self.member(contract, member, read_scope.is_some())?;
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                self.intern(match read_scope {
                    Some(read_scope) => TypeForm::AssociatedReadViewProjection {
                        scope: *scope,
                        parameter: *parameter,
                        contract,
                        member,
                        read_scope,
                    },
                    None => TypeForm::AssociatedWitnessProjection {
                        scope: *scope,
                        parameter: *parameter,
                        contract,
                        member,
                    },
                })
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let declaration = self.declaration(*implementation)?;
                let DeclarationPayload::OwnedImplementation(record) = declaration.payload else {
                    return Err(invalid("associated selection has no implementation"));
                };
                if record.contract != contract
                    || record.type_parameters.len() != type_arguments.len()
                    || record.implementation_parameters.len() != implementations.len()
                {
                    return Err(invalid(
                        "associated selection changed complete witness application",
                    ));
                }
                let mut substitution = Substitution::default();
                for (formal, actual) in record.type_parameters.iter().zip(type_arguments) {
                    substitution
                        .types
                        // Operand applications were admitted in the caller.
                        // Copy whole actuals; a callee E/R/T map cannot run on
                        // them again, including recursive same-ID permutations.
                        .insert(*formal, *actual);
                }
                for (formal, actual) in record.implementation_parameters.iter().zip(implementations)
                {
                    substitution.witnesses.insert(
                        (*implementation, formal.id),
                        self.copy_operand(actual, depth + 1)?,
                    );
                }
                let binding = record
                    .associated_types
                    .iter()
                    .find(|b| b.member == member)
                    .ok_or_else(|| invalid("selected associated binding absent"))?;
                match (&binding.value, read_scope) {
                    (AssociatedTypeBindingValue::Owned { ty }, None) => {
                        self.substitute(*ty, &substitution, depth + 1)
                    }
                    (
                        AssociatedTypeBindingValue::ReadViewFamily {
                            scope_parameter,
                            ty,
                        },
                        Some(scope),
                    ) => {
                        substitution.scopes.insert(
                            ReadScopeReference {
                                binder: ReadScopeBinder::AssociatedBinding {
                                    implementation: *implementation,
                                    member,
                                },
                                parameter: scope_parameter.id,
                            },
                            scope,
                        );
                        self.substitute(*ty, &substitution, depth + 1)
                    }
                    _ => Err(invalid("associated member binding kind/arity swap")),
                }
            }
        }
    }
    fn reserve_operand_copy(
        &mut self,
        operand: &ImplementationOperand,
        depth: usize,
    ) -> Result<()> {
        self.budget.step(depth)?;
        if let ImplementationOperand::Concrete {
            type_arguments,
            implementations,
            ..
        } = operand
        {
            self.budget
                .reserve(type_arguments.len() + implementations.len())?;
            for _ in type_arguments {
                self.budget.step(depth)?;
            }
            for operand in implementations {
                self.reserve_operand_copy(operand, depth + 1)?;
            }
        }
        Ok(())
    }
    fn copy_operand(
        &mut self,
        operand: &ImplementationOperand,
        depth: usize,
    ) -> Result<ImplementationOperand> {
        self.reserve_operand_copy(operand, depth)?;
        Ok(operand.clone())
    }
    fn copy_substitution(&mut self, source: &Substitution, depth: usize) -> Result<Substitution> {
        self.budget.reserve(
            source.types.len()
                + source.effects.len()
                + source.requirements.len()
                + source.scopes.len()
                + source.members.len()
                + source.witnesses.len(),
        )?;
        for _ in source
            .types
            .keys()
            .map(|_| ())
            .chain(source.requirements.keys().map(|_| ()))
            .chain(source.scopes.keys().map(|_| ()))
            .chain(source.members.keys().map(|_| ()))
        {
            self.budget.step(depth)?;
        }
        for row in source.effects.values() {
            self.budget.step(depth)?;
            self.budget
                .reserve(row.requirements.len() + row.parameters.len())?;
            for _ in row
                .requirements
                .iter()
                .map(|_| ())
                .chain(row.parameters.iter().map(|_| ()))
            {
                self.budget.step(depth)?;
            }
        }
        for operand in source.witnesses.values() {
            self.reserve_operand_copy(operand, depth + 1)?;
        }
        Ok(source.clone())
    }
    fn implementation_inventory(
        &mut self,
        reference: DeclarationReference,
        implementation: &kernel::OwnedImplementation,
    ) -> Result<()> {
        let context = Context::declaration(reference);
        self.prerequisites(
            reference,
            &implementation.implementation_parameters,
            &context,
        )?;
        let mut contract_substitution = self.contract_application(
            implementation.contract,
            implementation.self_type,
            &implementation.type_arguments,
            &context,
        )?;
        let declaration = self.declaration(implementation.contract)?;
        let DeclarationPayload::OwnedContract(contract) = declaration.payload else {
            return Err(invalid("implementation contract absent"));
        };
        if implementation.associated_types.len() != contract.associated_types.len()
            || implementation
                .associated_types
                .windows(2)
                .any(|p| p[0].member >= p[1].member)
        {
            return Err(invalid(
                "associated binding inventory is incomplete or not strictly ordered",
            ));
        }
        for member in &contract.associated_types {
            let binding = implementation
                .associated_types
                .iter()
                .find(|b| b.member == member.id)
                .ok_or_else(|| invalid("missing unused associated binding"))?;
            match (&member.bound, &binding.value) {
                (AssociatedTypeBound::Owned, AssociatedTypeBindingValue::Owned { ty }) => {
                    if self.facts(*ty, &context, 0)?.category != ValueCategory::Owned {
                        return Err(invalid("associated Owned binding is not Owned"));
                    }
                    self.projection_free(*ty, 0)?;
                    contract_substitution
                        .members
                        .insert((implementation.contract, member.id), (*ty, None));
                }
                (
                    AssociatedTypeBound::ReadViewFamily { .. },
                    AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter,
                        ty,
                    },
                ) => {
                    let binder = ReadScopeBinder::AssociatedBinding {
                        implementation: reference,
                        member: member.id,
                    };
                    let formal = ReadScopeReference {
                        binder,
                        parameter: scope_parameter.id,
                    };
                    let TypeForm::ReadViewApplied { read_scope, .. } = self.object(*ty)?.form
                    else {
                        return Err(invalid(
                            "family RHS is not a nominal view application template",
                        ));
                    };
                    if read_scope != ReadScopeArgument::Parameter(formal) {
                        return Err(invalid("family RHS uses another binding's scope formal"));
                    }
                    self.facts(*ty, &Context::binder(binder), 0)?;
                    self.projection_free(*ty, 0)?;
                    contract_substitution
                        .members
                        .insert((implementation.contract, member.id), (*ty, Some(formal)));
                }
                _ => return Err(invalid("binding changes associated member kind/arity")),
            }
        }
        if implementation.methods.len() != contract.methods.len()
            || implementation
                .methods
                .windows(2)
                .any(|p| p[0].method >= p[1].method)
        {
            return Err(invalid(
                "implementation method inventory incomplete or unordered",
            ));
        }
        for method in &contract.methods {
            let mapped = implementation
                .methods
                .iter()
                .find(|m| m.method == method.id)
                .ok_or_else(|| invalid("unused mapped method omitted"))?;
            let mapped_declaration = self.declaration(mapped.function)?;
            let DeclarationPayload::Function(function) = mapped_declaration.payload else {
                return Err(invalid("method mapping requires a named graph function"));
            };
            let method_context = Context::binder(ReadScopeBinder::ContractMethod {
                contract: implementation.contract,
                method: method.id,
            });
            let mut expected = self.copy_substitution(&contract_substitution, 0)?;
            if function.read_scope_parameters.len() != mapped.read_scope_arguments.len() {
                return Err(invalid("mapped function scope argument arity changed"));
            }
            let mut actual = self.bind_types(mapped.function, &mapped.type_arguments, &context)?;
            for (formal, scope) in function
                .read_scope_parameters
                .iter()
                .zip(&mapped.read_scope_arguments)
            {
                self.scope(*scope, &method_context)?;
                actual.scopes.insert(
                    ReadScopeReference {
                        binder: ReadScopeBinder::Declaration(mapped.function),
                        parameter: formal.id,
                    },
                    *scope,
                );
            }
            if function.implementation_parameters.len() != mapped.implementations.len() {
                return Err(invalid(
                    "mapped method omits unused function witness actuals",
                ));
            }
            for (formal, operand) in function
                .implementation_parameters
                .iter()
                .zip(&mapped.implementations)
            {
                let got = self.operand(operand, &context, 0)?;
                if got
                    != (
                        formal.contract,
                        self.substitute(formal.self_type, &actual, 0)?,
                        self.substitute_many(&formal.type_arguments, &actual, 0)?,
                    )
                {
                    return Err(invalid(
                        "mapped function witness actual changes prerequisite",
                    ));
                }
                actual
                    .witnesses
                    .insert((mapped.function, formal.id), operand.clone());
            }
            // Contract type formals and implementation RHS formals have separate
            // owners. Binding templates were already checked in the latter.
            for parameter in &implementation.type_parameters {
                expected.types.insert(
                    *parameter,
                    self.intern(TypeForm::TypeParameter {
                        parameter: *parameter,
                    })?,
                );
            }
            let (parameters, result) = self.function_signature(mapped.function, &function)?;
            if parameters.len() != method.parameters.len() || function.effect != method.effect {
                return Err(invalid("mapped method parameter order/effects differ"));
            }
            for ((ty, use_mode), method_parameter) in parameters.iter().zip(&method.parameters) {
                if *use_mode != method_parameter.use_mode
                    || self.substitute(*ty, &actual, 0)?
                        != self.substitute(method_parameter.ty, &expected, 0)?
                {
                    return Err(invalid(
                        "mapped method changes complete parameter category/type/use",
                    ));
                }
            }
            if self.substitute(result.ty, &actual, 0)?
                != self.substitute(method.result.ty, &expected, 0)?
                || substitute_relation(result.relation, &actual) != method.result.relation
            {
                return Err(invalid(
                    "mapped method changes result kind/type/source/scope",
                ));
            }
            let got_sources: Vec<_> = function
                .read_scope_sources
                .iter()
                .map(|s| {
                    let source = function
                        .parameters
                        .iter()
                        .position(|p| *p == s.source)
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| invalid("mapped source missing"))?;
                    Ok(ReadScopeSource {
                        source,
                        scope: substitute_scope(s.scope, &actual),
                    })
                })
                .collect::<Result<_>>()?;
            if got_sources != method.read_scope_sources {
                return Err(invalid(
                    "mapped method changes unused source/scope inventory",
                ));
            }
        }
        Ok(())
    }
    fn projection_free(&mut self, ty: TypeObjectDigest, depth: usize) -> Result<()> {
        self.budget.step(depth)?;
        let object = self.object(ty)?;
        if matches!(
            object.form,
            TypeForm::AssociatedContractMember { .. }
                | TypeForm::AssociatedWitnessProjection { .. }
                | TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
        ) {
            return Err(invalid(
                "stored associated binding RHS contains an associated equation/projection",
            ));
        }
        for child in object.child_types() {
            self.projection_free(child, depth + 1)?;
        }
        Ok(())
    }

    fn admit_all_expressions(
        &mut self,
    ) -> Result<(
        BTreeMap<(PackageId, BindingId), DeclarationReference>,
        BTreeSet<DeclarationReference>,
    )> {
        let mut syntax = Syntax::default();
        let declarations: Vec<_> = self
            .proof
            .owners
            .iter()
            .filter_map(|((package, key), record)| {
                if let (OwnerKey::Declaration(id), OwnerRecord::Declaration(record)) = (key, record)
                {
                    Some((
                        DeclarationReference {
                            package: *package,
                            declaration: *id,
                        },
                        record.payload.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        self.budget.reserve(declarations.len())?;
        for (reference, payload) in declarations {
            let context = Context::declaration(reference);
            match payload {
                DeclarationPayload::Function(function) => {
                    let mut locals = Locals::new();
                    for (position, parameter) in function.parameters.iter().enumerate() {
                        let OwnerRecord::Parameter(record) =
                            self.owner(reference.package, OwnerKey::Parameter(*parameter))?
                        else {
                            return Err(invalid("function parameter absent"));
                        };
                        let value = LocalValueReference::FunctionParameter(*parameter);
                        let mut scopes: BTreeSet<_> = self
                            .proof
                            .scope_sources(ReadScopeBinder::Declaration(reference))?
                            .iter()
                            .filter(|s| s.source as usize == position)
                            .map(|s| s.scope)
                            .collect();
                        if self.facts(record.ty, &context, 0)?.category == ValueCategory::ReadView {
                            scopes.insert(self.view_scope(record.ty)?);
                            syntax.view_functions.insert(reference);
                        }
                        self.budget.reserve(1)?;
                        locals.insert(
                            value,
                            Local {
                                ty: record.ty,
                                source: value,
                                scopes,
                                read: record.use_mode == ParameterUse::Borrow,
                                live: true,
                            },
                        );
                    }
                    let value = self.scan(
                        function.body,
                        &context,
                        &mut locals,
                        &mut syntax,
                        Use::Return,
                        0,
                    )?;
                    if self.value_category(&value, &context)? == ValueCategory::ReadView {
                        syntax.view_functions.insert(reference);
                    }
                    if value.ty != Some(function.result.ty) {
                        return Err(invalid("function body/result canonical types differ"));
                    }
                    match function.result.relation {
                        ResultRelation::Value => {
                            if self.value_category(&value, &context)? == ValueCategory::ReadView
                                || value.read
                            {
                                return Err(invalid(
                                    "owning/Value function result carries a view or loan",
                                ));
                            }
                        }
                        ResultRelation::ReadFrom { source } => {
                            if !value.read
                                || value.source
                                    != Some(LocalValueReference::FunctionParameter(source))
                                || self.value_category(&value, &context)? != ValueCategory::Owned
                            {
                                return Err(invalid(
                                    "ReadFrom body derives from another source or an owning/local value",
                                ));
                            }
                        }
                        ResultRelation::ViewFrom { source, scope } => {
                            if value.read
                                || value.source
                                    != Some(LocalValueReference::FunctionParameter(source))
                                || self.value_category(&value, &context)? != ValueCategory::ReadView
                                || !value.scopes.contains(&scope)
                            {
                                return Err(invalid(
                                    "ViewFrom body changes exact parameter lineage/scope or escapes a local owner",
                                ));
                            }
                        }
                    }
                }
                DeclarationPayload::Constant { value, .. } => {
                    self.scan(
                        value,
                        &context,
                        &mut Locals::new(),
                        &mut syntax,
                        Use::Value,
                        0,
                    )?;
                }
                DeclarationPayload::Test {
                    actual, expected, ..
                } => {
                    self.scan(
                        actual,
                        &context,
                        &mut Locals::new(),
                        &mut syntax,
                        Use::Value,
                        0,
                    )?;
                    self.scan(
                        expected,
                        &context,
                        &mut Locals::new(),
                        &mut syntax,
                        Use::Value,
                        0,
                    )?;
                }
                _ => {}
            }
        }
        let ports: Vec<_> = self
            .proof
            .owners
            .iter()
            .filter_map(|((package, key), record)| {
                if let (OwnerKey::Port(id), OwnerRecord::Port(port)) = (key, record) {
                    if let kernel::PortImplementation::Expression(expression) = port.implementation
                    {
                        Some((
                            kernel::PortReference {
                                package: *package,
                                port: *id,
                            },
                            port.declaration,
                            expression,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        for (port, declaration, expression) in ports {
            let mut context = Context::declaration(DeclarationReference {
                package: port.package,
                declaration,
            });
            context.port = Some(port);
            let expected = self.admit_port(port, &context)?;
            let value = self.scan(
                expression,
                &context,
                &mut Locals::new(),
                &mut syntax,
                Use::Value,
                0,
            )?;
            if value.ty != Some(expected) {
                return Err(invalid(
                    "port expression differs from exact declared callable type",
                ));
            }
        }
        for ((package, key), record) in &self.proof.owners {
            self.budget.step(0)?;
            match (key, record) {
                (OwnerKey::Expression(expression), OwnerRecord::Expression(_))
                    if !syntax.reached.contains(&(*package, *expression)) =>
                {
                    return Err(invalid(
                        "unattached expression syntax has no admitted callable context",
                    ));
                }
                (OwnerKey::Binding(binding), OwnerRecord::Binding(_))
                    if !syntax.bindings.contains(&(*package, *binding)) =>
                {
                    return Err(invalid(
                        "unused binding inventory has no exact lexical container",
                    ));
                }
                _ => {}
            }
        }
        Ok((syntax.lexical_owners, syntax.view_functions))
    }
    fn value_category(&mut self, value: &Value, context: &Context) -> Result<ValueCategory> {
        match value.ty {
            Some(ty) => Ok(self.facts(ty, context, 0)?.category),
            None => Ok(ValueCategory::OrdinaryData),
        }
    }
    fn scalar(&mut self, form: TypeForm) -> Result<Value> {
        Ok(Value {
            ty: Some(self.intern(form)?),
            ..Value::default()
        })
    }
    fn local(
        &mut self,
        value: LocalValueReference,
        context: &Context,
        locals: &Locals,
    ) -> Result<Local> {
        let local = locals
            .get(&value)
            .filter(|l| l.live)
            .cloned()
            .ok_or_else(|| invalid("view/Owned operation uses a dead or foreign lexical local"))?;
        if let LocalValueReference::FunctionParameter(parameter) = value {
            let owner = context
                .owner
                .ok_or_else(|| invalid("parameter lacks callable owner"))?;
            let OwnerRecord::Parameter(record) =
                self.owner(owner.package, OwnerKey::Parameter(parameter))?
            else {
                return Err(invalid("parameter record absent"));
            };
            if record.parent != ParameterParent::Function(owner.declaration) {
                return Err(invalid("parameter anchor imports another callable"));
            }
        }
        Ok(local)
    }
    fn local_value(
        &mut self,
        value: LocalValueReference,
        context: &Context,
        locals: &mut Locals,
        demand: Use,
    ) -> Result<Value> {
        let local = self.local(value, context, locals)?;
        let category = self.facts(local.ty, context, 0)?.category;
        if category == ValueCategory::Owned && demand == Use::Value {
            return Err(invalid(
                "affine Owned local requires consume/borrow/return use",
            ));
        }
        if category == ValueCategory::ReadView
            && !local.read
            && !matches!(demand, Use::Borrow | Use::Reference)
        {
            return Err(invalid("affine view copy requires explicit move/reborrow"));
        }
        if (demand == Use::Consume
            && matches!(category, ValueCategory::Owned | ValueCategory::ReadView))
            || (demand == Use::Return && category == ValueCategory::Owned && !local.read)
        {
            if local.read {
                return Err(invalid("borrowed parameter/read handle is consumed"));
            }
            self.consume(value, &local, context, locals)?;
        }
        Ok(Value {
            ty: Some(local.ty),
            source: Some(local.source),
            scopes: local.scopes,
            // An ordinary borrowed-choice payload is a copied value. Its Local
            // still retains the enclosing choice's source guard through the arm,
            // but that custody does not turn the copied result into a loan.
            read: category != ValueCategory::OrdinaryData
                && (local.read
                    || (matches!(category, ValueCategory::Owned | ValueCategory::ReadView)
                        && matches!(demand, Use::Borrow | Use::Reference))),
        })
    }
    fn consume(
        &mut self,
        value: LocalValueReference,
        local: &Local,
        context: &Context,
        locals: &mut Locals,
    ) -> Result<()> {
        for (other, guard) in locals.iter() {
            if *other != value && guard.live && guard.source == local.source {
                let category = self.facts(guard.ty, context, 0)?.category;
                if guard.read || category == ValueCategory::ReadView {
                    return Err(invalid(
                        "source consume/mutation occurs under a retained read/view guard",
                    ));
                }
            }
        }
        locals
            .get_mut(&value)
            .ok_or_else(|| invalid("missing consuming local"))?
            .live = false;
        Ok(())
    }
    fn fork_locals(&mut self, locals: &Locals) -> Result<Locals> {
        // Admit the entire map and every scope-set copy before allocating any
        // branch snapshot. Scope entries are scalar canonical coordinates.
        self.budget.reserve(locals.len())?;
        for local in locals.values() {
            self.budget.step(0)?;
            self.budget.reserve(local.scopes.len())?;
            for _ in &local.scopes {
                self.budget.step(0)?;
            }
        }
        Ok(locals.clone())
    }
    fn merge_locals(&mut self, left: &mut Locals, right: &Locals, context: &Context) -> Result<()> {
        if left.len() != right.len() {
            return Err(invalid(
                "untaken branch changes affine liveness/source guards",
            ));
        }
        for (key, local) in left.iter_mut() {
            self.budget.step(0)?;
            let other = right
                .get(key)
                .ok_or_else(|| invalid("untaken branch changes affine liveness/source guards"))?;
            if local.ty != other.ty
                || local.source != other.source
                || local.read != other.read
                || local.scopes.len() != other.scopes.len()
            {
                return Err(invalid(
                    "untaken branch changes affine liveness/source guards",
                ));
            }
            for (scope, other) in local.scopes.iter().zip(&other.scopes) {
                self.budget.step(0)?;
                if scope != other {
                    return Err(invalid(
                        "untaken branch changes affine liveness/source guards",
                    ));
                }
            }
            // A live read/view local also retains source custody. Unlike an
            // unguarded owner, its differing lifetime cannot be erased by the
            // owner-liveness intersection: the other path still freezes its
            // source. Keep the previous exact guard-lifetime obligation.
            if local.live != other.live
                && (local.read
                    || self.facts(local.ty, context, 0)?.category == ValueCategory::ReadView)
            {
                return Err(invalid(
                    "untaken branch changes affine liveness/source guards",
                ));
            }
            // An affine local remains usable only if every complete branch
            // retains it. Exact source/read/type/scope authority never joins.
            local.live &= other.live;
        }
        Ok(())
    }
    fn binding(
        &mut self,
        binding: BindingId,
        value: &Value,
        context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        expected: Option<BindingKind>,
    ) -> Result<()> {
        let package = context
            .owner
            .ok_or_else(|| invalid("binding lacks owner"))?
            .package;
        let OwnerRecord::Binding(record) = self.owner(package, OwnerKey::Binding(binding))? else {
            return Err(invalid("binding record absent"));
        };
        if matches!(
            record.kind,
            BindingKind::ReadView | BindingKind::ReadViewBorrow
        ) {
            if record.header.contract_version < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION {
                return Err(invalid("new view binding requires generation34"));
            }
            self.named_view_function(context)?;
        }
        if expected.is_some_and(|kind| kind != record.kind) {
            return Err(invalid("binding kind does not match its lexical operation"));
        }
        if !syntax.bindings.insert((package, binding)) {
            return Err(invalid("binding has more than one lexical container"));
        }
        syntax.lexical_owners.insert(
            (package, binding),
            context
                .owner
                .ok_or_else(|| invalid("binding callable absent"))?,
        );
        let ty = record
            .declared_type
            .or(value.ty)
            .ok_or_else(|| invalid("binding lacks independently known type"))?;
        if value.ty.is_some_and(|actual| actual != ty) {
            return Err(invalid("binding annotation changes admitted result type"));
        }
        let facts = self.facts(ty, context, 0)?;
        if matches!(
            record.kind,
            BindingKind::ReadView | BindingKind::ReadViewBorrow
        ) != (facts.category == ValueCategory::ReadView)
            && record.kind != BindingKind::Let
        {
            return Err(invalid("view/ordinary binding category mismatch"));
        }
        if facts.category == ValueCategory::ReadView && record.declared_type.is_none() {
            return Err(invalid(
                "view binding lacks exact nominal/family/scope annotation",
            ));
        }
        if facts.category == ValueCategory::ReadView {
            syntax.view_functions.insert(
                context
                    .owner
                    .ok_or_else(|| invalid("view binding callable absent"))?,
            );
        }
        if record.kind == BindingKind::Let && facts.category == ValueCategory::ReadView {
            return Err(invalid("ordinary Let binding contains an affine view"));
        }
        if record.kind == BindingKind::ReadView && (value.read || value.source.is_none()) {
            return Err(invalid(
                "owning view binding initializer is not an affine source-tied handle",
            ));
        }
        let local_value = LocalValueReference::LexicalBinding(binding);
        self.budget.reserve(1)?;
        if locals
            .insert(
                local_value,
                Local {
                    ty,
                    source: value.source.unwrap_or(local_value),
                    scopes: value.scopes.clone(),
                    read: value.read,
                    live: true,
                },
            )
            .is_some()
        {
            return Err(invalid("lexical binding captures another local"));
        }
        Ok(())
    }
    fn check_source_scope(
        &mut self,
        local: &Local,
        scope: ReadScopeArgument,
        context: &Context,
    ) -> Result<()> {
        self.scope(scope, context)?;
        if local.scopes.contains(&scope) {
            return Ok(());
        }
        if let ReadScopeArgument::Lexical { callable, binding } = scope
            && context.owner == Some(callable)
            && context.lexical.contains(&binding)
        {
            return Ok(());
        }
        Err(invalid(
            "source scope is widened or assigned to an unrelated parameter/local owner",
        ))
    }
    fn private_layout(
        &mut self,
        ty: TypeObjectDigest,
        field: kernel::ReadViewFieldReference,
        context: &Context,
    ) -> Result<Field> {
        let layout = self.layout(ty, context, 0)?;
        if layout.declaration != field.declaration {
            return Err(invalid(
                "view field reference changes exact instantiated declaration",
            ));
        }
        let owner = context
            .owner
            .ok_or_else(|| invalid("view field operation has no callable owner"))?;
        let declaration = self.declaration(owner)?;
        let view = self.declaration(field.declaration)?;
        if owner.package != field.declaration.package || declaration.module != view.module {
            return Err(invalid(
                "foreign module has no private view construction/field authority",
            ));
        }
        layout
            .fields
            .into_iter()
            .find(|f| f.id == field.field)
            .ok_or_else(|| invalid("field absent from complete exact view layout"))
    }
    fn expression(
        &mut self,
        package: PackageId,
        expression: ExpressionId,
    ) -> Result<kernel::ExpressionRecord> {
        match self.owner(package, OwnerKey::Expression(expression))? {
            OwnerRecord::Expression(record) if record.id == expression => Ok(record),
            _ => Err(invalid("expression owner identity mismatch")),
        }
    }
    fn scan(
        &mut self,
        expression: ExpressionId,
        context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        demand: Use,
        depth: usize,
    ) -> Result<Value> {
        self.budget.control.check()?;
        if depth > kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(invalid("all-path canonical expression depth exceeded"));
        }
        self.budget.step(0)?;
        let package = context
            .owner
            .ok_or_else(|| invalid("expression has no enclosing canonical declaration"))?
            .package;
        if !syntax.active.insert((package, expression)) {
            return Err(invalid("cycle in all-path canonical expression syntax"));
        }
        syntax.reached.insert((package, expression));
        self.budget.reserve(1)?;
        let record = self.expression(package, expression)?;
        let joined = syntax.joined_calls.contains(&(package, expression));
        let is_drop = matches!(record.operation, ExpressionOperation::DropReadView { .. });
        let operation = record.operation;
        self.operation_types(&operation, context)?;
        let argument_context =
            if let Some((pending_package, pending_expression, binding)) = syntax.pending_call {
                if pending_package == package && pending_expression == expression {
                    self.budget.reserve(context.lexical.len())?;
                    let mut prior = context.clone();
                    prior.lexical.remove(&binding);
                    Some(prior)
                } else {
                    None
                }
            } else {
                None
            };
        if (operation.is_read_view_operation() || !operation.read_scope_arguments().is_empty())
            && record.contract_version < kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
        {
            return Err(invalid("new view syntax uses a predecessor graph profile"));
        }
        if operation.is_read_view_operation() || !operation.read_scope_arguments().is_empty() {
            self.named_view_function(context)?;
        }
        if operation.is_read_view_operation() {
            syntax.view_functions.insert(
                context
                    .owner
                    .ok_or_else(|| invalid("view callable absent"))?,
            );
        }
        let value = match operation {
            ExpressionOperation::Unit {} => self.scalar(TypeForm::Unit)?,
            ExpressionOperation::Bool { .. } => self.scalar(TypeForm::Bool)?,
            ExpressionOperation::I64 { .. } => self.scalar(TypeForm::I64)?,
            ExpressionOperation::F64 { .. } => self.scalar(TypeForm::F64)?,
            ExpressionOperation::Text { .. } => self.scalar(TypeForm::Text)?,
            ExpressionOperation::StaticText { .. } => self.scalar(TypeForm::StaticText)?,
            ExpressionOperation::Local { value } => {
                self.local_value(value, context, locals, demand)?
            }
            ExpressionOperation::Constant { declaration } => {
                self.check_reference(context, declaration)?;
                let DeclarationPayload::Constant { ty, .. } =
                    self.declaration(declaration)?.payload
                else {
                    return Err(invalid("constant syntax names another declaration kind"));
                };
                Value {
                    ty: Some(ty),
                    ..Value::default()
                }
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.scan(condition, context, locals, syntax, Use::Value, depth + 1)?;
                let mut left = self.fork_locals(locals)?;
                let mut right = self.fork_locals(locals)?;
                let mut a = self.scan(when_true, context, &mut left, syntax, demand, depth + 1)?;
                let b = self.scan(when_false, context, &mut right, syntax, demand, depth + 1)?;
                if a.ty.is_some() && b.ty.is_some() && a.ty != b.ty {
                    return Err(invalid("untaken branch changes result type"));
                }
                let category = self.value_category(&a, context)?;
                if (a.source != b.source
                    || a.read != b.read
                    || (category == ValueCategory::ReadView && a.scopes != b.scopes))
                    && (category == ValueCategory::ReadView || a.read || b.read)
                {
                    return Err(invalid(
                        "untaken branch changes selected source lineage/scope",
                    ));
                }
                if category == ValueCategory::Owned && a.read {
                    common_scope_permissions(&mut a, &b, &mut self.budget)?;
                }
                self.merge_locals(&mut left, &right, context)?;
                *locals = left;
                a
            }
            ExpressionOperation::Let { bindings, body } => {
                unique(&bindings, "let bindings")?;
                let mut nested = context.clone();
                for binding in &bindings {
                    let OwnerRecord::Binding(record) =
                        self.owner(package, OwnerKey::Binding(*binding))?
                    else {
                        return Err(invalid("let binding absent"));
                    };
                    let source = record
                        .value
                        .ok_or_else(|| invalid("let binding lacks its value syntax"))?;
                    if record.kind == BindingKind::ReadView {
                        nested.lexical.insert(*binding);
                    }
                    let use_mode = if let Some(ty) = record.declared_type {
                        match self.facts(ty, &nested, 0)?.category {
                            ValueCategory::Owned => Use::Consume,
                            ValueCategory::ReadView if record.kind == BindingKind::ReadView => {
                                Use::Return
                            }
                            _ => Use::Value,
                        }
                    } else {
                        Use::Value
                    };
                    let value = self.scan(source, &nested, locals, syntax, use_mode, depth + 1)?;
                    if !matches!(record.kind, BindingKind::Let | BindingKind::ReadView) {
                        return Err(invalid("let syntax binds an unsupported lexical kind"));
                    }
                    self.binding(*binding, &value, &nested, locals, syntax, Some(record.kind))?;
                    nested.lexical.insert(*binding);
                }
                let value = self.scan(body, &nested, locals, syntax, demand, depth + 1)?;
                for binding in bindings {
                    locals.remove(&LocalValueReference::LexicalBinding(binding));
                }
                value
            }
            ExpressionOperation::Sequence { items } => {
                let count = items.len();
                let mut result = self.scalar(TypeForm::Unit)?;
                for (position, item) in items.into_iter().enumerate() {
                    let last = position + 1 == count;
                    result = self.scan(
                        item,
                        context,
                        locals,
                        syntax,
                        if last { demand } else { Use::Consume },
                        depth + 1,
                    )?;
                    if !last && self.value_category(&result, context)? == ValueCategory::ReadView {
                        return Err(invalid("sequence silently drops an affine view value"));
                    }
                }
                result
            }
            ExpressionOperation::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments,
                arguments,
            } => self.scan_call(
                function,
                &type_arguments,
                &effect_arguments,
                &requirement_arguments,
                &read_scope_arguments,
                &[],
                &arguments,
                context,
                argument_context.as_ref().unwrap_or(context),
                locals,
                syntax,
                depth + 1,
                joined,
            )?,
            ExpressionOperation::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments,
                implementations,
                arguments,
            } => self.scan_call(
                function,
                &type_arguments,
                &effect_arguments,
                &requirement_arguments,
                &read_scope_arguments,
                &implementations,
                &arguments,
                context,
                argument_context.as_ref().unwrap_or(context),
                locals,
                syntax,
                depth + 1,
                joined,
            )?,
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments,
            } => {
                let ty = self.callable_type(
                    function,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    &read_scope_arguments,
                    context,
                )?;
                self.annotated(ty, context)?
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                read_scope_arguments,
                arguments,
            } => {
                let (actual_contract, self_type, type_arguments) =
                    self.operand(&witness, context, 0)?;
                if actual_contract != contract {
                    return Err(invalid("method call changes exact witness contract"));
                }
                let declaration = self.declaration(contract)?;
                let DeclarationPayload::OwnedContract(contract_record) = declaration.payload else {
                    return Err(invalid("method call contract absent"));
                };
                let method = contract_record
                    .methods
                    .iter()
                    .find(|m| m.id == method)
                    .ok_or_else(|| invalid("method call identity absent"))?;
                let mut substitution =
                    self.contract_application(contract, self_type, &type_arguments, context)?;
                let binder = ReadScopeBinder::ContractMethod {
                    contract,
                    method: method.id,
                };
                if read_scope_arguments.len() != method.read_scope_parameters.len() {
                    return Err(invalid("method scope arity changed"));
                }
                for (formal, actual) in method
                    .read_scope_parameters
                    .iter()
                    .zip(read_scope_arguments)
                {
                    self.scope(actual, context)?;
                    substitution.scopes.insert(
                        ReadScopeReference {
                            binder,
                            parameter: formal.id,
                        },
                        actual,
                    );
                }
                for member in &contract_record.associated_types {
                    let scope = match &member.bound {
                        AssociatedTypeBound::Owned => None,
                        AssociatedTypeBound::ReadViewFamily { scope_parameter } => {
                            Some(ReadScopeArgument::Parameter(ReadScopeReference {
                                binder: ReadScopeBinder::AssociatedMember {
                                    contract,
                                    member: member.id,
                                },
                                parameter: scope_parameter.id,
                            }))
                        }
                    };
                    let ty = self.select_member(&witness, contract, member.id, scope, 0)?;
                    let formal = scope.and_then(|s| {
                        if let ReadScopeArgument::Parameter(p) = s {
                            Some(p)
                        } else {
                            None
                        }
                    });
                    substitution
                        .members
                        .insert((contract, member.id), (ty, formal));
                }
                let parameters = method
                    .parameters
                    .iter()
                    .map(|p| Ok((self.substitute(p.ty, &substitution, 0)?, p.use_mode)))
                    .collect::<Result<Vec<_>>>()?;
                let result = CallableResult {
                    ty: self.substitute(method.result.ty, &substitution, 0)?,
                    relation: substitute_relation(method.result.relation, &substitution),
                };
                let sources = method
                    .read_scope_sources
                    .iter()
                    .map(|s| ReadScopeSource {
                        source: s.source,
                        scope: substitute_scope(s.scope, &substitution),
                    })
                    .collect::<Vec<_>>();
                let row = self.effect_row(&method.effect, 0)?;
                self.admit_invocation(
                    matches!(method.effect, FunctionEffect::Task { .. }),
                    &row,
                    argument_context.as_ref().unwrap_or(context),
                )?;
                self.call_arguments(
                    &parameters,
                    result,
                    &sources,
                    &arguments,
                    context,
                    argument_context.as_ref().unwrap_or(context),
                    locals,
                    syntax,
                    depth + 1,
                    false,
                )?
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                if !matches!(
                    self.expression(package, call)?.operation,
                    ExpressionOperation::Call { .. }
                        | ExpressionOperation::ImplementationCall { .. }
                        | ExpressionOperation::MethodCall { .. }
                ) {
                    return Err(invalid(
                        "BorrowCall prospective scope requires an exact direct invocation",
                    ));
                }
                let mut nested = context.clone();
                nested.lexical.insert(binding);
                let previous_pending = syntax.pending_call.replace((package, call, binding));
                let value = self.scan(call, &nested, locals, syntax, Use::Return, depth + 1)?;
                syntax.pending_call = previous_pending;
                let category = self.value_category(&value, &nested)?;
                let kind = match category {
                    ValueCategory::ReadView => BindingKind::ReadView,
                    ValueCategory::Owned if value.read => BindingKind::OwnedBorrow,
                    _ => {
                        return Err(invalid(
                            "BorrowCall adopts an unsealed ordinary/owning result",
                        ));
                    }
                };
                self.binding(binding, &value, &nested, locals, syntax, Some(kind))?;
                let result = self.scan(body, &nested, locals, syntax, demand, depth + 1)?;
                locals.remove(&LocalValueReference::LexicalBinding(binding));
                result
            }
            ExpressionOperation::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                let source_local = self.local(source, context, locals)?;
                let source_category = self.facts(source_local.ty, context, 0)?.category;
                if !matches!(
                    source_category,
                    ValueCategory::Owned | ValueCategory::ReadView
                ) {
                    return Err(invalid(
                        "view constructor's designated source is not a live affine read",
                    ));
                }
                self.check_source_scope(&source_local, read_scope, context)?;
                let owner = context
                    .owner
                    .ok_or_else(|| invalid("constructor has no defining callable"))?;
                if owner.package != declaration.package
                    || self.declaration(owner)?.module != self.declaration(declaration)?.module
                {
                    return Err(invalid("foreign module constructs a private view"));
                }
                let ty = self.intern(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: type_arguments,
                    read_scope,
                })?;
                self.facts(ty, context, 0)?;
                let layout = self.layout(ty, context, 0)?;
                if fields.len() != layout.fields.len() {
                    return Err(invalid("constructor omits inactive view fields"));
                }
                // Protect the selected source before evaluating even ordinary
                // metadata, including a zero-reference descriptor's fields.
                locals
                    .get_mut(&source)
                    .ok_or_else(|| invalid("constructor source disappeared"))?
                    .read = true;
                for (field, expected) in fields.iter().zip(&layout.fields) {
                    if field.field != expected.id {
                        return Err(invalid(
                            "constructor field identities/order differ from complete authored layout",
                        ));
                    }
                    let use_mode =
                        if matches!(expected.kind, ReadViewFieldKind::ReadReference { .. }) {
                            Use::Reference
                        } else {
                            Use::Value
                        };
                    let value =
                        self.scan(field.value, context, locals, syntax, use_mode, depth + 1)?;
                    if value.ty != Some(expected.ty) {
                        return Err(invalid("constructor field has another instantiated type"));
                    }
                    if use_mode == Use::Reference
                        && (!value.read || value.source != Some(source_local.source))
                    {
                        return Err(invalid(
                            "view reference field derives from another same-typed source parameter",
                        ));
                    }
                    if use_mode == Use::Value
                        && (value.read
                            || self.value_category(&value, context)? != ValueCategory::OrdinaryData)
                    {
                        return Err(invalid(
                            "ordinary view metadata conveys source read authority",
                        ));
                    }
                }
                locals
                    .get_mut(&source)
                    .ok_or_else(|| invalid("constructor source disappeared"))?
                    .read = source_local.read;
                Value {
                    ty: Some(ty),
                    source: Some(source_local.source),
                    scopes: [read_scope].into_iter().collect(),
                    read: false,
                }
            }
            ExpressionOperation::ReborrowReadView { value, read_scope } => {
                let local = self.local(value, context, locals)?;
                if self.facts(local.ty, context, 0)?.category != ValueCategory::ReadView {
                    return Err(invalid("view reborrow selected a nonview"));
                }
                self.check_source_scope(&local, read_scope, context)?;
                let ty = match self.object(local.ty)?.form {
                    TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        ..
                    } => self.intern(TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        read_scope,
                    })?,
                    TypeForm::AssociatedReadViewProjection {
                        scope,
                        parameter,
                        contract,
                        member,
                        ..
                    } => self.intern(TypeForm::AssociatedReadViewProjection {
                        scope,
                        parameter,
                        contract,
                        member,
                        read_scope,
                    })?,
                    TypeForm::AssociatedReadViewMember {
                        contract, member, ..
                    } => self.intern(TypeForm::AssociatedReadViewMember {
                        contract,
                        member,
                        read_scope,
                    })?,
                    _ => unreachable!(),
                };
                Value {
                    ty: Some(ty),
                    source: Some(local.source),
                    scopes: [read_scope].into_iter().collect(),
                    read: false,
                }
            }
            ExpressionOperation::MoveReadView { value }
            | ExpressionOperation::DropReadView { value } => {
                let local = self.local(value, context, locals)?;
                if local.read
                    || self.facts(local.ty, context, 0)?.category != ValueCategory::ReadView
                {
                    return Err(invalid(
                        "view move/drop consumes a borrowed or nonview local",
                    ));
                }
                // Handle release/move does not consume its designated source.
                locals
                    .get_mut(&value)
                    .ok_or_else(|| invalid("view handle absent"))?
                    .live = false;
                if is_drop {
                    self.scalar(TypeForm::Unit)?
                } else {
                    Value {
                        ty: Some(local.ty),
                        source: Some(local.source),
                        scopes: local.scopes,
                        read: false,
                    }
                }
            }
            ExpressionOperation::BorrowReadView {
                value,
                binding,
                body,
            } => {
                let local = self.local(value, context, locals)?;
                if self.facts(local.ty, context, 0)?.category != ValueCategory::ReadView {
                    return Err(invalid("view borrow selected nonview local"));
                }
                let mut nested = context.clone();
                nested.lexical.insert(binding);
                let value = Value {
                    ty: Some(local.ty),
                    source: Some(local.source),
                    scopes: local.scopes,
                    read: true,
                };
                self.binding(
                    binding,
                    &value,
                    &nested,
                    locals,
                    syntax,
                    Some(BindingKind::ReadViewBorrow),
                )?;
                let result = self.scan(body, &nested, locals, syntax, demand, depth + 1)?;
                locals.remove(&LocalValueReference::LexicalBinding(binding));
                result
            }
            ExpressionOperation::ReadViewDataField { value, field } => {
                let local = self.local(value, context, locals)?;
                let field = self.private_layout(local.ty, field, context)?;
                if field.kind != ReadViewFieldKind::Ordinary {
                    return Err(invalid(
                        "ordinary view field syntax tries to recover a reference/custody guard",
                    ));
                }
                Value {
                    ty: Some(field.ty),
                    ..Value::default()
                }
            }
            ExpressionOperation::BorrowReadViewField {
                value,
                field,
                binding,
                body,
            } => {
                let local = self.local(value, context, locals)?;
                let field = self.private_layout(local.ty, field, context)?;
                if !matches!(field.kind, ReadViewFieldKind::ReadReference { .. }) {
                    return Err(invalid(
                        "reference field syntax promotes metadata/CustodyOnly into access",
                    ));
                }
                let mut nested = context.clone();
                nested.lexical.insert(binding);
                let category = self.facts(field.ty, context, 0)?.category;
                let value = Value {
                    ty: Some(field.ty),
                    source: Some(local.source),
                    scopes: local.scopes,
                    read: true,
                };
                self.binding(
                    binding,
                    &value,
                    &nested,
                    locals,
                    syntax,
                    Some(if category == ValueCategory::ReadView {
                        BindingKind::ReadViewBorrow
                    } else {
                        BindingKind::OwnedBorrow
                    }),
                )?;
                let result = self.scan(body, &nested, locals, syntax, demand, depth + 1)?;
                locals.remove(&LocalValueReference::LexicalBinding(binding));
                result
            }
            operation => {
                self.scan_ordinary(operation, context, locals, syntax, demand, depth + 1)?
            }
        };
        syntax.active.remove(&(package, expression));
        if matches!(demand, Use::Value | Use::Consume)
            && self.value_category(&value, context)? == ValueCategory::ReadView
        {
            return Err(invalid(
                "view occurrence crosses an ordinary/copy/capture/container boundary",
            ));
        }
        Ok(value)
    }
    fn operation_types(
        &mut self,
        operation: &ExpressionOperation,
        context: &Context,
    ) -> Result<()> {
        let required = match operation {
            ExpressionOperation::PackOwned { product_type, .. }
            | ExpressionOperation::UnpackOwned { product_type, .. }
            | ExpressionOperation::BorrowOwnedField { product_type, .. } => {
                Some((*product_type, 0))
            }
            ExpressionOperation::ChooseOwned { choice_type, .. }
            | ExpressionOperation::MatchOwned { choice_type, .. }
            | ExpressionOperation::MatchBorrowedOwned { choice_type, .. } => {
                Some((*choice_type, 1))
            }
            ExpressionOperation::SequenceEmpty { sequence_type }
            | ExpressionOperation::SequenceLength { sequence_type, .. }
            | ExpressionOperation::SequencePush { sequence_type, .. }
            | ExpressionOperation::SequencePop { sequence_type, .. }
            | ExpressionOperation::BorrowOwnedItem { sequence_type, .. }
            | ExpressionOperation::SequenceGet { sequence_type, .. }
            | ExpressionOperation::SequenceReplace { sequence_type, .. } => {
                Some((*sequence_type, 2))
            }
            ExpressionOperation::PackOwnedRecord { record_type, .. }
            | ExpressionOperation::UnpackOwnedRecord { record_type, .. }
            | ExpressionOperation::BorrowOwnedRecordField { record_type, .. } => {
                Some((*record_type, 3))
            }
            _ => None,
        };
        if let Some((ty, kind)) = required {
            if self.facts(ty, context, 0)?.category != ValueCategory::Owned {
                return Err(invalid(
                    "Owned operation type annotation counterfeits a view/ordinary category",
                ));
            }
            let form = self.object(ty)?.form;
            let shape = match (kind, form) {
                (0, TypeForm::OwnedProduct { .. })
                | (1, TypeForm::OwnedChoice { .. })
                | (2, TypeForm::OwnedSequence { .. }) => true,
                (3, TypeForm::Named { declaration })
                | (3, TypeForm::Applied { declaration, .. }) => matches!(
                    self.declaration(declaration)?.payload,
                    DeclarationPayload::OwnedRecord { .. }
                ),
                _ => false,
            };
            if !shape {
                return Err(invalid(
                    "Owned operation annotation changes exact product/case/index/nominal path kind",
                ));
            }
        }
        match operation {
            ExpressionOperation::TransactionOutcome { type_argument, .. } => {
                if self.facts(*type_argument, context, 0)?.category == ValueCategory::ReadView {
                    return Err(invalid(
                        "transaction syntax hides a view in an unused type actual",
                    ));
                }
            }
            ExpressionOperation::SequencePop { result_type, .. }
            | ExpressionOperation::SequenceReplace { result_type, .. }
                if self.facts(*result_type, context, 0)?.category != ValueCategory::Owned =>
            {
                return Err(invalid(
                    "Owned sequence result contract contains an ordinary/view occurrence",
                ));
            }
            _ => {}
        }
        Ok(())
    }
    fn bind_scopes(
        &mut self,
        function: DeclarationReference,
        arguments: &[ReadScopeArgument],
        context: &Context,
        substitution: &mut Substitution,
    ) -> Result<()> {
        let binder = ReadScopeBinder::Declaration(function);
        let formals = self.proof.scope_parameters(binder)?;
        if formals.len() != arguments.len() {
            return Err(invalid(
                "call scope actuals omit unused formals or change their order/arity",
            ));
        }
        self.budget.reserve(arguments.len())?;
        for (formal, actual) in formals.iter().zip(arguments) {
            self.scope(*actual, context)?;
            substitution.scopes.insert(
                ReadScopeReference {
                    binder,
                    parameter: formal.id,
                },
                *actual,
            );
        }
        Ok(())
    }
    fn scan_call(
        &mut self,
        reference: DeclarationReference,
        type_arguments: &[TypeObjectDigest],
        effect_arguments: &[kernel::EffectRow],
        requirement_arguments: &[kernel::RequirementOperand],
        scopes: &[ReadScopeArgument],
        witnesses: &[ImplementationOperand],
        arguments: &[ExpressionId],
        context: &Context,
        argument_context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        depth: usize,
        joined: bool,
    ) -> Result<Value> {
        self.check_reference(context, reference)?;
        let declaration = self.declaration(reference)?;
        let mut substitution = self.bind_types(reference, type_arguments, context)?;
        let (parameters, result, sources, named) = match declaration.payload {
            DeclarationPayload::Function(function) => {
                self.bind_rows(
                    reference,
                    &function,
                    effect_arguments,
                    requirement_arguments,
                    argument_context,
                    &mut substitution,
                )?;
                let source_row = self.effect_row(&function.effect, 0)?;
                let required_row = self.substitute_effect(&source_row, &substitution, 0)?;
                self.admit_invocation(
                    matches!(function.effect, FunctionEffect::Task { .. }),
                    &required_row,
                    argument_context,
                )?;
                self.bind_scopes(reference, scopes, context, &mut substitution)?;
                if witnesses.len() != function.implementation_parameters.len() {
                    return Err(invalid("named call omits unused witness actuals"));
                }
                for (formal, operand) in function.implementation_parameters.iter().zip(witnesses) {
                    let got = self.operand(operand, context, 0)?;
                    if got
                        != (
                            formal.contract,
                            self.substitute(formal.self_type, &substitution, 0)?,
                            self.substitute_many(&formal.type_arguments, &substitution, 0)?,
                        )
                    {
                        return Err(invalid("named call changes exact witness application"));
                    }
                    substitution
                        .witnesses
                        .insert((reference, formal.id), self.copy_operand(operand, 0)?);
                }
                let (parameters, result) = self.function_signature(reference, &function)?;
                let sources = self
                    .proof
                    .scope_sources(ReadScopeBinder::Declaration(reference))?
                    .iter()
                    .map(|s| ReadScopeSource {
                        source: s.source,
                        scope: substitute_scope(s.scope, &substitution),
                    })
                    .collect();
                (parameters, result, sources, true)
            }
            DeclarationPayload::External(function) => {
                if !scopes.is_empty()
                    || !witnesses.is_empty()
                    || !effect_arguments.is_empty()
                    || !requirement_arguments.is_empty()
                {
                    return Err(invalid("external call accepts scope/witness operands"));
                }
                let mut parameters = Vec::new();
                for parameter in function.parameters {
                    let OwnerRecord::Parameter(record) =
                        self.owner(reference.package, OwnerKey::Parameter(parameter))?
                    else {
                        return Err(invalid("external parameter absent"));
                    };
                    parameters.push((record.ty, record.use_mode));
                }
                (
                    parameters,
                    CallableResult {
                        ty: function.result,
                        relation: ResultRelation::Value,
                    },
                    Vec::new(),
                    false,
                )
            }
            _ => return Err(invalid("named call changes callable kind")),
        };
        self.budget.reserve(parameters.len())?;
        let parameters = parameters
            .into_iter()
            .enumerate()
            .map(|(position, (ty, use_mode))| {
                self.budget.step(0)?;
                let direct = matches!(
                    &self
                        .derived
                        .get(&ty)
                        .or_else(|| self.input.get(&ty))
                        .or_else(|| self.proof.types.get(&ty))
                        .ok_or_else(|| invalid("call source parameter canonical type absent"))?
                        .form,
                    TypeForm::CapabilityResource { .. }
                );
                let actual = self.substitute(ty, &substitution, 0)?;
                // Task/binding authority was admitted under the source callee.
                // Actual types and scopes belong to the caller Context, which
                // may be a port or test rather than another named function.
                if named && !direct && !self.resource_free(actual, context, 0)? {
                    return Err(invalid(format!(
                        "applied function parameter contains a forbidden capability container; owner={}/{}; position={position}; type={actual}",
                        reference.package, reference.declaration,
                    )));
                }
                Ok((actual, use_mode))
            })
            .collect::<Result<Vec<_>>>()?;
        let result = CallableResult {
            ty: self.substitute(result.ty, &substitution, 0)?,
            relation: substitute_relation(result.relation, &substitution),
        };
        if named && !self.resource_free(result.ty, context, 0)? {
            return Err(invalid(format!(
                "applied function result contains a capability resource; owner={}/{}; type={}",
                reference.package, reference.declaration, result.ty,
            )));
        }
        self.call_arguments(
            &parameters,
            result,
            &sources,
            arguments,
            context,
            argument_context,
            locals,
            syntax,
            depth,
            joined,
        )
    }
    fn call_arguments(
        &mut self,
        parameters: &[(TypeObjectDigest, ParameterUse)],
        result: CallableResult<u32>,
        sources: &[ReadScopeSource<u32>],
        arguments: &[ExpressionId],
        context: &Context,
        argument_context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        depth: usize,
        joined: bool,
    ) -> Result<Value> {
        if parameters.len() != arguments.len() {
            return Err(invalid("call argument arity changed"));
        }
        self.budget.reserve(arguments.len())?;
        let mut values = Vec::new();
        for (position, ((ty, use_mode), expression)) in parameters.iter().zip(arguments).enumerate()
        {
            let demand = match use_mode {
                ParameterUse::Borrow => Use::Borrow,
                ParameterUse::Consume => Use::Consume,
                ParameterUse::Unrestricted => Use::Value,
            };
            let value = self.scan(
                *expression,
                argument_context,
                locals,
                syntax,
                demand,
                depth + 1,
            )?;
            if value.ty != Some(*ty) {
                let caller = context
                    .owner
                    .ok_or_else(|| invalid("call argument has no canonical caller owner"))?;
                return Err(invalid(format!(
                    "call argument differs from complete substituted parameter type; caller={}/{}; argument={expression}; position={position}; expected={ty}; actual={}",
                    caller.package,
                    caller.declaration,
                    value
                        .ty
                        .map_or_else(|| "unproved".to_owned(), |actual| actual.to_string()),
                )));
            }
            let category = self.facts(*ty, context, 0)?.category;
            if joined {
                let facts = self.facts(*ty, context, 0)?;
                if (*use_mode == ParameterUse::Borrow && !facts.share)
                    || (*use_mode != ParameterUse::Borrow && !facts.transfer)
                {
                    return Err(invalid(
                        "joined argument lacks independent complete ReadShareable/Shareable/Transferable closure",
                    ));
                }
            }
            if category == ValueCategory::ReadView && *use_mode != ParameterUse::Borrow {
                return Err(invalid("call copies/transfers a view argument"));
            }
            values.push(value);
        }
        for source in sources {
            let value = values
                .get(source.source as usize)
                .ok_or_else(|| invalid("unused call source position omitted"))?;
            self.scope(source.scope, context)?;
            if !value.scopes.contains(&source.scope)
                && !matches!(source.scope, ReadScopeArgument::Lexical { callable, binding }
                if context.owner == Some(callable) && context.lexical.contains(&binding))
            {
                return Err(invalid(
                    "call source actual has no explicit forwarding/lexical containment evidence",
                ));
            }
        }
        let facts = self.facts(result.ty, context, 0)?;
        match result.relation {
            ResultRelation::Value => {
                if facts.category == ValueCategory::ReadView {
                    return Err(invalid("Value call result carries a view"));
                }
                Ok(Value {
                    ty: Some(result.ty),
                    ..Value::default()
                })
            }
            ResultRelation::ReadFrom { source } => {
                let source = values
                    .get(source as usize)
                    .ok_or_else(|| invalid("call result source absent"))?;
                if facts.category != ValueCategory::Owned || source.source.is_none() {
                    return Err(invalid("read result has no exact argument anchor"));
                }
                Ok(Value {
                    ty: Some(result.ty),
                    source: source.source,
                    scopes: source.scopes.clone(),
                    read: true,
                })
            }
            ResultRelation::ViewFrom { source, scope } => {
                let source = values
                    .get(source as usize)
                    .ok_or_else(|| invalid("view result source absent"))?;
                if facts.category != ValueCategory::ReadView
                    || source.source.is_none()
                    || self.view_scope(result.ty)? != scope
                {
                    return Err(invalid("view result changes argument anchor/type/scope"));
                }
                Ok(Value {
                    ty: Some(result.ty),
                    source: source.source,
                    scopes: [scope].into_iter().collect(),
                    read: false,
                })
            }
        }
    }
    /// Reject only canonical container roots that forbid capability occurrence.
    /// A bare nominal variant can describe task-local resource metadata, and an
    /// unused generic actual does not invent a universal resource-free bound.
    fn require_resource_container_free(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        depth: usize,
    ) -> Result<()> {
        self.budget.step(depth)?;
        let forbidden_root = match &self
            .derived
            .get(&ty)
            .or_else(|| self.input.get(&ty))
            .or_else(|| self.proof.types.get(&ty))
            .ok_or_else(|| invalid("canonical container type absent"))?
            .form
        {
            TypeForm::StructuralRecord { .. }
            | TypeForm::List { .. }
            | TypeForm::Option { .. }
            | TypeForm::Stream { .. }
            | TypeForm::Map { .. }
            | TypeForm::Result { .. }
            | TypeForm::Function { .. }
            | TypeForm::TaskFunction { .. } => true,
            TypeForm::Applied { declaration, .. } => !matches!(
                self.proof.declaration(*declaration)?.payload,
                DeclarationPayload::OwnedRecord { .. }
            ),
            _ => false,
        };
        if forbidden_root && !self.resource_free(ty, context, depth)? {
            return Err(invalid(format!(
                "forbidden canonical container contains a capability resource; type={ty}",
            )));
        }
        Ok(())
    }

    /// Capability occurrence is a separate raw closure property. Every actual,
    /// including a phantom, and every stored/future child is visited once. The
    /// owner/binder/lexical Context stays fixed; field and actual edges stay distinct.
    /// No result is returned until the whole reachable closure completes, and no
    /// optimistic inner result enters a cache.
    fn resource_free(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        depth: usize,
    ) -> Result<bool> {
        let mut pending = Vec::new();
        let mut visited = BTreeSet::new();
        let mut free = true;
        let field_context = self.property_context(context)?;
        self.resource_edge(&mut pending, ty, depth, context.structural_properties)?;
        while let Some((ty, depth, structural)) = pending.pop() {
            self.budget.step(depth)?;
            if visited.contains(&(ty, structural)) {
                continue;
            }
            self.budget.reserve(1)?;
            visited.insert((ty, structural));
            let context = if structural { &field_context } else { context };
            // The owned TypeObject copy has variable payload only in these forms.
            // Admit its complete copied entries and work before object() clones it.
            let copied = match &self
                .derived
                .get(&ty)
                .or_else(|| self.input.get(&ty))
                .or_else(|| self.proof.types.get(&ty))
                .ok_or_else(|| invalid("resource closure canonical type absent"))?
                .form
            {
                TypeForm::Applied { arguments, .. }
                | TypeForm::ReadViewApplied { arguments, .. } => arguments.len(),
                TypeForm::StructuralRecord { fields }
                | TypeForm::OwnedProduct { fields }
                | TypeForm::OwnedChoice { cases: fields } => fields.len(),
                TypeForm::Function { parameters, .. } => parameters.len(),
                TypeForm::TaskFunction {
                    parameters, effect, ..
                } => parameters
                    .len()
                    .checked_add(effect.requirements.len())
                    .and_then(|count| count.checked_add(effect.parameters.len()))
                    .ok_or_else(|| invalid("resource closure copied inventory overflow"))?,
                _ => 0,
            };
            self.budget.reserve(copied)?;
            for _ in 0..copied {
                self.budget.step(depth)?;
            }
            let object = self.object(ty)?;
            let node_free = match object.form {
                TypeForm::CapabilityResource { .. } => false,
                TypeForm::TypeParameter { parameter } => {
                    self.resource_formal(parameter, context)?;
                    true
                }
                TypeForm::Named { declaration }
                | TypeForm::Applied { declaration, .. }
                | TypeForm::ReadViewApplied { declaration, .. } => {
                    self.check_reference(context, declaration)?;
                    let arguments = match &object.form {
                        TypeForm::Applied { arguments, .. }
                        | TypeForm::ReadViewApplied { arguments, .. } => arguments.as_slice(),
                        _ => &[],
                    };
                    // Borrow the raw declaration and child records. The new
                    // predicate allocates only the admitted substitution map.
                    let proof = self.proof;
                    let record = proof.declaration(declaration)?;
                    let formals = record.payload.type_parameters();
                    if formals.len() != arguments.len() {
                        return Err(invalid(
                            "resource closure nominal application has wrong arity",
                        ));
                    }
                    self.budget.reserve(formals.len())?;
                    let mut substitution = Substitution::default();
                    for (formal, actual) in formals.iter().zip(arguments) {
                        self.budget.step(depth)?;
                        self.resource_formal(*formal, &Context::declaration(declaration))?;
                        substitution.types.insert(*formal, *actual);
                        self.resource_edge(
                            &mut pending,
                            *actual,
                            depth + 1,
                            context.structural_properties,
                        )?;
                    }
                    match &record.payload {
                        DeclarationPayload::Record { fields, .. }
                        | DeclarationPayload::OwnedRecord { fields, .. } => {
                            for id in fields {
                                self.budget.step(depth)?;
                                let Some(OwnerRecord::Field(field)) = proof
                                    .owners
                                    .get(&(declaration.package, OwnerKey::Field(*id)))
                                else {
                                    return Err(invalid(
                                        "resource closure field inventory missing",
                                    ));
                                };
                                if field.declaration != declaration.declaration
                                    || field.header.kind != OwnerKind::Field
                                {
                                    return Err(invalid(
                                        "resource closure field has another owner",
                                    ));
                                }
                                let child = self.substitute(field.ty, &substitution, depth + 1)?;
                                self.resource_edge(&mut pending, child, depth + 1, true)?;
                            }
                        }
                        DeclarationPayload::Variant { cases, .. } => {
                            for id in cases {
                                self.budget.step(depth)?;
                                let Some(OwnerRecord::Case(case)) = proof
                                    .owners
                                    .get(&(declaration.package, OwnerKey::Case(*id)))
                                else {
                                    return Err(invalid("resource closure case inventory missing"));
                                };
                                if case.declaration != declaration.declaration
                                    || case.header.kind != OwnerKind::Case
                                {
                                    return Err(invalid("resource closure case has another owner"));
                                }
                                if let Some(payload) = case.payload {
                                    let child =
                                        self.substitute(payload, &substitution, depth + 1)?;
                                    self.resource_edge(&mut pending, child, depth + 1, true)?;
                                }
                            }
                        }
                        DeclarationPayload::ReadView(view) => {
                            let TypeForm::ReadViewApplied { read_scope, .. } = object.form else {
                                return Err(invalid(
                                    "resource closure view is not a nominal view instance",
                                ));
                            };
                            self.budget.reserve(1)?;
                            substitution.scopes.insert(
                                ReadScopeReference {
                                    binder: ReadScopeBinder::Declaration(declaration),
                                    parameter: view.scope_parameter.id,
                                },
                                read_scope,
                            );
                            for field in &view.fields {
                                self.budget.step(depth)?;
                                let child = self.substitute(field.ty, &substitution, depth + 1)?;
                                self.resource_edge(&mut pending, child, depth + 1, true)?;
                            }
                        }
                        _ => return Err(invalid("resource closure nominal owner has no layout")),
                    }
                    true
                }
                TypeForm::StructuralRecord { fields }
                | TypeForm::OwnedProduct { fields }
                | TypeForm::OwnedChoice { cases: fields } => {
                    for field in fields {
                        self.resource_edge(
                            &mut pending,
                            field.ty,
                            depth + 1,
                            context.structural_properties,
                        )?;
                    }
                    true
                }
                TypeForm::Function { parameters, result }
                | TypeForm::TaskFunction {
                    parameters, result, ..
                } => {
                    self.resource_edge(
                        &mut pending,
                        result,
                        depth + 1,
                        context.structural_properties,
                    )?;
                    for parameter in parameters {
                        self.resource_edge(
                            &mut pending,
                            parameter,
                            depth + 1,
                            context.structural_properties,
                        )?;
                    }
                    true
                }
                TypeForm::List { item }
                | TypeForm::Option { item }
                | TypeForm::Stream { item }
                | TypeForm::OwnedSequence { item } => {
                    self.resource_edge(
                        &mut pending,
                        item,
                        depth + 1,
                        context.structural_properties,
                    )?;
                    true
                }
                TypeForm::Map { key, value }
                | TypeForm::Result {
                    ok: key,
                    error: value,
                } => {
                    self.resource_edge(
                        &mut pending,
                        key,
                        depth + 1,
                        context.structural_properties,
                    )?;
                    self.resource_edge(
                        &mut pending,
                        value,
                        depth + 1,
                        context.structural_properties,
                    )?;
                    true
                }
                // These exact abstract members/formals have positive Owned/view
                // meaning, not capability authority. Their contextual inventories
                // and every selected concrete member are independently admitted.
                TypeForm::AssociatedContractMember { .. }
                | TypeForm::AssociatedWitnessProjection { .. }
                | TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
                | TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::F64
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::Secret => true,
            };
            free &= node_free;
        }
        Ok(free)
    }

    /// Symbolic capability absence needs exact formal authority, not a copied
    /// declaration or its property constraints. Borrow both raw inventories and
    /// charge every compared identity before selecting the exact parent record.
    fn resource_formal(&mut self, parameter: TypeParameterId, context: &Context) -> Result<()> {
        self.budget.step(0)?;
        let owner = context
            .owner
            .ok_or_else(|| invalid("type formal has no exact occurrence owner"))?;
        let proof = self.proof;
        let declaration = proof.declaration(owner)?;
        let formals = declaration.payload.type_parameters();
        let mut listed = false;
        for candidate in formals.iter() {
            self.budget.step(0)?;
            if *candidate == parameter {
                listed = true;
                break;
            }
        }
        if !listed {
            return Err(invalid("foreign or unused unlisted type formal"));
        }
        self.budget.step(0)?;
        match proof
            .owners
            .get(&(owner.package, OwnerKey::TypeParameter(parameter)))
        {
            Some(OwnerRecord::TypeParameter(record))
                if record.header.owner == OwnerKey::TypeParameter(parameter)
                    && record.header.kind == OwnerKind::TypeParameter
                    && record.declaration == owner.declaration =>
            {
                Ok(())
            }
            _ => Err(invalid("type formal's record names another declaration")),
        }
    }

    fn resource_edge(
        &mut self,
        pending: &mut Vec<(TypeObjectDigest, usize, bool)>,
        ty: TypeObjectDigest,
        depth: usize,
        structural: bool,
    ) -> Result<()> {
        self.budget.step(depth)?;
        self.budget.reserve(1)?;
        pending.try_reserve(1).map_err(|_| {
            ExecutionError::resource(
                "reference_read_view_proof_storage",
                "cannot reserve raw capability closure worklist",
            )
        })?;
        pending.push((ty, depth, structural));
        Ok(())
    }

    /// A separate stored-value property: callable signatures are future inputs,
    /// not captures. The unchanged root Context qualifies every formal here;
    /// ordinary coinduction is private to this root and publishes no inner fact.
    fn capture_safe(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        active: &mut BTreeSet<(TypeObjectDigest, bool)>,
        depth: usize,
    ) -> Result<bool> {
        self.closure_property(ty, context, active, depth, true)
    }
    fn memory_free(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        depth: usize,
    ) -> Result<bool> {
        self.closure_property(ty, context, &mut BTreeSet::new(), depth, false)
    }
    fn closure_property(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        active: &mut BTreeSet<(TypeObjectDigest, bool)>,
        depth: usize,
        capture: bool,
    ) -> Result<bool> {
        self.budget.step(depth)?;
        let key = (ty, context.structural_properties);
        if active.contains(&key) {
            return Ok(true);
        }
        self.budget.reserve(1)?;
        active.insert(key);
        let object = self.object(ty)?;
        let safe = match object.form {
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. } if capture => true,
            TypeForm::TypeParameter { parameter } => {
                let constraints = self.parameter(parameter, context)?;
                if capture {
                    constraints.proves_capture_safe()
                } else {
                    !constraints.has_owned()
                }
            }
            TypeForm::Secret | TypeForm::Stream { .. } if capture => false,
            TypeForm::CapabilityResource { .. }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
            | TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => false,
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                self.check_reference(context, declaration)?;
                let arguments = match &object.form {
                    TypeForm::Applied { arguments, .. } => arguments.as_slice(),
                    _ => &[],
                };
                let record = self.declaration(declaration)?;
                let formals = record.payload.type_parameters();
                if formals.len() != arguments.len() {
                    return Err(invalid("capture nominal application has wrong arity"));
                }
                self.budget.reserve(formals.len())?;
                let mut substitution = Substitution::default();
                let mut safe = true;
                for (formal, actual) in formals.iter().zip(arguments) {
                    self.parameter(*formal, &Context::declaration(declaration))?;
                    substitution.types.insert(*formal, *actual);
                    safe &= self.closure_property(*actual, context, active, depth + 1, capture)?;
                }
                let field_context = self.property_context(context)?;
                match record.payload {
                    DeclarationPayload::Record { fields, .. } => {
                        for id in fields {
                            let OwnerRecord::Field(field) =
                                self.owner(declaration.package, OwnerKey::Field(id))?
                            else {
                                return Err(invalid("capture field inventory missing"));
                            };
                            if field.declaration != declaration.declaration {
                                return Err(invalid("capture field has another owner"));
                            }
                            let child = self.substitute(field.ty, &substitution, depth + 1)?;
                            safe &= self.closure_property(
                                child,
                                &field_context,
                                active,
                                depth + 1,
                                capture,
                            )?;
                        }
                    }
                    DeclarationPayload::Variant { cases, .. } => {
                        for id in cases {
                            let OwnerRecord::Case(case) =
                                self.owner(declaration.package, OwnerKey::Case(id))?
                            else {
                                return Err(invalid("capture case inventory missing"));
                            };
                            if case.declaration != declaration.declaration {
                                return Err(invalid("capture case has another owner"));
                            }
                            if let Some(payload) = case.payload {
                                let child = self.substitute(payload, &substitution, depth + 1)?;
                                safe &= self.closure_property(
                                    child,
                                    &field_context,
                                    active,
                                    depth + 1,
                                    capture,
                                )?;
                            }
                        }
                    }
                    DeclarationPayload::OwnedRecord { .. } => safe = false,
                    _ => return Err(invalid("capture nominal form selects a non-layout owner")),
                }
                safe
            }
            _ => {
                let mut safe = true;
                for child in object.child_types() {
                    safe &= self.closure_property(child, context, active, depth + 1, capture)?;
                }
                safe
            }
        };
        active.remove(&key);
        Ok(safe)
    }
    fn callable_type(
        &mut self,
        reference: DeclarationReference,
        types: &[TypeObjectDigest],
        effects: &[kernel::EffectRow],
        requirements: &[kernel::RequirementOperand],
        scopes: &[ReadScopeArgument],
        context: &Context,
    ) -> Result<TypeObjectDigest> {
        self.check_reference(context, reference)?;
        let record = self.declaration(reference)?;
        for ty in types {
            if !self.memory_free(*ty, context, 0)? {
                return Err(invalid(
                    "first-class callable type actual contains memory/view/resource",
                ));
            }
        }
        let mut substitution = self.bind_types(reference, types, context)?;
        let (parameters, result, task, row) = match record.payload {
            DeclarationPayload::Function(function) => {
                if function.result.relation != ResultRelation::Value
                    || !function.implementation_parameters.is_empty()
                    || !function.read_scope_parameters.is_empty()
                    || !function.read_scope_sources.is_empty()
                    || !scopes.is_empty()
                {
                    return Err(invalid(
                        "first-class function syntax carries borrowed result, scopes or lexical witnesses",
                    ));
                }
                self.bind_rows(
                    reference,
                    &function,
                    effects,
                    requirements,
                    context,
                    &mut substitution,
                )?;
                let (parameters, result) = self.function_signature(reference, &function)?;
                let task = matches!(function.effect, FunctionEffect::Task { .. });
                let source_row = self.effect_row(&function.effect, 0)?;
                let row = self.substitute_effect(&source_row, &substitution, 0)?;
                (parameters, result.ty, task, row)
            }
            DeclarationPayload::External(function) => {
                if !effects.is_empty() || !requirements.is_empty() || !scopes.is_empty() {
                    return Err(invalid(
                        "external function value imports effect/requirement/view scopes",
                    ));
                }
                self.budget.reserve(function.parameters.len())?;
                let mut parameters = Vec::new();
                for id in function.parameters {
                    let OwnerRecord::Parameter(parameter) =
                        self.owner(reference.package, OwnerKey::Parameter(id))?
                    else {
                        return Err(invalid("external callable parameter missing"));
                    };
                    if parameter.parent != ParameterParent::Function(reference.declaration) {
                        return Err(invalid("external callable parameter has another owner"));
                    }
                    parameters.push((parameter.ty, parameter.use_mode));
                }
                (
                    parameters,
                    function.result,
                    false,
                    kernel::EffectRow::default(),
                )
            }
            _ => return Err(invalid("function-value syntax is not an exact callable")),
        };
        self.budget.reserve(parameters.len())?;
        let mut actual_parameters = Vec::new();
        for (parameter, mode) in parameters {
            if mode != ParameterUse::Unrestricted {
                return Err(invalid(
                    "first-class callable carries borrowed/consuming parameters",
                ));
            }
            let ty = self.substitute(parameter, &substitution, 0)?;
            self.facts(ty, context, 0)?;
            if !self.memory_free(ty, context, 0)? {
                return Err(invalid(
                    "first-class callable parameter contains memory/view/resource",
                ));
            }
            actual_parameters.push(ty);
        }
        let result = self.substitute(result, &substitution, 0)?;
        self.facts(result, context, 0)?;
        if !self.memory_free(result, context, 0)? {
            return Err(invalid(
                "first-class callable result contains memory/view/resource",
            ));
        }
        let ty = self.intern(if task {
            TypeForm::TaskFunction {
                parameters: actual_parameters,
                result,
                effect: row,
            }
        } else {
            TypeForm::Function {
                parameters: actual_parameters,
                result,
            }
        })?;
        self.facts(ty, context, 0)?;
        Ok(ty)
    }
    fn nominal_fields(
        &mut self,
        ty: TypeObjectDigest,
        context: &Context,
        owned: bool,
    ) -> Result<Vec<(kernel::FieldReference, TypeObjectDigest)>> {
        let (reference, arguments) = match self.object(ty)?.form {
            TypeForm::Named { declaration } => (declaration, Vec::new()),
            TypeForm::Applied {
                declaration,
                arguments,
            } => (declaration, arguments),
            _ => return Err(invalid("record layout requires an exact nominal instance")),
        };
        self.check_reference(context, reference)?;
        let record = self.declaration(reference)?;
        let fields = match record.payload {
            DeclarationPayload::OwnedRecord { fields, .. } if owned => {
                let caller = context
                    .owner
                    .ok_or_else(|| invalid("private Owned record access lacks canonical owner"))?;
                if caller.package != reference.package
                    || self.declaration(caller)?.module != record.module
                {
                    return Err(invalid(
                        "foreign module accesses a private Owned record layout",
                    ));
                }
                fields
            }
            DeclarationPayload::Record { fields, .. } if !owned => fields,
            _ => {
                return Err(invalid(
                    "record syntax selects another exact nominal category",
                ));
            }
        };
        let substitution = self.bind_types(reference, &arguments, context)?;
        self.budget.reserve(fields.len())?;
        let mut result = Vec::new();
        for id in fields {
            let OwnerRecord::Field(field) = self.owner(reference.package, OwnerKey::Field(id))?
            else {
                return Err(invalid("record layout has a missing field"));
            };
            if field.declaration != reference.declaration {
                return Err(invalid("record field names another declaration"));
            }
            let ty = self.substitute(field.ty, &substitution, 0)?;
            self.facts(ty, context, 0)?;
            result.push((
                kernel::FieldReference {
                    package: reference.package,
                    field: id,
                },
                ty,
            ));
        }
        Ok(result)
    }
    fn transaction_outcome_type(
        &mut self,
        contract: kernel::TransactionOutcomeContract,
        actual: TypeObjectDigest,
        context: &Context,
    ) -> Result<TypeObjectDigest> {
        contract.validate_identity().map_err(|_| {
            invalid("transaction outcome changes exact canonical standard identities")
        })?;
        self.check_reference(context, contract.outcome)?;
        self.check_reference(context, contract.abort_reason)?;
        let DeclarationPayload::Variant {
            type_parameters,
            cases,
        } = self.declaration(contract.outcome)?.payload
        else {
            return Err(invalid("transaction outcome is not an ordinary variant"));
        };
        let DeclarationPayload::Variant {
            type_parameters: reason_parameters,
            cases: reason_cases,
        } = self.declaration(contract.abort_reason)?.payload
        else {
            return Err(invalid(
                "transaction abort reason is not an ordinary variant",
            ));
        };
        if type_parameters.len() != 1
            || !reason_parameters.is_empty()
            || cases.len() != 2
            || reason_cases.len() != 2
            || self.parameter(type_parameters[0], &Context::declaration(contract.outcome))?
                != TypeParameterConstraints::None
        {
            return Err(invalid(
                "transaction outcome has an incompatible complete nominal shape",
            ));
        }
        let mut committed = false;
        let mut aborted = false;
        for case in &cases {
            self.budget.step(0)?;
            committed |= *case == contract.committed.case;
            aborted |= *case == contract.aborted.case;
        }
        let mut condition = false;
        let mut conflict = false;
        for case in &reason_cases {
            self.budget.step(0)?;
            condition |= *case == contract.condition_failed.case;
            conflict |= *case == contract.conflict.case;
        }
        if !committed || !aborted || !condition || !conflict {
            return Err(invalid(
                "transaction outcome changes complete case identity inventory",
            ));
        }
        let reason = self.intern(TypeForm::Named {
            declaration: contract.abort_reason,
        })?;
        for (reference, owner, expected) in [
            (
                contract.committed,
                contract.outcome,
                Some(TypeForm::TypeParameter {
                    parameter: type_parameters[0],
                }),
            ),
            (
                contract.aborted,
                contract.outcome,
                Some(TypeForm::Named {
                    declaration: contract.abort_reason,
                }),
            ),
            (contract.condition_failed, contract.abort_reason, None),
            (contract.conflict, contract.abort_reason, None),
        ] {
            let OwnerRecord::Case(case) =
                self.owner(reference.package, OwnerKey::Case(reference.case))?
            else {
                return Err(invalid("transaction outcome case is absent"));
            };
            if reference.package != owner.package
                || case.declaration != owner.declaration
                || match (case.payload, expected) {
                    (None, None) => false,
                    (Some(payload), Some(expected)) => self.object(payload)?.form != expected,
                    _ => true,
                }
            {
                return Err(invalid(
                    "transaction outcome case has incompatible payload/owner",
                ));
            }
            if reference == contract.aborted && case.payload != Some(reason) {
                return Err(invalid(
                    "transaction abort case has another exact reason instance",
                ));
            }
        }
        self.bind_types(contract.outcome, &[actual], context)?;
        let ty = self.intern(TypeForm::Applied {
            declaration: contract.outcome,
            arguments: vec![actual],
        })?;
        self.facts(ty, context, 0)?;
        Ok(ty)
    }
    fn direct_requirement(
        &mut self,
        requirement: kernel::RequirementOperand,
        context: &Context,
    ) -> Result<kernel::RequirementConstraint> {
        let constraint = self.requirement_constraint(requirement, context)?;
        let (task, available) = self.caller_row(context)?;
        if !task || !available.requirements.contains(&requirement) {
            return Err(invalid(
                "capability/transaction requirement is absent from exact task allowance",
            ));
        }
        Ok(constraint)
    }
    fn scan_transaction(
        &mut self,
        requirement: kernel::RequirementOperand,
        binding: BindingId,
        body: ExpressionId,
        context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        demand: Use,
        depth: usize,
    ) -> Result<Value> {
        self.direct_requirement(requirement, context)?;
        let owner = context
            .owner
            .ok_or_else(|| invalid("transaction has no owner"))?;
        let OwnerRecord::Binding(record) = self.owner(owner.package, OwnerKey::Binding(binding))?
        else {
            return Err(invalid("transaction binding absent"));
        };
        let ty = record
            .declared_type
            .ok_or_else(|| invalid("transaction binding lacks exact Unit marker"))?;
        if record.value.is_some()
            || record.kind != BindingKind::Transaction
            || self.object(ty)?.form != TypeForm::Unit
        {
            return Err(invalid("transaction binding is not an exact Unit marker"));
        }
        self.binding(
            binding,
            &Value {
                ty: Some(ty),
                ..Value::default()
            },
            context,
            locals,
            syntax,
            Some(BindingKind::Transaction),
        )?;
        let local = locals
            .remove(&LocalValueReference::LexicalBinding(binding))
            .ok_or_else(|| invalid("transaction binding was not admitted"))?;
        locals.insert(LocalValueReference::TransactionBinding(binding), local);
        let result = self.scan(body, context, locals, syntax, demand, depth + 1)?;
        locals.remove(&LocalValueReference::TransactionBinding(binding));
        Ok(result)
    }
    fn annotated(&mut self, ty: TypeObjectDigest, context: &Context) -> Result<Value> {
        self.facts(ty, context, 0)?;
        Ok(Value {
            ty: Some(ty),
            ..Value::default()
        })
    }
    fn scan_ordinary(
        &mut self,
        operation: ExpressionOperation,
        context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        demand: Use,
        depth: usize,
    ) -> Result<Value> {
        let package = context
            .owner
            .ok_or_else(|| invalid("ordinary operation has no enclosing owner"))?
            .package;
        let borrowed_match = matches!(operation, ExpressionOperation::MatchBorrowedOwned { .. });
        let binding_callable = matches!(operation, ExpressionOperation::Bind { .. });
        let value = match operation {
            ExpressionOperation::Invoke { callee, arguments }
            | ExpressionOperation::Bind { callee, arguments } => {
                let callee = self.scan(callee, context, locals, syntax, Use::Value, depth + 1)?;
                let ty = callee
                    .ty
                    .ok_or_else(|| invalid("indirect callable type was not reconstructed"))?;
                if !self.memory_free(ty, context, 0)? {
                    return Err(invalid(
                        "indirect callable signature contains memory/view/resource",
                    ));
                }
                let (parameters, result, effect) = match self.object(ty)?.form {
                    TypeForm::Function { parameters, result } => (parameters, result, None),
                    TypeForm::TaskFunction {
                        parameters,
                        result,
                        effect,
                    } => (parameters, result, Some(effect)),
                    _ => {
                        return Err(invalid(
                            "indirect callee is not an exact pure/task callable",
                        ));
                    }
                };
                if (!binding_callable && arguments.len() != parameters.len())
                    || (binding_callable && arguments.len() > parameters.len())
                {
                    return Err(invalid(
                        "indirect invocation/bound prefix has wrong exact arity",
                    ));
                }
                for (parameter, argument) in parameters.iter().zip(&arguments) {
                    if binding_callable
                        && !self.capture_safe(*parameter, context, &mut BTreeSet::new(), 0)?
                    {
                        return Err(invalid(
                            "bound callable prefix is not independently capture-safe",
                        ));
                    }
                    let actual =
                        self.scan(*argument, context, locals, syntax, Use::Value, depth + 1)?;
                    if actual.ty != Some(*parameter) {
                        return Err(invalid(
                            "indirect callable argument differs from exact parameter type",
                        ));
                    }
                }
                if binding_callable {
                    self.budget.reserve(parameters.len() - arguments.len())?;
                    let parameters = parameters.into_iter().skip(arguments.len()).collect();
                    let ty = self.intern(match effect {
                        Some(effect) => TypeForm::TaskFunction {
                            parameters,
                            result,
                            effect,
                        },
                        None => TypeForm::Function { parameters, result },
                    })?;
                    self.annotated(ty, context)?
                } else {
                    if let Some(effect) = &effect {
                        self.admit_invocation(true, effect, context)?;
                    }
                    self.annotated(result, context)?
                }
            }
            ExpressionOperation::Record {
                nominal_type,
                type_arguments,
                fields,
            } => {
                self.budget.reserve(fields.len() * 2)?;
                let mut structural = Vec::new();
                let nominal = if let Some(declaration) = nominal_type {
                    self.budget.reserve(type_arguments.len())?;
                    for _ in &type_arguments {
                        self.budget.step(0)?;
                    }
                    let ty = self.intern(if type_arguments.is_empty() {
                        TypeForm::Named { declaration }
                    } else {
                        TypeForm::Applied {
                            declaration,
                            arguments: type_arguments.clone(),
                        }
                    })?;
                    Some((ty, self.nominal_fields(ty, context, false)?))
                } else {
                    None
                };
                if nominal
                    .as_ref()
                    .is_some_and(|(_, expected)| expected.len() != fields.len())
                {
                    return Err(invalid(
                        "nominal record construction changes complete field inventory",
                    ));
                }
                let mut seen = BTreeSet::new();
                for field in fields {
                    let value =
                        self.scan(field.value, context, locals, syntax, Use::Value, depth + 1)?;
                    let ty = value
                        .ty
                        .ok_or_else(|| invalid("record field has no exact reconstructed type"))?;
                    if !seen.insert(field.selector.clone()) {
                        return Err(invalid("record construction duplicates a field selector"));
                    }
                    match (field.selector, &nominal) {
                        (kernel::FieldSelector::Structural(name), None) => {
                            structural.push(StructuralTypeField { name, ty })
                        }
                        (kernel::FieldSelector::Nominal(reference), Some((_, expected))) => {
                            let mut matched = false;
                            for (field, expected) in expected {
                                self.budget.step(0)?;
                                matched |= *field == reference && *expected == ty;
                            }
                            if !matched {
                                return Err(invalid(
                                    "nominal record field identity/type differs from exact substituted layout",
                                ));
                            }
                        }
                        _ => {
                            return Err(invalid(
                                "record field selector changes exact nominal/structural kind",
                            ));
                        }
                    }
                }
                if let Some((ty, _)) = nominal {
                    return self.annotated(ty, context);
                }
                let form = if let Some(declaration) = nominal_type {
                    self.bind_types(declaration, &type_arguments, context)?;
                    if type_arguments.is_empty() {
                        TypeForm::Named { declaration }
                    } else {
                        TypeForm::Applied {
                            declaration,
                            arguments: type_arguments,
                        }
                    }
                } else {
                    if !type_arguments.is_empty() {
                        return Err(invalid("structural record has phantom nominal actuals"));
                    }
                    structural.sort_by(|a, b| a.name.cmp(&b.name));
                    TypeForm::StructuralRecord { fields: structural }
                };
                let ty = self.intern(form)?;
                self.annotated(ty, context)?
            }
            ExpressionOperation::Variant {
                case,
                type_arguments,
                payload,
            } => {
                let OwnerRecord::Case(case_record) =
                    self.owner(case.package, OwnerKey::Case(case.case))?
                else {
                    return Err(invalid("variant case absent"));
                };
                let declaration = DeclarationReference {
                    package: case.package,
                    declaration: case_record.declaration,
                };
                let DeclarationPayload::Variant { cases, .. } =
                    self.declaration(declaration)?.payload
                else {
                    return Err(invalid("variant constructor case has a non-variant owner"));
                };
                if !cases.contains(&case.case) {
                    return Err(invalid(
                        "variant constructor case is outside complete owner inventory",
                    ));
                }
                let substitution = self.bind_types(declaration, &type_arguments, context)?;
                let expected = case_record
                    .payload
                    .map(|ty| self.substitute(ty, &substitution, 0))
                    .transpose()?;
                match (payload, expected) {
                    (Some(payload), Some(expected)) => {
                        let value =
                            self.scan(payload, context, locals, syntax, Use::Value, depth + 1)?;
                        if value.ty != Some(expected) {
                            return Err(invalid(
                                "variant payload differs from exact substituted case type",
                            ));
                        }
                    }
                    (None, None) => {}
                    _ => {
                        return Err(invalid(
                            "variant payload presence differs from exact case inventory",
                        ));
                    }
                }
                let form = if type_arguments.is_empty() {
                    TypeForm::Named { declaration }
                } else {
                    TypeForm::Applied {
                        declaration,
                        arguments: type_arguments,
                    }
                };
                let ty = self.intern(form)?;
                self.annotated(ty, context)?
            }
            ExpressionOperation::Field { value, selector } => {
                let value = self.scan(value, context, locals, syntax, Use::Borrow, depth + 1)?;
                if self.value_category(&value, context)? == ValueCategory::ReadView {
                    return Err(invalid(
                        "ordinary field operation hides view layout authority",
                    ));
                }
                let ty = match selector {
                    kernel::FieldSelector::Nominal(field) => {
                        let OwnerRecord::Field(field_record) =
                            self.owner(field.package, OwnerKey::Field(field.field))?
                        else {
                            return Err(invalid("nominal field absent"));
                        };
                        let declaration = DeclarationReference {
                            package: field.package,
                            declaration: field_record.declaration,
                        };
                        if let Some(ty) = value.ty {
                            let arguments = match self.object(ty)?.form {
                                TypeForm::Named { declaration: d } if d == declaration => {
                                    Vec::new()
                                }
                                TypeForm::Applied {
                                    declaration: d,
                                    arguments,
                                } if d == declaration => arguments,
                                _ => {
                                    return Err(invalid(
                                        "nominal field source type changes exact declaration",
                                    ));
                                }
                            };
                            let substitution = self.bind_types(declaration, &arguments, context)?;
                            Some(self.substitute(field_record.ty, &substitution, 0)?)
                        } else {
                            return Err(invalid(
                                "nominal field source has no exact reconstructed type",
                            ));
                        }
                    }
                    kernel::FieldSelector::Structural(name) => {
                        if let Some(ty) = value.ty {
                            match self.object(ty)?.form {
                                TypeForm::StructuralRecord { fields }
                                | TypeForm::OwnedProduct { fields } => {
                                    fields.into_iter().find(|f| f.name == name).map(|f| f.ty)
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                };
                let ty =
                    ty.ok_or_else(|| invalid("field selector is absent from exact source layout"))?;
                {
                    let f = self.facts(ty, context, 0)?;
                    if matches!(f.category, ValueCategory::Owned | ValueCategory::ReadView) {
                        return Err(invalid("ordinary field syntax copies an affine child"));
                    }
                }
                Value {
                    ty: Some(ty),
                    ..Value::default()
                }
            }
            ExpressionOperation::List { item_type, items } => {
                self.facts(item_type, context, 0)?;
                for item in items {
                    let value = self.scan(item, context, locals, syntax, Use::Value, depth + 1)?;
                    if value.ty != Some(item_type) {
                        return Err(invalid("list item differs from exact declared item type"));
                    }
                }
                let ty = self.intern(TypeForm::List { item: item_type })?;
                self.annotated(ty, context)?
            }
            ExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => {
                for entry in entries {
                    let key =
                        self.scan(entry.key, context, locals, syntax, Use::Value, depth + 1)?;
                    let value =
                        self.scan(entry.value, context, locals, syntax, Use::Value, depth + 1)?;
                    if key.ty != Some(key_type) || value.ty != Some(value_type) {
                        return Err(invalid(
                            "map entry differs from exact declared key/value types",
                        ));
                    }
                }
                let ty = self.intern(TypeForm::Map {
                    key: key_type,
                    value: value_type,
                })?;
                self.annotated(ty, context)?
            }
            ExpressionOperation::Match { value, arms } => {
                let scrutinee = self.scan(value, context, locals, syntax, Use::Value, depth + 1)?;
                let ty = scrutinee
                    .ty
                    .ok_or_else(|| invalid("match scrutinee has no exact reconstructed type"))?;
                let (declaration, arguments) = match self.object(ty)?.form {
                    TypeForm::Named { declaration } => (declaration, Vec::new()),
                    TypeForm::Applied {
                        declaration,
                        arguments,
                    } => (declaration, arguments),
                    _ => return Err(invalid("ordinary match requires an exact nominal variant")),
                };
                let DeclarationPayload::Variant { cases, .. } =
                    self.declaration(declaration)?.payload
                else {
                    return Err(invalid("ordinary match selects a non-variant declaration"));
                };
                if cases.len() != arms.len()
                    || arms.iter().zip(&cases).any(|(arm, expected)| {
                        arm.case.package != declaration.package || arm.case.case != *expected
                    })
                {
                    return Err(invalid(
                        "match arms change complete exact case identity/order/inventory",
                    ));
                }
                let substitution = self.bind_types(declaration, &arguments, context)?;
                let initial = self.fork_locals(locals)?;
                let mut result = None;
                let mut state = None;
                for arm in arms {
                    let mut branch = self.fork_locals(&initial)?;
                    let OwnerRecord::Case(case) =
                        self.owner(arm.case.package, OwnerKey::Case(arm.case.case))?
                    else {
                        return Err(invalid("match case absent"));
                    };
                    if case.declaration != declaration.declaration {
                        return Err(invalid("match case imports another declaration"));
                    }
                    let expected = case
                        .payload
                        .map(|ty| self.substitute(ty, &substitution, 0))
                        .transpose()?;
                    if expected.is_some() != arm.payload_binding.is_some() {
                        return Err(invalid(
                            "match payload binding differs from exact case payload presence",
                        ));
                    }
                    if let Some(binding) = arm.payload_binding {
                        let OwnerRecord::Binding(record) =
                            self.owner(package, OwnerKey::Binding(binding))?
                        else {
                            return Err(invalid("match payload binding record absent"));
                        };
                        if record.value.is_some() || record.declared_type != expected {
                            return Err(invalid(
                                "match payload annotation differs from substituted case type",
                            ));
                        }
                        let expected =
                            expected.ok_or_else(|| invalid("match binding has no payload"))?;
                        if matches!(
                            self.facts(expected, context, 0)?.category,
                            ValueCategory::Owned | ValueCategory::ReadView
                        ) {
                            return Err(invalid("ordinary match binds an affine payload"));
                        }
                        self.binding(
                            binding,
                            &Value {
                                ty: Some(expected),
                                ..Value::default()
                            },
                            context,
                            &mut branch,
                            syntax,
                            Some(BindingKind::MatchPayload),
                        )?;
                        if let Some(local) =
                            branch.remove(&LocalValueReference::LexicalBinding(binding))
                        {
                            branch.insert(LocalValueReference::MatchPayload(binding), local);
                        }
                    }
                    let mut value =
                        self.scan(arm.body, context, &mut branch, syntax, demand, depth + 1)?;
                    if let Some(binding) = arm.payload_binding {
                        branch.remove(&LocalValueReference::MatchPayload(binding));
                    }
                    if let Some(previous) = &result {
                        compare_branch_values(&mut value, previous, &mut self.budget)?;
                    }
                    if let Some(previous) = &mut state {
                        self.merge_locals(previous, &branch, context)?;
                    } else {
                        state = Some(branch);
                    }
                    result = Some(value);
                }
                *locals = state
                    .ok_or_else(|| invalid("ordinary match has no admitted complete arm state"))?;
                result
                    .ok_or_else(|| invalid("ordinary match has no admitted complete arm result"))?
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                let constraint = self.direct_requirement(requirement, context)?;
                if !constraint.operations.contains(&operation)
                    || operation.package != constraint.interface.package
                {
                    return Err(invalid(
                        "capability operation is outside exact requirement operations",
                    ));
                }
                let OwnerRecord::Operation(record) =
                    self.owner(operation.package, OwnerKey::Operation(operation.operation))?
                else {
                    return Err(invalid("capability operation absent"));
                };
                if record.declaration != constraint.interface.declaration
                    || record.parameters.len() != arguments.len()
                {
                    return Err(invalid(
                        "capability call changes exact interface or parameter arity",
                    ));
                }
                for (parameter, argument) in record.parameters.iter().zip(arguments) {
                    let OwnerRecord::Parameter(parameter) =
                        self.owner(operation.package, OwnerKey::Parameter(*parameter))?
                    else {
                        return Err(invalid("capability parameter record absent"));
                    };
                    if parameter.parent != ParameterParent::Operation(operation.operation) {
                        return Err(invalid("capability parameter imports another operation"));
                    }
                    let value =
                        self.scan(argument, context, locals, syntax, Use::Value, depth + 1)?;
                    if value.ty != Some(parameter.ty) {
                        return Err(invalid(
                            "capability argument differs from exact operation parameter type",
                        ));
                    }
                }
                self.annotated(record.result, context)?
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => self.scan_transaction(
                requirement,
                binding,
                body,
                context,
                locals,
                syntax,
                demand,
                depth + 1,
            )?,
            ExpressionOperation::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            } => {
                let constraint = self.direct_requirement(requirement, context)?;
                let interface = DeclarationReference {
                    package: outcome.outcome.package,
                    declaration: "decl_640e96fa57dee1c09557eb4bc7b53398"
                        .parse()
                        .map_err(|_| invalid("invalid canonical DataStore identity"))?,
                };
                let operation = kernel::OperationReference {
                    package: outcome.outcome.package,
                    operation: "op_1c083402875f8f088541c27751f61d22"
                        .parse()
                        .map_err(|_| invalid("invalid canonical transaction operation identity"))?,
                };
                if constraint.interface != interface || !constraint.operations.contains(&operation)
                {
                    return Err(invalid(
                        "transaction outcome lacks exact DataStore transaction operation",
                    ));
                }
                let result = self.scan_transaction(
                    requirement,
                    binding,
                    body,
                    context,
                    locals,
                    syntax,
                    Use::Value,
                    depth + 1,
                )?;
                if result.ty != Some(type_argument) {
                    return Err(invalid(
                        "transaction outcome body differs from exact type actual",
                    ));
                }
                let ty = self.transaction_outcome_type(outcome, type_argument, context)?;
                self.annotated(ty, context)?
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                for field in fields {
                    self.scan(
                        field.value,
                        context,
                        locals,
                        syntax,
                        Use::Consume,
                        depth + 1,
                    )?;
                }
                self.annotated(product_type, context)?
            }
            ExpressionOperation::PackOwnedRecord {
                record_type,
                fields,
            } => {
                for field in fields {
                    self.scan(
                        field.value,
                        context,
                        locals,
                        syntax,
                        Use::Consume,
                        depth + 1,
                    )?;
                }
                self.annotated(record_type, context)?
            }
            ExpressionOperation::ChooseOwned {
                choice_type, value, ..
            } => {
                self.scan(value, context, locals, syntax, Use::Consume, depth + 1)?;
                self.annotated(choice_type, context)?
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                self.facts(product_type, context, 0)?;
                let source = self.scan(source, context, locals, syntax, Use::Consume, depth + 1)?;
                if source.ty != Some(product_type) {
                    return Err(invalid(
                        "unpack source differs from exact Owned product annotation",
                    ));
                }
                let TypeForm::OwnedProduct { fields: expected } = self.object(product_type)?.form
                else {
                    return Err(invalid("unpack source is not an Owned product"));
                };
                if fields.len() != expected.len() {
                    return Err(invalid("unpack omits a complete ordered field inventory"));
                }
                self.budget.reserve(fields.len())?;
                let mut bindings = Vec::new();
                for (field, contract) in fields.into_iter().zip(expected) {
                    let binding = field.binding;
                    let OwnerRecord::Binding(record) =
                        self.owner(package, OwnerKey::Binding(binding))?
                    else {
                        return Err(invalid("unpack binding absent"));
                    };
                    if field.name != contract.name
                        || record.value.is_some()
                        || record.declared_type != Some(contract.ty)
                    {
                        return Err(invalid(
                            "unpack field identity/order/annotation differs from exact product layout",
                        ));
                    }
                    self.binding(
                        binding,
                        &Value {
                            ty: Some(contract.ty),
                            ..Value::default()
                        },
                        context,
                        locals,
                        syntax,
                        Some(BindingKind::OwnedUnpack),
                    )?;
                    bindings.push(binding);
                }
                let result = self.scan(body, context, locals, syntax, demand, depth + 1)?;
                for binding in bindings {
                    locals.remove(&LocalValueReference::LexicalBinding(binding));
                }
                result
            }
            ExpressionOperation::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            } => {
                self.facts(record_type, context, 0)?;
                let expected = self.nominal_fields(record_type, context, true)?;
                let source = self.scan(source, context, locals, syntax, Use::Consume, depth + 1)?;
                if source.ty != Some(record_type) {
                    return Err(invalid(
                        "unpack source differs from exact Owned record annotation",
                    ));
                }
                if fields.len() != expected.len() {
                    return Err(invalid(
                        "nominal unpack omits a complete ordered field inventory",
                    ));
                }
                self.budget.reserve(fields.len())?;
                let mut bindings = Vec::new();
                for (field, (reference, ty)) in fields.into_iter().zip(expected) {
                    let binding = field.binding;
                    let OwnerRecord::Binding(record) =
                        self.owner(package, OwnerKey::Binding(binding))?
                    else {
                        return Err(invalid("unpack-record binding absent"));
                    };
                    if field.field != reference
                        || record.value.is_some()
                        || record.declared_type != Some(ty)
                    {
                        return Err(invalid(
                            "nominal unpack field identity/order/annotation differs from exact private layout",
                        ));
                    }
                    self.binding(
                        binding,
                        &Value {
                            ty: Some(ty),
                            ..Value::default()
                        },
                        context,
                        locals,
                        syntax,
                        Some(BindingKind::OwnedUnpack),
                    )?;
                    bindings.push(binding);
                }
                let result = self.scan(body, context, locals, syntax, demand, depth + 1)?;
                for binding in bindings {
                    locals.remove(&LocalValueReference::LexicalBinding(binding));
                }
                result
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            }
            | ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                self.facts(choice_type, context, 0)?;
                let borrowed = borrowed_match;
                let source = self.scan(
                    source,
                    context,
                    locals,
                    syntax,
                    if borrowed { Use::Borrow } else { Use::Consume },
                    depth + 1,
                )?;
                if source.ty != Some(choice_type) {
                    return Err(invalid(
                        "OwnedChoice path source differs from exact choice type",
                    ));
                }
                let TypeForm::OwnedChoice { cases } = self.object(choice_type)?.form else {
                    return Err(invalid("choice path selects another category"));
                };
                if arms.len() != cases.len()
                    || arms
                        .iter()
                        .zip(&cases)
                        .any(|(arm, case)| arm.name != case.name)
                {
                    return Err(invalid("choice path omits or duplicates inactive cases"));
                }
                let initial = self.fork_locals(locals)?;
                let mut result = None;
                let mut state = None;
                for (arm, case) in arms.into_iter().zip(&cases) {
                    let mut branch = self.fork_locals(&initial)?;
                    let OwnerRecord::Binding(record) =
                        self.owner(package, OwnerKey::Binding(arm.binding))?
                    else {
                        return Err(invalid("choice payload binding absent"));
                    };
                    if record.declared_type != Some(case.ty) {
                        return Err(invalid(
                            "choice payload annotation counterfeits an OwnedChild path",
                        ));
                    }
                    if record.value.is_some() {
                        return Err(invalid("choice payload binding has an initializer"));
                    }
                    self.budget.reserve(context.lexical.len() + 1)?;
                    for _ in &context.lexical {
                        self.budget.step(0)?;
                    }
                    let mut nested = context.clone();
                    nested.lexical.insert(arm.binding);
                    self.budget.reserve(source.scopes.len() + 1)?;
                    for _ in &source.scopes {
                        self.budget.step(0)?;
                    }
                    let mut scopes = source.scopes.clone();
                    if borrowed {
                        if source.source.is_none() || !source.read {
                            return Err(invalid(
                                "borrowed choice lacks an admitted live source anchor",
                            ));
                        }
                        let owner = context
                            .owner
                            .ok_or_else(|| invalid("borrowed choice has no owner"))?;
                        let declaration = self.declaration(owner)?;
                        if matches!(declaration.payload, DeclarationPayload::Function(_))
                            && declaration.header.contract_version
                                >= kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
                        {
                            scopes.insert(ReadScopeArgument::Lexical {
                                callable: owner,
                                binding: arm.binding,
                            });
                        }
                    }
                    let value = Value {
                        ty: Some(case.ty),
                        source: if borrowed { source.source } else { None },
                        scopes,
                        // Both ordinary and Owned arms protect the whole source.
                        // local_value exposes only an Owned child as a read loan;
                        // ordinary payloads remain copied values under this guard.
                        read: borrowed,
                    };
                    self.binding(
                        arm.binding,
                        &value,
                        &nested,
                        &mut branch,
                        syntax,
                        Some(if borrowed {
                            BindingKind::OwnedBorrow
                        } else {
                            BindingKind::OwnedChoicePayload
                        }),
                    )?;
                    let mut value =
                        self.scan(arm.body, &nested, &mut branch, syntax, demand, depth + 1)?;
                    branch.remove(&LocalValueReference::LexicalBinding(arm.binding));
                    // The arm's loan permission ends here. Its returned Owned read can
                    // retain the same source, but cannot grant this departed lexical scope.
                    if let Some(callable) = context.owner {
                        value.scopes.remove(&ReadScopeArgument::Lexical {
                            callable,
                            binding: arm.binding,
                        });
                    }
                    if let Some(previous) = &result {
                        compare_branch_values(&mut value, previous, &mut self.budget)?;
                    }
                    if let Some(previous) = &mut state {
                        self.merge_locals(previous, &branch, context)?;
                    } else {
                        state = Some(branch);
                    }
                    result = Some(value);
                }
                *locals = state
                    .ok_or_else(|| invalid("Owned choice has no admitted complete arm state"))?;
                result.ok_or_else(|| invalid("Owned choice has no admitted complete arm result"))?
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                binding,
                body,
                field,
            } => {
                let source = self.scan(source, context, locals, syntax, Use::Borrow, depth + 1)?;
                if source.ty != Some(product_type) {
                    return Err(invalid(
                        "product path source differs from exact Owned product type",
                    ));
                }
                let TypeForm::OwnedProduct { fields } = self.object(product_type)?.form else {
                    return Err(invalid("borrow product type is not an Owned product"));
                };
                let field = fields
                    .into_iter()
                    .find(|f| f.name == field)
                    .ok_or_else(|| invalid("borrowed product field absent"))?;
                self.scan_owned_loan(
                    field.ty,
                    source,
                    binding,
                    body,
                    context,
                    locals,
                    syntax,
                    demand,
                    depth + 1,
                )?
            }
            ExpressionOperation::BorrowOwnedRecordField {
                record_type,
                source,
                binding,
                body,
                field,
            } => {
                self.facts(record_type, context, 0)?;
                let source = self.scan(source, context, locals, syntax, Use::Borrow, depth + 1)?;
                if source.ty != Some(record_type) {
                    return Err(invalid(
                        "nominal path source differs from exact Owned record instance",
                    ));
                }
                let OwnerRecord::Field(field_record) =
                    self.owner(field.package, OwnerKey::Field(field.field))?
                else {
                    return Err(invalid("borrowed nominal field absent"));
                };
                let (declaration, arguments) = match self.object(record_type)?.form {
                    TypeForm::Named { declaration } => (declaration, Vec::new()),
                    TypeForm::Applied {
                        declaration,
                        arguments,
                    } => (declaration, arguments),
                    _ => return Err(invalid("borrowed nominal type is not an Owned record")),
                };
                if field.package != declaration.package
                    || field_record.declaration != declaration.declaration
                {
                    return Err(invalid("borrowed field changes exact nominal declaration"));
                }
                let owned_record = self.declaration(declaration)?;
                let DeclarationPayload::OwnedRecord { fields, .. } = &owned_record.payload else {
                    return Err(invalid(
                        "OwnedChild field path selects a non-Owned nominal declaration",
                    ));
                };
                if !fields.contains(&field.field) {
                    return Err(invalid(
                        "OwnedChild field is outside the complete nominal field inventory",
                    ));
                }
                let callable = context
                    .owner
                    .ok_or_else(|| invalid("Owned field operation callable absent"))?;
                if callable.package != declaration.package
                    || self.declaration(callable)?.module != owned_record.module
                {
                    return Err(invalid(
                        "foreign module accesses a private Owned reference field",
                    ));
                }
                let substitution = self.bind_types(declaration, &arguments, context)?;
                let ty = self.substitute(field_record.ty, &substitution, 0)?;
                self.scan_owned_loan(
                    ty,
                    source,
                    binding,
                    body,
                    context,
                    locals,
                    syntax,
                    demand,
                    depth + 1,
                )?
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                self.scan(index, context, locals, syntax, Use::Value, depth + 1)?;
                let source = self.scan(source, context, locals, syntax, Use::Borrow, depth + 1)?;
                if source.ty != Some(sequence_type) {
                    return Err(invalid(
                        "sequence path source differs from exact Owned sequence type",
                    ));
                }
                let TypeForm::OwnedSequence { item } = self.object(sequence_type)?.form else {
                    return Err(invalid("borrow sequence type changed"));
                };
                self.scan_owned_loan(
                    item,
                    source,
                    binding,
                    body,
                    context,
                    locals,
                    syntax,
                    demand,
                    depth + 1,
                )?
            }
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                self.annotated(sequence_type, context)?
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                self.facts(sequence_type, context, 0)?;
                self.scan(source, context, locals, syntax, Use::Borrow, depth + 1)?;
                self.scalar(TypeForm::I64)?
            }
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            } => {
                self.scan(index, context, locals, syntax, Use::Value, depth + 1)?;
                self.scan(source, context, locals, syntax, Use::Borrow, depth + 1)?;
                let TypeForm::OwnedSequence { item } = self.object(sequence_type)?.form else {
                    return Err(invalid("sequence get source type changed"));
                };
                let facts = self.facts(item, context, 0)?;
                if facts.category != ValueCategory::OrdinaryData {
                    return Err(invalid("sequence get copies an affine value"));
                }
                self.annotated(item, context)?
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                self.scan(value, context, locals, syntax, Use::Consume, depth + 1)?;
                self.scan(source, context, locals, syntax, Use::Consume, depth + 1)?;
                self.annotated(sequence_type, context)?
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                self.facts(sequence_type, context, 0)?;
                self.scan(source, context, locals, syntax, Use::Consume, depth + 1)?;
                self.annotated(result_type, context)?
            }
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            } => {
                self.facts(sequence_type, context, 0)?;
                self.scan(index, context, locals, syntax, Use::Value, depth + 1)?;
                self.scan(value, context, locals, syntax, Use::Consume, depth + 1)?;
                self.scan(source, context, locals, syntax, Use::Consume, depth + 1)?;
                self.annotated(result_type, context)?
            }
            ExpressionOperation::Parallel { left, right } => {
                self.budget.reserve(2)?;
                let mut fields = Vec::new();
                let mut owned = false;
                for (label, expression) in [("left", left), ("right", right)] {
                    let record = self.expression(package, expression)?;
                    let function = match record.operation {
                        ExpressionOperation::Call { function, .. }
                        | ExpressionOperation::ImplementationCall { function, .. } => function,
                        _ => return Err(invalid("joined child is not an exact named task call")),
                    };
                    let declaration = self.declaration(function)?;
                    if !matches!(declaration.payload, DeclarationPayload::Function(ref f) if matches!(f.effect, FunctionEffect::Task { .. }) && f.result.relation == ResultRelation::Value)
                    {
                        return Err(invalid("joined child kind/result relationship changed"));
                    }
                    syntax.joined_calls.insert((package, expression));
                    let result =
                        self.scan(expression, context, locals, syntax, Use::Value, depth + 1)?;
                    syntax.joined_calls.remove(&(package, expression));
                    let ty = result.ty.ok_or_else(|| {
                        invalid("joined result type was not independently reconstructed")
                    })?;
                    let facts = self.facts(ty, context, 0)?;
                    if !facts.transfer
                        || !matches!(
                            facts.category,
                            ValueCategory::OrdinaryData | ValueCategory::Owned
                        )
                    {
                        return Err(invalid(
                            "joined child result is not independently transferable",
                        ));
                    }
                    owned |= facts.category == ValueCategory::Owned;
                    fields.push(StructuralTypeField {
                        name: kernel::Name::new(label)
                            .map_err(|_| invalid("invalid joined field identity"))?,
                        ty,
                    });
                }
                let ty = self.intern(if owned {
                    TypeForm::OwnedProduct { fields }
                } else {
                    TypeForm::StructuralRecord { fields }
                })?;
                self.annotated(ty, context)?
            }
            _ => {
                return Err(invalid(
                    "canonical expression has no independent all-path view interpretation",
                ));
            }
        };
        if self.value_category(&value, context)? == ValueCategory::ReadView {
            return Err(invalid("ordinary syntax produces/copies an affine view"));
        }
        Ok(value)
    }
    fn scan_owned_loan(
        &mut self,
        ty: TypeObjectDigest,
        source: Value,
        binding: BindingId,
        body: ExpressionId,
        context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        demand: Use,
        depth: usize,
    ) -> Result<Value> {
        if self.facts(ty, context, 0)?.category != ValueCategory::Owned {
            return Err(invalid(
                "Owned child loan changes to view/ordinary category",
            ));
        }
        let mut nested = context.clone();
        nested.lexical.insert(binding);
        let mut scopes = source.scopes;
        let callable = context
            .owner
            .ok_or_else(|| invalid("loan callable absent"))?;
        let declaration = self.proof.declaration(callable)?;
        if matches!(declaration.payload, DeclarationPayload::Function(_))
            && declaration.header.contract_version
                >= kernel::contract::READ_VIEW_GRAPH_CONTRACT_VERSION
        {
            scopes.insert(ReadScopeArgument::Lexical { callable, binding });
        }
        let value = Value {
            ty: Some(ty),
            source: source.source,
            scopes,
            read: true,
        };
        self.binding(
            binding,
            &value,
            &nested,
            locals,
            syntax,
            Some(BindingKind::OwnedBorrow),
        )?;
        let mut result = self.scan(body, &nested, locals, syntax, demand, depth + 1)?;
        locals.remove(&LocalValueReference::LexicalBinding(binding));
        // This permission is valid only inside the loan body. Keep the exact
        // returned source/read/type and all enclosing source scopes intact.
        result
            .scopes
            .remove(&ReadScopeArgument::Lexical { callable, binding });
        Ok(result)
    }
}

fn substitute_scope(scope: ReadScopeArgument, substitution: &Substitution) -> ReadScopeArgument {
    match scope {
        ReadScopeArgument::Parameter(formal) => {
            substitution.scopes.get(&formal).copied().unwrap_or(scope)
        }
        _ => scope,
    }
}

fn raw_view_metadata(record: &OwnerRecord) -> bool {
    match record {
        OwnerRecord::Binding(binding) => matches!(
            binding.kind,
            BindingKind::ReadView | BindingKind::ReadViewBorrow
        ),
        OwnerRecord::Expression(expression) => {
            expression.operation.is_read_view_operation()
                || !expression.operation.read_scope_arguments().is_empty()
        }
        OwnerRecord::Declaration(declaration) => {
            match &declaration.payload {
                DeclarationPayload::ReadView(_) => true,
                DeclarationPayload::Function(function) => {
                    !function.read_scope_parameters.is_empty()
                        || !function.read_scope_sources.is_empty()
                        || matches!(function.result.relation, ResultRelation::ViewFrom { .. })
                }
                DeclarationPayload::OwnedContract(contract) => {
                    contract
                        .associated_types
                        .iter()
                        .any(|m| matches!(m.bound, AssociatedTypeBound::ReadViewFamily { .. }))
                        || contract.methods.iter().any(|m| {
                            !m.read_scope_parameters.is_empty()
                                || !m.read_scope_sources.is_empty()
                                || matches!(m.result.relation, ResultRelation::ViewFrom { .. })
                        })
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.associated_types.iter().any(|b| {
                        matches!(b.value, AssociatedTypeBindingValue::ReadViewFamily { .. })
                    }) || implementation
                        .methods
                        .iter()
                        .any(|m| !m.read_scope_arguments.is_empty())
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn substitute_relation(
    relation: ResultRelation<u32>,
    substitution: &Substitution,
) -> ResultRelation<u32> {
    match relation {
        ResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
            source,
            scope: substitute_scope(scope, substitution),
        },
        _ => relation,
    }
}

fn unique<T: Ord>(values: &[T], what: &str) -> Result<()> {
    if values.len() > kernel::contract::MAXIMUM_CHILDREN
        || values.iter().collect::<BTreeSet<_>>().len() != values.len()
    {
        Err(invalid(format!(
            "{what} must have a bounded complete distinct inventory"
        )))
    } else {
        Ok(())
    }
}

fn common_scope_permissions(
    left: &mut Value,
    right: &Value,
    budget: &mut Budget<'_>,
) -> Result<()> {
    // Owned read scopes are available permissions, not new source identities.
    // A join can retain only permission proved on every path. This never
    // invents a common source or widens a scope; view types keep exact scopes.
    for _ in &left.scopes {
        budget.step(0)?;
    }
    left.scopes.retain(|scope| right.scopes.contains(scope));
    Ok(())
}

fn compare_branch_values(left: &mut Value, right: &Value, budget: &mut Budget<'_>) -> Result<()> {
    if left.ty.is_some() && right.ty.is_some() && left.ty != right.ty {
        return Err(invalid("match branches change canonical type"));
    }
    if (left.read || right.read) && (left.source != right.source || left.read != right.read) {
        return Err(invalid(
            "inactive match arm changes borrowed source lineage",
        ));
    }
    // These matches belong to scan_ordinary, which rejects ReadView results.
    if left.read {
        common_scope_permissions(left, right, budget)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "reference_read_view_admission_tests.rs"]
mod tests;
