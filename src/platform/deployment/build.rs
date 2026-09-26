//! Immutable deployment snapshots without selecting a process or rebasing local data.

mod output;
#[cfg(test)]
mod tests;

use super::{
    MAXIMUM_DEPLOYMENT_BYTES, decode_deployment, deployment_error, deployment_io,
    validate_program_descriptor,
};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::normalized_lifecycle::PreparedApplication;
use crate::platform::owned_output::{OwnedOutputReceipt, reject_symlinked_path};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct BuildTemplate {
    source: PathBuf,
    directory: PathBuf,
    value: serde_json::Value,
}

pub(crate) struct BuildSnapshot {
    pub(crate) source: PathBuf,
    pub(crate) artifact: OwnedOutputReceipt,
    pub(crate) deployment: OwnedOutputReceipt,
}

impl BuildTemplate {
    /// Observe a strict template, but never open its artifact, secrets or adapters.
    pub(crate) fn read(path: &Path) -> Result<Self, Diagnostic> {
        bounded_path(path)?;
        let name = path.file_name().ok_or_else(|| {
            deployment_error(
                "deployment_build_path",
                "deployment template needs a file name",
            )
        })?;
        let parent = path
            .parent()
            .filter(|value| !value.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        reject_symlinked_path(parent)?;
        let directory = fs::canonicalize(parent)
            .map_err(|error| deployment_io("deployment_read", parent, error))?;
        let source = directory.join(name);
        bounded_path(&source)?;
        let bytes = output::read_regular(&source, MAXIMUM_DEPLOYMENT_BYTES)?;
        decode_deployment(&bytes)?;
        // Preserve all non-artifact values, including omitted legacy policy fields.
        // Strict decoding above rejects duplicate/unknown fields before this projection.
        let value = serde_json::from_slice(&bytes).map_err(|error| {
            deployment_error(
                "deployment_build_json",
                format!("invalid deployment JSON: {error}"),
            )
        })?;
        Ok(Self {
            source,
            directory,
            value,
        })
    }

    pub(crate) fn publish(
        self,
        prepared: &PreparedApplication,
        maximum_artifact_bytes: usize,
    ) -> Result<BuildSnapshot, Diagnostic> {
        let descriptor = decode_deployment(&serde_json::to_vec(&self.value).map_err(json_error)?)?;
        validate_program_descriptor(&descriptor, &prepared.program)?;
        let artifact_parent = Path::new(&descriptor.artifact)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let relative_artifact = artifact_parent.join(format!(
            "build-{}.lkja",
            blake3::hash(&prepared.artifact_bytes).to_hex()
        ));
        let relative_artifact = relative_artifact.to_str().ok_or_else(|| {
            deployment_error("deployment_build_path", "artifact path is not UTF-8")
        })?;
        let mut value = self.value;
        value["artifact"] = serde_json::Value::String(relative_artifact.to_owned());
        let mut bytes = serde_json::to_vec_pretty(&value).map_err(json_error)?;
        bytes.push(b'\n');
        // Recheck path/container bounds after substituting the content-derived name.
        decode_deployment(&bytes)?;
        let artifact_path = self.directory.join(relative_artifact);
        let deployment_path = self.directory.join(format!(
            "build-{}.deployment.json",
            blake3::hash(&bytes).to_hex()
        ));
        bounded_path(&artifact_path)?;
        bounded_path(&deployment_path)?;
        for grant in &descriptor.grants {
            let root = match &grant.adapter {
                super::AdapterDescriptor::Data { root, .. }
                | super::AdapterDescriptor::ObjectLocal { root, .. }
                | super::AdapterDescriptor::DurableQueueData { root, .. } => root,
                _ => continue,
            };
            reject_data_overlap(&self.directory.join(root), &artifact_path, &deployment_path)?;
        }
        // Inspect both destinations before publishing either. A later race can still
        // leave an inert complete artifact, never a descriptor pointing at partial bytes.
        output::inspect_exact(
            &artifact_path,
            &prepared.artifact_bytes,
            maximum_artifact_bytes,
        )?;
        output::inspect_exact(&deployment_path, &bytes, MAXIMUM_DEPLOYMENT_BYTES)?;
        let artifact = output::publish_exact(
            &artifact_path,
            &prepared.artifact_bytes,
            maximum_artifact_bytes,
        )?;
        let deployment = output::publish_exact(
            &deployment_path, &bytes, MAXIMUM_DEPLOYMENT_BYTES,
        ).map_err(|mut error| {
            error.notes.push(format!(
                "complete build artifact retained at '{}'; original deployment and running processes are unchanged",
                artifact.path.display()
            ));
            error
        })?;
        Ok(BuildSnapshot {
            source: self.source,
            artifact,
            deployment,
        })
    }
}

fn reject_data_overlap(root: &Path, artifact: &Path, deployment: &Path) -> Result<(), Diagnostic> {
    if artifact.starts_with(root) || deployment.starts_with(root) {
        return Err(deployment_error(
            "deployment_build_data_overlap",
            "build outputs may not be placed inside a declared local data, object or queue root",
        ));
    }
    Ok(())
}

fn bounded_path(path: &Path) -> Result<(), Diagnostic> {
    if path
        .to_str()
        .is_none_or(|value| value.is_empty() || value.len() > 4096 || value.contains('\0'))
    {
        return Err(deployment_error(
            "deployment_build_path",
            "build snapshot paths must be nonempty UTF-8 paths of at most 4096 bytes",
        ));
    }
    Ok(())
}

fn json_error(error: serde_json::Error) -> Diagnostic {
    deployment_error(
        "deployment_build_json",
        format!("deployment snapshot could not be encoded: {error}"),
    )
}
