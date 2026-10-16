//! Independent closed application derivation from canonical snapshots.
//! Prepared member arrays, type lookup, and production substitution are never
//! inputs to this derivation. This is preparation correspondence, not an ownership
//! or finite-growth admission token; complete schema proof and source/artifact admission
//! are separate boundaries.

use super::super::reference_effects;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel;
use crate::platform::kernel::{
    AssociatedTypeBindingValue, AssociatedTypeBound, CallableResult, DeclarationPayload,
    DeclarationReference, EffectParameterReference, EffectRow, ExpressionOperation,
    ImplementationOperand, ImplementationParameter, KernelSnapshot, LocalValueReference, OwnerKey,
    OwnerRecord, PackageId, ParameterParent, ParameterUse, ReadScopeArgument, ReadScopeBinder,
    ReadScopeParameter, ReadScopeReference, ReadScopeSource, RequirementOperand,
    RequirementParameterReference, RequirementReference, ResultRelation, TypeForm, TypeObject,
    TypeObjectDigest, TypeParameterConstraints, encode_type_object,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ExpressionId, ImplementationParameterId, MethodId, TypeParameterId,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

struct AuthoredCallApplication<'a> {
    types: &'a [TypeObjectDigest],
    implementations: &'a [ImplementationOperand],
    effects: &'a [EffectRow],
    requirements: &'a [RequirementOperand],
    scopes: &'a [ReadScopeArgument],
}

type Ordinary = BTreeMap<TypeParameterId, TypeObjectDigest>;
type Witnesses = BTreeMap<ImplementationParameterId, Arc<Selection>>;
type SelectionKey = (DeclarationReference, Vec<TypeObjectDigest>, Vec<usize>);
type SelectionIndex = BTreeMap<
    DeclarationReference,
    BTreeMap<Vec<TypeObjectDigest>, BTreeMap<Vec<usize>, Arc<Selection>>>,
>;
type Assumptions = BTreeMap<TypeParameterId, TypeParameterConstraints>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in super::super) struct Application {
    pub function: DeclarationReference,
    pub types: Vec<TypeObjectDigest>,
    pub witnesses: Vec<usize>,
    pub effects: Vec<EffectRow>,
    pub requirements: Vec<RequirementReference>,
    pub read_scope_arguments: Vec<ReadScopeArgument>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in super::super) enum ReadContext {
    Application(Application),
    Symbolic(DeclarationReference),
    Executable((PackageId, OwnerKey)),
}

#[derive(Debug, Eq, PartialEq)]
pub(in super::super) struct ScopedSignature {
    pub parameters: Vec<(TypeObjectDigest, ParameterUse)>,
    pub result: CallableResult<u32>,
    pub read_scope_parameters: Vec<ReadScopeParameter>,
    pub read_scope_sources: Vec<ReadScopeSource<u32>>,
    pub effect: EffectRow,
    pub is_task: bool,
    pub requires_associated_generation: bool,
    pub requires_read_view_generation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in super::super) struct Member {
    pub member: AssociatedTypeMemberId,
    pub template: TypeObjectDigest,
    pub ty: TypeObjectDigest,
    pub scope_parameter: Option<ReadScopeReference>,
}

#[derive(Debug)]
pub(in super::super) struct Selection {
    pub identity: usize,
    pub depth: usize,
    pub implementation: DeclarationReference,
    pub contract: DeclarationReference,
    pub implementation_generation: u16,
    pub contract_generation: u16,
    pub requires_read_view_generation: bool,
    pub actuals: Vec<TypeObjectDigest>,
    pub parameters: Vec<TypeParameterId>,
    pub prerequisite_parameters: Vec<ImplementationParameterId>,
    pub constraints: Vec<TypeParameterConstraints>,
    pub self_type: TypeObjectDigest,
    pub self_template: TypeObjectDigest,
    pub contract_argument_templates: Vec<TypeObjectDigest>,
    pub prerequisite_templates: Vec<ImplementationParameter>,
    pub contract_actuals: Vec<TypeObjectDigest>,
    pub contract_constraints: Vec<TypeParameterConstraints>,
    pub prerequisites: Arc<[Arc<Selection>]>,
    pub members: Vec<Member>,
}

#[derive(Debug, Eq, PartialEq)]
pub(in super::super) struct Invocation {
    pub callee: ExpressionId,
    pub arguments: Vec<ExpressionId>,
}

#[derive(Debug)]
pub(in super::super) struct Function {
    pub is_task: bool,
    pub requires_associated_generation: bool,
    pub requires_read_view_generation: bool,
    pub implementation_templates: Vec<ImplementationParameter>,
    pub parameters: Vec<(TypeObjectDigest, ParameterUse)>,
    pub result: CallableResult<u32>,
    pub read_scope_parameters: Vec<ReadScopeParameter>,
    pub read_scope_sources: Vec<ReadScopeSource<u32>>,
    pub effect: EffectRow,
    pub roots: BTreeMap<(OwnerKey, TypeObjectDigest), TypeObjectDigest>,
    pub targets: BTreeMap<ExpressionId, Application>,
    pub invocations: BTreeMap<ExpressionId, Invocation>,
    pub occurrences: BTreeMap<OwnerKey, u16>,
    pub symbolic_targets: BTreeMap<ExpressionId, SymbolicTarget>,
    pub associated_targets: BTreeSet<ExpressionId>,
    pub read_view_targets: BTreeSet<ExpressionId>,
}

// One body traversal populates these parts of the same canonical function
// inventory. Group their borrows without allocating or changing source custody.
struct BodyOutputs<'a> {
    roots: &'a mut BTreeMap<(OwnerKey, TypeObjectDigest), TypeObjectDigest>,
    targets: &'a mut BTreeMap<ExpressionId, Application>,
    invocations: &'a mut BTreeMap<ExpressionId, Invocation>,
    occurrences: &'a mut BTreeMap<OwnerKey, u16>,
    symbolic_targets: &'a mut BTreeMap<ExpressionId, SymbolicTarget>,
    associated_targets: &'a mut BTreeSet<ExpressionId>,
    read_view_targets: &'a mut BTreeSet<ExpressionId>,
}

#[derive(Debug)]
pub(in super::super) struct SymbolicSelection {
    pub source: SymbolicSelectionSource,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub arguments: Vec<TypeObjectDigest>,
}

#[derive(Debug)]
pub(in super::super) enum SymbolicSelectionSource {
    Parameter(DeclarationReference, ImplementationParameterId),
    Concrete {
        implementation: DeclarationReference,
        actuals: Vec<TypeObjectDigest>,
        prerequisites: Vec<Arc<SymbolicSelection>>,
        members: Vec<Member>,
    },
}

#[derive(Debug)]
pub(in super::super) struct SymbolicTarget {
    pub signature: ScopedSignature,
    pub function: Option<DeclarationReference>,
    pub method: Option<(DeclarationReference, MethodId)>,
    pub types: Vec<TypeObjectDigest>,
    pub witnesses: Vec<Arc<SymbolicSelection>>,
    pub effects: Vec<EffectRow>,
    pub requirements: Vec<RequirementOperand>,
    pub read_scope_arguments: Vec<ReadScopeArgument>,
}

#[derive(Debug)]
pub(in super::super) struct Proof {
    pub types: BTreeMap<TypeObjectDigest, TypeObject>,
    pub selections: Vec<Arc<Selection>>,
    pub applications: BTreeMap<Application, Function>,
    pub methods: BTreeMap<(usize, MethodId, Vec<ReadScopeArgument>), Application>,
    contract_templates: BTreeMap<DeclarationReference, crate::platform::kernel::OwnedContract>,
    implementation_inventory: BTreeSet<DeclarationReference>,
    mapping_templates: BTreeMap<
        (DeclarationReference, MethodId),
        crate::platform::kernel::OwnedMethodImplementation,
    >,
    pub symbolic: BTreeMap<DeclarationReference, Function>,
    pub symbolic_mappings: BTreeMap<(DeclarationReference, MethodId), SymbolicTarget>,
    pub contract_methods: BTreeMap<(DeclarationReference, MethodId), Function>,
    pub executables: BTreeMap<(PackageId, OwnerKey), Function>,
    selection_index: SelectionIndex,
    type_identities: BTreeSet<TypeObjectDigest>,
    pub raw_templates: BTreeSet<TypeObjectDigest>,
    pub raw_read_view_templates: BTreeSet<TypeObjectDigest>,
    symbolic_formals:
        BTreeMap<(DeclarationReference, ImplementationParameterId), ImplementationParameter>,
    #[cfg(test)]
    pub flow_work: crate::platform::kernel::associated_flow_canonical_reference::ReferenceFlowWork,
    pub steps: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy)]
pub(in super::super) struct Limits {
    pub steps: usize,
    pub bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            steps: crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
            bytes: 256 * 1024 * 1024,
        }
    }
}

struct Budget<'a> {
    steps: usize,
    bytes: usize,
    limits: Limits,
    control: &'a ExecutionControl,
}
impl Budget<'_> {
    fn tick(&mut self) -> Result<(), ExecutionError> {
        self.control.check()?;
        let next = self.steps.checked_add(1).ok_or_else(capacity)?;
        if next > self.limits.steps {
            return Err(capacity());
        }
        self.steps = next;
        Ok(())
    }
    fn reserve<T>(&mut self, count: usize) -> Result<(), ExecutionError> {
        self.control.check()?;
        let next = count
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|added| self.bytes.checked_add(added))
            .ok_or_else(capacity)?;
        if next > self.limits.bytes {
            return Err(capacity());
        }
        self.bytes = next;
        Ok(())
    }
    fn node<T>(&mut self) -> Result<(), ExecutionError> {
        self.reserve::<T>(1)?;
        self.reserve::<usize>(3)
    }
    fn object(&mut self, object: &TypeObject) -> Result<(), ExecutionError> {
        match &object.form {
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                self.reserve::<crate::platform::kernel::StructuralTypeField>(fields.len())?;
                for field in fields {
                    self.reserve::<u8>(field.name.as_str().len())?;
                }
            }
            TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
                self.reserve::<TypeObjectDigest>(arguments.len())?
            }
            TypeForm::Function { parameters, .. } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?
            }
            TypeForm::TaskFunction {
                parameters, effect, ..
            } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?;
                self.row(effect)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn row(&mut self, row: &EffectRow) -> Result<(), ExecutionError> {
        self.reserve::<RequirementOperand>(row.requirements.len())?;
        self.reserve::<EffectParameterReference>(row.parameters.len())
    }

    fn validate_row(&mut self, row: &EffectRow) -> Result<(), ExecutionError> {
        for _ in &row.requirements {
            self.tick()?;
        }
        for _ in &row.parameters {
            self.tick()?;
        }
        row.validate().map_err(|_| reject())
    }
}

#[derive(Default)]
struct Environment {
    scope: Option<DeclarationReference>,
    source_package: Option<PackageId>,
    generation: u16,
    lexical: bool,
    symbolic: bool,
    ordinary: Ordinary,
    read_scopes: BTreeMap<ReadScopeReference, ReadScopeArgument>,
    witnesses: Witnesses,
    symbolic_witnesses: BTreeMap<ImplementationParameterId, Arc<SymbolicSelection>>,
    symbolic_contract: Option<Arc<SymbolicSelection>>,
    symbolic_effects: BTreeMap<EffectParameterReference, EffectRow>,
    symbolic_requirements: BTreeMap<RequirementParameterReference, RequirementOperand>,
    contract: Option<Arc<Selection>>,
    effects: reference_effects::Bindings,
    requirements: reference_effects::RequirementBindings,
}

pub(in super::super) struct Entry<'a> {
    pub function: DeclarationReference,
    pub types: &'a [TypeObjectDigest],
    pub implementations: &'a [ImplementationOperand],
    pub effects: &'a [EffectRow],
    pub requirements: &'a [RequirementReference],
    pub read_scope_arguments: &'a [ReadScopeArgument],
}

struct Derivation<'a> {
    snapshots: BTreeMap<PackageId, &'a KernelSnapshot>,
    budget: Budget<'a>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    interned: BTreeMap<SelectionKey, Arc<Selection>>,
    selections: Vec<Arc<Selection>>,
    pending: VecDeque<Application>,
    queued: BTreeSet<Application>,
    applications: BTreeMap<Application, Function>,
    methods: BTreeMap<(usize, MethodId, Vec<ReadScopeArgument>), Application>,
    contract_templates: BTreeMap<DeclarationReference, crate::platform::kernel::OwnedContract>,
    implementation_inventory: BTreeSet<DeclarationReference>,
    mapping_templates: BTreeMap<
        (DeclarationReference, MethodId),
        crate::platform::kernel::OwnedMethodImplementation,
    >,
    symbolic: BTreeMap<DeclarationReference, Function>,
    symbolic_mappings: BTreeMap<(DeclarationReference, MethodId), SymbolicTarget>,
    symbolic_formals:
        BTreeMap<(DeclarationReference, ImplementationParameterId), ImplementationParameter>,
    contract_methods: BTreeMap<(DeclarationReference, MethodId), Function>,
    executables: BTreeMap<(PackageId, OwnerKey), Function>,
    #[cfg(test)]
    flow_work: crate::platform::kernel::associated_flow_canonical_reference::ReferenceFlowWork,
}

fn reject() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_associated_preparation",
        "canonical associated application has a mismatched scope, inventory, binding, or target",
    )
}
fn capacity() -> ExecutionError {
    ExecutionError::resource(
        "reference_associated_preparation_capacity",
        "independent canonical associated closure exceeded its finite preparation capacity",
    )
}

impl<'a> Derivation<'a> {
    fn new(
        snapshots: &[&'a KernelSnapshot],
        limits: Limits,
        control: &'a ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        let mut result = Self {
            snapshots: BTreeMap::new(),
            budget: Budget {
                steps: 0,
                bytes: 0,
                limits,
                control,
            },
            types: BTreeMap::new(),
            interned: BTreeMap::new(),
            selections: Vec::new(),
            pending: VecDeque::new(),
            queued: BTreeSet::new(),
            applications: BTreeMap::new(),
            methods: BTreeMap::new(),
            contract_templates: BTreeMap::new(),
            implementation_inventory: BTreeSet::new(),
            mapping_templates: BTreeMap::new(),
            symbolic: BTreeMap::new(),
            symbolic_mappings: BTreeMap::new(),
            symbolic_formals: BTreeMap::new(),
            contract_methods: BTreeMap::new(),
            executables: BTreeMap::new(),
            #[cfg(test)]
            flow_work: Default::default(),
        };
        for snapshot in snapshots {
            result.budget.tick()?;
            result.budget.node::<(PackageId, &KernelSnapshot)>()?;
            if result
                .snapshots
                .insert(snapshot.root.package_id, snapshot)
                .is_some()
            {
                return Err(reject());
            }
            // This entry also admits private fixture inventories directly;
            // establish the source-root coverage that public admission checks.
            for owner in snapshot.owners.values() {
                result.budget.tick()?;
                if owner.header().contract_version != snapshot.root.graph_contract_version {
                    return Err(reject());
                }
            }
            for (ty, object) in snapshot.types.iter().chain(&snapshot.dependency_types) {
                result.budget.tick()?;
                if encode_type_object(object).map_err(|_| reject())?.0 != *ty {
                    return Err(reject());
                }
                if let Some(previous) = result.types.get(ty) {
                    if previous != object {
                        return Err(reject());
                    }
                } else {
                    result.budget.node::<(TypeObjectDigest, TypeObject)>()?;
                    result.budget.object(object)?;
                    result.types.insert(*ty, object.clone());
                }
            }
        }
        Ok(result)
    }

    fn owner(
        &mut self,
        package: PackageId,
        key: OwnerKey,
    ) -> Result<&'a OwnerRecord, ExecutionError> {
        self.budget.tick()?;
        let owner = self
            .snapshots
            .get(&package)
            .and_then(|snapshot| snapshot.owners.get(&key))
            .ok_or_else(reject)?;
        if owner.header().owner != key {
            return Err(reject());
        }
        super::admit_parameter_generation(owner)?;
        Ok(owner)
    }

    fn declaration(
        &mut self,
        reference: DeclarationReference,
        from: Option<PackageId>,
    ) -> Result<&'a crate::platform::kernel::DeclarationRecord, ExecutionError> {
        let OwnerRecord::Declaration(owner) = self.owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )?
        else {
            return Err(reject());
        };
        if let Some(from) = from.filter(|package| *package != reference.package) {
            self.budget.tick()?;
            let source = self.snapshots.get(&from).ok_or_else(reject)?;
            let dependency = source
                .dependencies
                .get(&reference.package)
                .ok_or_else(reject)?;
            let interface = source
                .dependency_interfaces
                .get(&dependency.package_revision)
                .and_then(|interface| interface.get(&OwnerKey::Declaration(reference.declaration)))
                .ok_or_else(reject)?;
            if dependency.package != reference.package
                || owner.visibility != crate::platform::kernel::DeclarationVisibility::Public
                || !matches!(
                    interface,
                    crate::platform::kernel::PackageInterfaceRecord::Declaration(_)
                )
            {
                return Err(reject());
            }
        }
        Ok(owner)
    }

    fn current(
        &mut self,
        scope: DeclarationReference,
        generation: u16,
    ) -> Result<(), ExecutionError> {
        self.budget.tick()?;
        if generation != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || self
                .snapshots
                .get(&scope.package)
                .ok_or_else(reject)?
                .root
                .graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || self.declaration(scope, None)?.header.contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(reject());
        }
        Ok(())
    }

    fn associated_generation(
        &mut self,
        scope: DeclarationReference,
        generation: u16,
        read_view: bool,
    ) -> Result<(), ExecutionError> {
        if read_view {
            return self.current(scope, generation);
        }
        self.budget.tick()?;
        let root_generation = self
            .snapshots
            .get(&scope.package)
            .ok_or_else(reject)?
            .root
            .graph_contract_version;
        let owner_generation = self.declaration(scope, None)?.header.contract_version;
        if generation != root_generation
            || owner_generation != root_generation
            || !super::associated_generation_admitted(generation, false)
            || !super::associated_generation_admitted(root_generation, false)
            || !super::associated_generation_admitted(owner_generation, false)
        {
            return Err(reject());
        }
        Ok(())
    }

    // Call records belong to the caller's package. A concrete supplier has its
    // own independently checked root; never compare its generation to this one.
    // Port inventories have no declaration binder, but still have an exact
    // package and occurrence generation.
    fn call_generation(
        &mut self,
        caller: &Environment,
        read_view: bool,
    ) -> Result<(), ExecutionError> {
        if let Some(scope) = caller.scope {
            if caller
                .source_package
                .is_some_and(|package| package != scope.package)
            {
                return Err(reject());
            }
            return self.associated_generation(scope, caller.generation, read_view);
        }
        self.budget.tick()?;
        let package = caller.source_package.ok_or_else(reject)?;
        let root = self
            .snapshots
            .get(&package)
            .ok_or_else(reject)?
            .root
            .graph_contract_version;
        if !super::associated_generation_admitted(root, read_view)
            || !super::associated_generation_admitted(caller.generation, read_view)
            || caller.generation != root
        {
            return Err(reject());
        }
        Ok(())
    }

    fn intern_type(&mut self, form: TypeForm) -> Result<TypeObjectDigest, ExecutionError> {
        let object = TypeObject::new(form).map_err(|_| reject())?;
        let ty = encode_type_object(&object).map_err(|_| reject())?.0;
        self.budget.tick()?;
        if let Some(previous) = self.types.get(&ty) {
            if previous != &object {
                return Err(reject());
            }
        } else {
            self.budget.node::<(TypeObjectDigest, TypeObject)>()?;
            self.types.insert(ty, object);
        }
        Ok(ty)
    }

    fn ty(
        &mut self,
        ty: TypeObjectDigest,
        env: &Environment,
        depth: usize,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = self.types.get(&ty).ok_or_else(reject)?;
        self.budget.object(object)?;
        let form = object.form.clone();
        let form = match form {
            TypeForm::TypeParameter { parameter } => {
                let scope = env.scope.ok_or_else(reject)?;
                let OwnerRecord::TypeParameter(owner) =
                    self.owner(scope.package, OwnerKey::TypeParameter(parameter))?
                else {
                    return Err(reject());
                };
                let mut present = 0;
                for id in self
                    .declaration(scope, None)?
                    .payload
                    .type_parameters()
                    .iter()
                {
                    self.budget.tick()?;
                    present += usize::from(*id == parameter);
                }
                if owner.declaration != scope.declaration || present != 1 {
                    return Err(reject());
                }
                // Actuals are complete caller values; do not traverse them under
                // the source effect or witness environment.
                return env.ordinary.get(&parameter).copied().ok_or_else(reject);
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                if !env.lexical || env.scope != Some(scope) {
                    return Err(reject());
                }
                self.associated_generation(scope, env.generation, false)?;
                let inventory = match &self.declaration(scope, None)?.payload {
                    DeclarationPayload::Function(function) => &function.implementation_parameters,
                    DeclarationPayload::OwnedImplementation(scheme) => {
                        &scheme.implementation_parameters
                    }
                    _ => return Err(reject()),
                };
                let mut present = 0;
                for formal in inventory {
                    self.budget.tick()?;
                    present += usize::from(formal.id == parameter && formal.contract == contract);
                }
                if present != 1 {
                    return Err(reject());
                }
                if env.symbolic {
                    self.associated_bound(ty)?;
                    return self.symbolic_member(
                        env.symbolic_witnesses.get(&parameter).ok_or_else(reject)?,
                        contract,
                        member,
                    );
                }
                let selection = env.witnesses.get(&parameter).ok_or_else(reject)?;
                return self.member(selection, contract, member);
            }
            TypeForm::AssociatedContractMember { contract, member } => {
                if env.scope != Some(contract) || env.lexical {
                    return Err(reject());
                }
                self.associated_generation(contract, env.generation, false)?;
                if env.symbolic {
                    self.associated_bound(ty)?;
                    if let Some(selected) = &env.symbolic_contract {
                        return self.symbolic_member(selected, contract, member);
                    }
                    return Ok(ty);
                }
                return self.member(env.contract.as_ref().ok_or_else(reject)?, contract, member);
            }
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                let owner = self.declaration(
                    declaration,
                    env.scope.map(|scope| scope.package).or(env.source_package),
                )?;
                let DeclarationPayload::ReadView(view) = &owner.payload else {
                    return Err(reject());
                };
                if owner.header.contract_version
                    != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                    || arguments.len() != view.type_parameters.len()
                {
                    return Err(reject());
                }
                TypeForm::ReadViewApplied {
                    declaration,
                    arguments: arguments
                        .into_iter()
                        .map(|ty| self.ty(ty, env, depth + 1))
                        .collect::<Result<_, _>>()?,
                    read_scope: self.read_scope(read_scope, env)?,
                }
            }
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                if !env.lexical || env.scope != Some(scope) {
                    return Err(reject());
                }
                self.current(scope, env.generation)?;
                self.associated_bound(ty)?;
                let read_scope = self.read_scope(read_scope, env)?;
                if env.symbolic {
                    return self.symbolic_view_member(
                        env.symbolic_witnesses.get(&parameter).ok_or_else(reject)?,
                        contract,
                        member,
                        read_scope,
                    );
                }
                return self.view_member(
                    env.witnesses.get(&parameter).ok_or_else(reject)?,
                    contract,
                    member,
                    read_scope,
                );
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                if env.scope != Some(contract) || env.lexical {
                    return Err(reject());
                }
                self.current(contract, env.generation)?;
                self.associated_bound(ty)?;
                let read_scope = self.read_scope(read_scope, env)?;
                if env.symbolic {
                    if let Some(selected) = &env.symbolic_contract {
                        return self.symbolic_view_member(selected, contract, member, read_scope);
                    }
                    return self.intern_type(TypeForm::AssociatedReadViewMember {
                        contract,
                        member,
                        read_scope,
                    });
                }
                return self.view_member(
                    env.contract.as_ref().ok_or_else(reject)?,
                    contract,
                    member,
                    read_scope,
                );
            }
            TypeForm::Applied {
                declaration,
                arguments,
            } => TypeForm::Applied {
                declaration,
                arguments: arguments
                    .into_iter()
                    .map(|ty| self.ty(ty, env, depth + 1))
                    .collect::<Result<_, _>>()?,
            },
            TypeForm::Region { region } => TypeForm::Region {
                region: self.ty(region, env, depth + 1)?,
            },
            TypeForm::RelPtr { region, item } => TypeForm::RelPtr {
                region: self.ty(region, env, depth + 1)?,
                item: self.ty(item, env, depth + 1)?,
            },
            TypeForm::RegionSlice { region, item } => TypeForm::RegionSlice {
                region: self.ty(region, env, depth + 1)?,
                item: self.ty(item, env, depth + 1)?,
            },
            TypeForm::RegionCap { region, mode } => TypeForm::RegionCap {
                region: self.ty(region, env, depth + 1)?,
                mode: match mode {
                    kernel::RegionMode::Shared { scope } => kernel::RegionMode::Shared {
                        scope: self.read_scope(scope, env)?,
                    },
                    other => other,
                },
            },
            TypeForm::List { item } => TypeForm::List {
                item: self.ty(item, env, depth + 1)?,
            },
            TypeForm::OwnedSequence { item } => TypeForm::OwnedSequence {
                item: self.ty(item, env, depth + 1)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: self.ty(item, env, depth + 1)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: self.ty(item, env, depth + 1)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: self.ty(key, env, depth + 1)?,
                value: self.ty(value, env, depth + 1)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: self.ty(ok, env, depth + 1)?,
                error: self.ty(error, env, depth + 1)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: parameters
                    .into_iter()
                    .map(|ty| self.ty(ty, env, depth + 1))
                    .collect::<Result<_, _>>()?,
                result: self.ty(result, env, depth + 1)?,
            },
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => {
                // Close the authored row before inserting opaque caller types.
                let effect = self.row(&effect, env)?;
                TypeForm::TaskFunction {
                    parameters: parameters
                        .into_iter()
                        .map(|ty| self.ty(ty, env, depth + 1))
                        .collect::<Result<_, _>>()?,
                    result: self.ty(result, env, depth + 1)?,
                    effect,
                }
            }
            TypeForm::OwnedChoice { mut cases } => {
                for case in &mut cases {
                    case.ty = self.ty(case.ty, env, depth + 1)?;
                }
                TypeForm::OwnedChoice { cases }
            }
            TypeForm::OwnedProduct { mut fields } => {
                for field in &mut fields {
                    field.ty = self.ty(field.ty, env, depth + 1)?;
                }
                TypeForm::OwnedProduct { fields }
            }
            TypeForm::StructuralRecord { mut fields } => {
                for field in &mut fields {
                    field.ty = self.ty(field.ty, env, depth + 1)?;
                }
                TypeForm::StructuralRecord { fields }
            }
            form => form,
        };
        self.intern_type(form)
    }

    fn row(&mut self, row: &EffectRow, env: &Environment) -> Result<EffectRow, ExecutionError> {
        self.budget.tick()?;
        if env.symbolic {
            return self.symbolic_row(row, env);
        }
        reference_effects::close(row, &env.effects, &env.requirements, |count| {
            self.budget.reserve::<RequirementReference>(count)
        })
    }

    fn member(
        &mut self,
        selection: &Selection,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.budget.tick()?;
        if selection.contract != contract {
            return Err(reject());
        }
        let owner = self.declaration(contract, Some(selection.implementation.package))?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        if contract
            .associated_types
            .iter()
            .filter(|value| value.id == member)
            .count()
            != 1
        {
            return Err(reject());
        }
        selection
            .members
            .iter()
            .find(|binding| binding.member == member)
            .map(|binding| binding.ty)
            .ok_or_else(reject)
    }

    fn view_type(&mut self, ty: TypeObjectDigest) -> Result<bool, ExecutionError> {
        self.budget.tick()?;
        Ok(matches!(
            self.types.get(&ty).ok_or_else(reject)?.form,
            TypeForm::ReadViewApplied { .. }
                | TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
        ))
    }

    fn apply_view_member(
        &mut self,
        member: &Member,
        read_scope: ReadScopeArgument,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        let formal = member.scope_parameter.ok_or_else(reject)?;
        let TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope: template_scope,
        } = self.types.get(&member.ty).ok_or_else(reject)?.form.clone()
        else {
            return Err(reject());
        };
        if template_scope != ReadScopeArgument::Parameter(formal) {
            return Err(reject());
        }
        self.budget.reserve::<TypeObjectDigest>(arguments.len())?;
        self.intern_type(TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope,
        })
    }

    fn view_member(
        &mut self,
        selection: &Selection,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        read_scope: ReadScopeArgument,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        if selection.contract != contract {
            return Err(reject());
        }
        let binding = selection
            .members
            .iter()
            .find(|binding| binding.member == member)
            .ok_or_else(reject)?;
        self.apply_view_member(binding, read_scope)
    }

    fn ordinary(
        &mut self,
        reference: DeclarationReference,
        formals: &[TypeParameterId],
        actuals: &[TypeObjectDigest],
    ) -> Result<Ordinary, ExecutionError> {
        if formals.len() != actuals.len() {
            return Err(reject());
        }
        let mut result = BTreeMap::new();
        let mut names = BTreeSet::new();
        for (formal, actual) in formals.iter().zip(actuals) {
            let OwnerRecord::TypeParameter(owner) =
                self.owner(reference.package, OwnerKey::TypeParameter(*formal))?
            else {
                return Err(reject());
            };
            if owner.declaration != reference.declaration
                || names.contains(&owner.name)
                || result.contains_key(formal)
            {
                return Err(reject());
            }
            let constraints = owner.constraints;
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            names.insert(&owner.name);
            if self.view_type(*actual)? {
                return Err(reject());
            }
            let owned = self.classify(*actual, false, 0, &mut BTreeSet::new())?;
            if owned != constraints.has_owned() {
                return Err(reject());
            }
            // Movement/share promises are independently checked by full schema
            // admission. This correspondence path cannot supply those permissions.
            result.insert(*formal, *actual);
        }
        Ok(result)
    }

    fn constraints(
        &mut self,
        scope: DeclarationReference,
        parameters: &[TypeParameterId],
    ) -> Result<Vec<TypeParameterConstraints>, ExecutionError> {
        self.budget
            .reserve::<TypeParameterConstraints>(parameters.len())?;
        parameters
            .iter()
            .map(|parameter| {
                let OwnerRecord::TypeParameter(owner) =
                    self.owner(scope.package, OwnerKey::TypeParameter(*parameter))?
                else {
                    return Err(reject());
                };
                if owner.declaration != scope.declaration {
                    return Err(reject());
                }
                Ok(owner.constraints)
            })
            .collect()
    }

    fn assumptions<'b>(
        &mut self,
        scope: DeclarationReference,
        parameters: impl IntoIterator<Item = &'b TypeParameterId>,
    ) -> Result<Assumptions, ExecutionError> {
        let mut result = BTreeMap::new();
        let mut names = BTreeSet::new();
        for parameter in parameters {
            let OwnerRecord::TypeParameter(owner) =
                self.owner(scope.package, OwnerKey::TypeParameter(*parameter))?
            else {
                return Err(reject());
            };
            if owner.declaration != scope.declaration
                || result.contains_key(parameter)
                || names.contains(&owner.name)
                || (owner.constraints.has_owned()
                    && owner.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION)
                || (owner.constraints.requires_transfer()
                    && owner.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION)
                || (owner.constraints.requires_share()
                    && owner.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION)
            {
                return Err(reject());
            }
            self.budget
                .node::<(TypeParameterId, TypeParameterConstraints)>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            result.insert(*parameter, owner.constraints);
            names.insert(&owner.name);
        }
        Ok(result)
    }

    fn witness_bindings(
        &mut self,
        scope: DeclarationReference,
        formals: &[ImplementationParameter],
        supplied: &[Arc<Selection>],
        env: &mut Environment,
    ) -> Result<(), ExecutionError> {
        if formals.len() != supplied.len() || !env.witnesses.is_empty() {
            return Err(reject());
        }
        let mut names = BTreeSet::new();
        for (formal, selected) in formals.iter().zip(supplied) {
            self.budget.tick()?;
            let TypeForm::TypeParameter { parameter } =
                self.types.get(&formal.self_type).ok_or_else(reject)?.form
            else {
                return Err(reject());
            };
            let OwnerRecord::TypeParameter(owner) =
                self.owner(scope.package, OwnerKey::TypeParameter(parameter))?
            else {
                return Err(reject());
            };
            if owner.declaration != scope.declaration
                || !owner.constraints.has_owned()
                || names.contains(&formal.name)
                || env.witnesses.contains_key(&formal.id)
            {
                return Err(reject());
            }
            self.budget
                .node::<(ImplementationParameterId, Arc<Selection>)>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            names.insert(&formal.name);
            env.witnesses.insert(formal.id, Arc::clone(selected));
        }
        // Formal arguments may refer to their own or a later supplied witness's member.
        // Every selected value was closed in the caller before entering this environment.
        for (formal, selected) in formals.iter().zip(supplied) {
            self.budget.tick()?;
            if self.ty(formal.self_type, env, 0)? != selected.self_type
                || formal.contract != selected.contract
                || formal.type_arguments.len() != selected.contract_actuals.len()
            {
                return Err(reject());
            }
            for (expected, actual) in formal.type_arguments.iter().zip(&selected.contract_actuals) {
                if self.ty(*expected, env, 0)? != *actual {
                    return Err(reject());
                }
            }
        }
        Ok(())
    }

    fn operand(
        &mut self,
        operand: &ImplementationOperand,
        caller: &Environment,
        depth: usize,
    ) -> Result<Arc<Selection>, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let ImplementationOperand::Concrete {
            implementation,
            type_arguments,
            implementations,
        } = operand
        else {
            let ImplementationOperand::Parameter { scope, parameter } = operand else {
                unreachable!()
            };
            if caller.scope != Some(*scope) || !caller.lexical {
                return Err(reject());
            }
            let inventory = match &self.declaration(*scope, None)?.payload {
                DeclarationPayload::Function(function) => &function.implementation_parameters,
                DeclarationPayload::OwnedImplementation(scheme) => {
                    &scheme.implementation_parameters
                }
                _ => return Err(reject()),
            };
            if inventory
                .iter()
                .filter(|formal| formal.id == *parameter)
                .count()
                != 1
            {
                return Err(reject());
            }
            let selected = caller.witnesses.get(parameter).ok_or_else(reject)?;
            self.associated_generation(
                *scope,
                caller.generation,
                selected.requires_read_view_generation,
            )?;
            if depth.saturating_add(selected.depth)
                > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
            {
                return Err(capacity());
            }
            return Ok(Arc::clone(selected));
        };
        let owner = self.declaration(
            *implementation,
            caller
                .scope
                .map(|scope| scope.package)
                .or(caller.source_package),
        )?;
        let DeclarationPayload::OwnedImplementation(scheme) = &owner.payload else {
            return Err(reject());
        };
        if scheme.type_parameters.len() != type_arguments.len()
            || scheme.implementation_parameters.len() != implementations.len()
        {
            return Err(reject());
        }
        let requires_read_view_generation = self.read_view_owner(*implementation)?;
        if !scheme.associated_types.is_empty() || requires_read_view_generation {
            self.associated_generation(
                *implementation,
                owner.header.contract_version,
                requires_read_view_generation,
            )?;
        }
        self.budget
            .reserve::<TypeObjectDigest>(type_arguments.len())?;
        let actuals = type_arguments
            .iter()
            .map(|ty| self.ty(*ty, caller, 0))
            .collect::<Result<Vec<_>, _>>()?;
        self.budget
            .reserve::<Arc<Selection>>(implementations.len())?;
        let prerequisites = implementations
            .iter()
            .map(|operand| self.operand(operand, caller, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        self.budget.reserve::<TypeObjectDigest>(actuals.len())?;
        self.budget.reserve::<usize>(prerequisites.len())?;
        let key = (
            *implementation,
            actuals.clone(),
            prerequisites.iter().map(|value| value.identity).collect(),
        );
        let subtree_depth = prerequisites
            .iter()
            .map(|value| value.depth.saturating_add(1))
            .max()
            .unwrap_or(0);
        if depth.saturating_add(subtree_depth)
            > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
        {
            return Err(capacity());
        }
        if let Some(selected) = self.interned.get(&key) {
            return Ok(Arc::clone(selected));
        }
        let mut env = Environment {
            scope: Some(*implementation),
            generation: owner.header.contract_version,
            lexical: true,
            ordinary: self.ordinary(*implementation, &scheme.type_parameters, &actuals)?,
            ..Environment::default()
        };
        self.witness_bindings(
            *implementation,
            &scheme.implementation_parameters,
            &prerequisites,
            &mut env,
        )?;
        let self_type = self.ty(scheme.self_type, &env, 0)?;
        if !self.classify(self_type, false, 0, &mut BTreeSet::new())? {
            return Err(reject());
        }
        self.budget
            .reserve::<TypeObjectDigest>(scheme.type_arguments.len())?;
        let contract_actuals = scheme
            .type_arguments
            .iter()
            .map(|ty| self.ty(*ty, &env, 0))
            .collect::<Result<Vec<_>, _>>()?;
        let contract_owner = self.declaration(scheme.contract, Some(implementation.package))?;
        let DeclarationPayload::OwnedContract(contract) = &contract_owner.payload else {
            return Err(reject());
        };
        self.budget
            .reserve::<TypeParameterId>(contract.type_parameters.len() + 1)?;
        let formals = std::iter::once(contract.self_parameter)
            .chain(contract.type_parameters.iter().copied())
            .collect::<Vec<_>>();
        self.budget
            .reserve::<TypeObjectDigest>(contract_actuals.len() + 1)?;
        let actual = std::iter::once(self_type)
            .chain(contract_actuals.iter().copied())
            .collect::<Vec<_>>();
        self.ordinary(scheme.contract, &formals, &actual)?;
        self.inventory(*implementation, scheme)?;
        self.budget
            .reserve::<Member>(scheme.associated_types.len())?;
        let mut members = Vec::new();
        env.lexical = false;
        for member in &contract.associated_types {
            let binding = scheme
                .associated_types
                .iter()
                .find(|binding| binding.member == member.id)
                .ok_or_else(reject)?;
            // Stored RHS gets only its own formals and no associated authority.
            let ty = self.ty(binding.ty(), &env, 0)?;
            if matches!(binding.value, AssociatedTypeBindingValue::Owned { .. })
                && !self.classify(ty, false, 0, &mut BTreeSet::new())?
            {
                return Err(reject());
            }
            members.push(Member {
                member: binding.member,
                template: binding.ty(),
                ty,
                scope_parameter: match &binding.value {
                    AssociatedTypeBindingValue::Owned { .. } => None,
                    AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter, ..
                    } => Some(ReadScopeReference {
                        binder: ReadScopeBinder::AssociatedBinding {
                            implementation: *implementation,
                            member: binding.member,
                        },
                        parameter: scope_parameter.id,
                    }),
                },
            });
        }
        self.budget.reserve::<Arc<Selection>>(1)?;
        self.budget.node::<Selection>()?;
        self.budget
            .reserve::<TypeParameterId>(scheme.type_parameters.len())?;
        self.budget
            .reserve::<ImplementationParameterId>(scheme.implementation_parameters.len())?;
        let parameters = scheme.type_parameters.clone();
        let prerequisite_parameters = scheme
            .implementation_parameters
            .iter()
            .map(|formal| formal.id)
            .collect();
        let constraints = self.constraints(*implementation, &parameters)?;
        let contract_constraints = self.constraints(scheme.contract, &contract.type_parameters)?;
        self.budget
            .reserve::<TypeObjectDigest>(scheme.type_arguments.len())?;
        self.budget
            .reserve::<ImplementationParameter>(scheme.implementation_parameters.len())?;
        for formal in &scheme.implementation_parameters {
            self.budget.tick()?;
            self.budget.reserve::<u8>(formal.name.as_str().len())?;
            self.budget
                .reserve::<TypeObjectDigest>(formal.type_arguments.len())?;
        }
        let selected = Arc::new(Selection {
            identity: self.selections.len(),
            depth: subtree_depth,
            implementation: *implementation,
            contract: scheme.contract,
            actuals,
            self_type,
            self_template: scheme.self_type,
            contract_argument_templates: scheme.type_arguments.clone(),
            prerequisite_templates: scheme.implementation_parameters.clone(),
            implementation_generation: owner.header.contract_version,
            contract_generation: contract_owner.header.contract_version,
            requires_read_view_generation: requires_read_view_generation
                || prerequisites
                    .iter()
                    .any(|child| child.requires_read_view_generation),
            parameters,
            prerequisite_parameters,
            constraints,
            contract_actuals,
            contract_constraints,
            prerequisites: prerequisites.into(),
            members,
        });
        self.budget.node::<(SelectionKey, Arc<Selection>)>()?;
        self.interned.insert(key, Arc::clone(&selected));
        self.selections.push(Arc::clone(&selected));
        Ok(selected)
    }

    fn inventory(
        &mut self,
        implementation: DeclarationReference,
        scheme: &crate::platform::kernel::OwnedImplementation,
    ) -> Result<(), ExecutionError> {
        let owner = self.declaration(scheme.contract, Some(implementation.package))?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        if contract.associated_types.len() != scheme.associated_types.len() {
            return Err(reject());
        }
        let read_view = self.read_view_contract(scheme.contract)?;
        if !contract.associated_types.is_empty() || read_view {
            self.associated_generation(scheme.contract, owner.header.contract_version, read_view)?;
        }
        let implementation_owner = self.declaration(implementation, None)?;
        let implementation_view = self.read_view_owner(implementation)?;
        if !scheme.associated_types.is_empty() || implementation_view {
            self.associated_generation(
                implementation,
                implementation_owner.header.contract_version,
                implementation_view,
            )?;
        }
        let mut members = BTreeSet::new();
        let mut names = BTreeSet::new();
        for member in &contract.associated_types {
            self.budget.tick()?;
            self.budget.node::<AssociatedTypeMemberId>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            if !members.insert(member.id) || !names.insert(&member.name) {
                return Err(reject());
            }
        }
        let assumptions = self.assumptions(implementation, &scheme.type_parameters)?;
        let mut previous = None;
        for binding in &scheme.associated_types {
            self.budget.tick()?;
            if previous.is_some_and(|value| value >= binding.member)
                || !members.remove(&binding.member)
            {
                return Err(reject());
            }
            previous = Some(binding.member);
            self.stored(binding.ty(), implementation, &scheme.type_parameters, 0)?;
            let member = contract
                .associated_types
                .iter()
                .find(|member| member.id == binding.member)
                .ok_or_else(reject)?;
            match (&member.bound, &binding.value) {
                (AssociatedTypeBound::Owned, AssociatedTypeBindingValue::Owned { .. }) => {}
                (
                    AssociatedTypeBound::ReadViewFamily { .. },
                    AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter,
                        ty,
                    },
                ) => {
                    let formal = ReadScopeArgument::Parameter(ReadScopeReference {
                        binder: ReadScopeBinder::AssociatedBinding {
                            implementation,
                            member: binding.member,
                        },
                        parameter: scope_parameter.id,
                    });
                    if !matches!(self.types.get(ty).ok_or_else(reject)?.form, TypeForm::ReadViewApplied { read_scope, .. } if read_scope == formal)
                    {
                        return Err(reject());
                    }
                    continue;
                }
                _ => return Err(reject()),
            }
            if !self.classify_scoped(binding.ty(), false, 0, &mut BTreeSet::new(), &assumptions)? {
                return Err(reject());
            }
        }
        if !members.is_empty() {
            return Err(reject());
        }
        Ok(())
    }

    fn stored(
        &mut self,
        ty: TypeObjectDigest,
        scope: DeclarationReference,
        parameters: &[TypeParameterId],
        depth: usize,
    ) -> Result<(), ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = self.types.get(&ty).ok_or_else(reject)?;
        self.budget.object(object)?;
        let object = object.clone();
        if object.form.is_associated() {
            return Err(reject());
        }
        if let TypeForm::ReadViewApplied { read_scope, .. } = object.form {
            self.read_scope(
                read_scope,
                &Environment {
                    scope: Some(scope),
                    symbolic: true,
                    ..Environment::default()
                },
            )?;
        }
        if let TypeForm::TypeParameter { parameter } = object.form {
            if !parameters.contains(&parameter) {
                return Err(reject());
            }
            let OwnerRecord::TypeParameter(owner) =
                self.owner(scope.package, OwnerKey::TypeParameter(parameter))?
            else {
                return Err(reject());
            };
            if owner.declaration != scope.declaration {
                return Err(reject());
            }
            return Ok(());
        }
        self.budget
            .reserve::<TypeObjectDigest>(object.child_type_count())?;
        let children = object.child_types();
        for child in children {
            self.stored(child, scope, parameters, depth + 1)?;
        }
        Ok(())
    }

    fn classify(
        &mut self,
        ty: TypeObjectDigest,
        data: bool,
        depth: usize,
        active: &mut BTreeSet<TypeObjectDigest>,
    ) -> Result<bool, ExecutionError> {
        self.classify_scoped(ty, data, depth, active, &Assumptions::new())
    }

    fn classify_scoped(
        &mut self,
        ty: TypeObjectDigest,
        data: bool,
        depth: usize,
        active: &mut BTreeSet<TypeObjectDigest>,
        assumptions: &Assumptions,
    ) -> Result<bool, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = self.types.get(&ty).ok_or_else(reject)?;
        self.budget.object(object)?;
        let form = object.form.clone();
        match form {
            TypeForm::TypeParameter { parameter } => assumptions
                .get(&parameter)
                .map(|constraints| constraints.has_owned())
                .ok_or_else(reject),
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
                if !assumptions.is_empty() =>
            {
                self.associated_bound(ty)?;
                Ok(true)
            }
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => Err(reject()),
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => {
                if data {
                    Err(reject())
                } else {
                    Ok(false)
                }
            }
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Ok(true),
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                let mut owned = false;
                for field in fields {
                    owned |=
                        self.classify_scoped(field.ty, true, depth + 1, active, assumptions)?;
                }
                if !owned {
                    return Err(reject());
                }
                Ok(true)
            }
            TypeForm::OwnedSequence { item } => {
                self.classify_scoped(item, true, depth + 1, active, assumptions)?;
                Ok(true)
            }
            TypeForm::Named { declaration } => {
                self.nominal(ty, declaration, &[], depth, active, assumptions)
            }
            TypeForm::Applied {
                declaration,
                arguments,
            } => self.nominal(ty, declaration, &arguments, depth, active, assumptions),
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                if data {
                    return Err(reject());
                }
                for parameter in parameters {
                    self.classify_scoped(parameter, false, depth + 1, active, assumptions)?;
                }
                self.classify_scoped(result, false, depth + 1, active, assumptions)?;
                Ok(false)
            }
            TypeForm::Secret | TypeForm::Stream { .. } | TypeForm::CapabilityResource { .. }
                if data =>
            {
                Err(reject())
            }
            _ => {
                let object = self.types.get(&ty).ok_or_else(reject)?;
                self.budget
                    .reserve::<TypeObjectDigest>(object.child_type_count())?;
                let children = object.child_types();
                for child in children {
                    if self.classify_scoped(child, data, depth + 1, active, assumptions)? {
                        return Err(reject());
                    }
                }
                Ok(false)
            }
        }
    }

    fn nominal(
        &mut self,
        ty: TypeObjectDigest,
        reference: DeclarationReference,
        arguments: &[TypeObjectDigest],
        depth: usize,
        active: &mut BTreeSet<TypeObjectDigest>,
        assumptions: &Assumptions,
    ) -> Result<bool, ExecutionError> {
        // Private layout evidence is read in the nominal declaration's own
        // context. It supplies no authored field access to any function.
        let owner = self.declaration(reference, None)?;
        let (formals, fields, cases, owned) = match &owner.payload {
            DeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } => (type_parameters.as_slice(), fields.as_slice(), &[][..], true),
            DeclarationPayload::Record {
                type_parameters,
                fields,
            } => (
                type_parameters.as_slice(),
                fields.as_slice(),
                &[][..],
                false,
            ),
            DeclarationPayload::Variant {
                type_parameters,
                cases,
            } => (type_parameters.as_slice(), &[][..], cases.as_slice(), false),
            _ => return Err(reject()),
        };
        if active.contains(&ty) {
            return if owned { Err(reject()) } else { Ok(false) };
        }
        self.budget.node::<TypeObjectDigest>()?;
        active.insert(ty);
        let mut ordinary = BTreeMap::new();
        if formals.len() != arguments.len() {
            return Err(reject());
        }
        for (formal, actual) in formals.iter().zip(arguments) {
            let OwnerRecord::TypeParameter(parameter) =
                self.owner(reference.package, OwnerKey::TypeParameter(*formal))?
            else {
                return Err(reject());
            };
            if parameter.declaration != reference.declaration || ordinary.contains_key(formal) {
                return Err(reject());
            }
            let actual_owned =
                self.classify_scoped(*actual, false, depth + 1, active, assumptions)?;
            if actual_owned != parameter.constraints.has_owned() {
                return Err(reject());
            }
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            ordinary.insert(*formal, *actual);
        }
        let mut env = Environment {
            scope: Some(reference),
            generation: owner.header.contract_version,
            ordinary,
            symbolic: !assumptions.is_empty(),
            ..Environment::default()
        };
        for field in fields {
            let OwnerRecord::Field(field) =
                self.owner(reference.package, OwnerKey::Field(*field))?
            else {
                return Err(reject());
            };
            if field.declaration != reference.declaration {
                return Err(reject());
            }
            env.generation = field.header.contract_version;
            let field = self.ty(field.ty, &env, 0)?;
            if self.classify_scoped(field, true, depth + 1, active, assumptions)? && !owned {
                return Err(reject());
            }
        }
        for case in cases {
            let OwnerRecord::Case(case) = self.owner(reference.package, OwnerKey::Case(*case))?
            else {
                return Err(reject());
            };
            if case.declaration != reference.declaration {
                return Err(reject());
            }
            if let Some(payload) = case.payload {
                env.generation = case.header.contract_version;
                let payload = self.ty(payload, &env, 0)?;
                if self.classify_scoped(payload, true, depth + 1, active, assumptions)? {
                    return Err(reject());
                }
            }
        }
        active.remove(&ty);
        Ok(owned)
    }

    fn enqueue(
        &mut self,
        function: DeclarationReference,
        types: Vec<TypeObjectDigest>,
        witnesses: Vec<Arc<Selection>>,
        effects: Vec<EffectRow>,
        requirements: Vec<RequirementReference>,
        read_scope_arguments: Vec<ReadScopeArgument>,
    ) -> Result<Application, ExecutionError> {
        self.budget.tick()?;
        self.budget.reserve::<usize>(witnesses.len())?;
        let application = Application {
            function,
            types,
            witnesses: witnesses.iter().map(|value| value.identity).collect(),
            effects,
            requirements,
            read_scope_arguments,
        };
        if !self.queued.contains(&application) {
            self.budget.node::<Application>()?;
            self.budget.reserve::<Application>(1)?;
            self.charge_application(&application)?;
            self.charge_application(&application)?;
            self.queued.insert(application.clone());
            self.pending.push_back(application.clone());
        }
        Ok(application)
    }

    fn charge_application(&mut self, application: &Application) -> Result<(), ExecutionError> {
        self.budget
            .reserve::<ReadScopeArgument>(application.read_scope_arguments.len())?;
        self.budget
            .reserve::<TypeObjectDigest>(application.types.len())?;
        self.budget.reserve::<usize>(application.witnesses.len())?;
        self.budget
            .reserve::<EffectRow>(application.effects.len())?;
        for row in &application.effects {
            self.budget.row(row)?;
        }
        self.budget
            .reserve::<RequirementReference>(application.requirements.len())
    }

    fn environment(&mut self, application: &Application) -> Result<Environment, ExecutionError> {
        let owner = self.declaration(application.function, None)?;
        let (types, implementations, effect_parameters, requirement_parameters) =
            match &owner.payload {
                DeclarationPayload::Function(function) => (
                    &function.type_parameters[..],
                    &function.implementation_parameters[..],
                    &function.effect_parameters[..],
                    &function.requirement_parameters[..],
                ),
                DeclarationPayload::External(function) => {
                    (&function.type_parameters[..], &[][..], &[][..], &[][..])
                }
                _ => return Err(reject()),
            };
        if effect_parameters.len() != application.effects.len()
            || requirement_parameters.len() != application.requirements.len()
            || implementations.len() != application.witnesses.len()
        {
            return Err(reject());
        }
        let mut env = Environment {
            scope: Some(application.function),
            generation: owner.header.contract_version,
            lexical: matches!(owner.payload, DeclarationPayload::Function(_)),
            ordinary: self.ordinary(application.function, types, &application.types)?,
            ..Environment::default()
        };
        let scope_parameters = match &owner.payload {
            DeclarationPayload::Function(function) => &function.read_scope_parameters[..],
            _ => &[][..],
        };
        self.scope_bindings(
            ReadScopeBinder::Declaration(application.function),
            scope_parameters,
            &application.read_scope_arguments,
            &mut env,
        )?;
        // Establish authored row closure before ordinary/witness signature roots.
        for (formal, actual) in effect_parameters.iter().zip(&application.effects) {
            if !actual.is_closed() || actual.validate().is_err() {
                return Err(reject());
            }
            let OwnerRecord::EffectParameter(owner) = self.owner(
                application.function.package,
                OwnerKey::EffectParameter(*formal),
            )?
            else {
                return Err(reject());
            };
            if owner.declaration != application.function.declaration {
                return Err(reject());
            }
            self.budget
                .node::<(EffectParameterReference, EffectRow)>()?;
            self.budget.row(actual)?;
            if env
                .effects
                .insert(
                    EffectParameterReference {
                        package: application.function.package,
                        parameter: *formal,
                    },
                    actual.clone(),
                )
                .is_some()
            {
                return Err(reject());
            }
        }
        for (formal, actual) in requirement_parameters.iter().zip(&application.requirements) {
            let OwnerRecord::RequirementParameter(owner) = self.owner(
                application.function.package,
                OwnerKey::RequirementParameter(*formal),
            )?
            else {
                return Err(reject());
            };
            if owner.declaration != application.function.declaration {
                return Err(reject());
            }
            self.budget
                .node::<(RequirementParameterReference, RequirementReference)>()?;
            if env
                .requirements
                .insert(
                    RequirementParameterReference {
                        package: application.function.package,
                        parameter: *formal,
                    },
                    *actual,
                )
                .is_some()
            {
                return Err(reject());
            }
        }
        self.budget
            .reserve::<Arc<Selection>>(application.witnesses.len())?;
        let supplied = application
            .witnesses
            .iter()
            .map(|id| self.selections.get(*id).cloned().ok_or_else(reject))
            .collect::<Result<Vec<_>, _>>()?;
        self.witness_bindings(application.function, implementations, &supplied, &mut env)?;
        Ok(env)
    }

    fn root(
        &mut self,
        roots: &mut BTreeMap<(OwnerKey, TypeObjectDigest), TypeObjectDigest>,
        consumer: OwnerKey,
        ty: TypeObjectDigest,
        env: &Environment,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        let actual = self.ty(ty, env, 0)?;
        if roots
            .get(&(consumer, ty))
            .is_some_and(|previous| *previous != actual)
        {
            return Err(reject());
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = roots.entry((consumer, ty)) {
            self.budget
                .node::<((OwnerKey, TypeObjectDigest), TypeObjectDigest)>()?;
            entry.insert(actual);
        }
        Ok(actual)
    }

    pub(super) fn implementation_templates(
        &mut self,
        scope: DeclarationReference,
    ) -> Result<Vec<ImplementationParameter>, ExecutionError> {
        let owner = self.declaration(scope, None)?;
        let formals = match &owner.payload {
            DeclarationPayload::Function(function) => &function.implementation_parameters[..],
            DeclarationPayload::External(_) => &[],
            _ => return Err(reject()),
        };
        self.budget
            .reserve::<ImplementationParameter>(formals.len())?;
        for formal in formals {
            self.budget.tick()?;
            self.budget.reserve::<u8>(formal.name.as_str().len())?;
            self.budget
                .reserve::<TypeObjectDigest>(formal.type_arguments.len())?;
        }
        Ok(formals.to_vec())
    }

    fn function(&mut self, application: &Application) -> Result<Function, ExecutionError> {
        let owner = self.declaration(application.function, None)?;
        let (parameters, result, scope_parameters, scope_sources, effect, body) =
            match &owner.payload {
                DeclarationPayload::Function(function) => (
                    &function.parameters,
                    function.result,
                    &function.read_scope_parameters[..],
                    &function.read_scope_sources[..],
                    function.effect.row(),
                    Some(function.body),
                ),
                DeclarationPayload::External(function) => (
                    &function.parameters,
                    CallableResult {
                        ty: function.result,
                        relation: ResultRelation::Value,
                    },
                    &[][..],
                    &[][..],
                    crate::platform::kernel::FunctionEffect::Pure.row(),
                    None,
                ),
                _ => return Err(reject()),
            };
        let mut env = self.environment(application)?;
        let effect = self.row(&effect, &env)?;
        let mut roots = BTreeMap::new();
        let mut occurrences = BTreeMap::new();
        self.occurrence(
            &mut occurrences,
            application.function.package,
            OwnerKey::Declaration(application.function.declaration),
        )?;
        let result = self.root(
            &mut roots,
            OwnerKey::Declaration(application.function.declaration),
            result.ty,
            &env,
        )?;
        self.budget
            .reserve::<(TypeObjectDigest, ParameterUse)>(parameters.len())?;
        let mut actual_parameters = Vec::new();
        for parameter in parameters {
            self.occurrence(
                &mut occurrences,
                application.function.package,
                OwnerKey::Parameter(*parameter),
            )?;
            let OwnerRecord::Parameter(record) = self.owner(
                application.function.package,
                OwnerKey::Parameter(*parameter),
            )?
            else {
                return Err(reject());
            };
            if record.parent != ParameterParent::Function(application.function.declaration) {
                return Err(reject());
            }
            env.generation = record.header.contract_version;
            // The same signature type has a separate parameter-owner occurrence.
            let ty = self.root(&mut roots, OwnerKey::Parameter(*parameter), record.ty, &env)?;
            env.generation = owner.header.contract_version;
            self.root(
                &mut roots,
                OwnerKey::Declaration(application.function.declaration),
                record.ty,
                &env,
            )?;
            actual_parameters.push((ty, record.use_mode));
        }
        let ordinal = |source| {
            parameters
                .iter()
                .position(|id| *id == source)
                .and_then(|position| u32::try_from(position).ok())
                .ok_or_else(reject)
        };
        let result = self.closed_result(
            match &owner.payload {
                DeclarationPayload::Function(function) => function.result,
                DeclarationPayload::External(function) => CallableResult {
                    ty: function.result,
                    relation: ResultRelation::Value,
                },
                _ => return Err(reject()),
            },
            result,
            &env,
            ordinal,
        )?;
        let read_scope_sources = self.closed_sources(scope_sources, &env, ordinal)?;
        let mut targets = BTreeMap::new();
        let mut invocations = BTreeMap::new();
        let mut symbolic_targets = BTreeMap::new();
        let mut associated_targets = BTreeSet::new();
        let mut read_view_targets = BTreeSet::new();
        if let Some(body) = body {
            self.body(
                application.function.package,
                body,
                &mut env,
                BodyOutputs {
                    roots: &mut roots,
                    targets: &mut targets,
                    invocations: &mut invocations,
                    occurrences: &mut occurrences,
                    symbolic_targets: &mut symbolic_targets,
                    associated_targets: &mut associated_targets,
                    read_view_targets: &mut read_view_targets,
                },
            )?;
        }
        let mut requires_read_view_generation = self.read_view_owner(application.function)?;
        requires_read_view_generation |= self.read_view_type(result.ty, 0)?;
        for (ty, _) in &actual_parameters {
            requires_read_view_generation |= self.read_view_type(*ty, 0)?;
        }
        for identity in &application.witnesses {
            requires_read_view_generation |= self
                .selections
                .get(*identity)
                .ok_or_else(reject)?
                .requires_read_view_generation;
        }
        let requires_associated_generation =
            self.associated_function(application.function)? || requires_read_view_generation;
        if requires_associated_generation {
            self.associated_generation(
                application.function,
                owner.header.contract_version,
                requires_read_view_generation,
            )?;
        }
        Ok(Function {
            is_task: matches!(&owner.payload, DeclarationPayload::Function(function) if matches!(function.effect, crate::platform::kernel::FunctionEffect::Task { .. })),
            requires_associated_generation,
            requires_read_view_generation,
            implementation_templates: self.implementation_templates(application.function)?,
            parameters: actual_parameters,
            result,
            read_scope_parameters: self.copy_scope_parameters(scope_parameters)?,
            read_scope_sources,
            effect,
            roots,
            targets,
            invocations,
            occurrences,
            symbolic_targets,
            associated_targets,
            read_view_targets,
        })
    }

    fn local_roots(
        &mut self,
        package: PackageId,
        expression: ExpressionId,
        value: LocalValueReference,
        env: &mut Environment,
        roots: &mut BTreeMap<(OwnerKey, TypeObjectDigest), TypeObjectDigest>,
        occurrences: &mut BTreeMap<OwnerKey, u16>,
    ) -> Result<(), ExecutionError> {
        let key = match value {
            LocalValueReference::FunctionParameter(parameter)
            | LocalValueReference::OperationParameter(parameter) => OwnerKey::Parameter(parameter),
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding)
            | LocalValueReference::TransactionBinding(binding) => OwnerKey::Binding(binding),
        };
        self.occurrence(occurrences, package, key)?;
        let local = self.owner(package, key)?;
        if let LocalValueReference::FunctionParameter(parameter) = value {
            let scope = env.scope.ok_or_else(reject)?;
            let OwnerRecord::Parameter(owner) = local else {
                return Err(reject());
            };
            let DeclarationPayload::Function(function) = &self.declaration(scope, None)?.payload
            else {
                return Err(reject());
            };
            if owner.parent != ParameterParent::Function(scope.declaration)
                || function
                    .parameters
                    .iter()
                    .filter(|candidate| **candidate == parameter)
                    .count()
                    != 1
            {
                return Err(reject());
            }
        }
        let generation = env.generation;
        self.budget
            .reserve::<TypeObjectDigest>(local.type_roots().len())?;
        for ty in local.type_roots() {
            self.root(roots, OwnerKey::Expression(expression), ty, env)?;
            env.generation = local.header().contract_version;
            self.root(roots, key, ty, env)?;
            env.generation = generation;
        }
        Ok(())
    }

    fn body(
        &mut self,
        package: PackageId,
        root: ExpressionId,
        env: &mut Environment,
        outputs: BodyOutputs<'_>,
    ) -> Result<(), ExecutionError> {
        let BodyOutputs {
            roots,
            targets,
            invocations,
            occurrences,
            symbolic_targets,
            associated_targets,
            read_view_targets,
        } = outputs;
        env.source_package = Some(package);
        self.budget.reserve::<ExpressionId>(1)?;
        let mut pending = vec![root];
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            self.budget.tick()?;
            if visited.contains(&id) {
                continue;
            }
            self.budget.node::<ExpressionId>()?;
            visited.insert(id);
            self.occurrence(occurrences, package, OwnerKey::Expression(id))?;
            let OwnerRecord::Expression(record) = self.owner(package, OwnerKey::Expression(id))?
            else {
                return Err(reject());
            };
            env.generation = record.contract_version;
            if record.operation.is_read_view_operation()
                || !record.operation.read_scope_arguments().is_empty()
            {
                self.current(env.scope.ok_or_else(reject)?, record.contract_version)?;
            }
            let embedded_local = match &record.operation {
                ExpressionOperation::CreateReadView {
                    source,
                    read_scope,
                    declaration,
                    ..
                } => {
                    let owner = self.declaration(*declaration, Some(package))?;
                    let DeclarationPayload::ReadView(_) = &owner.payload else {
                        return Err(reject());
                    };
                    self.current(*declaration, owner.header.contract_version)?;
                    self.read_scope(*read_scope, env)?;
                    Some(*source)
                }
                ExpressionOperation::ReborrowReadView { value, read_scope } => {
                    self.read_scope(*read_scope, env)?;
                    Some(*value)
                }
                ExpressionOperation::BorrowReadView { value, .. }
                | ExpressionOperation::MoveReadView { value }
                | ExpressionOperation::DropReadView { value } => Some(*value),
                ExpressionOperation::ReadViewDataField { value, field }
                | ExpressionOperation::BorrowReadViewField { value, field, .. } => {
                    let declaration = self.declaration(field.declaration, Some(package))?;
                    let DeclarationPayload::ReadView(view) = &declaration.payload else {
                        return Err(reject());
                    };
                    self.current(field.declaration, declaration.header.contract_version)?;
                    if view
                        .fields
                        .iter()
                        .filter(|candidate| candidate.id == field.field)
                        .count()
                        != 1
                    {
                        return Err(reject());
                    }
                    Some(*value)
                }
                _ => None,
            };
            if let Some(value) = embedded_local {
                self.local_roots(package, id, value, env, roots, occurrences)?;
            }
            if let ExpressionOperation::Invoke { callee, arguments } = &record.operation {
                self.budget.node::<(ExpressionId, Invocation)>()?;
                self.budget.reserve::<ExpressionId>(arguments.len())?;
                for _ in arguments {
                    self.budget.tick()?;
                }
                if invocations
                    .insert(
                        id,
                        Invocation {
                            callee: *callee,
                            arguments: arguments.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(reject());
                }
            }
            self.budget
                .reserve::<TypeObjectDigest>(record.type_roots().len())?;
            for ty in record.type_roots() {
                self.root(roots, OwnerKey::Expression(id), ty, env)?;
            }
            let bindings: Vec<_> = match &record.operation {
                ExpressionOperation::Let { bindings, .. } => {
                    self.budget
                        .reserve::<crate::platform::semantic_id::BindingId>(bindings.len())?;
                    bindings.clone()
                }
                ExpressionOperation::BorrowCall { binding, .. }
                | ExpressionOperation::BorrowOwnedItem { binding, .. }
                | ExpressionOperation::BorrowOwnedField { binding, .. }
                | ExpressionOperation::BorrowOwnedRecordField { binding, .. }
                | ExpressionOperation::BorrowReadView { binding, .. }
                | ExpressionOperation::BorrowReadViewField { binding, .. } => {
                    self.budget
                        .reserve::<crate::platform::semantic_id::BindingId>(1)?;
                    vec![*binding]
                }
                ExpressionOperation::UnpackOwned { fields, .. } => {
                    self.budget
                        .reserve::<crate::platform::semantic_id::BindingId>(fields.len())?;
                    fields.iter().map(|field| field.binding).collect()
                }
                ExpressionOperation::UnpackOwnedRecord { fields, .. } => {
                    self.budget
                        .reserve::<crate::platform::semantic_id::BindingId>(fields.len())?;
                    fields.iter().map(|field| field.binding).collect()
                }
                ExpressionOperation::MatchOwned { arms, .. }
                | ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                    self.budget
                        .reserve::<crate::platform::semantic_id::BindingId>(arms.len())?;
                    arms.iter().map(|arm| arm.binding).collect()
                }
                ExpressionOperation::Match { arms, .. } => {
                    self.budget
                        .reserve::<crate::platform::semantic_id::BindingId>(arms.len())?;
                    arms.iter().filter_map(|arm| arm.payload_binding).collect()
                }
                ExpressionOperation::Local { value } => {
                    self.local_roots(package, id, *value, env, roots, occurrences)?;
                    Vec::new()
                }
                _ => Vec::new(),
            };
            for binding in bindings {
                let OwnerRecord::Binding(record) =
                    self.owner(package, OwnerKey::Binding(binding))?
                else {
                    return Err(reject());
                };
                self.occurrence(occurrences, package, OwnerKey::Binding(binding))?;
                env.generation = record.header.contract_version;
                if let Some(ty) = record.declared_type {
                    self.root(roots, OwnerKey::Binding(binding), ty, env)?;
                }
                if let Some(value) = record.value {
                    self.budget.reserve::<ExpressionId>(1)?;
                    pending.push(value);
                }
            }
            env.generation = record.contract_version;
            let symbolic = if env.symbolic {
                self.symbolic_target(&record.operation, env)?
            } else {
                None
            };
            if let Some(target) = symbolic {
                if (target.signature.requires_associated_generation
                    || target.signature.requires_read_view_generation)
                    && !associated_targets.contains(&id)
                {
                    self.budget.node::<ExpressionId>()?;
                    associated_targets.insert(id);
                }
                if target.signature.requires_read_view_generation
                    && !read_view_targets.contains(&id)
                {
                    self.budget.node::<ExpressionId>()?;
                    read_view_targets.insert(id);
                }
                self.budget.node::<(ExpressionId, SymbolicTarget)>()?;
                symbolic_targets.insert(id, target);
            }
            let target = match &record.operation {
                _ if env.symbolic => None,
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                } => Some(self.call(
                    *function,
                    AuthoredCallApplication {
                        types: type_arguments,
                        implementations: &[],
                        effects: effect_arguments,
                        requirements: requirement_arguments,
                        scopes: read_scope_arguments,
                    },
                    env,
                )?),
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                    ..
                } => Some(self.call(
                    *function,
                    AuthoredCallApplication {
                        types: type_arguments,
                        implementations,
                        effects: effect_arguments,
                        requirements: requirement_arguments,
                        scopes: read_scope_arguments,
                    },
                    env,
                )?),
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    read_scope_arguments,
                    ..
                } => {
                    let selected = self.operand(witness, env, 0)?;
                    if selected.contract != *contract {
                        return Err(reject());
                    }
                    self.budget
                        .reserve::<ReadScopeArgument>(read_scope_arguments.len())?;
                    let scopes = read_scope_arguments
                        .iter()
                        .map(|scope| self.read_scope(*scope, env))
                        .collect::<Result<Vec<_>, _>>()?;
                    Some(self.method(&selected, *method, &scopes)?)
                }
                _ => None,
            };
            if let Some(target) = target {
                let mut view = self.read_view_operation(&record.operation, target.function)?;
                for identity in &target.witnesses {
                    view |= self
                        .selections
                        .get(*identity)
                        .ok_or_else(reject)?
                        .requires_read_view_generation;
                }
                let associated =
                    self.associated_operation(&record.operation, target.function)? || view;
                if associated {
                    self.call_generation(env, view)?;
                }
                if associated && !associated_targets.contains(&id) {
                    self.budget.node::<ExpressionId>()?;
                    associated_targets.insert(id);
                }
                if view && !read_view_targets.contains(&id) {
                    self.budget.node::<ExpressionId>()?;
                    read_view_targets.insert(id);
                }
                self.budget.node::<(ExpressionId, Application)>()?;
                self.charge_application(&target)?;
                targets.insert(id, target);
            }
            match &record.operation {
                ExpressionOperation::CreateReadView { fields, .. } => {
                    self.budget.reserve::<ExpressionId>(fields.len())?;
                    for field in fields {
                        self.budget.tick()?;
                        pending.push(field.value);
                    }
                }
                ExpressionOperation::BorrowReadView { body, .. }
                | ExpressionOperation::BorrowReadViewField { body, .. } => {
                    self.budget.reserve::<ExpressionId>(1)?;
                    pending.push(*body);
                }
                ExpressionOperation::ReborrowReadView { .. }
                | ExpressionOperation::MoveReadView { .. }
                | ExpressionOperation::DropReadView { .. }
                | ExpressionOperation::ReadViewDataField { .. } => {}
                _ => {
                    self.budget
                        .reserve::<crate::platform::kernel::ExpressionChild>(
                            record.children().len(),
                        )?;
                    for child in record.children() {
                        self.budget.reserve::<ExpressionId>(1)?;
                        pending.push(child.expression);
                    }
                }
            }
        }
        Ok(())
    }

    fn occurrence(
        &mut self,
        occurrences: &mut BTreeMap<OwnerKey, u16>,
        package: PackageId,
        key: OwnerKey,
    ) -> Result<(), ExecutionError> {
        let version = self.owner(package, key)?.header().contract_version;
        if let std::collections::btree_map::Entry::Vacant(entry) = occurrences.entry(key) {
            self.budget.node::<(OwnerKey, u16)>()?;
            entry.insert(version);
        } else if occurrences.get(&key) != Some(&version) {
            return Err(reject());
        }
        Ok(())
    }

    fn associated_function(
        &mut self,
        function: DeclarationReference,
    ) -> Result<bool, ExecutionError> {
        let reference = function;
        let owner = self.declaration(reference, None)?;
        if matches!(&owner.payload, DeclarationPayload::Function(function) if !function.read_scope_parameters.is_empty() || !function.read_scope_sources.is_empty() || matches!(function.result.relation, ResultRelation::ViewFrom { .. }))
        {
            return Ok(true);
        }
        let (parameters, result, implementations) = match &owner.payload {
            DeclarationPayload::Function(function) => (
                function.parameters.as_slice(),
                function.result.ty,
                function.implementation_parameters.as_slice(),
            ),
            DeclarationPayload::External(function) => {
                (function.parameters.as_slice(), function.result, &[][..])
            }
            _ => return Err(reject()),
        };
        if self.associated_type(result, 0)? {
            return Ok(true);
        }
        for parameter in parameters {
            let OwnerRecord::Parameter(parameter) =
                self.owner(reference.package, OwnerKey::Parameter(*parameter))?
            else {
                return Err(reject());
            };
            if self.associated_type(parameter.ty, 0)? {
                return Ok(true);
            }
        }
        for parameter in implementations {
            let contract = self.declaration(parameter.contract, Some(reference.package))?;
            if matches!(&contract.payload, DeclarationPayload::OwnedContract(contract) if !contract.associated_types.is_empty())
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn associated_type(
        &mut self,
        ty: TypeObjectDigest,
        depth: usize,
    ) -> Result<bool, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = self.types.get(&ty).ok_or_else(reject)?;
        if object.form.is_associated() || matches!(object.form, TypeForm::ReadViewApplied { .. }) {
            return Ok(true);
        }
        self.budget
            .reserve::<TypeObjectDigest>(object.child_type_count())?;
        for child in object.child_types() {
            if self.associated_type(child, depth + 1)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // Preserve raw view provenance even when closing a type erases its template.
    // All child types count, including unused/phantom callable arguments.
    fn read_view_type(
        &mut self,
        ty: TypeObjectDigest,
        depth: usize,
    ) -> Result<bool, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let object = self.types.get(&ty).ok_or_else(reject)?;
        let mut view = matches!(
            object.form,
            TypeForm::ReadViewApplied { .. }
                | TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
        );
        self.budget
            .reserve::<TypeObjectDigest>(object.child_type_count())?;
        for child in object.child_types() {
            view |= self.read_view_type(child, depth + 1)?;
        }
        Ok(view)
    }

    // Inspect complete source inventories rather than the requested member or
    // the substituted result. Contracts have no witness formals, so these
    // prerequisite-contract visits do not recurse through callable mappings.
    fn read_view_owner(&mut self, scope: DeclarationReference) -> Result<bool, ExecutionError> {
        self.budget.tick()?;
        let owner = self.declaration(scope, None)?;
        let mut view = owner.payload.has_read_view_metadata();
        match &owner.payload {
            DeclarationPayload::Function(function) => {
                view |= self.read_view_type(function.result.ty, 0)?;
                for id in &function.parameters {
                    self.budget.tick()?;
                    let OwnerRecord::Parameter(parameter) =
                        self.owner(scope.package, OwnerKey::Parameter(*id))?
                    else {
                        return Err(reject());
                    };
                    view |= self.read_view_type(parameter.ty, 0)?;
                }
                for formal in &function.implementation_parameters {
                    self.budget.tick()?;
                    view |= self.read_view_contract(formal.contract)?;
                    view |= self.read_view_type(formal.self_type, 0)?;
                    for ty in &formal.type_arguments {
                        view |= self.read_view_type(*ty, 0)?;
                    }
                }
            }
            DeclarationPayload::External(function) => {
                view |= self.read_view_type(function.result, 0)?;
                for id in &function.parameters {
                    self.budget.tick()?;
                    let OwnerRecord::Parameter(parameter) =
                        self.owner(scope.package, OwnerKey::Parameter(*id))?
                    else {
                        return Err(reject());
                    };
                    view |= self.read_view_type(parameter.ty, 0)?;
                }
            }
            DeclarationPayload::OwnedContract(contract) => {
                for method in &contract.methods {
                    self.budget.tick()?;
                    view |= self.read_view_type(method.result.ty, 0)?;
                    for parameter in &method.parameters {
                        view |= self.read_view_type(parameter.ty, 0)?;
                    }
                }
            }
            DeclarationPayload::OwnedImplementation(scheme) => {
                view |= self.read_view_contract(scheme.contract)?;
                view |= self.read_view_type(scheme.self_type, 0)?;
                for ty in &scheme.type_arguments {
                    view |= self.read_view_type(*ty, 0)?;
                }
                for binding in &scheme.associated_types {
                    self.budget.tick()?;
                    view |= self.read_view_type(binding.ty(), 0)?;
                }
                for formal in &scheme.implementation_parameters {
                    self.budget.tick()?;
                    view |= self.read_view_contract(formal.contract)?;
                    view |= self.read_view_type(formal.self_type, 0)?;
                    for ty in &formal.type_arguments {
                        view |= self.read_view_type(*ty, 0)?;
                    }
                }
                for mapping in &scheme.methods {
                    self.budget.tick()?;
                    for ty in &mapping.type_arguments {
                        view |= self.read_view_type(*ty, 0)?;
                    }
                }
            }
            _ => return Err(reject()),
        }
        Ok(view)
    }

    fn read_view_contract(
        &mut self,
        contract: DeclarationReference,
    ) -> Result<bool, ExecutionError> {
        if !matches!(
            self.declaration(contract, None)?.payload,
            DeclarationPayload::OwnedContract(_)
        ) {
            return Err(reject());
        }
        self.read_view_owner(contract)
    }

    fn symbolic_selection_read_view(
        &mut self,
        selected: &SymbolicSelection,
        depth: usize,
    ) -> Result<bool, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let mut view = self.read_view_contract(selected.contract)?;
        view |= self.read_view_type(selected.self_type, 0)?;
        for ty in &selected.arguments {
            view |= self.read_view_type(*ty, 0)?;
        }
        if let SymbolicSelectionSource::Concrete {
            implementation,
            prerequisites,
            ..
        } = &selected.source
        {
            view |= self.read_view_owner(*implementation)?;
            for child in prerequisites {
                view |= self.symbolic_selection_read_view(child, depth + 1)?;
            }
        }
        Ok(view)
    }

    fn call(
        &mut self,
        function: DeclarationReference,
        application: AuthoredCallApplication<'_>,
        caller: &Environment,
    ) -> Result<Application, ExecutionError> {
        let AuthoredCallApplication {
            types,
            implementations,
            effects,
            requirements,
            scopes: read_scope_arguments,
        } = application;
        self.declaration(
            function,
            caller
                .scope
                .map(|scope| scope.package)
                .or(caller.source_package),
        )?;
        self.budget.reserve::<TypeObjectDigest>(types.len())?;
        let types = types
            .iter()
            .map(|ty| self.ty(*ty, caller, 0))
            .collect::<Result<Vec<_>, _>>()?;
        self.budget
            .reserve::<Arc<Selection>>(implementations.len())?;
        let implementations = implementations
            .iter()
            .map(|operand| self.operand(operand, caller, 0))
            .collect::<Result<Vec<_>, _>>()?;
        self.budget.reserve::<EffectRow>(effects.len())?;
        let effects = effects
            .iter()
            .map(|row| self.row(row, caller))
            .collect::<Result<Vec<_>, _>>()?;
        self.budget
            .reserve::<RequirementReference>(requirements.len())?;
        let requirements = requirements
            .iter()
            .map(|operand| {
                self.budget.tick()?;
                reference_effects::resolve(*operand, &caller.requirements)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.budget
            .reserve::<ReadScopeArgument>(read_scope_arguments.len())?;
        let read_scope_arguments = read_scope_arguments
            .iter()
            .map(|scope| self.read_scope(*scope, caller))
            .collect::<Result<Vec<_>, _>>()?;
        self.enqueue(
            function,
            types,
            implementations,
            effects,
            requirements,
            read_scope_arguments,
        )
    }

    fn method(
        &mut self,
        selected: &Arc<Selection>,
        method: MethodId,
        read_scope_arguments: &[ReadScopeArgument],
    ) -> Result<Application, ExecutionError> {
        self.budget.tick()?;
        if let Some(target) =
            self.methods
                .get(&(selected.identity, method, read_scope_arguments.to_vec()))
        {
            self.budget
                .reserve::<TypeObjectDigest>(target.types.len())?;
            self.budget.reserve::<usize>(target.witnesses.len())?;
            self.budget.reserve::<EffectRow>(target.effects.len())?;
            self.budget
                .reserve::<RequirementReference>(target.requirements.len())?;
            return Ok(target.clone());
        }
        let owner = self.declaration(selected.implementation, None)?;
        let DeclarationPayload::OwnedImplementation(scheme) = &owner.payload else {
            return Err(reject());
        };
        let mapping = scheme
            .methods
            .iter()
            .find(|mapping| mapping.method == method)
            .ok_or_else(reject)?;
        if scheme
            .methods
            .iter()
            .filter(|mapping| mapping.method == method)
            .count()
            != 1
        {
            return Err(reject());
        }
        let mut env = Environment {
            scope: Some(selected.implementation),
            generation: owner.header.contract_version,
            lexical: true,
            ordinary: self.ordinary(
                selected.implementation,
                &scheme.type_parameters,
                &selected.actuals,
            )?,
            ..Environment::default()
        };
        self.witness_bindings(
            selected.implementation,
            &scheme.implementation_parameters,
            &selected.prerequisites,
            &mut env,
        )?;
        let contract_owner = self.declaration(selected.contract, None)?;
        let DeclarationPayload::OwnedContract(contract) = &contract_owner.payload else {
            return Err(reject());
        };
        let method_contract = contract
            .methods
            .iter()
            .find(|candidate| candidate.id == method)
            .ok_or_else(reject)?;
        self.scope_bindings(
            ReadScopeBinder::ContractMethod {
                contract: selected.contract,
                method,
            },
            &method_contract.read_scope_parameters,
            read_scope_arguments,
            &mut env,
        )?;
        let target = self.call(
            mapping.function,
            AuthoredCallApplication {
                types: &mapping.type_arguments,
                implementations: &mapping.implementations,
                effects: &[],
                requirements: &[],
                scopes: &mapping.read_scope_arguments,
            },
            &env,
        )?;
        self.compare_method(selected, method, &target, read_scope_arguments)?;
        self.budget
            .node::<((usize, MethodId, Vec<ReadScopeArgument>), Application)>()?;
        self.charge_application(&target)?;
        self.methods.insert(
            (selected.identity, method, read_scope_arguments.to_vec()),
            target.clone(),
        );
        Ok(target)
    }

    fn compare_method(
        &mut self,
        selected: &Arc<Selection>,
        method: MethodId,
        target: &Application,
        read_scope_arguments: &[ReadScopeArgument],
    ) -> Result<(), ExecutionError> {
        let owner = self.declaration(selected.contract, None)?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        let method = contract
            .methods
            .iter()
            .find(|candidate| candidate.id == method)
            .ok_or_else(reject)?;
        let mut env = Environment {
            scope: Some(selected.contract),
            generation: owner.header.contract_version,
            contract: Some(Arc::clone(selected)),
            ..Environment::default()
        };
        self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
        env.ordinary
            .insert(contract.self_parameter, selected.self_type);
        if contract.type_parameters.len() != selected.contract_actuals.len() {
            return Err(reject());
        }
        for (formal, actual) in contract
            .type_parameters
            .iter()
            .zip(&selected.contract_actuals)
        {
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            if env.ordinary.insert(*formal, *actual).is_some() {
                return Err(reject());
            }
        }
        self.scope_bindings(
            ReadScopeBinder::ContractMethod {
                contract: selected.contract,
                method: method.id,
            },
            &method.read_scope_parameters,
            read_scope_arguments,
            &mut env,
        )?;
        let expected_ty = self.ty(method.result.ty, &env, 0)?;
        let expected = self.closed_result(method.result, expected_ty, &env, Ok)?;
        let expected_sources = self.closed_sources(&method.read_scope_sources, &env, Ok)?;
        let target_owner =
            self.declaration(target.function, Some(selected.implementation.package))?;
        let DeclarationPayload::Function(function) = &target_owner.payload else {
            return Err(reject());
        };
        if method.parameters.len() != function.parameters.len() {
            return Err(reject());
        }
        let mut target_env = self.environment(target)?;
        let actual_ty = self.ty(function.result.ty, &target_env, 0)?;
        let ordinal = |source| {
            function
                .parameters
                .iter()
                .position(|parameter| *parameter == source)
                .and_then(|position| u32::try_from(position).ok())
                .ok_or_else(reject)
        };
        let actual = self.closed_result(function.result, actual_ty, &target_env, ordinal)?;
        let actual_sources =
            self.closed_sources(&function.read_scope_sources, &target_env, ordinal)?;
        if expected != actual
            || expected_sources != actual_sources
            || matches!(
                method.effect,
                crate::platform::kernel::FunctionEffect::Task { .. }
            ) != matches!(
                function.effect,
                crate::platform::kernel::FunctionEffect::Task { .. }
            )
            || self.row(&method.effect.row(), &env)?
                != self.row(&function.effect.row(), &target_env)?
        {
            return Err(reject());
        }
        for (expected, actual) in method.parameters.iter().zip(&function.parameters) {
            let OwnerRecord::Parameter(actual) =
                self.owner(target.function.package, OwnerKey::Parameter(*actual))?
            else {
                return Err(reject());
            };
            if actual.parent != ParameterParent::Function(target.function.declaration)
                || actual.use_mode != expected.use_mode
                || actual.resource_requirement.is_some()
            {
                return Err(reject());
            }
            target_env.generation = actual.header.contract_version;
            if self.ty(expected.ty, &env, 0)? != self.ty(actual.ty, &target_env, 0)? {
                return Err(reject());
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Proof, ExecutionError> {
        let mut next_selection = 0;
        loop {
            while next_selection < self.selections.len() {
                self.budget.tick()?;
                let selected = Arc::clone(&self.selections[next_selection]);
                next_selection += 1;
                let owner = self.declaration(selected.contract, None)?;
                let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
                    return Err(reject());
                };
                let owner = self.declaration(selected.implementation, None)?;
                let DeclarationPayload::OwnedImplementation(scheme) = &owner.payload else {
                    return Err(reject());
                };
                if contract.methods.len() != scheme.methods.len() {
                    return Err(reject());
                }
                let mut ids = BTreeSet::new();
                let mut names = BTreeSet::new();
                for method in &contract.methods {
                    self.budget.node::<MethodId>()?;
                    self.budget.node::<&crate::platform::kernel::Name>()?;
                    if !ids.insert(method.id) || !names.insert(&method.name) {
                        return Err(reject());
                    }
                    self.budget
                        .reserve::<ReadScopeArgument>(method.read_scope_parameters.len())?;
                    let scopes = method
                        .read_scope_parameters
                        .iter()
                        .map(|formal| {
                            ReadScopeArgument::Parameter(ReadScopeReference {
                                binder: ReadScopeBinder::ContractMethod {
                                    contract: selected.contract,
                                    method: method.id,
                                },
                                parameter: formal.id,
                            })
                        })
                        .collect::<Vec<_>>();
                    self.method(&selected, method.id, &scopes)?;
                }
            }
            let Some(application) = self.pending.pop_front() else {
                break;
            };
            self.budget.tick()?;
            let function = self.function(&application)?;
            self.budget.node::<(Application, Function)>()?;
            self.applications.insert(application, function);
        }
        self.budget.reserve::<TypeObjectDigest>(self.types.len())?;
        let inventory = self.types.keys().copied().collect::<Vec<_>>();
        let mut raw_templates = BTreeSet::new();
        let mut raw_read_view_templates = BTreeSet::new();
        for ty in inventory {
            if self.associated_type(ty, 0)? {
                self.budget.node::<TypeObjectDigest>()?;
                raw_templates.insert(ty);
            }
            if self.read_view_type(ty, 0)? {
                self.budget.node::<TypeObjectDigest>()?;
                raw_read_view_templates.insert(ty);
            }
        }
        let mut selection_index: SelectionIndex = BTreeMap::new();
        for ((implementation, actuals, prerequisites), selected) in self.interned {
            self.budget.tick()?;
            if !selection_index.contains_key(&implementation) {
                self.budget.node::<(
                    DeclarationReference,
                    BTreeMap<Vec<TypeObjectDigest>, BTreeMap<Vec<usize>, Arc<Selection>>>,
                )>()?;
            }
            if !selection_index
                .get(&implementation)
                .is_some_and(|actuals_index| actuals_index.contains_key(&actuals))
            {
                self.budget
                    .node::<(Vec<TypeObjectDigest>, BTreeMap<Vec<usize>, Arc<Selection>>)>()?;
            }
            self.budget.node::<(Vec<usize>, Arc<Selection>)>()?;
            selection_index
                .entry(implementation)
                .or_default()
                .entry(actuals)
                .or_default()
                .insert(prerequisites, selected);
        }
        let mut type_identities = BTreeSet::new();
        for ty in self.types.keys() {
            self.budget.tick()?;
            self.budget.node::<TypeObjectDigest>()?;
            type_identities.insert(*ty);
        }
        let mut result = Proof {
            types: self.types,
            selections: self.selections,
            applications: self.applications,
            methods: self.methods,
            contract_templates: self.contract_templates,
            implementation_inventory: self.implementation_inventory,
            mapping_templates: self.mapping_templates,
            symbolic: self.symbolic,
            symbolic_mappings: self.symbolic_mappings,
            contract_methods: self.contract_methods,
            executables: self.executables,
            selection_index,
            type_identities,
            raw_templates,
            raw_read_view_templates,
            symbolic_formals: self.symbolic_formals,
            #[cfg(test)]
            flow_work: self.flow_work,
            steps: self.budget.steps,
            bytes: self.budget.bytes,
        };
        result.closed(self.budget.limits, self.budget.control)?;
        Ok(result)
    }
}

impl Proof {
    fn contains_type(&self, ty: &TypeObjectDigest) -> bool {
        self.type_identities.contains(ty)
    }

    pub(super) fn take_types(&mut self) -> BTreeMap<TypeObjectDigest, TypeObject> {
        std::mem::take(&mut self.types)
    }

    fn closed(&mut self, limits: Limits, control: &ExecutionControl) -> Result<(), ExecutionError> {
        let mut budget = Budget {
            steps: self.steps,
            bytes: self.bytes,
            limits,
            control,
        };
        budget.tick()?;
        if self.steps == 0 || self.bytes == 0 {
            return Err(reject());
        }
        for selection in &self.selections {
            budget.tick()?;
            if selection.identity >= self.selections.len()
                || selection.depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
                || selection.parameters.len() != selection.actuals.len()
                || selection.constraints.len() != selection.actuals.len()
                || selection.prerequisite_parameters.len() != selection.prerequisites.len()
                || selection.prerequisite_templates.len() != selection.prerequisites.len()
                || selection.contract_argument_templates.len() != selection.contract_actuals.len()
                || !self.contains_type(&selection.self_template)
                || ((!selection.members.is_empty() || selection.requires_read_view_generation)
                    && (!super::associated_generation_admitted(
                        selection.implementation_generation,
                        selection.requires_read_view_generation,
                    ) || !super::associated_generation_admitted(
                        selection.contract_generation,
                        selection.requires_read_view_generation,
                    )))
                || selection.contract_constraints.len() != selection.contract_actuals.len()
                || selection
                    .actuals
                    .iter()
                    .chain(&selection.contract_actuals)
                    .chain([&selection.self_type])
                    .any(|ty| !self.contains_type(ty))
                || selection.members.iter().any(|member| {
                    !self.contains_type(&member.template) || !self.contains_type(&member.ty)
                })
                || selection
                    .prerequisites
                    .iter()
                    .any(|child| child.identity >= selection.identity)
            {
                return Err(reject());
            }
        }
        for (application, function) in &self.applications {
            budget.tick()?;
            if application
                .witnesses
                .iter()
                .any(|id| *id >= self.selections.len())
                || !self.contains_type(&function.result.ty)
                || function
                    .parameters
                    .iter()
                    .any(|(ty, _)| !self.contains_type(ty))
                || function
                    .result
                    .relation
                    .source()
                    .copied()
                    .is_some_and(|source| source as usize >= function.parameters.len())
                || !function.effect.is_closed()
                || function.roots.values().any(|ty| !self.contains_type(ty))
                || function
                    .targets
                    .values()
                    .any(|target| !self.applications.contains_key(target))
            {
                return Err(reject());
            }
            Self::scope_source_inventory(
                &function.parameters,
                &function.read_scope_parameters,
                &function.read_scope_sources,
                &mut budget,
            )?;
            self.associated_occurrences(function, &mut budget)?;
            self.invocation_inventory(function, &mut budget)?;
        }
        for ((selection, _, _), target) in &self.methods {
            budget.tick()?;
            if *selection >= self.selections.len() || !self.applications.contains_key(target) {
                return Err(reject());
            }
        }
        for function in self
            .symbolic
            .values()
            .chain(self.contract_methods.values())
            .chain(self.executables.values())
        {
            budget.tick()?;
            self.associated_occurrences(function, &mut budget)?;
            self.invocation_inventory(function, &mut budget)?;
            if !self.contains_type(&function.result.ty)
                || function
                    .result
                    .relation
                    .source()
                    .copied()
                    .is_some_and(|position| position as usize >= function.parameters.len())
            {
                return Err(reject());
            }
            for (ty, _) in &function.parameters {
                budget.tick()?;
                if !self.contains_type(ty) {
                    return Err(reject());
                }
            }
            for ((owner, raw), closed) in &function.roots {
                budget.tick()?;
                if !function.occurrences.contains_key(owner)
                    || !self.contains_type(raw)
                    || !self.contains_type(closed)
                {
                    return Err(reject());
                }
            }
            for (expression, target) in &function.symbolic_targets {
                budget.tick()?;
                if !function
                    .occurrences
                    .contains_key(&OwnerKey::Expression(*expression))
                    || target.function.is_some() == target.method.is_some()
                {
                    return Err(reject());
                }
                if let Some(function) = target.function
                    && !self.symbolic.contains_key(&function)
                {
                    return Err(reject());
                }
                if let Some((contract, method)) = target.method
                    && !self.contract_methods.contains_key(&(contract, method))
                {
                    return Err(reject());
                }
                self.scoped_signature(&target.signature, &mut budget)?;
                for ty in &target.types {
                    budget.tick()?;
                    if !self.contains_type(ty) {
                        return Err(reject());
                    }
                }
                for selected in &target.witnesses {
                    self.symbolic_selection(selected, &mut budget, 0)?;
                }
                for row in &target.effects {
                    budget.tick()?;
                    budget.validate_row(row)?;
                }
                for requirement in &target.requirements {
                    budget.tick()?;
                    match requirement {
                        RequirementOperand::Concrete(_) | RequirementOperand::Parameter(_) => {}
                    }
                }
            }
            for target in function.targets.values() {
                budget.tick()?;
                if !self.applications.contains_key(target) {
                    return Err(reject());
                }
            }
        }
        for ((implementation, method), target) in &self.symbolic_mappings {
            budget.tick()?;
            if !self.implementation_inventory.contains(implementation)
                || !self
                    .mapping_templates
                    .contains_key(&(*implementation, *method))
                || !target
                    .function
                    .is_some_and(|function| self.symbolic.contains_key(&function))
                || target.method.is_some()
            {
                return Err(reject());
            }
            self.scoped_signature(&target.signature, &mut budget)?;
            for ty in &target.types {
                budget.tick()?;
                if !self.contains_type(ty) {
                    return Err(reject());
                }
            }
            for selected in &target.witnesses {
                self.symbolic_selection(selected, &mut budget, 0)?;
            }
            for row in &target.effects {
                budget.tick()?;
                budget.validate_row(row)?;
            }
            for _ in &target.requirements {
                budget.tick()?;
            }
        }
        self.steps = budget.steps;
        Ok(())
    }

    fn scoped_signature(
        &self,
        signature: &ScopedSignature,
        budget: &mut Budget<'_>,
    ) -> Result<(), ExecutionError> {
        budget.tick()?;
        if !self.contains_type(&signature.result.ty) {
            return Err(reject());
        }
        for (ty, _) in &signature.parameters {
            budget.tick()?;
            if !self.contains_type(ty) {
                return Err(reject());
            }
        }
        budget.validate_row(&signature.effect)?;
        Self::scope_source_inventory(
            &signature.parameters,
            &signature.read_scope_parameters,
            &signature.read_scope_sources,
            budget,
        )?;
        if let Some(position) = signature.result.relation.source().copied() {
            let parameter = signature
                .parameters
                .get(position as usize)
                .ok_or_else(reject)?;
            if signature.is_task || parameter.1 != ParameterUse::Borrow {
                return Err(reject());
            }
        }
        Ok(())
    }

    fn scope_source_inventory(
        parameters: &[(TypeObjectDigest, ParameterUse)],
        scope_parameters: &[ReadScopeParameter],
        sources: &[ReadScopeSource<u32>],
        budget: &mut Budget<'_>,
    ) -> Result<(), ExecutionError> {
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for formal in scope_parameters {
            budget.tick()?;
            budget.node::<crate::platform::semantic_id::ReadScopeParameterId>()?;
            budget.node::<&crate::platform::kernel::Name>()?;
            if !ids.insert(formal.id) || !names.insert(&formal.name) {
                return Err(reject());
            }
        }
        let mut seen = BTreeSet::new();
        for source in sources {
            budget.tick()?;
            budget.node::<(u32, ReadScopeArgument)>()?;
            if parameters
                .get(source.source as usize)
                .is_none_or(|parameter| parameter.1 != ParameterUse::Borrow)
                || !seen.insert((source.source, source.scope))
            {
                return Err(reject());
            }
        }
        Ok(())
    }

    fn invocation_inventory(
        &self,
        function: &Function,
        budget: &mut Budget<'_>,
    ) -> Result<(), ExecutionError> {
        for (expression, invocation) in &function.invocations {
            budget.tick()?;
            if !function
                .occurrences
                .contains_key(&OwnerKey::Expression(*expression))
                || !function
                    .occurrences
                    .contains_key(&OwnerKey::Expression(invocation.callee))
                || function.targets.contains_key(expression)
                || function.symbolic_targets.contains_key(expression)
            {
                return Err(reject());
            }
            for argument in &invocation.arguments {
                budget.tick()?;
                if !function
                    .occurrences
                    .contains_key(&OwnerKey::Expression(*argument))
                {
                    return Err(reject());
                }
            }
        }
        Ok(())
    }

    fn associated_occurrences(
        &self,
        function: &Function,
        budget: &mut Budget<'_>,
    ) -> Result<(), ExecutionError> {
        for id in &function.associated_targets {
            budget.tick()?;
            if !function
                .occurrences
                .get(&OwnerKey::Expression(*id))
                .is_some_and(|generation| {
                    super::associated_generation_admitted(
                        *generation,
                        function.read_view_targets.contains(id),
                    )
                })
                || (!function.targets.contains_key(id)
                    && !function.symbolic_targets.contains_key(id))
            {
                return Err(reject());
            }
        }
        for id in &function.read_view_targets {
            budget.tick()?;
            if !function.associated_targets.contains(id) {
                return Err(reject());
            }
        }
        Ok(())
    }

    fn symbolic_selection(
        &self,
        selection: &SymbolicSelection,
        budget: &mut Budget<'_>,
        depth: usize,
    ) -> Result<(), ExecutionError> {
        budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
            || !self.contains_type(&selection.self_type)
        {
            return Err(reject());
        }
        for actual in &selection.arguments {
            budget.tick()?;
            if !self.contains_type(actual) {
                return Err(reject());
            }
        }
        match &selection.source {
            SymbolicSelectionSource::Parameter(scope, parameter) => {
                let formal = self
                    .symbolic_formals
                    .get(&(*scope, *parameter))
                    .ok_or_else(reject)?;
                for _ in &formal.type_arguments {
                    budget.tick()?;
                }
                if formal.contract != selection.contract
                    || formal.self_type != selection.self_type
                    || formal.type_arguments != selection.arguments
                {
                    return Err(reject());
                }
            }
            SymbolicSelectionSource::Concrete {
                implementation,
                actuals,
                prerequisites,
                members,
            } => {
                if !self.implementation_inventory.contains(implementation)
                    || !self.contract_templates.contains_key(&selection.contract)
                {
                    return Err(reject());
                }
                for actual in actuals {
                    budget.tick()?;
                    if !self.contains_type(actual) {
                        return Err(reject());
                    }
                }
                for member in members {
                    budget.tick()?;
                    if !self.contains_type(&member.template) || !self.contains_type(&member.ty) {
                        return Err(reject());
                    }
                }
                for child in prerequisites {
                    self.symbolic_selection(child, budget, depth + 1)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl Proof {
    /// Compare a producer selection with independently derived canonical meaning.
    /// Catalogue membership is checked before memo reuse, so a second node cannot
    /// borrow an equal numeric identity to evade complete child comparison.
    pub(in super::super) fn compare_selection(
        &self,
        expected: &Selection,
        actual: &super::super::prepare::NormalizedImplementationArgument,
        catalogue: &[super::super::prepare::NormalizedImplementationArgument],
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        let mut pending = vec![(expected, actual)];
        let mut seen = BTreeSet::new();
        while let Some((expected, actual)) = pending.pop() {
            control.check()?;
            if !catalogue
                .get(actual.identity as usize)
                .is_some_and(|entry| Arc::ptr_eq(entry, actual))
            {
                return Err(reject());
            }
            if !seen.insert((expected.identity, actual.identity)) {
                continue;
            }
            if expected.implementation != actual.implementation
                || expected.contract != actual.contract
                || expected.depth != actual.depth
                || expected.implementation_generation != actual.implementation_generation
                || expected.contract_generation != actual.contract_generation
                || expected.parameters.as_slice() != actual.implementation_type_parameters.as_ref()
                || expected.prerequisite_parameters.as_slice()
                    != actual.implementation_parameter_ids.as_ref()
                || expected.actuals.as_slice() != actual.implementation_type_arguments.as_ref()
                || expected.constraints.as_slice()
                    != actual.implementation_type_parameter_constraints.as_ref()
                || expected.self_type != actual.self_type
                || expected.contract_actuals.as_slice() != actual.type_arguments.as_ref()
                || expected.contract_constraints.as_slice()
                    != actual.contract_type_parameter_constraints.as_ref()
                || expected.prerequisites.len() != actual.implementations.len()
                || expected.prerequisites.len() != actual.prerequisites.len()
                || expected.members.len() != actual.associated_types.len()
            {
                return Err(reject());
            }
            let actual_implementation = actual.implementation;
            for (expected, actual) in expected.members.iter().zip(actual.associated_types.iter()) {
                control.check()?;
                if expected.member != actual.member
                    || expected.template != actual.template
                    || expected.ty != actual.ty
                    || expected.scope_parameter
                        != actual
                            .scope_parameter
                            .as_ref()
                            .map(|parameter| ReadScopeReference {
                                binder: ReadScopeBinder::AssociatedBinding {
                                    implementation: actual_implementation,
                                    member: actual.member,
                                },
                                parameter: parameter.id,
                            })
                    || self.types.get(&expected.ty) != types.get(&actual.ty)
                {
                    return Err(reject());
                }
            }
            for ((expected, actual), obligation) in expected
                .prerequisites
                .iter()
                .zip(actual.implementations.iter())
                .zip(actual.prerequisites.iter())
            {
                control.check()?;
                if expected.contract != obligation.contract
                    || expected.self_type != obligation.self_type
                    || expected.contract_actuals.as_slice() != obligation.type_arguments.as_ref()
                {
                    return Err(reject());
                }
                pending.push((expected, actual));
            }
        }
        Ok(())
    }
}

#[path = "reference_associated_context.rs"]
mod context;

#[path = "reference_associated_inventory.rs"]
mod inventory;

#[path = "reference_associated_signatures.rs"]
mod signatures;

pub(in super::super) fn derive<'a>(
    snapshots: &[&'a KernelSnapshot],
    entries: &[Entry<'_>],
    limits: Limits,
    control: &'a ExecutionControl,
) -> Result<Proof, ExecutionError> {
    let mut derivation = Derivation::new(snapshots, limits, control)?;
    let derivation_entry = derivation.budget.steps;
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let offset = maximum.saturating_sub(limits.steps);
    let mut supplier_work = offset
        .checked_add(derivation.budget.steps)
        .ok_or_else(capacity)?;
    super::super::reference_schema::bind_associated_dependencies(
        snapshots,
        control,
        &mut supplier_work,
    )?;
    derivation.budget.steps = supplier_work.checked_sub(offset).ok_or_else(capacity)?;
    if derivation.budget.steps > limits.steps {
        return Err(capacity());
    }
    let after_dependencies = derivation.budget.steps;
    for snapshot in snapshots {
        for (ty, object) in snapshot.types.iter().chain(&snapshot.dependency_types) {
            derivation.budget.tick()?;
            if object.form.is_associated() {
                derivation.associated_bound(*ty)?;
            }
        }
    }
    for snapshot in snapshots {
        for (key, owner) in &snapshot.owners {
            derivation.budget.tick()?;
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(declaration)) = (key, owner)
            else {
                if matches!(owner, OwnerRecord::Port(_)) {
                    derivation.executable_inventory(snapshot.root.package_id, *key, owner)?;
                }
                continue;
            };
            let reference = DeclarationReference {
                package: snapshot.root.package_id,
                declaration: *id,
            };
            match &declaration.payload {
                DeclarationPayload::OwnedImplementation(scheme) => {
                    derivation.budget.node::<DeclarationReference>()?;
                    derivation.implementation_inventory.insert(reference);
                    derivation.inventory(reference, scheme)?;
                    derivation.mapping_inventory(reference, scheme)?;
                }
                DeclarationPayload::Function(function) => {
                    derivation.symbolic_inventory(reference)?;
                    if function.type_parameters.is_empty()
                        && function.implementation_parameters.is_empty()
                        && function.effect_parameters.is_empty()
                        && function.requirement_parameters.is_empty()
                        && function.read_scope_parameters.is_empty()
                    {
                        derivation.enqueue(
                            reference,
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                        )?;
                    }
                }
                DeclarationPayload::External(_) => derivation.symbolic_inventory(reference)?,
                DeclarationPayload::OwnedContract(_) => derivation.contract_inventory(reference)?,
                _ => derivation.executable_inventory(snapshot.root.package_id, *key, owner)?,
            }
        }
    }
    for entry in entries {
        derivation.budget.tick()?;
        let caller = Environment::default();
        derivation
            .budget
            .reserve::<TypeObjectDigest>(entry.types.len())?;
        let types = entry
            .types
            .iter()
            .map(|ty| derivation.ty(*ty, &caller, 0))
            .collect::<Result<Vec<_>, _>>()?;
        derivation
            .budget
            .reserve::<Arc<Selection>>(entry.implementations.len())?;
        let witnesses = entry
            .implementations
            .iter()
            .map(|operand| derivation.operand(operand, &caller, 0))
            .collect::<Result<Vec<_>, _>>()?;
        derivation
            .budget
            .reserve::<EffectRow>(entry.effects.len())?;
        for row in entry.effects {
            derivation.budget.row(row)?;
        }
        derivation
            .budget
            .reserve::<RequirementReference>(entry.requirements.len())?;
        derivation
            .budget
            .reserve::<ReadScopeArgument>(entry.read_scope_arguments.len())?;
        let scope_caller = Environment {
            symbolic: true,
            ..Environment::default()
        };
        let read_scope_arguments = entry
            .read_scope_arguments
            .iter()
            .map(|argument| derivation.read_scope(*argument, &scope_caller))
            .collect::<Result<Vec<_>, _>>()?;
        derivation.enqueue(
            entry.function,
            types,
            witnesses,
            entry.effects.to_vec(),
            entry.requirements.to_vec(),
            read_scope_arguments,
        )?;
    }
    // Raw inventories and symbolic obligations precede the independent growth
    // verdict. No concrete application/mapping body is enumerated before it.
    let mut work = derivation.budget.steps;
    let flow_entry = work;
    let flow = crate::platform::kernel::associated_flow_canonical_reference::validate(
        snapshots,
        control,
        &mut work,
        limits.steps,
    )
    .map_err(|error| {
        let class = match error.class {
            crate::platform::diagnostic::DiagnosticClass::Cancelled => {
                ExecutionFailureClass::Cancelled
            }
            crate::platform::diagnostic::DiagnosticClass::Resource => {
                ExecutionFailureClass::Resource
            }
            crate::platform::diagnostic::DiagnosticClass::Semantic => ExecutionFailureClass::Trap,
            _ => ExecutionFailureClass::Infrastructure,
        };
        let mut message = error.message;
        if error.code == "kernel_associated_reference_capacity" && !error.notes.is_empty() {
            for note in error.notes {
                message.push_str("; ");
                message.push_str(&note);
            }
            message.push_str(&format!(
                "; reference_derivation entry={} after_dependencies={} flow_entry={} work_debit={} maximum={}",
                derivation_entry, after_dependencies, flow_entry, offset, limits.steps,
            ));
        }
        ExecutionError::new(class, error.code, message)
    })?;
    derivation.budget.steps = work;
    derivation.budget.reserve::<u8>(flow.metadata_bytes)?;
    #[cfg(test)]
    {
        derivation.flow_work = flow;
    }
    derivation.finish()
}

/// Called by complete canonical reference schema construction. Complete stored
/// inventories are never skipped by
/// an empty executable entry set or by a member not being requested.
pub(super) fn check_inventory(
    snapshots: &[&KernelSnapshot],
    limits: Limits,
    control: &ExecutionControl,
) -> Result<Proof, ExecutionError> {
    derive(snapshots, &[], limits, control)
}

#[cfg(test)]
#[path = "reference_associated_preparation_tests.rs"]
mod tests;

#[cfg(test)]
pub(in super::super) mod test_support {
    pub(in super::super::super) use super::tests::{
        Fixture, InvokeFixture, base_source, invoke_fixture, make_fixture, operand, prove,
    };
}
