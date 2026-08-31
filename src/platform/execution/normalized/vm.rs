//! Bounded dense-index virtual machine for normalized Graph 5 compiler units.

use super::capability::{
    NormalizedCapabilities, NormalizedCapabilityTransaction, validate_outcome,
};
use super::codec::{decode_typed, encode_typed};
use super::prepare::{
    NormalizedCode, NormalizedEntryPoint, NormalizedFieldSelector, NormalizedFunctionBody,
    NormalizedInstruction, NormalizedProgram, NormalizedTarget,
};
use super::resource::NormalizedResourceScope;
use super::value::{
    FunctionIndex, NormalizedMapKey, NormalizedRecord, NormalizedValue, RequirementIndex,
};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::JsonLimits;
use crate::platform::kernel::{DeclarationReference, ImplementationName, Name, TypeObjectDigest};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizedRunPolicy {
    pub instruction_steps: u64,
    pub maximum_call_depth: usize,
    pub maximum_value_stack: usize,
    pub maximum_allocated_bytes: u64,
    pub maximum_collection_items: u64,
    pub maximum_capability_calls: u64,
}

impl Default for NormalizedRunPolicy {
    fn default() -> Self {
        Self {
            instruction_steps: 10_000_000,
            maximum_call_depth: 4_096,
            maximum_value_stack: 1_000_000,
            maximum_allocated_bytes: 256 * 1024 * 1024,
            maximum_collection_items: 1_000_000,
            maximum_capability_calls: 100_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedRunObservation {
    pub instructions: u64,
    pub calls: u64,
    pub external_calls: u64,
    pub capability_calls: u64,
    pub allocated_bytes: u64,
    pub collection_items: u64,
    pub maximum_call_depth: usize,
    pub maximum_value_stack: usize,
    pub production_tier: &'static str,
}

pub type NormalizedInvocation = (NormalizedValue, NormalizedRunObservation);
pub type NormalizedTestInvocation = (NormalizedInvocation, NormalizedInvocation);

pub trait NormalizedHost: Send + Sync {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &super::prepare::NormalizedFunction,
        implementation: &ImplementationName,
        type_arguments: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoreNormalizedHost;

impl NormalizedHost for CoreNormalizedHost {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &super::prepare::NormalizedFunction,
        implementation: &ImplementationName,
        type_arguments: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        call_core_intrinsic(
            program,
            function,
            implementation.as_str(),
            type_arguments,
            arguments,
        )
    }
}

static CORE_HOST: CoreNormalizedHost = CoreNormalizedHost;

pub struct NormalizedVm<'a> {
    program: &'a NormalizedProgram,
    policy: NormalizedRunPolicy,
    host: &'a dyn NormalizedHost,
}

impl<'a> NormalizedVm<'a> {
    pub fn new(program: &'a NormalizedProgram, policy: NormalizedRunPolicy) -> Self {
        Self {
            program,
            policy,
            host: &CORE_HOST,
        }
    }

    #[cfg(test)]
    pub fn invoke(
        &self,
        declaration: DeclarationReference,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedInvocation, ExecutionError> {
        let function = self.program.function(declaration).ok_or_else(|| {
            runtime_error(
                "normalized_function_missing",
                "exact declaration has no prepared callable runtime unit",
            )
        })?;
        self.invoke_entry(
            NormalizedEntryPoint::Function(function),
            arguments,
            capabilities,
            control,
        )
    }

    pub fn invoke_root_target(
        &self,
        name: &Name,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedInvocation, ExecutionError> {
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
    ) -> Result<NormalizedInvocation, ExecutionError> {
        let target = self.program.root_target(name).ok_or_else(|| {
            runtime_error(
                "normalized_target_missing",
                "root artifact package has no target with the exact selected name",
            )
        })?;
        self.invoke_target_scoped(target, arguments, capabilities, resources, control)
    }

    pub(crate) fn invoke_target_scoped(
        &self,
        target: &NormalizedTarget,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedInvocation, ExecutionError> {
        let port = self
            .program
            .ports
            .get(target.port.0 as usize)
            .ok_or_else(|| {
                runtime_error(
                    "normalized_target_port",
                    "prepared target port index is outside the runtime table",
                )
            })?;
        if port.component != target.component {
            return Err(runtime_error(
                "normalized_target_component",
                "prepared target and port disagree on their exact component",
            ));
        }
        self.invoke_entry_scoped(
            port.entry.clone(),
            arguments,
            capabilities,
            resources,
            control,
        )
    }

    pub fn invoke_test(
        &self,
        declaration: DeclarationReference,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedTestInvocation, ExecutionError> {
        let test = self.program.tests.get(&declaration).ok_or_else(|| {
            runtime_error(
                "normalized_test_missing",
                "exact declaration has no prepared test runtime unit",
            )
        })?;
        let actual = self.invoke_entry(
            NormalizedEntryPoint::Code(test.actual.clone()),
            Vec::new(),
            capabilities,
            control,
        )?;
        let expected = self.invoke_entry(
            NormalizedEntryPoint::Code(test.expected.clone()),
            Vec::new(),
            capabilities,
            control,
        )?;
        Ok((actual, expected))
    }

    fn invoke_entry(
        &self,
        entry: NormalizedEntryPoint,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        control: &ExecutionControl,
    ) -> Result<NormalizedInvocation, ExecutionError> {
        let resources = NormalizedResourceScope::new()?;
        self.invoke_entry_scoped(entry, arguments, capabilities, &resources, control)
    }

    fn invoke_entry_scoped(
        &self,
        entry: NormalizedEntryPoint,
        arguments: Vec<NormalizedValue>,
        capabilities: Option<&NormalizedCapabilities>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedInvocation, ExecutionError> {
        validate_policy(self.policy)?;
        control.check()?;
        let mut machine = Machine {
            program: self.program,
            policy: self.policy,
            host: self.host,
            capabilities,
            resources,
            control,
            remaining_steps: self.policy.instruction_steps,
            stack: Vec::new(),
            frames: Vec::new(),
            next_frame: 0,
            next_transaction: 0,
            transactions: BTreeMap::new(),
            calls_by_requirement: BTreeMap::new(),
            observation: NormalizedRunObservation {
                instructions: 0,
                calls: 0,
                external_calls: 0,
                capability_calls: 0,
                allocated_bytes: 0,
                collection_items: 0,
                maximum_call_depth: 0,
                maximum_value_stack: 0,
                production_tier: "graph5_dense_bytecode_1",
            },
        };
        let result = (|| {
            let port_arguments = match entry {
                NormalizedEntryPoint::Function(function) => {
                    machine.call(function, Arc::from([]), arguments)?;
                    None
                }
                NormalizedEntryPoint::Code(code) => {
                    machine.call_code(code, arguments, BTreeMap::new())?;
                    None
                }
                NormalizedEntryPoint::PortExpression(code) => {
                    machine.call_code(code, Vec::new(), BTreeMap::new())?;
                    Some(arguments)
                }
            };
            let value = finish_admitted(&mut machine)?;
            let Some(arguments) = port_arguments else {
                return Ok(value);
            };
            let NormalizedValue::Function {
                function,
                type_arguments,
            } = value
            else {
                return Err(type_error(
                    "expression-backed target port did not evaluate to a function",
                ));
            };
            machine.call(function, type_arguments, arguments)?;
            finish_admitted(&mut machine)
        })();
        match result {
            Ok(value) => Ok((value, machine.observation)),
            Err(error) => {
                machine.rollback_all();
                Err(error)
            }
        }
    }
}

fn finish_admitted(machine: &mut Machine<'_>) -> Result<NormalizedValue, ExecutionError> {
    if !machine.frames.is_empty() {
        return machine.run();
    }
    let value = machine.pop()?;
    if !machine.stack.is_empty() {
        return Err(runtime_error(
            "normalized_stack_residue",
            "normalized external entry left more than one result value",
        ));
    }
    Ok(value)
}

struct Frame {
    id: u64,
    code: NormalizedCode,
    instruction: usize,
    locals: Vec<Option<NormalizedValue>>,
    type_arguments: BTreeMap<TypeParameterId, TypeObjectDigest>,
    stack_base: usize,
}

struct ActiveTransaction {
    owner_frame: u64,
    binding: u32,
    generation: u64,
    transaction: Box<dyn NormalizedCapabilityTransaction>,
}

struct Machine<'a> {
    program: &'a NormalizedProgram,
    policy: NormalizedRunPolicy,
    host: &'a dyn NormalizedHost,
    capabilities: Option<&'a NormalizedCapabilities>,
    resources: &'a NormalizedResourceScope,
    control: &'a ExecutionControl,
    remaining_steps: u64,
    stack: Vec<NormalizedValue>,
    frames: Vec<Frame>,
    next_frame: u64,
    next_transaction: u64,
    transactions: BTreeMap<RequirementIndex, ActiveTransaction>,
    calls_by_requirement: BTreeMap<RequirementIndex, u64>,
    observation: NormalizedRunObservation,
}

impl Machine<'_> {
    fn run(&mut self) -> Result<NormalizedValue, ExecutionError> {
        loop {
            self.control.check()?;
            if self.remaining_steps == 0 {
                return Err(resource_error(
                    "normalized_instruction_steps",
                    "normalized execution exhausted its instruction-step budget",
                ));
            }
            self.remaining_steps -= 1;
            self.observation.instructions = self.observation.instructions.saturating_add(1);
            let instruction = {
                let frame = self.current_frame_mut()?;
                let instruction = frame
                    .code
                    .instructions
                    .get(frame.instruction)
                    .cloned()
                    .ok_or_else(|| {
                        runtime_error(
                            "normalized_instruction_missing",
                            "normalized instruction pointer escaped its verified code",
                        )
                    })?;
                frame.instruction = frame.instruction.saturating_add(1);
                instruction
            };
            match instruction {
                NormalizedInstruction::Unit => self.push(NormalizedValue::Unit)?,
                NormalizedInstruction::Bool(value) => self.push(NormalizedValue::Bool(value))?,
                NormalizedInstruction::I64(value) => self.push(NormalizedValue::I64(value))?,
                NormalizedInstruction::Text(value) => self.push(NormalizedValue::Text(value))?,
                NormalizedInstruction::StaticText(value) => {
                    self.push(NormalizedValue::StaticText(value))?
                }
                NormalizedInstruction::LoadLocal(local) => {
                    let value = self
                        .frames
                        .last()
                        .and_then(|frame| frame.locals.get(local as usize))
                        .and_then(Clone::clone)
                        .ok_or_else(|| {
                            runtime_error(
                                "normalized_local_uninitialized",
                                "normalized code read an uninitialized local",
                            )
                        })?;
                    self.push(value)?;
                }
                NormalizedInstruction::StoreLocal(local) => {
                    let value = self.pop()?;
                    self.set_local(local, Some(value))?;
                }
                NormalizedInstruction::Drop => {
                    self.pop()?;
                }
                NormalizedInstruction::JumpIfFalse(target) => {
                    let NormalizedValue::Bool(condition) = self.pop()? else {
                        return Err(type_error("if condition is not boolean"));
                    };
                    if !condition {
                        self.jump(target)?;
                    }
                }
                NormalizedInstruction::Jump(target) => self.jump(target)?,
                NormalizedInstruction::Call {
                    function,
                    type_arguments,
                    arguments,
                } => {
                    let arguments = self.pop_many(arguments as usize)?;
                    let type_arguments = self.resolve_type_arguments(&type_arguments)?;
                    self.call(function, type_arguments, arguments)?;
                }
                NormalizedInstruction::FunctionValue {
                    function,
                    type_arguments,
                } => {
                    let type_arguments = self.resolve_type_arguments(&type_arguments)?;
                    self.push(NormalizedValue::Function {
                        function,
                        type_arguments,
                    })?;
                }
                NormalizedInstruction::Invoke { arguments } => {
                    let arguments = self.pop_many(arguments as usize)?;
                    let NormalizedValue::Function {
                        function,
                        type_arguments,
                    } = self.pop()?
                    else {
                        return Err(type_error("invoke callee is not a function"));
                    };
                    self.call(function, type_arguments, arguments)?;
                }
                NormalizedInstruction::Record { layout, fields } => {
                    let values = self.pop_many(fields.len())?;
                    let value = self.record(layout, &fields, values)?;
                    self.push(value)?;
                }
                NormalizedInstruction::Variant {
                    layout,
                    case,
                    has_payload,
                } => {
                    let payload = has_payload.then(|| self.pop()).transpose()?;
                    self.charge_allocation(if payload.is_some() {
                        std::mem::size_of::<NormalizedValue>() as u64
                    } else {
                        0
                    })?;
                    self.push(NormalizedValue::Variant {
                        layout,
                        case,
                        payload: payload.map(Box::new),
                    })?;
                }
                NormalizedInstruction::Field(field) => {
                    let value = self.pop()?;
                    self.push(select_field(value, &field)?)?;
                }
                NormalizedInstruction::List { items } => {
                    let items = self.pop_many(items as usize)?;
                    self.charge_collection(items.len(), std::mem::size_of::<NormalizedValue>())?;
                    self.push(NormalizedValue::List(Arc::new(items)))?;
                }
                NormalizedInstruction::Map { entries } => {
                    let count = (entries as usize).checked_mul(2).ok_or_else(|| {
                        resource_error(
                            "normalized_map_items",
                            "normalized map entry count overflowed",
                        )
                    })?;
                    let values = self.pop_many(count)?;
                    let mut map = BTreeMap::new();
                    let mut key_bytes = 0_u64;
                    let (pairs, remainder) = values.as_chunks::<2>();
                    if !remainder.is_empty() {
                        return Err(runtime_error(
                            "normalized_map_pairs",
                            "verified map instruction produced an incomplete key-value pair",
                        ));
                    }
                    for pair in pairs {
                        let key =
                            NormalizedMapKey::from_value(pair[0].clone()).ok_or_else(|| {
                                type_error("map key is not a deterministically ordered primitive")
                            })?;
                        key_bytes = key_bytes.saturating_add(map_key_bytes(&key));
                        if map.insert(key, pair[1].clone()).is_some() {
                            return Err(trap_error(
                                "normalized_map_duplicate_key",
                                "map expression contains a duplicate key",
                            ));
                        }
                    }
                    self.charge_collection(
                        map.len(),
                        std::mem::size_of::<(NormalizedMapKey, NormalizedValue)>(),
                    )?;
                    self.charge_allocation(key_bytes)?;
                    self.push(NormalizedValue::Map(Arc::new(map)))?;
                }
                NormalizedInstruction::SwitchVariant(jumps) => {
                    let NormalizedValue::Variant {
                        layout,
                        case,
                        payload,
                    } = self.pop()?
                    else {
                        return Err(type_error("match value is not a variant"));
                    };
                    let jump = jumps
                        .iter()
                        .find(|jump| jump.layout == layout && jump.case == case)
                        .ok_or_else(|| {
                            runtime_error(
                                "normalized_match_case",
                                "verified exhaustive match omitted the runtime case tag",
                            )
                        })?;
                    match (jump.binding_local, payload) {
                        (Some(local), Some(payload)) => self.set_local(local, Some(*payload))?,
                        (None, None) => {}
                        _ => {
                            return Err(runtime_error(
                                "normalized_match_payload",
                                "runtime variant payload disagrees with the verified match arm",
                            ));
                        }
                    }
                    self.jump(jump.target)?;
                }
                NormalizedInstruction::Perform {
                    requirement,
                    operation,
                    arguments,
                } => {
                    let arguments = self.pop_many(arguments as usize)?;
                    self.charge_capability_call(requirement)?;
                    let capabilities = self.capabilities.ok_or_else(capabilities_unbound)?;
                    let canonical = capabilities.canonical_requirement(requirement)?;
                    let value = if let Some(transaction) = self.transactions.get_mut(&canonical) {
                        let policy = self
                            .capabilities
                            .ok_or_else(capabilities_unbound)?
                            .call_policy(self.program, requirement, operation)?;
                        let result = transaction.transaction.call(
                            &policy,
                            arguments,
                            self.resources,
                            self.control,
                        );
                        validate_outcome(&policy, result)?
                    } else {
                        capabilities.call(
                            self.program,
                            requirement,
                            operation,
                            arguments,
                            self.resources,
                            self.control,
                        )?
                    };
                    self.charge_external_value(&value)?;
                    self.push(value)?;
                }
                NormalizedInstruction::BeginTransaction {
                    requirement,
                    binding,
                } => {
                    let capabilities = self.capabilities.ok_or_else(capabilities_unbound)?;
                    let canonical = capabilities.canonical_requirement(requirement)?;
                    if self.transactions.contains_key(&canonical) {
                        return Err(runtime_error(
                            "normalized_transaction_nested",
                            "one exact requirement cannot begin a nested transaction",
                        ));
                    }
                    let owner_frame = self.current_frame()?;
                    if owner_frame
                        .locals
                        .get(binding as usize)
                        .is_none_or(Option::is_some)
                    {
                        return Err(runtime_error(
                            "normalized_transaction_binding",
                            "transaction binding escaped its verified empty local slot",
                        ));
                    }
                    let owner_frame = owner_frame.id;
                    let generation = self.next_transaction;
                    let next_generation =
                        self.next_transaction.checked_add(1).ok_or_else(|| {
                            resource_error(
                                "normalized_transaction_generation",
                                "transaction generation counter overflowed",
                            )
                        })?;
                    self.charge_capability_call(requirement)?;
                    let transaction = capabilities.begin_transaction(
                        self.program,
                        requirement,
                        self.resources,
                        self.control,
                    )?;
                    self.next_transaction = next_generation;
                    self.set_local(binding, Some(NormalizedValue::Unit))?;
                    self.transactions.insert(
                        canonical,
                        ActiveTransaction {
                            owner_frame,
                            binding,
                            generation,
                            transaction,
                        },
                    );
                }
                NormalizedInstruction::CommitTransaction {
                    requirement,
                    binding,
                } => {
                    let canonical = self
                        .capabilities
                        .ok_or_else(capabilities_unbound)?
                        .canonical_requirement(requirement)?;
                    let mut transaction =
                        self.transactions.remove(&canonical).ok_or_else(|| {
                            runtime_error(
                                "normalized_transaction_missing",
                                "transaction commit has no active exact requirement scope",
                            )
                        })?;
                    let frame = self.current_frame()?;
                    let token = frame.locals.get(binding as usize).and_then(Option::as_ref);
                    if transaction.owner_frame != frame.id
                        || transaction.binding != binding
                        || !matches!(token, Some(NormalizedValue::Unit))
                    {
                        let _ = transaction.transaction.rollback();
                        return Err(runtime_error(
                            "normalized_transaction_binding",
                            "transaction commit disagrees with its exact runtime binding",
                        ));
                    }
                    transaction.transaction.commit(self.control)?;
                    self.set_local(binding, None)?;
                }
                NormalizedInstruction::Return => {
                    let result = self.pop()?;
                    let stack_base = self.current_frame()?.stack_base;
                    if self.stack.len() != stack_base {
                        return Err(runtime_error(
                            "normalized_stack_residue",
                            "normalized function returned with extra operand values",
                        ));
                    }
                    let frame = self.frames.pop().ok_or_else(|| {
                        runtime_error(
                            "normalized_frame_missing",
                            "normalized execution lost its returning frame",
                        )
                    })?;
                    if self
                        .transactions
                        .values()
                        .any(|transaction| transaction.owner_frame == frame.id)
                    {
                        return Err(runtime_error(
                            "normalized_transaction_leak",
                            "normalized function returned with an active transaction",
                        ));
                    }
                    if self.frames.is_empty() {
                        if !self.stack.is_empty() {
                            return Err(runtime_error(
                                "normalized_stack_residue",
                                "normalized execution left values beneath its result",
                            ));
                        }
                        return Ok(result);
                    }
                    self.push(result)?;
                }
            }
        }
    }

    fn resolve_type_arguments(
        &self,
        type_arguments: &[TypeObjectDigest],
    ) -> Result<Arc<[TypeObjectDigest]>, ExecutionError> {
        let substitutions = self
            .frames
            .last()
            .map(|frame| &frame.type_arguments)
            .ok_or_else(|| {
                runtime_error(
                    "normalized_frame_missing",
                    "normalized call type arguments have no active frame",
                )
            })?;
        type_arguments
            .iter()
            .map(|ty| {
                self.program
                    .substitute_type(*ty, substitutions, 0)
                    .ok_or_else(|| {
                        type_error("normalized call type argument escaped its exact function scope")
                    })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Into::into)
    }

    fn call(
        &mut self,
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
        arguments: Vec<NormalizedValue>,
    ) -> Result<(), ExecutionError> {
        let function = self
            .program
            .functions
            .get(function.0 as usize)
            .ok_or_else(|| {
                runtime_error(
                    "normalized_function_index",
                    "normalized function index escaped the prepared table",
                )
            })?;
        if arguments.len() != function.parameter_count as usize {
            return Err(type_error("function argument count is foreign"));
        }
        if !type_arguments.is_empty() && type_arguments.len() != function.type_parameters.len() {
            return Err(type_error("function type-argument count is foreign"));
        }
        let type_arguments_by_parameter = function
            .type_parameters
            .iter()
            .copied()
            .zip(type_arguments.iter().copied())
            .collect::<BTreeMap<_, _>>();
        if type_arguments_by_parameter.len() != type_arguments.len()
            || type_arguments.iter().any(|ty| {
                self.program
                    .substitute_type(*ty, &BTreeMap::new(), 0)
                    .is_none()
            })
        {
            return Err(type_error(
                "function type arguments are not exact runtime types",
            ));
        }
        if function.task_requirements.iter().any(|requirement| {
            self.capabilities
                .is_none_or(|capabilities| !capabilities.requires(*requirement))
        }) {
            return Err(capabilities_unbound());
        }
        self.observation.calls = self.observation.calls.saturating_add(1);
        match &function.body {
            NormalizedFunctionBody::Code(code) => {
                self.call_code(code.clone(), arguments, type_arguments_by_parameter)
            }
            NormalizedFunctionBody::External(implementation) => {
                self.observation.external_calls = self.observation.external_calls.saturating_add(1);
                let value = self.host.call(
                    self.program,
                    function,
                    implementation,
                    &type_arguments,
                    arguments,
                    self.control,
                )?;
                self.charge_external_value(&value)?;
                self.push(value)
            }
        }
    }

    fn call_code(
        &mut self,
        code: NormalizedCode,
        arguments: Vec<NormalizedValue>,
        type_arguments: BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<(), ExecutionError> {
        if arguments.len() != code.parameter_count as usize {
            return Err(type_error("code argument count is foreign"));
        }
        if self.frames.len() >= self.policy.maximum_call_depth {
            return Err(resource_error(
                "normalized_call_depth",
                "normalized execution exceeded its call-depth budget",
            ));
        }
        let locals_bytes = (code.local_count as u64)
            .checked_mul(std::mem::size_of::<Option<NormalizedValue>>() as u64)
            .ok_or_else(|| {
                resource_error(
                    "normalized_local_allocation",
                    "normalized local allocation overflowed",
                )
            })?;
        self.charge_allocation(locals_bytes)?;
        let mut locals = vec![None; code.local_count as usize];
        for (index, argument) in arguments.into_iter().enumerate() {
            locals[index] = Some(argument);
        }
        let id = self.next_frame;
        self.next_frame = self.next_frame.checked_add(1).ok_or_else(|| {
            resource_error(
                "normalized_frame_generation",
                "normalized frame generation counter overflowed",
            )
        })?;
        self.frames.push(Frame {
            id,
            code,
            instruction: 0,
            locals,
            type_arguments,
            stack_base: self.stack.len(),
        });
        self.observation.maximum_call_depth =
            self.observation.maximum_call_depth.max(self.frames.len());
        Ok(())
    }

    fn record(
        &mut self,
        layout: Option<super::value::RecordLayoutIndex>,
        fields: &[NormalizedFieldSelector],
        values: Vec<NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.charge_collection(values.len(), std::mem::size_of::<NormalizedValue>())?;
        if let Some(layout) = layout {
            let field_count = self
                .program
                .records
                .get(layout.0 as usize)
                .ok_or_else(|| {
                    runtime_error(
                        "normalized_record_layout",
                        "normalized record layout index escaped the prepared table",
                    )
                })?
                .fields
                .len();
            if fields.len() != field_count {
                return Err(runtime_error(
                    "normalized_record_field_count",
                    "nominal record construction does not cover its exact layout",
                ));
            }
            let mut slots = vec![None; field_count];
            for (selector, value) in fields.iter().zip(values) {
                let NormalizedFieldSelector::Nominal {
                    layout: field_layout,
                    offset,
                } = selector
                else {
                    return Err(runtime_error(
                        "normalized_record_field_kind",
                        "nominal record contains a structural field selector",
                    ));
                };
                if *field_layout != layout {
                    return Err(runtime_error(
                        "normalized_record_field_layout",
                        "nominal record field belongs to another dense layout",
                    ));
                }
                let slot = slots.get_mut(*offset as usize).ok_or_else(|| {
                    runtime_error(
                        "normalized_record_field_offset",
                        "nominal record field offset escaped its dense layout",
                    )
                })?;
                if slot.replace(value).is_some() {
                    return Err(runtime_error(
                        "normalized_record_field_duplicate",
                        "nominal record repeats one dense field offset",
                    ));
                }
            }
            let slots = slots
                .into_iter()
                .map(|slot| {
                    slot.ok_or_else(|| {
                        runtime_error(
                            "normalized_record_field_missing",
                            "nominal record omits one dense field offset",
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
                layout,
                fields: Arc::new(slots),
            }))
        } else {
            let mut structural = fields
                .iter()
                .cloned()
                .zip(values)
                .map(|(selector, value)| match selector {
                    NormalizedFieldSelector::Structural(name) => Ok((name, value)),
                    NormalizedFieldSelector::Nominal { .. } => Err(runtime_error(
                        "normalized_structural_field_kind",
                        "structural record contains a nominal field selector",
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            structural.sort_by(|left, right| left.0.cmp(&right.0));
            if structural.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(runtime_error(
                    "normalized_structural_field_duplicate",
                    "structural record repeats one exact field name",
                ));
            }
            self.charge_allocation(structural.iter().fold(0_u64, |total, (name, _)| {
                total.saturating_add(name.as_str().len() as u64)
            }))?;
            Ok(NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(structural),
            }))
        }
    }

    fn charge_capability_call(
        &mut self,
        requirement: RequirementIndex,
    ) -> Result<(), ExecutionError> {
        if self.observation.capability_calls >= self.policy.maximum_capability_calls {
            return Err(resource_error(
                "normalized_capability_calls",
                "normalized execution exhausted its capability-call budget",
            ));
        }
        let capabilities = self.capabilities.ok_or_else(capabilities_unbound)?;
        let maximum = capabilities.maximum_calls(requirement)?;
        let canonical = capabilities.canonical_requirement(requirement)?;
        let calls = self.calls_by_requirement.entry(canonical).or_default();
        if *calls >= maximum {
            return Err(resource_error(
                "normalized_grant_calls",
                "normalized execution exhausted one deployment-grant call bound",
            ));
        }
        *calls = calls.saturating_add(1);
        self.observation.capability_calls = self.observation.capability_calls.saturating_add(1);
        Ok(())
    }

    fn charge_collection(&mut self, items: usize, item_bytes: usize) -> Result<(), ExecutionError> {
        let items = items as u64;
        let next = self
            .observation
            .collection_items
            .checked_add(items)
            .ok_or_else(|| {
                resource_error(
                    "normalized_collection_items",
                    "normalized collection-item accounting overflowed",
                )
            })?;
        if next > self.policy.maximum_collection_items {
            return Err(resource_error(
                "normalized_collection_items",
                "normalized execution exhausted its collection-item budget",
            ));
        }
        self.observation.collection_items = next;
        let bytes = items.checked_mul(item_bytes as u64).ok_or_else(|| {
            resource_error(
                "normalized_allocation",
                "normalized collection allocation overflowed",
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
                resource_error(
                    "normalized_allocation",
                    "normalized allocation accounting overflowed",
                )
            })?;
        if next > self.policy.maximum_allocated_bytes {
            return Err(resource_error(
                "normalized_allocation",
                "normalized execution exhausted its allocation budget",
            ));
        }
        self.observation.allocated_bytes = next;
        Ok(())
    }

    fn charge_external_value(&mut self, value: &NormalizedValue) -> Result<(), ExecutionError> {
        let (bytes, items) = value_cost(value)?;
        let next_items = self
            .observation
            .collection_items
            .checked_add(items)
            .ok_or_else(|| {
                resource_error(
                    "normalized_collection_items",
                    "external value item accounting overflowed",
                )
            })?;
        if next_items > self.policy.maximum_collection_items {
            return Err(resource_error(
                "normalized_collection_items",
                "external value exceeds the normalized collection-item budget",
            ));
        }
        self.observation.collection_items = next_items;
        self.charge_allocation(bytes)
    }

    fn current_frame(&self) -> Result<&Frame, ExecutionError> {
        self.frames.last().ok_or_else(|| {
            runtime_error(
                "normalized_frame_missing",
                "normalized execution has no active frame",
            )
        })
    }

    fn current_frame_mut(&mut self) -> Result<&mut Frame, ExecutionError> {
        self.frames.last_mut().ok_or_else(|| {
            runtime_error(
                "normalized_frame_missing",
                "normalized execution has no active frame",
            )
        })
    }

    fn set_local(
        &mut self,
        local: u32,
        value: Option<NormalizedValue>,
    ) -> Result<(), ExecutionError> {
        let destination = self
            .current_frame_mut()?
            .locals
            .get_mut(local as usize)
            .ok_or_else(|| {
                runtime_error(
                    "normalized_local_index",
                    "normalized local index escaped its verified frame",
                )
            })?;
        *destination = value;
        Ok(())
    }

    fn push(&mut self, value: NormalizedValue) -> Result<(), ExecutionError> {
        if self.stack.len() >= self.policy.maximum_value_stack {
            return Err(resource_error(
                "normalized_value_stack",
                "normalized execution exceeded its value-stack budget",
            ));
        }
        self.stack.push(value);
        self.observation.maximum_value_stack =
            self.observation.maximum_value_stack.max(self.stack.len());
        Ok(())
    }

    fn pop(&mut self) -> Result<NormalizedValue, ExecutionError> {
        let base = self.frames.last().map_or(0, |frame| frame.stack_base);
        if self.stack.len() <= base {
            return Err(runtime_error(
                "normalized_stack_underflow",
                "normalized instruction consumed beneath its frame stack",
            ));
        }
        self.stack.pop().ok_or_else(|| {
            runtime_error(
                "normalized_stack_underflow",
                "normalized instruction consumed an empty stack",
            )
        })
    }

    fn pop_many(&mut self, count: usize) -> Result<Vec<NormalizedValue>, ExecutionError> {
        let base = self.frames.last().map_or(0, |frame| frame.stack_base);
        if self.stack.len().saturating_sub(base) < count {
            return Err(runtime_error(
                "normalized_stack_underflow",
                "normalized instruction consumed too many values",
            ));
        }
        let start = self.stack.len() - count;
        Ok(self.stack.split_off(start))
    }

    fn jump(&mut self, target: u32) -> Result<(), ExecutionError> {
        let frame = self.current_frame_mut()?;
        if target as usize >= frame.code.instructions.len() {
            return Err(runtime_error(
                "normalized_jump_target",
                "normalized jump target escaped its verified code",
            ));
        }
        frame.instruction = target as usize;
        Ok(())
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

fn select_field(
    value: NormalizedValue,
    selector: &NormalizedFieldSelector,
) -> Result<NormalizedValue, ExecutionError> {
    match (value, selector) {
        (
            NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }),
            NormalizedFieldSelector::Nominal {
                layout: expected,
                offset,
            },
        ) if layout == *expected => fields.get(*offset as usize).cloned().ok_or_else(|| {
            runtime_error(
                "normalized_field_offset",
                "nominal field offset escaped its runtime record layout",
            )
        }),
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields }),
            NormalizedFieldSelector::Structural(name),
        ) => fields
            .binary_search_by(|(candidate, _)| candidate.cmp(name))
            .ok()
            .map(|index| fields[index].1.clone())
            .ok_or_else(|| type_error("structural record has no selected field")),
        _ => Err(type_error(
            "field selection received a foreign record layout",
        )),
    }
}

fn value_cost(value: &NormalizedValue) -> Result<(u64, u64), ExecutionError> {
    let mut pending = vec![value];
    let mut bytes = 0_u64;
    let mut items = 0_u64;
    while let Some(value) = pending.pop() {
        match value {
            NormalizedValue::Bytes(value) => {
                bytes = bytes.checked_add(value.len() as u64).ok_or_else(|| {
                    resource_error(
                        "normalized_external_value",
                        "external value bytes overflowed",
                    )
                })?;
            }
            NormalizedValue::Text(value) | NormalizedValue::StaticText(value) => {
                bytes = bytes.checked_add(value.len() as u64).ok_or_else(|| {
                    resource_error(
                        "normalized_external_value",
                        "external text bytes overflowed",
                    )
                })?;
            }
            NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) => {
                items = items.checked_add(fields.len() as u64).ok_or_else(|| {
                    resource_error(
                        "normalized_external_value",
                        "external value items overflowed",
                    )
                })?;
                pending.extend(fields.iter());
            }
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                items = items.checked_add(fields.len() as u64).ok_or_else(|| {
                    resource_error(
                        "normalized_external_value",
                        "external value items overflowed",
                    )
                })?;
                for (name, value) in fields.iter() {
                    bytes = bytes
                        .checked_add(name.as_str().len() as u64)
                        .ok_or_else(|| {
                            resource_error(
                                "normalized_external_value",
                                "external structural-name bytes overflowed",
                            )
                        })?;
                    pending.push(value);
                }
            }
            NormalizedValue::Variant { payload, .. } => {
                if let Some(payload) = payload {
                    items = items.checked_add(1).ok_or_else(|| {
                        resource_error(
                            "normalized_external_value",
                            "external variant item count overflowed",
                        )
                    })?;
                    pending.push(payload);
                }
            }
            NormalizedValue::List(values) => {
                items = items.checked_add(values.len() as u64).ok_or_else(|| {
                    resource_error(
                        "normalized_external_value",
                        "external list items overflowed",
                    )
                })?;
                pending.extend(values.iter());
            }
            NormalizedValue::Map(values) => {
                items = items.checked_add(values.len() as u64).ok_or_else(|| {
                    resource_error("normalized_external_value", "external map items overflowed")
                })?;
                for (key, value) in values.iter() {
                    bytes = bytes.checked_add(map_key_bytes(key)).ok_or_else(|| {
                        resource_error(
                            "normalized_external_value",
                            "external map-key bytes overflowed",
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

fn map_key_bytes(key: &NormalizedMapKey) -> u64 {
    match key {
        NormalizedMapKey::Bytes(value) => value.len() as u64,
        NormalizedMapKey::Text(value) => value.len() as u64,
        NormalizedMapKey::Bool(_) | NormalizedMapKey::I64(_) => 0,
    }
}

fn call_core_intrinsic(
    program: &NormalizedProgram,
    function: &super::prepare::NormalizedFunction,
    implementation: &str,
    type_arguments: &[TypeObjectDigest],
    arguments: Vec<NormalizedValue>,
) -> Result<NormalizedValue, ExecutionError> {
    match implementation {
        "identity_host" => match arguments.as_slice() {
            [] => Ok(NormalizedValue::Unit),
            [value] => Ok(value.clone()),
            _ => Err(type_error("identity host received a foreign arity")),
        },
        "core.i64.add" => binary_i64(arguments, i64::checked_add, "integer addition overflow"),
        "core.i64.subtract" => {
            binary_i64(arguments, i64::checked_sub, "integer subtraction overflow")
        }
        "core.i64.multiply" => binary_i64(
            arguments,
            i64::checked_mul,
            "integer multiplication overflow",
        ),
        "core.i64.divide" => {
            let (left, right) = i64_pair(arguments)?;
            left.checked_div(right)
                .map(NormalizedValue::I64)
                .ok_or_else(|| {
                    trap_error(
                        "normalized_integer_division",
                        "integer division by zero or signed overflow",
                    )
                })
        }
        "core.i64.equal" => {
            let (left, right) = i64_pair(arguments)?;
            Ok(NormalizedValue::Bool(left == right))
        }
        "core.i64.less" | "core.i64.less-equal" => {
            let (left, right) = i64_pair(arguments)?;
            Ok(NormalizedValue::Bool(
                if implementation == "core.i64.less" {
                    left < right
                } else {
                    left <= right
                },
            ))
        }
        "core.i64.to-text" => {
            let [NormalizedValue::I64(value)] = arguments.as_slice() else {
                return Err(type_error("integer formatter received a foreign value"));
            };
            Ok(NormalizedValue::text(value.to_string()))
        }
        "core.i64.parse" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("integer parser received a foreign value"));
            };
            parse_normalized_i64(value)
                .map(NormalizedValue::I64)
                .ok_or_else(|| {
                    trap_error(
                        "normalized_integer_parse",
                        "text is not a canonical signed 64-bit integer",
                    )
                })
        }
        "core.i64.parse-result" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("integer parser received a foreign value"));
            };
            let parsed = parse_normalized_i64(value);
            normalized_structural_record([
                ("valid", NormalizedValue::Bool(parsed.is_some())),
                ("value", NormalizedValue::I64(parsed.unwrap_or_default())),
            ])
        }
        "core.bool.not" => {
            let [NormalizedValue::Bool(value)] = arguments.as_slice() else {
                return Err(type_error("boolean intrinsic received a foreign value"));
            };
            Ok(NormalizedValue::Bool(!value))
        }
        "core.bool.and" | "core.bool.or" => {
            let [NormalizedValue::Bool(left), NormalizedValue::Bool(right)] = arguments.as_slice()
            else {
                return Err(type_error("boolean intrinsic received a foreign value"));
            };
            Ok(NormalizedValue::Bool(
                if implementation == "core.bool.and" {
                    *left && *right
                } else {
                    *left || *right
                },
            ))
        }
        "core.text.concat" => {
            let [NormalizedValue::Text(left), NormalizedValue::Text(right)] = arguments.as_slice()
            else {
                return Err(type_error("text intrinsic received a foreign value"));
            };
            let length = left.len().checked_add(right.len()).ok_or_else(|| {
                resource_error(
                    "normalized_text_length",
                    "text concatenation length overflowed",
                )
            })?;
            let mut result = String::with_capacity(length);
            result.push_str(left);
            result.push_str(right);
            Ok(NormalizedValue::Text(Arc::from(result)))
        }
        "core.text.equal" => {
            let [NormalizedValue::Text(left), NormalizedValue::Text(right)] = arguments.as_slice()
            else {
                return Err(type_error("text intrinsic received a foreign value"));
            };
            Ok(NormalizedValue::Bool(left == right))
        }
        "core.text.contains" | "core.text.starts-with" => {
            let [NormalizedValue::Text(value), NormalizedValue::Text(part)] = arguments.as_slice()
            else {
                return Err(type_error("text predicate received a foreign value"));
            };
            Ok(NormalizedValue::Bool(
                if implementation == "core.text.contains" {
                    value.contains(part.as_ref())
                } else {
                    value.starts_with(part.as_ref())
                },
            ))
        }
        "core.text.length" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("text length received a foreign value"));
            };
            Ok(NormalizedValue::I64(normalized_length(value.len())?))
        }
        "core.text.empty" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("text emptiness received a foreign value"));
            };
            Ok(NormalizedValue::Bool(value.is_empty()))
        }
        "core.text.from-static" => {
            let [NormalizedValue::StaticText(value)] = arguments.as_slice() else {
                return Err(type_error(
                    "static-text conversion received a foreign value",
                ));
            };
            Ok(NormalizedValue::text(value.clone()))
        }
        "core.html.escape-text" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("HTML escaping received a foreign value"));
            };
            let mut escaped = String::with_capacity(value.len());
            for character in value.chars() {
                match character {
                    '&' => escaped.push_str("&amp;"),
                    '<' => escaped.push_str("&lt;"),
                    '>' => escaped.push_str("&gt;"),
                    '"' => escaped.push_str("&quot;"),
                    '\'' => escaped.push_str("&#39;"),
                    _ => escaped.push(character),
                }
            }
            Ok(NormalizedValue::text(escaped))
        }
        "core.json.string" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("JSON string encoding received a foreign value"));
            };
            serde_json::to_string(value.as_ref())
                .map(NormalizedValue::text)
                .map_err(|_| {
                    runtime_error(
                        "normalized_json_string_encode",
                        "JSON string encoding failed",
                    )
                })
        }
        "core.json.encode" => {
            let [value] = arguments.as_slice() else {
                return Err(type_error("typed JSON encoding received a foreign arity"));
            };
            let parameter = function.parameters.first().ok_or_else(|| {
                runtime_error(
                    "normalized_json_signature",
                    "typed JSON encoder has no exact parameter type",
                )
            })?;
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => parameter.ty,
                _ => {
                    return Err(type_error(
                        "typed JSON encoder requires one exact runtime type",
                    ));
                }
            };
            encode_typed(program, value, ty, JsonLimits::default())
                .map(NormalizedValue::bytes)
                .map_err(normalized_json_error)
        }
        "core.json.decode-or" => {
            let [NormalizedValue::Bytes(bytes), fallback] = arguments.as_slice() else {
                return Err(type_error("typed JSON decoding received foreign values"));
            };
            let fallback_type = function.parameters.get(1).ok_or_else(|| {
                runtime_error(
                    "normalized_json_signature",
                    "typed JSON decoder has no exact fallback type",
                )
            })?;
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => fallback_type.ty,
                _ => {
                    return Err(type_error(
                        "typed JSON decoder requires one exact runtime type",
                    ));
                }
            };
            let decoded = decode_typed(program, bytes, ty, JsonLimits::default());
            let (valid, value, error) = match decoded {
                Ok(value) => (true, value, String::new()),
                Err(error) => (false, fallback.clone(), error.code),
            };
            normalized_structural_record([
                ("error", NormalizedValue::text(error)),
                ("valid", NormalizedValue::Bool(valid)),
                ("value", value),
            ])
        }
        "core.data.encode" => {
            let [value] = arguments.as_slice() else {
                return Err(type_error("typed data encoding received a foreign arity"));
            };
            let parameter = function.parameters.first().ok_or_else(|| {
                runtime_error(
                    "normalized_data_signature",
                    "typed data encoder has no exact parameter type",
                )
            })?;
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => parameter.ty,
                _ => {
                    return Err(type_error(
                        "typed data encoder requires one exact runtime type",
                    ));
                }
            };
            super::data_codec::encode_typed(program, value, ty)
                .map(NormalizedValue::bytes)
                .map_err(normalized_json_error)
        }
        "core.data.decode-or" => {
            let [NormalizedValue::Bytes(bytes), fallback] = arguments.as_slice() else {
                return Err(type_error("typed data decoding received foreign values"));
            };
            let fallback_type = function.parameters.get(1).ok_or_else(|| {
                runtime_error(
                    "normalized_data_signature",
                    "typed data decoder has no exact fallback type",
                )
            })?;
            let ty = match type_arguments {
                [ty] => *ty,
                [] if function.type_parameters.is_empty() => fallback_type.ty,
                _ => {
                    return Err(type_error(
                        "typed data decoder requires one exact runtime type",
                    ));
                }
            };
            match super::data_codec::decode_typed(program, bytes, ty) {
                Ok(value) => Ok(value),
                Err(_) => Ok(fallback.clone()),
            }
        }
        "core.http.bearer-token" => normalized_bearer_token(program, &arguments),
        "core.http.media-type-is" => {
            let [
                NormalizedValue::Bytes(value),
                NormalizedValue::Text(expected),
            ] = arguments.as_slice()
            else {
                return Err(type_error("media-type predicate received foreign values"));
            };
            Ok(NormalizedValue::Bool(
                crate::platform::http_client::media_type_matches(value, expected),
            ))
        }
        "core.bytes.from-text" => {
            let [NormalizedValue::Text(value)] = arguments.as_slice() else {
                return Err(type_error("text-to-bytes received a foreign value"));
            };
            Ok(NormalizedValue::bytes(value.as_bytes()))
        }
        "core.bytes.to-text" => {
            let [NormalizedValue::Bytes(value)] = arguments.as_slice() else {
                return Err(type_error("bytes-to-text received a foreign value"));
            };
            let value = std::str::from_utf8(value).map_err(|_| {
                trap_error(
                    "normalized_bytes_utf8",
                    "bytes are not a valid UTF-8 text encoding",
                )
            })?;
            Ok(NormalizedValue::text(value))
        }
        "core.bytes.concat" => {
            let [NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)] =
                arguments.as_slice()
            else {
                return Err(type_error("byte concatenation received foreign values"));
            };
            let length = left.len().checked_add(right.len()).ok_or_else(|| {
                resource_error(
                    "normalized_bytes_length",
                    "byte concatenation length overflowed",
                )
            })?;
            let mut value = Vec::with_capacity(length);
            value.extend_from_slice(left);
            value.extend_from_slice(right);
            Ok(NormalizedValue::bytes(value))
        }
        "core.bytes.length" => {
            let [NormalizedValue::Bytes(value)] = arguments.as_slice() else {
                return Err(type_error("byte length received a foreign value"));
            };
            Ok(NormalizedValue::I64(normalized_length(value.len())?))
        }
        "core.bytes.to-hex" => {
            let [NormalizedValue::Bytes(value)] = arguments.as_slice() else {
                return Err(type_error("hex encoding received a foreign value"));
            };
            let capacity = value.len().checked_mul(2).ok_or_else(|| {
                resource_error("normalized_text_length", "hex output length overflowed")
            })?;
            let mut output = String::with_capacity(capacity);
            const HEX: &[u8; 16] = b"0123456789abcdef";
            for byte in value.iter().copied() {
                output.push(char::from(HEX[usize::from(byte >> 4)]));
                output.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
            Ok(NormalizedValue::text(output))
        }
        "core.bytes.equal" => {
            let [NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)] =
                arguments.as_slice()
            else {
                return Err(type_error("byte equality received foreign values"));
            };
            Ok(NormalizedValue::Bool(left == right))
        }
        "core.bytes.blake3" => {
            let [NormalizedValue::Bytes(value)] = arguments.as_slice() else {
                return Err(type_error("BLAKE3 hashing received a foreign value"));
            };
            Ok(NormalizedValue::bytes(
                blake3::hash(value).as_bytes().to_vec(),
            ))
        }
        "core.value.equal" => {
            let [left, right] = arguments.as_slice() else {
                return Err(type_error("value equality received a foreign arity"));
            };
            Ok(NormalizedValue::Bool(normalized_equal(left, right)?))
        }
        "core.list.length" => {
            let [NormalizedValue::List(values)] = arguments.as_slice() else {
                return Err(type_error("list length received a foreign value"));
            };
            let length = i64::try_from(values.len()).map_err(|_| {
                resource_error("normalized_value_length", "list length exceeds i64")
            })?;
            Ok(NormalizedValue::I64(length))
        }
        "core.list.get" => {
            let [NormalizedValue::List(values), NormalizedValue::I64(index)] = arguments.as_slice()
            else {
                return Err(type_error("list lookup received foreign values"));
            };
            let index = usize::try_from(*index).map_err(|_| {
                trap_error(
                    "normalized_list_index",
                    "list index is negative or excessive",
                )
            })?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| trap_error("normalized_list_index", "list index is out of bounds"))
        }
        "core.list.append" => {
            let [NormalizedValue::List(values), value] = arguments.as_slice() else {
                return Err(type_error("list append received foreign values"));
            };
            let capacity = values.len().checked_add(1).ok_or_else(|| {
                resource_error("normalized_list_length", "list length overflowed")
            })?;
            let mut output = Vec::with_capacity(capacity);
            output.extend(values.iter().cloned());
            output.push(value.clone());
            Ok(NormalizedValue::List(Arc::new(output)))
        }
        "core.map.length" => {
            let [NormalizedValue::Map(values)] = arguments.as_slice() else {
                return Err(type_error("map length received a foreign value"));
            };
            Ok(NormalizedValue::I64(normalized_length(values.len())?))
        }
        "core.map.get" | "core.map.contains" | "core.map.get-or" | "core.map.insert" => {
            normalized_map_intrinsic(implementation, arguments)
        }
        _ => Err(runtime_error(
            "normalized_intrinsic_missing",
            "normalized host has no implementation for the exact external declaration",
        )),
    }
}

fn parse_normalized_i64(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if value.is_empty()
        || digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || value == "-0"
    {
        return None;
    }
    value.parse().ok()
}

fn normalized_length(length: usize) -> Result<i64, ExecutionError> {
    i64::try_from(length).map_err(|_| {
        resource_error(
            "normalized_value_length",
            "value length exceeds signed 64-bit range",
        )
    })
}

fn normalized_structural_record<const N: usize>(
    fields: [(&str, NormalizedValue); N],
) -> Result<NormalizedValue, ExecutionError> {
    let mut fields = fields
        .into_iter()
        .map(|(name, value)| {
            Name::new(name.to_owned())
                .map(|name| (name, value))
                .map_err(|_| {
                    runtime_error(
                        "normalized_intrinsic_field",
                        "intrinsic field name is invalid",
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(fields),
    }))
}

fn normalized_record_field<'a>(
    program: &NormalizedProgram,
    record: &'a NormalizedRecord,
    name: &str,
) -> Option<&'a NormalizedValue> {
    match record {
        NormalizedRecord::Structural { fields } => fields
            .iter()
            .find_map(|(candidate, value)| (candidate.as_str() == name).then_some(value)),
        NormalizedRecord::Nominal { layout, fields } => program
            .records
            .get(layout.0 as usize)?
            .fields
            .iter()
            .position(|field| field.name.as_str() == name)
            .and_then(|index| fields.get(index)),
    }
}

fn normalized_bearer_token(
    program: &NormalizedProgram,
    arguments: &[NormalizedValue],
) -> Result<NormalizedValue, ExecutionError> {
    let [NormalizedValue::List(headers)] = arguments else {
        return Err(type_error(
            "bearer-token extraction received a foreign value",
        ));
    };
    let mut token = None;
    for header in headers.iter() {
        let NormalizedValue::Record(record) = header else {
            return Err(type_error("bearer-token header is not a record"));
        };
        let (Some(NormalizedValue::Text(name)), Some(NormalizedValue::Bytes(value))) = (
            normalized_record_field(program, record, "name"),
            normalized_record_field(program, record, "value"),
        ) else {
            return Err(type_error("bearer-token header has foreign fields"));
        };
        if !name.eq_ignore_ascii_case("authorization") {
            continue;
        }
        if token.is_some() {
            return Ok(NormalizedValue::text(""));
        }
        let Ok(value) = std::str::from_utf8(value) else {
            return Ok(NormalizedValue::text(""));
        };
        let Some(value) = value.strip_prefix("Bearer ") else {
            return Ok(NormalizedValue::text(""));
        };
        if value.is_empty()
            || value.len() > 512
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Ok(NormalizedValue::text(""));
        }
        token = Some(value.to_owned());
    }
    Ok(NormalizedValue::text(token.unwrap_or_default()))
}

fn normalized_map_intrinsic(
    implementation: &str,
    arguments: Vec<NormalizedValue>,
) -> Result<NormalizedValue, ExecutionError> {
    let values = match arguments.first() {
        Some(NormalizedValue::Map(values)) => values,
        _ => return Err(type_error("map intrinsic received a foreign map")),
    };
    let key = arguments
        .get(1)
        .cloned()
        .and_then(NormalizedMapKey::from_value)
        .ok_or_else(|| {
            trap_error(
                "normalized_map_key",
                "map key is not a deterministically ordered primitive",
            )
        })?;
    match implementation {
        "core.map.get" if arguments.len() == 2 => values
            .get(&key)
            .cloned()
            .ok_or_else(|| trap_error("normalized_map_key_absent", "map lookup key is absent")),
        "core.map.contains" if arguments.len() == 2 => {
            Ok(NormalizedValue::Bool(values.contains_key(&key)))
        }
        "core.map.get-or" if arguments.len() == 3 => Ok(values
            .get(&key)
            .cloned()
            .unwrap_or_else(|| arguments[2].clone())),
        "core.map.insert" if arguments.len() == 3 => {
            let mut output = values.as_ref().clone();
            output.insert(key, arguments[2].clone());
            Ok(NormalizedValue::Map(Arc::new(output)))
        }
        _ => Err(type_error("map intrinsic received a foreign arity")),
    }
}

fn normalized_json_error(error: crate::platform::diagnostic::Diagnostic) -> ExecutionError {
    ExecutionError::new(
        if error.class == crate::platform::diagnostic::DiagnosticClass::Resource {
            ExecutionFailureClass::Resource
        } else {
            ExecutionFailureClass::Infrastructure
        },
        error.code,
        "typed JSON operation failed",
    )
}

pub(crate) fn normalized_equal(
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
            equal_sequences(left, right)
        }
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
            NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
        ) if left.len() == right.len() => {
            for ((left_name, left), (right_name, right)) in left.iter().zip(right.iter()) {
                if left_name != right_name || !normalized_equal(left, right)? {
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
            (Some(left), Some(right)) => normalized_equal(left, right),
            _ => Ok(false),
        },
        (NormalizedValue::List(left), NormalizedValue::List(right))
            if left.len() == right.len() =>
        {
            equal_sequences(left, right)
        }
        (NormalizedValue::Map(left), NormalizedValue::Map(right)) if left.len() == right.len() => {
            for (key, left) in left.iter() {
                let Some(right) = right.get(key) else {
                    return Ok(false);
                };
                if !normalized_equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (NormalizedValue::Function { .. }, _) | (_, NormalizedValue::Function { .. }) => {
            Err(trap_error(
                "normalized_value_not_comparable",
                "functions do not support semantic equality",
            ))
        }
        (NormalizedValue::Resource(_), _) | (_, NormalizedValue::Resource(_)) => Err(trap_error(
            "normalized_value_not_comparable",
            "live resources do not support semantic equality",
        )),
        _ => Ok(false),
    }
}

fn equal_sequences(
    left: &[NormalizedValue],
    right: &[NormalizedValue],
) -> Result<bool, ExecutionError> {
    for (left, right) in left.iter().zip(right.iter()) {
        if !normalized_equal(left, right)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn binary_i64(
    arguments: Vec<NormalizedValue>,
    operation: fn(i64, i64) -> Option<i64>,
    message: &'static str,
) -> Result<NormalizedValue, ExecutionError> {
    let (left, right) = i64_pair(arguments)?;
    operation(left, right)
        .map(NormalizedValue::I64)
        .ok_or_else(|| trap_error("normalized_integer_overflow", message))
}

fn i64_pair(arguments: Vec<NormalizedValue>) -> Result<(i64, i64), ExecutionError> {
    let [NormalizedValue::I64(left), NormalizedValue::I64(right)] = arguments.as_slice() else {
        return Err(type_error("integer intrinsic received a foreign value"));
    };
    Ok((*left, *right))
}

fn validate_policy(policy: NormalizedRunPolicy) -> Result<(), ExecutionError> {
    if policy.instruction_steps == 0
        || policy.maximum_call_depth == 0
        || policy.maximum_value_stack == 0
        || policy.maximum_allocated_bytes == 0
        || policy.maximum_collection_items == 0
        || policy.maximum_capability_calls == 0
    {
        return Err(resource_error(
            "normalized_run_policy",
            "normalized runtime policy dimensions must all be positive",
        ));
    }
    Ok(())
}

fn capabilities_unbound() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_capability_unbound",
        "effectful normalized execution requires exact bound deployment grants",
    )
}

fn type_error(message: &'static str) -> ExecutionError {
    runtime_error("normalized_runtime_type", message)
}

fn trap_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Trap, code, message)
}

fn resource_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::resource(code, message)
}

fn runtime_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}
