use super::*;

#[derive(Debug)]
pub(super) struct InstallerState {
    pub(super) limits: ExecutableLimits,
    pub(super) usage: Cell<ExecutableUsage>,
}

/// Bounded non-Send executable allocation session. Installed images retain an
/// owned lease on this state, so mappings cannot outlive their accounting.
#[derive(Clone, Debug)]
pub struct ExecutableInstaller {
    pub(super) state: Rc<InstallerState>,
    pub(super) not_send_or_sync: PhantomData<Rc<()>>,
}

impl ExecutableInstaller {
    #[must_use]
    pub fn new(limits: ExecutableLimits) -> Self {
        Self {
            state: Rc::new(InstallerState {
                limits,
                usage: Cell::new(ExecutableUsage::default()),
            }),
            not_send_or_sync: PhantomData,
        }
    }

    #[must_use]
    pub fn usage(&self) -> ExecutableUsage {
        self.state.usage.get()
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
        let next_usage = checked_usage(self.state.usage.get(), accounting, self.state.limits)?;
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
                RelocationTarget::Runtime(slot) => runtime_symbol(slot),
            };
            mapping.write_absolute64(item.offset() as usize, address)?;
        }
        mapping.seal_rx()?;
        self.state.usage.set(next_usage);
        Ok(InstalledImage {
            installer: Rc::clone(&self.state),
            image,
            mapping,
            usage: ExecutableUsage {
                code_bytes: accounting.code_bytes(),
                metadata_bytes: accounting.metadata_bytes(),
                work_units: accounting.work_units(),
                objects: 1,
            },
            not_send_or_sync: PhantomData,
        })
    }
}

impl Default for ExecutableInstaller {
    fn default() -> Self {
        Self::new(ExecutableLimits::default())
    }
}
