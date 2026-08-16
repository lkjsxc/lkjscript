use crate::compile;
use crate::core_ir::{
    self, BOOL_TYPE, BlockId, CoreProgram, CoreTypeId, CoreTypeKind, FunctionId, I64_TYPE,
    Instruction, SwitchArgument, Terminator, ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::schema::{Node, SemanticType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

pub const MAX_RUN_ARGUMENTS: usize = 1_024;
pub const MAX_RUN_FUEL: u64 = 10_000_000;
pub const MAX_RUN_FRAMES: u32 = 100_000;
pub const MAX_RUN_LIVE_CELLS: usize = 65_536;
pub const MAX_RUNTIME_VALUE_DEPTH: usize = 24;
pub const MAX_RUNTIME_VALUE_ITEMS: usize = 4_096;
pub const MAX_RUNTIME_VALUE_BYTES: usize = 64 * 1024;

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
    Product,
    Sum,
}
impl RuntimeValueCode {
    pub const ALL: [Self; 5] = [Self::Unit, Self::Bool, Self::I64, Self::Product, Self::Sum];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
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
            Self::Product { .. } => RuntimeValueCode::Product,
            Self::Sum { .. } => RuntimeValueCode::Sum,
        }
    }
    fn semantic_type(&self) -> SemanticType {
        match self {
            Self::Unit => SemanticType::Unit,
            Self::Bool(_) => SemanticType::Bool,
            Self::I64(_) => SemanticType::I64,
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
    let mut flat_arguments = Vec::with_capacity(arguments.len());
    for (value, parameter) in arguments.iter().zip(&entry_function.parameters) {
        flat_arguments.push(to_flat(
            &program,
            value,
            core_ir::value_type(entry_function, *parameter)?,
            1,
        )?);
    }
    let execute_started = Instant::now();
    let flat = interpret(&program, flat_arguments, policy)?;
    let value = from_flat(&program, &flat, 1)?;
    validate_runtime_value(snapshot, &value, result_type, entry)?;
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
                "run arguments exceed encoded runtime value byte policy",
            ));
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
                "runtime value exceeds encoded byte policy",
            ));
        }
    }
    Ok((items, bytes))
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
            RuntimeValue::Unit | RuntimeValue::Bool(_) | RuntimeValue::I64(_) => {}
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
            "mandatory result exceeds encoded runtime value policy",
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

#[derive(Clone, Debug)]
struct FlatValue {
    ty: CoreTypeId,
    cells: Vec<u64>,
}
fn to_flat(
    program: &CoreProgram,
    value: &RuntimeValue,
    expected: CoreTypeId,
    depth: usize,
) -> Result<FlatValue> {
    let mut cells = vec![0; core_ir::type_cells(program, expected)?];
    write_flat(program, value, expected, depth, &mut cells)?;
    Ok(FlatValue {
        ty: expected,
        cells,
    })
}

fn write_flat(
    program: &CoreProgram,
    value: &RuntimeValue,
    expected: CoreTypeId,
    depth: usize,
    destination: &mut [u64],
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
            destination[0] = u64::from(*value);
            Ok(())
        }
        (CoreTypeKind::I64, RuntimeValue::I64(value)) => {
            destination[0] = *value as u64;
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
                    &field.value,
                    expected_field.ty,
                    depth + 1,
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
            destination[0] = u64::try_from(ordinal)
                .map_err(|_| invalid_ir("runtime discriminant overflows u64"))?;
            if let (Some(payload_ty), Some(payload)) = (selected.payload, payload) {
                let count = core_ir::type_cells(program, payload_ty)?;
                write_flat(
                    program,
                    payload,
                    payload_ty,
                    depth + 1,
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

fn from_flat(program: &CoreProgram, value: &FlatValue, depth: usize) -> Result<RuntimeValue> {
    from_flat_cells(program, value.ty, &value.cells, depth)
}

fn from_flat_cells(
    program: &CoreProgram,
    value_ty: CoreTypeId,
    cells: &[u64],
    depth: usize,
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
            cells
                .first()
                .copied()
                .ok_or_else(|| invalid_ir("bool result cell is absent"))?
                != 0,
        )),
        CoreTypeKind::I64 => Ok(RuntimeValue::I64(
            cells
                .first()
                .copied()
                .ok_or_else(|| invalid_ir("i64 result cell is absent"))? as i64,
        )),
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
                    value: from_flat_cells(program, field.ty, field_cells, depth + 1)?,
                });
            }
            Ok(RuntimeValue::Product { ty, fields: result })
        }
        CoreTypeKind::Sum { variants } => {
            let ty = core
                .origin
                .ok_or_else(|| invalid_ir("sum Core origin is absent"))?;
            let ordinal = usize::try_from(
                *cells
                    .first()
                    .ok_or_else(|| invalid_ir("sum discriminant cell is absent"))?,
            )
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
                        payload_ty,
                        payload_cells,
                        depth + 1,
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
}
struct Frame {
    function: FunctionId,
    block: BlockId,
    instruction: usize,
    arena: Vec<u64>,
    offsets: Vec<usize>,
    initialized: Vec<bool>,
    continuation: Option<Continuation>,
}

fn interpret(
    program: &CoreProgram,
    arguments: Vec<FlatValue>,
    policy: RunPolicy,
) -> Result<FlatValue> {
    core_ir::verify(program)?;
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
        if frames[frame_index].instruction < block.instructions.len() {
            let instruction = &block.instructions[frames[frame_index].instruction];
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
                Instruction::AddI64 {
                    origin,
                    result,
                    lhs,
                    rhs,
                } => {
                    let value = require_i64(program, function, &frames[frame_index], *lhs)?
                        .checked_add(require_i64(program, function, &frames[frame_index], *rhs)?)
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
                        .map(|value| read_value(program, function, &frames[frame_index], *value))
                        .collect::<Result<Vec<_>>>()?;
                    frames.push(new_frame(
                        program,
                        *callee,
                        &values,
                        Some(Continuation { result: *result }),
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
                let continuation = frames[frame_index].continuation;
                let released = frames[frame_index].arena.len();
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
                let selected = if require_bool(program, function, &frames[frame_index], *condition)?
                {
                    (then_target, then_arguments)
                } else {
                    (else_target, else_arguments)
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
                let ordinal = usize::try_from(frames[frame_index].arena[sum_range.start])
                    .map_err(|_| invalid_ir("sum discriminant overflows host"))?;
                let arm = arms
                    .get(ordinal)
                    .ok_or_else(|| invalid_ir("verified switch discriminant is out of bounds"))?;
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
                                cells: frames[frame_index].arena
                                    [sum_range.start + 1..sum_range.start + 1 + count]
                                    .to_vec(),
                            });
                        }
                    }
                }
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
    additional_arena_cells: usize,
    origin: NodeId,
) -> Result<()> {
    if live_cells
        .checked_add(scratch_cells)
        .and_then(|total| total.checked_add(additional_arena_cells))
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
    frame.arena[range.start] = value;
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
        frame.arena.copy_within(source, destination);
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
    frame.arena.copy_within(source, destination.start);
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
    frame.arena[destination.clone()].fill(0);
    frame.arena[destination.start] = u64::from(variant);
    if let Some(payload) = payload {
        let source = value_range(frame, payload)?;
        frame.arena.copy_within(source, destination.start + 1);
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
        arena: vec![0; next],
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
    frame.arena[range].copy_from_slice(&value.cells);
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
        .arena
        .get(range)
        .ok_or_else(|| invalid_ir("runtime value cell range is out of bounds"))?
        .to_vec();
    if cells.len() != core_ir::type_cells(program, ty)? {
        return Err(invalid_ir("runtime value cell range has wrong length"));
    }
    Ok(FlatValue { ty, cells })
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
    Ok(frame.arena[range.start] as i64)
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
    Ok(frame.arena[range.start] != 0)
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
mod tests {
    use super::*;
    use crate::core_ir::{
        CoreBlock, CoreField, CoreFunction, CoreType, CoreVariant, SwitchArm, UNIT_TYPE,
    };
    use crate::ids::{Revision, SnapshotHash, WorkspaceId};
    use crate::schema::Node;
    use crate::type_layout::{FieldLayout, LayoutShape, ValueLayout, VariantLayout};
    fn node(serial: u64) -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([0x51; 16]), serial).expect("node")
    }
    fn primitives() -> Vec<CoreType> {
        vec![
            CoreType {
                origin: None,
                kind: CoreTypeKind::Unit,
                layout: ValueLayout {
                    size: 0,
                    align: 1,
                    cells: 0,
                    shape: LayoutShape::Primitive,
                },
            },
            CoreType {
                origin: None,
                kind: CoreTypeKind::Bool,
                layout: ValueLayout {
                    size: 1,
                    align: 1,
                    cells: 1,
                    shape: LayoutShape::Primitive,
                },
            },
            CoreType {
                origin: None,
                kind: CoreTypeKind::I64,
                layout: ValueLayout {
                    size: 8,
                    align: 8,
                    cells: 1,
                    shape: LayoutShape::Primitive,
                },
            },
        ]
    }
    fn scalar_program() -> CoreProgram {
        CoreProgram {
            types: primitives(),
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: vec![],
                result: I64_TYPE,
                value_types: vec![I64_TYPE],
                frame_cells: 1,
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: vec![],
                    instructions: vec![Instruction::ConstI64 {
                        origin: node(3),
                        result: ValueId(0),
                        value: 7,
                    }],
                    terminator: Terminator::Return {
                        origin: node(4),
                        value: ValueId(0),
                    },
                }],
            }],
        }
    }
    fn product_type() -> CoreType {
        CoreType {
            origin: Some(node(10)),
            kind: CoreTypeKind::Product {
                fields: vec![
                    CoreField {
                        origin: node(11),
                        ty: I64_TYPE,
                        cell_offset: 0,
                    },
                    CoreField {
                        origin: node(12),
                        ty: I64_TYPE,
                        cell_offset: 1,
                    },
                ],
            },
            layout: ValueLayout {
                size: 16,
                align: 8,
                cells: 2,
                shape: LayoutShape::Product {
                    fields: vec![
                        FieldLayout {
                            field: node(11),
                            offset: 0,
                            cells: 1,
                        },
                        FieldLayout {
                            field: node(12),
                            offset: 8,
                            cells: 1,
                        },
                    ],
                },
            },
        }
    }

    fn two_cell_product_program(extra_i64_values: usize) -> CoreProgram {
        let mut types = primitives();
        types.push(product_type());
        let mut instructions = vec![
            Instruction::ConstI64 {
                origin: node(20),
                result: ValueId(0),
                value: 7,
            },
            Instruction::ConstI64 {
                origin: node(21),
                result: ValueId(1),
                value: 9,
            },
            Instruction::ConstructProduct {
                origin: node(22),
                result: ValueId(2),
                ty: CoreTypeId(3),
                fields: vec![ValueId(0), ValueId(1)],
            },
        ];
        for offset in 0..extra_i64_values {
            instructions.push(Instruction::ConstI64 {
                origin: node(23),
                result: ValueId(u32::try_from(3 + offset).expect("filler value")),
                value: 0,
            });
        }
        let mut value_types = vec![I64_TYPE, I64_TYPE, CoreTypeId(3)];
        value_types.extend(std::iter::repeat_n(I64_TYPE, extra_i64_values));
        CoreProgram {
            types,
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(19),
                parameters: vec![],
                result: CoreTypeId(3),
                frame_cells: u64::try_from(4 + extra_i64_values).expect("frame cells"),
                value_types,
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(20),
                    parameters: vec![],
                    instructions,
                    terminator: Terminator::Return {
                        origin: node(24),
                        value: ValueId(2),
                    },
                }],
            }],
        }
    }

    fn policy(fuel: u64) -> RunPolicy {
        RunPolicy {
            fuel,
            maximum_frames: 10,
        }
    }

    #[test]
    fn mandatory_deep_nominal_result_rejects_before_compile_or_execution() {
        let workspace = WorkspaceId::from_bytes([0x52; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let depth = MAX_RUNTIME_VALUE_DEPTH as u64 + 1;
        let declarations = (0..depth)
            .map(|index| id(4 + index * 2))
            .collect::<Vec<_>>();
        let function = id(4 + depth * 2);
        let mut nodes = BTreeMap::from([
            (
                id(1),
                Node::WorkspaceRoot {
                    packages: vec![id(2)],
                },
            ),
            (
                id(2),
                Node::Package {
                    owner: id(1),
                    name: "p".into(),
                    modules: vec![id(3)],
                    entry: Some(function),
                },
            ),
            (
                id(3),
                Node::Module {
                    owner: id(2),
                    name: "m".into(),
                    types: declarations.clone(),
                    functions: vec![function],
                },
            ),
            (
                function,
                Node::Function {
                    owner: id(3),
                    name: "main".into(),
                    parameters: vec![],
                    result: SemanticType::Nominal(declarations[0]),
                    body: None,
                },
            ),
        ]);
        for index in 0..depth {
            let declaration = id(4 + index * 2);
            let field = id(5 + index * 2);
            nodes.insert(
                declaration,
                Node::ProductType {
                    owner: id(3),
                    name: format!("T{index}"),
                    fields: vec![field],
                },
            );
            let ty = if index + 1 == depth {
                SemanticType::I64
            } else {
                SemanticType::Nominal(id(4 + (index + 1) * 2))
            };
            nodes.insert(
                field,
                Node::ProductField {
                    owner: declaration,
                    ordinal: 0,
                    name: "next".into(),
                    ty,
                },
            );
        }
        let snapshot = Snapshot {
            workspace,
            revision: Revision::INITIAL,
            root: id(1),
            next_serial: function.serial() + 1,
            tombstones: BTreeSet::new(),
            nodes,
            hash: SnapshotHash::from_bytes([0; 32]),
        };
        let error =
            compile_and_run(&snapshot, function, &[], policy(100)).expect_err("result preflight");
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert!(error.message.contains("mandatory result"));
    }

    #[test]
    fn sum_result_preflight_uses_componentwise_variant_maxima() {
        let workspace = WorkspaceId::from_bytes([0x53; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let root = id(1);
        let wide = id(2);
        let deep_start = id(100);
        let wide_fields = (0..64_u64).map(|offset| id(3 + offset)).collect::<Vec<_>>();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root,
            Node::SumType {
                owner: id(999),
                name: "Mixed".into(),
                variants: vec![id(70), id(71)],
            },
        );
        nodes.insert(
            id(70),
            Node::SumVariant {
                owner: root,
                ordinal: 0,
                name: "wide".into(),
                payload: Some(SemanticType::Nominal(wide)),
            },
        );
        nodes.insert(
            id(71),
            Node::SumVariant {
                owner: root,
                ordinal: 1,
                name: "deep".into(),
                payload: Some(SemanticType::Nominal(deep_start)),
            },
        );
        nodes.insert(
            wide,
            Node::ProductType {
                owner: id(999),
                name: "Wide".into(),
                fields: wide_fields.clone(),
            },
        );
        for (ordinal, field) in wide_fields.iter().enumerate() {
            nodes.insert(
                *field,
                Node::ProductField {
                    owner: wide,
                    ordinal: u32::try_from(ordinal).expect("ordinal"),
                    name: format!("f{ordinal}"),
                    ty: SemanticType::Unit,
                },
            );
        }
        for offset in 0..MAX_RUNTIME_VALUE_DEPTH {
            let declaration = id(100 + u64::try_from(offset * 2).expect("serial"));
            let field = id(101 + u64::try_from(offset * 2).expect("serial"));
            nodes.insert(
                declaration,
                Node::ProductType {
                    owner: id(999),
                    name: format!("Deep{offset}"),
                    fields: vec![field],
                },
            );
            nodes.insert(
                field,
                Node::ProductField {
                    owner: declaration,
                    ordinal: 0,
                    name: "next".into(),
                    ty: if offset + 1 == MAX_RUNTIME_VALUE_DEPTH {
                        SemanticType::Unit
                    } else {
                        SemanticType::Nominal(id(
                            100 + u64::try_from((offset + 1) * 2).expect("serial")
                        ))
                    },
                },
            );
        }
        let snapshot = Snapshot {
            workspace,
            revision: Revision::INITIAL,
            root: id(999),
            next_serial: 1_000,
            tombstones: BTreeSet::new(),
            nodes,
            hash: SnapshotHash::from_bytes([0; 32]),
        };
        let error = preflight_result(&snapshot, SemanticType::Nominal(root), root)
            .expect_err("deep variant must dominate depth while wide variant dominates items");
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert!(error.message.contains("depth"));
    }

    #[test]
    fn deterministic_fuel_charges_base_and_returned_cells_and_traps_leave_runtime_usable() {
        let program = scalar_program();
        assert_eq!(
            interpret(&program, vec![], policy(3))
                .expect("exact fuel")
                .cells,
            vec![7]
        );
        assert_eq!(
            interpret(&program, vec![], policy(2))
                .expect_err("copy fuel")
                .code,
            ErrorCode::ExecutionFuelExhausted
        );
        assert_eq!(
            interpret(&program, vec![], policy(3))
                .expect("later run")
                .cells,
            vec![7]
        );
    }

    #[test]
    fn aggregate_copy_fuel_is_exact_and_peak_overflow_precedes_return_copy() {
        let program = two_cell_product_program(0);
        assert_eq!(
            interpret(&program, vec![], policy(8))
                .expect("exact product fuel")
                .cells,
            vec![7, 9]
        );
        assert_eq!(
            interpret(&program, vec![], policy(7))
                .expect_err("product copy fuel")
                .code,
            ErrorCode::ExecutionFuelExhausted
        );

        let peak = two_cell_product_program(MAX_RUN_LIVE_CELLS - 4);
        let error = interpret(&peak, vec![], policy(MAX_RUN_FUEL))
            .expect_err("return scratch exceeds live-cell peak");
        assert_eq!(error.code, ErrorCode::ExecutionFrameExhausted);
        assert_eq!(error.target, Some(node(24)));
        assert_eq!(
            interpret(&program, vec![], policy(8))
                .expect("runtime remains usable after peak trap")
                .cells,
            vec![7, 9]
        );
    }

    #[test]
    fn fuel_contract_meters_projection_variant_match_call_edge_and_zero_cell_values() {
        let mut types = primitives();
        types.push(product_type());
        types.push(CoreType {
            origin: Some(node(13)),
            kind: CoreTypeKind::Sum {
                variants: vec![
                    CoreVariant {
                        origin: node(14),
                        payload: None,
                        discriminant: 0,
                    },
                    CoreVariant {
                        origin: node(15),
                        payload: Some(CoreTypeId(3)),
                        discriminant: 1,
                    },
                    CoreVariant {
                        origin: node(16),
                        payload: Some(UNIT_TYPE),
                        discriminant: 2,
                    },
                ],
            },
            layout: ValueLayout {
                size: 24,
                align: 8,
                cells: 3,
                shape: LayoutShape::Sum {
                    discriminant_bytes: 1,
                    payload_offset: 8,
                    variants: vec![
                        VariantLayout {
                            variant: node(14),
                            discriminant: 0,
                            payload_size: 0,
                            payload_align: 1,
                            payload_cells: 0,
                        },
                        VariantLayout {
                            variant: node(15),
                            discriminant: 1,
                            payload_size: 16,
                            payload_align: 8,
                            payload_cells: 2,
                        },
                        VariantLayout {
                            variant: node(16),
                            discriminant: 2,
                            payload_size: 0,
                            payload_align: 1,
                            payload_cells: 0,
                        },
                    ],
                },
            },
        });
        let program = CoreProgram {
            types,
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(30),
                parameters: vec![ValueId(0), ValueId(1)],
                result: UNIT_TYPE,
                value_types: vec![UNIT_TYPE, CoreTypeId(3), CoreTypeId(4)],
                frame_cells: 5,
                entry: BlockId(0),
                blocks: vec![],
            }],
        };
        let function = &program.functions[0];
        assert_eq!(
            instruction_copy_cells(
                &program,
                function,
                &Instruction::ProjectField {
                    origin: node(31),
                    result: ValueId(1),
                    value: ValueId(1),
                    field: 0,
                },
            )
            .expect("projection fuel"),
            2
        );
        assert_eq!(
            instruction_copy_cells(
                &program,
                function,
                &Instruction::ConstructVariant {
                    origin: node(32),
                    result: ValueId(2),
                    sum: CoreTypeId(4),
                    variant: 0,
                    payload: None,
                },
            )
            .expect("nullary variant full canonicalization fuel"),
            3
        );
        assert_eq!(
            instruction_copy_cells(
                &program,
                function,
                &Instruction::ConstructVariant {
                    origin: node(33),
                    result: ValueId(2),
                    sum: CoreTypeId(4),
                    variant: 1,
                    payload: Some(ValueId(1)),
                },
            )
            .expect("payload variant canonicalization and copy fuel"),
            5
        );
        assert_eq!(
            instruction_copy_cells(
                &program,
                function,
                &Instruction::ConstructVariant {
                    origin: node(34),
                    result: ValueId(2),
                    sum: CoreTypeId(4),
                    variant: 2,
                    payload: Some(ValueId(0)),
                },
            )
            .expect("zero-cell payload logical copy fuel"),
            4
        );
        assert_eq!(
            edge_copy_cost(&program, function, &[ValueId(0), ValueId(1)])
                .expect("call and edge fuel"),
            3
        );
        let payload_arm = SwitchArm {
            variant: 1,
            target: BlockId(0),
            arguments: vec![SwitchArgument::Payload, SwitchArgument::Value(ValueId(0))],
        };
        assert_eq!(
            switch_edge_cost_and_cells(&program, function, &payload_arm, Some(CoreTypeId(3)),)
                .expect("selected match payload fuel"),
            (3, 2)
        );
    }

    #[test]
    fn selected_large_switch_arm_exhausts_fuel_before_edge_materialization() {
        const ARM_ARGUMENTS: usize = 4_096;
        let mut types = primitives();
        types.push(CoreType {
            origin: Some(node(40)),
            kind: CoreTypeKind::Sum {
                variants: vec![CoreVariant {
                    origin: node(41),
                    payload: None,
                    discriminant: 0,
                }],
            },
            layout: ValueLayout {
                size: 1,
                align: 1,
                cells: 1,
                shape: LayoutShape::Sum {
                    discriminant_bytes: 1,
                    payload_offset: 1,
                    variants: vec![VariantLayout {
                        variant: node(41),
                        discriminant: 0,
                        payload_size: 0,
                        payload_align: 1,
                        payload_cells: 0,
                    }],
                },
            },
        });
        let source_values = (1..=ARM_ARGUMENTS)
            .map(|index| ValueId(u32::try_from(index).expect("source value")))
            .collect::<Vec<_>>();
        let target_parameters = (ARM_ARGUMENTS + 1..=ARM_ARGUMENTS * 2)
            .map(|index| ValueId(u32::try_from(index).expect("target parameter")))
            .collect::<Vec<_>>();
        let mut instructions = vec![Instruction::ConstructVariant {
            origin: node(42),
            result: ValueId(0),
            sum: CoreTypeId(3),
            variant: 0,
            payload: None,
        }];
        instructions.extend(source_values.iter().map(|result| Instruction::ConstUnit {
            origin: node(43),
            result: *result,
        }));
        let program = CoreProgram {
            types,
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(39),
                parameters: vec![],
                result: UNIT_TYPE,
                value_types: std::iter::once(CoreTypeId(3))
                    .chain(std::iter::repeat_n(UNIT_TYPE, ARM_ARGUMENTS * 2))
                    .collect(),
                frame_cells: 1,
                entry: BlockId(0),
                blocks: vec![
                    CoreBlock {
                        origin: node(42),
                        parameters: vec![],
                        instructions,
                        terminator: Terminator::SwitchVariant {
                            origin: node(44),
                            scrutinee: ValueId(0),
                            arms: vec![SwitchArm {
                                variant: 0,
                                target: BlockId(1),
                                arguments: source_values
                                    .iter()
                                    .copied()
                                    .map(SwitchArgument::Value)
                                    .collect(),
                            }],
                        },
                    },
                    CoreBlock {
                        origin: node(45),
                        parameters: target_parameters.clone(),
                        instructions: vec![],
                        terminator: Terminator::Return {
                            origin: node(46),
                            value: target_parameters[0],
                        },
                    },
                ],
            }],
        };
        let error = interpret(
            &program,
            vec![],
            policy(u64::try_from(ARM_ARGUMENTS).expect("fuel") + 3),
        )
        .expect_err("selected edge copy must require fuel before values are materialized");
        assert_eq!(error.code, ErrorCode::ExecutionFuelExhausted);
        assert_eq!(error.target, Some(node(44)));
    }

    #[test]
    fn scalar_operations_run_with_a_frame_exactly_at_the_live_cell_cap() {
        let filler_count = MAX_RUN_LIVE_CELLS - 4;
        let filler_parameters = (5..5 + filler_count)
            .map(|index| ValueId(u32::try_from(index).expect("filler parameter")))
            .collect::<Vec<_>>();
        let filler_unit = ValueId(u32::try_from(5 + filler_count).expect("unit parameter"));
        let mut value_types = vec![I64_TYPE, I64_TYPE, I64_TYPE, BOOL_TYPE, UNIT_TYPE];
        value_types.extend(std::iter::repeat_n(I64_TYPE, filler_count));
        value_types.push(UNIT_TYPE);
        let program = CoreProgram {
            types: primitives(),
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(50),
                parameters: vec![],
                result: UNIT_TYPE,
                value_types,
                frame_cells: u64::try_from(MAX_RUN_LIVE_CELLS).expect("frame cells"),
                entry: BlockId(0),
                blocks: vec![
                    CoreBlock {
                        origin: node(51),
                        parameters: vec![],
                        instructions: vec![
                            Instruction::ConstI64 {
                                origin: node(52),
                                result: ValueId(0),
                                value: 20,
                            },
                            Instruction::ConstI64 {
                                origin: node(53),
                                result: ValueId(1),
                                value: 22,
                            },
                            Instruction::AddI64 {
                                origin: node(54),
                                result: ValueId(2),
                                lhs: ValueId(0),
                                rhs: ValueId(1),
                            },
                            Instruction::LtI64 {
                                origin: node(55),
                                result: ValueId(3),
                                lhs: ValueId(0),
                                rhs: ValueId(2),
                            },
                            Instruction::ConstUnit {
                                origin: node(56),
                                result: ValueId(4),
                            },
                        ],
                        terminator: Terminator::Return {
                            origin: node(57),
                            value: ValueId(4),
                        },
                    },
                    CoreBlock {
                        origin: node(58),
                        parameters: filler_parameters
                            .iter()
                            .copied()
                            .chain(std::iter::once(filler_unit))
                            .collect(),
                        instructions: vec![],
                        terminator: Terminator::Return {
                            origin: node(59),
                            value: filler_unit,
                        },
                    },
                ],
            }],
        };
        assert_eq!(
            interpret(&program, vec![], policy(7))
                .expect("scalar direct writes at exact live-cell cap")
                .cells,
            Vec::<u64>::new()
        );
    }

    #[test]
    fn returned_frames_release_live_cells_and_recursive_callee_exhausts_before_allocation() {
        const CALLEE_VALUES: usize = 1_024;
        const CALLS: usize = 70;
        let callee_instructions = (0..CALLEE_VALUES)
            .map(|value| Instruction::ConstI64 {
                origin: node(20),
                result: ValueId(u32::try_from(value).expect("value")),
                value: i64::try_from(value).expect("literal"),
            })
            .collect::<Vec<_>>();
        let callee = CoreFunction {
            origin: node(19),
            parameters: vec![],
            result: I64_TYPE,
            value_types: vec![I64_TYPE; CALLEE_VALUES],
            frame_cells: CALLEE_VALUES as u64,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(20),
                parameters: vec![],
                instructions: callee_instructions,
                terminator: Terminator::Return {
                    origin: node(21),
                    value: ValueId(u32::try_from(CALLEE_VALUES - 1).expect("return")),
                },
            }],
        };
        let caller_instructions = (0..CALLS)
            .map(|value| Instruction::Call {
                origin: node(4),
                result: ValueId(u32::try_from(value).expect("call value")),
                function: FunctionId(1),
                arguments: vec![],
            })
            .collect::<Vec<_>>();
        let caller = CoreFunction {
            origin: node(1),
            parameters: vec![],
            result: I64_TYPE,
            value_types: vec![I64_TYPE; CALLS],
            frame_cells: CALLS as u64,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(2),
                parameters: vec![],
                instructions: caller_instructions,
                terminator: Terminator::Return {
                    origin: node(5),
                    value: ValueId(u32::try_from(CALLS - 1).expect("return")),
                },
            }],
        };
        let program = CoreProgram {
            types: primitives(),
            functions: vec![caller, callee],
            entry: FunctionId(0),
        };
        assert_eq!(
            interpret(
                &program,
                vec![],
                RunPolicy {
                    fuel: 1_000_000,
                    maximum_frames: 1_000
                }
            )
            .expect("released callee cells")
            .cells,
            vec![(CALLEE_VALUES - 1) as u64]
        );

        let mut recursive_instructions = (0..CALLEE_VALUES)
            .map(|value| Instruction::ConstI64 {
                origin: node(30),
                result: ValueId(u32::try_from(value).expect("value")),
                value: 0,
            })
            .collect::<Vec<_>>();
        recursive_instructions.push(Instruction::Call {
            origin: node(31),
            result: ValueId(u32::try_from(CALLEE_VALUES).expect("call result")),
            function: FunctionId(0),
            arguments: vec![],
        });
        let recursive = CoreProgram {
            types: primitives(),
            functions: vec![CoreFunction {
                origin: node(29),
                parameters: vec![],
                result: I64_TYPE,
                value_types: vec![I64_TYPE; CALLEE_VALUES + 1],
                frame_cells: (CALLEE_VALUES + 1) as u64,
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(30),
                    parameters: vec![],
                    instructions: recursive_instructions,
                    terminator: Terminator::Return {
                        origin: node(32),
                        value: ValueId(u32::try_from(CALLEE_VALUES).expect("return")),
                    },
                }],
            }],
            entry: FunctionId(0),
        };
        let exhausted = interpret(
            &recursive,
            vec![],
            RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 1_000,
            },
        )
        .expect_err("recursive live cells");
        assert_eq!(exhausted.code, ErrorCode::ExecutionFrameExhausted);
        assert_eq!(exhausted.target, Some(node(31)));
    }

    #[test]
    fn entry_live_cell_exhaustion_precedes_allocation() {
        let mut program = scalar_program();
        program.functions[0].value_types = vec![I64_TYPE; MAX_RUN_LIVE_CELLS + 1];
        program.functions[0].frame_cells = u64::try_from(MAX_RUN_LIVE_CELLS + 1).expect("cells");
        program.functions[0].parameters = (0..MAX_RUN_LIVE_CELLS + 1)
            .map(|value| ValueId(u32::try_from(value).expect("value")))
            .collect();
        let parameters = program.functions[0].parameters.clone();
        program.functions[0].blocks[0].parameters = parameters;
        program.functions[0].blocks[0].instructions.clear();
        program.functions[0].blocks[0].terminator = Terminator::Return {
            origin: node(4),
            value: ValueId(0),
        };
        let args = vec![
            FlatValue {
                ty: I64_TYPE,
                cells: vec![0]
            };
            MAX_RUN_LIVE_CELLS + 1
        ];
        let error = interpret(
            &program,
            args,
            RunPolicy {
                fuel: MAX_RUN_FUEL,
                maximum_frames: MAX_RUN_FRAMES,
            },
        )
        .expect_err("live cells");
        assert_eq!(error.code, ErrorCode::ExecutionFrameExhausted);
        assert!(error.message.contains("live-cell"));
    }
}
