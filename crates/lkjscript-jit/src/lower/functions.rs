use super::*;

pub(super) fn lower_function(
    program: &lkjscript_ir::Program,
    function: &Function,
    native_functions: &[(FunctionId, lkjscript_native::FunctionId)],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
    explicit_traps: &mut Vec<(u32, String)>,
) -> Result<(), LoweringError> {
    let value_types = collect_value_types(function, layouts)?;
    let mut locals = Vec::with_capacity(value_types.len());
    for value_type in &value_types {
        locals.push(
            builder
                .create_local(*value_type)
                .map_err(LoweringError::backend)?,
        );
    }

    let mut blocks = Vec::with_capacity(function.blocks.len());
    for _ in &function.blocks {
        blocks.push(builder.create_block().map_err(LoweringError::backend)?);
    }
    let mut edges = Vec::with_capacity(function.blocks.len());
    for block in &function.blocks {
        let edge = match block.terminator {
            Terminator::Branch { .. } => EdgeBlocks {
                branch: Some(builder.create_block().map_err(LoweringError::backend)?),
                when_true: None,
                when_false: None,
            },
            Terminator::ConditionalBranch { .. } => EdgeBlocks {
                branch: None,
                when_true: Some(builder.create_block().map_err(LoweringError::backend)?),
                when_false: Some(builder.create_block().map_err(LoweringError::backend)?),
            },
            _ => EdgeBlocks {
                branch: None,
                when_true: None,
                when_false: None,
            },
        };
        edges.push(edge);
    }

    let entry_index = function.entry.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block ID is invalid",
        )
    })?;
    let entry = *blocks.get(entry_index).ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block is absent",
        )
    })?;
    builder.set_entry(entry).map_err(LoweringError::backend)?;

    let source_entry = function.blocks.get(entry_index).ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block is absent",
        )
    })?;
    if source_entry.parameters.len() != function.signature.parameters.len() {
        return Err(LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block parameters do not match the function signature",
        ));
    }
    for (index, parameter) in source_entry.parameters.iter().enumerate() {
        let value = builder.parameter(index).map_err(LoweringError::backend)?;
        let local = value_local(&locals, parameter.id, function.id)?;
        builder
            .write_local(entry, local, value)
            .map_err(LoweringError::backend)?;
    }
    // ABI-2 frame registration records the exact source-function entry and
    // consumes its mandatory poll before generated body effects. Separate
    // EnterFunctionV1 and PollV1 calls would duplicate transition overhead.

    for (index, block) in function.blocks.iter().enumerate() {
        let native_block = blocks[index];
        for instruction in &block.instructions {
            lower_instruction(
                program,
                function,
                instruction,
                native_block,
                &locals,
                &value_types,
                native_functions,
                layouts,
                builder,
            )?;
        }
        lower_terminator(
            function,
            block,
            TerminatorContext {
                native_block,
                edges: edges[index],
                blocks: &blocks,
                locals: &locals,
            },
            builder,
            explicit_traps,
        )?;
    }
    Ok(())
}

pub(super) fn collect_value_types(
    function: &Function,
    layouts: &LayoutInterner,
) -> Result<Vec<ValueType>, LoweringError> {
    let mut types: Vec<Option<ValueType>> = Vec::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            set_value_type(
                &mut types,
                parameter.id,
                lower_type(function.id, &parameter.ty, layouts)?,
            )?;
        }
        for instruction in &block.instructions {
            set_value_type(
                &mut types,
                instruction.id,
                lower_type(function.id, &instruction.ty, layouts)?,
            )?;
        }
    }
    types
        .into_iter()
        .map(|ty| {
            ty.ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::InvalidFunction,
                    Some(function.id),
                    "SSA value IDs are not dense",
                )
            })
        })
        .collect()
}

pub(super) fn set_value_type(
    types: &mut Vec<Option<ValueType>>,
    value: ValueId,
    ty: ValueType,
) -> Result<(), LoweringError> {
    let index = value.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            None,
            "SSA value ID cannot index native locals",
        )
    })?;
    if types.len() <= index {
        types.resize(index + 1, None);
    }
    if types[index].replace(ty).is_some() {
        return Err(LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            None,
            "duplicate SSA value during native lowering",
        ));
    }
    Ok(())
}
