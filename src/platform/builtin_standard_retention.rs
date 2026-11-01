//! Finite retained-material accounting, deliberately not allocator or RSS accounting.
//! Encoded bytes cover variable strings/vectors with a fourfold decoded allowance;
//! each decoded inventory record reserves another 256 modeled bytes. All copies count.

use super::*;
use crate::platform::package_transport::source::SourceObservation;

pub(super) const PROGRAM_MATERIAL_RESERVATION: usize = 16 * 1_048_576;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RetainedMaterial {
    pub bytes: usize,
    pub records: usize,
}

impl RetainedMaterial {
    fn add(&mut self, bytes: usize, records: usize) -> Option<()> {
        self.bytes = self
            .bytes
            .checked_add(bytes)?
            .checked_add(records.checked_mul(256)?)?;
        self.records = self.records.checked_add(records)?;
        Some(())
    }

    pub fn require_within(self, bytes: usize, records: usize) -> Result<Self, Diagnostic> {
        if self.bytes > bytes || self.records > records {
            return Err(builtin_retention_limit());
        }
        Ok(self)
    }
}

// A finite cache-retention proof requires exact counts. A saturated passive
// observation remains inexact even if a later transformation reduced its value.
pub(super) fn exact_observation(value: SourceObservation) -> Option<usize> {
    if value.saturated {
        None
    } else {
        usize::try_from(value.consumed).ok()
    }
}

fn sum(values: impl IntoIterator<Item = usize>) -> Option<usize> {
    values.into_iter().try_fold(0, usize::checked_add)
}

pub(super) fn retained_material(
    transport_bytes: usize,
    artifact_bytes: usize,
    admitted: &AdmittedClosure,
    oracle: &super::super::package_transport::oracle::OracleClosure,
    artifact: &LoadedArtifact,
    public_interface_records: usize,
) -> Option<RetainedMaterial> {
    let mut retained = RetainedMaterial::default();
    retained.add(transport_bytes.checked_add(artifact_bytes)?, 1)?;
    let source_bytes = sum(admitted.container.objects.values().map(Vec::len))?;
    retained.add(
        source_bytes,
        admitted
            .container
            .objects
            .len()
            .checked_add(admitted.container.selections.len())?,
    )?;
    for package in admitted.packages.values() {
        let snapshot = &package.snapshot;
        let source_records = sum([
            1,
            snapshot.owners.len(),
            snapshot.types.len(),
            snapshot.dependency_interfaces.len(),
            snapshot.dependency_types.len(),
            snapshot.representation_dependencies.len(),
            snapshot.blobs.len(),
            snapshot.dependencies.len(),
            snapshot.retirements.len(),
            package.interface_owners.len(),
            package.interface_types.len(),
        ])?;
        retained.add(source_bytes.checked_mul(4)?, source_records)?;
        let witness = &package.witness;
        let witness_bytes = sum([
            witness.manifest_bytes.len(),
            witness.pages.stored_bytes(),
            sum(witness.summary_objects.values().map(Vec::len))?,
        ])?;
        let entries = &witness.entries;
        let witness_records = sum([
            witness.summaries.len(),
            witness.summary_objects.len(),
            witness.pages.object_count(),
            entries.summaries.len(),
            entries.namespaces.len(),
            entries.ownership.len(),
            entries.relations.len(),
            entries.reverse_relations.len(),
            entries.test_dependencies.len(),
            entries.test_dependencies_by_test.len(),
            sum(entries
                .test_dependencies_by_test
                .values()
                .map(|set| set.len()))?,
        ])?;
        retained.add(witness_bytes.checked_mul(4)?, witness_records)?;
        let (oracle_bytes, oracle_records) = oracle.package_copy_material(
            package.revision.package,
            SourceObservation::exact(u64::try_from(source_bytes).ok()?),
        )?;
        // package_copy_material already includes its per-record allowance. Its
        // passive lower bounds cannot establish this finite retention proof.
        retained.bytes = retained
            .bytes
            .checked_add(exact_observation(oracle_bytes)?)?;
        retained.records = retained
            .records
            .checked_add(exact_observation(oracle_records)?)?;
    }
    // Public interface maps are owned separately from both admitted closures.
    retained.add(source_bytes.checked_mul(4)?, public_interface_records)?;
    let artifact_records = sum([
        artifact.objects.len(),
        artifact.manifest.packages.len(),
        sum(artifact
            .manifest
            .packages
            .iter()
            .map(|package| package.runtime_owners.len()))?,
    ])?;
    retained.add(artifact_bytes.checked_mul(2)?, artifact_records)?;
    // Includes the lazy program's own LoadedArtifact copy. Admission before retaining
    // a prepared program must fit this slot; unused capacity cannot fund another cache.
    retained.add(PROGRAM_MATERIAL_RESERVATION, 1)?;
    Some(retained)
}

pub(super) fn program_material(
    artifact_bytes: usize,
    program: &super::super::execution::normalized::NormalizedProgram,
) -> Option<RetainedMaterial> {
    let work = &program.work;
    let records = [
        work.packages,
        work.compiler_units,
        work.runtime_owners,
        work.type_objects,
        work.instructions,
        work.functions,
        work.record_layouts,
        work.read_view_layouts,
        work.variant_layouts,
        work.requirements,
        work.operations,
        work.components,
        work.ports,
        work.targets,
        work.http_routes,
        work.tests,
    ]
    .into_iter()
    .try_fold(0_u64, u64::checked_add)?;
    let mut material = RetainedMaterial::default();
    material.add(
        artifact_bytes
            .checked_mul(4)?
            .checked_add(usize::try_from(work.type_metadata_bytes).ok()?)?,
        usize::try_from(records).ok()?,
    )?;
    Some(material)
}
