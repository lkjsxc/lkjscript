use std::path::Path;
use std::sync::Arc;

use lkjscript_core::ValidatedChunk;

use crate::state::{AppRecord, IsolatedProcessSpec};
use crate::{
    ApplicationId, ApplicationManifest, ExecutionCellClass, Lifecycle, PackageContentId,
    RuntimeError, RuntimeSystem,
};

impl RuntimeSystem {
    pub fn install(
        &self,
        manifest: ApplicationManifest,
        package: PackageContentId,
        chunk: Arc<ValidatedChunk>,
        host: lkjscript_host::HostEnvironment,
    ) -> Result<ApplicationId, RuntimeError> {
        manifest.validate()?;
        if !matches!(manifest.cell, ExecutionCellClass::TrustedInProcess) {
            return Err(RuntimeError::ExecutionCellClassMismatch);
        }
        validate_providers(&manifest, &host)?;
        for capability in chunk.required_capabilities() {
            if manifest.capabilities.binary_search(capability).is_err() {
                return Err(RuntimeError::CapabilityNotGranted(*capability));
            }
        }
        let mut state = self.lock_state()?;
        let application = ApplicationId::from_nonzero(state.allocate()?);
        let lease = state.cache.lease(package, chunk)?;
        state.apps.insert(
            application,
            AppRecord {
                manifest,
                package,
                chunk: Some(lease),
                process_spec: None,
                host,
                lifecycle: Lifecycle::Installed,
                incarnation_counter: 0,
                instance: None,
            },
        );
        Ok(application)
    }

    pub fn install_isolated(
        &self,
        manifest: ApplicationManifest,
        package: PackageContentId,
        package_root: &Path,
        worker: &Path,
        host: lkjscript_host::HostEnvironment,
    ) -> Result<ApplicationId, RuntimeError> {
        manifest.validate()?;
        let ExecutionCellClass::IsolatedProcess { entry } = &manifest.cell else {
            return Err(RuntimeError::ExecutionCellClassMismatch);
        };
        validate_providers(&manifest, &host)?;
        let package_root = package_root
            .canonicalize()
            .map_err(|error| RuntimeError::ProcessCell(format!("package root: {error}")))?;
        if !package_root.is_dir() {
            return Err(RuntimeError::ProcessCell(
                "package root is not a directory".into(),
            ));
        }
        let entry = package_root
            .join(entry.as_str())
            .canonicalize()
            .map_err(|error| RuntimeError::ProcessCell(format!("package entry: {error}")))?;
        if !entry.starts_with(&package_root) || !entry.is_file() {
            return Err(RuntimeError::ProcessCell(
                "package entry escapes root or is not a file".into(),
            ));
        }
        if entry.to_str().is_none() {
            return Err(RuntimeError::ProcessCell(
                "package entry is not UTF-8".into(),
            ));
        }
        let worker = worker
            .canonicalize()
            .map_err(|error| RuntimeError::ProcessCell(format!("worker executable: {error}")))?;
        if !worker.is_file() {
            return Err(RuntimeError::ProcessCell(
                "worker executable is not a file".into(),
            ));
        }
        let mut state = self.lock_state()?;
        let application = ApplicationId::from_nonzero(state.allocate()?);
        state.apps.insert(
            application,
            AppRecord {
                manifest,
                package,
                chunk: None,
                process_spec: Some(IsolatedProcessSpec { worker, entry }),
                host,
                lifecycle: Lifecycle::Installed,
                incarnation_counter: 0,
                instance: None,
            },
        );
        Ok(application)
    }
}

fn validate_providers(
    manifest: &ApplicationManifest,
    host: &lkjscript_host::HostEnvironment,
) -> Result<(), RuntimeError> {
    for capability in &manifest.capabilities {
        if !crate::providers::supports(*capability, host) {
            return Err(RuntimeError::UnsupportedCapability(*capability));
        }
    }
    Ok(())
}
