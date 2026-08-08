use lkjscript_core::{CapabilityKind, Error, Result};

use super::{model::Target, LockedTargetMemory, VerifiedCompilationPackage};

#[derive(Clone)]
pub(crate) struct CapturedPackageCompilation {
    target: LockedTargetMemory,
    capabilities: Vec<CapabilityKind>,
}

/// Capture the locked target needed to validate a later in-process memory plan.
/// Snapshot compilation never returns to the file system or reconstructs this
/// boundary fact from presentation attachments.
pub(crate) fn capture(verified: &VerifiedCompilationPackage) -> Result<CapturedPackageCompilation> {
    let package = verified
        .lock()
        .packages
        .iter()
        .find(|package| package.origin == ".")
        .ok_or_else(|| Error::msg("package lock omits root package"))?;
    let relative = verified.entry_module();
    let target = package
        .targets
        .iter()
        .find(|target| target.module == relative)
        .ok_or_else(|| Error::msg("compiled package module is not an executable target"))?
        .clone();
    Ok(CapturedPackageCompilation {
        target,
        capabilities: verified.capabilities().to_vec(),
    })
}

impl CapturedPackageCompilation {
    pub(crate) fn validate_memory_plan(&self, plan: &crate::HirMemoryPlan) -> Result<()> {
        let generated = super::target_memory::target_record(
            &Target {
                name: self.target.name.clone(),
                module: self.target.module.clone(),
            },
            plan,
        )?;
        if generated != self.target {
            return Err(Error::msg(
                "compiled memory plan or witness closure differs from locked target",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_required_capabilities(&self, required: &[CapabilityKind]) -> Result<()> {
        for capability in required {
            if !self.capabilities.contains(capability) {
                return Err(Error::msg(format!(
                    "package does not grant required {} capability",
                    capability.as_str()
                )));
            }
        }
        Ok(())
    }
}
