//! Implementation-disjoint evaluator over canonical Graph 5 owner and expression records.

use super::capability::{
    NormalizedCapabilities, NormalizedCapabilityTransaction, validate_outcome,
};
use super::codec::{decode_typed, encode_typed};
use super::prepare::NormalizedProgram;
use super::resource::NormalizedResourceScope;
use super::value::{
    FunctionIndex, NormalizedMapKey, NormalizedRecord, NormalizedValue, RecordLayoutIndex,
    VariantLayoutIndex,
};
use super::vm::NormalizedRunPolicy;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::JsonLimits;
use crate::platform::kernel::{
    BindingKind, CaseReference, DeclarationPayload, DeclarationReference, ExpressionOperation,
    FieldReference, FieldSelector, FunctionEffect, ImplementationName, KernelSnapshot,
    LocalValueReference, Name, OperationReference, OwnerKey, OwnerRecord, PackageId,
    PortImplementation, RequirementReference, SemanticStateDigest, TextValue,
};
use crate::platform::semantic_id::{BindingId, ExpressionId, RepositoryId, RevisionId};
use crate::platform::storage::object::{ImmutableObjectStore, ObjectDomain, ObjectKey, StoreWork};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedReferenceObservation {
    pub expressions: u64,
    pub calls: u64,
    pub external_calls: u64,
    pub capability_calls: u64,
    pub allocated_bytes: u64,
    pub collection_items: u64,
    pub maximum_call_depth: usize,
    pub canonical_owner_reads: u64,
    pub canonical_map_pages_read: u64,
    pub canonical_objects_read: u64,
    pub canonical_bytes_read: u64,
    pub production_tier: &'static str,
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
}

impl NormalizedReferenceRead for KernelSnapshot {
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
        program: &NormalizedProgram,
        function: &super::prepare::NormalizedFunction,
        implementation: &ImplementationName,
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoreNormalizedReferenceHost;

impl NormalizedReferenceHost for CoreNormalizedReferenceHost {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &super::prepare::NormalizedFunction,
        implementation: &ImplementationName,
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        reference_intrinsic(program, function, implementation.as_str(), arguments)
    }
}

static CORE_REFERENCE_HOST: CoreNormalizedReferenceHost = CoreNormalizedReferenceHost;

pub struct NormalizedReferenceInterpreter<'a> {
    authority: &'a dyn NormalizedReferenceRead,
    program: &'a NormalizedProgram,
    policy: NormalizedRunPolicy,
    host: &'a dyn NormalizedReferenceHost,
}

impl<'a> NormalizedReferenceInterpreter<'a> {
    pub fn new(
        snapshot: &'a KernelSnapshot,
        program: &'a NormalizedProgram,
        policy: NormalizedRunPolicy,
    ) -> Self {
        Self::from_reader(snapshot, program, policy)
    }

    pub fn with_host(
        snapshot: &'a KernelSnapshot,
        program: &'a NormalizedProgram,
        policy: NormalizedRunPolicy,
        host: &'a dyn NormalizedReferenceHost,
    ) -> Self {
        Self::with_reader_and_host(snapshot, program, policy, host)
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
            host: &CORE_REFERENCE_HOST,
        }
    }

    pub fn with_reader_and_host(
        authority: &'a dyn NormalizedReferenceRead,
        program: &'a NormalizedProgram,
        policy: NormalizedRunPolicy,
        host: &'a dyn NormalizedReferenceHost,
    ) -> Self {
        Self {
            authority,
            program,
            policy,
            host,
        }
    }

    pub fn invoke(
        &self,
        declaration: DeclarationReference,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        self.execute(capabilities, control, |state| {
            state.call_declaration(declaration, arguments)
        })
    }

    pub fn invoke_root_target(
        &self,
        name: &Name,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let resources = NormalizedResourceScope::new()?;
        self.invoke_root_target_scoped(name, arguments, capabilities, &resources, control)
    }

    pub(crate) fn invoke_root_target_scoped(
        &self,
        name: &Name,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let target = self.program.root_target(name).ok_or_else(|| {
            reference_error(
                "normalized_reference_target_missing",
                "root artifact package has no target with the exact selected name",
            )
        })?;
        let target_id = target.target;
        let expected_name = name.clone();
        self.execute_scoped(capabilities, resources, control, move |state| {
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
                        "selected target is absent from canonical Graph 5 authority",
                    ));
                }
            };
            if record.name != expected_name
                || record.component.package != state.binding.package
                || record.port.package != state.binding.package
            {
                return Err(reference_error(
                    "normalized_reference_target_binding",
                    "selected target disagrees with its exact prepared artifact binding",
                ));
            }
            let port = match state.owner(OwnerKey::Port(record.port.port))? {
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
                        "selected target port is absent from canonical Graph 5 authority",
                    ));
                }
            };
            if port.declaration != record.component.declaration {
                return Err(reference_error(
                    "normalized_reference_port_component",
                    "selected target port belongs to another exact component",
                ));
            }
            match port.implementation {
                PortImplementation::Function(function) => {
                    state.call_declaration(function, arguments)
                }
                PortImplementation::Expression(expression) => {
                    if !arguments.is_empty() {
                        return Err(reference_type_error(
                            "expression-backed target port received arguments",
                        ));
                    }
                    state.evaluate(expression, &mut BTreeMap::new())
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
        operation: impl FnOnce(&mut ReferenceState<'_>) -> Result<NormalizedValue, ExecutionError>,
    ) -> Result<NormalizedReferenceInvocation, ExecutionError> {
        let resources = NormalizedResourceScope::new()?;
        self.execute_scoped(capabilities, &resources, control, operation)
    }

    fn execute_scoped(
        &self,
        capabilities: Option<&NormalizedCapabilities>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
        operation: impl FnOnce(&mut ReferenceState<'_>) -> Result<NormalizedValue, ExecutionError>,
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
        let mut state = ReferenceState {
            authority: self.authority,
            binding,
            active_package: binding.package,
            program: self.program,
            policy: self.policy,
            host: self.host,
            capabilities,
            resources,
            control,
            remaining_expressions: self.policy.instruction_steps,
            call_depth: 0,
            next_transaction: 0,
            transactions: BTreeMap::new(),
            calls_by_requirement: BTreeMap::new(),
            observation: NormalizedReferenceObservation {
                expressions: 0,
                calls: 0,
                external_calls: 0,
                capability_calls: 0,
                allocated_bytes: 0,
                collection_items: 0,
                maximum_call_depth: 0,
                canonical_owner_reads: 0,
                canonical_map_pages_read: 0,
                canonical_objects_read: 0,
                canonical_bytes_read: 0,
                production_tier: "graph5_reference_records_1",
            },
        };
        match operation(&mut state) {
            Ok(value) if state.transactions.is_empty() => Ok((value, state.observation)),
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
        }
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
    policy: NormalizedRunPolicy,
    host: &'a dyn NormalizedReferenceHost,
    capabilities: Option<&'a NormalizedCapabilities>,
    resources: &'a NormalizedResourceScope,
    control: &'a ExecutionControl,
    remaining_expressions: u64,
    call_depth: usize,
    next_transaction: u64,
    transactions: BTreeMap<RequirementReference, ReferenceTransaction>,
    calls_by_requirement: BTreeMap<RequirementReference, u64>,
    observation: NormalizedReferenceObservation,
}

impl ReferenceState<'_> {
    fn evaluate_test(
        &mut self,
        reference: DeclarationReference,
        expected: bool,
    ) -> Result<NormalizedValue, ExecutionError> {
        if self.program.artifact().package(reference.package).is_none() {
            return Err(reference_error(
                "normalized_reference_dependency_package",
                "exact test reference names a package outside the linked artifact closure",
            ));
        }
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

    fn call_declaration(
        &mut self,
        reference: DeclarationReference,
        arguments: Vec<NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.control.check()?;
        if self.program.artifact().package(reference.package).is_none() {
            return Err(reference_error(
                "normalized_reference_dependency_package",
                "exact declaration reference names a package outside the linked artifact closure",
            ));
        }
        if self.call_depth >= self.policy.maximum_call_depth {
            return Err(reference_resource(
                "normalized_reference_call_depth",
                "reference execution exceeded its call-depth budget",
            ));
        }
        self.call_depth += 1;
        self.observation.calls = self.observation.calls.saturating_add(1);
        self.observation.maximum_call_depth =
            self.observation.maximum_call_depth.max(self.call_depth);
        let previous_package = self.active_package;
        self.active_package = reference.package;
        let result = (|| {
            let declaration = self.declaration(reference)?;
            match declaration.payload {
                DeclarationPayload::Function(function) => {
                    if arguments.len() != function.parameters.len() {
                        Err(reference_type_error(
                            "function argument count disagrees with canonical parameters",
                        ))
                    } else if matches!(
                        &function.effect,
                        FunctionEffect::Task { requirements }
                            if requirements.iter().any(|requirement| {
                                self.capabilities.is_none_or(|capabilities| {
                                    !capabilities.requires_exact(*requirement)
                                })
                            })
                    ) {
                        Err(reference_capabilities_unbound())
                    } else {
                        let mut locals = function
                            .parameters
                            .into_iter()
                            .zip(arguments)
                            .map(|(parameter, value)| {
                                (LocalValueReference::FunctionParameter(parameter), value)
                            })
                            .collect();
                        self.evaluate(function.body, &mut locals)
                    }
                }
                DeclarationPayload::External(external) => {
                    if arguments.len() != external.parameters.len() {
                        Err(reference_type_error(
                            "external argument count disagrees with canonical parameters",
                        ))
                    } else {
                        let function_index = self.program.function(reference).ok_or_else(|| {
                            reference_error(
                                "normalized_reference_external_function",
                                "canonical external declaration has no prepared function",
                            )
                        })?;
                        let normalized_function = self
                            .program
                            .functions
                            .get(function_index.0 as usize)
                            .ok_or_else(|| {
                                reference_error(
                                    "normalized_reference_external_function",
                                    "prepared external function index escaped the runtime table",
                                )
                            })?;
                        self.observation.external_calls =
                            self.observation.external_calls.saturating_add(1);
                        self.host
                            .call(
                                self.program,
                                normalized_function,
                                &external.implementation,
                                arguments,
                                self.control,
                            )
                            .and_then(|value| {
                                self.charge_value(&value)?;
                                Ok(value)
                            })
                    }
                }
                DeclarationPayload::Constant { value, .. } => {
                    if arguments.is_empty() {
                        self.evaluate(value, &mut BTreeMap::new())
                    } else {
                        Err(reference_type_error("constant call received arguments"))
                    }
                }
                _ => Err(reference_type_error(
                    "exact callable reference names a non-callable declaration",
                )),
            }
        })();
        self.active_package = previous_package;
        self.call_depth -= 1;
        result
    }

    fn evaluate(
        &mut self,
        expression: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.control.check()?;
        if self.remaining_expressions == 0 {
            return Err(reference_resource(
                "normalized_reference_expression_steps",
                "reference execution exhausted its expression-step budget",
            ));
        }
        self.remaining_expressions -= 1;
        self.observation.expressions = self.observation.expressions.saturating_add(1);
        let operation = match self.owner(OwnerKey::Expression(expression))? {
            Some(OwnerRecord::Expression(record)) => record.operation,
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
        match operation {
            ExpressionOperation::Unit {} => Ok(NormalizedValue::Unit),
            ExpressionOperation::Bool { value } => Ok(NormalizedValue::Bool(value)),
            ExpressionOperation::I64 { value } => Ok(NormalizedValue::I64(value)),
            ExpressionOperation::Text { value } => self.text(value).map(NormalizedValue::Text),
            ExpressionOperation::StaticText { value } => {
                self.text(value).map(NormalizedValue::StaticText)
            }
            ExpressionOperation::Local { value } => locals.get(&value).cloned().ok_or_else(|| {
                reference_error(
                    "normalized_reference_local_missing",
                    "canonical local reference escaped its exact lexical scope",
                )
            }),
            ExpressionOperation::Constant { declaration } => {
                self.call_declaration(declaration, Vec::new())
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => match self.evaluate(condition, locals)? {
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
                function,
                arguments,
                ..
            } => {
                let arguments = self.evaluate_many(&arguments, locals)?;
                self.call_declaration(function, arguments)
            }
            ExpressionOperation::FunctionValue { function, .. } => self
                .program
                .function(function)
                .map(NormalizedValue::Function)
                .ok_or_else(|| {
                    reference_error(
                        "normalized_reference_function_value",
                        "exact function value has no executable artifact unit",
                    )
                }),
            ExpressionOperation::Invoke { callee, arguments } => {
                let callee = self.evaluate(callee, locals)?;
                let NormalizedValue::Function(function) = callee else {
                    return Err(reference_type_error("invoke callee is not a function"));
                };
                let declaration = self.function_reference(function)?;
                let arguments = self.evaluate_many(&arguments, locals)?;
                self.call_declaration(declaration, arguments)
            }
            ExpressionOperation::Record {
                nominal_type,
                fields,
            } => {
                let mut values = Vec::with_capacity(fields.len());
                for field in &fields {
                    values.push(self.evaluate(field.value, locals)?);
                }
                self.record(
                    nominal_type,
                    fields.into_iter().map(|field| field.selector),
                    values,
                )
            }
            ExpressionOperation::Variant { case, payload } => {
                let (layout, tag) = self.case_layout(case)?;
                let payload = payload
                    .map(|payload| self.evaluate(payload, locals))
                    .transpose()?
                    .map(Box::new);
                if payload.is_some() {
                    self.charge_items(1, std::mem::size_of::<NormalizedValue>())?;
                }
                Ok(NormalizedValue::Variant {
                    layout,
                    case: tag,
                    payload,
                })
            }
            ExpressionOperation::Field { value, selector } => {
                let value = self.evaluate(value, locals)?;
                self.field(value, selector)
            }
            ExpressionOperation::List { items, .. } => {
                let values = self.evaluate_many(&items, locals)?;
                self.charge_items(values.len(), std::mem::size_of::<NormalizedValue>())?;
                Ok(NormalizedValue::List(Arc::new(values)))
            }
            ExpressionOperation::Map { entries, .. } => {
                let mut values = BTreeMap::new();
                let mut key_bytes = 0_u64;
                for entry in entries {
                    let key = NormalizedMapKey::from_value(self.evaluate(entry.key, locals)?)
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
                Ok(NormalizedValue::Map(Arc::new(values)))
            }
            ExpressionOperation::Match { value, arms } => {
                let NormalizedValue::Variant {
                    layout,
                    case,
                    payload,
                } = self.evaluate(value, locals)?
                else {
                    return Err(reference_type_error("match value is not a variant"));
                };
                let mut selected = None;
                for arm in arms {
                    if self.case_layout(arm.case)? == (layout, case) {
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
                        if locals.insert(local, *payload).is_some() {
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
                let arguments = self.evaluate_many(&arguments, locals)?;
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
        locals: &mut BTreeMap<LocalValueReference, NormalizedValue>,
    ) -> Result<Vec<NormalizedValue>, ExecutionError> {
        expressions
            .iter()
            .map(|expression| self.evaluate(*expression, locals))
            .collect()
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
        let read = if package == self.binding.package {
            self.authority.owner(owner)?
        } else {
            let mut map = crate::platform::persistent_map::MapWork::default();
            let mut store = StoreWork::default();
            let record = self
                .program
                .artifact()
                .reference_owner(package, owner, &mut map, &mut store)
                .map_err(|diagnostic| {
                    reference_error(
                        "normalized_reference_dependency_authority",
                        format!(
                            "linked canonical reference-owner read failed ({}): {}",
                            diagnostic.code, diagnostic.message
                        ),
                    )
                })?;
            NormalizedReferenceOwnerRead {
                record,
                work: NormalizedReferenceReadWork {
                    owner_reads: 1,
                    map_pages_read: map.pages_read,
                    objects_read: store.objects_read,
                    bytes_read: map.bytes_read.saturating_add(store.bytes_read),
                },
            }
        };
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
        self.program
            .functions
            .get(function.0 as usize)
            .map(|function| function.declaration)
            .ok_or_else(|| {
                reference_error(
                    "normalized_reference_function_index",
                    "function value escaped the prepared artifact table",
                )
            })
    }

    fn record(
        &mut self,
        nominal_type: Option<DeclarationReference>,
        selectors: impl IntoIterator<Item = FieldSelector>,
        values: Vec<NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.charge_items(values.len(), std::mem::size_of::<NormalizedValue>())?;
        if let Some(declaration) = nominal_type {
            let layout = self.record_layout(declaration)?;
            let field_count = self.program.records[layout.0 as usize].fields.len();
            let mut slots = vec![None; field_count];
            for (selector, value) in selectors.into_iter().zip(values) {
                let FieldSelector::Nominal(field) = selector else {
                    return Err(reference_error(
                        "normalized_reference_record_field_kind",
                        "nominal record contains a structural field selector",
                    ));
                };
                let (field_layout, offset) = self.field_layout(field)?;
                if field_layout != layout {
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
            Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
                layout,
                fields: Arc::new(fields),
            }))
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
            Ok(NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(fields),
            }))
        }
    }

    fn field(
        &self,
        value: NormalizedValue,
        selector: FieldSelector,
    ) -> Result<NormalizedValue, ExecutionError> {
        match (value, selector) {
            (
                NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }),
                FieldSelector::Nominal(field),
            ) => {
                let (expected, offset) = self.field_layout(field)?;
                if expected != layout {
                    return Err(reference_type_error(
                        "nominal field belongs to another runtime record layout",
                    ));
                }
                fields.get(offset as usize).cloned().ok_or_else(|| {
                    reference_error(
                        "normalized_reference_field_offset",
                        "nominal field offset escaped its runtime layout",
                    )
                })
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields }),
                FieldSelector::Structural(name),
            ) => fields
                .binary_search_by(|(candidate, _)| candidate.cmp(&name))
                .ok()
                .map(|index| fields[index].1.clone())
                .ok_or_else(|| reference_type_error("structural record has no selected field")),
            _ => Err(reference_type_error(
                "field selection received a foreign record layout",
            )),
        }
    }

    fn record_layout(
        &self,
        declaration: DeclarationReference,
    ) -> Result<RecordLayoutIndex, ExecutionError> {
        self.program
            .records
            .iter()
            .position(|layout| layout.declaration == declaration)
            .and_then(|index| u32::try_from(index).ok())
            .map(RecordLayoutIndex)
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
        for (layout_index, layout) in self.program.records.iter().enumerate() {
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
                return Ok((RecordLayoutIndex(layout_index), offset));
            }
        }
        Err(reference_error(
            "normalized_reference_field_layout",
            "exact field has no prepared runtime layout",
        ))
    }

    fn case_layout(
        &self,
        case: CaseReference,
    ) -> Result<(VariantLayoutIndex, u32), ExecutionError> {
        for (layout_index, layout) in self.program.variants.iter().enumerate() {
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
                return Ok((VariantLayoutIndex(layout_index), tag));
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
        arguments: Vec<NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.charge_capability_call(requirement)?;
        let value = if let Some(transaction) = self.transactions.get_mut(&requirement) {
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
            self.capabilities
                .ok_or_else(reference_capabilities_unbound)?
                .call_exact(
                    self.program,
                    requirement,
                    operation,
                    arguments,
                    self.resources,
                    self.control,
                )?
        };
        self.charge_value(&value)?;
        Ok(value)
    }

    fn transaction(
        &mut self,
        requirement: RequirementReference,
        binding: BindingId,
        body: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.binding(binding, BindingKind::Transaction)?;
        if self.transactions.contains_key(&requirement) {
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
        let transaction = self
            .capabilities
            .ok_or_else(reference_capabilities_unbound)?
            .begin_transaction_exact(self.program, requirement, self.resources, self.control)?;
        self.next_transaction = next_generation;
        debug_assert!(locals.insert(local, NormalizedValue::Unit).is_none());
        self.transactions.insert(
            requirement,
            ReferenceTransaction {
                binding,
                generation,
                transaction,
            },
        );
        let result = self.evaluate(body, locals);
        let token = locals.remove(&local);
        let mut transaction = self.transactions.remove(&requirement).ok_or_else(|| {
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
                let key = ObjectKey::from_digest(ObjectDomain::Blob, digest.bytes());
                let value = self
                    .program
                    .artifact()
                    .read(
                        key,
                        ObjectDomain::Blob.maximum_bytes(),
                        &mut StoreWork::default(),
                    )
                    .map_err(|error| {
                        reference_error(
                            "normalized_reference_blob_read",
                            format!("reference text blob read failed: {}", error.message),
                        )
                    })?
                    .ok_or_else(|| {
                        reference_error(
                            "normalized_reference_blob_missing",
                            "reference text blob is absent from the exact artifact closure",
                        )
                    })?;
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
        if self.observation.capability_calls >= self.policy.maximum_capability_calls {
            return Err(reference_resource(
                "normalized_reference_capability_calls",
                "reference execution exhausted its capability-call budget",
            ));
        }
        let capabilities = self
            .capabilities
            .ok_or_else(reference_capabilities_unbound)?;
        let maximum = capabilities.maximum_calls_exact(requirement)?;
        let calls = self.calls_by_requirement.entry(requirement).or_default();
        if *calls >= maximum {
            return Err(reference_resource(
                "normalized_reference_grant_calls",
                "reference execution exhausted one deployment-grant call bound",
            ));
        }
        *calls = calls.saturating_add(1);
        self.observation.capability_calls = self.observation.capability_calls.saturating_add(1);
        Ok(())
    }

    fn charge_items(&mut self, items: usize, item_bytes: usize) -> Result<(), ExecutionError> {
        let items = items as u64;
        let next = self
            .observation
            .collection_items
            .checked_add(items)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_collection_items",
                    "reference collection-item accounting overflowed",
                )
            })?;
        if next > self.policy.maximum_collection_items {
            return Err(reference_resource(
                "normalized_reference_collection_items",
                "reference execution exhausted its collection-item budget",
            ));
        }
        self.observation.collection_items = next;
        let bytes = items.checked_mul(item_bytes as u64).ok_or_else(|| {
            reference_resource(
                "normalized_reference_allocation",
                "reference collection allocation overflowed",
            )
        })?;
        self.charge_allocation(bytes)
    }

    fn charge_allocation(&mut self, bytes: u64) -> Result<(), ExecutionError> {
        let next = self
            .observation
            .allocated_bytes
            .checked_add(bytes)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_allocation",
                    "reference allocation accounting overflowed",
                )
            })?;
        if next > self.policy.maximum_allocated_bytes {
            return Err(reference_resource(
                "normalized_reference_allocation",
                "reference execution exhausted its allocation budget",
            ));
        }
        self.observation.allocated_bytes = next;
        Ok(())
    }

    fn charge_value(&mut self, value: &NormalizedValue) -> Result<(), ExecutionError> {
        let (bytes, items) = reference_value_cost(value)?;
        let next = self
            .observation
            .collection_items
            .checked_add(items)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_collection_items",
                    "reference external value item accounting overflowed",
                )
            })?;
        if next > self.policy.maximum_collection_items {
            return Err(reference_resource(
                "normalized_reference_collection_items",
                "reference external value exceeds its collection-item budget",
            ));
        }
        self.observation.collection_items = next;
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
    program: &NormalizedProgram,
    function: &super::prepare::NormalizedFunction,
    implementation: &str,
    arguments: Vec<NormalizedValue>,
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
            encode_typed(program, value, parameter.ty, JsonLimits::default())
                .map(NormalizedValue::bytes)
                .map_err(reference_json_error)
        }
        "core.json.decode-or" => {
            let [NormalizedValue::Bytes(bytes), fallback] = arguments.as_slice() else {
                return Err(reference_type_error(
                    "typed JSON decoding received foreign values",
                ));
            };
            let Some(parameter) = function.parameters.get(1) else {
                return Err(reference_error(
                    "reference_json_signature",
                    "typed JSON decoder has no exact fallback type",
                ));
            };
            let decoded = decode_typed(program, bytes, parameter.ty, JsonLimits::default());
            let (valid, value, error) = match decoded {
                Ok(value) => (true, value, String::new()),
                Err(diagnostic) => (false, fallback.clone(), diagnostic.code),
            };
            reference_structural_record(vec![
                ("value", value),
                ("error", NormalizedValue::text(error)),
                ("valid", NormalizedValue::Bool(valid)),
            ])
        }
        "core.http.bearer-token" => reference_bearer_token(program, arguments.as_slice()),
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
        "core.list.append" => match arguments.as_slice() {
            [NormalizedValue::List(values), value] => {
                let next = values.len().checked_add(1).ok_or_else(|| {
                    reference_resource("reference_list_length", "list length overflowed")
                })?;
                let mut output = Vec::with_capacity(next);
                output.extend_from_slice(values);
                output.push(value.clone());
                Ok(NormalizedValue::List(Arc::new(output)))
            }
            _ => Err(reference_type_error("list append received foreign values")),
        },
        "core.map.length" => match arguments.as_slice() {
            [NormalizedValue::Map(values)] => {
                Ok(NormalizedValue::I64(reference_length(values.len())?))
            }
            _ => Err(reference_type_error("map length received a foreign value")),
        },
        "core.map.get" | "core.map.contains" | "core.map.get-or" | "core.map.insert" => {
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
    program: &NormalizedProgram,
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
            let layout = program.records.get(usize::try_from(layout.0).ok()?)?;
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
    program: &NormalizedProgram,
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

fn reference_map_intrinsic(
    implementation: &str,
    arguments: Vec<NormalizedValue>,
) -> Result<NormalizedValue, ExecutionError> {
    let Some(NormalizedValue::Map(entries)) = arguments.first() else {
        return Err(reference_type_error("map intrinsic received a foreign map"));
    };
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
        _ => Err(reference_type_error(
            "map intrinsic received a foreign arity",
        )),
    }
}

fn reference_json_error(error: crate::platform::diagnostic::Diagnostic) -> ExecutionError {
    let class = match error.class {
        crate::platform::diagnostic::DiagnosticClass::Resource => ExecutionFailureClass::Resource,
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
        ) if left_layout == right_layout && left.len() == right.len() => {
            reference_equal_sequence(left, right)
        }
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
            NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
        ) if left.len() == right.len() => {
            for ((left_name, left), (right_name, right)) in left.iter().zip(right.iter()) {
                if left_name != right_name || !reference_equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
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
        ) if left_layout == right_layout && left_case == right_case => match (left, right) {
            (None, None) => Ok(true),
            (Some(left), Some(right)) => reference_equal(left, right),
            _ => Ok(false),
        },
        (NormalizedValue::List(left), NormalizedValue::List(right))
            if left.len() == right.len() =>
        {
            reference_equal_sequence(left, right)
        }
        (NormalizedValue::Map(left), NormalizedValue::Map(right)) if left.len() == right.len() => {
            for (key, left) in left.iter() {
                let Some(right) = right.get(key) else {
                    return Ok(false);
                };
                if !reference_equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (NormalizedValue::Function(_), _) | (_, NormalizedValue::Function(_)) => {
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
        _ => Ok(false),
    }
}

fn reference_equal_sequence(
    left: &[NormalizedValue],
    right: &[NormalizedValue],
) -> Result<bool, ExecutionError> {
    for (left, right) in left.iter().zip(right.iter()) {
        if !reference_equal(left, right)? {
            return Ok(false);
        }
    }
    Ok(true)
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
            | NormalizedValue::Function(_)
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
    if policy.instruction_steps == 0
        || policy.maximum_call_depth == 0
        || policy.maximum_value_stack == 0
        || policy.maximum_allocated_bytes == 0
        || policy.maximum_collection_items == 0
        || policy.maximum_capability_calls == 0
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

fn reference_resource(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::resource(code, message)
}

fn reference_error(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}
