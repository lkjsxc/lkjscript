use crate::compile;
use crate::core_ir::{
    self, BOOL_TYPE, BYTES_TYPE, BlockId, CoreProgram, CoreTypeId, CoreTypeKind, FunctionId,
    I64_TYPE, Instruction, SwitchArgument, Terminator, ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::managed::{ByteHandle, ManagedStore};
use crate::ownership::{
    self, EdgeOwnership, EdgeSource, InstructionOwnership, OwnershipPlan, TerminatorOwnership,
    UseAction,
};
use crate::schema::{ByteString, MAXIMUM_BYTE_STRING_BYTES, Node, SemanticType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

pub use crate::managed::{
    MAX_RUN_MANAGED_OBJECTS, MAX_RUN_MANAGED_VISIBLE_BYTES, MAX_RUN_RETAINED_BACKING_BYTES,
};

pub const MAX_RUN_ARGUMENTS: usize = 1_024;
pub const MAX_RUN_FUEL: u64 = 10_000_000;
pub const MAX_RUN_FRAMES: u32 = 100_000;
pub const MAX_RUN_LIVE_CELLS: usize = 65_536;
pub const MAX_RUNTIME_VALUE_DEPTH: usize = 24;
pub const MAX_RUNTIME_VALUE_ITEMS: usize = 4_096;
pub const MAX_RUNTIME_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_RUN_ARGUMENT_BYTE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFieldValue {
    pub field: NodeId,
    pub value: RuntimeValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeValue {
    Unit,
    Bool(bool),
    I64(i64),
    Bytes(ByteString),
    Product {
        ty: NodeId,
        fields: Vec<RuntimeFieldValue>,
    },
    Sum {
        ty: NodeId,
        variant: NodeId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Box<RuntimeValue>>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeValueCode {
    Unit,
    Bool,
    I64,
    Bytes,
    Product,
    Sum,
}
impl RuntimeValueCode {
    pub const ALL: [Self; 6] = [
        Self::Unit,
        Self::Bool,
        Self::I64,
        Self::Bytes,
        Self::Product,
        Self::Sum,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::Bytes => "bytes",
            Self::Product => "product",
            Self::Sum => "sum",
        }
    }
}
impl RuntimeValue {
    #[cfg(test)]
    pub(crate) const fn code(&self) -> RuntimeValueCode {
        match self {
            Self::Unit => RuntimeValueCode::Unit,
            Self::Bool(_) => RuntimeValueCode::Bool,
            Self::I64(_) => RuntimeValueCode::I64,
            Self::Bytes(_) => RuntimeValueCode::Bytes,
            Self::Product { .. } => RuntimeValueCode::Product,
            Self::Sum { .. } => RuntimeValueCode::Sum,
        }
    }
    fn semantic_type(&self) -> SemanticType {
        match self {
            Self::Unit => SemanticType::Unit,
            Self::Bool(_) => SemanticType::Bool,
            Self::I64(_) => SemanticType::I64,
            Self::Bytes(_) => SemanticType::Bytes,
            Self::Product { ty, .. } | Self::Sum { ty, .. } => SemanticType::Nominal(*ty),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPolicy {
    pub fuel: u64,
    pub maximum_frames: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub value: RuntimeValue,
    pub compile_nanoseconds: u64,
    pub execute_nanoseconds: u64,
}

pub(crate) fn compile_and_run(
    snapshot: &Snapshot,
    entry: NodeId,
    arguments: &[RuntimeValue],
    policy: RunPolicy,
) -> Result<RunResult> {
    validate_policy(policy)?;
    let result_type = validate_invocation(snapshot, entry, arguments)?;
    preflight_result(snapshot, result_type, entry)?;
    let compile_started = Instant::now();
    let program = compile::compile(snapshot, entry)?;
    let compile_nanoseconds = nanos(compile_started.elapsed().as_nanos());
    let entry_function = program
        .functions
        .get(index(program.entry.0, "entry function")?)
        .ok_or_else(|| invalid_ir("entry function is out of bounds"))?;
    let argument_cells =
        entry_function
            .parameters
            .iter()
            .try_fold(0_usize, |total, parameter| {
                total
                    .checked_add(core_ir::type_cells(
                        &program,
                        core_ir::value_type(entry_function, *parameter)?,
                    )?)
                    .ok_or_else(|| invalid_ir("entry argument cell count overflowed"))
            })?;
    ensure_peak_cells(frame_cells(entry_function)?, argument_cells, 0, entry)?;
    let mut managed = InvocationStore::default();
    let mut flat_arguments = Vec::with_capacity(arguments.len());
    for (value, parameter) in arguments.iter().zip(&entry_function.parameters) {
        flat_arguments.push(to_flat(
            &program,
            &mut managed,
            value,
            core_ir::value_type(entry_function, *parameter)?,
            1,
            entry_function.origin,
        )?);
    }
    let execute_started = Instant::now();
    let flat = interpret_with_store(&program, flat_arguments, policy, &mut managed)?;
    preflight_flat_output(&program, &managed, &flat, entry)?;
    let value = from_flat(&program, &managed, &flat, 1, entry)?;
    validate_runtime_value(snapshot, &value, result_type, entry)?;
    if runtime_byte_value_bytes(&value)? > MAXIMUM_BYTE_STRING_BYTES {
        return Err(LkError::new(
            ErrorCode::ResultBytePolicyExceeded,
            "run result exceeds the decoded byte output policy",
        )
        .for_node(entry));
    }
    let execute_nanoseconds = nanos(execute_started.elapsed().as_nanos());
    Ok(RunResult {
        value,
        compile_nanoseconds,
        execute_nanoseconds,
    })
}

fn validate_policy(policy: RunPolicy) -> Result<()> {
    if policy.fuel == 0 || policy.fuel > MAX_RUN_FUEL {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "run fuel must be positive and within the runtime policy",
        ));
    }
    if policy.maximum_frames == 0 || policy.maximum_frames > MAX_RUN_FRAMES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "run maximum_frames must be positive and within the runtime policy",
        ));
    }
    Ok(())
}

fn validate_invocation(
    snapshot: &Snapshot,
    entry: NodeId,
    arguments: &[RuntimeValue],
) -> Result<SemanticType> {
    if arguments.len() > MAX_RUN_ARGUMENTS {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "run argument count exceeds the invocation boundary",
        )
        .for_node(entry));
    }
    let Node::Function {
        parameters, result, ..
    } = snapshot.node(entry)?
    else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "run entry must be a function").for_node(entry),
        );
    };
    if arguments.len() != parameters.len() {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "run argument count disagrees with entry parameters",
        )
        .for_node(entry)
        .with_related(parameters.iter().copied()));
    }
    let mut total_items = 0_usize;
    let mut total_bytes = 0_usize;
    let mut total_byte_values = 0_usize;
    for (argument, parameter) in arguments.iter().zip(parameters) {
        let Node::Parameter { ty, .. } = snapshot.node(*parameter)? else {
            return Err(invalid_ir("entry parameter slot is not a parameter").for_node(*parameter));
        };
        total_items = total_items
            .checked_add(validate_runtime_value(snapshot, argument, *ty, *parameter)?)
            .ok_or_else(|| value_policy(*parameter, "runtime value item accounting overflowed"))?;
        if total_items > MAX_RUNTIME_VALUE_ITEMS {
            return Err(value_policy(
                *parameter,
                "run arguments exceed runtime value item policy",
            ));
        }
        total_bytes = total_bytes
            .checked_add(runtime_value_policy_metrics(argument)?.1)
            .ok_or_else(|| value_policy(*parameter, "runtime value byte accounting overflowed"))?;
        if total_bytes > MAX_RUNTIME_VALUE_BYTES {
            return Err(value_policy(
                *parameter,
                "run arguments exceed structural runtime value byte policy",
            ));
        }
        total_byte_values = total_byte_values
            .checked_add(runtime_byte_value_bytes(argument)?)
            .ok_or_else(|| value_policy(*parameter, "runtime byte input accounting overflowed"))?;
        if total_byte_values > MAX_RUN_ARGUMENT_BYTE_BYTES {
            return Err(LkError::new(
                ErrorCode::RuntimeByteInputTooLarge,
                "run arguments exceed decoded byte input policy",
            )
            .for_node(*parameter));
        }
    }
    Ok(*result)
}

pub(crate) fn runtime_value_policy_metrics(root: &RuntimeValue) -> Result<(usize, usize)> {
    let mut bytes = 0_usize;
    let mut items = 0_usize;
    let mut stack = vec![(root, 1_usize)];
    while let Some((value, depth)) = stack.pop() {
        if depth > MAX_RUNTIME_VALUE_DEPTH {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime value exceeds nesting depth policy",
            ));
        }
        items = items.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime value item accounting overflowed",
            )
        })?;
        if items > MAX_RUNTIME_VALUE_ITEMS {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime value exceeds item policy",
            ));
        }
        let own_bytes = match value {
            RuntimeValue::Unit => 1,
            RuntimeValue::Bool(_) => 2,
            RuntimeValue::I64(_) => 9,
            RuntimeValue::Bytes(_) => 16,
            RuntimeValue::Product { fields, .. } => {
                for field in fields.iter().rev() {
                    stack.push((&field.value, depth + 1));
                }
                1_usize
                    .checked_add(24)
                    .and_then(|value| value.checked_add(8))
                    .and_then(|value| value.checked_add(fields.len().checked_mul(24)?))
                    .ok_or_else(|| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "runtime value byte accounting overflowed",
                        )
                    })?
            }
            RuntimeValue::Sum { payload, .. } => {
                if let Some(payload) = payload {
                    stack.push((payload, depth + 1));
                }
                1 + 24 + 24 + 1
            }
        };
        bytes = bytes.checked_add(own_bytes).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime value byte accounting overflowed",
            )
        })?;
        if bytes > MAX_RUNTIME_VALUE_BYTES {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "runtime value exceeds structural byte policy",
            ));
        }
    }
    Ok((items, bytes))
}

pub(crate) fn runtime_byte_value_bytes(root: &RuntimeValue) -> Result<usize> {
    let mut total = 0_usize;
    let mut stack = vec![root];
    while let Some(value) = stack.pop() {
        match value {
            RuntimeValue::Bytes(value) => {
                if value.len() > MAXIMUM_BYTE_STRING_BYTES {
                    return Err(LkError::new(
                        ErrorCode::RuntimeByteInputTooLarge,
                        "one runtime byte value exceeds its decoded byte policy",
                    ));
                }
                total = total.checked_add(value.len()).ok_or_else(|| {
                    LkError::new(
                        ErrorCode::RuntimeByteInputTooLarge,
                        "runtime byte input accounting overflowed",
                    )
                })?;
            }
            RuntimeValue::Product { fields, .. } => {
                stack.extend(fields.iter().rev().map(|field| &field.value));
            }
            RuntimeValue::Sum { payload, .. } => {
                if let Some(payload) = payload {
                    stack.push(payload);
                }
            }
            RuntimeValue::Unit | RuntimeValue::Bool(_) | RuntimeValue::I64(_) => {}
        }
    }
    Ok(total)
}

fn validate_runtime_value(
    snapshot: &Snapshot,
    root: &RuntimeValue,
    expected: SemanticType,
    origin: NodeId,
) -> Result<usize> {
    let mut stack = vec![(root, expected, 1_usize)];
    let mut items = 0_usize;
    while let Some((value, expected, depth)) = stack.pop() {
        if depth > MAX_RUNTIME_VALUE_DEPTH {
            return Err(value_policy(
                origin,
                "runtime value exceeds nesting depth policy",
            ));
        }
        items = items
            .checked_add(1)
            .ok_or_else(|| value_policy(origin, "runtime value item accounting overflowed"))?;
        if items > MAX_RUNTIME_VALUE_ITEMS {
            return Err(value_policy(origin, "runtime value exceeds item policy"));
        }
        if value.semantic_type() != expected {
            return Err(LkError::new(
                ErrorCode::RunArgumentMismatch,
                "runtime value type disagrees with its exact contract",
            )
            .for_node(origin)
            .with_types(expected, value.semantic_type()));
        }
        match value {
            RuntimeValue::Unit
            | RuntimeValue::Bool(_)
            | RuntimeValue::I64(_)
            | RuntimeValue::Bytes(_) => {}
            RuntimeValue::Product { ty, fields } => {
                let Node::ProductType {
                    fields: expected_fields,
                    ..
                } = snapshot.node(*ty)?
                else {
                    return Err(LkError::new(
                        ErrorCode::RunArgumentMismatch,
                        "runtime product type is not a product declaration",
                    )
                    .for_node(*ty));
                };
                if fields.len() != expected_fields.len() {
                    return Err(LkError::new(
                        ErrorCode::RunArgumentMismatch,
                        "runtime product field count is malformed",
                    )
                    .for_node(*ty));
                }
                let mut seen = BTreeSet::new();
                for field in fields {
                    if !seen.insert(field.field) {
                        return Err(LkError::new(
                            ErrorCode::RunArgumentMismatch,
                            "runtime product repeats a field identity",
                        )
                        .for_node(field.field));
                    }
                    let Node::ProductField {
                        owner,
                        ty: field_ty,
                        ..
                    } = snapshot.node(field.field)?
                    else {
                        return Err(LkError::new(
                            ErrorCode::RunArgumentMismatch,
                            "runtime product member is not a field",
                        )
                        .for_node(field.field));
                    };
                    if owner != ty || !expected_fields.contains(&field.field) {
                        return Err(LkError::new(
                            ErrorCode::RunArgumentMismatch,
                            "runtime product field is foreign",
                        )
                        .for_node(field.field)
                        .with_related([*ty]));
                    }
                    stack.push((&field.value, *field_ty, depth + 1));
                }
            }
            RuntimeValue::Sum {
                ty,
                variant,
                payload,
            } => {
                let Node::SumType { variants, .. } = snapshot.node(*ty)? else {
                    return Err(LkError::new(
                        ErrorCode::RunArgumentMismatch,
                        "runtime sum type is not a sum declaration",
                    )
                    .for_node(*ty));
                };
                let Node::SumVariant {
                    owner,
                    payload: expected_payload,
                    ..
                } = snapshot.node(*variant)?
                else {
                    return Err(LkError::new(
                        ErrorCode::RunArgumentMismatch,
                        "runtime sum member is not a variant",
                    )
                    .for_node(*variant));
                };
                if owner != ty || !variants.contains(variant) {
                    return Err(LkError::new(
                        ErrorCode::RunArgumentMismatch,
                        "runtime sum variant is foreign",
                    )
                    .for_node(*variant)
                    .with_related([*ty]));
                }
                match (expected_payload, payload) {
                    (None, None) => {}
                    (Some(ty), Some(value)) => stack.push((value, *ty, depth + 1)),
                    _ => {
                        return Err(LkError::new(
                            ErrorCode::RunArgumentMismatch,
                            "runtime sum payload contract is malformed",
                        )
                        .for_node(*variant));
                    }
                }
            }
        }
    }
    Ok(items)
}

#[derive(Clone, Copy, Default)]
struct ValueMetric {
    depth: usize,
    items: usize,
    bytes: usize,
}
fn preflight_result(snapshot: &Snapshot, result: SemanticType, origin: NodeId) -> Result<()> {
    let metric = type_metric(snapshot, result, origin)?;
    if metric.depth > MAX_RUNTIME_VALUE_DEPTH {
        return Err(value_policy(
            origin,
            "mandatory result exceeds runtime value depth policy",
        ));
    }
    if metric.items > MAX_RUNTIME_VALUE_ITEMS {
        return Err(value_policy(
            origin,
            "mandatory result exceeds runtime value item policy",
        ));
    }
    if metric.bytes > MAX_RUNTIME_VALUE_BYTES {
        return Err(value_policy(
            origin,
            "mandatory result exceeds structural runtime value policy",
        ));
    }
    Ok(())
}
fn type_metric(snapshot: &Snapshot, root: SemanticType, origin: NodeId) -> Result<ValueMetric> {
    let primitive = ValueMetric {
        depth: 1,
        items: 1,
        bytes: 16,
    };
    let SemanticType::Nominal(root_id) = root else {
        return Ok(primitive);
    };
    let mut metrics = BTreeMap::<NodeId, ValueMetric>::new();
    let mut visiting = BTreeSet::new();
    let mut stack = vec![(root_id, false)];
    while let Some((id, expanded)) = stack.pop() {
        if metrics.contains_key(&id) {
            continue;
        }
        if !expanded {
            if !visiting.insert(id) {
                return Err(
                    invalid_ir("runtime result type contains a by-value cycle").for_node(id)
                );
            }
            stack.push((id, true));
            for dependency in nominal_dependencies(snapshot, id)?.into_iter().rev() {
                if !metrics.contains_key(&dependency) {
                    stack.push((dependency, false));
                }
            }
            continue;
        }
        visiting.remove(&id);
        let metric = match snapshot.node(id)? {
            Node::ProductType { fields, .. } => {
                let mut result = ValueMetric {
                    depth: 1,
                    items: 1,
                    bytes: 96,
                };
                for field in fields {
                    let Node::ProductField { ty, .. } = snapshot.node(*field)? else {
                        return Err(invalid_ir("runtime result product member is malformed")
                            .for_node(*field));
                    };
                    let child = metric_of(*ty, &metrics, primitive)?;
                    result.depth = result.depth.max(
                        child
                            .depth
                            .checked_add(1)
                            .ok_or_else(|| value_policy(origin, "result depth overflowed"))?,
                    );
                    result.items = result
                        .items
                        .checked_add(child.items)
                        .ok_or_else(|| value_policy(origin, "result item count overflowed"))?;
                    result.bytes = result
                        .bytes
                        .checked_add(child.bytes)
                        .and_then(|v| v.checked_add(96))
                        .ok_or_else(|| value_policy(origin, "result byte estimate overflowed"))?;
                }
                result
            }
            Node::SumType { variants, .. } => {
                let mut maximum = ValueMetric::default();
                for variant in variants {
                    let Node::SumVariant { payload, .. } = snapshot.node(*variant)? else {
                        return Err(
                            invalid_ir("runtime result sum member is malformed").for_node(*variant)
                        );
                    };
                    let child = payload
                        .map(|ty| metric_of(ty, &metrics, primitive))
                        .transpose()?
                        .unwrap_or_default();
                    maximum.depth = maximum.depth.max(child.depth);
                    maximum.items = maximum.items.max(child.items);
                    maximum.bytes = maximum.bytes.max(child.bytes);
                }
                ValueMetric {
                    depth: maximum
                        .depth
                        .checked_add(1)
                        .ok_or_else(|| value_policy(origin, "result depth overflowed"))?,
                    items: maximum
                        .items
                        .checked_add(1)
                        .ok_or_else(|| value_policy(origin, "result items overflowed"))?,
                    bytes: maximum
                        .bytes
                        .checked_add(144)
                        .ok_or_else(|| value_policy(origin, "result bytes overflowed"))?,
                }
            }
            _ => {
                return Err(
                    invalid_ir("runtime result nominal target is not a declaration").for_node(id),
                );
            }
        };
        metrics.insert(id, metric);
    }
    metrics
        .get(&root_id)
        .copied()
        .ok_or_else(|| invalid_ir("runtime result metric is absent"))
}
fn nominal_dependencies(snapshot: &Snapshot, id: NodeId) -> Result<Vec<NodeId>> {
    let mut dependencies = Vec::new();
    match snapshot.node(id)? {
        Node::ProductType { fields, .. } => {
            for field in fields {
                if let Node::ProductField {
                    ty: SemanticType::Nominal(target),
                    ..
                } = snapshot.node(*field)?
                {
                    dependencies.push(*target);
                }
            }
        }
        Node::SumType { variants, .. } => {
            for variant in variants {
                if let Node::SumVariant {
                    payload: Some(SemanticType::Nominal(target)),
                    ..
                } = snapshot.node(*variant)?
                {
                    dependencies.push(*target);
                }
            }
        }
        _ => {
            return Err(
                invalid_ir("runtime nominal dependency target is not a declaration").for_node(id),
            );
        }
    }
    dependencies.sort();
    dependencies.dedup();
    Ok(dependencies)
}
fn metric_of(
    ty: SemanticType,
    metrics: &BTreeMap<NodeId, ValueMetric>,
    primitive: ValueMetric,
) -> Result<ValueMetric> {
    match ty {
        SemanticType::Nominal(id) => metrics
            .get(&id)
            .copied()
            .ok_or_else(|| invalid_ir("runtime result dependency metric is absent")),
        _ => Ok(primitive),
    }
}

type InvocationStore = ManagedStore;

fn invalid_handle(origin: NodeId, message: &str) -> LkError {
    LkError::new(ErrorCode::InvalidManagedHandle, message).for_node(origin)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Cell {
    Scalar(u64),
    Bytes(ByteHandle),
}

impl Default for Cell {
    fn default() -> Self {
        Self::Scalar(0)
    }
}

#[cfg(test)]
impl PartialEq<u64> for Cell {
    fn eq(&self, other: &u64) -> bool {
        matches!(self, Self::Scalar(value) if value == other)
    }
}

#[derive(Clone, Debug)]
struct FlatValue {
    ty: CoreTypeId,
    cells: Vec<Cell>,
}

fn managed_handles(
    plan: &OwnershipPlan,
    ty: CoreTypeId,
    cells: &[Cell],
    origin: NodeId,
) -> Result<Vec<ByteHandle>> {
    let map = plan
        .managed_maps
        .get(index(ty.0, "managed-reference type")?)
        .ok_or_else(|| invalid_ir("managed-reference map is out of bounds"))?;
    let mut result = Vec::new();
    'path: for path in &map.paths {
        for condition in &path.conditions {
            let condition_cell = cells
                .get(index(condition.cell, "managed discriminant cell")?)
                .ok_or_else(|| invalid_ir("managed discriminant cell is out of bounds"))?;
            match condition_cell {
                Cell::Scalar(value) if *value == u64::from(condition.variant) => {}
                Cell::Scalar(_) => continue 'path,
                Cell::Bytes(_) => {
                    return Err(invalid_handle(
                        origin,
                        "managed discriminant cell contains a byte handle",
                    ));
                }
            }
        }
        match cells
            .get(index(path.cell, "managed cell")?)
            .copied()
            .ok_or_else(|| invalid_ir("managed cell is out of bounds"))?
        {
            Cell::Bytes(handle) => result.push(handle),
            Cell::Scalar(_) => {
                return Err(invalid_handle(
                    origin,
                    "managed-reference cell contains the wrong runtime kind",
                ));
            }
        }
    }
    Ok(result)
}

fn share_flat_value(
    managed: &mut InvocationStore,
    plan: &OwnershipPlan,
    value: &FlatValue,
    origin: NodeId,
) -> Result<()> {
    for handle in managed_handles(plan, value.ty, &value.cells, origin)? {
        managed.share(handle, origin)?;
    }
    Ok(())
}

fn to_flat(
    program: &CoreProgram,
    managed: &mut InvocationStore,
    value: &RuntimeValue,
    expected: CoreTypeId,
    depth: usize,
    origin: NodeId,
) -> Result<FlatValue> {
    let mut cells = vec![Cell::default(); core_ir::type_cells(program, expected)?];
    write_flat(program, managed, value, expected, depth, origin, &mut cells)?;
    Ok(FlatValue {
        ty: expected,
        cells,
    })
}

fn write_flat(
    program: &CoreProgram,
    managed: &mut InvocationStore,
    value: &RuntimeValue,
    expected: CoreTypeId,
    depth: usize,
    origin: NodeId,
    destination: &mut [Cell],
) -> Result<()> {
    if depth > MAX_RUNTIME_VALUE_DEPTH {
        return Err(invalid_ir(
            "validated runtime value exceeds conversion depth",
        ));
    }
    let core = core_ir::type_at(program, expected)?;
    if destination.len() != core_ir::type_cells(program, expected)? {
        return Err(invalid_ir(
            "runtime flatten destination has wrong cell count",
        ));
    }
    match (&core.kind, value) {
        (CoreTypeKind::Unit, RuntimeValue::Unit) => Ok(()),
        (CoreTypeKind::Bool, RuntimeValue::Bool(value)) => {
            destination[0] = Cell::Scalar(u64::from(*value));
            Ok(())
        }
        (CoreTypeKind::I64, RuntimeValue::I64(value)) => {
            destination[0] = Cell::Scalar(*value as u64);
            Ok(())
        }
        (CoreTypeKind::Bytes, RuntimeValue::Bytes(value)) => {
            destination[0] = Cell::Bytes(managed.allocate_backing(value.as_slice(), origin)?);
            Ok(())
        }
        (
            CoreTypeKind::Product {
                fields: expected_fields,
            },
            RuntimeValue::Product { fields, .. },
        ) => {
            for expected_field in expected_fields {
                let field = fields
                    .iter()
                    .find(|field| field.field == expected_field.origin)
                    .ok_or_else(|| invalid_ir("validated runtime product field is absent"))?;
                let start = usize::try_from(expected_field.cell_offset)
                    .map_err(|_| invalid_ir("field cell offset overflows host"))?;
                let count = core_ir::type_cells(program, expected_field.ty)?;
                write_flat(
                    program,
                    managed,
                    &field.value,
                    expected_field.ty,
                    depth + 1,
                    expected_field.origin,
                    &mut destination[start..start + count],
                )?;
            }
            Ok(())
        }
        (
            CoreTypeKind::Sum { variants },
            RuntimeValue::Sum {
                variant, payload, ..
            },
        ) => {
            let (ordinal, selected) = variants
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.origin == *variant)
                .ok_or_else(|| invalid_ir("validated runtime variant is absent from Core type"))?;
            destination[0] = Cell::Scalar(
                u64::try_from(ordinal)
                    .map_err(|_| invalid_ir("runtime discriminant overflows u64"))?,
            );
            if let (Some(payload_ty), Some(payload)) = (selected.payload, payload) {
                let count = core_ir::type_cells(program, payload_ty)?;
                write_flat(
                    program,
                    managed,
                    payload,
                    payload_ty,
                    depth + 1,
                    selected.origin,
                    &mut destination[1..1 + count],
                )?;
            }
            Ok(())
        }
        _ => Err(invalid_ir(
            "validated runtime value disagrees with Core type",
        )),
    }
}

fn from_flat(
    program: &CoreProgram,
    managed: &InvocationStore,
    value: &FlatValue,
    depth: usize,
    origin: NodeId,
) -> Result<RuntimeValue> {
    from_flat_cells(program, managed, value.ty, &value.cells, depth, origin)
}

fn preflight_flat_output(
    program: &CoreProgram,
    managed: &InvocationStore,
    value: &FlatValue,
    origin: NodeId,
) -> Result<()> {
    let visible = flat_visible_bytes(program, managed, value.ty, &value.cells, 1, origin)?;
    if visible > MAXIMUM_BYTE_STRING_BYTES {
        return Err(LkError::new(
            ErrorCode::ResultBytePolicyExceeded,
            "run result exceeds the decoded byte output policy",
        )
        .for_node(origin));
    }
    Ok(())
}

fn flat_visible_bytes(
    program: &CoreProgram,
    managed: &InvocationStore,
    ty: CoreTypeId,
    cells: &[Cell],
    depth: usize,
    origin: NodeId,
) -> Result<usize> {
    if depth > MAX_RUNTIME_VALUE_DEPTH {
        return Err(LkError::new(
            ErrorCode::ResultBytePolicyExceeded,
            "run result byte preflight exceeds value depth policy",
        )
        .for_node(origin));
    }
    match &core_ir::type_at(program, ty)?.kind {
        CoreTypeKind::Bytes => match cells.first().copied() {
            Some(Cell::Bytes(handle)) => Ok(managed.bytes(handle, origin)?.len()),
            _ => Err(invalid_handle(
                origin,
                "byte output cell has the wrong kind",
            )),
        },
        CoreTypeKind::Product { fields } => fields.iter().try_fold(0_usize, |total, field| {
            let start = usize::try_from(field.cell_offset)
                .map_err(|_| invalid_ir("output field offset overflows host"))?;
            let count = core_ir::type_cells(program, field.ty)?;
            let child = cells
                .get(start..start + count)
                .ok_or_else(|| invalid_ir("output field cell range is malformed"))?;
            total
                .checked_add(flat_visible_bytes(
                    program,
                    managed,
                    field.ty,
                    child,
                    depth + 1,
                    origin,
                )?)
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::ResultBytePolicyExceeded,
                        "run result visible byte accounting overflowed",
                    )
                    .for_node(origin)
                })
        }),
        CoreTypeKind::Sum { variants } => {
            let ordinal = match cells.first().copied() {
                Some(Cell::Scalar(value)) => usize::try_from(value)
                    .map_err(|_| invalid_ir("output sum discriminant overflows host"))?,
                _ => {
                    return Err(invalid_handle(
                        origin,
                        "output sum discriminant has the wrong kind",
                    ));
                }
            };
            let variant = variants
                .get(ordinal)
                .ok_or_else(|| invalid_ir("output sum discriminant is out of bounds"))?;
            if let Some(payload) = variant.payload {
                let count = core_ir::type_cells(program, payload)?;
                flat_visible_bytes(
                    program,
                    managed,
                    payload,
                    cells
                        .get(1..1 + count)
                        .ok_or_else(|| invalid_ir("output sum payload range is malformed"))?,
                    depth + 1,
                    origin,
                )
            } else {
                Ok(0)
            }
        }
        CoreTypeKind::Unit | CoreTypeKind::Bool | CoreTypeKind::I64 => Ok(0),
    }
}

fn from_flat_cells(
    program: &CoreProgram,
    managed: &InvocationStore,
    value_ty: CoreTypeId,
    cells: &[Cell],
    depth: usize,
    origin: NodeId,
) -> Result<RuntimeValue> {
    if depth > MAX_RUNTIME_VALUE_DEPTH {
        return Err(invalid_ir(
            "verified runtime result exceeds conversion depth",
        ));
    }
    let core = core_ir::type_at(program, value_ty)?;
    match &core.kind {
        CoreTypeKind::Unit => Ok(RuntimeValue::Unit),
        CoreTypeKind::Bool => Ok(RuntimeValue::Bool(
            match cells
                .first()
                .copied()
                .ok_or_else(|| invalid_ir("bool result cell is absent"))?
            {
                Cell::Scalar(value) => value != 0,
                Cell::Bytes(_) => {
                    return Err(invalid_handle(origin, "bool result contains a byte handle"));
                }
            },
        )),
        CoreTypeKind::I64 => Ok(RuntimeValue::I64(
            match cells
                .first()
                .copied()
                .ok_or_else(|| invalid_ir("i64 result cell is absent"))?
            {
                Cell::Scalar(value) => value as i64,
                Cell::Bytes(_) => {
                    return Err(invalid_handle(origin, "i64 result contains a byte handle"));
                }
            },
        )),
        CoreTypeKind::Bytes => {
            let handle = match cells
                .first()
                .copied()
                .ok_or_else(|| invalid_handle(origin, "byte result cell is absent"))?
            {
                Cell::Bytes(handle) => handle,
                Cell::Scalar(_) => {
                    return Err(invalid_handle(
                        origin,
                        "byte result cell has the wrong kind",
                    ));
                }
            };
            let bytes = managed.bytes(handle, origin)?;
            Ok(RuntimeValue::Bytes(ByteString::from_slice(bytes).map_err(
                |_| {
                    LkError::new(
                        ErrorCode::ResultBytePolicyExceeded,
                        "byte result exceeds the public materialization policy",
                    )
                    .for_node(origin)
                },
            )?))
        }
        CoreTypeKind::Product { fields } => {
            let ty = core
                .origin
                .ok_or_else(|| invalid_ir("product Core origin is absent"))?;
            let mut result = Vec::with_capacity(fields.len());
            for field in fields {
                let start = usize::try_from(field.cell_offset)
                    .map_err(|_| invalid_ir("field cell offset overflows host"))?;
                let count = core_ir::type_cells(program, field.ty)?;
                let field_cells = cells
                    .get(start..start + count)
                    .ok_or_else(|| invalid_ir("product field cell range is malformed"))?;
                result.push(RuntimeFieldValue {
                    field: field.origin,
                    value: from_flat_cells(
                        program,
                        managed,
                        field.ty,
                        field_cells,
                        depth + 1,
                        origin,
                    )?,
                });
            }
            Ok(RuntimeValue::Product { ty, fields: result })
        }
        CoreTypeKind::Sum { variants } => {
            let ty = core
                .origin
                .ok_or_else(|| invalid_ir("sum Core origin is absent"))?;
            let discriminant = match cells
                .first()
                .copied()
                .ok_or_else(|| invalid_ir("sum discriminant cell is absent"))?
            {
                Cell::Scalar(value) => value,
                Cell::Bytes(_) => {
                    return Err(invalid_handle(
                        origin,
                        "sum discriminant has the wrong cell kind",
                    ));
                }
            };
            let ordinal = usize::try_from(discriminant)
                .map_err(|_| invalid_ir("sum discriminant overflows host"))?;
            let variant = variants
                .get(ordinal)
                .ok_or_else(|| invalid_ir("sum discriminant is out of bounds"))?;
            let payload = variant
                .payload
                .map(|payload_ty| {
                    let count = core_ir::type_cells(program, payload_ty)?;
                    let payload_cells = cells
                        .get(1..1 + count)
                        .ok_or_else(|| invalid_ir("sum payload cell range is malformed"))?;
                    Ok::<Box<RuntimeValue>, LkError>(Box::new(from_flat_cells(
                        program,
                        managed,
                        payload_ty,
                        payload_cells,
                        depth + 1,
                        origin,
                    )?))
                })
                .transpose()?;
            Ok(RuntimeValue::Sum {
                ty,
                variant: variant.origin,
                payload,
            })
        }
    }
}

#[derive(Clone, Copy)]
struct Continuation {
    result: ValueId,
    origin: NodeId,
    drop_result: bool,
}
struct Frame {
    function: FunctionId,
    block: BlockId,
    instruction: usize,
    cells: Vec<Cell>,
    offsets: Vec<usize>,
    initialized: Vec<bool>,
    continuation: Option<Continuation>,
}

#[cfg(test)]
fn interpret(
    program: &CoreProgram,
    arguments: Vec<FlatValue>,
    policy: RunPolicy,
) -> Result<FlatValue> {
    let mut managed = InvocationStore::default();
    interpret_with_store(program, arguments, policy, &mut managed)
}

fn interpret_with_store(
    program: &CoreProgram,
    arguments: Vec<FlatValue>,
    policy: RunPolicy,
    managed: &mut InvocationStore,
) -> Result<FlatValue> {
    let ownership_plan = ownership::derive(program)?;
    validate_policy(policy)?;
    let mut fuel = policy.fuel;
    let entry = program
        .functions
        .get(index(program.entry.0, "entry function")?)
        .ok_or_else(|| invalid_ir("entry function is out of bounds"))?;
    let mut live_cells = frame_cells(entry)?;
    let entry_scratch = arguments.iter().try_fold(0_usize, |total, value| {
        total
            .checked_add(value.cells.len())
            .ok_or_else(|| invalid_ir("entry argument scratch cell count overflowed"))
    })?;
    ensure_peak_cells(live_cells, entry_scratch, 0, entry.origin)?;
    let entry_frame = new_frame(program, program.entry, &arguments, None)?;
    drop(arguments);
    let mut frames = vec![entry_frame];
    let outcome = (|| -> Result<FlatValue> {
        loop {
            let frame_index = frames
                .len()
                .checked_sub(1)
                .ok_or_else(|| invalid_ir("interpreter frame stack became empty"))?;
            let function_id = frames[frame_index].function;
            let block_id = frames[frame_index].block;
            let function = program
                .functions
                .get(index(function_id.0, "function")?)
                .ok_or_else(|| invalid_ir("runtime function is out of bounds"))?;
            let block = function
                .blocks
                .get(index(block_id.0, "block")?)
                .ok_or_else(|| invalid_ir("runtime block is out of bounds"))?;
            let block_plan = ownership_plan
                .functions
                .get(index(function_id.0, "ownership function")?)
                .and_then(|function| {
                    function
                        .blocks
                        .get(index(block_id.0, "ownership block").ok()?)
                })
                .ok_or_else(|| invalid_ir("verified ownership block plan is absent"))?;
            if frames[frame_index].instruction == 0 {
                drop_frame_values(
                    function,
                    &mut frames[frame_index],
                    &block_plan.entry_drops,
                    &ownership_plan,
                    managed,
                    block.origin,
                )?;
            }
            if frames[frame_index].instruction < block.instructions.len() {
                let instruction_index = frames[frame_index].instruction;
                let instruction = &block.instructions[instruction_index];
                let instruction_plan = block_plan
                    .instructions
                    .get(instruction_index)
                    .ok_or_else(|| invalid_ir("verified ownership instruction plan is absent"))?;
                let copy_cells = instruction_copy_cells(program, function, instruction)?;
                consume_fuel(
                    &mut fuel,
                    1_u64
                        .checked_add(copy_cells)
                        .ok_or_else(|| invalid_ir("instruction fuel cost overflowed"))?,
                    instruction.origin(),
                )?;
                match instruction {
                    Instruction::ConstUnit { result, .. } => {
                        write_unit_direct(program, function, &mut frames[frame_index], *result)?
                    }
                    Instruction::ConstBool { result, value, .. } => write_scalar_direct(
                        program,
                        function,
                        &mut frames[frame_index],
                        *result,
                        BOOL_TYPE,
                        u64::from(*value),
                    )?,
                    Instruction::ConstI64 { result, value, .. } => write_scalar_direct(
                        program,
                        function,
                        &mut frames[frame_index],
                        *result,
                        I64_TYPE,
                        *value as u64,
                    )?,
                    Instruction::ConstBytes {
                        origin,
                        result,
                        value,
                    } => {
                        let handle = managed.allocate_backing(value.as_slice(), *origin)?;
                        write_bytes_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            handle,
                        )?;
                    }
                    Instruction::AddI64 {
                        origin,
                        result,
                        lhs,
                        rhs,
                    } => {
                        let value = require_i64(program, function, &frames[frame_index], *lhs)?
                            .checked_add(require_i64(
                                program,
                                function,
                                &frames[frame_index],
                                *rhs,
                            )?)
                            .ok_or_else(|| {
                                LkError::new(ErrorCode::RuntimeTrap, "i64 addition overflowed")
                                    .for_node(*origin)
                            })?;
                        write_scalar_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            I64_TYPE,
                            value as u64,
                        )?;
                    }
                    Instruction::LtI64 {
                        result, lhs, rhs, ..
                    } => {
                        let value = require_i64(program, function, &frames[frame_index], *lhs)?
                            < require_i64(program, function, &frames[frame_index], *rhs)?;
                        write_scalar_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            BOOL_TYPE,
                            u64::from(value),
                        )?;
                    }
                    Instruction::BytesLen {
                        origin,
                        result,
                        value,
                    } => {
                        let handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *value,
                            *origin,
                        )?;
                        let length = i64::try_from(managed.bytes(handle, *origin)?.len())
                            .map_err(|_| invalid_handle(*origin, "byte length exceeds i64"))?;
                        write_scalar_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            I64_TYPE,
                            length as u64,
                        )?;
                    }
                    Instruction::BytesAt {
                        origin,
                        result,
                        value,
                        index,
                    } => {
                        let handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *value,
                            *origin,
                        )?;
                        let requested =
                            require_i64(program, function, &frames[frame_index], *index)?;
                        let requested = usize::try_from(requested).map_err(|_| {
                            LkError::new(
                                ErrorCode::ByteIndexOutOfBounds,
                                "byte index must be nonnegative and within the visible value",
                            )
                            .for_node(*origin)
                        })?;
                        let octet = managed
                            .bytes(handle, *origin)?
                            .get(requested)
                            .copied()
                            .ok_or_else(|| {
                                LkError::new(
                                    ErrorCode::ByteIndexOutOfBounds,
                                    "byte index is outside the visible value",
                                )
                                .for_node(*origin)
                            })?;
                        write_scalar_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            I64_TYPE,
                            u64::from(octet),
                        )?;
                    }
                    Instruction::BytesSlice {
                        origin,
                        result,
                        value,
                        start,
                        length,
                    } => {
                        let handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *value,
                            *origin,
                        )?;
                        let start = require_i64(program, function, &frames[frame_index], *start)?;
                        let length = require_i64(program, function, &frames[frame_index], *length)?;
                        let slice = managed.slice(handle, start, length, *origin)?;
                        write_bytes_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            slice,
                        )?;
                    }
                    Instruction::BytesEqual {
                        origin,
                        result,
                        lhs,
                        rhs,
                    } => {
                        let lhs_handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *lhs,
                            *origin,
                        )?;
                        let rhs_handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *rhs,
                            *origin,
                        )?;
                        let left = managed.bytes(lhs_handle, *origin)?;
                        let right = managed.bytes(rhs_handle, *origin)?;
                        let mut equal = left.len() == right.len();
                        if equal {
                            for (left, right) in left.iter().zip(right) {
                                consume_fuel(&mut fuel, 1, *origin)?;
                                if left != right {
                                    equal = false;
                                    break;
                                }
                            }
                        }
                        write_scalar_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            BOOL_TYPE,
                            u64::from(equal),
                        )?;
                    }
                    Instruction::BytesConcat {
                        origin,
                        result,
                        lhs,
                        rhs,
                    } => {
                        let lhs_handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *lhs,
                            *origin,
                        )?;
                        let rhs_handle = require_bytes_handle(
                            program,
                            function,
                            &frames[frame_index],
                            *rhs,
                            *origin,
                        )?;
                        let result_len = managed
                            .bytes(lhs_handle, *origin)?
                            .len()
                            .checked_add(managed.bytes(rhs_handle, *origin)?.len())
                            .ok_or_else(|| {
                                LkError::new(
                                    ErrorCode::ByteValueTooLarge,
                                    "byte concatenation length overflowed",
                                )
                                .for_node(*origin)
                            })?;
                        if result_len > MAXIMUM_BYTE_STRING_BYTES {
                            return Err(LkError::new(
                                ErrorCode::ByteValueTooLarge,
                                "byte concatenation result exceeds the byte value policy",
                            )
                            .for_node(*origin));
                        }
                        consume_fuel(
                            &mut fuel,
                            u64::try_from(result_len)
                                .map_err(|_| invalid_ir("concat fuel length overflows u64"))?,
                            *origin,
                        )?;
                        let (value, reused) = managed.concat(
                            lhs_handle,
                            rhs_handle,
                            instruction_plan.reuse_left,
                            MAXIMUM_BYTE_STRING_BYTES,
                            *origin,
                        )?;
                        write_bytes_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            value,
                        )?;
                        if reused {
                            transfer_frame_value(&mut frames[frame_index], *lhs, managed)?;
                        }
                    }
                    Instruction::Call {
                        origin,
                        result,
                        function: callee,
                        arguments,
                    } => {
                        frames[frame_index].instruction += 1;
                        if frames.len()
                            >= usize::try_from(policy.maximum_frames)
                                .map_err(|_| invalid_ir("frame policy overflows host indexes"))?
                        {
                            return Err(LkError::new(
                                ErrorCode::ExecutionFrameExhausted,
                                "execution frame policy exhausted before call",
                            )
                            .for_node(*origin));
                        }
                        let callee_function = program
                            .functions
                            .get(index(callee.0, "callee")?)
                            .ok_or_else(|| invalid_ir("callee is out of bounds"))?;
                        let callee_cells = frame_cells(callee_function)?;
                        let scratch_cells = edge_scratch_cells(program, function, arguments)?;
                        ensure_peak_cells(live_cells, scratch_cells, callee_cells, *origin)?;
                        let values = arguments
                            .iter()
                            .map(|value| {
                                read_value(program, function, &frames[frame_index], *value)
                            })
                            .collect::<Result<Vec<_>>>()?;
                        apply_use_actions(
                            function,
                            &mut frames[frame_index],
                            instruction,
                            instruction_plan,
                            &ownership_plan,
                            managed,
                        )?;
                        let call_result = match instruction {
                            Instruction::Call { result, .. } => *result,
                            _ => unreachable!(),
                        };
                        let mut call_source_drops = instruction_plan.drops_after.clone();
                        let drop_result = call_source_drops.contains(&call_result);
                        call_source_drops.retain(|value| *value != call_result);
                        drop_frame_values(
                            function,
                            &mut frames[frame_index],
                            &call_source_drops,
                            &ownership_plan,
                            managed,
                            *origin,
                        )?;
                        frames.push(new_frame(
                            program,
                            *callee,
                            &values,
                            Some(Continuation {
                                result: *result,
                                origin: *origin,
                                drop_result,
                            }),
                        )?);
                        live_cells = live_cells
                            .checked_add(callee_cells)
                            .ok_or_else(|| invalid_ir("live-cell accounting overflowed"))?;
                        continue;
                    }
                    Instruction::ConstructProduct {
                        result, ty, fields, ..
                    } => {
                        write_product_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            *ty,
                            fields,
                        )?;
                    }
                    Instruction::ProjectField {
                        result,
                        value,
                        field,
                        ..
                    } => {
                        project_field_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            *value,
                            *field,
                        )?;
                    }
                    Instruction::ConstructVariant {
                        result,
                        sum,
                        variant,
                        payload,
                        ..
                    } => {
                        construct_variant_direct(
                            program,
                            function,
                            &mut frames[frame_index],
                            *result,
                            *sum,
                            *variant,
                            *payload,
                        )?;
                    }
                }
                apply_instruction_ownership(
                    function,
                    &mut frames[frame_index],
                    instruction,
                    instruction_plan,
                    &ownership_plan,
                    managed,
                )?;
                frames[frame_index].instruction += 1;
                continue;
            }
            let terminator = &block.terminator;
            match terminator {
                Terminator::Return { origin, value } => {
                    let scratch_cells =
                        core_ir::type_cells(program, core_ir::value_type(function, *value)?)?;
                    let copy_cost = logical_copy_cost(scratch_cells)?;
                    consume_fuel(
                        &mut fuel,
                        1_u64
                            .checked_add(copy_cost)
                            .ok_or_else(|| invalid_ir("return fuel cost overflowed"))?,
                        *origin,
                    )?;
                    ensure_peak_cells(live_cells, scratch_cells, 0, *origin)?;
                    let returned = read_value(program, function, &frames[frame_index], *value)?;
                    let TerminatorOwnership::Return { action, .. } = &block_plan.terminator else {
                        return Err(invalid_ir(
                            "verified ownership return plan has the wrong kind",
                        ));
                    };
                    if *action == UseAction::Transfer {
                        transfer_frame_value(&mut frames[frame_index], *value, managed)?;
                    }
                    drop_frame_values(
                        function,
                        &mut frames[frame_index],
                        &block_plan.cleanup_roots,
                        &ownership_plan,
                        managed,
                        *origin,
                    )?;
                    let continuation = frames[frame_index].continuation;
                    let released = frames[frame_index].cells.len();
                    frames.pop();
                    live_cells = live_cells
                        .checked_sub(released)
                        .ok_or_else(|| invalid_ir("live-cell accounting underflow"))?;
                    if let Some(continuation) = continuation {
                        let caller_index = frames
                            .len()
                            .checked_sub(1)
                            .ok_or_else(|| invalid_ir("call return has no caller frame"))?;
                        let caller_function = program
                            .functions
                            .get(index(frames[caller_index].function.0, "caller")?)
                            .ok_or_else(|| invalid_ir("caller is out of bounds"))?;
                        write_value(
                            program,
                            caller_function,
                            &mut frames[caller_index],
                            continuation.result,
                            &returned,
                        )?;
                        if continuation.drop_result {
                            drop_frame_value(
                                caller_function,
                                &mut frames[caller_index],
                                continuation.result,
                                &ownership_plan,
                                managed,
                                continuation.origin,
                            )?;
                        }
                    } else {
                        if !frames.is_empty() {
                            return Err(invalid_ir("entry return left unexpected frames"));
                        }
                        return Ok(returned);
                    }
                }
                Terminator::Branch {
                    origin,
                    target,
                    arguments,
                } => {
                    let cost = edge_copy_cost(program, function, arguments)?;
                    consume_fuel(
                        &mut fuel,
                        1_u64
                            .checked_add(cost)
                            .ok_or_else(|| invalid_ir("branch fuel cost overflowed"))?,
                        *origin,
                    )?;
                    let scratch_cells = edge_scratch_cells(program, function, arguments)?;
                    ensure_peak_cells(live_cells, scratch_cells, 0, *origin)?;
                    let values = arguments
                        .iter()
                        .map(|value| read_value(program, function, &frames[frame_index], *value))
                        .collect::<Result<Vec<_>>>()?;
                    let TerminatorOwnership::Branch(edge) = &block_plan.terminator else {
                        return Err(invalid_ir(
                            "verified ownership branch plan has the wrong kind",
                        ));
                    };
                    apply_edge_ownership(
                        function,
                        &mut frames[frame_index],
                        &values,
                        edge,
                        &ownership_plan,
                        managed,
                        *origin,
                    )?;
                    enter_block(
                        program,
                        function,
                        &mut frames[frame_index],
                        *target,
                        &values,
                    )?;
                }
                Terminator::CondBranch {
                    origin,
                    condition,
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                } => {
                    consume_fuel(&mut fuel, 1, *origin)?;
                    let selected =
                        if require_bool(program, function, &frames[frame_index], *condition)? {
                            (then_target, then_arguments, true)
                        } else {
                            (else_target, else_arguments, false)
                        };
                    let cost = edge_copy_cost(program, function, selected.1)?;
                    consume_fuel(&mut fuel, cost, *origin)?;
                    let scratch_cells = edge_scratch_cells(program, function, selected.1)?;
                    ensure_peak_cells(live_cells, scratch_cells, 0, *origin)?;
                    let values = selected
                        .1
                        .iter()
                        .map(|value| read_value(program, function, &frames[frame_index], *value))
                        .collect::<Result<Vec<_>>>()?;
                    let TerminatorOwnership::CondBranch {
                        then_edge,
                        else_edge,
                    } = &block_plan.terminator
                    else {
                        return Err(invalid_ir(
                            "verified ownership conditional plan has the wrong kind",
                        ));
                    };
                    let edge = if selected.2 { then_edge } else { else_edge };
                    apply_edge_ownership(
                        function,
                        &mut frames[frame_index],
                        &values,
                        edge,
                        &ownership_plan,
                        managed,
                        *origin,
                    )?;
                    enter_block(
                        program,
                        function,
                        &mut frames[frame_index],
                        *selected.0,
                        &values,
                    )?;
                }
                Terminator::SwitchVariant {
                    origin,
                    scrutinee,
                    arms,
                } => {
                    consume_fuel(&mut fuel, 1, *origin)?;
                    let sum_ty = core_ir::value_type(function, *scrutinee)?;
                    let sum_range = value_range(&frames[frame_index], *scrutinee)?;
                    let discriminant = match frames[frame_index].cells[sum_range.start] {
                        Cell::Scalar(value) => value,
                        Cell::Bytes(_) => {
                            return Err(invalid_handle(
                                *origin,
                                "sum discriminant contains a managed byte handle",
                            ));
                        }
                    };
                    let ordinal = usize::try_from(discriminant)
                        .map_err(|_| invalid_ir("sum discriminant overflows host"))?;
                    let arm = arms.get(ordinal).ok_or_else(|| {
                        invalid_ir("verified switch discriminant is out of bounds")
                    })?;
                    let CoreTypeKind::Sum { variants } = &core_ir::type_at(program, sum_ty)?.kind
                    else {
                        return Err(invalid_ir("verified switch value is not a sum"));
                    };
                    let variant = &variants[ordinal];
                    let (copy_cost, scratch_cells) =
                        switch_edge_cost_and_cells(program, function, arm, variant.payload)?;
                    consume_fuel(&mut fuel, copy_cost, *origin)?;
                    ensure_peak_cells(live_cells, scratch_cells, 0, *origin)?;
                    let mut values = Vec::with_capacity(arm.arguments.len());
                    for argument in &arm.arguments {
                        match argument {
                            SwitchArgument::Value(value) => values.push(read_value(
                                program,
                                function,
                                &frames[frame_index],
                                *value,
                            )?),
                            SwitchArgument::Payload => {
                                let ty = variant.payload.ok_or_else(|| {
                                    invalid_ir("payload marker selected nullary variant")
                                })?;
                                let count = core_ir::type_cells(program, ty)?;
                                values.push(FlatValue {
                                    ty,
                                    cells: frames[frame_index].cells
                                        [sum_range.start + 1..sum_range.start + 1 + count]
                                        .to_vec(),
                                });
                            }
                        }
                    }
                    let TerminatorOwnership::SwitchVariant { arms: edge_plans } =
                        &block_plan.terminator
                    else {
                        return Err(invalid_ir(
                            "verified ownership switch plan has the wrong kind",
                        ));
                    };
                    let edge = edge_plans
                        .get(ordinal)
                        .ok_or_else(|| invalid_ir("verified ownership switch arm is absent"))?;
                    managed.record_borrow()?;
                    apply_edge_ownership(
                        function,
                        &mut frames[frame_index],
                        &values,
                        edge,
                        &ownership_plan,
                        managed,
                        *origin,
                    )?;
                    enter_block(
                        program,
                        function,
                        &mut frames[frame_index],
                        arm.target,
                        &values,
                    )?;
                }
            }
        }
    })();
    if outcome.is_err() {
        for frame in &mut frames {
            let function_index = index(frame.function.0, "cleanup function")?;
            let function = program
                .functions
                .get(function_index)
                .ok_or_else(|| invalid_ir("cleanup function is out of bounds"))?;
            let cleanup_roots = ownership_plan
                .functions
                .get(function_index)
                .and_then(|plan| plan.blocks.get(index(frame.block.0, "cleanup block").ok()?))
                .ok_or_else(|| invalid_ir("cleanup ownership roots are absent"))?;
            drop_frame_values(
                function,
                frame,
                &cleanup_roots.cleanup_roots,
                &ownership_plan,
                managed,
                function.origin,
            )?;
        }
    }
    outcome
}

fn instruction_copy_cells(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    instruction: &Instruction,
) -> Result<u64> {
    match instruction {
        Instruction::Call { arguments, .. }
        | Instruction::ConstructProduct {
            fields: arguments, ..
        } => edge_copy_cost(program, function, arguments),
        Instruction::ProjectField { result, .. } => logical_copy_cost(core_ir::type_cells(
            program,
            core_ir::value_type(function, *result)?,
        )?),
        Instruction::BytesSlice { .. } => Ok(1),
        Instruction::ConstructVariant { sum, payload, .. } => {
            let canonicalization = logical_copy_cost(core_ir::type_cells(program, *sum)?)?;
            payload.map_or(Ok(canonicalization), |payload| {
                canonicalization
                    .checked_add(logical_copy_cost(core_ir::type_cells(
                        program,
                        core_ir::value_type(function, payload)?,
                    )?)?)
                    .ok_or_else(|| invalid_ir("variant fuel cost overflowed"))
            })
        }
        _ => Ok(0),
    }
}

fn logical_copy_cost(cells: usize) -> Result<u64> {
    u64::try_from(cells.max(1)).map_err(|_| invalid_ir("copy cell count overflows u64"))
}

fn edge_copy_cost(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    values: &[ValueId],
) -> Result<u64> {
    values.iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(logical_copy_cost(core_ir::type_cells(
                program,
                core_ir::value_type(function, *value)?,
            )?)?)
            .ok_or_else(|| invalid_ir("edge copy fuel cost overflowed"))
    })
}

fn edge_scratch_cells(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    values: &[ValueId],
) -> Result<usize> {
    values.iter().try_fold(0_usize, |total, value| {
        total
            .checked_add(core_ir::type_cells(
                program,
                core_ir::value_type(function, *value)?,
            )?)
            .ok_or_else(|| invalid_ir("edge scratch cell count overflowed"))
    })
}

fn switch_edge_cost_and_cells(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    arm: &crate::core_ir::SwitchArm,
    payload: Option<CoreTypeId>,
) -> Result<(u64, usize)> {
    let mut cost = 0_u64;
    let mut cells = 0_usize;
    for argument in &arm.arguments {
        let count = match argument {
            SwitchArgument::Value(value) => {
                core_ir::type_cells(program, core_ir::value_type(function, *value)?)?
            }
            SwitchArgument::Payload => payload
                .map(|ty| core_ir::type_cells(program, ty))
                .transpose()?
                .ok_or_else(|| invalid_ir("payload marker selected nullary variant"))?,
        };
        cells = cells
            .checked_add(count)
            .ok_or_else(|| invalid_ir("switch scratch cell count overflowed"))?;
        cost = cost
            .checked_add(logical_copy_cost(count)?)
            .ok_or_else(|| invalid_ir("switch copy fuel cost overflowed"))?;
    }
    Ok((cost, cells))
}

fn ensure_peak_cells(
    live_cells: usize,
    scratch_cells: usize,
    additional_frame_cells: usize,
    origin: NodeId,
) -> Result<()> {
    if live_cells
        .checked_add(scratch_cells)
        .and_then(|total| total.checked_add(additional_frame_cells))
        .is_none_or(|total| total > MAX_RUN_LIVE_CELLS)
    {
        return Err(LkError::new(
            ErrorCode::ExecutionFrameExhausted,
            "execution live-cell materialized peak policy exhausted before allocation or copy",
        )
        .for_node(origin));
    }
    Ok(())
}
fn write_unit_direct(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    result: ValueId,
) -> Result<()> {
    let range = prepare_direct_write(program, function, frame, result, core_ir::UNIT_TYPE)?;
    if !range.is_empty() {
        return Err(invalid_ir("unit runtime write has nonzero cells"));
    }
    mark_initialized(frame, result)
}

fn write_scalar_direct(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    result: ValueId,
    ty: CoreTypeId,
    value: u64,
) -> Result<()> {
    let range = prepare_direct_write(program, function, frame, result, ty)?;
    if range.len() != 1 {
        return Err(invalid_ir("scalar runtime write has wrong cell count"));
    }
    frame.cells[range.start] = Cell::Scalar(value);
    mark_initialized(frame, result)
}

fn write_bytes_direct(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    result: ValueId,
    handle: ByteHandle,
) -> Result<()> {
    let range = prepare_direct_write(program, function, frame, result, BYTES_TYPE)?;
    if range.len() != 1 {
        return Err(invalid_ir(
            "managed byte runtime write has wrong cell count",
        ));
    }
    frame.cells[range.start] = Cell::Bytes(handle);
    mark_initialized(frame, result)
}

fn write_product_direct(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    result: ValueId,
    ty: CoreTypeId,
    fields: &[ValueId],
) -> Result<()> {
    let result_range = prepare_direct_write(program, function, frame, result, ty)?;
    let mut destination = result_range.start;
    for field in fields {
        let source = value_range(frame, *field)?;
        let count = source.len();
        frame.cells.copy_within(source, destination);
        destination = destination
            .checked_add(count)
            .ok_or_else(|| invalid_ir("product destination cell offset overflowed"))?;
    }
    if destination != result_range.end {
        return Err(invalid_ir("product result cell range is malformed"));
    }
    mark_initialized(frame, result)
}

fn project_field_direct(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    result: ValueId,
    value: ValueId,
    field_index: u32,
) -> Result<()> {
    let owner = core_ir::value_type(function, value)?;
    let CoreTypeKind::Product { fields } = &core_ir::type_at(program, owner)?.kind else {
        return Err(invalid_ir("verified projection operand is not a product"));
    };
    let field = fields
        .get(index(field_index, "field")?)
        .ok_or_else(|| invalid_ir("verified projection field is out of bounds"))?;
    let source_value = value_range(frame, value)?;
    let field_offset = usize::try_from(field.cell_offset)
        .map_err(|_| invalid_ir("field cell offset overflows host"))?;
    let count = core_ir::type_cells(program, field.ty)?;
    let source = source_value.start + field_offset..source_value.start + field_offset + count;
    let destination = prepare_direct_write(program, function, frame, result, field.ty)?;
    frame.cells.copy_within(source, destination.start);
    mark_initialized(frame, result)
}

fn construct_variant_direct(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    result: ValueId,
    sum: CoreTypeId,
    variant: u32,
    payload: Option<ValueId>,
) -> Result<()> {
    let destination = prepare_direct_write(program, function, frame, result, sum)?;
    frame.cells[destination.clone()].fill(Cell::default());
    frame.cells[destination.start] = Cell::Scalar(u64::from(variant));
    if let Some(payload) = payload {
        let source = value_range(frame, payload)?;
        frame.cells.copy_within(source, destination.start + 1);
    }
    mark_initialized(frame, result)
}

fn prepare_direct_write(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &Frame,
    id: ValueId,
    expected: CoreTypeId,
) -> Result<std::ops::Range<usize>> {
    if core_ir::value_type(function, id)? != expected {
        return Err(invalid_ir("direct runtime write has wrong Core type"));
    }
    let range = value_range_unchecked(frame, id)?;
    if range.len() != core_ir::type_cells(program, expected)? {
        return Err(invalid_ir("direct runtime write has wrong cell count"));
    }
    let value_index = index(id.0, "value")?;
    if frame.initialized.get(value_index).copied().unwrap_or(false) {
        return Err(invalid_ir("runtime value was defined twice in one block"));
    }
    Ok(range)
}

fn mark_initialized(frame: &mut Frame, id: ValueId) -> Result<()> {
    let value_index = index(id.0, "value")?;
    *frame
        .initialized
        .get_mut(value_index)
        .ok_or_else(|| invalid_ir("runtime result value is out of bounds"))? = true;
    Ok(())
}

fn value_range(frame: &Frame, id: ValueId) -> Result<std::ops::Range<usize>> {
    let value_index = index(id.0, "value")?;
    if !frame.initialized.get(value_index).copied().unwrap_or(false) {
        return Err(invalid_ir("runtime operand is unavailable in this block"));
    }
    value_range_unchecked(frame, id)
}

fn value_range_unchecked(frame: &Frame, id: ValueId) -> Result<std::ops::Range<usize>> {
    let value_index = index(id.0, "value")?;
    let start = *frame
        .offsets
        .get(value_index)
        .ok_or_else(|| invalid_ir("runtime value cell start is out of bounds"))?;
    let end = *frame
        .offsets
        .get(value_index + 1)
        .ok_or_else(|| invalid_ir("runtime value cell end is out of bounds"))?;
    Ok(start..end)
}

fn frame_cells(function: &crate::core_ir::CoreFunction) -> Result<usize> {
    usize::try_from(function.frame_cells)
        .map_err(|_| invalid_ir("frame cell footprint overflows host indexes"))
}
fn new_frame(
    program: &CoreProgram,
    function_id: FunctionId,
    arguments: &[FlatValue],
    continuation: Option<Continuation>,
) -> Result<Frame> {
    let function = program
        .functions
        .get(index(function_id.0, "function")?)
        .ok_or_else(|| invalid_ir("callee function is out of bounds"))?;
    if arguments.len() != function.parameters.len() {
        return Err(invalid_ir(
            "runtime call argument count disagrees with verified function",
        ));
    }
    let mut offsets = Vec::with_capacity(function.value_types.len() + 1);
    offsets.push(0);
    let mut next = 0_usize;
    for ty in &function.value_types {
        next = next
            .checked_add(core_ir::type_cells(program, *ty)?)
            .ok_or_else(|| invalid_ir("frame cell offsets overflow host"))?;
        offsets.push(next);
    }
    let mut frame = Frame {
        function: function_id,
        block: function.entry,
        instruction: 0,
        cells: vec![Cell::default(); next],
        offsets,
        initialized: vec![false; function.value_types.len()],
        continuation,
    };
    bind_parameters(
        program,
        function,
        &mut frame,
        &function.parameters,
        arguments,
    )?;
    Ok(frame)
}
fn enter_block(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    target: BlockId,
    arguments: &[FlatValue],
) -> Result<()> {
    let block = function
        .blocks
        .get(index(target.0, "block")?)
        .ok_or_else(|| invalid_ir("branch target is out of bounds"))?;
    if arguments.len() != block.parameters.len() {
        return Err(invalid_ir(
            "runtime branch argument count disagrees with verified block",
        ));
    }
    frame.initialized.fill(false);
    frame.block = target;
    frame.instruction = 0;
    bind_parameters(program, function, frame, &block.parameters, arguments)
}
fn bind_parameters(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    parameters: &[ValueId],
    arguments: &[FlatValue],
) -> Result<()> {
    for (parameter, argument) in parameters.iter().zip(arguments) {
        write_value(program, function, frame, *parameter, argument)?;
    }
    Ok(())
}
fn write_value(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    id: ValueId,
    value: &FlatValue,
) -> Result<()> {
    let value_index = index(id.0, "value")?;
    let expected = core_ir::value_type(function, id)?;
    if value.ty != expected || value.cells.len() != core_ir::type_cells(program, expected)? {
        return Err(invalid_ir(
            "runtime value disagrees with verified Core value type",
        ));
    }
    let range = value_range_unchecked(frame, id)?;
    let initialized = frame
        .initialized
        .get_mut(value_index)
        .ok_or_else(|| invalid_ir("runtime result value is out of bounds"))?;
    if *initialized {
        return Err(invalid_ir("runtime value was defined twice in one block"));
    }
    frame.cells[range].copy_from_slice(&value.cells);
    *initialized = true;
    Ok(())
}
fn read_value(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &Frame,
    id: ValueId,
) -> Result<FlatValue> {
    let ty = core_ir::value_type(function, id)?;
    let range = value_range(frame, id)?;
    let cells = frame
        .cells
        .get(range)
        .ok_or_else(|| invalid_ir("runtime value cell range is out of bounds"))?
        .to_vec();
    if cells.len() != core_ir::type_cells(program, ty)? {
        return Err(invalid_ir("runtime value cell range has wrong length"));
    }
    Ok(FlatValue { ty, cells })
}

fn share_frame_value(
    function: &crate::core_ir::CoreFunction,
    frame: &Frame,
    value: ValueId,
    plan: &OwnershipPlan,
    managed: &mut InvocationStore,
    origin: NodeId,
) -> Result<()> {
    let ty = core_ir::value_type(function, value)?;
    let range = value_range(frame, value)?;
    for handle in managed_handles(plan, ty, &frame.cells[range], origin)? {
        managed.share(handle, origin)?;
    }
    Ok(())
}

fn transfer_frame_value(
    frame: &mut Frame,
    value: ValueId,
    managed: &mut InvocationStore,
) -> Result<()> {
    let initialized = frame
        .initialized
        .get_mut(index(value.0, "ownership transfer value")?)
        .ok_or_else(|| invalid_ir("ownership transfer value is out of bounds"))?;
    if !*initialized {
        return Err(invalid_ir("ownership transfer source is not initialized"));
    }
    *initialized = false;
    managed.record_transfer()
}

fn drop_frame_value(
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    value: ValueId,
    plan: &OwnershipPlan,
    managed: &mut InvocationStore,
    origin: NodeId,
) -> Result<()> {
    let value_index = index(value.0, "ownership drop value")?;
    if !frame
        .initialized
        .get(value_index)
        .copied()
        .ok_or_else(|| invalid_ir("ownership drop value is out of bounds"))?
    {
        return Ok(());
    }
    let ty = core_ir::value_type(function, value)?;
    let range = value_range_unchecked(frame, value)?;
    let handles = managed_handles(plan, ty, &frame.cells[range], origin)?;
    frame.initialized[value_index] = false;
    for handle in handles {
        managed.drop_claim(handle, origin)?;
    }
    Ok(())
}

fn drop_frame_values(
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    values: &[ValueId],
    plan: &OwnershipPlan,
    managed: &mut InvocationStore,
    origin: NodeId,
) -> Result<()> {
    for value in values {
        drop_frame_value(function, frame, *value, plan, managed, origin)?;
    }
    Ok(())
}

fn apply_instruction_ownership(
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    instruction: &Instruction,
    instruction_plan: &InstructionOwnership,
    plan: &OwnershipPlan,
    managed: &mut InvocationStore,
) -> Result<()> {
    apply_use_actions(
        function,
        frame,
        instruction,
        instruction_plan,
        plan,
        managed,
    )?;
    drop_frame_values(
        function,
        frame,
        &instruction_plan.drops_after,
        plan,
        managed,
        instruction.origin(),
    )
}

fn apply_use_actions(
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    instruction: &Instruction,
    instruction_plan: &InstructionOwnership,
    plan: &OwnershipPlan,
    managed: &mut InvocationStore,
) -> Result<()> {
    for (value, action) in &instruction_plan.uses {
        match action {
            UseAction::Immediate => {}
            UseAction::Borrow => managed.record_borrow()?,
            UseAction::Share if matches!(instruction, Instruction::ProjectField { .. }) => {
                let result = match instruction {
                    Instruction::ProjectField { result, .. } => *result,
                    _ => unreachable!(),
                };
                share_frame_value(function, frame, result, plan, managed, instruction.origin())?;
            }
            UseAction::Share => {
                share_frame_value(function, frame, *value, plan, managed, instruction.origin())?
            }
            UseAction::Transfer => transfer_frame_value(frame, *value, managed)?,
        }
    }
    Ok(())
}

fn apply_edge_ownership(
    function: &crate::core_ir::CoreFunction,
    frame: &mut Frame,
    values: &[FlatValue],
    edge: &EdgeOwnership,
    plan: &OwnershipPlan,
    managed: &mut InvocationStore,
    origin: NodeId,
) -> Result<()> {
    if values.len() != edge.sources.len() {
        return Err(invalid_ir(
            "runtime edge values disagree with the verified ownership plan",
        ));
    }
    for (value, (source, action)) in values.iter().zip(&edge.sources) {
        match action {
            UseAction::Immediate => {}
            UseAction::Borrow => managed.record_borrow()?,
            UseAction::Share => share_flat_value(managed, plan, value, origin)?,
            UseAction::Transfer => match source {
                EdgeSource::Value(source) => transfer_frame_value(frame, *source, managed)?,
                EdgeSource::Payload => {
                    return Err(invalid_ir(
                        "payload ownership cannot transfer independently of its wrapper",
                    ));
                }
            },
        }
    }
    drop_frame_values(function, frame, &edge.drops, plan, managed, origin)
}
fn require_i64(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &Frame,
    id: ValueId,
) -> Result<i64> {
    if core_ir::value_type(function, id)? != I64_TYPE {
        return Err(invalid_ir("verified i64 value has a non-i64 runtime type"));
    }
    let range = value_range(frame, id)?;
    if range.len() != core_ir::type_cells(program, I64_TYPE)? {
        return Err(invalid_ir("verified i64 value has a malformed cell range"));
    }
    match frame.cells[range.start] {
        Cell::Scalar(value) => Ok(value as i64),
        Cell::Bytes(_) => Err(invalid_handle(
            function.origin,
            "i64 runtime cell contains a managed byte handle",
        )),
    }
}
fn require_bool(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &Frame,
    id: ValueId,
) -> Result<bool> {
    if core_ir::value_type(function, id)? != BOOL_TYPE {
        return Err(invalid_ir(
            "verified bool value has a non-bool runtime type",
        ));
    }
    let range = value_range(frame, id)?;
    if range.len() != core_ir::type_cells(program, BOOL_TYPE)? {
        return Err(invalid_ir("verified bool value has a malformed cell range"));
    }
    match frame.cells[range.start] {
        Cell::Scalar(value) => Ok(value != 0),
        Cell::Bytes(_) => Err(invalid_handle(
            function.origin,
            "bool runtime cell contains a managed byte handle",
        )),
    }
}

fn require_bytes_handle(
    program: &CoreProgram,
    function: &crate::core_ir::CoreFunction,
    frame: &Frame,
    id: ValueId,
    origin: NodeId,
) -> Result<ByteHandle> {
    if core_ir::value_type(function, id)? != BYTES_TYPE {
        return Err(invalid_handle(
            origin,
            "verified byte value has a non-byte runtime type",
        ));
    }
    let range = value_range(frame, id)?;
    if range.len() != core_ir::type_cells(program, BYTES_TYPE)? {
        return Err(invalid_handle(
            origin,
            "verified byte value has a malformed cell range",
        ));
    }
    match frame.cells[range.start] {
        Cell::Bytes(handle) => Ok(handle),
        Cell::Scalar(_) => Err(invalid_handle(
            origin,
            "managed byte cell has the wrong runtime kind",
        )),
    }
}
fn consume_fuel(fuel: &mut u64, cost: u64, origin: NodeId) -> Result<()> {
    if *fuel < cost {
        return Err(LkError::new(
            ErrorCode::ExecutionFuelExhausted,
            "execution fuel exhausted before instruction, transfer, or materialized-cell copy",
        )
        .for_node(origin));
    }
    *fuel -= cost;
    Ok(())
}
fn value_policy(origin: NodeId, message: &str) -> LkError {
    LkError::new(ErrorCode::PolicyExceeded, message).for_node(origin)
}
fn index(value: u32, category: &str) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| invalid_ir(format!("runtime {category} index overflows host indexes")))
}
fn invalid_ir(message: impl Into<String>) -> LkError {
    LkError::new(ErrorCode::CoreIrInvalid, message)
}
fn nanos(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
