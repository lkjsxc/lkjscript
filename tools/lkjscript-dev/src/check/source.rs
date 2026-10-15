//! Opaque admission of an original producing run. Output paths come only from
//! the current registry, after the complete original source proof is admitted.
use super::{RELEASE_SOURCE_GATE_COUNT, reader};
use crate::{
    error::DevError,
    evidence::{self, FileKind, FileProof},
    process,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceOwner {
    OfflinePackages,
    PureTail,
}

impl SourceOwner {
    pub(crate) fn gate(self) -> &'static str {
        match self {
            Self::OfflinePackages => "offline_packages",
            Self::PureTail => "pure_tail",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerProof {
    owner: SourceOwner,
    receipt: FileProof,
}

/// A serialized binding is evidence, not authority. Only AdmittedSource::read
/// authenticates it; callers compare it with that owner's newly admitted proof.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceProof {
    source_commit: String,
    required_gates: usize,
    receipt: FileProof,
    verifier: FileProof,
    workspace_stdout: FileProof,
    workspace_stderr: FileProof,
    owner_receipts: Vec<OwnerProof>,
}

impl SourceProof {
    pub(crate) fn original_path(&self, repository: &Path) -> Result<PathBuf, DevError> {
        original_path(repository, &self.receipt)
    }

    /// Evidence location only, for fault tests after the frozen CLI has admitted
    /// their aggregate baseline. This does not construct an AdmittedSource.
    #[cfg(test)]
    pub(crate) fn recorded_owner_path(
        &self,
        repository: &Path,
        owner: SourceOwner,
    ) -> Result<PathBuf, DevError> {
        let proof = self
            .owner_receipts
            .iter()
            .find(|proof| proof.owner == owner)
            .ok_or_else(|| DevError::corrupt("recorded source owner is absent"))?;
        original_path(repository, &proof.receipt)
    }
}

#[derive(Clone)]
pub(crate) struct AdmittedSource {
    proof: SourceProof,
    repository: PathBuf,
    path: PathBuf,
    owner_paths: BTreeMap<SourceOwner, PathBuf>,
}

impl AdmittedSource {
    pub(crate) fn read(repository: &Path, path: &Path, commit: &str) -> Result<Self, DevError> {
        let original = reader::read_release_source_receipt(path, repository, commit)?;
        let runtime = original
            .receipt
            .runtime
            .as_ref()
            .ok_or_else(|| DevError::corrupt("admitted source runtime is absent"))?;
        let workspace = original
            .receipt
            .gates
            .iter()
            .find(|gate| gate.name == "workspace_tests")
            .and_then(|gate| gate.process.as_ref())
            .ok_or_else(|| DevError::corrupt("admitted source workspace process is absent"))?;
        let mut owner_paths = BTreeMap::new();
        let mut owner_receipts = Vec::new();
        for owner in [SourceOwner::OfflinePackages, SourceOwner::PureTail] {
            let declared = original.registry.gate(owner.gate())?;
            let observed = original
                .receipt
                .gates
                .iter()
                .find(|gate| gate.name == owner.gate())
                .ok_or_else(|| DevError::corrupt("admitted source semantic gate is absent"))?;
            let (path, proof) = declared_output(&original.repository, declared, &observed.outputs)?;
            owner_paths.insert(owner, path);
            owner_receipts.push(OwnerProof {
                owner,
                receipt: proof,
            });
        }
        let admitted = Self {
            proof: SourceProof {
                source_commit: commit.to_owned(),
                required_gates: original.gates,
                receipt: original.proof,
                verifier: runtime.harness.clone(),
                workspace_stdout: workspace.stdout.clone(),
                workspace_stderr: workspace.stderr.clone(),
                owner_receipts,
            },
            repository: original.repository,
            path: original.path,
            owner_paths,
        };
        admitted.unchanged()?;
        Ok(admitted)
    }

    pub(crate) fn proof(&self) -> &SourceProof {
        &self.proof
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn repository(&self) -> &Path {
        &self.repository
    }
    pub(crate) fn gates(&self) -> usize {
        self.proof.required_gates
    }
    pub(crate) fn source_commit(&self) -> &str {
        &self.proof.source_commit
    }

    pub(crate) fn owner_receipt(&self, owner: SourceOwner) -> Result<&Path, DevError> {
        self.unchanged()?;
        self.owner_paths
            .get(&owner)
            .map(PathBuf::as_path)
            .ok_or_else(|| DevError::corrupt("source owner receipt was not admitted"))
    }

    pub(crate) fn owner_proof(&self, owner: SourceOwner) -> Result<FileProof, DevError> {
        let path = self.owner_receipt(owner)?;
        let mut proof = self
            .proof
            .owner_receipts
            .iter()
            .find(|item| item.owner == owner)
            .ok_or_else(|| DevError::corrupt("source owner output proof is absent"))?
            .receipt
            .clone();
        proof.path = path.display().to_string();
        Ok(proof)
    }

    pub(crate) fn workspace_logs(&self) -> Result<(String, String), DevError> {
        self.unchanged()?;
        let read = |proof: &FileProof| -> Result<String, DevError> {
            let path = original_path(&self.repository, proof)?;
            let bytes = process::read_bounded(&path, 64 * 1024 * 1024)?;
            verify(&path, proof)?;
            String::from_utf8(bytes)
                .map_err(|_| DevError::corrupt("source workspace log is not UTF-8"))
        };
        let result = (
            read(&self.proof.workspace_stdout)?,
            read(&self.proof.workspace_stderr)?,
        );
        self.unchanged()?;
        Ok(result)
    }

    pub(crate) fn unchanged(&self) -> Result<(), DevError> {
        require(
            self.proof.required_gates == RELEASE_SOURCE_GATE_COUNT,
            "source prerequisite uses a different complete gate inventory",
        )?;
        verify(&self.path, &self.proof.receipt)?;
        let current = std::env::current_exe()?;
        verify(&current, &self.proof.verifier)?;
        for proof in [&self.proof.workspace_stdout, &self.proof.workspace_stderr] {
            verify(&original_path(&self.repository, proof)?, proof)?;
        }
        for owner in &self.proof.owner_receipts {
            let path = self
                .owner_paths
                .get(&owner.owner)
                .ok_or_else(|| DevError::corrupt("source output identity is absent"))?;
            verify(path, &owner.receipt)?;
        }
        Ok(())
    }
}

fn original_path(repository: &Path, proof: &FileProof) -> Result<PathBuf, DevError> {
    let path = Path::new(&proof.path);
    require(
        !path.as_os_str().is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "source proof path is not a canonical repository-relative original",
    )?;
    Ok(repository.join(path))
}

fn declared_output(
    repository: &Path,
    gate: &super::model::Gate,
    outputs: &[FileProof],
) -> Result<(PathBuf, FileProof), DevError> {
    require(
        gate.required_outputs.len() == 1 && outputs.len() == 1,
        "source semantic owner must declare exactly its original receipt",
    )?;
    let path = gate.required_outputs[0].clone();
    let proof = outputs[0].clone();
    require(
        proof.path == evidence::relative(repository, &path),
        "source semantic receipt differs from the registry output",
    )?;
    verify(&path, &proof)?;
    Ok((path, proof))
}

fn verify(path: &Path, proof: &FileProof) -> Result<(), DevError> {
    let metadata = std::fs::symlink_metadata(path)?;
    require(
        metadata.is_file()
            && !metadata.file_type().is_symlink()
            && path.is_absolute()
            && path.canonicalize()? == path
            && proof.kind == FileKind::File
            && *proof == evidence::proof(path, proof.path.clone())?,
        "admitted source original bytes, mode or path changed",
    )
}

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
