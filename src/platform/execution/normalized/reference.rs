//! Implementation-disjoint evaluator over canonical Graph 14 owner and expression records.

use super::capability::{
    NormalizedCapabilities, NormalizedCapabilityTransaction, validate_outcome,
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
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::JsonLimits;
use crate::platform::kernel::{
    BindingKind, BlobObjectDigest, CaseReference, DeclarationPayload, DeclarationReference,
    EffectParameterReference, EffectRow, ExpressionOperation, FieldReference, FieldSelector,
    FunctionDeclaration, FunctionEffect, ImplementationName, KernelSnapshot, LocalValueReference,
    Name, OperationReference, OwnerKey, OwnerRecord, PackageId, ParameterRecord, ParameterUse,
    PortImplementation, RequirementReference, SemanticStateDigest, TextValue, TypeForm,
    TypeObjectDigest,
};
use crate::platform::semantic_id::{
    BindingId, ExpressionId, ParameterId, RepositoryId, RevisionId, TypeParameterId,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[path = "reference_checked.rs"]
mod checked;
use checked::{Ownership, Value as CheckedValue};
#[path = "reference_intrinsics.rs"]
mod checked_intrinsics;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NormalizedReferenceObservation {
    pub expressions: u64,
    pub calls: u64,
    pub external_calls: u64,
    pub capability_calls: u64,
    pub allocated_bytes: u64,
    pub allocation_charges: u64,
    pub type_derivation_steps: u64,
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
        control.check()?;
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
    effect_parameters: Vec<crate::platform::semantic_id::EffectParameterId>,
    effect: FunctionEffect,
    type_parameters: Vec<TypeParameterId>,
    type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
    parameters: Vec<ParameterRecord>,
    result: TypeObjectDigest,
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
            state.call_declaration(declaration, &[], &[], arguments)
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
            state.call_declaration(declaration, types, &[], arguments)
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
                    state.call_declaration(function, &[], &[], arguments)
                }
                PortImplementation::Expression(expression) => {
                    let arguments =
                        state.admit_port_arguments(port.function_type, arguments.into_vec())?;
                    let callee = state.evaluate(expression, &mut BTreeMap::new())?;
                    let (declaration, type_arguments, effect_arguments, arguments) =
                        state.callable_arguments(callee, arguments)?;
                    state.call_declaration(
                        declaration,
                        &type_arguments,
                        &effect_arguments,
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
        let type_metadata_bytes = schema.type_metadata_bytes;
        let list_work = super::list::Work::current();
        let mut state = ReferenceState {
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
            local_counts: Vec::new(),
            next_transaction: 0,
            transactions: BTreeMap::new(),
            calls_by_requirement: BTreeMap::new(),
            type_scopes: Vec::new(),
            effect_scopes: Vec::new(),
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
                type_metadata_bytes,
                value_work: super::value::ValueWork::default(),
                collection_items: 0,
                maximum_call_depth: 0,
                canonical_owner_reads: schema_work.owner_reads,
                canonical_map_pages_read: schema_work.map_pages_read,
                canonical_objects_read: schema_work.objects_read,
                canonical_bytes_read: schema_work.bytes_read,
                production_tier: "graph14_reference_records_9",
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
        if let Some(observer) = self.observer {
            let mut observed = observer.lock().map_err(|_| {
                reference_error(
                    "normalized_reference_observation_lock",
                    "execution observation lock is poisoned",
                )
            })?;
            *observed = Some(state.observation.clone());
        }
        result.map(|value| (value.release(), state.observation))
    }
}

struct ReferenceTransaction {
    binding: BindingId,
    generation: u64,
    transaction: Box<dyn NormalizedCapabilityTransaction>,
}

struct ReferenceState<'a> {
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
    local_counts: Vec<usize>,
    next_transaction: u64,
    transactions: BTreeMap<RequirementReference, ReferenceTransaction>,
    calls_by_requirement: BTreeMap<RequirementReference, u64>,
    type_scopes: Vec<BTreeMap<TypeParameterId, TypeObjectDigest>>,
    effect_scopes: Vec<super::reference_effects::Bindings>,
    allowances: Vec<Option<EffectRow>>,
    root_allowance: Option<EffectRow>,
    observation: NormalizedReferenceObservation,
}

enum ReferenceStep {
    Value(CheckedValue),
    Tail(Box<AdmittedGraphCall>),
}

/// One internal transition, constructed only by canonical call admission in this state.
/// It is neither a callable value nor a reusable proof, and carries no component grant.
struct AdmittedGraphCall {
    declaration: DeclarationReference,
    function: FunctionDeclaration,
    types: BTreeMap<TypeParameterId, TypeObjectDigest>,
    effects: super::reference_effects::Bindings,
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
    ) -> Result<EffectRow, ExecutionError> {
        super::reference_effects::close(row, scope, |count| {
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
        let result = arguments
            .iter()
            .map(|row| self.close_row(row, &scope))
            .collect();
        // Keep the exact caller scope even when closing an argument fails.
        if had_scope {
            self.effect_scopes.push(scope);
        }
        result
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
            let closed = self.close_row(row, &BTreeMap::new())?;
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
        for requirement in &required.requirements {
            let capabilities = self
                .capabilities
                .ok_or_else(reference_capabilities_unbound)?;
            let grant = capabilities.canonical_requirement_exact(self.program, *requirement)?;
            let mut found = false;
            for candidate in &available {
                self.control.check()?;
                if (candidate == requirement
                    || self.reference_requirement_covers(*requirement, *candidate)?)
                    && capabilities.canonical_requirement_exact(self.program, *candidate)? == grant
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
            .is_none_or(|row| !row.requirements.contains(&requirement))
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
            if !function.effect_parameters.is_empty() {
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
    ) -> Result<ReferenceSignature, ExecutionError> {
        let mut signature = self.function_signature(reference)?;
        if signature.type_parameters.len() != types.len() {
            return Err(reference_type_error(
                "callable type argument arity differs from its exact target",
            ));
        }
        let scope = self.effect_bindings(reference, &signature.effect_parameters, effects)?;
        let declared = self.declared_row(&signature.effect)?;
        let row = self.close_row(&declared, &scope)?;
        if !signature.pure {
            signature.effect = FunctionEffect::Task {
                requirements: row.requirements,
                effect_parameters: Vec::new(),
            };
        }
        // Zero-effect-arity signatures retain their canonical ordinary-type templates.
        // Raw argument/prefix admission substitutes those templates structurally, including
        // valid pure applications whose aggregate digest was not a prepared graph root.
        if scope.is_empty() {
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
                .instantiated_with_effects(*ty, &bindings, &scope, 0)
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

    fn resolve_type_arguments(
        &self,
        type_arguments: &[TypeObjectDigest],
    ) -> Result<Vec<TypeObjectDigest>, ExecutionError> {
        let empty = BTreeMap::new();
        let substitutions = self.type_scopes.last().unwrap_or(&empty);
        let empty_effects = BTreeMap::new();
        let effects = self.effect_scopes.last().unwrap_or(&empty_effects);
        type_arguments
            .iter()
            .map(|ty| {
                self.schema
                    .instantiated_with_effects(*ty, substitutions, effects, 0)
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
        arguments: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.control.check()?;
        let type_arguments = self.resolve_type_arguments(type_arguments)?;
        let effect_arguments = self.resolve_effect_arguments(effect_arguments)?;
        if self.call_depth >= self.policy.maximum_call_depth {
            return Err(reference_resource(
                "normalized_reference_call_depth",
                "reference execution exceeded its call-depth budget",
            ));
        }
        self.call_depth += 1;
        self.observation.maximum_call_depth =
            self.observation.maximum_call_depth.max(self.call_depth);
        let previous_package = self.active_package;
        self.active_package = reference.package;
        let mut step =
            self.call_activation(reference, &type_arguments, &effect_arguments, arguments);
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
                        arguments,
                    )?;
                    self.enter_graph_call(target, false)
                }
                DeclarationPayload::External(external) => {
                    self.count_call(arguments.len())?;
                    let parameters = self.parameters(reference.package, &external.parameters)?;
                    self.validate_call_resources(&parameters, &arguments)?;
                    if type_arguments.len() != external.type_parameters.len()
                        || !effect_arguments.is_empty()
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
                            effect_parameters: Vec::new(),
                            effect: FunctionEffect::Pure,
                            type_parameter_constraints: self
                                .type_parameter_constraints(reference, &external.type_parameters)?,
                            type_parameters: external.type_parameters,
                            parameters,
                            result: external.result,
                            pure: true,
                        };
                        self.observation.external_calls =
                            self.observation.external_calls.saturating_add(1);
                        let value = if let Some(host) = self.host {
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
        arguments: Vec<CheckedValue>,
    ) -> Result<AdmittedGraphCall, ExecutionError> {
        self.control.check()?;
        if arguments.len() != function.parameters.len()
            || types.len() != function.type_parameters.len()
            || effects.len() != function.effect_parameters.len()
        {
            return Err(reference_type_error(
                "call application disagrees with its exact canonical signature",
            ));
        }
        let constraints =
            self.type_parameter_constraints(declaration, &function.type_parameters)?;
        if constraints.iter().zip(types).any(|(constraint, ty)| {
            *constraint == crate::platform::kernel::TypeParameterConstraints::CaptureSafe
                && !self.schema.capture_safe_types.contains(ty)
        }) {
            return Err(reference_type_error(
                "canonical callable type arguments fail capture-safe constraints",
            ));
        }
        let effect_scope =
            self.effect_bindings(declaration, &function.effect_parameters, effects)?;
        let declared = self.declared_row(&function.effect)?;
        let row = self.close_row(&declared, &effect_scope)?;
        let allowance = if matches!(function.effect, FunctionEffect::Pure) {
            None
        } else {
            // This must execute before the outgoing allowance is popped, even for empty rows.
            self.admit_task_row(&row)?;
            Some(row)
        };
        let parameters = self.parameters(declaration.package, &function.parameters)?;
        self.validate_call_resources(&parameters, &arguments)?;
        let types = function
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect::<BTreeMap<_, _>>();
        if types.len() != function.type_parameters.len() {
            return Err(reference_type_error(
                "function type parameters are not unique",
            ));
        }
        Ok(AdmittedGraphCall {
            declaration,
            function,
            types,
            effects: effect_scope,
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
        let mut locals = target
            .function
            .parameters
            .into_iter()
            .zip(target.arguments)
            .map(|(parameter, value)| (LocalValueReference::FunctionParameter(parameter), value))
            .collect::<BTreeMap<_, _>>();
        self.control.check()?;
        self.active_package = target.declaration.package;
        self.type_scopes.push(target.types);
        self.effect_scopes.push(target.effects);
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
        let result = self.evaluate_tail(target.function.body, &mut locals);
        self.local_counts.pop();
        self.type_scopes.pop();
        self.effect_scopes.pop();
        self.allowances.pop();
        result
    }

    fn validate_call_resources(
        &mut self,
        parameters: &[ParameterRecord],
        arguments: &[CheckedValue],
    ) -> Result<(), ExecutionError> {
        for (index, (parameter, argument)) in parameters.iter().zip(arguments).enumerate() {
            match parameter.resource_requirement {
                Some(requirement) => {
                    if index.saturating_add(1) != parameters.len()
                        || parameter.use_mode != ParameterUse::Consume
                        || argument.ownership(&self.schema, &mut self.observation.value_work)?
                            != Ownership::Capability
                    {
                        return Err(reference_error(
                            "normalized_reference_resource_call_shape",
                            "resource-bearing call does not use one final consume parameter and direct handle",
                        ));
                    }
                    let NormalizedValue::Resource(handle) = argument.raw() else {
                        return Err(reference_error(
                            "normalized_reference_resource_call_value",
                            "resource-bearing call argument is not one exact runtime handle",
                        ));
                    };
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
                    self.resources.validate_queue_lease_transfer(
                        requirement,
                        record.interface,
                        *handle,
                    )?;
                }
                None => {
                    if parameter.use_mode != ParameterUse::Unrestricted
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
                ExpressionOperation::Call {
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                } => {
                    let uses = self.function_parameter_uses(function)?;
                    let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                    return self.tail_step(function, &type_arguments, &effect_arguments, arguments);
                }
                ExpressionOperation::Invoke { callee, arguments } => {
                    let callee = self.evaluate(callee, locals)?;
                    let arguments = self.evaluate_many(&arguments, locals)?;
                    let (declaration, type_arguments, effect_arguments, arguments) =
                        self.callable_arguments(callee, arguments)?;
                    return self.tail_step(
                        declaration,
                        &type_arguments,
                        &effect_arguments,
                        arguments,
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
        arguments: Vec<CheckedValue>,
    ) -> Result<ReferenceStep, ExecutionError> {
        let callable = self.declaration(declaration)?;
        if let DeclarationPayload::Function(function) = callable.payload {
            let types = self.resolve_type_arguments(types)?;
            let effects = self.resolve_effect_arguments(effect_arguments)?;
            let target =
                self.admit_graph_call(declaration, function, &types, &effects, arguments)?;
            self.charge_allocation(std::mem::size_of::<AdmittedGraphCall>() as u64)?;
            self.control.check()?;
            Ok(ReferenceStep::Tail(Box::new(target)))
        } else {
            self.call_declaration(declaration, types, effect_arguments, arguments)
                .map(ReferenceStep::Value)
        }
    }

    fn evaluate(
        &mut self,
        expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
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
        self.charge_allocation(
            (std::mem::size_of::<CheckedValue>() - std::mem::size_of::<NormalizedValue>()) as u64,
        )?;
        match operation {
            ExpressionOperation::Unit {} => {
                CheckedValue::primitive(&self.schema, NormalizedValue::Unit)
            }
            ExpressionOperation::Bool { value } => {
                CheckedValue::primitive(&self.schema, NormalizedValue::Bool(value))
            }
            ExpressionOperation::I64 { value } => {
                CheckedValue::primitive(&self.schema, NormalizedValue::I64(value))
            }
            ExpressionOperation::Text { value } => self.text(value).and_then(|value| {
                CheckedValue::primitive(&self.schema, NormalizedValue::Text(value))
            }),
            ExpressionOperation::StaticText { value } => self.text(value).and_then(|value| {
                CheckedValue::primitive(&self.schema, NormalizedValue::StaticText(value))
            }),
            ExpressionOperation::Local { value } => {
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
                self.call_declaration(declaration, &[], &[], Vec::new())
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => match self.evaluate(condition, locals)?.release() {
                NormalizedValue::Bool(true) => self.evaluate(when_true, locals),
                NormalizedValue::Bool(false) => self.evaluate(when_false, locals),
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
                let result = self.evaluate(body, locals);
                for local in scoped {
                    locals.remove(&local);
                }
                result
            }
            ExpressionOperation::Sequence { items } => {
                let mut result = None;
                for item in items {
                    result = Some(self.evaluate(item, locals)?);
                }
                result.ok_or_else(|| {
                    reference_error(
                        "normalized_reference_sequence_empty",
                        "canonical sequence has no result expression",
                    )
                })
            }
            ExpressionOperation::Call {
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => {
                let uses = self.function_parameter_uses(function)?;
                let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
                self.call_declaration(function, &type_arguments, &effect_arguments, arguments)
            }
            ExpressionOperation::FunctionValue {
                effect_arguments,
                function,
                type_arguments,
            } => {
                let signature = self.function_signature(function)?;
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
                let (declaration, type_arguments, effect_arguments, arguments) =
                    self.callable_arguments(callee, arguments)?;
                self.call_declaration(declaration, &type_arguments, &effect_arguments, arguments)
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
                self.record(
                    nominal_type,
                    &self.resolve_type_arguments(&type_arguments)?,
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
                let value = self.evaluate(value, locals)?;
                self.field(value, selector)
            }
            ExpressionOperation::List { items, .. } => {
                let values = self.evaluate_many(&items, locals)?;
                self.charge_items(values.len(), std::mem::size_of::<NormalizedValue>())?;
                self.list_value(values)
            }
            ExpressionOperation::Map { entries, .. } => {
                let mut values = BTreeMap::new();
                let mut key_bytes = 0_u64;
                for entry in entries {
                    let key =
                        NormalizedMapKey::from_value(self.evaluate(entry.key, locals)?.release())
                            .ok_or_else(|| {
                            reference_type_error(
                                "map key is not a deterministically ordered primitive",
                            )
                        })?;
                    let value = self.evaluate(entry.value, locals)?;
                    key_bytes = key_bytes.saturating_add(reference_map_key_bytes(&key));
                    if values.insert(key, value).is_some() {
                        return Err(reference_trap(
                            "normalized_reference_map_duplicate_key",
                            "map expression contains a duplicate key",
                        ));
                    }
                }
                self.charge_items(
                    values.len(),
                    std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                )?;
                self.charge_allocation(key_bytes)?;
                self.map_value(values)
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
                let result = self.evaluate(arm.body, locals);
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
                self.capability_call(requirement, operation, arguments)
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => self.transaction(requirement, binding, body, locals),
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
        let value = if let Some(local) = local {
            match use_mode {
                ParameterUse::Consume => locals.remove(&local),
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
            ParameterUse::Borrow => ownership == Ownership::Capability,
            ParameterUse::Consume => ownership != Ownership::Ordinary,
        };
        if !valid {
            return Err(reference_error(
                "normalized_reference_local_resource_use",
                "canonical parameter use disagrees with its runtime affine value",
            ));
        }
        if let NormalizedValue::Resource(handle) = value.raw() {
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
        let (type_parameters, effect_parameters, effect, parameters, result, pure) =
            match self.declaration(reference)?.payload {
                DeclarationPayload::Function(function) => (
                    function.type_parameters,
                    function.effect_parameters,
                    function.effect.clone(),
                    function.parameters,
                    function.result,
                    matches!(function.effect, FunctionEffect::Pure),
                ),
                DeclarationPayload::External(external) => (
                    external.type_parameters,
                    Vec::new(),
                    FunctionEffect::Pure,
                    external.parameters,
                    external.result,
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
            effect_parameters,
            effect,
            type_parameters,
            type_parameter_constraints,
            parameters: self.parameters(reference.package, &parameters)?,
            result,
            pure,
        })
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
        &self,
        value: CheckedValue,
        selector: FieldSelector,
    ) -> Result<CheckedValue, ExecutionError> {
        value.project_field(selector, &self.schema)
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
    ) -> Result<CheckedValue, ExecutionError> {
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
                transaction.transaction.commit(self.control)?;
                Ok(value)
            }
            Err(error) => {
                let _ = transaction.transaction.rollback();
                Err(error)
            }
        }
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
        self.observation.collection_items = crate::platform::execution::cumulative_charge(
            self.observation.collection_items,
            items as u64,
            self.policy.maximum_collection_items,
            "normalized_reference_collection_items",
            "execution exhausted its collection-item budget",
        )?;
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
        self.observation.allocated_bytes = crate::platform::execution::cumulative_charge(
            self.observation.allocated_bytes,
            bytes,
            self.policy.maximum_allocated_bytes,
            "normalized_reference_allocation",
            "execution exhausted its allocation budget",
        )?;
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
        self.observation.collection_items = crate::platform::execution::cumulative_charge(
            self.observation.collection_items,
            items,
            self.policy.maximum_collection_items,
            "normalized_reference_collection_items",
            "external value exceeds the collection-item budget",
        )?;
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
            reference_map_intrinsic(implementation, arguments)
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
) -> Result<NormalizedValue, ExecutionError> {
    let Some(NormalizedValue::Map(entries)) = arguments.first() else {
        return Err(reference_type_error("map intrinsic received a foreign map"));
    };
    if implementation == "core.map.entries" {
        if arguments.len() != 1 {
            return Err(reference_type_error("map entries received a foreign arity"));
        }
        let mut output = Vec::with_capacity(entries.len());
        for (key, value) in entries.iter() {
            output.push(reference_structural_record(vec![
                ("key", key.to_value()),
                ("value", value.clone()),
            ])?);
        }
        return NormalizedValue::list(output);
    }
    let Some(key_value) = arguments.get(1) else {
        return Err(reference_type_error("map intrinsic omitted its key"));
    };
    let Some(key) = NormalizedMapKey::from_value(key_value.clone()) else {
        return Err(reference_trap(
            "reference_map_key",
            "map key is not a deterministically ordered primitive",
        ));
    };
    match (implementation, arguments.len()) {
        ("core.map.get", 2) => entries
            .get(&key)
            .cloned()
            .ok_or_else(|| reference_trap("reference_map_key_absent", "map lookup key is absent")),
        ("core.map.contains", 2) => Ok(NormalizedValue::Bool(entries.get(&key).is_some())),
        ("core.map.get-or", 3) => match entries.get(&key) {
            Some(value) => Ok(value.clone()),
            None => Ok(arguments[2].clone()),
        },
        ("core.map.insert", 3) => {
            let mut updated = BTreeMap::new();
            for (existing_key, existing_value) in entries.iter() {
                updated.insert(existing_key.clone(), existing_value.clone());
            }
            updated.insert(key, arguments[2].clone());
            Ok(NormalizedValue::Map(Arc::new(updated)))
        }
        ("core.map.remove", 2) => {
            let mut updated = BTreeMap::new();
            for (existing_key, existing_value) in entries.iter() {
                if existing_key != &key {
                    updated.insert(existing_key.clone(), existing_value.clone());
                }
            }
            Ok(NormalizedValue::Map(Arc::new(updated)))
        }
        _ => Err(reference_type_error(
            "map intrinsic received a foreign arity",
        )),
    }
}

fn reference_json_error(error: crate::platform::diagnostic::Diagnostic) -> ExecutionError {
    let class = match error.class {
        crate::platform::diagnostic::DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        crate::platform::diagnostic::DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        _ => ExecutionFailureClass::Infrastructure,
    };
    ExecutionError::new(class, error.code, "typed JSON operation failed")
}

pub(crate) fn reference_equal(
    left: &NormalizedValue,
    right: &NormalizedValue,
) -> Result<bool, ExecutionError> {
    match (left, right) {
        (NormalizedValue::Unit, NormalizedValue::Unit) => Ok(true),
        (NormalizedValue::Bool(left), NormalizedValue::Bool(right)) => Ok(left == right),
        (NormalizedValue::I64(left), NormalizedValue::I64(right)) => Ok(left == right),
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
            let contents = reference_equal_sequence(left, right)?;
            Ok(left_layout == right_layout && contents)
        }
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
            NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
        ) => {
            let mut equal = left.len() == right.len();
            for ((left_name, left), (right_name, right)) in left.iter().zip(right.iter()) {
                let values = reference_equal(left, right)?;
                equal &= left_name == right_name && values;
            }
            for (_, value) in left
                .iter()
                .skip(right.len())
                .chain(right.iter().skip(left.len()))
            {
                reference_equal(value, value)?;
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
            let payloads = reference_optional_equality(left.as_deref(), right.as_deref())?;
            Ok(left_layout == right_layout && left_case == right_case && payloads)
        }
        (NormalizedValue::Option(left), NormalizedValue::Option(right)) => {
            reference_optional_equality(left.as_deref(), right.as_deref())
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
            let contents = reference_equal(left, right)?;
            Ok(left_case == right_case && contents)
        }
        (NormalizedValue::List(left), NormalizedValue::List(right)) => {
            let mut equal = left.len() == right.len();
            for (left, right) in left.iter().zip(right.iter()) {
                equal &= reference_equal(left, right)?;
            }
            for value in left
                .iter()
                .skip(right.len())
                .chain(right.iter().skip(left.len()))
            {
                reference_equal(value, value)?;
            }
            Ok(equal)
        }
        (NormalizedValue::Map(left), NormalizedValue::Map(right)) => {
            let mut equal = left.len() == right.len();
            for (key, left) in left.iter() {
                let values = match right.get(key) {
                    Some(right) => reference_equal(left, right)?,
                    None => {
                        reference_equal(left, left)?;
                        false
                    }
                };
                equal &= values;
            }
            for (key, value) in right.iter() {
                if !left.contains_key(key) {
                    reference_equal(value, value)?;
                    equal = false;
                }
            }
            Ok(equal)
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
        _ => {
            reference_equal(left, left)?;
            reference_equal(right, right)?;
            Ok(false)
        }
    }
}

fn reference_optional_equality(
    left: Option<&NormalizedValue>,
    right: Option<&NormalizedValue>,
) -> Result<bool, ExecutionError> {
    if let (Some(left), Some(right)) = (left, right) {
        return reference_equal(left, right);
    }
    for value in left.into_iter().chain(right) {
        reference_equal(value, value)?;
    }
    Ok(left.is_none() && right.is_none())
}

fn reference_equal_sequence(
    left: &[NormalizedValue],
    right: &[NormalizedValue],
) -> Result<bool, ExecutionError> {
    let mut equal = left.len() == right.len();
    for (left, right) in left.iter().zip(right.iter()) {
        equal &= reference_equal(left, right)?;
    }
    for value in left
        .iter()
        .skip(right.len())
        .chain(right.iter().skip(left.len()))
    {
        reference_equal(value, value)?;
    }
    Ok(equal)
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

fn reference_trap(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Trap, code, message)
}

pub(super) fn reference_resource(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::resource(code, message)
}

pub(super) fn reference_error(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}
