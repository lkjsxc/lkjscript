use crate::core_ir::{
    self, BlockId, CoreBlock, CoreFunction, CoreProgram, Instruction, Terminator, ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::query;
use crate::schema::{Node, OperationKind, ValueRef};
use std::collections::BTreeMap;

pub(crate) fn compile(snapshot: &Snapshot, entry: NodeId) -> Result<CoreProgram> {
    let blockers = query::entry_blockers(snapshot, entry)?;
    if !blockers.is_empty() {
        let related = blockers.iter().filter_map(|blocker| blocker.target);
        return Err(LkError::new(
            ErrorCode::CompileIncomplete,
            "entry dependency closure contains completeness blockers",
        )
        .for_workspace(snapshot.workspace())
        .at_revision(snapshot.revision())
        .for_node(entry)
        .with_related(related));
    }
    let function = snapshot.node(entry)?;
    let Node::Function {
        parameters,
        result,
        body,
        ..
    } = function
    else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "compile entry must be a function").for_node(entry),
        );
    };
    if !parameters.is_empty() {
        return Err(LkError::new(
            ErrorCode::CompileIncomplete,
            "bootstrap invocation supports only zero-parameter entry functions",
        )
        .for_node(entry)
        .with_related(parameters.iter().copied()));
    }
    let body = body.ok_or_else(|| {
        LkError::new(ErrorCode::CompileIncomplete, "entry function has no body").for_node(entry)
    })?;
    let Node::Region { blocks, .. } = snapshot.node(body)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "function body must reference a region",
        )
        .for_node(body));
    };
    let block_id = *blocks.first().ok_or_else(|| {
        LkError::new(
            ErrorCode::CompileIncomplete,
            "entry function region has no block",
        )
        .for_node(body)
    })?;
    let Node::Block {
        operations,
        terminator,
        ..
    } = snapshot.node(block_id)?
    else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "region child is not a block").for_node(block_id),
        );
    };
    let mut value_types = Vec::new();
    let mut values = BTreeMap::new();
    let mut instructions = Vec::with_capacity(operations.len());
    for operation_id in operations {
        let Node::Operation { operation, .. } = snapshot.node(*operation_id)? else {
            return Err(
                LkError::new(ErrorCode::WrongKind, "block child is not an operation")
                    .for_node(*operation_id),
            );
        };
        let result_type = operation.result_type(0, None).ok_or_else(|| {
            LkError::new(
                ErrorCode::CoreIrInvalid,
                "non-terminator operation has no result contract",
            )
            .for_node(*operation_id)
        })?;
        let result_index = u32::try_from(value_types.len()).map_err(|_| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "Core IR value count exceeds dense index representation",
            )
            .for_node(*operation_id)
        })?;
        let result_id = ValueId(result_index);
        value_types.push(result_type);
        values.insert((*operation_id, 0_u8), result_id);
        let instruction = match operation {
            OperationKind::ConstI64(value) => Instruction::ConstI64 {
                origin: *operation_id,
                result: result_id,
                value: *value,
            },
            OperationKind::ConstBool(value) => Instruction::ConstBool {
                origin: *operation_id,
                result: result_id,
                value: *value,
            },
            OperationKind::AddI64 { lhs, rhs } => Instruction::AddI64 {
                origin: *operation_id,
                result: result_id,
                lhs: lower_value(&values, *lhs)?,
                rhs: lower_value(&values, *rhs)?,
            },
            OperationKind::Hole { .. } => {
                return Err(LkError::new(
                    ErrorCode::CompileIncomplete,
                    "hole reached Core IR lowering",
                )
                .for_node(*operation_id));
            }
            OperationKind::Return { .. } => {
                return Err(LkError::new(
                    ErrorCode::CoreIrInvalid,
                    "return appeared in a non-terminator slot",
                )
                .for_node(*operation_id));
            }
        };
        instructions.push(instruction);
    }
    let terminator_id = terminator.ok_or_else(|| {
        LkError::new(
            ErrorCode::CompileIncomplete,
            "entry block has no terminator",
        )
        .for_node(block_id)
    })?;
    let Node::Operation { operation, .. } = snapshot.node(terminator_id)? else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "block terminator is not an operation")
                .for_node(terminator_id),
        );
    };
    let OperationKind::Return { value } = operation else {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "bootstrap block terminator is not return",
        )
        .for_node(terminator_id));
    };
    let program = CoreProgram {
        function: CoreFunction {
            origin: entry,
            result: *result,
            value_types,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: block_id,
                parameters: Vec::new(),
                instructions,
                terminator: Terminator::Return {
                    origin: terminator_id,
                    value: lower_value(&values, *value)?,
                },
            }],
        },
    };
    core_ir::verify(&program)?;
    Ok(program)
}

fn lower_value(values: &BTreeMap<(NodeId, u8), ValueId>, value: ValueRef) -> Result<ValueId> {
    match value {
        ValueRef::OperationResult { operation, output } => {
            values.get(&(operation, output)).copied().ok_or_else(|| {
                LkError::new(
                    ErrorCode::CoreIrInvalid,
                    "semantic value has no lowered Core IR definition",
                )
                .for_node(operation)
            })
        }
        ValueRef::FunctionParameter(parameter) => Err(LkError::new(
            ErrorCode::CompileIncomplete,
            "bootstrap entry parameters are unsupported",
        )
        .for_node(parameter)),
    }
}
