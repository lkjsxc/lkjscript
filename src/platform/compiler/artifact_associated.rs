//! Occurrence authority for complete compiler-unit type tables, including unused entries.

use super::{CompilationUnit, artifact_error};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::owned_associated::OwnedAssociatedAdmission;
use crate::platform::kernel::{
    DeclarationPayload, DeclarationReference, ExpressionRead, OwnerKey, OwnerRecord, TypeObject,
    TypeObjectDigest,
};
use std::collections::BTreeMap;

pub(super) fn admit_work(work: &mut crate::platform::kernel::ValidationWork, maximum: Option<usize>) -> Result<(), Diagnostic> {
    if !work.charge(1, maximum) {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_associated_work",
            "artifact associated table admission exhausted its explicit work budget",
        ));
    }
    Ok(())
}

pub(super) fn contains_associated_types(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    work: &mut crate::platform::kernel::ValidationWork,
    maximum: Option<usize>,
) -> Result<bool, Diagnostic> {
    // The loaded map is already the complete child closure. Charge discovery too;
    // no temporary inventory or per-unit feature scan is needed.
    for object in types.values() {
        admit_work(work, maximum)?;
        if object.form.is_associated() || object.form.is_read_view() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn validate_unit_types(
    read: &(impl ExpressionRead + ?Sized),
    unit: &CompilationUnit,
    admission: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    if read.package_id() != unit.source.package {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_associated_owner",
            "compiler-unit type admission has the wrong source package",
        ));
    }
    let record = read.owner(unit.source.owner)?.ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_associated_owner",
            "compiler-unit type admission omits its canonical source owner",
        )
    })?;
    if record.header().owner != unit.source.owner || record.kind() != unit.source.kind {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_associated_owner",
            "compiler-unit type admission has a mismatched canonical source owner",
        ));
    }
    // Keep the canonical owner's actual generation. A current compiler unit does
    // not upgrade a historical source owner or grant it associated authority.
    let result = match (&record, unit.source.owner) {
        (OwnerRecord::Declaration(declaration), OwnerKey::Declaration(id))
            if matches!(
                declaration.payload,
                DeclarationPayload::OwnedContract(_) | DeclarationPayload::OwnedImplementation(_)
            ) =>
        {
            // The kernel enumerates and proves each actual inline method/member/
            // binding occurrence and its complete equations. A shared table does
            // not grant one inline root another root's binder authority.
            admission.validate_aggregate_type_at(
                read,
                DeclarationReference {
                    package: unit.source.package,
                    declaration: id,
                },
                &record,
                &unit.tables.types,
            )
        }
        _ => {
            let occurrence = crate::platform::kernel::memory::owner_occurrence(
                read,
                unit.source.owner,
                &record,
            )?
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_associated_owner",
                    "compiler-unit source has no independent type occurrence context",
                )
            })?;
            admission.validate_occurrence_at(read, &unit.tables.types, occurrence)
        }
    };
    result.map_err(|error| {
        if matches!(
            error.class,
            DiagnosticClass::Resource | DiagnosticClass::Cancelled
        ) {
            error
        } else {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_associated_type",
                format!(
                    "compiler-unit type authority is invalid: {}: {}",
                    error.code, error.message
                ),
            )
        }
    })
}

#[cfg(test)]
#[path = "artifact_associated_tests.rs"]
mod tests;
