//! Compare transported instructions with exact canonical lowering before deriving runtime tails.
//! This is strict loading, not an independent evaluator or an editable program authority.
use super::*;
use crate::platform::change::{CanonicalRead, CanonicalReadWork};
use crate::platform::compiler::lower::{CodeRead, canonical_code};
use crate::platform::compiler::unit::CompiledCode;
use crate::platform::semantic_id::ExpressionId;
use std::cell::Cell;

struct Read<'a> {
    compiler_generation: u16,
    package: PackageId,
    owners: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    runtime: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    interfaces: &'a BTreeMap<PackageId, &'a PackageInterfaceValidation>,
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
    dependencies: &'a [crate::platform::kernel::DependencyRecord],
    representation_packages: &'a BTreeSet<PackageId>,
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
            tables.read_view_fields.len(),
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
        let scope = match unit.source.owner {
            OwnerKey::Declaration(declaration) => Some(declaration),
            _ => None,
        };
        self.check_scope_owners(unit, actual)?;
        let expected = canonical_code(self, self.package, scope, tables, root, parameters)?;
        if expected != *actual {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_compiled_control_meaning",
                "compiled evaluation order, operands, locals or continuations differ from canonical expressions",
            ));
        }
        Ok(())
    }

    fn check_scope_owners(
        &self,
        unit: &CompilationUnit,
        code: &CompiledCode,
    ) -> Result<(), Diagnostic> {
        use crate::platform::kernel::owned_associated::{
            AssociatedTypeContext, OwnedAssociatedAdmission, read_view_admission,
        };
        let admission = OwnedAssociatedAdmission::new();
        let mut admitted = BTreeSet::new();
        for instruction in &code.instructions {
            self.reserve(1)?;
            // Includes complete call inventories and the zero-stack-effect lexical
            // scope markers. Every instruction is visited, including untaken syntax.
            for argument in instruction.scope_arguments() {
                self.reserve(1)?;
                if admitted.contains(argument) {
                    continue;
                }
                self.reserve(1)?;
                let OwnerKey::Declaration(declaration) = unit.source.owner else {
                    return Err(scope_error("read scope has no exact callable owner"));
                };
                let callable = DeclarationReference {
                    package: unit.source.package,
                    declaration,
                };
                let result = match *argument {
                    ReadScopeArgument::Parameter(_) => read_view_admission::ScopeAuthority {
                        binder: Some(crate::platform::kernel::ReadScopeBinder::Declaration(
                            callable,
                        )),
                        lexical: Some(callable),
                        live_loans: &BTreeSet::new(),
                    }
                    .validate(self, *argument, &admission),
                    ReadScopeArgument::Lexical {
                        callable: owner,
                        binding,
                    } => {
                        if owner != callable {
                            return Err(scope_error(
                                "lexical read scope names another exact callable",
                            ));
                        }
                        // A Binding point read is insufficient. The kernel proves its
                        // exact occurrence in this canonical body's complete closure.
                        // Lifetime/containment and source provenance remain full-flow gates.
                        read_view_admission::lexical_owner(
                            self,
                            owner,
                            binding,
                            AssociatedTypeContext::Lexical(callable),
                            &admission,
                        )
                    }
                };
                result.map_err(|error| {
                    if matches!(
                        error.class,
                        DiagnosticClass::Resource | DiagnosticClass::Cancelled
                    ) {
                        error
                    } else {
                        scope_error(format!(
                            "read-scope owner is invalid: {}: {}",
                            error.code, error.message
                        ))
                    }
                })?;
                admitted.insert(*argument);
            }
        }
        Ok(())
    }
}

impl crate::platform::kernel::ExpressionRead for Read<'_> {
    fn package_id(&self) -> PackageId {
        self.package
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.code_owner(owner).map(|record| record.value)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.code_type(ty).map(|record| record.value)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.code_interface(package, owner)
            .map(|record| record.value)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.code_dependency(package).map(|record| record.value)
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.code_representation_owner(package, owner)
            .map(|record| record.value)
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.code_representation_dependency(package)
            .map(|record| record.value)
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.reserve(1)
    }
}

fn scope_error(message: impl Into<String>) -> Diagnostic {
    artifact_error(
        DiagnosticClass::Corrupt,
        "artifact_compiled_control_scope",
        message,
    )
}

impl CodeRead for Read<'_> {
    fn code_dependency(&self, package: PackageId) -> Result<CanonicalRead<bool>, Diagnostic> {
        self.observed(|| {
            self.dependencies
                .binary_search_by_key(&package, |dependency| dependency.package)
                .is_ok()
        })
    }
    fn code_representation_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<bool>, Diagnostic> {
        self.observed(|| self.representation_packages.contains(&package))
    }
    fn code_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.observed(|| {
            if !self.representation_packages.contains(&package) {
                return None;
            }
            self.interfaces
                .get(&package)
                .and_then(|interface| interface.owners.get(&owner))
                .map(|owner| owner.record.clone())
        })
    }

    fn code_step(&self) -> Result<(), Diagnostic> {
        self.reserve(1)
    }
    fn code_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        let result = self.observed(|| {
            self.owners
                .get(&(self.package, owner))
                .or_else(|| self.runtime.get(&(self.package, owner)))
                .cloned()
        })?;
        if self.compiler_generation < 14
            && result.value.as_ref().is_some_and(|owner| {
                owner.type_roots().iter().any(|ty| {
                    self.types
                        .get(ty)
                        .is_some_and(|t| matches!(t.form, TypeForm::ByteBuffer))
                })
            })
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_buffer_generation",
                "predecessor compiled control refers to a ByteBuffer contract",
            ));
        }
        Ok(result)
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
            if self
                .dependencies
                .binary_search_by_key(&package, |dependency| dependency.package)
                .is_err()
            {
                return None;
            }
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
    revisions: &BTreeMap<
        PackageRevisionDigest,
        crate::platform::package_transport::PackageRevision,
    >,
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
    let mut representation_package = None;
    let mut representation_packages = BTreeSet::new();
    for ((package, owner), unit) in units {
        let dependencies = manifest
            .packages
            .binary_search_by_key(package, |package| package.package)
            .ok()
            .and_then(|index| revisions.get(&manifest.packages[index].package_revision))
            .map(|revision| revision.dependencies.as_slice())
            .ok_or_else(missing)?;
        if representation_package != Some(*package) {
            let mut work = 0;
            representation_packages =
                artifact_representation_scope(dependencies, revisions, &mut work, remaining.get())?;
            remaining.set(remaining.get() - work);
            representation_package = Some(*package);
        }
        let read = Read {
            compiler_generation: unit.contract_version,
            package: *package,
            owners,
            runtime,
            interfaces: &interfaces,
            types,
            dependencies,
            representation_packages: &representation_packages,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_representation_reads_exclude_unrelated_artifact_branches() {
        let (revisions, [root, independent, _, leaf]) =
            super::super::contract_tests::branched_revisions();
        let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
            "declarations.begin\n(units (module create leaf (owned-record create Leaf (visibility private))))\ndeclarations.end\n",
        ).unwrap();
        let (owner, record) = source.owners.iter().find(|(_, owner)|
            matches!(owner, OwnerRecord::Declaration(record) if matches!(record.payload, DeclarationPayload::OwnedRecord { .. }))
        ).unwrap();
        let interface = PackageInterfaceValidation {
            owners: BTreeMap::from([(
                *owner,
                crate::platform::package_interface::PackageInterfaceOwner {
                    contract_version:
                        crate::platform::package_interface::PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::project_public(record)
                        .unwrap()
                        .unwrap(),
                },
            )]),
            type_objects: BTreeMap::new(),
            reachable_objects: BTreeSet::new(),
            map_work: MapWork::default(),
        };
        let interfaces = BTreeMap::from([(leaf.package, &interface)]);
        let owners = BTreeMap::new();
        let types = BTreeMap::new();
        let remaining = Cell::new(crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK);
        for (package, expected_representation) in [(&root, true), (&independent, false)] {
            let representation_packages = artifact_representation_scope(
                &package.dependencies,
                &revisions,
                &mut 0,
                remaining.get(),
            )
            .unwrap();
            let read = Read {
                compiler_generation: COMPILER_UNIT_CONTRACT_VERSION,
                package: package.package,
                owners: &owners,
                runtime: &owners,
                interfaces: &interfaces,
                types: &types,
                dependencies: &package.dependencies,
                representation_packages: &representation_packages,
                remaining: &remaining,
            };
            assert!(!read.code_dependency(leaf.package).unwrap().value);
            assert!(
                read.code_interface(leaf.package, *owner)
                    .unwrap()
                    .value
                    .is_none()
            );
            assert_eq!(
                read.code_representation_dependency(leaf.package)
                    .unwrap()
                    .value,
                expected_representation
            );
            assert_eq!(
                read.code_representation_owner(leaf.package, *owner)
                    .unwrap()
                    .value
                    .is_some(),
                expected_representation
            );
        }
    }
    #[test]
    fn byte_buffer_predecessor_control_admission_checks_canonical_local_annotations() {
        let source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author("").unwrap();
        let buffer = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let key = source
            .owners
            .iter()
            .find_map(|(key, owner)| {
                matches!(owner, OwnerRecord::Binding(b) if b.declared_type == Some(buffer))
                    .then_some(*key)
            })
            .unwrap();
        let owners = source
            .owners
            .iter()
            .map(|(key, owner)| ((source.root.package_id, *key), owner.clone()))
            .collect();
        let runtime = BTreeMap::new();
        let interfaces = BTreeMap::new();
        for generation in [13, 14] {
            let read = Read {
                compiler_generation: generation,
                package: source.root.package_id,
                owners: &owners,
                runtime: &runtime,
                interfaces: &interfaces,
                types: &source.types,
                dependencies: &[],
                representation_packages: &BTreeSet::new(),
                remaining: &Cell::new(crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK),
            };
            let result = read.code_owner(key);
            if generation == 13 {
                assert_eq!(result.unwrap_err().code, "artifact_buffer_generation");
            } else {
                assert_eq!(result.unwrap().value, source.owners.get(&key).cloned());
            }
        }
    }
}
