use super::*;

pub(super) fn owner_plan(
    body: impl FnOnce(
        &mut FunctionBuilder,
        BlockId,
        ValueId,
        StructuralTypeIdentity,
    ) -> Result<(), PlanError>,
) -> Result<MachinePlanBuilder, Box<dyn std::error::Error>> {
    let value_type = ty(1, StructuralKind::String);
    let mut plan = MachinePlanBuilder::new();
    let bytes = plan.intern_static_bytes(b"owner")?;
    let function = plan.declare_function(
        SourceFunctionId::new(5),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let artifact = builder.static_string_const(block, bytes, value_type)?;
    let owner = sc(
        &mut builder,
        block,
        StructuralOperation::PublishStatic {
            value_type,
            payload: StructuralPayloadKind::String,
        },
        vec![artifact],
    )?;
    body(&mut builder, block, owner, value_type)?;
    plan.define_function(builder.finish())?;
    Ok(plan)
}

pub(super) fn destination_plan(
    aggregate: &StructuralAggregateDescriptor,
    body: impl FnOnce(&mut FunctionBuilder, BlockId, ValueId, ValueId) -> Result<(), PlanError>,
) -> Result<MachinePlanBuilder, Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(6),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let block = builder.create_block()?;
    builder.set_entry(block)?;
    let raw = builder.i64_const(block, 1)?;
    let field_owner = sc(
        &mut builder,
        block,
        StructuralOperation::PublishI64(aggregate.fields()[0]),
        vec![raw],
    )?;
    let destination = sc(
        &mut builder,
        block,
        StructuralOperation::DestinationCreate(aggregate.clone()),
        vec![],
    )?;
    body(&mut builder, block, destination, field_owner)?;
    plan.define_function(builder.finish())?;
    Ok(plan)
}

pub(super) fn verify(plan: MachinePlanBuilder) -> Result<(), VerificationError> {
    plan.verify(BackendLimits::default())
        .map(|_| ())
        .map_err(|error| match error {
            NativeError::Verification(error) => error,
            other => panic!("unexpected error: {other}"),
        })
}

pub(super) fn ty(id: u64, kind: StructuralKind) -> StructuralTypeIdentity {
    StructuralTypeIdentity::new(id * 2 + 1, id * 2 + 2, kind)
}

pub(super) fn sc(
    builder: &mut FunctionBuilder,
    block: BlockId,
    operation: StructuralOperation,
    arguments: Vec<ValueId>,
) -> Result<ValueId, PlanError> {
    builder.structural_call(block, StructuralCallDescriptor::new(operation)?, arguments)
}
