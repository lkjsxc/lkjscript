//! Implementation-disjoint evaluator over canonical Graph 14 owner and expression records.

use super::capability::{
    NormalizedCapabilities, NormalizedCapabilityTransaction, NormalizedTransactionCompletion,
    validate_outcome,
};
use super::prepare::NormalizedProgram;
use super::reference_schema::NormalizedReferenceSchema;
use super::resource::NormalizedResourceScope;
use super::value::{
    FunctionIndex, NormalizedMapKey, NormalizedRecord, NormalizedValue, RecordLayoutIndex,
    VariantLayoutIndex,
};
use super::value_schema::NormalizedValueSchema;
use super::vm::NormalizedRunPolicy;
use crate::platform::binary64::Binary64;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::JsonLimits;
use crate::platform::kernel::{
    BindingKind, BlobObjectDigest, CaseReference, DeclarationPayload, DeclarationReference,
    EffectParameterReference, EffectRow, ExpressionOperation, FieldReference, FieldSelector,
    FunctionDeclaration, FunctionEffect, ImplementationName, KernelSnapshot, LocalValueReference,
    Name, OperationReference, OwnerKey, OwnerRecord, PackageId, ParameterRecord, ParameterUse,
    PortImplementation, RequirementReference, SemanticStateDigest, TextValue,
    TransactionOutcomeContract, TypeForm, TypeObjectDigest,
};
use crate::platform::kernel::{RequirementOperand, RequirementParameterReference};
use crate::platform::semantic_id::{
    BindingId, ExpressionId, ParameterId, RepositoryId, RevisionId, TypeParameterId,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[path = "reference_checked.rs"]
mod checked;
use checked::{Ownership, Value as CheckedValue};
#[cfg(test)]
#[path = "reference_borrow_result_tests.rs"]
mod borrowed_result_tests;
#[path = "reference_borrowed_results.rs"]
mod borrowed_results;
#[path = "reference_intrinsics.rs"]
mod checked_intrinsics;
#[cfg(test)]
#[path = "reference_implementation_prerequisite_tests.rs"]
mod prerequisite_tests;
#[path = "reference_parallel.rs"]
mod structured;

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub struct NormalizedReferenceObservation {
    pub expressions: u64,
    pub calls: u64,
    pub external_calls: u64,
    pub capability_calls: u64,
    pub allocated_bytes: u64,
    pub allocation_charges: u64,
    pub type_derivation_steps: u64,
    pub source_admission_steps: u64,
    pub type_metadata_bytes: u64,
    pub(crate) value_work: super::value::ValueWork,
    pub collection_items: u64,
    pub maximum_call_depth: usize,
    pub canonical_owner_reads: u64,
    pub canonical_map_pages_read: u64,
    pub canonical_objects_read: u64,
    pub canonical_bytes_read: u64,
    pub production_tier: &'static str,
    pub tail_transfers: u64,
    pub maximum_control_frames: usize,
    pub maximum_live_locals: usize,
    pub maximum_live_type_bindings: usize,
    pub maximum_live_effect_bindings: usize,
    pub maximum_live_allowances: usize,
    pub live_call_frames_after: usize,
    pub live_control_frames_after: usize,
    pub live_local_scopes_after: usize,
    pub live_type_scopes_after: usize,
    pub live_effect_scopes_after: usize,
    pub live_allowances_after: usize,
    pub live_transactions_after: usize,
    pub live_handles_after: usize,
}

pub type NormalizedReferenceInvocation = (NormalizedValue, NormalizedReferenceObservation);
pub type NormalizedReferenceTestInvocation =
    (NormalizedReferenceInvocation, NormalizedReferenceInvocation);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizedReferenceBinding {
    pub repository: RepositoryId,
    pub package: PackageId,
    pub revision: Option<RevisionId>,
    pub semantic_state: Option<SemanticStateDigest>,
}

impl NormalizedReferenceBinding {
    pub fn matches(self, program: &NormalizedProgram) -> bool {
        self.repository == program.root_repository
            && self.package == program.root_package
            && self
                .semantic_state
                .is_none_or(|state| state == program.root_semantic_state)
            && self
                .revision
                .is_none_or(|revision| revision == program.root_revision)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NormalizedReferenceReadWork {
    pub owner_reads: u64,
    pub map_pages_read: u64,
    pub objects_read: u64,
    pub bytes_read: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedReferenceOwnerRead {
    pub record: Option<OwnerRecord>,
    pub work: NormalizedReferenceReadWork,
}

/// Exact canonical owner reads used by the implementation-disjoint reference tier.
///
/// An accepted repository implements this with revision-pinned persistent-map point reads. The
/// in-memory snapshot implementation remains the independent full-oracle fixture path.
pub trait NormalizedReferenceRead {
    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError>;

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError>;

    fn schema(&self) -> Result<Arc<NormalizedReferenceSchema>, ExecutionError>;

    fn blob(&self, _digest: BlobObjectDigest) -> Result<Vec<u8>, ExecutionError> {
        Err(reference_error(
            "normalized_reference_blob_missing",
            "canonical blob source is unavailable to this reference reader",
        ))
    }

    fn owner_in_package(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        if package == self.binding()?.package {
            return self.owner(owner);
        }
        Err(reference_error(
            "normalized_reference_dependency_authority",
            "canonical dependency source is unavailable to the reference reader",
        ))
    }
}

impl NormalizedReferenceRead for KernelSnapshot {
    fn schema(&self) -> Result<Arc<NormalizedReferenceSchema>, ExecutionError> {
        NormalizedReferenceSchema::reconstruct([self]).map(Arc::new)
    }

    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError> {
        Ok(NormalizedReferenceBinding {
            repository: self.root.repository_id,
            package: self.root.package_id,
            revision: None,
            semantic_state: None,
        })
    }

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        Ok(NormalizedReferenceOwnerRead {
            record: self.owners.get(&owner).cloned(),
            work: NormalizedReferenceReadWork {
                owner_reads: 1,
                ..NormalizedReferenceReadWork::default()
            },
        })
    }
}

pub trait NormalizedReferenceHost: Send + Sync {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        function: &ReferenceSignature,
        implementation: &ImplementationName,
        type_arguments: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoreNormalizedReferenceHost;

impl NormalizedReferenceHost for CoreNormalizedReferenceHost {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        function: &ReferenceSignature,
        implementation: &ImplementationName,
        type_arguments: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let arguments = super::value::RawArguments::new(arguments);
        control.check()?;
        let arguments = arguments.into_vec();
        if implementation.as_str() == "core.list.append" {
            let mut raw = arguments.into_iter();
            let list = raw
                .next()
                .ok_or_else(|| reference_type_error("raw append is missing its list"))?;
            let item = raw
                .next()
                .ok_or_else(|| reference_type_error("raw append is missing its item"))?;
            let NormalizedValue::List(list) = list else {
                return Err(reference_type_error("raw append received a foreign list"));
            };
            if raw.next().is_some() {
                return Err(reference_type_error("raw append has foreign arity"));
            }
            return Ok(NormalizedValue::List(list.raw_append(item, control)?));
        }
        reference_intrinsic(
            schema,
            function,
            implementation.as_str(),
            type_arguments,
            arguments,
            control,
        )
    }
}

pub(super) struct BoundReferenceSchema {
    pub(super) canonical: Arc<NormalizedReferenceSchema>,
    pub(super) value_origin: super::value::ValueOrigin,
}

impl std::ops::Deref for BoundReferenceSchema {
    type Target = NormalizedReferenceSchema;
    fn deref(&self) -> &Self::Target {
        &self.canonical
    }
}

impl NormalizedValueSchema for BoundReferenceSchema {
    fn comparable(&self, ty: TypeObjectDigest) -> bool {
        self.canonical.comparable_types.contains(&ty)
    }
    fn application_free(&self, ty: TypeObjectDigest) -> bool {
        self.canonical.application_free_types.contains(&ty)
    }
    fn record_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.canonical.record_instances.get(&ty).copied()
    }
    fn variant_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.canonical.variant_instances.get(&ty).copied()
    }
    fn value_origin(&self) -> super::value::ValueOrigin {
        self.value_origin
    }
    fn records(&self) -> &[super::prepare::NormalizedRecordLayout] {
        &self.canonical.records
    }
    fn variants(&self) -> &[super::prepare::NormalizedVariantLayout] {
        &self.canonical.variants
    }
    fn types(&self) -> &BTreeMap<TypeObjectDigest, crate::platform::kernel::TypeObject> {
        &self.canonical.types
    }
}

pub struct ReferenceSignature {
    has_implementations: bool,
    requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    effect_parameters: Vec<crate::platform::semantic_id::EffectParameterId>,
    effect: FunctionEffect,
    type_parameters: Vec<TypeParameterId>,
    type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
    parameters: Vec<ParameterRecord>,
    result: TypeObjectDigest,
    result_borrow: Option<ParameterId>,
    pure: bool,
}

pub struct NormalizedReferenceInterpreter<'a> {
    authority: &'a dyn NormalizedReferenceRead,
    program: &'a NormalizedProgram,
    policy: NormalizedRunPolicy,
    host: Option<&'a dyn NormalizedReferenceHost>,
    observer: Option<&'a std::sync::Mutex<Option<NormalizedReferenceObservation>>>,
}

impl<'a> NormalizedReferenceInterpreter<'a> {
    #[cfg(test)]
    pub fn new(
        snapshot: &'a KernelSnapshot,
        program: &'a NormalizedProgram,
        policy: NormalizedRunPolicy,
    ) -> Self {
        Self::from_reader(snapshot, program, policy)
    }

    pub fn from_reader(
        authority: &'a dyn NormalizedReferenceRead,
        program: &'a NormalizedProgram,
        policy: NormalizedRunPolicy,
    ) -> Self {
        Self {
            authority,
            program,
            policy,
            host: None,
            observer: None,
        }
    }

    pub(super) fn observing_checked(
        mut self,
        observer: &'a std::sync::Mutex<Option<NormalizedReferenceObservation>>,
    ) -> Self {
        self.observer = Some(observer);
        self
    }

    pub(super) fn observing(
        mut self,
        observer: &'a std::sync::Mutex<Option<NormalizedReferenceObservation>>,
        host: &'a dyn NormalizedReferenceHost,
    ) -> Self {
        self.observer = Some(observer);
        self.host = Some(host);
        self
    }

    pub(super) fn invoke(
        &self,
        declaration: DeclarationReference,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let arguments = super::value::RawArguments::new(arguments);
        self.execute(capabilities, control, |state| {
            state.select_root_function(declaration)?;
            let arguments = state.admit_call_arguments(declaration, &[], arguments.into_vec())?;
            state.call_declaration(declaration, &[], &[], &[], arguments)
        })
    }

    #[cfg(test)]
    pub(super) fn invoke_instantiated(
        &self,
        declaration: DeclarationReference,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let arguments = super::value::RawArguments::new(arguments);
        self.execute(None, control, |state| {
            let arguments = state.admit_call_arguments(declaration, types, arguments.into_vec())?;
            state.call_declaration(declaration, types, &[], &[], arguments)
        })
    }

    pub fn invoke_root_target(
        &self,
        name: &Name,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let arguments = super::value::RawArguments::new(arguments);
        let resources = NormalizedResourceScope::new()?;
        self.invoke_root_target_scoped(
            name,
            arguments.into_vec(),
            capabilities,
            &resources,
            control,
        )
    }

    pub(crate) fn invoke_root_target_scoped(
        &self,
        name: &Name,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let arguments = super::value::RawArguments::new(arguments);
        let expected_name = name.clone();
        self.execute_scoped(capabilities, resources, control, move |state| {
            let target_id = state
                .schema
                .targets
                .get(&(state.binding.package, expected_name.clone()))
                .copied()
                .ok_or_else(|| {
                    reference_error(
                        "normalized_reference_target_missing",
                        "canonical root package has no target with the exact selected name",
                    )
                })?;
            let record = match state.owner(OwnerKey::Target(target_id))? {
                Some(OwnerRecord::Target(record)) => record,
                Some(_) => {
                    return Err(reference_error(
                        "normalized_reference_target_kind",
                        "selected target identity names another canonical owner kind",
                    ));
                }
                None => {
                    return Err(reference_error(
                        "normalized_reference_target_owner",
                        "selected target is absent from canonical Graph 14 authority",
                    ));
                }
            };
            let port_reference = record.port.ok_or_else(|| {
                reference_error(
                    "normalized_reference_target_port_missing",
                    "selected non-HTTP target has no exact port",
                )
            })?;
            if record.name != expected_name
                || record.component.package != state.binding.package
                || port_reference.package != state.binding.package
            {
                return Err(reference_error(
                    "normalized_reference_target_binding",
                    "selected target disagrees with its exact prepared artifact binding",
                ));
            }
            let port = match state.owner(OwnerKey::Port(port_reference.port))? {
                Some(OwnerRecord::Port(port)) => port,
                Some(_) => {
                    return Err(reference_error(
                        "normalized_reference_port_kind",
                        "selected target port identity names another canonical owner kind",
                    ));
                }
                None => {
                    return Err(reference_error(
                        "normalized_reference_port_missing",
                        "selected target port is absent from canonical Graph 14 authority",
                    ));
                }
            };
            if port.declaration != record.component.declaration {
                return Err(reference_error(
                    "normalized_reference_port_component",
                    "selected target port belongs to another exact component",
                ));
            }
            state.root_allowance = match state
                .schema
                .types
                .get(&port.function_type)
                .map(|ty| &ty.form)
            {
                Some(TypeForm::TaskFunction { effect, .. }) if effect.is_closed() => {
                    Some(effect.clone())
                }
                Some(TypeForm::Function { .. }) => None,
                _ => {
                    return Err(reference_type_error(
                        "port entry must have a closed exact callable kind and effect row",
                    ));
                }
            };
            match port.implementation {
                PortImplementation::Function(function) => {
                    let arguments =
                        state.admit_call_arguments(function, &[], arguments.into_vec())?;
                    state.call_declaration(function, &[], &[], &[], arguments)
                }
                PortImplementation::Expression(expression) => {
                    let arguments =
                        state.admit_port_arguments(port.function_type, arguments.into_vec())?;
                    let callee = state.evaluate(expression, &mut BTreeMap::new())?;
                    let (
                        declaration,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        arguments,
                    ) = state.callable_arguments(callee, arguments)?;
                    state.call_declaration(
                        declaration,
                        &type_arguments,
                        &effect_arguments,
                        &requirement_arguments,
                        arguments,
                    )
                }
            }
        })
    }

    pub fn invoke_test(
        &self,
        declaration: DeclarationReference,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceTestInvocation, ExecutionError> {
        let actual = self.execute(capabilities, control, |state| {
            state.evaluate_test(declaration, false)
        })?;
        let expected = self.execute(capabilities, control, |state| {
            state.evaluate_test(declaration, true)
        })?;
        Ok((actual, expected))
    }

    fn execute(
        &self,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
        operation: impl FnOnce(&mut ReferenceState<'_>) -> Result<CheckedValue, ExecutionError>,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let resources = NormalizedResourceScope::new()?;
        self.execute_scoped(capabilities, &resources, control, operation)
    }

    fn execute_scoped(
        &self,
        capabilities: Option<&NormalizedCapabilities>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
        operation: impl FnOnce(&mut ReferenceState<'_>) -> Result<CheckedValue, ExecutionError>,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        validate_reference_policy(self.policy)?;
        control.check()?;
        let binding = self.authority.binding()?;
        if !binding.matches(self.program) {
            return Err(reference_error(
                "normalized_reference_authority_binding",
                "reference authority and executable artifact do not bind one exact accepted root",
            ));
        }
        let schema = Arc::new(BoundReferenceSchema {
            canonical: self.authority.schema()?,
            value_origin: self.program.value_origin,
        });
        let schema_work = schema.work;
        let type_derivation_steps = schema.type_derivation_steps;
        let source_admission_steps = schema.source_admission_steps;
        let type_metadata_bytes = schema.type_metadata_bytes;
        let list_work = super::list::Work::current();
        let map_work = super::map::Work::current();
        let mut state = ReferenceState {
            reference_witnesses: Arc::new(std::sync::Mutex::new(
                ReferenceWitnessInterner::default(),
            )),
            shared_budget: None,
            structured_depth: 0,
            ancestor_depth: 0,
            memory_domain: super::value::ValueOrigin::fresh().ok_or_else(|| {
                reference_resource(
                    "normalized_buffer_domain",
                    "memory invocation identity exhausted",
                )
            })?,
            authority: self.authority,
            binding,
            active_package: binding.package,
            program: self.program,
            schema,
            policy: self.policy,
            host: self.host,
            capabilities,
            resources,
            control,
            remaining_expressions: self.policy.instruction_steps,
            call_depth: 0,
            control_frames: 0,
            lexical_loan_scopes: 0,
            borrow_result_sources: Vec::new(),
            borrow_result_demand: false,
            local_counts: Vec::new(),
            next_transaction: 0,
            transactions: BTreeMap::new(),
            calls_by_requirement: BTreeMap::new(),
            implementation_scopes: Vec::new(),
            type_scopes: Vec::new(),
            effect_scopes: Vec::new(),
            requirement_scopes: Vec::new(),
            allowances: Vec::new(),
            root_allowance: None,
            observation: NormalizedReferenceObservation {
                expressions: 0,
                calls: 0,
                external_calls: 0,
                capability_calls: 0,
                allocated_bytes: 0,
                allocation_charges: 0,
                type_derivation_steps,
                source_admission_steps,
                type_metadata_bytes,
                value_work: super::value::ValueWork::default(),
                collection_items: 0,
                maximum_call_depth: 0,
                canonical_owner_reads: schema_work.owner_reads,
                canonical_map_pages_read: schema_work.map_pages_read,
                canonical_objects_read: schema_work.objects_read,
                canonical_bytes_read: schema_work.bytes_read,
                production_tier: "graph14_reference_records_10",
                tail_transfers: 0,
                maximum_control_frames: 0,
                maximum_live_locals: 0,
                maximum_live_type_bindings: 0,
                maximum_live_effect_bindings: 0,
                maximum_live_allowances: 0,
                live_call_frames_after: 0,
                live_control_frames_after: 0,
                live_local_scopes_after: 0,
                live_type_scopes_after: 0,
                live_effect_scopes_after: 0,
                live_allowances_after: 0,
                live_transactions_after: 0,
                live_handles_after: 0,
            },
        };
        let operation = (|| {
            state.control.check()?;
            let bytes = usize::try_from(state.schema.type_metadata_bytes)
                .ok()
                .and_then(|bytes| bytes.checked_add(state.schema.affine_variants.len()))
                .and_then(|bytes| bytes.checked_add(std::mem::size_of::<BoundReferenceSchema>()))
                .ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_allocation",
                        "canonical proof storage accounting overflowed",
                    )
                })?;
            state.charge_allocation(bytes as u64)?;
            operation(&mut state)
        })();
        let result = match operation {
            Ok(value) if state.transactions.is_empty() => Ok(value),
            Ok(_) => {
                state.rollback_all();
                Err(reference_error(
                    "normalized_reference_transaction_leak",
                    "reference execution completed with a live transaction",
                ))
            }
            Err(error) => {
                state.rollback_all();
                Err(error)
            }
        };
        if result.is_err() {
            resources.release_all();
        }
        state.observation.live_handles_after = resources.live_resources();
        state.observation.live_call_frames_after = state.call_depth;
        state.observation.live_control_frames_after = state.control_frames;
        state.observation.live_local_scopes_after = state.local_counts.len();
        state.observation.live_type_scopes_after = state.type_scopes.len();
        state.observation.live_effect_scopes_after = state.effect_scopes.len();
        state.observation.live_allowances_after = state.allowances.len();
        state.observation.live_transactions_after = state.transactions.len();
        state.observation.value_work.lists = list_work.since();
        state.observation.value_work.maps = map_work.since();
        if let Some(observer) = self.observer {
            let mut observed = observer.lock().map_err(|_| {
                reference_error(
                    "normalized_reference_observation_lock",
                    "execution observation lock is poisoned",
                )
            })?;
            *observed = Some(state.observation.clone());
        }
        result.and_then(|value| {
            if value.ownership(&state.schema, &mut state.observation.value_work)?
                == Ownership::Memory
            {
                return Err(reference_type_error(
                    "ByteBuffer cannot cross raw results; freeze it",
                ));
            }
            Ok((value.release(), state.observation))
        })
    }
}

#[path = "reference_implementations.rs"]
mod implementations;
use implementations::{AppliedReferenceImplementation, ReferenceWitnessInterner};

struct ReferenceTransaction {
    binding: BindingId,
    generation: u64,
    transaction: Box<dyn NormalizedCapabilityTransaction>,
}

#[derive(Default)]
struct ReferenceApplication<'a> {
    types: &'a [TypeObjectDigest],
    effects: &'a [EffectRow],
    requirements: &'a [RequirementOperand],
    implementations: &'a [AppliedReferenceImplementation],
}

struct ReferenceState<'a> {
    reference_witnesses: Arc<std::sync::Mutex<ReferenceWitnessInterner>>,
    shared_budget: Option<Arc<super::shared_budget::SharedBudget>>,
    structured_depth: usize,
    ancestor_depth: usize,
    memory_domain: super::value::ValueOrigin,
    authority: &'a dyn NormalizedReferenceRead,
    binding: NormalizedReferenceBinding,
    active_package: PackageId,
    program: &'a NormalizedProgram,
    schema: Arc<BoundReferenceSchema>,
    policy: NormalizedRunPolicy,
    host: Option<&'a dyn NormalizedReferenceHost>,
    capabilities: Option<&'a NormalizedCapabilities>,
    resources: &'a NormalizedResourceScope,
    control: &'a ExecutionControl,
    remaining_expressions: Option<u64>,
    call_depth: usize,
    control_frames: usize,
    lexical_loan_scopes: usize,
    borrow_result_sources: Vec<Option<ParameterId>>,
    borrow_result_demand: bool,
    local_counts: Vec<usize>,
    next_transaction: u64,
    transactions: BTreeMap<RequirementReference, ReferenceTransaction>,
    calls_by_requirement: BTreeMap<RequirementReference, u64>,
    implementation_scopes: Vec<(
        DeclarationReference,
        BTreeMap<
            crate::platform::semantic_id::ImplementationParameterId,
            AppliedReferenceImplementation,
        >,
    )>,
    type_scopes: Vec<BTreeMap<TypeParameterId, TypeObjectDigest>>,
    effect_scopes: Vec<super::reference_effects::Bindings>,
    requirement_scopes: Vec<super::reference_effects::RequirementBindings>,
    allowances: Vec<Option<EffectRow>>,
    root_allowance: Option<EffectRow>,
    observation: NormalizedReferenceObservation,
}

enum ReferenceStep {
    Value(CheckedValue),
    Tail(Box<AdmittedGraphCall>),
}

/// Lexical custody is retained across body execution and host unwind. Removing
/// the child before dropping the parent also preserves nested loan order.
struct ReferenceLoanScope<'a> {
    parent: Option<CheckedValue>,
    locals: &'a mut BTreeMap<LocalValueReference, CheckedValue>,
    local: LocalValueReference,
}

impl Drop for ReferenceLoanScope<'_> {
    fn drop(&mut self) {
        self.locals.remove(&self.local);
    }
}

/// One internal transition, constructed only by canonical call admission in this state.
/// It is neither a callable value nor a reusable proof, and carries no component grant.
struct AdmittedGraphCall {
    implementations: BTreeMap<
        crate::platform::semantic_id::ImplementationParameterId,
        AppliedReferenceImplementation,
    >,
    declaration: DeclarationReference,
    function: FunctionDeclaration,
    types: BTreeMap<TypeParameterId, TypeObjectDigest>,
    effects: super::reference_effects::Bindings,
    requirements: super::reference_effects::RequirementBindings,
    allowance: Option<EffectRow>,
    arguments: Vec<CheckedValue>,
}

impl ReferenceState<'_> {
    fn declared_row(&mut self, effect: &FunctionEffect) -> Result<EffectRow, ExecutionError> {
        if let FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = effect
        {
            self.control.check()?;
            self.charge_items(
                requirements.len(),
                std::mem::size_of::<RequirementReference>(),
            )?;
            self.charge_items(
                effect_parameters.len(),
                std::mem::size_of::<EffectParameterReference>(),
            )?;
        }
        Ok(effect.row())
    }

    fn close_row(
        &mut self,
        row: &EffectRow,
        scope: &super::reference_effects::Bindings,
        requirements: &super::reference_effects::RequirementBindings,
    ) -> Result<EffectRow, ExecutionError> {
        super::reference_effects::close(row, scope, requirements, |count| {
            self.control.check()?;
            self.charge_items(
                count,
                std::mem::size_of::<RequirementReference>() + 3 * std::mem::size_of::<usize>(),
            )
        })
    }

    fn resolve_effect_arguments(
        &mut self,
        arguments: &[EffectRow],
    ) -> Result<Vec<EffectRow>, ExecutionError> {
        self.charge_items(arguments.len(), std::mem::size_of::<EffectRow>())?;
        let had_scope = !self.effect_scopes.is_empty();
        let scope = self.effect_scopes.pop().unwrap_or_default();
        let requirement_scope = self.requirement_scopes.pop().unwrap_or_default();
        let result = arguments
            .iter()
            .map(|row| self.close_row(row, &scope, &requirement_scope))
            .collect();
        // Keep the exact caller scope even when closing an argument fails.
        if had_scope {
            self.effect_scopes.push(scope);
            self.requirement_scopes.push(requirement_scope);
        }
        result
    }

    fn resolve_requirement(
        &self,
        operand: RequirementOperand,
    ) -> Result<RequirementReference, ExecutionError> {
        self.control.check()?;
        let empty = BTreeMap::new();
        super::reference_effects::resolve(operand, self.requirement_scopes.last().unwrap_or(&empty))
    }

    fn resolve_requirement_arguments(
        &mut self,
        operands: &[RequirementOperand],
    ) -> Result<Vec<RequirementOperand>, ExecutionError> {
        self.charge_items(operands.len(), std::mem::size_of::<RequirementOperand>())?;
        operands
            .iter()
            .map(|operand| {
                self.resolve_requirement(*operand)
                    .map(RequirementOperand::Concrete)
            })
            .collect()
    }

    fn requirement_bindings(
        &mut self,
        declaration: DeclarationReference,
        parameters: &[crate::platform::semantic_id::RequirementParameterId],
        arguments: &[RequirementOperand],
    ) -> Result<super::reference_effects::RequirementBindings, ExecutionError> {
        if parameters.len() != arguments.len() {
            return Err(reference_type_error(
                "requirement argument arity differs from its exact target",
            ));
        }
        self.charge_items(
            arguments.len(),
            std::mem::size_of::<(RequirementParameterReference, RequirementReference)>()
                + 3 * std::mem::size_of::<usize>(),
        )?;
        let mut bindings = BTreeMap::new();
        for (parameter, argument) in parameters.iter().zip(arguments) {
            self.control.check()?;
            let Some(OwnerRecord::RequirementParameter(formal)) = self.owner_in_package(
                declaration.package,
                OwnerKey::RequirementParameter(*parameter),
            )?
            else {
                return Err(reference_type_error(
                    "requirement formal has no canonical owner",
                ));
            };
            let RequirementOperand::Concrete(reference) = argument else {
                return Err(reference_type_error(
                    "requirement argument must be closed before entering its callee",
                ));
            };
            let Some(OwnerRecord::Requirement(actual)) = self.owner_in_package(
                reference.package,
                OwnerKey::Requirement(reference.requirement),
            )?
            else {
                return Err(reference_type_error(
                    "requirement argument has no exact component owner",
                ));
            };
            if formal.declaration != declaration.declaration
                || formal.constraint.interface != actual.interface
            {
                return Err(reference_type_error(
                    "requirement argument has a foreign scope or exact interface",
                ));
            }
            for operation in &formal.constraint.operations {
                self.control.check()?;
                if !actual.operations.contains(operation) {
                    return Err(reference_type_error(
                        "requirement argument does not supply the formal minimum operations",
                    ));
                }
            }
            if bindings
                .insert(
                    RequirementParameterReference {
                        package: declaration.package,
                        parameter: *parameter,
                    },
                    *reference,
                )
                .is_some()
            {
                return Err(reference_type_error(
                    "requirement parameters repeat an exact identity",
                ));
            }
        }
        Ok(bindings)
    }

    fn effect_bindings(
        &mut self,
        reference: DeclarationReference,
        parameters: &[crate::platform::semantic_id::EffectParameterId],
        arguments: &[EffectRow],
    ) -> Result<super::reference_effects::Bindings, ExecutionError> {
        if parameters.len() != arguments.len() {
            return Err(reference_type_error(
                "effect argument arity disagrees with the exact target declaration",
            ));
        }
        self.charge_items(
            arguments.len(),
            std::mem::size_of::<(EffectParameterReference, EffectRow)>()
                + 3 * std::mem::size_of::<usize>(),
        )?;
        let mut result = BTreeMap::new();
        for (parameter, row) in parameters.iter().zip(arguments) {
            self.control.check()?;
            if !matches!(self.owner_in_package(reference.package, OwnerKey::EffectParameter(*parameter))?, Some(OwnerRecord::EffectParameter(owner)) if owner.declaration == reference.declaration)
            {
                return Err(reference_type_error(
                    "effect parameter belongs to another exact function",
                ));
            }
            let closed = self.close_row(row, &BTreeMap::new(), &BTreeMap::new())?;
            if result
                .insert(
                    EffectParameterReference {
                        package: reference.package,
                        parameter: *parameter,
                    },
                    closed,
                )
                .is_some()
            {
                return Err(reference_type_error("duplicate effect parameter identity"));
            }
        }
        Ok(result)
    }

    fn admit_task_row(&mut self, required: &EffectRow) -> Result<(), ExecutionError> {
        let allowance = self
            .allowances
            .last()
            .unwrap_or(&self.root_allowance)
            .as_ref()
            .ok_or_else(|| {
                reference_type_error(
                    "task invocation requires a declared task context, including an empty row",
                )
            })?;
        self.control.check()?;
        let count = allowance.requirements.len();
        self.charge_items(count, std::mem::size_of::<RequirementReference>())?;
        let available = self
            .allowances
            .last()
            .unwrap_or(&self.root_allowance)
            .as_ref()
            .ok_or_else(|| reference_type_error("activation allowance disappeared"))?
            .requirements
            .clone();
        for operand in &required.requirements {
            let requirement = operand
                .concrete()
                .ok_or_else(|| reference_type_error("unclosed activation requirement"))?;
            let capabilities = self
                .capabilities
                .ok_or_else(reference_capabilities_unbound)?;
            let grant = capabilities.canonical_requirement_exact(self.program, requirement)?;
            let mut found = false;
            for operand in &available {
                let candidate = operand
                    .concrete()
                    .ok_or_else(|| reference_type_error("unclosed available requirement"))?;
                self.control.check()?;
                if (candidate == requirement
                    || self.reference_requirement_covers(requirement, candidate)?)
                    && capabilities.canonical_requirement_exact(self.program, candidate)? == grant
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(reference_type_error(format!(
                    "callback requirement {}/{} exceeds the current canonical activation allowance or grant binding",
                    requirement.package, requirement.requirement
                )));
            }
        }
        Ok(())
    }

    fn reference_requirement_covers(
        &mut self,
        required: RequirementReference,
        available: RequirementReference,
    ) -> Result<bool, ExecutionError> {
        if required.package != available.package {
            return Ok(false);
        }
        let Some(OwnerRecord::Requirement(required)) = self.owner_in_package(
            required.package,
            OwnerKey::Requirement(required.requirement),
        )?
        else {
            return Ok(false);
        };
        let Some(OwnerRecord::Requirement(available)) = self.owner_in_package(
            available.package,
            OwnerKey::Requirement(available.requirement),
        )?
        else {
            return Ok(false);
        };
        Ok(required.name == available.name
            && required.interface == available.interface
            && required
                .operations
                .iter()
                .all(|operation| available.operations.contains(operation))
            && required.limits.iter().all(|limit| {
                available.limits.iter().any(|candidate| {
                    candidate.name == limit.name
                        && candidate.unit == limit.unit
                        && candidate.maximum <= limit.maximum
                })
            }))
    }

    fn admit_operation_allowance(
        &self,
        requirement: RequirementReference,
    ) -> Result<(), ExecutionError> {
        if self
            .allowances
            .last()
            .unwrap_or(&self.root_allowance)
            .as_ref()
            .is_none_or(|row| !row.requirements.contains(&requirement.into()))
        {
            return Err(reference_type_error(
                "operation is outside the current canonical activation allowance",
            ));
        }
        Ok(())
    }

    fn select_root_function(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<(), ExecutionError> {
        let function = self.declaration(reference)?;
        if let DeclarationPayload::Function(function) = function.payload {
            if !function.effect_parameters.is_empty() || !function.requirement_parameters.is_empty()
            {
                return Err(reference_type_error(
                    "entry effect arguments are not closed",
                ));
            }
            if !matches!(function.effect, FunctionEffect::Pure) {
                self.root_allowance = Some(self.declared_row(&function.effect)?);
            }
        }
        Ok(())
    }

    fn applied_signature(
        &mut self,
        reference: DeclarationReference,
        types: &[TypeObjectDigest],
        effects: &[EffectRow],
        requirements: &[RequirementOperand],
    ) -> Result<ReferenceSignature, ExecutionError> {
        let mut signature = self.function_signature(reference)?;
        if signature.type_parameters.len() != types.len() {
            return Err(reference_type_error(
                "callable type argument arity differs from its exact target",
            ));
        }
        let requirement_scope =
            self.requirement_bindings(reference, &signature.requirement_parameters, requirements)?;
        let scope = self.effect_bindings(reference, &signature.effect_parameters, effects)?;
        let declared = self.declared_row(&signature.effect)?;
        let row = self.close_row(&declared, &scope, &requirement_scope)?;
        if !signature.pure {
            signature.effect = FunctionEffect::Task {
                requirements: row.requirements,
                effect_parameters: Vec::new(),
            };
        }
        // Zero-effect-arity signatures retain their canonical ordinary-type templates.
        // Raw argument/prefix admission substitutes those templates structurally, including
        // valid pure applications whose aggregate digest was not a prepared graph root.
        if scope.is_empty() && requirement_scope.is_empty() {
            return Ok(signature);
        }
        self.charge_items(
            types.len(),
            std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>(),
        )?;
        let bindings = signature
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect();
        for ty in signature
            .parameters
            .iter_mut()
            .map(|p| &mut p.ty)
            .chain(std::iter::once(&mut signature.result))
        {
            *ty = self
                .schema
                .instantiated_with_effects(*ty, &bindings, &scope, &requirement_scope, 0)
                .filter(|ty| self.schema.types.contains_key(ty))
                .ok_or_else(|| {
                    reference_type_error(
                        "callable signature has an unresolved nested type or effect application",
                    )
                })?;
        }
        Ok(signature)
    }

    fn evaluate_test(
        &mut self,
        reference: DeclarationReference,
        expected: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        let previous_package = self.active_package;
        self.active_package = reference.package;
        let result = (|| {
            let record = self.declaration(reference)?;
            let DeclarationPayload::Test {
                actual,
                expected: expected_expression,
                ..
            } = record.payload
            else {
                return Err(reference_error(
                    "normalized_reference_test_kind",
                    "exact test selection names another declaration kind",
                ));
            };
            self.evaluate(
                if expected {
                    expected_expression
                } else {
                    actual
                },
                &mut BTreeMap::new(),
            )
        })();
        self.active_package = previous_package;
        result
    }

    fn product_child(
        &mut self,
        raw: NormalizedValue,
        ty: TypeObjectDigest,
    ) -> Result<CheckedValue, ExecutionError> {
        let expected = direct_memory_type(&self.schema, ty, &BTreeMap::new(), self.control)?;
        if let Some(expected) = expected {
            if raw.memory_form() != Some(expected) {
                return Err(reference_type_error("owned product child type mismatch"));
            }
            raw.memory_validate(self.memory_domain, true)?;
            CheckedValue::memory(&self.schema, raw)
        } else {
            self.admit_raw(raw, ty, &BTreeMap::new(), None, false)
        }
    }

    fn borrowed_product_child(
        &mut self,
        raw: NormalizedValue,
        ty: TypeObjectDigest,
    ) -> Result<CheckedValue, ExecutionError> {
        let expected = direct_memory_type(&self.schema, ty, &BTreeMap::new(), self.control)?;
        if let Some(expected) = expected {
            if raw.memory_form() != Some(expected) || !raw.memory_is_borrowed() {
                return Err(reference_type_error("borrowed child type or loan mismatch"));
            }
            raw.memory_validate(self.memory_domain, false)?;
            CheckedValue::memory(&self.schema, raw)
        } else {
            self.admit_raw(raw, ty, &BTreeMap::new(), None, false)
        }
    }

    fn sequence_item_type(
        &mut self,
        sequence_type: TypeObjectDigest,
    ) -> Result<(TypeObjectDigest, TypeObjectDigest), ExecutionError> {
        let ty = self.resolve_type_arguments(&[sequence_type])?[0];
        let Some(TypeForm::OwnedSequence { item }) =
            self.schema.types.get(&ty).map(|object| &object.form)
        else {
            return Err(reference_type_error(
                "sequence requires a closed owned sequence type",
            ));
        };
        let item = *item;
        if direct_memory_type(&self.schema, item, &BTreeMap::new(), self.control)?.is_none() {
            return Err(reference_type_error("sequence element must be owned"));
        }
        Ok((ty, item))
    }

    fn sequence_pop_types(
        &mut self,
        sequence: TypeObjectDigest,
        item: TypeObjectDigest,
        result_type: TypeObjectDigest,
    ) -> Result<(TypeObjectDigest, TypeObjectDigest), ExecutionError> {
        let result = self.resolve_type_arguments(&[result_type])?[0];
        let Some(TypeForm::OwnedChoice { cases }) =
            self.schema.types.get(&result).map(|object| &object.form)
        else {
            return Err(reference_type_error(
                "sequence pop requires its exact owned result choice",
            ));
        };
        if cases.len() != 2
            || cases[0].name.as_str() != "empty"
            || cases[0].ty != sequence
            || cases[1].name.as_str() != "item"
        {
            return Err(reference_type_error(
                "sequence pop result cases disagree with its sequence",
            ));
        }
        let product = cases[1].ty;
        let Some(TypeForm::OwnedProduct { fields }) =
            self.schema.types.get(&product).map(|object| &object.form)
        else {
            return Err(reference_type_error(
                "sequence pop item case requires an owned product",
            ));
        };
        if fields.len() != 2
            || fields[0].name.as_str() != "rest"
            || fields[0].ty != sequence
            || fields[1].name.as_str() != "value"
            || fields[1].ty != item
        {
            return Err(reference_type_error(
                "sequence pop item fields disagree with its element type",
            ));
        }
        Ok((result, product))
    }

    fn consume_sequence_local(
        &mut self,
        source: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        if !matches!(
            self.owner(OwnerKey::Expression(source))?,
            Some(OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Local { .. })
        ) {
            return Err(reference_type_error(
                "sequence consumption requires an exact owning local",
            ));
        }
        self.evaluate_with_use(source, locals, ParameterUse::Consume)
    }

    fn bind_owned_choice(
        &mut self,
        choice_type: TypeObjectDigest,
        source: ExpressionId,
        arms: &[crate::platform::kernel::OwnedChoiceArm],
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<(ExpressionId, LocalValueReference), ExecutionError> {
        let ty = self.resolve_type_arguments(&[choice_type])?[0];
        let Some(TypeForm::OwnedChoice { cases }) =
            self.schema.types.get(&ty).map(|object| &object.form)
        else {
            return Err(reference_type_error(
                "owned match requires a closed owned choice type",
            ));
        };
        if cases.len() != arms.len()
            || cases
                .iter()
                .zip(arms)
                .any(|(case, arm)| case.name != arm.name)
        {
            return Err(reference_type_error("owned match case coverage mismatch"));
        }
        self.charge_allocation(
            (std::mem::size_of::<LocalValueReference>() + std::mem::size_of::<CheckedValue>())
                as u64,
        )?;
        self.control.check()?;
        let NormalizedValue::OwnedChoice(token) = self.evaluate(source, locals)?.release() else {
            return Err(reference_type_error("owned match requires a choice token"));
        };
        let (selected, payload) = token.select(self.memory_domain, ty, self.control)?;
        let arm = arms
            .get(selected as usize)
            .ok_or_else(|| reference_type_error("invalid selected choice arm"))?;
        let TypeForm::OwnedChoice { cases } = &self.schema.types[&ty].form else {
            return Err(reference_type_error("missing choice shape"));
        };
        let payload_type = cases[selected as usize].ty;
        let binding = self.binding(arm.binding, BindingKind::OwnedChoicePayload)?;
        let declared = binding
            .declared_type
            .ok_or_else(|| reference_type_error("owned choice binding lacks type"))?;
        if self.resolve_type_arguments(&[declared])?[0] != payload_type {
            return Err(reference_type_error("owned choice payload type mismatch"));
        }
        let local = LocalValueReference::LexicalBinding(arm.binding);
        if locals.contains_key(&local) {
            return Err(reference_type_error(
                "owned choice binding aliases a live local",
            ));
        }
        let payload = self.product_child(payload, payload_type)?;
        locals.insert(local, payload);
        Ok((arm.body, local))
    }

    fn bind_owned_product(
        &mut self,
        product_type: TypeObjectDigest,
        source: ExpressionId,
        fields: &[crate::platform::kernel::OwnedProductBinding],
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<(), ExecutionError> {
        let ty = self.resolve_type_arguments(&[product_type])?[0];
        let Some(TypeForm::OwnedProduct { fields: expected }) =
            self.schema.types.get(&ty).map(|o| &o.form)
        else {
            return Err(reference_type_error(
                "unpack requires a closed product type",
            ));
        };
        if expected.len() != fields.len()
            || expected.iter().zip(fields).any(|(a, b)| a.name != b.name)
        {
            return Err(reference_type_error("unpack field coverage mismatch"));
        }
        if fields
            .iter()
            .any(|field| locals.contains_key(&LocalValueReference::LexicalBinding(field.binding)))
        {
            return Err(reference_type_error("duplicate unpack local"));
        }
        self.charge_allocation(super::value::collection_storage_bytes(
            fields.len() as u64,
            (std::mem::size_of::<LocalValueReference>() + std::mem::size_of::<CheckedValue>())
                as u64,
            "normalized_product_allocation",
        )?)?;
        self.control.check()?;
        let NormalizedValue::OwnedProduct(token) = self.evaluate(source, locals)?.release() else {
            return Err(reference_type_error("unpack requires product token"));
        };
        let values = token.unpack(self.memory_domain, ty, self.control)?;
        if values.len() != fields.len() {
            return Err(reference_type_error("product storage count mismatch"));
        }
        let mut bound = 0;
        let result = (|| {
            for (index, (field, value)) in fields.iter().zip(values).enumerate() {
                let TypeForm::OwnedProduct { fields: expected } = &self.schema.types[&ty].form
                else {
                    return Err(reference_type_error("missing product shape"));
                };
                let expected = expected[index].ty;
                let binding = self.binding(field.binding, BindingKind::OwnedUnpack)?;
                let declared = binding
                    .declared_type
                    .ok_or_else(|| reference_type_error("unpack binding lacks type"))?;
                if self.resolve_type_arguments(&[declared])?[0] != expected {
                    return Err(reference_type_error("unpack local type mismatch"));
                }
                let value = self.product_child(value, expected)?;
                let local = LocalValueReference::LexicalBinding(field.binding);
                if locals.contains_key(&local) {
                    return Err(reference_type_error("duplicate unpack local"));
                }
                locals.insert(local, value);
                bound += 1;
            }
            Ok(())
        })();
        if result.is_err() {
            for field in &fields[..bound] {
                locals.remove(&LocalValueReference::LexicalBinding(field.binding));
            }
        }
        result
    }

    fn borrow_parent(
        &mut self,
        source: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        if !matches!(
            self.owner(OwnerKey::Expression(source))?,
            Some(OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Local { .. })
        ) {
            return Err(reference_type_error(
                "borrow source requires an exact live local",
            ));
        }
        self.evaluate_with_use(source, locals, ParameterUse::Borrow)
    }

    fn evaluate_borrowed_body(
        &mut self,
        parent: CheckedValue,
        binding: BindingId,
        mut value: CheckedValue,
        body: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let local = LocalValueReference::LexicalBinding(binding);
        if locals.contains_key(&local) {
            return Err(reference_type_error(
                "borrowed child aliases a live binding",
            ));
        }
        let loan_scopes = self.lexical_loan_scopes.checked_add(1).ok_or_else(|| {
            reference_resource(
                "normalized_reference_loan_scopes",
                "loan scope count overflowed",
            )
        })?;
        if value.raw().memory_is_borrowed() {
            value.set_provenance(parent.provenance().ok_or_else(|| {
                reference_type_error("borrow parent has no canonical local provenance")
            })?);
        }
        locals.insert(local, value);
        let mut scope = ReferenceLoanScope {
            parent: Some(parent),
            locals,
            local,
        };
        self.lexical_loan_scopes = loan_scopes;
        let demand = self.borrow_result_demand;
        let result = self.evaluate_in_context(body, scope.locals, demand).and_then(|mut value| {
            if value.raw().memory_is_borrowed() {
                let source = self.borrow_result_sources.last().copied().flatten()
                    .map(LocalValueReference::FunctionParameter);
                if !demand || source.is_none() || value.provenance() != source {
                    return Err(reference_type_error(
                        "borrowed child escaped its lexical scope without a matching result source",
                    ));
                }
                let parent = scope.parent.as_ref().ok_or_else(|| {
                    reference_type_error("borrow scope lost its custody guard")
                })?;
                if parent.provenance() == value.provenance() {
                    self.charge_allocation(CheckedValue::parent_storage_bytes() as u64)?;
                    value.reserve_parent(parent)?;
                    value.retain_parent(|| scope.parent.take().ok_or_else(|| {
                        reference_type_error("borrow scope custody was already transferred")
                    }))?;
                }
            }
            Ok(value)
        });
        self.lexical_loan_scopes -= 1;
        drop(scope);
        result
    }

    fn resolve_type_arguments(
        &mut self,
        type_arguments: &[TypeObjectDigest],
    ) -> Result<Vec<TypeObjectDigest>, ExecutionError> {
        let scratch = {
            let empty = BTreeMap::new();
            let substitutions = self.type_scopes.last().unwrap_or(&empty);
            let empty_effects = BTreeMap::new();
            let effects = self.effect_scopes.last().unwrap_or(&empty_effects);
            let effects_empty = effects.is_empty()
                && self
                    .requirement_scopes
                    .last()
                    .is_none_or(BTreeMap::is_empty);
            let mut bytes = 0_u64;
            for ty in type_arguments {
                if effects_empty
                    && self
                        .schema
                        .admitted_type_identity(*ty, substitutions, self.control)?
                        .is_some()
                {
                    continue;
                }
                bytes = bytes
                    .checked_add(self.schema.type_instantiation_scratch(
                        *ty,
                        effects,
                        self.control,
                    )?)
                    .ok_or_else(|| {
                        reference_resource(
                            "reference_raw_instantiation_scratch",
                            "call type argument scratch size overflowed",
                        )
                    })?;
            }
            bytes
        };
        if scratch != 0 {
            self.charge_allocation(scratch)?;
        }
        let empty = BTreeMap::new();
        let substitutions = self.type_scopes.last().unwrap_or(&empty);
        let empty_effects = BTreeMap::new();
        let effects = self.effect_scopes.last().unwrap_or(&empty_effects);
        let empty_requirements = BTreeMap::new();
        let requirements = self
            .requirement_scopes
            .last()
            .unwrap_or(&empty_requirements);
        type_arguments
            .iter()
            .map(|ty| {
                if effects.is_empty()
                    && requirements.is_empty()
                    && let Some(exact) =
                        self.schema
                            .admitted_type_identity(*ty, substitutions, self.control)?
                {
                    return Ok(exact);
                }
                self.schema
                    .instantiated_with_effects(*ty, substitutions, effects, requirements, 0)
                    .filter(|ty| self.schema.types.contains_key(ty))
                    .ok_or_else(|| {
                        reference_type_error("call type argument escaped its exact function scope")
                    })
            })
            .collect()
    }

    fn call_declaration(
        &mut self,
        reference: DeclarationReference,
        type_arguments: &[TypeObjectDigest],
        effect_arguments: &[EffectRow],
        requirement_arguments: &[RequirementOperand],
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.control.check()?;
        self.require_owning_result(reference)?;
        let type_arguments = self.resolve_type_arguments(type_arguments)?;
        let effect_arguments = self.resolve_effect_arguments(effect_arguments)?;
        let requirement_arguments = self.resolve_requirement_arguments(requirement_arguments)?;
        if self.call_depth.saturating_add(self.ancestor_depth) >= self.policy.maximum_call_depth {
            return Err(reference_resource(
                "normalized_reference_call_depth",
                "reference execution exceeded its call-depth budget",
            ));
        }
        self.call_depth += 1;
        self.observation.maximum_call_depth = self
            .observation
            .maximum_call_depth
            .max(self.call_depth.saturating_add(self.ancestor_depth));
        let previous_package = self.active_package;
        self.active_package = reference.package;
        let mut step = self.call_activation(
            reference,
            &type_arguments,
            &effect_arguments,
            &requirement_arguments,
            arguments,
        );
        let result = loop {
            match step {
                Ok(ReferenceStep::Value(value)) => break Ok(value),
                Ok(ReferenceStep::Tail(target)) => {
                    // The outgoing canonical scope admitted this exact application. All its
                    // lexical/native frames have unwound; never reauthorize against an ancestor.
                    step = self.enter_graph_call(*target, true);
                }
                Err(error) => break Err(error),
            }
        };
        self.active_package = previous_package;
        self.call_depth -= 1;
        result
    }

    fn call_activation(
        &mut self,
        reference: DeclarationReference,
        type_arguments: &[TypeObjectDigest],
        effect_arguments: &[EffectRow],
        requirement_arguments: &[RequirementOperand],
        arguments: Vec<CheckedValue>,
    ) -> Result<ReferenceStep, ExecutionError> {
        self.control.check()?;
        (|| {
            let declaration = self.declaration(reference)?;
            match declaration.payload {
                DeclarationPayload::Function(function) => {
                    let target = self.admit_graph_call(
                        reference,
                        function,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        arguments,
                    )?;
                    self.enter_graph_call(target, false)
                }
                DeclarationPayload::External(external) => {
                    self.count_call(arguments.len())?;
                    let parameters = self.parameters(reference.package, &external.parameters)?;
                    for ty in type_arguments {
                        self.control.check()?;
                        if !self.schema.buffer_free_types.contains(ty) {
                            return Err(reference_type_error(
                                "external ordinary type argument contains owned memory",
                            ));
                        }
                    }
                    let types = external
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(type_arguments.iter().copied())
                        .collect();
                    self.validate_call_resources(&parameters, &types, &arguments, true)?;
                    if type_arguments.len() != external.type_parameters.len()
                        || !effect_arguments.is_empty()
                        || !requirement_arguments.is_empty()
                    {
                        Err(reference_type_error(
                            "external type-argument count disagrees with its exact signature",
                        ))
                    } else if arguments.len() != external.parameters.len() {
                        Err(reference_type_error(
                            "external argument count disagrees with canonical parameters",
                        ))
                    } else {
                        let signature = ReferenceSignature {
                            has_implementations: false,
                            requirement_parameters: Vec::new(),
                            effect_parameters: Vec::new(),
                            effect: FunctionEffect::Pure,
                            type_parameter_constraints: self
                                .type_parameter_constraints(reference, &external.type_parameters)?,
                            type_parameters: external.type_parameters,
                            parameters,
                            result: external.result,
                            result_borrow: None,
                            pure: true,
                        };
                        self.observation.external_calls =
                            self.observation.external_calls.saturating_add(1);
                        let value = if external.implementation.as_str().starts_with("core.buffer.")
                            || external.implementation.as_str().starts_with("core.cell.")
                        {
                            self.checked_intrinsic(
                                &signature,
                                external.implementation.as_str(),
                                type_arguments,
                                arguments,
                            )?
                        } else if let Some(host) = self.host {
                            let value = host.call(
                                self.schema.as_ref(),
                                &signature,
                                &external.implementation,
                                type_arguments,
                                arguments.into_iter().map(CheckedValue::release).collect(),
                                self.control,
                            )?;
                            let bindings = signature
                                .type_parameters
                                .iter()
                                .copied()
                                .zip(type_arguments.iter().copied())
                                .collect();
                            self.admit_raw(value, signature.result, &bindings, None, false)?
                        } else {
                            self.checked_intrinsic(
                                &signature,
                                external.implementation.as_str(),
                                type_arguments,
                                arguments,
                            )?
                        };
                        Ok(ReferenceStep::Value(value))
                    }
                }
                DeclarationPayload::Constant { value, .. } => {
                    self.count_call(arguments.len())?;
                    if !type_arguments.is_empty() {
                        Err(reference_type_error(
                            "constant call received type arguments",
                        ))
                    } else if arguments.is_empty() {
                        self.local_counts.push(0);
                        let result = self
                            .evaluate(value, &mut BTreeMap::new())
                            .map(ReferenceStep::Value);
                        self.local_counts.pop();
                        result
                    } else {
                        Err(reference_type_error("constant call received arguments"))
                    }
                }
                _ => Err(reference_type_error(
                    "exact callable reference names a non-callable declaration",
                )),
            }
        })()
    }

    fn count_call(&mut self, arguments: usize) -> Result<(), ExecutionError> {
        let bytes = arguments
            .checked_mul(
                std::mem::size_of::<CheckedValue>() - std::mem::size_of::<NormalizedValue>(),
            )
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_allocation",
                    "call allocation overflowed",
                )
            })?;
        self.charge_allocation(bytes as u64)?;
        self.observation.calls = self.observation.calls.saturating_add(1);
        Ok(())
    }

    fn admit_graph_call(
        &mut self,
        declaration: DeclarationReference,
        function: FunctionDeclaration,
        types: &[TypeObjectDigest],
        effects: &[EffectRow],
        requirements: &[RequirementOperand],
        arguments: Vec<CheckedValue>,
    ) -> Result<AdmittedGraphCall, ExecutionError> {
        self.admit_graph_call_with_implementations(
            declaration,
            function,
            arguments,
            ReferenceApplication {
                types,
                effects,
                requirements,
                implementations: &[],
            },
        )
    }

    fn admit_graph_call_with_implementations(
        &mut self,
        declaration: DeclarationReference,
        function: FunctionDeclaration,
        arguments: Vec<CheckedValue>,
        application: ReferenceApplication<'_>,
    ) -> Result<AdmittedGraphCall, ExecutionError> {
        let ReferenceApplication {
            types,
            effects,
            requirements,
            implementations: supplied,
        } = application;
        self.control.check()?;
        if arguments.len() != function.parameters.len()
            || types.len() != function.type_parameters.len()
            || effects.len() != function.effect_parameters.len()
            || requirements.len() != function.requirement_parameters.len()
        {
            return Err(reference_type_error(
                "call application disagrees with its exact canonical signature",
            ));
        }
        let constraints =
            self.type_parameter_constraints(declaration, &function.type_parameters)?;
        for (constraint, ty) in constraints.iter().zip(types) {
            self.control.check()?;
            let owned = matches!(
                self.schema.types.get(ty).map(|t| &t.form),
                Some(
                    TypeForm::ByteBuffer
                        | TypeForm::OwnedI64Cell
                        | TypeForm::OwnedProduct { .. }
                        | TypeForm::OwnedChoice { .. }
                        | TypeForm::OwnedSequence { .. }
                )
            );
            if owned != constraint.has_owned()
                || (!owned && !self.schema.buffer_free_types.contains(ty))
                || (constraint.requires_capture_safe()
                    && !self.schema.capture_safe_types.contains(ty))
                || (constraint.requires_transfer() && !self.schema.transferable_types.contains(ty))
            {
                return Err(reference_type_error(
                    "canonical callable type arguments fail their exact structural constraints",
                ));
            }
        }
        let requirement_scope =
            self.requirement_bindings(declaration, &function.requirement_parameters, requirements)?;
        let effect_scope =
            self.effect_bindings(declaration, &function.effect_parameters, effects)?;
        let declared = self.declared_row(&function.effect)?;
        let row = self.close_row(&declared, &effect_scope, &requirement_scope)?;
        let allowance = if matches!(function.effect, FunctionEffect::Pure) {
            None
        } else {
            // This must execute before the outgoing allowance is popped, even for empty rows.
            self.admit_task_row(&row)?;
            Some(row)
        };
        let parameters = self.parameters(declaration.package, &function.parameters)?;
        let types = function
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect::<BTreeMap<_, _>>();
        if let Some(source) = function.result_borrow {
            let source = function
                .parameters
                .iter()
                .position(|id| *id == source)
                .and_then(|position| parameters.get(position))
                .ok_or_else(|| {
                    reference_type_error("borrowed result selects a foreign parameter")
                })?;
            if !matches!(function.effect, FunctionEffect::Pure)
                || source.use_mode != ParameterUse::Borrow
                || source.resource_requirement.is_some()
                || direct_memory_type(&self.schema, source.ty, &types, self.control)?.is_none()
                || direct_memory_type(&self.schema, function.result, &types, self.control)?
                    .is_none()
            {
                return Err(reference_type_error(
                    "borrowed result requires a pure graph function and exact borrowed memory parameter",
                ));
            }
        }
        self.validate_call_resources(
            &parameters,
            &types,
            &arguments,
            matches!(function.effect, FunctionEffect::Pure),
        )?;
        if types.len() != function.type_parameters.len() {
            return Err(reference_type_error(
                "function type parameters are not unique",
            ));
        }
        let implementations = self.implementation_bindings(
            declaration,
            &function.implementation_parameters,
            &types,
            supplied,
        )?;
        Ok(AdmittedGraphCall {
            implementations,
            declaration,
            function,
            types,
            effects: effect_scope,
            requirements: requirement_scope,
            allowance,
            arguments,
        })
    }

    fn enter_graph_call(
        &mut self,
        target: AdmittedGraphCall,
        tail: bool,
    ) -> Result<ReferenceStep, ExecutionError> {
        self.count_call(target.arguments.len())?;
        let result_borrow = target.function.result_borrow;
        let mut locals = target
            .function
            .parameters
            .into_iter()
            .zip(target.arguments)
            .map(|(parameter, mut value)| {
                let local = LocalValueReference::FunctionParameter(parameter);
                if value.raw().memory_is_borrowed() {
                    value.set_provenance(local);
                }
                (local, value)
            })
            .collect::<BTreeMap<_, _>>();
        self.control.check()?;
        self.active_package = target.declaration.package;
        self.implementation_scopes
            .push((target.declaration, target.implementations));
        self.type_scopes.push(target.types);
        self.effect_scopes.push(target.effects);
        self.requirement_scopes.push(target.requirements);
        self.allowances.push(target.allowance);
        self.observation.maximum_live_effect_bindings = self
            .observation
            .maximum_live_effect_bindings
            .max(self.effect_scopes.iter().map(BTreeMap::len).sum());
        self.observation.maximum_live_allowances = self
            .observation
            .maximum_live_allowances
            .max(self.allowances.len());
        self.local_counts.push(locals.len());
        self.observe_locals(&locals);
        if tail {
            self.observation.tail_transfers = self.observation.tail_transfers.saturating_add(1);
        }
        let lexical_loan_scopes = std::mem::replace(&mut self.lexical_loan_scopes, 0);
        self.borrow_result_sources.push(result_borrow);
        let previous_demand = std::mem::replace(&mut self.borrow_result_demand, false);
        let result = if result_borrow.is_some() {
            self.evaluate_in_context(target.function.body, &mut locals, true)
                .map(ReferenceStep::Value)
        } else {
            self.evaluate_tail(target.function.body, &mut locals)
        };
        let result = result.and_then(|mut step| {
            if let ReferenceStep::Value(value) = &mut step {
                let memory_form = direct_memory_type(
                    &self.schema,
                    target.function.result,
                    self.type_scopes
                        .last()
                        .ok_or_else(|| reference_type_error("missing type scope"))?,
                    self.control,
                )?;
                let expected = memory_form.is_some();
                if (value.ownership(&self.schema, &mut self.observation.value_work)?
                    == Ownership::Memory)
                    != expected
                {
                    return Err(reference_type_error(
                        "reference memory result contract mismatch",
                    ));
                }
                if value.raw().memory_form().is_some() {
                    if value.raw().memory_form().as_ref() != memory_form.as_ref() {
                        return Err(reference_type_error(
                            "memory result representation mismatch",
                        ));
                    }
                    if let Some(source) = result_borrow {
                        let source = LocalValueReference::FunctionParameter(source);
                        if !value.raw().memory_is_borrowed() || value.provenance() != Some(source) {
                            return Err(reference_type_error(
                                "borrowed result came from another exact parameter",
                            ));
                        }
                        value.raw().memory_validate(self.memory_domain, false)?;
                        let parent = locals.get(&source).ok_or_else(|| {
                            reference_type_error(
                                "borrowed result source parameter is no longer live",
                            )
                        })?;
                        self.charge_allocation(CheckedValue::parent_storage_bytes() as u64)?;
                        value.reserve_parent(parent)?;
                        value.retain_parent(|| {
                            locals.remove(&source).ok_or_else(|| {
                                reference_type_error(
                                    "borrowed result source custody was already transferred",
                                )
                            })
                        })?;
                    } else {
                        value.raw().memory_validate(self.memory_domain, true)?;
                    }
                }
            }
            Ok(step)
        });
        self.local_counts.pop();
        self.borrow_result_sources.pop();
        self.borrow_result_demand = previous_demand;
        self.implementation_scopes.pop();
        self.type_scopes.pop();
        self.effect_scopes.pop();
        self.requirement_scopes.pop();
        self.allowances.pop();
        self.lexical_loan_scopes = lexical_loan_scopes;
        result
    }

    fn validate_call_resources(
        &mut self,
        parameters: &[ParameterRecord],
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        arguments: &[CheckedValue],
        pure: bool,
    ) -> Result<(), ExecutionError> {
        let mut resource_seen = false;
        for (index, (parameter, argument)) in parameters.iter().zip(arguments).enumerate() {
            let memory_form =
                direct_memory_type(&self.schema, parameter.ty, substitutions, self.control)?;
            if memory_form.is_some() {
                if resource_seen
                    || (!pure && parameter.use_mode != ParameterUse::Consume)
                    || parameter.resource_requirement.is_some()
                    || parameter.use_mode == ParameterUse::Unrestricted
                    || argument.ownership(&self.schema, &mut self.observation.value_work)?
                        != Ownership::Memory
                {
                    return Err(reference_type_error("invalid memory call signature"));
                }
                if argument.raw().memory_form().as_ref() != memory_form.as_ref() {
                    return Err(reference_type_error("memory call representation mismatch"));
                }
                argument.raw().memory_validate(
                    self.memory_domain,
                    parameter.use_mode == ParameterUse::Consume,
                )?;
                if (parameter.use_mode == ParameterUse::Borrow)
                    != argument.raw().memory_is_borrowed()
                {
                    return Err(reference_type_error("memory argument loan mode mismatch"));
                }
                continue;
            }
            match parameter.resource_requirement {
                Some(requirement) => {
                    resource_seen = true;
                    if parameter.use_mode == ParameterUse::Unrestricted
                        || argument.ownership(&self.schema, &mut self.observation.value_work)?
                            != Ownership::Capability
                    {
                        return Err(reference_error(
                            "normalized_reference_resource_call_shape",
                            "resource-bearing call requires a borrow/consume parameter and direct handle",
                        ));
                    }
                    let NormalizedValue::Resource(handle) = argument.raw() else {
                        return Err(reference_error(
                            "normalized_reference_resource_call_value",
                            "resource-bearing call argument is not one exact runtime handle",
                        ));
                    };
                    for (prior, value) in parameters[..index].iter().zip(&arguments[..index]) {
                        if let NormalizedValue::Resource(other) = value.raw()
                            && handle.borrow() == other.borrow()
                            && (parameter.use_mode == ParameterUse::Consume
                                || prior.use_mode == ParameterUse::Consume)
                        {
                            return Err(reference_error(
                                "normalized_reference_resource_call_alias",
                                "a consuming resource argument aliases another argument in the same call",
                            ));
                        }
                    }
                    let Some(OwnerRecord::Requirement(record)) = self.owner_in_package(
                        requirement.package,
                        OwnerKey::Requirement(requirement.requirement),
                    )?
                    else {
                        return Err(reference_error(
                            "normalized_reference_resource_call_requirement",
                            "resource parameter requirement is absent from canonical authority",
                        ));
                    };
                    match parameter.use_mode {
                        ParameterUse::Borrow => {
                            if !handle.is_borrowed() {
                                return Err(reference_error(
                                    "normalized_reference_resource_call_borrow",
                                    "reference borrow argument still owns its handle",
                                ));
                            }
                            self.resources.validate_queue_lease_borrow(
                                requirement,
                                record.interface,
                                *handle,
                            )?;
                        }
                        _ => self.resources.validate_queue_lease_transfer(
                            requirement,
                            record.interface,
                            *handle,
                        )?,
                    }
                }
                None => {
                    if resource_seen
                        || parameter.use_mode != ParameterUse::Unrestricted
                        || argument.ownership(&self.schema, &mut self.observation.value_work)?
                            != Ownership::Ordinary
                    {
                        return Err(reference_error(
                            "normalized_reference_resource_call_parameter",
                            "ordinary function parameter use or value contains unbound affine authority",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Tail context is derived from canonical syntax, independently of compiler control flow.
    /// Lexical maps belong to this activation and are dropped when this method returns a tail
    /// step. Non-tail children still use ordinary evaluation and retain their pending work.
    fn evaluate_tail(
        &mut self,
        mut expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<ReferenceStep, ExecutionError> {
        self.control_frames += 1;
        self.observation.maximum_control_frames = self
            .observation
            .maximum_control_frames
            .max(self.control_frames);
        let result = (|| loop {
            self.observe_locals(locals);
            match self.read_expression(expression)? {
                ExpressionOperation::If {
                    condition,
                    when_true,
                    when_false,
                } => {
                    expression = match self.evaluate(condition, locals)?.release() {
                        NormalizedValue::Bool(true) => when_true,
                        NormalizedValue::Bool(false) => when_false,
                        _ => return Err(reference_type_error("if condition is not boolean")),
                    };
                }
                ExpressionOperation::Let { bindings, body } => {
                    for binding in bindings {
                        let record = self.binding(binding, BindingKind::Let)?;
                        let value = record.value.ok_or_else(|| {
                            reference_error(
                                "normalized_reference_let_value",
                                "canonical let binding has no value expression",
                            )
                        })?;
                        let value = self.evaluate(value, locals)?;
                        if locals
                            .insert(LocalValueReference::LexicalBinding(binding), value)
                            .is_some()
                        {
                            return Err(reference_error(
                                "normalized_reference_local_duplicate",
                                "canonical local identity was bound twice in one scope",
                            ));
                        }
                    }
                    expression = body;
                }
                ExpressionOperation::Sequence { items } => {
                    let (last, preceding) = items.split_last().ok_or_else(|| {
                        reference_error(
                            "normalized_reference_sequence_empty",
                            "canonical sequence has no result expression",
                        )
                    })?;
                    for item in preceding {
                        self.evaluate(*item, locals)?;
                    }
                    expression = *last;
                }
                ExpressionOperation::Match { value, arms } => {
                    let (layout, case, payload) = self
                        .evaluate_match_value(value, locals)?
                        .open_variant(&self.schema)?;
                    let mut selected = None;
                    for arm in arms {
                        if self.matches_case(layout, case, arm.case)? {
                            selected = Some(arm);
                            break;
                        }
                    }
                    let arm = selected.ok_or_else(|| {
                        reference_error(
                            "normalized_reference_match_case",
                            "verified exhaustive match omitted the runtime case tag",
                        )
                    })?;
                    match (arm.payload_binding, payload) {
                        (Some(binding), Some(payload)) => {
                            self.binding(binding, BindingKind::MatchPayload)?;
                            if locals
                                .insert(LocalValueReference::MatchPayload(binding), payload)
                                .is_some()
                            {
                                return Err(reference_error(
                                    "normalized_reference_local_duplicate",
                                    "match payload identity was already bound",
                                ));
                            }
                        }
                        (None, None) => {}
                        _ => {
                            return Err(reference_error(
                                "normalized_reference_match_payload",
                                "runtime variant payload disagrees with its exact match arm",
                            ));
                        }
                    }
                    expression = arm.body;
                }
                ExpressionOperation::MatchOwned {
                    choice_type,
                    source,
                    arms,
                } => {
                    let (body, _) = self.bind_owned_choice(choice_type, source, &arms, locals)?;
                    expression = body;
                }
                ExpressionOperation::UnpackOwned {
                    product_type,
                    source,
                    fields,
                    body,
                } => {
                    self.bind_owned_product(product_type, source, &fields, locals)?;
                    expression = body;
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    implementations,
                    arguments,
                } => {
                    self.require_owning_result(function)?;
                    let uses = self.function_parameter_uses(function)?;
                    let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                    let target = self.witness_call(
                        function,
                        &type_arguments,
                        &effect_arguments,
                        &requirement_arguments,
                        &implementations,
                        arguments,
                    )?;
                    return self.transfer_graph_call(target, locals);
                }
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments,
                } => {
                    let (function, types, implementations) =
                        self.method_target(witness, contract, method)?;
                    self.require_owning_result(function)?;
                    let uses = self.function_parameter_uses(function)?;
                    let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                    let target = self.admit_resolved_witness_call(
                        function,
                        &types,
                        &[],
                        &[],
                        &implementations,
                        arguments,
                    )?;
                    return self.transfer_graph_call(target, locals);
                }
                ExpressionOperation::Call {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                } => {
                    let uses = self.function_parameter_uses(function)?;
                    let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                    return self.tail_step(
                        function,
                        &type_arguments,
                        &effect_arguments,
                        &requirement_arguments,
                        arguments,
                        locals,
                    );
                }
                ExpressionOperation::Invoke { callee, arguments } => {
                    let callee = self.evaluate(callee, locals)?;
                    let arguments = self.evaluate_many(&arguments, locals)?;
                    let (
                        declaration,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        arguments,
                    ) = self.callable_arguments(callee, arguments)?;
                    return self.tail_step(
                        declaration,
                        &type_arguments,
                        &effect_arguments,
                        &requirement_arguments,
                        arguments,
                        locals,
                    );
                }
                operation => {
                    return self
                        .evaluate_operation(operation, locals)
                        .map(ReferenceStep::Value);
                }
            }
        })();
        self.control_frames -= 1;
        result
    }

    fn tail_step(
        &mut self,
        declaration: DeclarationReference,
        types: &[TypeObjectDigest],
        effect_arguments: &[EffectRow],
        requirement_arguments: &[RequirementOperand],
        arguments: Vec<CheckedValue>,
        locals: &BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<ReferenceStep, ExecutionError> {
        self.require_owning_result(declaration)?;
        let callable = self.declaration(declaration)?;
        if let DeclarationPayload::Function(function) = callable.payload {
            let types = self.resolve_type_arguments(types)?;
            let effects = self.resolve_effect_arguments(effect_arguments)?;
            let requirements = self.resolve_requirement_arguments(requirement_arguments)?;
            let target = self.admit_graph_call(
                declaration,
                function,
                &types,
                &effects,
                &requirements,
                arguments,
            )?;
            self.transfer_graph_call(target, locals)
        } else {
            self.call_declaration(
                declaration,
                types,
                effect_arguments,
                requirement_arguments,
                arguments,
            )
            .map(ReferenceStep::Value)
        }
    }

    fn transfer_graph_call(
        &mut self,
        target: AdmittedGraphCall,
        locals: &BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<ReferenceStep, ExecutionError> {
        self.charge_allocation(std::mem::size_of::<AdmittedGraphCall>() as u64)?;
        self.control.check()?;
        if self.lexical_loan_scopes != 0
            || target.function.result_borrow.is_some()
            || self
                .borrow_result_sources
                .last()
                .is_some_and(Option::is_some)
            || locals.values().any(|v| v.raw().memory_owns_live_loans())
        {
            if self.call_depth.saturating_add(self.ancestor_depth) >= self.policy.maximum_call_depth
            {
                return Err(reference_resource(
                    "normalized_reference_call_depth",
                    "retained memory loan scope exceeded call-depth limit",
                ));
            }
            self.call_depth += 1;
            self.observation.maximum_call_depth = self
                .observation
                .maximum_call_depth
                .max(self.call_depth.saturating_add(self.ancestor_depth));
            let package = self.active_package;
            let mut step = self.enter_graph_call(target, false);
            let result = loop {
                match step {
                    Ok(ReferenceStep::Value(value)) => break Ok(ReferenceStep::Value(value)),
                    Ok(ReferenceStep::Tail(target)) => {
                        step = self.enter_graph_call(*target, true);
                    }
                    Err(error) => break Err(error),
                }
            };
            self.active_package = package;
            self.call_depth -= 1;
            result
        } else {
            Ok(ReferenceStep::Tail(Box::new(target)))
        }
    }

    fn evaluate(
        &mut self,
        expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.evaluate_in_context(expression, locals, false)
    }

    fn evaluate_in_context(
        &mut self,
        expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
        demand: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        let previous = std::mem::replace(&mut self.borrow_result_demand, demand);
        self.control_frames += 1;
        self.observation.maximum_control_frames = self
            .observation
            .maximum_control_frames
            .max(self.control_frames);
        self.observe_locals(locals);
        let result = self
            .read_expression(expression)
            .and_then(|operation| self.evaluate_operation(operation, locals));
        self.observe_locals(locals);
        self.control_frames -= 1;
        self.borrow_result_demand = previous;
        result
    }

    fn observe_locals(&mut self, locals: &BTreeMap<LocalValueReference, CheckedValue>) {
        if let Some(count) = self.local_counts.last_mut() {
            *count = locals.len();
        }
        self.observation.maximum_live_locals = self
            .observation
            .maximum_live_locals
            .max(self.local_counts.iter().sum());
        self.observation.maximum_live_type_bindings = self
            .observation
            .maximum_live_type_bindings
            .max(self.type_scopes.iter().map(BTreeMap::len).sum());
    }

    fn read_expression(
        &mut self,
        expression: ExpressionId,
    ) -> Result<ExpressionOperation, ExecutionError> {
        self.control.check()?;
        if self.remaining_expressions == Some(0) {
            return Err(reference_resource(
                "normalized_reference_expression_steps",
                "reference execution exhausted its expression-step budget",
            ));
        }
        if let Some(remaining) = &mut self.remaining_expressions {
            *remaining -= 1;
        }
        if let Some(budget) = &self.shared_budget {
            budget.step("normalized_reference_expression_steps")?;
        }
        self.observation.expressions = self.observation.expressions.saturating_add(1);
        match self.owner(OwnerKey::Expression(expression))? {
            Some(OwnerRecord::Expression(record)) => Ok(record.operation),
            Some(_) => Err(reference_error(
                "normalized_reference_expression_kind",
                "exact expression identity names another owner kind",
            )),
            None => Err(reference_error(
                "normalized_reference_expression_missing",
                "exact expression is missing from canonical authority",
            )),
        }
    }

    fn evaluate_operation(
        &mut self,
        operation: ExpressionOperation,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let demand = self.borrow_result_demand;
        self.charge_allocation(
            (std::mem::size_of::<CheckedValue>() - std::mem::size_of::<NormalizedValue>()) as u64,
        )?;
        match operation {
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => self.evaluate_borrow_call(call, binding, body, locals),
            ExpressionOperation::Parallel { left, right } => self.parallel(left, right, locals),
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                let (ty, _) = self.sequence_item_type(sequence_type)?;
                let control = self.control;
                let token = super::owned_sequence::OwnedSequence::create(
                    self.memory_domain,
                    ty,
                    control,
                    &mut |bytes| self.charge_allocation(bytes),
                )?;
                CheckedValue::memory(&self.schema, NormalizedValue::OwnedSequence(token))
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                let (ty, _) = self.sequence_item_type(sequence_type)?;
                let parent = self.borrow_parent(source, locals)?;
                let NormalizedValue::OwnedSequence(token) = parent.raw() else {
                    return Err(reference_type_error(
                        "sequence length requires a sequence token",
                    ));
                };
                if token.ty() != ty {
                    return Err(reference_type_error("sequence length source type mismatch"));
                }
                let length = token.len(self.memory_domain, self.control)?;
                CheckedValue::primitive(
                    &self.schema,
                    NormalizedValue::I64(reference_length(length)?),
                )
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let (ty, item) = self.sequence_item_type(sequence_type)?;
                let child = self.consume_sequence_local(value, locals)?.release();
                let child = self.product_child(child, item)?.release();
                let NormalizedValue::OwnedSequence(token) =
                    self.consume_sequence_local(source, locals)?.release()
                else {
                    return Err(reference_type_error(
                        "sequence push requires a sequence token",
                    ));
                };
                if token.ty() != ty {
                    return Err(reference_type_error("sequence push source type mismatch"));
                }
                let control = self.control;
                let token = token.push(self.memory_domain, child, control, &mut |bytes| {
                    self.charge_allocation(bytes)
                })?;
                CheckedValue::memory(&self.schema, NormalizedValue::OwnedSequence(token))
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                let (ty, item) = self.sequence_item_type(sequence_type)?;
                let (result, product) = self.sequence_pop_types(ty, item, result_type)?;
                let NormalizedValue::OwnedSequence(token) =
                    self.consume_sequence_local(source, locals)?.release()
                else {
                    return Err(reference_type_error(
                        "sequence pop requires a sequence token",
                    ));
                };
                if token.ty() != ty {
                    return Err(reference_type_error("sequence pop source type mismatch"));
                }
                let control = self.control;
                let result =
                    token.pop(self.memory_domain, result, product, control, &mut |bytes| {
                        self.charge_allocation(bytes)
                    })?;
                CheckedValue::memory(&self.schema, result)
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let (ty, item) = self.sequence_item_type(sequence_type)?;
                let record = self.binding(binding, BindingKind::OwnedBorrow)?;
                let declared = record.declared_type.ok_or_else(|| {
                    reference_type_error("borrowed sequence item binding lacks type")
                })?;
                if self.resolve_type_arguments(&[declared])?[0] != item {
                    return Err(reference_type_error("borrowed sequence item type mismatch"));
                }
                self.charge_allocation(
                    (std::mem::size_of::<LocalValueReference>()
                        + std::mem::size_of::<CheckedValue>()) as u64,
                )?;
                let NormalizedValue::I64(index) = self.evaluate(index, locals)?.release() else {
                    return Err(reference_type_error("sequence index requires I64"));
                };
                let parent = self.borrow_parent(source, locals)?;
                let NormalizedValue::OwnedSequence(token) = parent.raw() else {
                    return Err(reference_type_error(
                        "borrow source requires a sequence token",
                    ));
                };
                if token.ty() != ty {
                    return Err(reference_type_error("borrow source sequence type mismatch"));
                }
                let control = self.control;
                let child =
                    token.borrow_item(self.memory_domain, index, control, &mut |bytes| {
                        self.charge_allocation(bytes)
                    })?;
                let child = self.borrowed_product_child(child, item)?;
                self.evaluate_borrowed_body(parent, binding, child, body, locals)
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                let ty = self.resolve_type_arguments(&[product_type])?[0];
                let Some(TypeForm::OwnedProduct { fields }) =
                    self.schema.types.get(&ty).map(|object| &object.form)
                else {
                    return Err(reference_type_error(
                        "borrow requires a closed owned product",
                    ));
                };
                let (index, selected) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, candidate)| candidate.name == field)
                    .ok_or_else(|| reference_type_error("unknown borrowed product field"))?;
                let payload_type = selected.ty;
                if direct_memory_type(&self.schema, payload_type, &BTreeMap::new(), self.control)?
                    .is_none()
                {
                    return Err(reference_type_error("borrowed product field must be owned"));
                }
                let record = self.binding(binding, BindingKind::OwnedBorrow)?;
                let declared = record
                    .declared_type
                    .ok_or_else(|| reference_type_error("borrowed child binding lacks type"))?;
                if self.resolve_type_arguments(&[declared])?[0] != payload_type {
                    return Err(reference_type_error("borrowed product field type mismatch"));
                }
                self.charge_allocation(
                    (std::mem::size_of::<LocalValueReference>()
                        + std::mem::size_of::<CheckedValue>()) as u64,
                )?;
                self.control.check()?;
                let parent = self.borrow_parent(source, locals)?;
                let NormalizedValue::OwnedProduct(token) = parent.raw() else {
                    return Err(reference_type_error(
                        "borrow source requires a product token",
                    ));
                };
                if token.ty() != ty {
                    return Err(reference_type_error("borrow source product type mismatch"));
                }
                token.validate(self.memory_domain, false)?;
                let control = self.control;
                let value =
                    token.borrow_field(self.memory_domain, index, control, &mut |bytes| {
                        self.charge_allocation(bytes)
                    })?;
                let value = self.borrowed_product_child(value, payload_type)?;
                self.evaluate_borrowed_body(parent, binding, value, body, locals)
            }
            ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let ty = self.resolve_type_arguments(&[choice_type])?[0];
                let Some(TypeForm::OwnedChoice { cases }) =
                    self.schema.types.get(&ty).map(|object| &object.form)
                else {
                    return Err(reference_type_error(
                        "borrowed match requires an owned choice",
                    ));
                };
                if cases.len() != arms.len()
                    || cases
                        .iter()
                        .zip(&arms)
                        .any(|(case, arm)| case.name != arm.name)
                {
                    return Err(reference_type_error(
                        "borrowed match case coverage mismatch",
                    ));
                }
                for (index, arm) in arms.iter().enumerate() {
                    let TypeForm::OwnedChoice { cases } = &self.schema.types[&ty].form else {
                        return Err(reference_type_error("missing borrowed choice shape"));
                    };
                    let payload_type = cases[index].ty;
                    let record = self.binding(arm.binding, BindingKind::OwnedBorrow)?;
                    let declared = record.declared_type.ok_or_else(|| {
                        reference_type_error("borrowed choice binding lacks type")
                    })?;
                    if self.resolve_type_arguments(&[declared])?[0] != payload_type {
                        return Err(reference_type_error(
                            "borrowed choice payload type mismatch",
                        ));
                    }
                }
                self.charge_allocation(
                    (std::mem::size_of::<LocalValueReference>()
                        + std::mem::size_of::<CheckedValue>()) as u64,
                )?;
                self.control.check()?;
                let parent = self.borrow_parent(source, locals)?;
                let NormalizedValue::OwnedChoice(token) = parent.raw() else {
                    return Err(reference_type_error(
                        "borrowed match requires a choice token",
                    ));
                };
                if token.ty() != ty {
                    return Err(reference_type_error("borrowed choice source type mismatch"));
                }
                token.validate(self.memory_domain, false)?;
                let selected = token.case() as usize;
                let arm = arms
                    .get(selected)
                    .ok_or_else(|| reference_type_error("invalid borrowed choice arm"))?;
                let TypeForm::OwnedChoice { cases } = &self.schema.types[&ty].form else {
                    return Err(reference_type_error("missing borrowed choice shape"));
                };
                let payload_type = cases[selected].ty;
                let control = self.control;
                let value = token.borrow_payload(self.memory_domain, control, &mut |bytes| {
                    self.charge_allocation(bytes)
                })?;
                let value = self.borrowed_product_child(value, payload_type)?;
                self.evaluate_borrowed_body(parent, arm.binding, value, arm.body, locals)
            }
            ExpressionOperation::ChooseOwned {
                choice_type,
                case,
                value,
            } => {
                let ty = self.resolve_type_arguments(&[choice_type])?[0];
                let Some(TypeForm::OwnedChoice { cases }) =
                    self.schema.types.get(&ty).map(|object| &object.form)
                else {
                    return Err(reference_type_error(
                        "choice requires a closed owned choice type",
                    ));
                };
                let (index, selected) = cases
                    .iter()
                    .enumerate()
                    .find(|(_, candidate)| candidate.name == case)
                    .ok_or_else(|| reference_type_error("unknown owned choice case"))?;
                let payload_type = selected.ty;
                let raw = self.evaluate(value, locals)?.release();
                let raw = self.product_child(raw, payload_type)?.release();
                let control = self.control;
                let token = super::owned_choice::OwnedChoice::create(
                    self.memory_domain,
                    ty,
                    index as u32,
                    raw,
                    control,
                    &mut |bytes| self.charge_allocation(bytes),
                )?;
                CheckedValue::memory(&self.schema, NormalizedValue::OwnedChoice(token))
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => {
                let (body, local) = self.bind_owned_choice(choice_type, source, &arms, locals)?;
                let result = self.evaluate_in_context(body, locals, demand);
                locals.remove(&local);
                result
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                let ty = self.resolve_type_arguments(&[product_type])?[0];
                let Some(TypeForm::OwnedProduct { fields: expected }) =
                    self.schema.types.get(&ty).map(|o| &o.form)
                else {
                    return Err(reference_type_error("pack requires a closed product type"));
                };
                if fields.len() != expected.len() {
                    return Err(reference_type_error("incomplete product construction"));
                }
                self.charge_allocation(super::value::collection_storage_bytes(
                    fields.len() as u64,
                    std::mem::size_of::<(usize, NormalizedValue)>() as u64,
                    "normalized_product_allocation",
                )?)?;
                self.control.check()?;
                let mut children = Vec::with_capacity(fields.len());
                for field in fields {
                    let TypeForm::OwnedProduct { fields: expected } = &self.schema.types[&ty].form
                    else {
                        return Err(reference_type_error("missing product shape"));
                    };
                    let (index, expected) = expected
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.name == field.name)
                        .ok_or_else(|| reference_type_error("unknown product field"))?;
                    let expected = expected.ty;
                    let value = self.evaluate(field.value, locals)?.release();
                    children.push((index, self.product_child(value, expected)?.release()));
                }
                children.sort_unstable_by_key(|(i, _)| *i);
                if children.iter().enumerate().any(|(i, (j, _))| i != *j) {
                    return Err(reference_type_error("duplicate product field"));
                }
                self.charge_allocation(super::value::collection_storage_bytes(
                    children.len() as u64,
                    std::mem::size_of::<NormalizedValue>() as u64,
                    "normalized_product_allocation",
                )?)?;
                self.control.check()?;
                let fields = children.into_iter().map(|(_, value)| value).collect();
                let control = self.control;
                let token = super::owned_product::OwnedProduct::create(
                    self.memory_domain,
                    ty,
                    fields,
                    control,
                    &mut |n| self.charge_allocation(n),
                )?;
                CheckedValue::memory(&self.schema, NormalizedValue::OwnedProduct(token))
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                self.bind_owned_product(product_type, source, &fields, locals)?;
                let result = self.evaluate_in_context(body, locals, demand);
                for field in fields {
                    locals.remove(&LocalValueReference::LexicalBinding(field.binding));
                }
                result
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                implementations,
                arguments,
            } => {
                self.require_owning_result(function)?;
                let uses = self.function_parameter_uses(function)?;
                let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                let target = self.witness_call(
                    function,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    &implementations,
                    arguments,
                )?;
                self.execute_witness_call(target)
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let (function, types, implementations) =
                    self.method_target(witness, contract, method)?;
                self.require_owning_result(function)?;
                let uses = self.function_parameter_uses(function)?;
                let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                let target = self.admit_resolved_witness_call(
                    function,
                    &types,
                    &[],
                    &[],
                    &implementations,
                    arguments,
                )?;
                self.execute_witness_call(target)
            }
            ExpressionOperation::Unit {} => {
                CheckedValue::primitive(&self.schema, NormalizedValue::Unit)
            }
            ExpressionOperation::Bool { value } => {
                CheckedValue::primitive(&self.schema, NormalizedValue::Bool(value))
            }
            ExpressionOperation::I64 { value } => {
                CheckedValue::primitive(&self.schema, NormalizedValue::I64(value))
            }
            ExpressionOperation::F64 { value } => {
                CheckedValue::primitive(&self.schema, NormalizedValue::F64(value))
            }
            ExpressionOperation::Text { value } => self.text(value).and_then(|value| {
                CheckedValue::primitive(&self.schema, NormalizedValue::Text(value))
            }),
            ExpressionOperation::StaticText { value } => self.text(value).and_then(|value| {
                CheckedValue::primitive(&self.schema, NormalizedValue::StaticText(value))
            }),
            ExpressionOperation::Local { value } => {
                if locals.get(&value).is_some_and(|v| {
                    matches!(
                        v.raw(),
                        NormalizedValue::ByteBuffer(_)
                            | NormalizedValue::OwnedI64Cell(_)
                            | NormalizedValue::OwnedProduct(_)
                            | NormalizedValue::OwnedChoice(_)
                            | NormalizedValue::OwnedSequence(_)
                    )
                }) {
                    if locals[&value].raw().memory_is_borrowed() {
                        let source = self
                            .borrow_result_sources
                            .last()
                            .copied()
                            .flatten()
                            .map(LocalValueReference::FunctionParameter);
                        if !demand || source.is_none() || locals[&value].provenance() != source {
                            return Err(reference_type_error(
                                "borrowed local is outside its matching result context",
                            ));
                        }
                        let result = locals[&value].duplicate(ParameterUse::Borrow)?;
                        result.raw().memory_validate(self.memory_domain, false)?;
                        return Ok(result);
                    }
                    locals[&value]
                        .raw()
                        .memory_validate(self.memory_domain, true)?;
                    let result = locals
                        .remove(&value)
                        .ok_or_else(|| reference_type_error("missing memory owner"))?;
                    result.raw().memory_validate(self.memory_domain, true)?;
                    return Ok(result);
                }
                let value = locals.get(&value).ok_or_else(|| {
                    reference_error(
                        "normalized_reference_local_missing",
                        "canonical local reference escaped its exact lexical scope",
                    )
                })?;
                if !matches!(
                    value.ownership(&self.schema, &mut self.observation.value_work)?,
                    Ownership::Ordinary
                ) {
                    return Err(reference_error(
                        "normalized_reference_local_resource_use",
                        "affine local requires its exact ownership transfer",
                    ));
                }
                value.duplicate(ParameterUse::Unrestricted)
            }
            ExpressionOperation::Constant { declaration } => {
                self.call_declaration(declaration, &[], &[], &[], Vec::new())
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => match self.evaluate(condition, locals)?.release() {
                NormalizedValue::Bool(true) => self.evaluate_in_context(when_true, locals, demand),
                NormalizedValue::Bool(false) => {
                    self.evaluate_in_context(when_false, locals, demand)
                }
                _ => Err(reference_type_error("if condition is not boolean")),
            },
            ExpressionOperation::Let { bindings, body } => {
                let mut scoped = Vec::with_capacity(bindings.len());
                for binding in bindings {
                    let record = self.binding(binding, BindingKind::Let)?;
                    let value = record.value.ok_or_else(|| {
                        reference_error(
                            "normalized_reference_let_value",
                            "canonical let binding has no value expression",
                        )
                    })?;
                    let value = self.evaluate(value, locals)?;
                    let local = LocalValueReference::LexicalBinding(binding);
                    if locals.insert(local, value).is_some() {
                        return Err(reference_error(
                            "normalized_reference_local_duplicate",
                            "canonical local identity was bound twice in one scope",
                        ));
                    }
                    scoped.push(local);
                }
                let result = self.evaluate_in_context(body, locals, demand);
                for local in scoped {
                    locals.remove(&local);
                }
                result
            }
            ExpressionOperation::Sequence { items } => {
                let (last, preceding) = items.split_last().ok_or_else(|| {
                    reference_error(
                        "normalized_reference_sequence_empty",
                        "canonical sequence has no result expression",
                    )
                })?;
                // Discard each preceding result before its successor starts, including
                // owners whose storage must not survive through the following call.
                for item in preceding {
                    self.evaluate(*item, locals)?;
                }
                self.evaluate_in_context(*last, locals, demand)
            }
            ExpressionOperation::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => {
                self.require_owning_result(function)?;
                let uses = self.function_parameter_uses(function)?;
                let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                self.call_declaration(
                    function,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    arguments,
                )
            }
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => {
                let signature = self.function_signature(function)?;
                if signature.has_implementations || signature.result_borrow.is_some() {
                    return Err(reference_type_error(
                        "static witness templates cannot become callable values",
                    ));
                }
                if signature
                    .parameters
                    .iter()
                    .any(|parameter| parameter.resource_requirement.is_some())
                {
                    return Err(reference_error(
                        "normalized_reference_resource_function_value",
                        "resource-bearing task functions can be used only by direct named call",
                    ));
                }
                let type_arguments = self.resolve_type_arguments(&type_arguments)?.into();
                let effect_arguments = self.resolve_effect_arguments(&effect_arguments)?.into();
                let requirement_arguments =
                    self.resolve_requirement_arguments(&requirement_arguments)?;
                self.requirement_bindings(
                    function,
                    &signature.requirement_parameters,
                    &requirement_arguments,
                )?;
                let requirement_arguments = requirement_arguments.into();
                self.schema
                    .functions
                    .binary_search(&function)
                    .ok()
                    .and_then(|index| u32::try_from(index).ok())
                    .map(|index| FunctionIndex(index, self.schema.value_origin))
                    .ok_or_else(|| {
                        reference_error(
                            "normalized_reference_function_value",
                            "exact function value is absent from the canonical callable inventory",
                        )
                    })
                    .and_then(|function| {
                        CheckedValue::callable(
                            &self.schema,
                            function,
                            type_arguments,
                            effect_arguments,
                            requirement_arguments,
                            &signature,
                        )
                    })
            }
            ExpressionOperation::Bind { callee, arguments } => {
                let callee = self.evaluate(callee, locals)?;
                self.bind_expression(callee, arguments, locals)
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                let callee = self.evaluate(callee, locals)?;
                let arguments = self.evaluate_many(&arguments, locals)?;
                let (
                    declaration,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    arguments,
                ) = self.callable_arguments(callee, arguments)?;
                self.call_declaration(
                    declaration,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    arguments,
                )
            }
            ExpressionOperation::Record {
                nominal_type,
                fields,
                type_arguments,
            } => {
                let mut values = Vec::with_capacity(fields.len());
                for field in &fields {
                    values.push(self.evaluate(field.value, locals)?);
                }
                let type_arguments = self.resolve_type_arguments(&type_arguments)?;
                self.record(
                    nominal_type,
                    &type_arguments,
                    fields.into_iter().map(|field| field.selector),
                    values,
                )
            }
            ExpressionOperation::Variant {
                case,
                payload,
                type_arguments,
            } => {
                let (template, tag) = self.case_layout(case)?;
                let arguments = self.resolve_type_arguments(&type_arguments)?;
                let identity = super::value_schema::nominal_identity(
                    self.schema.variants[template.0 as usize].declaration,
                    &arguments,
                )
                .map_err(|_| reference_type_error("invalid nominal application"))?;
                if !arguments.is_empty() && !self.schema.ordinary_types.contains(&identity) {
                    return Err(reference_type_error(
                        "variant application contains live authority",
                    ));
                }
                let index = self
                    .schema
                    .variant_index(identity)
                    .and_then(|index| u32::try_from(index).ok())
                    .ok_or_else(|| {
                        reference_type_error("variant application has no canonical layout")
                    })?;
                let layout = VariantLayoutIndex(index, self.schema.value_origin);
                let payload = payload
                    .map(|payload| {
                        self.evaluate_with_use(
                            payload,
                            locals,
                            if self.case_payload_is_direct_resource(layout, tag) {
                                ParameterUse::Consume
                            } else {
                                ParameterUse::Unrestricted
                            },
                        )
                    })
                    .transpose()?;
                if payload.is_some() {
                    self.charge_items(1, std::mem::size_of::<NormalizedValue>())?;
                }
                self.variant_value(layout, tag, payload)
            }
            ExpressionOperation::Field { value, selector } => {
                let product_local = match self.owner(OwnerKey::Expression(value))? {
                    Some(OwnerRecord::Expression(record)) => match record.operation {
                        ExpressionOperation::Local { value: local } => {
                            locals.get(&local).is_some_and(|value| {
                                matches!(value.raw(), NormalizedValue::OwnedProduct(_))
                            })
                        }
                        _ => false,
                    },
                    _ => false,
                };
                let value = if product_local {
                    self.evaluate_with_use(value, locals, ParameterUse::Borrow)?
                } else {
                    self.evaluate(value, locals)?
                };
                self.field(value, selector)
            }
            ExpressionOperation::List { items, .. } => {
                let values = self.evaluate_many(&items, locals)?;
                self.charge_items(values.len(), std::mem::size_of::<NormalizedValue>())?;
                self.list_value(values)
            }
            ExpressionOperation::Map { entries, .. } => {
                let mut values = self.empty_map_value();
                for entry in entries {
                    let key = self.evaluate(entry.key, locals)?;
                    let key = self.map_key_value(key)?;
                    let value = self.evaluate(entry.value, locals)?;
                    let NormalizedValue::Map(current) = values.raw() else {
                        return Err(reference_type_error("map construction lost its carrier"));
                    };
                    if current.contains_key(&key) {
                        return Err(reference_trap(
                            "normalized_reference_map_duplicate_key",
                            "map expression contains a duplicate key",
                        ));
                    }
                    values = self.update_map_value(values, key, Some(value))?;
                }
                self.control.check()?;
                Ok(values)
            }
            ExpressionOperation::Match { value, arms } => {
                let (layout, case, payload) = self
                    .evaluate_match_value(value, locals)?
                    .open_variant(&self.schema)?;
                let mut selected = None;
                for arm in arms {
                    if self.matches_case(layout, case, arm.case)? {
                        selected = Some(arm);
                        break;
                    }
                }
                let arm = selected.ok_or_else(|| {
                    reference_error(
                        "normalized_reference_match_case",
                        "verified exhaustive match omitted the runtime case tag",
                    )
                })?;
                let bound = match (arm.payload_binding, payload) {
                    (Some(binding), Some(payload)) => {
                        self.binding(binding, BindingKind::MatchPayload)?;
                        let local = LocalValueReference::MatchPayload(binding);
                        if locals.insert(local, payload).is_some() {
                            return Err(reference_error(
                                "normalized_reference_local_duplicate",
                                "match payload identity was already bound",
                            ));
                        }
                        Some(local)
                    }
                    (None, None) => None,
                    _ => {
                        return Err(reference_error(
                            "normalized_reference_match_payload",
                            "runtime variant payload disagrees with its exact match arm",
                        ));
                    }
                };
                let result = self.evaluate_in_context(arm.body, locals, demand);
                if let Some(bound) = bound {
                    locals.remove(&bound);
                }
                result
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                let uses = self.operation_parameter_uses(operation)?;
                let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                self.capability_call(self.resolve_requirement(requirement)?, operation, arguments)
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => self.transaction(
                self.resolve_requirement(requirement)?,
                binding,
                body,
                locals,
                None,
            ),
            ExpressionOperation::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            } => self.transaction(
                self.resolve_requirement(requirement)?,
                binding,
                body,
                locals,
                Some((outcome, type_argument)),
            ),
        }
    }

    fn evaluate_many(
        &mut self,
        expressions: &[ExpressionId],
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<Vec<CheckedValue>, ExecutionError> {
        expressions
            .iter()
            .map(|expression| self.evaluate(*expression, locals))
            .collect()
    }

    fn evaluate_many_with_uses(
        &mut self,
        expressions: &[ExpressionId],
        uses: &[ParameterUse],
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<Vec<CheckedValue>, ExecutionError> {
        if expressions.len() != uses.len() {
            return Err(reference_type_error(
                "call arguments disagree with their exact parameter uses",
            ));
        }
        let mut values = Vec::with_capacity(expressions.len());
        for (expression, use_mode) in expressions.iter().zip(uses) {
            values.push(self.evaluate_with_use(*expression, locals, *use_mode)?);
        }
        Ok(values)
    }

    fn evaluate_with_use(
        &mut self,
        expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
        use_mode: ParameterUse,
    ) -> Result<CheckedValue, ExecutionError> {
        let local = match self.owner(OwnerKey::Expression(expression))? {
            Some(OwnerRecord::Expression(record)) => match record.operation {
                ExpressionOperation::Local { value } => Some(value),
                _ => None,
            },
            Some(_) => {
                return Err(reference_error(
                    "normalized_reference_expression_kind",
                    "exact expression identity names another owner kind",
                ));
            }
            None => {
                return Err(reference_error(
                    "normalized_reference_expression_missing",
                    "exact expression is missing from canonical authority",
                ));
            }
        };
        let mut value = if let Some(local) = local {
            match use_mode {
                ParameterUse::Consume => {
                    // Invalid consuming syntax must leave its source in custody
                    // until all lexical child scopes release their loans.
                    if let Some(value) = locals.get(&local)
                        && value.raw().memory_form().is_some()
                    {
                        value.raw().memory_validate(self.memory_domain, true)?;
                    }
                    locals.remove(&local)
                }
                ParameterUse::Unrestricted | ParameterUse::Borrow => locals
                    .get(&local)
                    .map(|value| value.duplicate(use_mode))
                    .transpose()?,
            }
            .ok_or_else(|| {
                reference_error(
                    "normalized_reference_local_missing",
                    "canonical local reference is absent, consumed, or outside its exact scope",
                )
            })?
        } else {
            self.evaluate(expression, locals)?
        };
        let ownership = value.ownership(&self.schema, &mut self.observation.value_work)?;
        let valid = match use_mode {
            ParameterUse::Unrestricted => {
                matches!(ownership, Ownership::Ordinary)
            }
            ParameterUse::Borrow => matches!(ownership, Ownership::Capability | Ownership::Memory),
            ParameterUse::Consume => ownership != Ownership::Ordinary,
        };
        if !valid {
            return Err(reference_error(
                "normalized_reference_local_resource_use",
                "canonical parameter use disagrees with its runtime affine value",
            ));
        }
        if value.raw().memory_form().is_some() {
            if use_mode == ParameterUse::Borrow
                && value.provenance().is_none()
                && let Some(local) = local
            {
                value.set_provenance(local);
            }
            value
                .raw()
                .memory_validate(self.memory_domain, use_mode == ParameterUse::Consume)?;
        }
        if let NormalizedValue::Resource(handle) = value.raw() {
            if use_mode == ParameterUse::Consume {
                handle.require_owned()?;
            }
            self.resources.validate_admission(*handle, None, None)?;
        }
        Ok(value)
    }

    fn evaluate_match_value(
        &mut self,
        expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let local = match self.owner(OwnerKey::Expression(expression))? {
            Some(OwnerRecord::Expression(record)) => match record.operation {
                ExpressionOperation::Local { value } => Some(value),
                _ => None,
            },
            Some(_) => {
                return Err(reference_error(
                    "normalized_reference_expression_kind",
                    "exact expression identity names another owner kind",
                ));
            }
            None => {
                return Err(reference_error(
                    "normalized_reference_expression_missing",
                    "exact expression is missing from canonical authority",
                ));
            }
        };
        if let Some(local) = local
            && locals
                .get(&local)
                .map(|value| value.ownership(&self.schema, &mut self.observation.value_work))
                .transpose()?
                .is_some_and(|ownership| ownership != Ownership::Ordinary)
        {
            if let Some(value) = locals.get(&local)
                && value.raw().memory_form().is_some()
            {
                value.raw().memory_validate(self.memory_domain, true)?;
            }
            return locals.remove(&local).ok_or_else(|| {
                reference_error(
                    "normalized_reference_local_missing",
                    "affine match value was already consumed",
                )
            });
        }
        self.evaluate(expression, locals)
    }

    fn function_signature(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<ReferenceSignature, ExecutionError> {
        let (
            has_implementations,
            type_parameters,
            effect_parameters,
            requirement_parameters,
            effect,
            parameters,
            result,
            result_borrow,
            pure,
        ) = match self.declaration(reference)?.payload {
            DeclarationPayload::Function(function) => (
                !function.implementation_parameters.is_empty(),
                function.type_parameters,
                function.effect_parameters,
                function.requirement_parameters,
                function.effect.clone(),
                function.parameters,
                function.result,
                function.result_borrow,
                matches!(function.effect, FunctionEffect::Pure),
            ),
            DeclarationPayload::External(external) => (
                false,
                external.type_parameters,
                Vec::new(),
                Vec::new(),
                FunctionEffect::Pure,
                external.parameters,
                external.result,
                None,
                true,
            ),
            DeclarationPayload::Constant { .. } => {
                return Err(reference_type_error(
                    "a constant is not a named callable descriptor",
                ));
            }
            _ => {
                return Err(reference_type_error(
                    "exact callable has a non-callable canonical owner",
                ));
            }
        };
        let type_parameter_constraints =
            self.type_parameter_constraints(reference, &type_parameters)?;
        Ok(ReferenceSignature {
            has_implementations,
            requirement_parameters,
            effect_parameters,
            effect,
            type_parameters,
            type_parameter_constraints,
            parameters: self.parameters(reference.package, &parameters)?,
            result,
            result_borrow,
            pure,
        })
    }

    fn require_owning_result(
        &mut self,
        function: DeclarationReference,
    ) -> Result<(), ExecutionError> {
        if matches!(self.declaration(function)?.payload,
            DeclarationPayload::Function(function) if function.result_borrow.is_some())
        {
            return Err(reference_type_error(
                "borrowed-result function requires a lexical borrow-call",
            ));
        }
        Ok(())
    }

    fn type_parameter_constraints(
        &mut self,
        reference: DeclarationReference,
        parameters: &[TypeParameterId],
    ) -> Result<Vec<crate::platform::kernel::TypeParameterConstraints>, ExecutionError> {
        self.charge_allocation(parameters.len() as u64)?;
        parameters
            .iter()
            .map(|parameter| {
                self.control.check()?;
                match self
                    .owner_in_package(reference.package, OwnerKey::TypeParameter(*parameter))?
                {
                    Some(OwnerRecord::TypeParameter(record))
                        if record.declaration == reference.declaration =>
                    {
                        Ok(record.constraints)
                    }
                    _ => Err(reference_type_error(
                        "canonical type parameter has no exact declaration binding",
                    )),
                }
            })
            .collect()
    }

    fn parameters(
        &mut self,
        package: PackageId,
        parameters: &[ParameterId],
    ) -> Result<Vec<ParameterRecord>, ExecutionError> {
        parameters
            .iter()
            .map(|parameter| {
                match self.owner_in_package(package, OwnerKey::Parameter(*parameter))? {
                    Some(OwnerRecord::Parameter(record)) => Ok(record),
                    _ => Err(reference_error(
                        "normalized_reference_parameter_missing",
                        "exact parameter is absent from canonical authority",
                    )),
                }
            })
            .collect()
    }

    fn function_parameter_uses(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<Vec<ParameterUse>, ExecutionError> {
        Ok(self
            .function_signature(reference)?
            .parameters
            .iter()
            .map(|parameter| parameter.use_mode)
            .collect())
    }

    fn operation_parameter_uses(
        &mut self,
        reference: OperationReference,
    ) -> Result<Vec<ParameterUse>, ExecutionError> {
        let Some(OwnerRecord::Operation(operation)) =
            self.owner_in_package(reference.package, OwnerKey::Operation(reference.operation))?
        else {
            return Err(reference_error(
                "normalized_reference_operation_missing",
                "exact capability operation is absent from canonical authority",
            ));
        };
        Ok(self
            .parameters(reference.package, &operation.parameters)?
            .into_iter()
            .map(|parameter| parameter.use_mode)
            .collect())
    }

    fn case_payload_is_direct_resource(&self, layout: VariantLayoutIndex, case: u32) -> bool {
        self.schema
            .variants
            .get(layout.0 as usize)
            .and_then(|variant| variant.cases.get(case as usize))
            .and_then(|case| case.payload)
            .and_then(|payload| self.schema.types.get(&payload))
            .is_some_and(|object| matches!(object.form, TypeForm::CapabilityResource { .. }))
    }

    fn binding(
        &mut self,
        binding: BindingId,
        expected: BindingKind,
    ) -> Result<crate::platform::kernel::BindingRecord, ExecutionError> {
        match self.owner(OwnerKey::Binding(binding))? {
            Some(OwnerRecord::Binding(record)) if record.kind == expected => Ok(record),
            Some(OwnerRecord::Binding(_)) => Err(reference_error(
                "normalized_reference_binding_kind",
                "canonical binding has the wrong exact lexical kind",
            )),
            Some(_) => Err(reference_error(
                "normalized_reference_binding_owner",
                "exact binding identity names another owner kind",
            )),
            None => Err(reference_error(
                "normalized_reference_binding_missing",
                "exact binding is missing from canonical authority",
            )),
        }
    }

    fn declaration(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<crate::platform::kernel::DeclarationRecord, ExecutionError> {
        match self.owner_in_package(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )? {
            Some(OwnerRecord::Declaration(record)) => Ok(record),
            Some(_) => Err(reference_error(
                "normalized_reference_declaration_kind",
                "exact declaration reference names another owner kind",
            )),
            None => Err(reference_error(
                "normalized_reference_declaration_missing",
                "exact declaration reference is missing from canonical authority",
            )),
        }
    }

    fn owner(&mut self, owner: OwnerKey) -> Result<Option<OwnerRecord>, ExecutionError> {
        self.owner_in_package(self.active_package, owner)
    }

    fn owner_in_package(
        &mut self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, ExecutionError> {
        let read = self.authority.owner_in_package(package, owner)?;
        self.observation.canonical_owner_reads = self
            .observation
            .canonical_owner_reads
            .saturating_add(read.work.owner_reads);
        self.observation.canonical_map_pages_read = self
            .observation
            .canonical_map_pages_read
            .saturating_add(read.work.map_pages_read);
        self.observation.canonical_objects_read = self
            .observation
            .canonical_objects_read
            .saturating_add(read.work.objects_read);
        self.observation.canonical_bytes_read = self
            .observation
            .canonical_bytes_read
            .saturating_add(read.work.bytes_read);
        Ok(read.record)
    }

    fn function_reference(
        &self,
        function: FunctionIndex,
    ) -> Result<DeclarationReference, ExecutionError> {
        self.schema
            .functions
            .get(function.0 as usize)
            .filter(|_| function.1 == self.schema.value_origin)
            .copied()
            .ok_or_else(|| {
                reference_error(
                    "normalized_reference_function_index",
                    "function value escaped the independent canonical callable inventory",
                )
            })
    }

    fn record(
        &mut self,
        nominal_type: Option<DeclarationReference>,
        arguments: &[TypeObjectDigest],
        selectors: impl IntoIterator<Item = FieldSelector>,
        values: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.charge_items(values.len(), std::mem::size_of::<NormalizedValue>())?;
        if let Some(declaration) = nominal_type {
            let layout = self.record_layout(declaration, arguments)?;
            let field_count = self.schema.records[layout.0 as usize].fields.len();
            let mut slots = (0..field_count).map(|_| None).collect::<Vec<_>>();
            for (selector, value) in selectors.into_iter().zip(values) {
                let FieldSelector::Nominal(field) = selector else {
                    return Err(reference_error(
                        "normalized_reference_record_field_kind",
                        "nominal record contains a structural field selector",
                    ));
                };
                let (field_layout, offset) = self.field_layout(field)?;
                if self.schema.records[field_layout.0 as usize].declaration != declaration {
                    return Err(reference_error(
                        "normalized_reference_record_field_layout",
                        "nominal record field belongs to another exact declaration",
                    ));
                }
                let slot = slots.get_mut(offset as usize).ok_or_else(|| {
                    reference_error(
                        "normalized_reference_record_field_offset",
                        "nominal field offset escaped its prepared layout",
                    )
                })?;
                if slot.replace(value).is_some() {
                    return Err(reference_error(
                        "normalized_reference_record_field_duplicate",
                        "nominal record repeats one exact field",
                    ));
                }
            }
            let fields = slots
                .into_iter()
                .map(|field| {
                    field.ok_or_else(|| {
                        reference_error(
                            "normalized_reference_record_field_missing",
                            "nominal record omits one exact field",
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let fields = self.schema.records[layout.0 as usize]
                .fields
                .iter()
                .map(|field| field.name.clone())
                .zip(fields)
                .collect();
            self.record_value(Some(layout), fields)
        } else {
            let mut fields = selectors
                .into_iter()
                .zip(values)
                .map(|(selector, value)| match selector {
                    FieldSelector::Structural(name) => Ok((name, value)),
                    FieldSelector::Nominal(_) => Err(reference_error(
                        "normalized_reference_structural_field_kind",
                        "structural record contains a nominal field selector",
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            fields.sort_by(|left, right| left.0.cmp(&right.0));
            if fields.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(reference_error(
                    "normalized_reference_structural_field_duplicate",
                    "structural record repeats one exact field name",
                ));
            }
            self.charge_allocation(fields.iter().fold(0_u64, |total, (name, _)| {
                total.saturating_add(name.as_str().len() as u64)
            }))?;
            self.record_value(None, fields)
        }
    }

    fn field(
        &mut self,
        value: CheckedValue,
        selector: FieldSelector,
    ) -> Result<CheckedValue, ExecutionError> {
        if let (NormalizedValue::OwnedProduct(token), FieldSelector::Structural(name)) =
            (value.raw(), &selector)
        {
            let Some(crate::platform::kernel::TypeObject {
                form: TypeForm::OwnedProduct { fields },
                ..
            }) = self.schema.types.get(&token.ty())
            else {
                return Err(reference_type_error("product metadata has no exact type"));
            };
            // Independent linear lookup rather than the VM's canonical binary search.
            let (index, field) = fields
                .iter()
                .enumerate()
                .find(|(_, field)| &field.name == name)
                .ok_or_else(|| reference_type_error("unknown product metadata field"))?;
            let ty = field.ty;
            if !self.schema.ordinary_types.contains(&ty) {
                return Err(reference_type_error(
                    "product field read cannot expose owned children",
                ));
            }
            let control = self.control;
            let raw = token.read_metadata(self.memory_domain, index, control, &mut |bytes| {
                self.charge_allocation(bytes)
            })?;
            self.admit_raw(raw, ty, &BTreeMap::new(), None, false)
        } else {
            value.project_field(selector, &self.schema)
        }
    }

    fn record_layout(
        &self,
        declaration: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Result<RecordLayoutIndex, ExecutionError> {
        let identity = super::value_schema::nominal_identity(declaration, arguments)
            .map_err(|_| reference_type_error("invalid record application"))?;
        if !arguments.is_empty() && !self.schema.ordinary_types.contains(&identity) {
            return Err(reference_type_error(
                "record application contains live authority",
            ));
        }
        self.schema
            .record_index(identity)
            .and_then(|index| u32::try_from(index).ok())
            .map(|index| RecordLayoutIndex(index, self.schema.value_origin))
            .ok_or_else(|| {
                reference_error(
                    "normalized_reference_record_layout",
                    "exact record declaration has no prepared runtime layout",
                )
            })
    }

    fn field_layout(
        &self,
        field: FieldReference,
    ) -> Result<(RecordLayoutIndex, u32), ExecutionError> {
        for (layout_index, layout) in self.schema.records.iter().enumerate() {
            if let Some(offset) = layout
                .fields
                .iter()
                .position(|candidate| candidate.reference == field)
            {
                let layout_index = u32::try_from(layout_index).map_err(|_| {
                    reference_resource(
                        "normalized_reference_record_count",
                        "prepared record layout count exceeds the dense index domain",
                    )
                })?;
                let offset = u32::try_from(offset).map_err(|_| {
                    reference_resource(
                        "normalized_reference_field_count",
                        "prepared field layout count exceeds the dense index domain",
                    )
                })?;
                return Ok((
                    RecordLayoutIndex(layout_index, self.schema.value_origin),
                    offset,
                ));
            }
        }
        Err(reference_error(
            "normalized_reference_field_layout",
            "exact field has no prepared runtime layout",
        ))
    }

    fn matches_case(
        &self,
        layout: VariantLayoutIndex,
        tag: u32,
        case: CaseReference,
    ) -> Result<bool, ExecutionError> {
        if layout.1 != self.schema.value_origin {
            return Err(reference_type_error("match value has a foreign origin"));
        }
        let member = self
            .schema
            .variants
            .get(layout.0 as usize)
            .and_then(|layout| layout.cases.get(tag as usize))
            .ok_or_else(|| reference_type_error("match value has no exact canonical case"))?;
        Ok(member.reference == case)
    }

    fn case_layout(
        &self,
        case: CaseReference,
    ) -> Result<(VariantLayoutIndex, u32), ExecutionError> {
        for (layout_index, layout) in self.schema.variants.iter().enumerate() {
            if let Some(tag) = layout
                .cases
                .iter()
                .position(|candidate| candidate.reference == case)
            {
                let layout_index = u32::try_from(layout_index).map_err(|_| {
                    reference_resource(
                        "normalized_reference_variant_count",
                        "prepared variant layout count exceeds the dense index domain",
                    )
                })?;
                let tag = u32::try_from(tag).map_err(|_| {
                    reference_resource(
                        "normalized_reference_case_count",
                        "prepared case count exceeds the dense index domain",
                    )
                })?;
                return Ok((
                    VariantLayoutIndex(layout_index, self.schema.value_origin),
                    tag,
                ));
            }
        }
        Err(reference_error(
            "normalized_reference_case_layout",
            "exact variant case has no prepared runtime layout",
        ))
    }

    fn capability_call(
        &mut self,
        requirement: RequirementReference,
        operation: OperationReference,
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.admit_operation_allowance(requirement)?;
        let Some(OwnerRecord::Operation(record)) =
            self.owner_in_package(operation.package, OwnerKey::Operation(operation.operation))?
        else {
            return Err(reference_type_error(
                "capability result type is absent from canonical authority",
            ));
        };
        let parameters = self.parameters(operation.package, &record.parameters)?;
        self.validate_capability_arguments(requirement, &parameters, &arguments)?;
        let arguments = arguments.into_iter().map(CheckedValue::release).collect();
        self.charge_capability_call(requirement)?;
        let capabilities = self
            .capabilities
            .ok_or_else(reference_capabilities_unbound)?;
        let canonical = capabilities.canonical_requirement_exact(self.program, requirement)?;
        let transactional = self.transactions.contains_key(&canonical);
        let value = if let Some(transaction) = self.transactions.get_mut(&canonical) {
            let policy = self
                .capabilities
                .ok_or_else(reference_capabilities_unbound)?
                .call_policy_exact(self.program, requirement, operation)?;
            let result =
                transaction
                    .transaction
                    .call(&policy, arguments, self.resources, self.control);
            validate_outcome(&policy, result)?
        } else {
            capabilities.call_exact(
                self.program,
                requirement,
                operation,
                arguments,
                self.resources,
                self.control,
            )?
        };
        self.admit_raw(value, record.result, &BTreeMap::new(), Some(requirement), false).map_err(|error| {
            if !transactional && record.external_visibility == crate::platform::kernel::ExternalVisibility::Possible {
                ExecutionError::new(ExecutionFailureClass::PossibleVisibility, error.code, "reference adapter result failed admission after a possibly visible operation; inspect the outcome before retrying")
            } else { error }
        })
    }

    fn transaction(
        &mut self,
        requirement: RequirementReference,
        binding: BindingId,
        body: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
        outcome: Option<(TransactionOutcomeContract, TypeObjectDigest)>,
    ) -> Result<CheckedValue, ExecutionError> {
        let outcome = outcome
            .map(|(contract, ty)| self.outcome_layouts(contract, ty))
            .transpose()?;
        self.admit_operation_allowance(requirement)?;
        self.binding(binding, BindingKind::Transaction)?;
        let capabilities = self
            .capabilities
            .ok_or_else(reference_capabilities_unbound)?;
        let canonical = capabilities.canonical_requirement_exact(self.program, requirement)?;
        if self.transactions.contains_key(&canonical) {
            return Err(reference_error(
                "normalized_reference_transaction_nested",
                "one exact requirement cannot begin a nested transaction",
            ));
        }
        let local = LocalValueReference::TransactionBinding(binding);
        if locals.contains_key(&local) {
            return Err(reference_error(
                "normalized_reference_transaction_binding",
                "transaction binding identity was already live",
            ));
        }
        let generation = self.next_transaction;
        let next_generation = self.next_transaction.checked_add(1).ok_or_else(|| {
            reference_resource(
                "normalized_reference_transaction_generation",
                "reference transaction generation overflowed",
            )
        })?;
        self.charge_capability_call(requirement)?;
        let transaction = capabilities.begin_transaction_exact(
            self.program,
            requirement,
            self.resources,
            self.control,
        )?;
        self.next_transaction = next_generation;
        // Binding insertion is required in optimized builds too.
        let previous = locals.insert(
            local,
            CheckedValue::primitive(&self.schema, NormalizedValue::Unit)?,
        );
        debug_assert!(previous.is_none());
        self.transactions.insert(
            canonical,
            ReferenceTransaction {
                binding,
                generation,
                transaction,
            },
        );
        let result = self.evaluate(body, locals);
        let token = locals.remove(&local).map(CheckedValue::release);
        let mut transaction = self.transactions.remove(&canonical).ok_or_else(|| {
            reference_error(
                "normalized_reference_transaction_missing",
                "reference transaction disappeared before scope completion",
            )
        })?;
        if transaction.binding != binding
            || transaction.generation != generation
            || !matches!(token, Some(NormalizedValue::Unit))
        {
            let _ = transaction.transaction.rollback();
            return Err(reference_error(
                "normalized_reference_transaction_binding",
                "transaction scope lost its exact runtime binding",
            ));
        }
        match result {
            Ok(value) => {
                if let Some((layout, reason_layout, [committed, aborted, condition, conflict])) =
                    outcome
                {
                    // Complete every fallible layout/ownership/capacity operation before commit.
                    // Retain the checked payload directly in a private, already built success.
                    let prepared = (|| {
                        self.charge_allocation(
                            (3 * std::mem::size_of::<NormalizedValue>()) as u64,
                        )?;
                        let committed = self.variant_value(layout, committed, Some(value))?;
                        let condition = self.variant_value(reason_layout, condition, None)?;
                        let condition = self.variant_value(layout, aborted, Some(condition))?;
                        let conflict = self.variant_value(reason_layout, conflict, None)?;
                        let conflict = self.variant_value(layout, aborted, Some(conflict))?;
                        Ok::<_, ExecutionError>((committed, condition, conflict))
                    })();
                    let (committed, condition, conflict) = match prepared {
                        Ok(choices) => choices,
                        Err(error) => {
                            let _ = transaction.transaction.rollback();
                            return Err(error);
                        }
                    };
                    let completion = match transaction.transaction.commit(self.control) {
                        Ok(completion) => completion,
                        Err(error) => {
                            let _ = transaction.transaction.rollback();
                            return Err(error);
                        }
                    };
                    Ok(match completion {
                        NormalizedTransactionCompletion::Committed => committed,
                        NormalizedTransactionCompletion::ConditionFailed => condition,
                        NormalizedTransactionCompletion::Conflict => conflict,
                    })
                } else {
                    transaction.transaction.commit(self.control)?.legacy()?;
                    Ok(value)
                }
            }
            Err(error) => {
                let _ = transaction.transaction.rollback();
                Err(error)
            }
        }
    }

    fn outcome_layouts(
        &mut self,
        contract: TransactionOutcomeContract,
        type_argument: TypeObjectDigest,
    ) -> Result<(VariantLayoutIndex, VariantLayoutIndex, [u32; 4]), ExecutionError> {
        contract.validate_identity().map_err(|_| {
            reference_type_error("transaction completion uses foreign nominal identities")
        })?;
        let arguments = self.resolve_type_arguments(&[type_argument])?;
        let result_type = super::value_schema::nominal_identity(contract.outcome, &arguments)
            .map_err(|_| reference_type_error("transaction result has an invalid application"))?;
        let reason_type = super::value_schema::nominal_identity(contract.abort_reason, &[])
            .map_err(|_| reference_type_error("transaction abort has an invalid identity"))?;
        if !self.schema.ordinary_types.contains(&result_type) {
            return Err(reference_type_error(
                "transaction result contains inadmissible authority",
            ));
        }
        let result_index = self
            .schema
            .variant_index(result_type)
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| reference_type_error("transaction result has no canonical layout"))?;
        let reason_index = self
            .schema
            .variant_index(reason_type)
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| reference_type_error("transaction abort has no canonical layout"))?;
        let result = &self.schema.variants[result_index as usize];
        let reason = &self.schema.variants[reason_index as usize];
        if result.declaration != contract.outcome
            || reason.declaration != contract.abort_reason
            || result.cases.len() != 2
            || reason.cases.len() != 2
            || result.arguments.as_ref() != arguments.as_slice()
            || !reason.arguments.is_empty()
        {
            return Err(reference_type_error(
                "transaction completion has foreign canonical declarations",
            ));
        }
        let select = |cases: &[super::prepare::NormalizedVariantCase], reference, payload| {
            cases
                .iter()
                .enumerate()
                .find(|(_, case)| case.reference == reference && case.payload == payload)
                .and_then(|(index, _)| u32::try_from(index).ok())
                .ok_or_else(|| {
                    reference_type_error(
                        "transaction completion has a foreign canonical case or payload",
                    )
                })
        };
        Ok((
            VariantLayoutIndex(result_index, self.schema.value_origin),
            VariantLayoutIndex(reason_index, self.schema.value_origin),
            [
                select(&result.cases, contract.committed, Some(arguments[0]))?,
                select(&result.cases, contract.aborted, Some(reason_type))?,
                select(&reason.cases, contract.condition_failed, None)?,
                select(&reason.cases, contract.conflict, None)?,
            ],
        ))
    }

    fn text(&mut self, value: TextValue) -> Result<Arc<str>, ExecutionError> {
        match value {
            TextValue::Inline { text } => Ok(Arc::from(text)),
            TextValue::Blob { digest, bytes } => {
                let value = self.authority.blob(digest)?;
                self.observation.canonical_objects_read =
                    self.observation.canonical_objects_read.saturating_add(1);
                self.observation.canonical_bytes_read = self
                    .observation
                    .canonical_bytes_read
                    .saturating_add(value.len() as u64);
                if value.len() as u64 != bytes {
                    return Err(reference_error(
                        "normalized_reference_blob_length",
                        "reference text blob length disagrees with canonical meaning",
                    ));
                }
                let value = String::from_utf8(value).map_err(|_| {
                    reference_error(
                        "normalized_reference_blob_utf8",
                        "reference text blob is not valid UTF-8",
                    )
                })?;
                Ok(Arc::from(value))
            }
        }
    }

    fn charge_capability_call(
        &mut self,
        requirement: RequirementReference,
    ) -> Result<(), ExecutionError> {
        if self
            .policy
            .maximum_capability_calls
            .is_some_and(|maximum| self.observation.capability_calls >= maximum)
        {
            return Err(reference_resource(
                "normalized_reference_capability_calls",
                "reference execution exhausted its capability-call budget",
            ));
        }
        let capabilities = self
            .capabilities
            .ok_or_else(reference_capabilities_unbound)?;
        let maximum = capabilities.maximum_calls_exact(requirement)?;
        let canonical = capabilities.canonical_requirement_exact(self.program, requirement)?;
        let calls = self.calls_by_requirement.entry(canonical).or_default();
        *calls = crate::platform::execution::cumulative_charge(
            *calls,
            1,
            Some(maximum),
            "normalized_reference_grant_calls",
            "reference execution exhausted one deployment-grant call bound",
        )?;
        self.observation.capability_calls = self.observation.capability_calls.saturating_add(1);
        Ok(())
    }

    fn charge_items(&mut self, items: usize, item_bytes: usize) -> Result<(), ExecutionError> {
        if items as u64 > super::value::MAXIMUM_ADMISSION_ITEMS {
            return Err(reference_resource(
                "normalized_reference_collection_items",
                "one container exceeds finite item admission",
            ));
        }
        let next_items = crate::platform::execution::cumulative_charge(
            self.observation.collection_items,
            items as u64,
            self.policy.maximum_collection_items,
            "normalized_reference_collection_items",
            "execution exhausted its collection-item budget",
        )?;
        if let Some(budget) = &self.shared_budget {
            budget.reserve(0, items as u64)?;
        }
        self.observation.collection_items = next_items;
        let bytes = super::value::collection_storage_bytes(
            items as u64,
            item_bytes as u64,
            "normalized_reference_allocation",
        )?;
        self.charge_allocation(bytes)
    }

    fn charge_allocation(&mut self, bytes: u64) -> Result<(), ExecutionError> {
        if bytes > super::value::MAXIMUM_VALUE_ALLOCATION_BYTES {
            return Err(reference_resource(
                "normalized_reference_allocation",
                "one allocation exceeds finite value storage",
            ));
        }
        let next_bytes = crate::platform::execution::cumulative_charge(
            self.observation.allocated_bytes,
            bytes,
            self.policy.maximum_allocated_bytes,
            "normalized_reference_allocation",
            "execution exhausted its allocation budget",
        )?;
        if let Some(budget) = &self.shared_budget {
            budget.reserve(bytes, 0)?;
        }
        self.observation.allocated_bytes = next_bytes;
        if bytes != 0 {
            self.observation.allocation_charges =
                self.observation.allocation_charges.saturating_add(1);
        }
        Ok(())
    }

    fn charge_value(&mut self, value: &NormalizedValue) -> Result<(), ExecutionError> {
        let (bytes, items) = reference_value_cost(value)?;
        if items > super::value::MAXIMUM_ADMISSION_ITEMS {
            return Err(reference_resource(
                "normalized_reference_collection_items",
                "one external value exceeds finite item admission",
            ));
        }
        let next_items = crate::platform::execution::cumulative_charge(
            self.observation.collection_items,
            items,
            self.policy.maximum_collection_items,
            "normalized_reference_collection_items",
            "external value exceeds the collection-item budget",
        )?;
        if let Some(budget) = &self.shared_budget {
            budget.reserve(0, items)?;
        }
        self.observation.collection_items = next_items;
        self.charge_allocation(bytes)
    }

    fn rollback_all(&mut self) {
        let transactions = std::mem::take(&mut self.transactions);
        let mut transactions = transactions.into_values().collect::<Vec<_>>();
        transactions.sort_by_key(|transaction| transaction.generation);
        for mut transaction in transactions.into_iter().rev() {
            let _ = transaction.transaction.rollback();
        }
    }
}

fn reference_intrinsic(
    program: &dyn NormalizedValueSchema,
    function: &ReferenceSignature,
    implementation: &str,
    type_arguments: &[TypeObjectDigest],
    arguments: Vec<NormalizedValue>,
    control: &ExecutionControl,
) -> Result<NormalizedValue, ExecutionError> {
    match implementation {
        "identity_host" => match arguments.as_slice() {
            [] => Ok(NormalizedValue::Unit),
            [value] => Ok(value.clone()),
            _ => Err(reference_type_error(
                "identity host received a foreign arity",
            )),
        },
        "core.f64.add" | "core.f64.subtract" | "core.f64.multiply" | "core.f64.divide" => {
            let (left, right) = reference_f64_pair(&arguments)?;
            let answer = match implementation {
                "core.f64.add" => left + right,
                "core.f64.subtract" => left - right,
                "core.f64.multiply" => left * right,
                _ => left / right,
            };
            Ok(NormalizedValue::F64(Binary64::from_float(answer)))
        }
        "core.f64.negate" => reference_unary_f64(&arguments, |number| -number),
        "core.f64.abs" => reference_unary_f64(&arguments, f64::abs),
        "core.f64.sqrt" => reference_unary_f64(&arguments, f64::sqrt),
        "core.f64.less" | "core.f64.less-equal" => {
            let (left, right) = reference_f64_pair(&arguments)?;
            let ordered = match implementation {
                "core.f64.less" => left < right,
                _ => left <= right,
            };
            Ok(NormalizedValue::Bool(ordered))
        }
        "core.f64.is-finite" | "core.f64.is-nan" => {
            let number = reference_f64_argument(&arguments)?;
            let predicate = match implementation {
                "core.f64.is-finite" => number.is_finite(),
                _ => number.is_nan(),
            };
            Ok(NormalizedValue::Bool(predicate))
        }
        "core.f64.from-i64" => match arguments.as_slice() {
            [NormalizedValue::I64(number)] => {
                Ok(NormalizedValue::F64(Binary64::from_float(*number as f64)))
            }
            _ => Err(reference_type_error("F64 conversion requires one integer")),
        },
        "core.f64.to-i64-result" => {
            let converted = reference_f64_to_i64(reference_f64_argument(&arguments)?);
            reference_structural_record(vec![
                ("value", NormalizedValue::I64(converted.unwrap_or_default())),
                ("valid", NormalizedValue::Bool(converted.is_some())),
            ])
        }
        "core.f64.parse-result" => match arguments.as_slice() {
            [NormalizedValue::Text(text)] => {
                let number = Binary64::parse(text);
                reference_structural_record(vec![
                    (
                        "value",
                        NormalizedValue::F64(number.unwrap_or_else(|| Binary64::from_float(0.0))),
                    ),
                    ("valid", NormalizedValue::Bool(number.is_some())),
                ])
            }
            _ => Err(reference_type_error("F64 parser requires one text value")),
        },
        "core.f64.to-text" => match arguments.as_slice() {
            [NormalizedValue::F64(number)] => Ok(NormalizedValue::text(number.to_text())),
            _ => Err(reference_type_error("F64 formatter requires one F64 value")),
        },
        "core.i64.add" => reference_binary_i64(
            arguments,
            i64::checked_add,
            "reference_integer_overflow",
            "integer addition overflow",
        ),
        "core.i64.subtract" => reference_binary_i64(
            arguments,
            i64::checked_sub,
            "reference_integer_overflow",
            "integer subtraction overflow",
        ),
        "core.i64.multiply" => reference_binary_i64(
            arguments,
            i64::checked_mul,
            "reference_integer_overflow",
            "integer multiplication overflow",
        ),
        "core.i64.divide" => {
            let (left, right) = reference_i64_pair(arguments)?;
            left.checked_div(right)
                .map(NormalizedValue::I64)
                .ok_or_else(|| {
                    reference_trap(
                        "reference_integer_division",
                        "integer division by zero or signed overflow",
                    )
                })
        }
        "core.i64.equal" => {
            let (left, right) = reference_i64_pair(arguments)?;
            Ok(NormalizedValue::Bool(left == right))
        }
        "core.i64.less" | "core.i64.less-equal" => {
            let (left, right) = reference_i64_pair(arguments)?;
            let value = if implementation == "core.i64.less" {
                left < right
            } else {
                left <= right
            };
            Ok(NormalizedValue::Bool(value))
        }
        "core.i64.to-text" => match arguments.as_slice() {
            [NormalizedValue::I64(value)] => Ok(NormalizedValue::text(format!("{value}"))),
            _ => Err(reference_type_error(
                "integer formatter received a foreign value",
            )),
        },
        "core.i64.parse" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => reference_parse_i64(value)
                .map(NormalizedValue::I64)
                .ok_or_else(|| {
                    reference_trap(
                        "reference_integer_parse",
                        "text is not a canonical signed 64-bit integer",
                    )
                }),
            _ => Err(reference_type_error(
                "integer parser received a foreign value",
            )),
        },
        "core.i64.parse-result" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => {
                let parsed = reference_parse_i64(value);
                reference_structural_record(vec![
                    ("value", NormalizedValue::I64(parsed.unwrap_or_default())),
                    ("valid", NormalizedValue::Bool(parsed.is_some())),
                ])
            }
            _ => Err(reference_type_error(
                "integer parser received a foreign value",
            )),
        },
        "core.bool.not" => {
            let [NormalizedValue::Bool(value)] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "boolean intrinsic received a foreign value",
                ));
            };
            Ok(NormalizedValue::Bool(!value))
        }
        "core.bool.and" | "core.bool.or" => match arguments.as_slice() {
            [NormalizedValue::Bool(left), NormalizedValue::Bool(right)] => {
                let value = match implementation {
                    "core.bool.and" => *left && *right,
                    _ => *left || *right,
                };
                Ok(NormalizedValue::Bool(value))
            }
            _ => Err(reference_type_error(
                "boolean intrinsic received a foreign value",
            )),
        },
        "core.text.concat" => {
            let [NormalizedValue::Text(left), NormalizedValue::Text(right)] = arguments.as_slice()
            else {
                return Err(reference_type_error(
                    "text intrinsic received a foreign value",
                ));
            };
            let length = left.len().checked_add(right.len()).ok_or_else(|| {
                reference_resource(
                    "normalized_reference_text_length",
                    "text concatenation length overflowed",
                )
            })?;
            let mut value = String::with_capacity(length);
            value.push_str(left);
            value.push_str(right);
            Ok(NormalizedValue::Text(Arc::from(value)))
        }
        "core.text.equal" => {
            let [NormalizedValue::Text(left), NormalizedValue::Text(right)] = arguments.as_slice()
            else {
                return Err(reference_type_error(
                    "text intrinsic received a foreign value",
                ));
            };
            Ok(NormalizedValue::Bool(left == right))
        }
        "core.text.contains" => match arguments.as_slice() {
            [
                NormalizedValue::Text(value),
                NormalizedValue::Text(fragment),
            ] => Ok(NormalizedValue::Bool(
                value.find(fragment.as_ref()).is_some(),
            )),
            _ => Err(reference_type_error(
                "text containment received a foreign value",
            )),
        },
        "core.text.starts-with" => match arguments.as_slice() {
            [NormalizedValue::Text(value), NormalizedValue::Text(prefix)] => Ok(
                NormalizedValue::Bool(value.get(..prefix.len()) == Some(prefix.as_ref())),
            ),
            _ => Err(reference_type_error(
                "text prefix predicate received a foreign value",
            )),
        },
        "core.text.length" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => {
                Ok(NormalizedValue::I64(reference_length(value.len())?))
            }
            _ => Err(reference_type_error("text length received a foreign value")),
        },
        "core.text.empty" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => Ok(NormalizedValue::Bool(value.is_empty())),
            _ => Err(reference_type_error(
                "text emptiness received a foreign value",
            )),
        },
        "core.text.from-static" => match arguments.as_slice() {
            [NormalizedValue::StaticText(value)] => {
                Ok(NormalizedValue::Text(Arc::<str>::from(value.as_ref())))
            }
            _ => Err(reference_type_error(
                "static-text conversion received a foreign value",
            )),
        },
        "core.html.escape-text" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => {
                let escaped = value.chars().fold(String::new(), |mut output, character| {
                    match character {
                        '&' => output.push_str("&amp;"),
                        '<' => output.push_str("&lt;"),
                        '>' => output.push_str("&gt;"),
                        '"' => output.push_str("&quot;"),
                        '\'' => output.push_str("&#39;"),
                        _ => output.push(character),
                    }
                    output
                });
                Ok(NormalizedValue::text(escaped))
            }
            _ => Err(reference_type_error(
                "HTML escaping received a foreign value",
            )),
        },
        "core.json.string" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => serde_json::to_string(value.as_ref())
                .map(NormalizedValue::text)
                .map_err(|_| {
                    reference_error(
                        "reference_json_string_encode",
                        "JSON string encoding failed",
                    )
                }),
            _ => Err(reference_type_error(
                "JSON string encoding received a foreign value",
            )),
        },
        "core.json.encode" => {
            let [value] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "typed JSON encoding received a foreign arity",
                ));
            };
            let Some(parameter) = function.parameters.first() else {
                return Err(reference_error(
                    "reference_json_signature",
                    "typed JSON encoder has no exact parameter type",
                ));
            };
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => parameter.ty,
                _ => {
                    return Err(reference_error(
                        "reference_json_signature",
                        "typed JSON encoder has no exact runtime type",
                    ));
                }
            };
            super::codec::encode_typed_with_control(
                program,
                value,
                ty,
                JsonLimits::default(),
                control,
            )
            .map(NormalizedValue::bytes)
            .map_err(reference_json_error)
        }
        "core.json.decode-or" => {
            let [NormalizedValue::Bytes(bytes), fallback] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "typed JSON decoding received foreign values",
                ));
            };
            let Some(fallback_type) = function.parameters.get(1) else {
                return Err(reference_error(
                    "reference_json_signature",
                    "typed JSON decoder has no exact fallback type",
                ));
            };
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => fallback_type.ty,
                _ => {
                    return Err(reference_error(
                        "reference_json_signature",
                        "typed JSON decoder has no exact runtime type",
                    ));
                }
            };
            let decoded = super::codec::decode_typed_with_control(
                program,
                bytes,
                ty,
                JsonLimits::default(),
                control,
            );
            control.check()?;
            let (valid, value, error) = match decoded {
                Ok(value) => (true, value, String::new()),
                Err(error)
                    if matches!(
                        error.class,
                        DiagnosticClass::Cancelled | DiagnosticClass::Resource
                    ) =>
                {
                    return Err(reference_json_error(error));
                }
                Err(diagnostic) => (false, fallback.clone(), diagnostic.code),
            };
            reference_structural_record(vec![
                ("value", value),
                ("error", NormalizedValue::text(error)),
                ("valid", NormalizedValue::Bool(valid)),
            ])
        }
        "core.data.encode" => {
            let [value] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "typed data encoding received a foreign arity",
                ));
            };
            let Some(parameter) = function.parameters.first() else {
                return Err(reference_error(
                    "reference_data_signature",
                    "typed data encoder has no exact parameter type",
                ));
            };
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => parameter.ty,
                _ => {
                    return Err(reference_error(
                        "reference_data_signature",
                        "typed data encoder has no exact runtime type",
                    ));
                }
            };
            super::data_codec_reference::encode_typed_with_control(program, value, ty, control)
                .map(NormalizedValue::bytes)
                .map_err(reference_json_error)
        }
        "core.data.decode-or" => {
            let [NormalizedValue::Bytes(bytes), fallback] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "typed data decoding received foreign values",
                ));
            };
            let Some(fallback_type) = function.parameters.get(1) else {
                return Err(reference_error(
                    "reference_data_signature",
                    "typed data decoder has no exact fallback type",
                ));
            };
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => fallback_type.ty,
                _ => {
                    return Err(reference_error(
                        "reference_data_signature",
                        "typed data decoder has no exact runtime type",
                    ));
                }
            };
            match super::data_codec_reference::decode_typed_with_control(
                program, bytes, ty, control,
            ) {
                Ok(value) => Ok(value),
                Err(error)
                    if matches!(
                        error.class,
                        DiagnosticClass::Cancelled | DiagnosticClass::Resource
                    ) =>
                {
                    Err(reference_json_error(error))
                }
                Err(_) => Ok(fallback.clone()),
            }
        }
        "core.http.bearer-token" => reference_bearer_token(program, arguments.as_slice()),
        "core.http.media-type-is" => match arguments.as_slice() {
            [
                NormalizedValue::Bytes(value),
                NormalizedValue::Text(expected),
            ] => Ok(NormalizedValue::Bool(reference_media_type_matches(
                value, expected,
            ))),
            _ => Err(reference_type_error(
                "media-type predicate received foreign values",
            )),
        },
        "core.bytes.from-list" => {
            let [NormalizedValue::List(items)] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "byte construction requires one integer list",
                ));
            };
            if items.len() as u64 > super::value::MAXIMUM_VALUE_ALLOCATION_BYTES {
                return Err(reference_resource(
                    "normalized_reference_allocation",
                    "byte output exceeds finite value storage",
                ));
            }
            let mut output = vec![0_u8; items.len()];
            for (index, item) in items.iter().enumerate() {
                control.check()?;
                match item {
                    NormalizedValue::I64(value) if (0..=255).contains(value) => {
                        output[index] = *value as u8
                    }
                    NormalizedValue::I64(_) => {
                        return Err(reference_trap(
                            "reference_bytes_octet",
                            "byte value is outside 0 through 255",
                        ));
                    }
                    _ => {
                        return Err(reference_type_error(
                            "byte construction requires integer elements",
                        ));
                    }
                }
            }
            control.check()?;
            Ok(NormalizedValue::Bytes(output.into()))
        }
        "core.bytes.to-text-result" => {
            let [NormalizedValue::Bytes(bytes)] = arguments.as_slice() else {
                return Err(reference_type_error("UTF-8 decoding requires bytes"));
            };
            if bytes.len() as u64 > super::value::MAXIMUM_VALUE_ALLOCATION_BYTES {
                return Err(reference_resource(
                    "normalized_reference_allocation",
                    "UTF-8 input exceeds finite value storage",
                ));
            }
            let (valid, text) = match std::str::from_utf8(bytes) {
                Ok(text) => (true, text),
                Err(_) => (false, ""),
            };
            control.check()?;
            reference_structural_record(vec![
                ("valid", NormalizedValue::Bool(valid)),
                ("value", NormalizedValue::text(text)),
            ])
        }
        "core.bytes.from-text" => match arguments.as_slice() {
            [NormalizedValue::Text(value)] => Ok(NormalizedValue::bytes(value.as_bytes())),
            _ => Err(reference_type_error(
                "text-to-bytes received a foreign value",
            )),
        },
        "core.bytes.to-text" => match arguments.as_slice() {
            [NormalizedValue::Bytes(value)] => std::str::from_utf8(value)
                .map(|value| NormalizedValue::text(value.to_owned()))
                .map_err(|_| {
                    reference_trap(
                        "reference_bytes_utf8",
                        "bytes are not a valid UTF-8 text encoding",
                    )
                }),
            _ => Err(reference_type_error(
                "bytes-to-text received a foreign value",
            )),
        },
        "core.bytes.concat" => match arguments.as_slice() {
            [NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)] => {
                let size = left.len().checked_add(right.len()).ok_or_else(|| {
                    reference_resource(
                        "reference_bytes_length",
                        "byte concatenation length overflowed",
                    )
                })?;
                let mut bytes = vec![0_u8; size];
                bytes[..left.len()].copy_from_slice(left);
                bytes[left.len()..].copy_from_slice(right);
                Ok(NormalizedValue::bytes(bytes))
            }
            _ => Err(reference_type_error(
                "byte concatenation received foreign values",
            )),
        },
        "core.bytes.length" => match arguments.as_slice() {
            [NormalizedValue::Bytes(value)] => {
                Ok(NormalizedValue::I64(reference_length(value.len())?))
            }
            _ => Err(reference_type_error("byte length received a foreign value")),
        },
        "core.bytes.slice" => match arguments.as_slice() {
            [
                NormalizedValue::Bytes(value),
                NormalizedValue::I64(start),
                NormalizedValue::I64(end),
            ] => {
                let selected = reference_byte_range(value, *start, *end)?;
                control.check()?;
                Ok(NormalizedValue::bytes(selected))
            }
            _ => Err(reference_type_error("byte slicing received foreign values")),
        },
        "core.bytes.copy" => match arguments.as_slice() {
            [NormalizedValue::Bytes(value)] => {
                control.check()?;
                Ok(NormalizedValue::bytes(value.as_ref()))
            }
            _ => Err(reference_type_error("byte copying received foreign values")),
        },
        "core.bytes.get" => match arguments.as_slice() {
            [NormalizedValue::Bytes(value), NormalizedValue::I64(index)] => {
                match usize::try_from(*index) {
                    Ok(index) if index < value.len() => {
                        Ok(NormalizedValue::I64(i64::from(value[index])))
                    }
                    _ => Err(reference_trap(
                        "reference_bytes_index",
                        "byte index is out of bounds",
                    )),
                }
            }
            _ => Err(reference_type_error("byte lookup received foreign values")),
        },
        "core.bytes.to-hex" => match arguments.as_slice() {
            [NormalizedValue::Bytes(value)] => {
                let size = value.len().checked_mul(2).ok_or_else(|| {
                    reference_resource("reference_text_length", "hex output length overflowed")
                })?;
                let mut bytes = Vec::with_capacity(size);
                const DIGITS: &[u8; 16] = b"0123456789abcdef";
                for value in value.iter().copied() {
                    bytes.push(DIGITS[usize::from(value / 16)]);
                    bytes.push(DIGITS[usize::from(value % 16)]);
                }
                let text = String::from_utf8(bytes).map_err(|_| {
                    reference_error(
                        "reference_hex_encoding",
                        "hex encoder produced invalid UTF-8",
                    )
                })?;
                Ok(NormalizedValue::text(text))
            }
            _ => Err(reference_type_error(
                "hex encoding received a foreign value",
            )),
        },
        "core.bytes.equal" => match arguments.as_slice() {
            [NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)] => {
                Ok(NormalizedValue::Bool(left.as_ref() == right.as_ref()))
            }
            _ => Err(reference_type_error(
                "byte equality received foreign values",
            )),
        },
        "core.bytes.blake3" => match arguments.as_slice() {
            [NormalizedValue::Bytes(value)] => Ok(NormalizedValue::bytes(
                blake3::hash(value).as_bytes().to_vec(),
            )),
            _ => Err(reference_type_error(
                "BLAKE3 hashing received a foreign value",
            )),
        },
        "core.value.equal" => {
            if type_arguments
                .iter()
                .any(|ty| !program.application_free(*ty) && !program.comparable(*ty))
            {
                return Err(reference_trap(
                    "normalized_reference_value_not_comparable",
                    "nominal application's complete type does not support equality",
                ));
            }
            let [left, right] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "value equality received a foreign arity",
                ));
            };
            Ok(NormalizedValue::Bool(reference_equal(left, right)?))
        }
        "core.list.length" => {
            let [NormalizedValue::List(values)] = arguments.as_slice() else {
                return Err(reference_type_error("list length received a foreign value"));
            };
            let length = i64::try_from(values.len()).map_err(|_| {
                reference_resource(
                    "normalized_reference_value_length",
                    "list length exceeds i64",
                )
            })?;
            Ok(NormalizedValue::I64(length))
        }
        "core.list.get" => match arguments.as_slice() {
            [NormalizedValue::List(values), NormalizedValue::I64(index)] => {
                let index = usize::try_from(*index).map_err(|_| {
                    reference_trap(
                        "reference_list_index",
                        "list index is negative or excessive",
                    )
                })?;
                values.get(index).cloned().ok_or_else(|| {
                    reference_trap("reference_list_index", "list index is out of bounds")
                })
            }
            _ => Err(reference_type_error("list lookup received foreign values")),
        },
        "core.option.some" => match arguments.as_slice() {
            [value] => Ok(NormalizedValue::Option(Some(Box::new(value.clone())))),
            _ => Err(reference_type_error(
                "option constructor received a foreign arity",
            )),
        },
        "core.option.none" => match arguments.as_slice() {
            [] => Ok(NormalizedValue::Option(None)),
            _ => Err(reference_type_error(
                "empty option constructor received arguments",
            )),
        },
        "core.option.get-or" => match arguments.as_slice() {
            [NormalizedValue::Option(value), fallback] => Ok(value
                .as_deref()
                .cloned()
                .unwrap_or_else(|| fallback.clone())),
            _ => Err(reference_type_error(
                "option fallback received foreign values",
            )),
        },
        "core.option.present" => match arguments.as_slice() {
            [NormalizedValue::Option(value)] => Ok(NormalizedValue::Bool(value.is_some())),
            _ => Err(reference_type_error(
                "option predicate received a foreign value",
            )),
        },
        "core.map.length" => match arguments.as_slice() {
            [NormalizedValue::Map(values)] => {
                Ok(NormalizedValue::I64(reference_length(values.len())?))
            }
            _ => Err(reference_type_error("map length received a foreign value")),
        },
        "core.map.get" | "core.map.contains" | "core.map.get-or" | "core.map.insert"
        | "core.map.remove" | "core.map.entries" => {
            reference_map_intrinsic(implementation, arguments, control)
        }
        _ => Err(reference_error(
            "normalized_reference_intrinsic_missing",
            "reference host has no implementation for the exact external declaration",
        )),
    }
}

fn reference_parse_i64(value: &str) -> Option<i64> {
    let negative = value.as_bytes().first() == Some(&b'-');
    let digits = if negative { value.get(1..)? } else { value };
    if digits.is_empty()
        || digits.bytes().any(|byte| !byte.is_ascii_digit())
        || (digits.len() > 1 && digits.as_bytes().first() == Some(&b'0'))
        || (negative && digits == "0")
    {
        None
    } else {
        value.parse::<i64>().ok()
    }
}

fn reference_length(length: usize) -> Result<i64, ExecutionError> {
    i64::try_from(length).map_err(|_| {
        reference_resource(
            "reference_value_length",
            "value length exceeds signed 64-bit range",
        )
    })
}

fn reference_structural_record(
    fields: Vec<(&str, NormalizedValue)>,
) -> Result<NormalizedValue, ExecutionError> {
    let mut output = Vec::with_capacity(fields.len());
    for (name, value) in fields {
        let name = Name::new(name.to_owned()).map_err(|_| {
            reference_error(
                "reference_intrinsic_field",
                "intrinsic field name is invalid",
            )
        })?;
        output.push((name, value));
    }
    output.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    Ok(NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(output),
    }))
}

fn reference_record_field<'a>(
    program: &dyn NormalizedValueSchema,
    record: &'a NormalizedRecord,
    name: &str,
) -> Option<&'a NormalizedValue> {
    match record {
        NormalizedRecord::Structural { fields } => {
            for (candidate, value) in fields.iter() {
                if candidate.as_str() == name {
                    return Some(value);
                }
            }
            None
        }
        NormalizedRecord::Nominal { layout, fields } => {
            let layout = program.records().get(usize::try_from(layout.0).ok()?)?;
            for (index, field) in layout.fields.iter().enumerate() {
                if field.name.as_str() == name {
                    return fields.get(index);
                }
            }
            None
        }
    }
}

fn reference_bearer_token(
    program: &dyn NormalizedValueSchema,
    arguments: &[NormalizedValue],
) -> Result<NormalizedValue, ExecutionError> {
    let [NormalizedValue::List(headers)] = arguments else {
        return Err(reference_type_error(
            "bearer-token extraction received a foreign value",
        ));
    };
    let mut found: Option<String> = None;
    for header in headers.iter() {
        let NormalizedValue::Record(record) = header else {
            return Err(reference_type_error("bearer-token header is not a record"));
        };
        let Some(NormalizedValue::Text(name)) = reference_record_field(program, record, "name")
        else {
            return Err(reference_type_error("bearer-token header name is foreign"));
        };
        let Some(NormalizedValue::Bytes(value)) = reference_record_field(program, record, "value")
        else {
            return Err(reference_type_error("bearer-token header value is foreign"));
        };
        if name.to_ascii_lowercase() != "authorization" {
            continue;
        }
        if found.is_some() {
            return Ok(NormalizedValue::text(String::new()));
        }
        let value = match std::str::from_utf8(value) {
            Ok(value) => value,
            Err(_) => return Ok(NormalizedValue::text(String::new())),
        };
        if !value.starts_with("Bearer ") {
            return Ok(NormalizedValue::text(String::new()));
        }
        let token = &value[7..];
        if token.is_empty()
            || token.len() > 512
            || token.chars().any(|character| !character.is_ascii_graphic())
        {
            return Ok(NormalizedValue::text(String::new()));
        }
        found = Some(token.to_owned());
    }
    Ok(NormalizedValue::text(found.unwrap_or_default()))
}

fn reference_media_type_matches(value: &[u8], expected: &str) -> bool {
    if !reference_media_token_pair(expected.as_bytes())
        || expected.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return false;
    }
    let Ok(value) = std::str::from_utf8(value) else {
        return false;
    };
    let mut sections = value.split(';');
    let Some(base) = sections.next().map(str::trim) else {
        return false;
    };
    if !reference_media_token_pair(base.as_bytes()) || !base.eq_ignore_ascii_case(expected) {
        return false;
    }
    for parameter in sections {
        let Some((name, value)) = parameter.trim().split_once('=') else {
            return false;
        };
        let name = name.trim();
        let value = value.trim();
        if name.is_empty()
            || !name.bytes().all(reference_http_token)
            || !reference_media_parameter_value(value)
        {
            return false;
        }
    }
    true
}

fn reference_media_token_pair(value: &[u8]) -> bool {
    let mut parts = value.split(|byte| *byte == b'/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(left), Some(right), None) => {
            !left.is_empty()
                && !right.is_empty()
                && left.iter().copied().all(reference_http_token)
                && right.iter().copied().all(reference_http_token)
        }
        _ => false,
    }
}

fn reference_media_parameter_value(value: &str) -> bool {
    if let Some(inner) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        if inner.is_empty() {
            return true;
        }
        let mut escaped = false;
        for byte in inner.bytes() {
            if escaped {
                if !matches!(byte, b'\t' | b' '..=b'~') {
                    return false;
                }
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' || !matches!(byte, b'\t' | b' '..=b'~') {
                return false;
            }
        }
        !escaped
    } else {
        !value.is_empty() && value.bytes().all(reference_http_token)
    }
}

fn reference_http_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn reference_map_intrinsic(
    implementation: &str,
    arguments: Vec<NormalizedValue>,
    control: &ExecutionControl,
) -> Result<NormalizedValue, ExecutionError> {
    let mut arguments = super::value::RawArguments::new(arguments);
    let entries = match arguments.next() {
        Some(NormalizedValue::Map(entries)) => entries,
        Some(value) => {
            super::value::release_raw_value(value);
            return Err(reference_type_error("map intrinsic received a foreign map"));
        }
        None => return Err(reference_type_error("map intrinsic received a foreign map")),
    };
    let mut reserve = super::value::raw_map_reservation(control);
    if implementation == "core.map.entries" {
        if !arguments.is_empty() {
            return Err(reference_type_error("map entries received a foreign arity"));
        }
        reserve(super::map::Charge {
            slots: 0,
            bytes: super::value::collection_storage_bytes(
                entries.len() as u64,
                std::mem::size_of::<NormalizedValue>() as u64,
                "normalized_map_storage",
            )?,
        })?;
        let mut output = Vec::with_capacity(entries.len());
        for (key, value) in entries.iter() {
            reserve(super::map::Charge {
                slots: 2,
                bytes: (2 * std::mem::size_of::<(Name, NormalizedValue)>()
                    + 2 * std::mem::size_of::<(&str, NormalizedValue)>()
                    + std::mem::size_of::<Vec<(Name, NormalizedValue)>>()
                    + 2 * std::mem::size_of::<usize>()
                    + 8) as u64,
            })?;
            reserve(super::map::Charge {
                slots: 0,
                bytes: super::value::projected_value_bytes(value)?,
            })?;
            output.push(reference_structural_record(vec![
                ("key", key.to_value()),
                ("value", value.clone()),
            ])?);
        }
        return super::list::List::from_items(
            output,
            super::value::MAXIMUM_ADMISSION_ITEMS,
            &mut |charge| {
                reserve(super::map::Charge {
                    slots: charge.slots,
                    bytes: charge.bytes,
                })
            },
        )
        .map(NormalizedValue::List);
    }
    let Some(key_value) = arguments.next() else {
        return Err(reference_type_error("map intrinsic omitted its key"));
    };
    let key_value = super::value::RawValue::new(key_value);
    reserve(super::map::Charge::default())?;
    let Some(key) = NormalizedMapKey::from_value(key_value.into_raw()) else {
        return Err(reference_trap(
            "reference_map_key",
            "map key is not a deterministically ordered primitive",
        ));
    };
    let value = arguments.next().map(super::value::RawValue::new);
    if !arguments.is_empty() {
        return Err(reference_type_error(
            "map intrinsic received a foreign arity",
        ));
    }
    let mut project = |value: &NormalizedValue| {
        reserve(super::map::Charge {
            slots: 0,
            bytes: super::value::projected_value_bytes(value)?,
        })?;
        Ok(value.clone())
    };
    let result = match (implementation, value) {
        ("core.map.get", None) => project(entries.get(&key).ok_or_else(|| {
            reference_trap("reference_map_key_absent", "map lookup key is absent")
        })?),
        ("core.map.contains", None) => Ok(NormalizedValue::Bool(entries.get(&key).is_some())),
        ("core.map.get-or", Some(fallback)) => match entries.get(&key) {
            Some(value) => project(value),
            None => Ok(fallback.into_raw()),
        },
        ("core.map.insert", Some(value)) => entries
            .insert(
                key,
                value.into_raw(),
                super::value::MAXIMUM_ADMISSION_ITEMS,
                &mut reserve,
            )
            .map(NormalizedValue::Map),
        ("core.map.remove", None) => entries.remove(&key, &mut reserve).map(NormalizedValue::Map),
        _ => Err(reference_type_error(
            "map intrinsic received a foreign arity",
        )),
    };
    let result = super::value::RawValue::new(result?);
    control.check()?;
    Ok(result.into_raw())
}

fn reference_json_error(error: crate::platform::diagnostic::Diagnostic) -> ExecutionError {
    let class = match error.class {
        crate::platform::diagnostic::DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        crate::platform::diagnostic::DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        crate::platform::diagnostic::DiagnosticClass::Semantic
            if error.code == "normalized_json_nonfinite" =>
        {
            ExecutionFailureClass::Trap
        }
        _ => ExecutionFailureClass::Infrastructure,
    };
    ExecutionError::new(class, error.code, "typed JSON operation failed")
}

pub(crate) fn reference_equal(
    left: &NormalizedValue,
    right: &NormalizedValue,
) -> Result<bool, ExecutionError> {
    reference_compare(left, right, false)
}

pub(crate) fn reference_observation_equal(
    left: &NormalizedValue,
    right: &NormalizedValue,
) -> Result<bool, ExecutionError> {
    reference_compare(left, right, true)
}

fn reference_compare(
    left: &NormalizedValue,
    right: &NormalizedValue,
    observation: bool,
) -> Result<bool, ExecutionError> {
    match (left, right) {
        (NormalizedValue::Unit, NormalizedValue::Unit) => Ok(true),
        (NormalizedValue::Bool(left), NormalizedValue::Bool(right)) => Ok(left == right),
        (NormalizedValue::I64(left), NormalizedValue::I64(right)) => Ok(left == right),
        (NormalizedValue::F64(left), NormalizedValue::F64(right)) => {
            if observation {
                Ok(left.bits() == right.bits())
            } else {
                Ok(left.to_float() == right.to_float())
            }
        }
        (NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)) => Ok(left == right),
        (NormalizedValue::Text(left), NormalizedValue::Text(right))
        | (NormalizedValue::StaticText(left), NormalizedValue::StaticText(right)) => {
            Ok(left == right)
        }
        (
            NormalizedValue::Record(NormalizedRecord::Nominal {
                layout: left_layout,
                fields: left,
            }),
            NormalizedValue::Record(NormalizedRecord::Nominal {
                layout: right_layout,
                fields: right,
            }),
        ) => {
            let contents = reference_equal_sequence(left, right, observation)?;
            Ok(left_layout == right_layout && contents)
        }
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
            NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
        ) => {
            let mut equal = left.len() == right.len();
            for ((left_name, left), (right_name, right)) in left.iter().zip(right.iter()) {
                let values = reference_compare(left, right, observation)?;
                equal &= left_name == right_name && values;
            }
            for (_, value) in left
                .iter()
                .skip(right.len())
                .chain(right.iter().skip(left.len()))
            {
                reference_compare(value, value, observation)?;
            }
            Ok(equal)
        }
        (
            NormalizedValue::Variant {
                layout: left_layout,
                case: left_case,
                payload: left,
            },
            NormalizedValue::Variant {
                layout: right_layout,
                case: right_case,
                payload: right,
            },
        ) => {
            let payloads =
                reference_optional_equality(left.as_deref(), right.as_deref(), observation)?;
            Ok(left_layout == right_layout && left_case == right_case && payloads)
        }
        (NormalizedValue::Option(left), NormalizedValue::Option(right)) => {
            reference_optional_equality(left.as_deref(), right.as_deref(), observation)
        }
        (
            NormalizedValue::Result {
                success: left_case,
                value: left,
            },
            NormalizedValue::Result {
                success: right_case,
                value: right,
            },
        ) => {
            let contents = reference_compare(left, right, observation)?;
            Ok(left_case == right_case && contents)
        }
        (NormalizedValue::List(left), NormalizedValue::List(right)) => {
            reference_list_compare(left, right, observation)
        }
        (NormalizedValue::Map(left), NormalizedValue::Map(right)) => {
            reference_map_compare(left, right, observation)
        }
        (NormalizedValue::Function { .. }, _) | (_, NormalizedValue::Function { .. }) => {
            Err(reference_trap(
                "normalized_reference_value_not_comparable",
                "functions do not support semantic equality",
            ))
        }
        (NormalizedValue::Resource(_), _) | (_, NormalizedValue::Resource(_)) => {
            Err(reference_trap(
                "normalized_reference_value_not_comparable",
                "live resources do not support semantic equality",
            ))
        }
        (
            NormalizedValue::ByteBuffer(_)
            | NormalizedValue::OwnedI64Cell(_)
            | NormalizedValue::OwnedProduct(_)
            | NormalizedValue::OwnedChoice(_)
            | NormalizedValue::OwnedSequence(_),
            _,
        )
        | (
            _,
            NormalizedValue::ByteBuffer(_)
            | NormalizedValue::OwnedI64Cell(_)
            | NormalizedValue::OwnedProduct(_)
            | NormalizedValue::OwnedChoice(_)
            | NormalizedValue::OwnedSequence(_),
        ) => Err(reference_trap(
            "normalized_reference_value_not_comparable",
            "owned memory does not support semantic equality",
        )),
        _ => {
            reference_compare(left, left, observation)?;
            reference_compare(right, right, observation)?;
            Ok(false)
        }
    }
}

// List adapters also carry traversal storage. Keep it outside nested map frames,
// while checking every unmatched tail value for comparability as before.
#[inline(never)]
fn reference_list_compare(
    left: &super::list::List,
    right: &super::list::List,
    observation: bool,
) -> Result<bool, ExecutionError> {
    let mut equal = left.len() == right.len();
    for (left, right) in left.iter().zip(right.iter()) {
        equal &= reference_compare(left, right, observation)?;
    }
    for value in left
        .iter()
        .skip(right.len())
        .chain(right.iter().skip(left.len()))
    {
        reference_compare(value, value, observation)?;
    }
    Ok(equal)
}

// This evaluator independently preserves complete comparability checks even
// after an unequal prefix. Outline only the cursor storage, not its validator.
#[inline(never)]
fn reference_map_compare(
    left: &super::map::Map,
    right: &super::map::Map,
    observation: bool,
) -> Result<bool, ExecutionError> {
    let mut equal = left.len() == right.len();
    let mut cursor = left.iter();
    for (key, value) in &mut cursor {
        let values = match right.get(key) {
            Some(other) => reference_compare(value, other, observation)?,
            None => {
                reference_compare(value, value, observation)?;
                false
            }
        };
        equal &= values;
    }
    // Assignment from right.iter() reserves another full inline buffer in debug
    // frames, even though the first walk has ended. Restart the existing storage.
    cursor.restart(right);
    for (key, value) in &mut cursor {
        if !left.contains_key(key) {
            reference_compare(value, value, observation)?;
            equal = false;
        }
    }
    Ok(equal)
}

fn reference_optional_equality(
    left: Option<&NormalizedValue>,
    right: Option<&NormalizedValue>,
    observation: bool,
) -> Result<bool, ExecutionError> {
    if let (Some(left), Some(right)) = (left, right) {
        return reference_compare(left, right, observation);
    }
    for value in left.into_iter().chain(right) {
        reference_compare(value, value, observation)?;
    }
    Ok(left.is_none() && right.is_none())
}

fn reference_equal_sequence(
    left: &[NormalizedValue],
    right: &[NormalizedValue],
    observation: bool,
) -> Result<bool, ExecutionError> {
    let mut equal = left.len() == right.len();
    for (left, right) in left.iter().zip(right.iter()) {
        equal &= reference_compare(left, right, observation)?;
    }
    for value in left
        .iter()
        .skip(right.len())
        .chain(right.iter().skip(left.len()))
    {
        reference_compare(value, value, observation)?;
    }
    Ok(equal)
}

fn reference_f64_pair(arguments: &[NormalizedValue]) -> Result<(f64, f64), ExecutionError> {
    match arguments {
        [NormalizedValue::F64(left), NormalizedValue::F64(right)] => {
            Ok((left.to_float(), right.to_float()))
        }
        _ => Err(reference_type_error(
            "binary F64 operation requires two F64 values",
        )),
    }
}

fn reference_f64_argument(arguments: &[NormalizedValue]) -> Result<f64, ExecutionError> {
    match arguments {
        [NormalizedValue::F64(number)] => Ok(number.to_float()),
        _ => Err(reference_type_error(
            "unary F64 operation requires one F64 value",
        )),
    }
}

fn reference_unary_f64(
    arguments: &[NormalizedValue],
    operation: fn(f64) -> f64,
) -> Result<NormalizedValue, ExecutionError> {
    let answer = operation(reference_f64_argument(arguments)?);
    Ok(NormalizedValue::F64(Binary64::from_float(answer)))
}

fn reference_f64_to_i64(number: f64) -> Option<i64> {
    if !number.is_finite()
        || number < -9_223_372_036_854_775_808.0
        || number >= 9_223_372_036_854_775_808.0
    {
        return None;
    }
    Some(number.trunc() as i64)
}

fn reference_binary_i64(
    arguments: Vec<NormalizedValue>,
    operation: fn(i64, i64) -> Option<i64>,
    code: &'static str,
    message: &'static str,
) -> Result<NormalizedValue, ExecutionError> {
    let (left, right) = reference_i64_pair(arguments)?;
    operation(left, right)
        .map(NormalizedValue::I64)
        .ok_or_else(|| reference_trap(code, message))
}

fn reference_i64_pair(arguments: Vec<NormalizedValue>) -> Result<(i64, i64), ExecutionError> {
    let [NormalizedValue::I64(left), NormalizedValue::I64(right)] = arguments.as_slice() else {
        return Err(reference_type_error(
            "integer intrinsic received a foreign value",
        ));
    };
    Ok((*left, *right))
}

fn reference_value_cost(value: &NormalizedValue) -> Result<(u64, u64), ExecutionError> {
    let mut pending = vec![value];
    let mut bytes = 0_u64;
    let mut items = 0_u64;
    while let Some(value) = pending.pop() {
        match value {
            NormalizedValue::ByteBuffer(_)
            | NormalizedValue::OwnedI64Cell(_)
            | NormalizedValue::OwnedProduct(_)
            | NormalizedValue::OwnedChoice(_)
            | NormalizedValue::OwnedSequence(_) => {
                return Err(ExecutionError::resource(
                    "normalized_buffer_boundary",
                    "ByteBuffer cannot cross raw/adapter boundaries",
                ));
            }
            NormalizedValue::Bytes(value) => {
                bytes = bytes.checked_add(value.len() as u64).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external value byte accounting overflowed",
                    )
                })?;
            }
            NormalizedValue::Text(value) | NormalizedValue::StaticText(value) => {
                bytes = bytes.checked_add(value.len() as u64).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external text byte accounting overflowed",
                    )
                })?;
            }
            NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) => {
                items = items.checked_add(fields.len() as u64).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external record item accounting overflowed",
                    )
                })?;
                pending.extend(fields.iter());
            }
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                items = items.checked_add(fields.len() as u64).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external structural-record item accounting overflowed",
                    )
                })?;
                for (name, value) in fields.iter() {
                    bytes = bytes
                        .checked_add(name.as_str().len() as u64)
                        .ok_or_else(|| {
                            reference_resource(
                                "normalized_reference_external_value",
                                "external structural-name accounting overflowed",
                            )
                        })?;
                    pending.push(value);
                }
            }
            NormalizedValue::Variant { payload, .. } => {
                if let Some(payload) = payload {
                    items = items.checked_add(1).ok_or_else(|| {
                        reference_resource(
                            "normalized_reference_external_value",
                            "external variant item accounting overflowed",
                        )
                    })?;
                    pending.push(payload);
                }
            }
            NormalizedValue::Option(value) => {
                if let Some(value) = value {
                    items = items.checked_add(1).ok_or_else(|| {
                        reference_resource(
                            "normalized_reference_external_value",
                            "external option item accounting overflowed",
                        )
                    })?;
                    pending.push(value);
                }
            }
            NormalizedValue::Result { value, .. } => {
                items = items.checked_add(1).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external result accounting overflowed",
                    )
                })?;
                pending.push(value);
            }
            NormalizedValue::List(values) => {
                items = items.checked_add(values.len() as u64).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external list item accounting overflowed",
                    )
                })?;
                pending.extend(values.iter());
            }
            NormalizedValue::Map(values) => {
                items = items.checked_add(values.len() as u64).ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_external_value",
                        "external map item accounting overflowed",
                    )
                })?;
                for (key, value) in values.iter() {
                    bytes = bytes
                        .checked_add(reference_map_key_bytes(key))
                        .ok_or_else(|| {
                            reference_resource(
                                "normalized_reference_external_value",
                                "external map-key accounting overflowed",
                            )
                        })?;
                    pending.push(value);
                }
            }
            NormalizedValue::Unit
            | NormalizedValue::Bool(_)
            | NormalizedValue::I64(_)
            | NormalizedValue::F64(_)
            | NormalizedValue::Function { .. }
            | NormalizedValue::Resource(_) => {}
        }
    }
    Ok((bytes, items))
}

fn reference_map_key_bytes(key: &NormalizedMapKey) -> u64 {
    match key {
        NormalizedMapKey::Bytes(value) => value.len() as u64,
        NormalizedMapKey::Text(value) => value.len() as u64,
        NormalizedMapKey::Bool(_) | NormalizedMapKey::I64(_) => 0,
    }
}

fn validate_reference_policy(policy: NormalizedRunPolicy) -> Result<(), ExecutionError> {
    if policy.instruction_steps == Some(0)
        || policy.maximum_call_depth == 0
        || policy.maximum_value_stack == 0
        || policy.maximum_allocated_bytes == Some(0)
        || policy.maximum_collection_items == Some(0)
        || policy.maximum_capability_calls == Some(0)
    {
        return Err(reference_resource(
            "normalized_reference_policy",
            "normalized reference policy dimensions must all be positive",
        ));
    }
    Ok(())
}

fn reference_capabilities_unbound() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_capability_unbound",
        "effectful normalized reference execution requires exact deployment grants",
    )
}

fn reference_type_error(message: impl Into<String>) -> ExecutionError {
    reference_error("normalized_reference_type", message)
}

fn reference_byte_range(bytes: &[u8], start: i64, end: i64) -> Result<&[u8], ExecutionError> {
    // Independent value oracle: signed bounds are checked before any conversion,
    // without using the optimized payload's offset or sharing implementation.
    if start < 0 || end < start || end as u64 > bytes.len() as u64 {
        return Err(reference_trap(
            "reference_bytes_range",
            "byte range is out of bounds",
        ));
    }
    Ok(&bytes[start as usize..end as usize])
}

fn reference_trap(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Trap, code, message)
}

pub(super) fn reference_resource(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::resource(code, message)
}

pub(super) fn reference_error(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn direct_memory_type(
    schema: &BoundReferenceSchema,
    mut ty: TypeObjectDigest,
    substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    control: &ExecutionControl,
) -> Result<Option<super::value::MemoryForm>, ExecutionError> {
    for _ in 0..=substitutions.len() {
        control.check()?;
        match &schema
            .types
            .get(&ty)
            .ok_or_else(|| reference_type_error("missing direct memory type metadata"))?
            .form
        {
            TypeForm::ByteBuffer => return Ok(Some(super::value::MemoryForm::ByteBuffer)),
            TypeForm::OwnedI64Cell => return Ok(Some(super::value::MemoryForm::OwnedI64Cell)),
            TypeForm::OwnedSequence { .. } => {
                return Ok(Some(super::value::MemoryForm::Sequence(
                    schema.transfer_type_identity(ty, substitutions, control)?,
                )));
            }
            TypeForm::OwnedChoice { .. } => {
                return Ok(Some(super::value::MemoryForm::Choice(
                    schema.transfer_type_identity(ty, substitutions, control)?,
                )));
            }
            TypeForm::OwnedProduct { .. } => {
                return Ok(Some(super::value::MemoryForm::Product(
                    schema.transfer_type_identity(ty, substitutions, control)?,
                )));
            }
            TypeForm::TypeParameter { parameter } => {
                ty = *substitutions
                    .get(parameter)
                    .ok_or_else(|| reference_type_error("unbound memory type parameter"))?;
            }
            _ => return Ok(None),
        }
    }
    Err(reference_type_error("cyclic memory type substitution"))
}
