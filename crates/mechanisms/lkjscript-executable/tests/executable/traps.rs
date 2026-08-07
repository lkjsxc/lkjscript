use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn explicit_trap_reports_preserve_full_u64_site_identity() -> Result<(), Box<dyn std::error::Error>>
{
    let mut plan = MachinePlanBuilder::new();
    let sites = [0, u64::from(u32::MAX) + 1, u64::MAX];
    let mut functions = Vec::new();
    for (ordinal, site) in sites.into_iter().enumerate() {
        let function = plan.declare_function(
            SourceFunctionId::new(u64::try_from(ordinal)?),
            Signature::new(Vec::new(), ValueType::Unit)?,
        )?;
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        builder.trap_at(entry, TrapCode::Explicit, site)?;
        plan.define_function(builder.finish())?;
        functions.push((function, Some(site)));
    }
    let without_site = plan.declare_function(
        SourceFunctionId::new(3),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(without_site)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    builder.trap(entry, TrapCode::Explicit)?;
    plan.define_function(builder.finish())?;
    functions.push((without_site, None));

    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    let installed = ExecutableInstaller::default().install(image)?;
    for (function, site) in functions {
        let report =
            installed.invoke_with_config(function, &[], &NativeInvocationConfig::unrestricted())?;
        assert_eq!(
            report.outcome(),
            InvocationOutcome::Trapped(TrapCode::Explicit)
        );
        assert_eq!(report.trap_site(), site);
    }
    Ok(())
}
