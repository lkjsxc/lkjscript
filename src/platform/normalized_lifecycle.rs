//! Shared Graph 10 compiler, linker, artifact-loader, and dense-runtime preparation.

#[cfg(test)]
use super::builtin_standard::BuiltinStandard;
use super::compiler::{
    ArtifactBundleDigest, ArtifactLinkWork, ArtifactManifestDigest, CompilationBuildProfile,
    CompilationManifestDigest, OptimizationPolicy, build_clean, link_artifact,
    load_current_compilation,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::ExecutionControl;
use super::execution::normalized::{
    NormalizedCommandPolicy, NormalizedCommandReceipt, NormalizedProgram, NormalizedTestReceipt,
    run_graph_tests, run_pure_command,
};
use super::kernel::{PackageId, SemanticStateDigest};
use super::publication::GraphRepository;
use super::semantic_id::{RepositoryId, RevisionId};
use super::{compiler::load_artifact, kernel::Name};
#[cfg(test)]
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleCacheProfile {
    ExactCurrent,
    Clean,
    CleanRecovery,
}

impl LifecycleCacheProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ExactCurrent => "exact-current",
            Self::Clean => "clean",
            Self::CleanRecovery => "clean-recovery",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreparedApplication {
    pub repository: GraphRepository,
    pub repository_id: RepositoryId,
    pub package: PackageId,
    pub revision: RevisionId,
    pub semantic_state: SemanticStateDigest,
    pub cache_profile: LifecycleCacheProfile,
    pub compilation: CompilationManifestDigest,
    pub units_compiled: u64,
    pub units_reused: u64,
    pub units_removed: u64,
    pub cache_recovery: Option<Diagnostic>,
    pub artifact_manifest: ArtifactManifestDigest,
    pub artifact_bundle: ArtifactBundleDigest,
    pub artifact_bytes: Vec<u8>,
    pub link_work: ArtifactLinkWork,
    pub program: NormalizedProgram,
    pub(crate) reference: PackageReference,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageReference {
    binding: super::execution::normalized::NormalizedReferenceBinding,
    oracle: super::package_transport::oracle::OracleClosure,
    schema: std::sync::Arc<super::execution::normalized::NormalizedReferenceSchema>,
    blobs: std::collections::BTreeMap<super::kernel::BlobObjectDigest, Vec<u8>>,
}

impl super::execution::normalized::NormalizedReferenceRead for PackageReference {
    fn schema(
        &self,
    ) -> Result<
        std::sync::Arc<super::execution::normalized::NormalizedReferenceSchema>,
        super::execution::ExecutionError,
    > {
        Ok(std::sync::Arc::clone(&self.schema))
    }

    fn blob(
        &self,
        digest: super::kernel::BlobObjectDigest,
    ) -> Result<Vec<u8>, super::execution::ExecutionError> {
        self.blobs.get(&digest).cloned().ok_or_else(|| super::execution::ExecutionError::new(
            super::execution::ExecutionFailureClass::Infrastructure,
            "normalized_reference_blob_missing",
            "canonical blob is absent from independently reconstructed source; restage exact closure",
        ))
    }

    fn binding(
        &self,
    ) -> Result<
        super::execution::normalized::NormalizedReferenceBinding,
        super::execution::ExecutionError,
    > {
        Ok(self.binding)
    }
    fn owner(
        &self,
        owner: super::kernel::OwnerKey,
    ) -> Result<
        super::execution::normalized::NormalizedReferenceOwnerRead,
        super::execution::ExecutionError,
    > {
        self.owner_in_package(self.binding.package, owner)
    }
    fn owner_in_package(
        &self,
        package: PackageId,
        owner: super::kernel::OwnerKey,
    ) -> Result<
        super::execution::normalized::NormalizedReferenceOwnerRead,
        super::execution::ExecutionError,
    > {
        let snapshot = self.oracle.snapshots.get(&package).ok_or_else(|| {
            super::execution::ExecutionError::new(
                super::execution::ExecutionFailureClass::Infrastructure,
                "normalized_reference_dependency_package",
                "exact package is absent from independently reconstructed canonical source",
            )
        })?;
        Ok(super::execution::normalized::NormalizedReferenceOwnerRead {
            record: snapshot.owners.get(&owner).cloned(),
            work: super::execution::normalized::NormalizedReferenceReadWork {
                owner_reads: 1,
                ..Default::default()
            },
        })
    }
}

impl PreparedApplication {
    pub fn check(&self, control: &ExecutionControl) -> Result<NormalizedTestReceipt, Diagnostic> {
        let view = self.repository.view_current()?;
        require_unchanged_authority(self, &view)?;
        run_graph_tests(
            &self.reference,
            &self.program,
            None,
            Default::default(),
            control,
        )
    }

    pub fn run(
        &self,
        target: &Name,
        arguments_json: &[u8],
        policy: NormalizedCommandPolicy,
        control: &ExecutionControl,
    ) -> Result<NormalizedCommandReceipt, Diagnostic> {
        let view = self.repository.view_current()?;
        require_unchanged_authority(self, &view)?;
        run_pure_command(
            &self.reference,
            &self.program,
            target,
            arguments_json,
            policy,
            control,
        )
    }
}

#[cfg(test)]
pub fn prepare_application(project: &Path) -> Result<PreparedApplication, Diagnostic> {
    let repository = super::project_discovery::discover_project(project)?;
    prepare_repository(repository)
}

pub fn prepare_repository(repository: GraphRepository) -> Result<PreparedApplication, Diagnostic> {
    let current = repository.current()?;
    let closure = repository.export_package_container()?;
    let oracle = super::package_transport::oracle::reconstruct(&closure.container)?;
    let mut schema = super::execution::normalized::NormalizedReferenceSchema::reconstruct(
        oracle.snapshots.values(),
    )
    .map_err(|error| {
        Diagnostic::new(
            if error.class == super::execution::ExecutionFailureClass::Resource {
                DiagnosticClass::Resource
            } else {
                DiagnosticClass::Corrupt
            },
            error.code,
            error.message,
        )
    })?;
    // This inventory is prepared once with canonical admission, then shared immutably. Execution
    // observations charge their actual owner/blob reads, not a fictitious reconstruction per call.
    schema.work = Default::default();
    let blobs = closure
        .container
        .objects
        .iter()
        .filter(|(key, _)| key.domain == super::storage::object::ObjectDomain::Blob)
        .map(|(key, bytes)| {
            (
                super::kernel::BlobObjectDigest::from_bytes(key.digest.bytes()),
                bytes.clone(),
            )
        })
        .collect();
    let reference = PackageReference {
        binding: super::execution::normalized::NormalizedReferenceBinding {
            repository: current.head.repository_id,
            package: current.semantic_root.package_id,
            revision: Some(current.head.revision),
            semantic_state: Some(current.accepted.semantic_state),
        },
        oracle,
        schema: std::sync::Arc::new(schema),
        blobs,
    };
    if reference
        .oracle
        .snapshots
        .get(&current.semantic_root.package_id)
        .is_none_or(|snapshot| {
            super::kernel::semantic_state_digest(snapshot).ok()
                != Some(current.accepted.semantic_state)
        })
    {
        return Err(lifecycle_error(
            DiagnosticClass::Corrupt,
            "lifecycle_reference_binding",
            "independent package source inventory differs from accepted authority",
        ));
    }
    let mut compiled = std::collections::BTreeMap::new();
    let mut dependency_units = 0_u64;
    for revision in &closure.dependency_order {
        if *revision == closure.container.root.package_revision {
            continue;
        }
        let package = &closure.packages[revision];
        let dependencies = package
            .revision
            .dependencies
            .iter()
            .map(|dependency| {
                compiled
                    .get(&dependency.package_revision)
                    .cloned()
                    .ok_or_else(|| {
                        lifecycle_error(
                            DiagnosticClass::Corrupt,
                            "lifecycle_dependency_order",
                            "validated source dependency order is incomplete",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let linked =
            super::compiler::compile_immutable(package, &closure.container.objects, &dependencies)?;
        let compiled_units = package
            .snapshot
            .owners
            .values()
            .filter(|owner| owner.kind().has_compilation_unit())
            .count() as u64;
        dependency_units = dependency_units
            .checked_add(compiled_units)
            .ok_or_else(|| {
                lifecycle_error(
                    DiagnosticClass::Resource,
                    "lifecycle_dependency_units",
                    "dependency compilation unit count overflowed",
                )
            })?;
        compiled.insert(*revision, load_artifact(&linked.artifact.bytes)?);
    }
    let root = &closure.packages[&closure.container.root.package_revision];
    let dependencies = root
        .revision
        .dependencies
        .iter()
        .map(|dependency| {
            compiled
                .get(&dependency.package_revision)
                .cloned()
                .ok_or_else(|| {
                    lifecycle_error(
                        DiagnosticClass::Corrupt,
                        "lifecycle_dependency_missing",
                        "exact compiled dependency is missing",
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let cached = load_current_compilation(&repository).and_then(|cached| {
        if let Some(cached) = &cached {
            super::compiler::validate_current_compilation(&repository, cached.digest)?;
        }
        Ok(cached)
    });
    // A missing pointer does not imply that disposable objects at the derived keys are sound.
    // Admit a failed clean rebuild to the same source-validated recovery branch as a bad pointer.
    let mut clean_result = None;
    let mut clean_attempted = false;
    let cached = match cached {
        Ok(None) => {
            clean_attempted = true;
            match clean_compilation(&repository) {
                Ok(receipt) => {
                    clean_result = Some(receipt);
                    Ok(None)
                }
                Err(diagnostic) => Err(diagnostic),
            }
        }
        other => other,
    };
    let mut recovered_link = None;
    let (cache_profile, compilation, units_compiled, units_reused, units_removed, cache_recovery) =
        match cached {
            Ok(Some(cached)) => (
                LifecycleCacheProfile::ExactCurrent,
                cached.digest,
                0,
                cached.manifest.units.entries(),
                0,
                None,
            ),
            Ok(None) => {
                let receipt = clean_result.ok_or_else(|| {
                    lifecycle_error(
                        DiagnosticClass::Corrupt,
                        "lifecycle_clean_result",
                        "clean compilation omitted its derived receipt",
                    )
                })?;
                (
                    LifecycleCacheProfile::Clean,
                    receipt.manifest_digest,
                    receipt.units_compiled,
                    receipt.units_reused,
                    receipt.units_removed,
                    None,
                )
            }
            Err(diagnostic) => {
                // A corrupt disposable unit can share its key with the clean result. Recovery
                // compiles admitted immutable source without rewriting damaged immutable packs.
                // Revalidate source first: canonical corruption is never a cache miss.
                let rechecked = repository.export_package_container()?;
                if rechecked.container.objects != closure.container.objects {
                    return Err(lifecycle_error(
                        DiagnosticClass::Corrupt,
                        "lifecycle_source_changed",
                        "canonical source changed during cache recovery; restage and retry",
                    ));
                }
                // Preserve exact-current recovery for a disposable pointer when immutable unit
                // objects remain sound. Only a failed clean persistence attempt needs the
                // read-only fallback; a bad immutable unit must never be overwritten.
                if let Some(receipt) = (!clean_attempted)
                    .then(|| clean_compilation(&repository).ok())
                    .flatten()
                {
                    (
                        LifecycleCacheProfile::CleanRecovery,
                        receipt.manifest_digest,
                        receipt.units_compiled,
                        receipt.units_reused,
                        receipt.units_removed,
                        Some(diagnostic),
                    )
                } else {
                    let linked = super::compiler::compile_immutable(
                        root,
                        &closure.container.objects,
                        &dependencies,
                    )?;
                    let manifest = linked
                        .artifact
                        .manifest
                        .packages
                        .iter()
                        .find(|package| package.package == current.semantic_root.package_id)
                        .ok_or_else(|| {
                            lifecycle_error(
                                DiagnosticClass::Corrupt,
                                "lifecycle_recovery_package",
                                "clean recovery lost the exact root package",
                            )
                        })?
                        .compilation;
                    recovered_link = Some(linked);
                    (
                        LifecycleCacheProfile::CleanRecovery,
                        manifest,
                        root.snapshot
                            .owners
                            .values()
                            .filter(|owner| owner.kind().has_compilation_unit())
                            .count() as u64,
                        0,
                        0,
                        Some(diagnostic),
                    )
                }
            }
        };
    let linked = match recovered_link {
        Some(linked) => linked,
        None => link_artifact(&repository, compilation, &dependencies)?,
    };
    let artifact_bytes = linked.artifact.bytes;
    let artifact = load_artifact(&artifact_bytes)?;
    let artifact_manifest = artifact.manifest_digest;
    let artifact_bundle = artifact.bundle_digest;
    let program = NormalizedProgram::prepare(artifact)?;
    let prepared = PreparedApplication {
        repository,
        repository_id: current.head.repository_id,
        package: current.semantic_root.package_id,
        revision: current.head.revision,
        semantic_state: current.accepted.semantic_state,
        cache_profile,
        compilation,
        units_compiled: units_compiled
            .checked_add(dependency_units)
            .ok_or_else(|| {
                lifecycle_error(
                    DiagnosticClass::Resource,
                    "lifecycle_dependency_units",
                    "total compilation unit count overflowed",
                )
            })?,
        units_reused,
        units_removed,
        cache_recovery,
        artifact_manifest,
        artifact_bundle,
        artifact_bytes,
        link_work: linked.work,
        program,
        reference,
    };
    let view = prepared.repository.view_current()?;
    require_unchanged_authority(&prepared, &view)?;
    Ok(prepared)
}

fn clean_compilation(
    repository: &GraphRepository,
) -> Result<super::compiler::CompilationBuildReceipt, Diagnostic> {
    let receipt = build_clean(repository, OptimizationPolicy::DeterministicBaseline)?;
    if receipt.profile != CompilationBuildProfile::Clean {
        return Err(lifecycle_error(
            DiagnosticClass::Corrupt,
            "lifecycle_cache_profile",
            "clean lifecycle preparation produced another compiler cache profile",
        ));
    }
    Ok(receipt)
}

fn require_unchanged_authority(
    prepared: &PreparedApplication,
    view: &super::publication::RepositoryView,
) -> Result<(), Diagnostic> {
    if view.current().head.repository_id != prepared.repository_id
        || view.package() != prepared.package
        || view.revision() != prepared.revision
        || view.current().accepted.semantic_state != prepared.semantic_state
    {
        return Err(lifecycle_error(
            DiagnosticClass::Semantic,
            "lifecycle_authority_changed",
            "accepted authority changed during normalized lifecycle preparation",
        ));
    }
    Ok(())
}

fn lifecycle_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn corrupt_derived_unit_rebuilds_from_canonical_source_with_and_without_cache_pointer() {
        use crate::platform::storage::{object::ObjectDomain, pack::PackMetadata};
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("consumer");
        let created = GraphRepository::create(
            &root,
            &crate::platform::kernel::tests::transport_snapshot(),
            None,
        )
        .unwrap();
        let compilation = build_clean(
            &created.repository,
            OptimizationPolicy::DeterministicBaseline,
        )
        .unwrap();
        let baseline = prepare_repository(created.repository.clone()).unwrap();
        let source = created
            .repository
            .export_package_container()
            .unwrap()
            .container
            .objects;
        let (path, original, offset) = compilation
            .seal
            .packs
            .iter()
            .find_map(|pack| {
                let path = root.join("packs").join(pack.file_name());
                let bytes = std::fs::read(&path).unwrap();
                let metadata = PackMetadata::decode(&bytes, true).unwrap();
                let entry = metadata
                    .entries
                    .iter()
                    .find(|entry| entry.key.domain == ObjectDomain::CompilerUnit)?;
                let offset = usize::try_from(entry.offset + entry.encoded_length - 1).unwrap();
                Some((path, bytes, offset))
            })
            .unwrap();
        let mut corrupted = original.clone();
        corrupted[offset] ^= 1;
        std::fs::write(&path, &corrupted).unwrap();
        for pointer_present in [true, false] {
            if !pointer_present {
                std::fs::remove_file(root.join("derived/compiler/CURRENT")).unwrap();
            }
            let recovered = prepare_repository(created.repository.clone()).unwrap();
            assert_eq!(
                recovered.cache_profile,
                LifecycleCacheProfile::CleanRecovery
            );
            assert!(recovered.cache_recovery.is_some());
            assert_eq!(recovered.artifact_bytes, baseline.artifact_bytes);
            assert_eq!(recovered.compilation, baseline.compilation);
            assert_eq!(
                created.repository.current().unwrap().head,
                created.current.head
            );
            assert_eq!(
                created
                    .repository
                    .export_package_container()
                    .unwrap()
                    .container
                    .objects,
                source
            );
            assert_eq!(
                std::fs::read(&path).unwrap(),
                corrupted,
                "recovery must not overwrite immutable storage"
            );
            assert!(
                created
                    .repository
                    .object_store()
                    .unwrap()
                    .staging_leftovers()
                    .is_empty()
            );
        }
        std::fs::write(&path, original).unwrap();
        let clean = prepare_repository(created.repository).unwrap();
        assert_eq!(clean.artifact_bytes, baseline.artifact_bytes);
    }

    #[test]
    fn maintained_application_clean_build_matches_its_owned_artifact() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let repository = GraphRepository::open(&root).expect("open maintained Graph 10 lkjournal");
        let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline)
            .expect("clean compile maintained lkjournal");
        let standard = BuiltinStandard::load().expect("load exact built-in standard");
        let linked = link_artifact(
            &repository,
            compilation.manifest_digest,
            std::slice::from_ref(&standard.artifact),
        )
        .expect("link maintained lkjournal");
        assert_eq!(
            linked.artifact.bytes.as_slice(),
            include_bytes!("../../applications/lkjournal/generated/lkjournal.lkja")
        );
    }
}
