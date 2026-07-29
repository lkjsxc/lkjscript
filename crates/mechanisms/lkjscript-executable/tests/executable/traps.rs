use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn explicit_trap_reports_preserve_full_u32_site_identity() -> Result<(), Box<dyn std::error::Error>>
{
    let mut plan = MachinePlanBuilder::new();
    let sites = [0x8000_0000_u32, u32::MAX];
    let mut functions = Vec::new();
    for (ordinal, site) in sites.into_iter().enumerate() {
        let function = plan.declare_function(
            SourceFunctionId::new(u32::try_from(ordinal)?),
            Signature::new(Vec::new(), ValueType::Unit)?,
        )?;
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        builder.trap_at(entry, TrapCode::Explicit, site)?;
        plan.define_function(builder.finish())?;
        functions.push((function, site));
    }
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    let installed = ExecutableInstaller::default().install(image)?;
    for (function, site) in functions {
        let report =
            installed.invoke_with_config(function, &[], &NativeInvocationConfig::default())?;
        assert_eq!(
            report.outcome(),
            InvocationOutcome::Trapped(TrapCode::Explicit)
        );
        assert_eq!(report.trap_site(), Some(site));
    }
    Ok(())
}
