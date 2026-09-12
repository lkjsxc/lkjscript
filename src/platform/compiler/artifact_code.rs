//! Compare transported instructions with exact canonical lowering before deriving runtime tails.
//! This is strict loading, not an independent evaluator or an editable program authority.
use super::*;
use crate::platform::change::{CanonicalRead, CanonicalReadWork};
use crate::platform::compiler::lower::{CodeRead, canonical_code};
use crate::platform::compiler::unit::CompiledCode;
use crate::platform::semantic_id::ExpressionId;
use std::cell::Cell;

struct Read<'a> {
    package: PackageId,
    owners: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    runtime: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    interfaces: &'a BTreeMap<PackageId, &'a PackageInterfaceValidation>,
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
    remaining: &'a Cell<usize>,
}

impl Read<'_> {
    fn reserve(&self, count: usize) -> Result<(), Diagnostic> {
        let remaining = self.remaining.get().checked_sub(count).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Resource,
                "artifact_compiled_control_work",
                "canonical instruction admission exceeds the existing validation work bound",
            )
        })?;
        self.remaining.set(remaining);
        Ok(())
    }

    fn observed<T>(&self, read: impl FnOnce() -> T) -> Result<CanonicalRead<T>, Diagnostic> {
        self.reserve(1)?;
        Ok(CanonicalRead {
            value: read(),
            work: CanonicalReadWork {
                point_reads: 1,
                ..Default::default()
            },
        })
    }

    fn check(
        &self,
        unit: &CompilationUnit,
        root: ExpressionId,
        parameters: &[ParameterId],
        actual: &CompiledCode,
    ) -> Result<(), Diagnostic> {
        let tables = &unit.tables;
        // Admit table copies and expected instructions before constructing either. Canonical
        // point reads also consume this shared per-load work budget, including rejected input.
        for count in [
            tables.declarations.len(),
            tables.fields.len(),
            tables.cases.len(),
            tables.requirements.len(),
            tables.operations.len(),
            tables.ports.len(),
            tables.types.len(),
            tables.structural_names.len(),
            tables.texts.len(),
            actual.instructions.len(),
            actual.local_count as usize,
        ] {
            self.reserve(count)?;
        }
        let expected = canonical_code(self, self.package, tables, root, parameters)?;
        if expected != *actual {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_compiled_control_meaning",
                "compiled evaluation order, operands, locals or continuations differ from canonical expressions",
            ));
        }
        Ok(())
    }
}

impl CodeRead for Read<'_> {
    fn code_step(&self) -> Result<(), Diagnostic> {
        self.reserve(1)
    }
    fn code_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        self.observed(|| {
            self.owners
                .get(&(self.package, owner))
                .or_else(|| self.runtime.get(&(self.package, owner)))
                .cloned()
        })
    }
    fn code_type(
        &self,
        ty: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.observed(|| self.types.get(&ty).cloned())
    }
    fn code_interface(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.observed(|| {
            self.interfaces
                .get(&package)
                .and_then(|interface| interface.owners.get(&owner))
                .map(|owner| owner.record.clone())
        })
    }
}

pub(super) fn validate(
    manifest: &ArtifactManifest,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    owners: &BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    runtime: &BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    interfaces: &BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
) -> Result<(), Diagnostic> {
    let interfaces = manifest
        .packages
        .iter()
        .map(|package| {
            interfaces
                .get(&package.package_revision)
                .map(|interface| (package.package, interface))
                .ok_or_else(missing)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let remaining = Cell::new(crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK);
    for ((package, owner), unit) in units {
        let read = Read {
            package: *package,
            owners,
            runtime,
            interfaces: &interfaces,
            types,
            remaining: &remaining,
        };
        match &unit.payload {
            CompilationPayload::Function { code, .. }
            | CompilationPayload::Constant { code, .. } => {
                let record = read.code_owner(*owner)?.value;
                match record {
                    Some(OwnerRecord::Declaration(record)) => match record.payload {
                        DeclarationPayload::Function(function) => {
                            read.check(unit, function.body, &function.parameters, code)?
                        }
                        DeclarationPayload::Constant { value, .. } => {
                            read.check(unit, value, &[], code)?
                        }
                        _ => return Err(missing()),
                    },
                    _ => return Err(missing()),
                }
            }
            CompilationPayload::Test {
                actual, expected, ..
            } => {
                let Some(OwnerRecord::Declaration(record)) = read.code_owner(*owner)?.value else {
                    return Err(missing());
                };
                let DeclarationPayload::Test {
                    actual: root,
                    expected: expected_root,
                    ..
                } = record.payload
                else {
                    return Err(missing());
                };
                read.check(unit, root, &[], actual)?;
                read.check(unit, expected_root, &[], expected)?;
            }
            CompilationPayload::Component { ports, .. } => {
                for port in ports {
                    if let CompiledPortImplementation::Expression(code) = &port.implementation {
                        let reference =
                            table_value(&unit.tables.ports, port.port, "compiled port")?;
                        let Some(OwnerRecord::Port(record)) =
                            read.code_owner(OwnerKey::Port(reference.port))?.value
                        else {
                            return Err(missing());
                        };
                        let PortImplementation::Expression(root) = record.implementation else {
                            return Err(missing());
                        };
                        read.check(unit, root, &[], code)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn missing() -> Diagnostic {
    artifact_error(
        DiagnosticClass::Corrupt,
        "artifact_compiled_control_owner",
        "compiled code has no matching canonical expression owner",
    )
}
