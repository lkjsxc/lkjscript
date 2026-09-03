//! Shared Graph 8 compiler, linker, artifact-loader, and dense-runtime preparation.

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
}

impl PreparedApplication {
    pub fn check(&self, control: &ExecutionControl) -> Result<NormalizedTestReceipt, Diagnostic> {
        let view = self.repository.view_current()?;
        require_unchanged_authority(self, &view)?;
        run_graph_tests(&view, &self.program, None, Default::default(), control)
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
            &view,
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
    let exported = repository.export_package_transport()?;
    let dependencies = match exported.revision.dependencies.as_slice() {
        [] => Vec::new(),
        [dependency] => {
            let standard = BuiltinStandard::load()?;
            if dependency.package != standard.package
                || dependency.semantic_revision != standard.semantic_revision
                || dependency.package_revision != standard.package_revision
            {
                return Err(lifecycle_error(
                    DiagnosticClass::Semantic,
                    "lifecycle_dependency_closure",
                    "current command lifecycle supports only the exact built-in standard dependency",
                ));
            }
            vec![standard.artifact.clone()]
        }
        _ => {
            return Err(lifecycle_error(
                DiagnosticClass::Semantic,
                "lifecycle_dependency_closure",
                "current command lifecycle supports only the exact built-in standard dependency",
            ));
        }
    };

    let cached = load_current_compilation(&repository);
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
                let receipt = clean_compilation(&repository)?;
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
                let receipt = clean_compilation(&repository)?;
                (
                    LifecycleCacheProfile::CleanRecovery,
                    receipt.manifest_digest,
                    receipt.units_compiled,
                    receipt.units_reused,
                    receipt.units_removed,
                    Some(diagnostic),
                )
            }
        };
    let linked = link_artifact(&repository, compilation, &dependencies)?;
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
        units_compiled,
        units_reused,
        units_removed,
        cache_recovery,
        artifact_manifest,
        artifact_bundle,
        artifact_bytes,
        link_work: linked.work,
        program,
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
    fn maintained_application_clean_build_matches_its_owned_artifact() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let repository = GraphRepository::open(&root).expect("open maintained Graph 8 lkjournal");
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
