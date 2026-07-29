pub(crate) fn supports(
    capability: lkjscript_core::CapabilityKind,
    host: &lkjscript_host::HostEnvironment,
) -> bool {
    match capability {
        lkjscript_core::CapabilityKind::Arguments => true,
        lkjscript_core::CapabilityKind::Clock => host.clock.is_some(),
        lkjscript_core::CapabilityKind::Stdio => host.stdio.is_some(),
        lkjscript_core::CapabilityKind::Entropy
        | lkjscript_core::CapabilityKind::FileSystem
        | lkjscript_core::CapabilityKind::Network
        | lkjscript_core::CapabilityKind::Sqlite
        | lkjscript_core::CapabilityKind::Terminal => false,
    }
}
