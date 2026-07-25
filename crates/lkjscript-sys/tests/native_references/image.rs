use super::*;

#[derive(Clone, Copy)]
pub(super) struct ReferenceEntries {
    pub(super) exact_local: FunctionId,
    pub(super) caller: FunctionId,
    pub(super) trap_caller: FunctionId,
    pub(super) exit_caller: FunctionId,
    pub(super) deadline_caller: FunctionId,
    pub(super) resource_caller: FunctionId,
    pub(super) host_caller: FunctionId,
}

pub(super) fn reference_image(
) -> Result<(InstallableImage, ReferenceEntries), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let exact_local = plan.declare_function(
        SourceFunctionId::new(1),
        Signature::new(vec![buf, buf], buf)?,
    )?;
    let callee =
        plan.declare_function(SourceFunctionId::new(2), Signature::new(vec![buf], buf)?)?;
    let caller =
        plan.declare_function(SourceFunctionId::new(3), Signature::new(vec![buf], buf)?)?;
    let trap = plan.declare_function(
        SourceFunctionId::new(4),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let exit = plan.declare_function(
        SourceFunctionId::new(5),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let deadline = plan.declare_function(
        SourceFunctionId::new(6),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let resource = plan.declare_function(
        SourceFunctionId::new(7),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let host = plan.declare_function(
        SourceFunctionId::new(8),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let trap_caller = plan.declare_function(
        SourceFunctionId::new(9),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let exit_caller = plan.declare_function(
        SourceFunctionId::new(10),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let deadline_caller = plan.declare_function(
        SourceFunctionId::new(11),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let resource_caller = plan.declare_function(
        SourceFunctionId::new(12),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let host_caller = plan.declare_function(
        SourceFunctionId::new(13),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;

    {
        let mut builder = plan.function_builder(exact_local)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let live = builder.parameter(0)?;
        let _dead = builder.parameter(1)?;
        let local = builder.create_local(buf)?;
        let _write = builder.write_local(entry, local, live)?;
        let _collected =
            builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![live])?;
        let returned = builder.read_local(entry, local)?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(callee)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let collected =
            builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![input])?;
        builder.return_value(entry, collected)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(caller)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let returned = builder.call(entry, callee, vec![input])?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(trap)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        builder.trap(entry, TrapCode::Explicit)?;
        plan.define_function(builder.finish())?;
    }
    {
        let mut builder = plan.function_builder(exit)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let status = builder.i64_const(entry, 23)?;
        builder.exit(entry, status)?;
        plan.define_function(builder.finish())?;
    }
    for (function, outcome) in [
        (deadline, RuntimeOutcome::DeadlineExceeded),
        (resource, RuntimeOutcome::ResourceLimitExceeded),
        (host, RuntimeOutcome::HostFailure),
    ] {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        builder.outcome(entry, outcome)?;
        plan.define_function(builder.finish())?;
    }
    for (function, callee) in [
        (trap_caller, trap),
        (exit_caller, exit),
        (deadline_caller, deadline),
        (resource_caller, resource),
        (host_caller, host),
    ] {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let returned = builder.call(entry, callee, Vec::new())?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }

    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::new(AbiVersions::current()),
    )?;
    Ok((
        image,
        ReferenceEntries {
            exact_local,
            caller,
            trap_caller,
            exit_caller,
            deadline_caller,
            resource_caller,
            host_caller,
        },
    ))
}
