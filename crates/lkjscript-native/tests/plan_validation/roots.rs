use super::*;

#[test]
fn derives_non_empty_exact_reference_maps_without_dead_slots(
) -> Result<(), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(20),
        Signature::new(vec![buf, buf], buf)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let live = builder.parameter(0)?;
    let _dead = builder.parameter(1)?;
    let local = builder.create_local(buf)?;
    let _write = builder.write_local(entry, local, live)?;
    let _collected = builder.runtime_call(entry, RuntimeCallSlot::CollectReference, vec![live])?;
    let returned = builder.read_local(entry, local)?;
    builder.return_value(entry, returned)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    image.validate_integrity()?;
    assert_eq!(image.safepoints().len(), 1);
    let roots = image.safepoints()[0].stack_map().roots();
    assert_eq!(roots.len(), 2);
    assert!(roots
        .iter()
        .all(|root| root.reference_type() == ReferenceType::Buf));
    assert!(roots
        .iter()
        .any(|root| root.kind() == FrameHomeKind::Value(0)));
    assert!(roots
        .iter()
        .any(|root| root.kind() == FrameHomeKind::Local(0)));
    assert!(!roots
        .iter()
        .any(|root| root.kind() == FrameHomeKind::Value(1)));
    assert!(roots.windows(2).all(|pair| pair[0] < pair[1]));
    Ok(())
}
