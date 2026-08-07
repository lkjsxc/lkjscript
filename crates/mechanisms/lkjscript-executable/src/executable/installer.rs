use super::*;

#[derive(Debug)]
pub(super) struct InstallerState {
    pub(super) limits: ExecutableLimits,
    pub(super) usage: Mutex<ExecutableUsage>,
}

/// Bounded executable allocation session. Installed images retain a shared
/// accounted lease, so mappings cannot outlive their installer state.
#[derive(Clone, Debug)]
pub struct ExecutableInstaller {
    pub(super) state: Arc<InstallerState>,
}

impl ExecutableInstaller {
    #[must_use]
    pub fn new(limits: ExecutableLimits) -> Self {
        Self {
            state: Arc::new(InstallerState {
                limits,
                usage: Mutex::new(ExecutableUsage::default()),
            }),
        }
    }

    #[must_use]
    pub fn usage(&self) -> ExecutableUsage {
        match self.state.usage.lock() {
            Ok(usage) => *usage,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    pub fn install(&self, image: InstallableImage) -> Result<InstalledImage, InstallError> {
        if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            return Err(InstallError::UnsupportedPlatform);
        }
        image
            .validate_integrity()
            .map_err(InstallError::InvalidImage)?;
        let expected = ImageContracts::current();
        if image.contracts() != expected {
            return Err(InstallError::ContractMismatch {
                expected: Box::new(expected),
                actual: Box::new(image.contracts()),
            });
        }
        // Build the deterministic wide source-ID to installed-entry mapping before
        // allocating or publishing executable state. Bookkeeping allocation failure
        // therefore leaves both installer usage and W^X mappings unchanged.
        let entry_mapping = NativeEntryMapping::try_new(&image)?;
        let accounting = image.accounting();
        check_object_limit(
            accounting.code_bytes(),
            self.state.limits.max_object_code_bytes,
            ExecutableLimitKind::ObjectCodeBytes,
        )?;
        check_object_limit(
            accounting.metadata_bytes(),
            self.state.limits.max_object_metadata_bytes,
            ExecutableLimitKind::ObjectMetadataBytes,
        )?;
        check_object_limit(
            accounting.work_units(),
            self.state.limits.max_object_work_units,
            ExecutableLimitKind::ObjectWorkUnits,
        )?;
        let mut usage = match self.state.usage.lock() {
            Ok(usage) => usage,
            Err(poisoned) => poisoned.into_inner(),
        };
        let next_usage = checked_usage(*usage, accounting, self.state.limits)?;
        let mut mapping = platform::Mapping::allocate_rw(image.bytes().len())?;
        mapping.copy_from(image.bytes())?;
        for item in image.relocations() {
            let address = match item.target() {
                RelocationTarget::Function(function) => {
                    let entry = image
                        .entries()
                        .iter()
                        .find(|entry| entry.function() == function)
                        .ok_or(InstallError::RelocationAddress)?;
                    mapping.address_at(entry.offset() as usize)?
                }
                RelocationTarget::Runtime(slot) => runtime_symbol(image.execution_domain(), slot)
                    .ok_or(InstallError::RelocationAddress)?,
            };
            mapping.write_absolute64(item.offset() as usize, address)?;
        }
        mapping.seal_rx()?;
        *usage = next_usage;
        drop(usage);
        Ok(InstalledImage {
            installer: Arc::clone(&self.state),
            image,
            entry_mapping,
            mapping,
            usage: ExecutableUsage {
                code_bytes: accounting.code_bytes(),
                metadata_bytes: accounting.metadata_bytes(),
                work_units: accounting.work_units(),
                objects: 1,
            },
        })
    }
}

impl Default for ExecutableInstaller {
    fn default() -> Self {
        Self::new(ExecutableLimits::default())
    }
}
