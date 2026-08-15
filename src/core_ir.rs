use crate::error::{ErrorCode, LkError, Result};
use crate::ids::NodeId;
use crate::type_layout::{FieldLayout, LayoutShape, ValueLayout, VariantLayout};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CoreTypeId(pub u32);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FunctionId(pub u32);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct BlockId(pub u32);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ValueId(pub u32);

pub(crate) const UNIT_TYPE: CoreTypeId = CoreTypeId(0);
pub(crate) const BOOL_TYPE: CoreTypeId = CoreTypeId(1);
pub(crate) const I64_TYPE: CoreTypeId = CoreTypeId(2);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreProgram {
    pub types: Vec<CoreType>,
    pub functions: Vec<CoreFunction>,
    pub entry: FunctionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreType {
    pub origin: Option<NodeId>,
    pub kind: CoreTypeKind,
    pub layout: ValueLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CoreTypeKind {
    Unit,
    Bool,
    I64,
    Product { fields: Vec<CoreField> },
    Sum { variants: Vec<CoreVariant> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreField {
    pub origin: NodeId,
    pub ty: CoreTypeId,
    pub cell_offset: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreVariant {
    pub origin: NodeId,
    pub payload: Option<CoreTypeId>,
    pub discriminant: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreFunction {
    pub origin: NodeId,
    pub parameters: Vec<ValueId>,
    pub result: CoreTypeId,
    pub value_types: Vec<CoreTypeId>,
    pub frame_cells: u64,
    pub blocks: Vec<CoreBlock>,
    pub entry: BlockId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreBlock {
    pub origin: NodeId,
    pub parameters: Vec<ValueId>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Instruction {
    ConstUnit {
        origin: NodeId,
        result: ValueId,
    },
    ConstBool {
        origin: NodeId,
        result: ValueId,
        value: bool,
    },
    ConstI64 {
        origin: NodeId,
        result: ValueId,
        value: i64,
    },
    AddI64 {
        origin: NodeId,
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    LtI64 {
        origin: NodeId,
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    Call {
        origin: NodeId,
        result: ValueId,
        function: FunctionId,
        arguments: Vec<ValueId>,
    },
    ConstructProduct {
        origin: NodeId,
        result: ValueId,
        ty: CoreTypeId,
        fields: Vec<ValueId>,
    },
    ProjectField {
        origin: NodeId,
        result: ValueId,
        value: ValueId,
        field: u32,
    },
    ConstructVariant {
        origin: NodeId,
        result: ValueId,
        sum: CoreTypeId,
        variant: u32,
        payload: Option<ValueId>,
    },
}

impl Instruction {
    pub const fn origin(&self) -> NodeId {
        match self {
            Self::ConstUnit { origin, .. }
            | Self::ConstBool { origin, .. }
            | Self::ConstI64 { origin, .. }
            | Self::AddI64 { origin, .. }
            | Self::LtI64 { origin, .. }
            | Self::Call { origin, .. }
            | Self::ConstructProduct { origin, .. }
            | Self::ProjectField { origin, .. }
            | Self::ConstructVariant { origin, .. } => *origin,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SwitchArgument {
    Value(ValueId),
    Payload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SwitchArm {
    pub variant: u32,
    pub target: BlockId,
    pub arguments: Vec<SwitchArgument>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Terminator {
    Return {
        origin: NodeId,
        value: ValueId,
    },
    Branch {
        origin: NodeId,
        target: BlockId,
        arguments: Vec<ValueId>,
    },
    CondBranch {
        origin: NodeId,
        condition: ValueId,
        then_target: BlockId,
        then_arguments: Vec<ValueId>,
        else_target: BlockId,
        else_arguments: Vec<ValueId>,
    },
    SwitchVariant {
        origin: NodeId,
        scrutinee: ValueId,
        arms: Vec<SwitchArm>,
    },
}

pub(crate) fn type_at(program: &CoreProgram, id: CoreTypeId) -> Result<&CoreType> {
    program
        .types
        .get(type_index(id)?)
        .ok_or_else(|| invalid("Core type ID is out of bounds"))
}

pub(crate) fn value_type(function: &CoreFunction, value: ValueId) -> Result<CoreTypeId> {
    function
        .value_types
        .get(value_index(value)?)
        .copied()
        .ok_or_else(|| invalid("value type index is out of bounds"))
}

pub(crate) fn type_cells(program: &CoreProgram, ty: CoreTypeId) -> Result<usize> {
    usize::try_from(type_at(program, ty)?.layout.cells)
        .map_err(|_| invalid("Core type cell count overflows host indexes"))
}

pub(crate) fn verify(program: &CoreProgram) -> Result<()> {
    verify_types(program)?;
    let entry = function_index(program.entry)?;
    if entry >= program.functions.len() {
        return Err(invalid("program entry function is out of bounds"));
    }
    let mut previous_origin = None;
    for function in &program.functions {
        if previous_origin.is_some_and(|previous| previous >= function.origin) {
            return Err(invalid(
                "Core functions are not in unique ascending persistent NodeId order",
            ));
        }
        previous_origin = Some(function.origin);
    }
    for function in &program.functions {
        verify_function(program, function)?;
    }
    Ok(())
}

fn verify_types(program: &CoreProgram) -> Result<()> {
    if program.types.len() < 3 {
        return Err(invalid("Core type table omits fixed primitive types"));
    }
    let primitive = [
        (UNIT_TYPE, CoreTypeKind::Unit, 0_u64, 0_u64, 1_u64),
        (BOOL_TYPE, CoreTypeKind::Bool, 1, 1, 1),
        (I64_TYPE, CoreTypeKind::I64, 1, 8, 8),
    ];
    for (id, kind, cells, size, align) in primitive {
        let ty = type_at(program, id)?;
        if ty.origin.is_some()
            || ty.kind != kind
            || ty.layout.cells != cells
            || ty.layout.size != size
            || ty.layout.align != align
            || ty.layout.shape != LayoutShape::Primitive
        {
            return Err(invalid("fixed primitive Core type contract is malformed"));
        }
    }
    let mut previous = None;
    let mut origins = BTreeSet::new();
    for ty in &program.types[3..] {
        let origin = ty
            .origin
            .ok_or_else(|| invalid("nominal Core type omits semantic origin"))?;
        if previous.is_some_and(|value| value >= origin) || !origins.insert(origin) {
            return Err(invalid(
                "nominal Core types are not in unique persistent NodeId order",
            ));
        }
        previous = Some(origin);
        if !matches!(
            ty.kind,
            CoreTypeKind::Product { .. } | CoreTypeKind::Sum { .. }
        ) {
            return Err(invalid("nominal Core type has a primitive kind"));
        }
    }

    let mut pending = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (index, ty) in program.types.iter().enumerate().skip(3) {
        let mut dependencies = BTreeSet::new();
        match &ty.kind {
            CoreTypeKind::Product { fields } => {
                let mut field_origins = BTreeSet::new();
                for field in fields {
                    if !field_origins.insert(field.origin) {
                        return Err(invalid("Core product repeats a field origin"));
                    }
                    let dependency = type_index(field.ty)?;
                    if dependency >= program.types.len() {
                        return Err(invalid("Core product field type is out of bounds"));
                    }
                    if dependency >= 3 {
                        dependencies.insert(dependency);
                    }
                }
            }
            CoreTypeKind::Sum { variants } => {
                if variants.is_empty() {
                    return Err(invalid("Core sum type has no variants"));
                }
                let mut variant_origins = BTreeSet::new();
                for (ordinal, variant) in variants.iter().enumerate() {
                    if !variant_origins.insert(variant.origin)
                        || variant.discriminant
                            != u64::try_from(ordinal)
                                .map_err(|_| invalid("variant ordinal overflows u64"))?
                    {
                        return Err(invalid(
                            "Core sum variant identity or discriminant is malformed",
                        ));
                    }
                    if let Some(payload) = variant.payload {
                        let dependency = type_index(payload)?;
                        if dependency >= program.types.len() {
                            return Err(invalid("Core sum payload type is out of bounds"));
                        }
                        if dependency >= 3 {
                            dependencies.insert(dependency);
                        }
                    }
                }
            }
            _ => return Err(invalid("non-fixed primitive appears in Core type table")),
        }
        pending.insert(index, dependencies);
    }
    let mut derived = program.types[..3]
        .iter()
        .map(|ty| ty.layout.clone())
        .enumerate()
        .collect::<BTreeMap<_, _>>();
    while !pending.is_empty() {
        let ready = pending.iter().find_map(|(index, dependencies)| {
            dependencies
                .iter()
                .all(|dependency| derived.contains_key(dependency))
                .then_some(*index)
        });
        let Some(index) = ready else {
            return Err(invalid("Core nominal type table contains a by-value cycle"));
        };
        let expected = derive_layout(program, index, &derived)?;
        if expected != program.types[index].layout {
            return Err(invalid(
                "Core nominal type layout disagrees with its exact descriptor",
            ));
        }
        derived.insert(index, expected);
        pending.remove(&index);
    }
    Ok(())
}

fn derive_layout(
    program: &CoreProgram,
    index: usize,
    layouts: &BTreeMap<usize, ValueLayout>,
) -> Result<ValueLayout> {
    match &program.types[index].kind {
        CoreTypeKind::Product { fields } => {
            let mut byte_offset = 0_u64;
            let mut cell_offset = 0_u64;
            let mut align = 1_u64;
            let mut result_fields = Vec::with_capacity(fields.len());
            for field in fields {
                let layout = layouts
                    .get(&type_index(field.ty)?)
                    .ok_or_else(|| invalid("Core product dependency layout is absent"))?;
                if field.cell_offset != cell_offset {
                    return Err(invalid("Core product field cell offset is malformed"));
                }
                align = align.max(layout.align);
                byte_offset = align_up(byte_offset, layout.align)
                    .ok_or_else(|| invalid("Core product byte layout overflowed"))?;
                result_fields.push(FieldLayout {
                    field: field.origin,
                    offset: byte_offset,
                    cells: layout.cells,
                });
                byte_offset = byte_offset
                    .checked_add(layout.size)
                    .ok_or_else(|| invalid("Core product byte layout overflowed"))?;
                cell_offset = cell_offset
                    .checked_add(layout.cells)
                    .ok_or_else(|| invalid("Core product cell layout overflowed"))?;
            }
            Ok(ValueLayout {
                size: align_up(byte_offset, align)
                    .ok_or_else(|| invalid("Core product size overflowed"))?,
                align,
                cells: cell_offset,
                shape: LayoutShape::Product {
                    fields: result_fields,
                },
            })
        }
        CoreTypeKind::Sum { variants } => {
            let width = discriminant_width(variants.len());
            let mut payload_size = 0_u64;
            let mut payload_align = 1_u64;
            let mut payload_cells = 0_u64;
            let mut result_variants = Vec::with_capacity(variants.len());
            for variant in variants {
                let layout = match variant.payload {
                    Some(payload) => layouts
                        .get(&type_index(payload)?)
                        .ok_or_else(|| invalid("Core sum dependency layout is absent"))?
                        .clone(),
                    None => ValueLayout {
                        size: 0,
                        align: 1,
                        cells: 0,
                        shape: LayoutShape::Primitive,
                    },
                };
                payload_size = payload_size.max(layout.size);
                payload_align = payload_align.max(layout.align);
                payload_cells = payload_cells.max(layout.cells);
                result_variants.push(VariantLayout {
                    variant: variant.origin,
                    discriminant: variant.discriminant,
                    payload_size: layout.size,
                    payload_align: layout.align,
                    payload_cells: layout.cells,
                });
            }
            let tag_size = u64::from(width);
            let payload_offset = align_up(tag_size, payload_align)
                .ok_or_else(|| invalid("Core sum payload offset overflowed"))?;
            let align = tag_size.max(payload_align);
            let size = align_up(
                payload_offset
                    .checked_add(payload_size)
                    .ok_or_else(|| invalid("Core sum size overflowed"))?,
                align,
            )
            .ok_or_else(|| invalid("Core sum size overflowed"))?;
            Ok(ValueLayout {
                size,
                align,
                cells: 1_u64
                    .checked_add(payload_cells)
                    .ok_or_else(|| invalid("Core sum cells overflowed"))?,
                shape: LayoutShape::Sum {
                    discriminant_bytes: width,
                    payload_offset,
                    variants: result_variants,
                },
            })
        }
        _ => Err(invalid(
            "layout derivation requested for primitive Core type",
        )),
    }
}

fn verify_function(program: &CoreProgram, function: &CoreFunction) -> Result<()> {
    type_at(program, function.result)?;
    let entry = block_index(function.entry)?;
    let entry_block = function
        .blocks
        .get(entry)
        .ok_or_else(|| invalid("function entry block is out of bounds"))?;
    if entry_block.parameters != function.parameters {
        return Err(invalid(
            "entry block parameters must exactly equal function parameters",
        ));
    }
    let mut expected_cells = 0_u64;
    for ty in &function.value_types {
        expected_cells = expected_cells
            .checked_add(type_at(program, *ty)?.layout.cells)
            .ok_or_else(|| invalid("function frame cell footprint overflows"))?;
    }
    if function.frame_cells != expected_cells {
        return Err(invalid("function frame cell footprint is malformed"));
    }
    let mut defined = vec![false; function.value_types.len()];
    for parameter in &function.parameters {
        value_type(function, *parameter)?;
        define(&mut defined, function, *parameter, None)?;
    }
    for (block_number, block) in function.blocks.iter().enumerate() {
        let mut local = vec![false; function.value_types.len()];
        for parameter in &block.parameters {
            let index = value_index(*parameter)?;
            value_type(function, *parameter)?;
            if local[index] {
                return Err(invalid("block parameter is repeated"));
            }
            if block_number != entry {
                define(&mut defined, function, *parameter, None)?;
            } else if !function.parameters.contains(parameter) {
                return Err(invalid("entry block contains a non-function parameter"));
            }
            local[index] = true;
        }
        for instruction in &block.instructions {
            let expected = verify_instruction(program, function, instruction, &local)?;
            let result = instruction_result(instruction);
            define(&mut defined, function, result, Some(expected))?;
            local[value_index(result)?] = true;
        }
        verify_terminator(program, function, &block.terminator, &local)?;
    }
    if defined.iter().any(|value| !*value) {
        return Err(invalid("Core IR declares a value that is never defined"));
    }
    Ok(())
}

fn instruction_result(instruction: &Instruction) -> ValueId {
    match instruction {
        Instruction::ConstUnit { result, .. }
        | Instruction::ConstBool { result, .. }
        | Instruction::ConstI64 { result, .. }
        | Instruction::AddI64 { result, .. }
        | Instruction::LtI64 { result, .. }
        | Instruction::Call { result, .. }
        | Instruction::ConstructProduct { result, .. }
        | Instruction::ProjectField { result, .. }
        | Instruction::ConstructVariant { result, .. } => *result,
    }
}

fn verify_instruction(
    program: &CoreProgram,
    function: &CoreFunction,
    instruction: &Instruction,
    local: &[bool],
) -> Result<CoreTypeId> {
    match instruction {
        Instruction::ConstUnit { .. } => Ok(UNIT_TYPE),
        Instruction::ConstBool { .. } => Ok(BOOL_TYPE),
        Instruction::ConstI64 { .. } => Ok(I64_TYPE),
        Instruction::AddI64 { lhs, rhs, .. } => {
            require_local(function, local, *lhs, I64_TYPE)?;
            require_local(function, local, *rhs, I64_TYPE)?;
            Ok(I64_TYPE)
        }
        Instruction::LtI64 { lhs, rhs, .. } => {
            require_local(function, local, *lhs, I64_TYPE)?;
            require_local(function, local, *rhs, I64_TYPE)?;
            Ok(BOOL_TYPE)
        }
        Instruction::Call {
            function: target,
            arguments,
            ..
        } => {
            let callee = program
                .functions
                .get(function_index(*target)?)
                .ok_or_else(|| invalid("call target function is out of bounds"))?;
            if arguments.len() != callee.parameters.len() {
                return Err(invalid(
                    "call argument count disagrees with callee parameters",
                ));
            }
            for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                require_local(function, local, *argument, value_type(callee, *parameter)?)?;
            }
            Ok(callee.result)
        }
        Instruction::ConstructProduct { ty, fields, .. } => {
            let CoreTypeKind::Product { fields: expected } = &type_at(program, *ty)?.kind else {
                return Err(invalid("product construction names a non-product type"));
            };
            if fields.len() != expected.len() {
                return Err(invalid("product construction field count is malformed"));
            }
            for (value, field) in fields.iter().zip(expected) {
                require_local(function, local, *value, field.ty)?;
            }
            Ok(*ty)
        }
        Instruction::ProjectField { value, field, .. } => {
            let owner = value_type(function, *value)?;
            require_local(function, local, *value, owner)?;
            let CoreTypeKind::Product { fields } = &type_at(program, owner)?.kind else {
                return Err(invalid("field projection operand is not a product"));
            };
            let field = fields
                .get(
                    usize::try_from(*field)
                        .map_err(|_| invalid("field index overflows host indexes"))?,
                )
                .ok_or_else(|| invalid("field projection index is out of bounds"))?;
            Ok(field.ty)
        }
        Instruction::ConstructVariant {
            sum,
            variant,
            payload,
            ..
        } => {
            let CoreTypeKind::Sum { variants } = &type_at(program, *sum)?.kind else {
                return Err(invalid("variant construction names a non-sum type"));
            };
            let variant = variants
                .get(
                    usize::try_from(*variant)
                        .map_err(|_| invalid("variant index overflows host indexes"))?,
                )
                .ok_or_else(|| invalid("variant construction index is out of bounds"))?;
            match (variant.payload, payload) {
                (None, None) => {}
                (Some(expected), Some(value)) => require_local(function, local, *value, expected)?,
                _ => {
                    return Err(invalid(
                        "variant construction payload contract is malformed",
                    ));
                }
            }
            Ok(*sum)
        }
    }
}

fn verify_terminator(
    program: &CoreProgram,
    function: &CoreFunction,
    terminator: &Terminator,
    local: &[bool],
) -> Result<()> {
    match terminator {
        Terminator::Return { value, .. } => require_local(function, local, *value, function.result),
        Terminator::Branch {
            target, arguments, ..
        } => verify_edge(
            function,
            local,
            *target,
            &arguments
                .iter()
                .copied()
                .map(SwitchArgument::Value)
                .collect::<Vec<_>>(),
            None,
        ),
        Terminator::CondBranch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            require_local(function, local, *condition, BOOL_TYPE)?;
            verify_edge(
                function,
                local,
                *then_target,
                &then_arguments
                    .iter()
                    .copied()
                    .map(SwitchArgument::Value)
                    .collect::<Vec<_>>(),
                None,
            )?;
            verify_edge(
                function,
                local,
                *else_target,
                &else_arguments
                    .iter()
                    .copied()
                    .map(SwitchArgument::Value)
                    .collect::<Vec<_>>(),
                None,
            )
        }
        Terminator::SwitchVariant {
            scrutinee, arms, ..
        } => {
            let sum = value_type(function, *scrutinee)?;
            require_local(function, local, *scrutinee, sum)?;
            let CoreTypeKind::Sum { variants } = &type_at(program, sum)?.kind else {
                return Err(invalid("switch scrutinee is not a sum"));
            };
            if arms.len() != variants.len() {
                return Err(invalid("switch is not exhaustive"));
            }
            for (ordinal, (arm, variant)) in arms.iter().zip(variants).enumerate() {
                if usize::try_from(arm.variant).ok() != Some(ordinal) {
                    return Err(invalid(
                        "switch variants are missing, duplicated, foreign, or unordered",
                    ));
                }
                verify_edge(function, local, arm.target, &arm.arguments, variant.payload)?;
            }
            Ok(())
        }
    }
}

fn verify_edge(
    function: &CoreFunction,
    local: &[bool],
    target: BlockId,
    arguments: &[SwitchArgument],
    payload: Option<CoreTypeId>,
) -> Result<()> {
    let block = function
        .blocks
        .get(block_index(target)?)
        .ok_or_else(|| invalid("branch target block is out of bounds"))?;
    if arguments.len() != block.parameters.len() {
        return Err(invalid(
            "branch argument count disagrees with target block parameters",
        ));
    }
    let mut payload_count = 0_usize;
    for (argument, parameter) in arguments.iter().zip(&block.parameters) {
        let expected = value_type(function, *parameter)?;
        match argument {
            SwitchArgument::Value(value) => require_local(function, local, *value, expected)?,
            SwitchArgument::Payload => {
                payload_count += 1;
                if payload != Some(expected) {
                    return Err(invalid("switch payload edge argument type is malformed"));
                }
            }
        }
    }
    if payload_count != usize::from(payload.is_some()) {
        return Err(invalid("switch payload edge argument count is malformed"));
    }
    Ok(())
}

fn define(
    defined: &mut [bool],
    function: &CoreFunction,
    value: ValueId,
    expected: Option<CoreTypeId>,
) -> Result<()> {
    let index = value_index(value)?;
    let actual = function
        .value_types
        .get(index)
        .copied()
        .ok_or_else(|| invalid("defined value is out of bounds"))?;
    if expected.is_some_and(|expected| expected != actual) {
        return Err(invalid(
            "instruction result type disagrees with its contract",
        ));
    }
    if defined[index] {
        return Err(invalid("Core IR value is defined more than once"));
    }
    defined[index] = true;
    Ok(())
}
fn require_local(
    function: &CoreFunction,
    local: &[bool],
    value: ValueId,
    expected: CoreTypeId,
) -> Result<()> {
    let index = value_index(value)?;
    let actual = function
        .value_types
        .get(index)
        .copied()
        .ok_or_else(|| invalid("operand value is out of bounds"))?;
    if !local.get(index).copied().unwrap_or(false) {
        return Err(invalid("Core IR operand is not available in this block"));
    }
    if actual != expected {
        return Err(invalid("Core IR operand type disagrees with its contract"));
    }
    Ok(())
}
fn type_index(id: CoreTypeId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("type index overflows host indexes"))
}
fn function_index(id: FunctionId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("function index overflows host indexes"))
}
fn block_index(id: BlockId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("block index overflows host indexes"))
}
fn value_index(id: ValueId) -> Result<usize> {
    usize::try_from(id.0).map_err(|_| invalid("value index overflows host indexes"))
}
fn align_up(value: u64, align: u64) -> Option<u64> {
    let mask = align.checked_sub(1)?;
    value.checked_add(mask).map(|value| value & !mask)
}
fn discriminant_width(count: usize) -> u8 {
    let maximum = count.saturating_sub(1);
    if maximum <= u8::MAX as usize {
        1
    } else if maximum <= u16::MAX as usize {
        2
    } else if u32::try_from(maximum).is_ok() {
        4
    } else {
        8
    }
}
fn invalid(message: &str) -> LkError {
    LkError::new(ErrorCode::CoreIrInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WorkspaceId;

    fn node(serial: u64) -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([3; 16]), serial).expect("node")
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
    fn aggregate_program() -> CoreProgram {
        let field = CoreField {
            origin: node(11),
            ty: I64_TYPE,
            cell_offset: 0,
        };
        let product = CoreType {
            origin: Some(node(10)),
            kind: CoreTypeKind::Product {
                fields: vec![field],
            },
            layout: ValueLayout {
                size: 8,
                align: 8,
                cells: 1,
                shape: LayoutShape::Product {
                    fields: vec![FieldLayout {
                        field: node(11),
                        offset: 0,
                        cells: 1,
                    }],
                },
            },
        };
        let variants = vec![
            CoreVariant {
                origin: node(21),
                payload: None,
                discriminant: 0,
            },
            CoreVariant {
                origin: node(22),
                payload: Some(CoreTypeId(3)),
                discriminant: 1,
            },
        ];
        let sum = CoreType {
            origin: Some(node(20)),
            kind: CoreTypeKind::Sum {
                variants: variants.clone(),
            },
            layout: ValueLayout {
                size: 16,
                align: 8,
                cells: 2,
                shape: LayoutShape::Sum {
                    discriminant_bytes: 1,
                    payload_offset: 8,
                    variants: vec![
                        VariantLayout {
                            variant: node(21),
                            discriminant: 0,
                            payload_size: 0,
                            payload_align: 1,
                            payload_cells: 0,
                        },
                        VariantLayout {
                            variant: node(22),
                            discriminant: 1,
                            payload_size: 8,
                            payload_align: 8,
                            payload_cells: 1,
                        },
                    ],
                },
            },
        };
        let mut types = primitives();
        types.extend([product, sum]);
        CoreProgram {
            types,
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(30),
                parameters: vec![],
                result: UNIT_TYPE,
                value_types: vec![
                    I64_TYPE,
                    CoreTypeId(3),
                    CoreTypeId(4),
                    UNIT_TYPE,
                    CoreTypeId(3),
                    UNIT_TYPE,
                ],
                frame_cells: 5,
                entry: BlockId(0),
                blocks: vec![
                    CoreBlock {
                        origin: node(31),
                        parameters: vec![],
                        instructions: vec![
                            Instruction::ConstI64 {
                                origin: node(32),
                                result: ValueId(0),
                                value: 7,
                            },
                            Instruction::ConstructProduct {
                                origin: node(33),
                                result: ValueId(1),
                                ty: CoreTypeId(3),
                                fields: vec![ValueId(0)],
                            },
                            Instruction::ConstructVariant {
                                origin: node(34),
                                result: ValueId(2),
                                sum: CoreTypeId(4),
                                variant: 1,
                                payload: Some(ValueId(1)),
                            },
                        ],
                        terminator: Terminator::SwitchVariant {
                            origin: node(35),
                            scrutinee: ValueId(2),
                            arms: vec![
                                SwitchArm {
                                    variant: 0,
                                    target: BlockId(1),
                                    arguments: vec![],
                                },
                                SwitchArm {
                                    variant: 1,
                                    target: BlockId(2),
                                    arguments: vec![SwitchArgument::Payload],
                                },
                            ],
                        },
                    },
                    CoreBlock {
                        origin: node(36),
                        parameters: vec![],
                        instructions: vec![Instruction::ConstUnit {
                            origin: node(37),
                            result: ValueId(3),
                        }],
                        terminator: Terminator::Return {
                            origin: node(38),
                            value: ValueId(3),
                        },
                    },
                    CoreBlock {
                        origin: node(39),
                        parameters: vec![ValueId(4)],
                        instructions: vec![Instruction::ConstUnit {
                            origin: node(40),
                            result: ValueId(5),
                        }],
                        terminator: Terminator::Return {
                            origin: node(41),
                            value: ValueId(5),
                        },
                    },
                ],
            }],
        }
    }

    #[test]
    fn verifier_accepts_exact_aggregate_table_instructions_and_switch() {
        verify(&aggregate_program()).expect("valid aggregate Core");
    }

    #[test]
    fn verifier_rejects_malformed_tables_layouts_aggregate_indexes_and_switches() {
        let mut cases = Vec::new();
        let mut primitive = aggregate_program();
        primitive.types[1].layout.cells = 2;
        cases.push(primitive);
        let mut order = aggregate_program();
        order.types.swap(3, 4);
        cases.push(order);
        let mut layout = aggregate_program();
        layout.types[3].layout.cells = 2;
        cases.push(layout);
        let mut dependency = aggregate_program();
        let CoreTypeKind::Product { fields } = &mut dependency.types[3].kind else {
            unreachable!()
        };
        fields[0].ty = CoreTypeId(99);
        cases.push(dependency);
        let mut product = aggregate_program();
        let Instruction::ConstructProduct { fields, .. } =
            &mut product.functions[0].blocks[0].instructions[1]
        else {
            unreachable!()
        };
        fields.clear();
        cases.push(product);
        let mut malformed_variant = aggregate_program();
        let Instruction::ConstructVariant { variant, .. } =
            &mut malformed_variant.functions[0].blocks[0].instructions[2]
        else {
            unreachable!()
        };
        *variant = 9;
        cases.push(malformed_variant);
        let mut missing = aggregate_program();
        let Terminator::SwitchVariant { arms, .. } = &mut missing.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        arms.pop();
        cases.push(missing);
        let mut duplicate = aggregate_program();
        let Terminator::SwitchVariant { arms, .. } =
            &mut duplicate.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        arms[1].variant = 0;
        cases.push(duplicate);
        let mut payload = aggregate_program();
        let Terminator::SwitchVariant { arms, .. } = &mut payload.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        arms[1].arguments.clear();
        cases.push(payload);
        let mut field_offset = aggregate_program();
        let CoreTypeKind::Product { fields } = &mut field_offset.types[3].kind else {
            unreachable!()
        };
        fields[0].cell_offset = 1;
        cases.push(field_offset);
        let mut projection_non_product = aggregate_program();
        projection_non_product.functions[0].blocks[0].instructions[1] = Instruction::ProjectField {
            origin: node(33),
            result: ValueId(1),
            value: ValueId(0),
            field: 0,
        };
        cases.push(projection_non_product);
        let mut projection_index = aggregate_program();
        projection_index.functions[0].blocks[0].instructions[2] = Instruction::ProjectField {
            origin: node(34),
            result: ValueId(2),
            value: ValueId(1),
            field: 99,
        };
        cases.push(projection_index);
        let mut projection_result = aggregate_program();
        projection_result.functions[0].blocks[0].instructions[2] = Instruction::ProjectField {
            origin: node(34),
            result: ValueId(2),
            value: ValueId(1),
            field: 0,
        };
        cases.push(projection_result);
        let mut variant_payload_omitted = aggregate_program();
        let Instruction::ConstructVariant { payload, .. } =
            &mut variant_payload_omitted.functions[0].blocks[0].instructions[2]
        else {
            unreachable!()
        };
        *payload = None;
        cases.push(variant_payload_omitted);
        let mut variant_payload_excess = aggregate_program();
        let Instruction::ConstructVariant {
            variant, payload, ..
        } = &mut variant_payload_excess.functions[0].blocks[0].instructions[2]
        else {
            unreachable!()
        };
        *variant = 0;
        *payload = Some(ValueId(1));
        cases.push(variant_payload_excess);
        let mut construct_non_product = aggregate_program();
        let Instruction::ConstructProduct { ty, .. } =
            &mut construct_non_product.functions[0].blocks[0].instructions[1]
        else {
            unreachable!()
        };
        *ty = I64_TYPE;
        cases.push(construct_non_product);
        let mut switch_non_sum = aggregate_program();
        let Terminator::SwitchVariant { scrutinee, .. } =
            &mut switch_non_sum.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *scrutinee = ValueId(0);
        cases.push(switch_non_sum);
        let mut switch_target = aggregate_program();
        let Terminator::SwitchVariant { arms, .. } =
            &mut switch_target.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        arms[0].target = BlockId(99);
        cases.push(switch_target);
        let mut function_order = aggregate_program();
        let mut duplicate = function_order.functions[0].clone();
        duplicate.origin = function_order.functions[0].origin;
        function_order.functions.push(duplicate);
        cases.push(function_order);
        let mut footprint = aggregate_program();
        footprint.functions[0].frame_cells = 4;
        cases.push(footprint);
        for case in cases {
            assert_eq!(
                verify(&case).expect_err("malformed Core").code,
                ErrorCode::CoreIrInvalid
            );
        }
    }

    #[test]
    fn verifier_accepts_transitive_nominal_closure_in_origin_order_with_exact_layouts() {
        let mut program = aggregate_program();
        program.types.push(CoreType {
            origin: Some(node(25)),
            kind: CoreTypeKind::Product {
                fields: vec![CoreField {
                    origin: node(26),
                    ty: CoreTypeId(4),
                    cell_offset: 0,
                }],
            },
            layout: ValueLayout {
                size: 16,
                align: 8,
                cells: 2,
                shape: LayoutShape::Product {
                    fields: vec![FieldLayout {
                        field: node(26),
                        offset: 0,
                        cells: 2,
                    }],
                },
            },
        });
        verify(&program).expect("transitive product to sum to product closure");
        assert_eq!(
            program.types[3..]
                .iter()
                .map(|ty| ty.origin.expect("nominal origin"))
                .collect::<Vec<_>>(),
            vec![node(10), node(20), node(25)]
        );
    }

    #[test]
    fn verifier_rejects_nominal_cycles_independently() {
        let mut program = aggregate_program();
        let CoreTypeKind::Product { fields } = &mut program.types[3].kind else {
            unreachable!()
        };
        fields[0].ty = CoreTypeId(4);
        let CoreTypeKind::Sum { variants } = &mut program.types[4].kind else {
            unreachable!()
        };
        variants[1].payload = Some(CoreTypeId(3));
        assert_eq!(
            verify(&program).expect_err("cycle").code,
            ErrorCode::CoreIrInvalid
        );
    }
}
