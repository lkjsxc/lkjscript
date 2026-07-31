use super::*;

fn shallow_wide_root_image() -> Result<(InstallableImage, FunctionId), Box<dyn std::error::Error>> {
    let product_ref = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(0)));
    let mut plan = MachinePlanBuilder::new();
    let sink = plan.declare_function(
        SourceFunctionId::new(41),
        Signature::new(vec![product_ref, product_ref], ValueType::Unit)?,
    )?;
    let wide = plan.declare_function(
        SourceFunctionId::new(42),
        Signature::new(vec![product_ref], product_ref)?,
    )?;

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
    for _ in 0..1024 {
        let local = builder.create_local(product_ref)?;
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
    let limits = BackendLimits::new(
        2,
        2,
        4_096,
        1_024,
        4 * 1024 * 1024,
        32 * 1024 * 1024,
        2_000_000,
    );
    Ok((
        encode(plan.verify(limits)?, EncodingConfig::default())?,
        wide,
    ))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn shallow_1025_root_map_reserves_dynamically_under_aggregate_cap(
) -> Result<(), Box<dyn std::error::Error>> {
    let (image, entry) = shallow_wide_root_image()?;
    assert_eq!(
        image
            .safepoints()
            .iter()
            .map(|safepoint| safepoint.stack_map().roots().len())
            .max(),
        Some(1025)
    );
    let unlimited_install = ExecutableLimits::new(
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
    );
    let installed = ExecutableInstaller::new(unlimited_install).install(image)?;
    let mut services = RecordingServices::default();
    let report = installed.invoke_with_services(
        entry,
        &[product_ref(55)],
        &NativeInvocationConfig::default(),
        &mut services,
    )?;
    assert_eq!(
        report.outcome(),
        InvocationOutcome::Returned(product_ref(55))
    );
    assert_eq!(report.exact_root_counts(), &[1025]);
    assert_eq!(report.maximum_roots(), 1025);
    assert_eq!(report.reserved_native_stack_bytes(), 0);
    assert_eq!(services.observed.len(), 1);
    assert_eq!(services.observed[0].len(), 1025);
    Ok(())
}
