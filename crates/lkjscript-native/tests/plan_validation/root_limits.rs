use super::*;

#[test]
fn rejects_adversarial_wide_root_certificates_before_metadata_allocation(
) -> Result<(), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let sink = plan.declare_function(
        SourceFunctionId::new(30),
        Signature::new(vec![buf, buf], ValueType::Unit)?,
    )?;
    let wide = plan.declare_function(SourceFunctionId::new(31), Signature::new(vec![buf], buf)?)?;

    let mut sink_builder = plan.function_builder(sink)?;
    let sink_entry = sink_builder.create_block()?;
    sink_builder.set_entry(sink_entry)?;
    let unit = sink_builder.unit(sink_entry)?;
    sink_builder.return_value(sink_entry, unit)?;
    plan.define_function(sink_builder.finish())?;

    let mut builder = plan.function_builder(wide)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let mut locals = Vec::new();
    for _ in 0..128 {
        let local = builder.create_local(buf)?;
        let _write = builder.write_local(entry, local, input)?;
        locals.push(local);
    }
    let collected = builder.runtime_call(entry, RuntimeCallSlot::CollectReference, vec![input])?;
    for pair in locals.chunks_exact(2) {
        let first = builder.read_local(entry, pair[0])?;
        let second = builder.read_local(entry, pair[1])?;
        let _call = builder.call(entry, sink, vec![first, second])?;
    }
    builder.return_value(entry, collected)?;
    plan.define_function(builder.finish())?;

    let limits = BackendLimits::new(2, 2, 512, 128, 1024 * 1024, 1024, 1_000_000);
    assert!(matches!(
        plan.verify(limits),
        Err(NativeError::Verification(VerificationError::LimitExceeded(
            "stack-map root metadata"
        )))
    ));
    Ok(())
}
