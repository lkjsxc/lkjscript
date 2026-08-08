use lkjscript_core::{Error, Result};

use super::{model::Target, LockedTargetMemory, VerifiedCompilationPackage};

#[derive(Clone)]
pub(crate) struct CapturedPackageCompilation {
    semantic_identity: [u8; 32],
    target: LockedTargetMemory,
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
        semantic_identity: digest(&package.package_sha256, "package content")?,
        target,
    })
}

impl CapturedPackageCompilation {
    pub(crate) const fn semantic_base_identity(&self) -> [u8; 32] {
        self.semantic_identity
    }

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
}

fn digest(text: &str, name: &str) -> Result<[u8; 32]> {
    lkjscript_contracts::ContractDigest::from_hex(text)
        .map(lkjscript_contracts::ContractDigest::as_bytes)
        .filter(|value| *value != [0; 32])
        .ok_or_else(|| Error::msg(format!("{name} is not a nonzero canonical digest")))
}
