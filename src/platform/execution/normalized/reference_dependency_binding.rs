//! Bind imported logical interfaces to the supplied canonical bodies without using
//! the package-interface producer. Snapshot inputs do not contain RevisionCore or
//! PackageRevision bytes: their digest authentication remains a source-loader duty.
//! Here every repeated exact selection must agree and every imported record and
//! structural type must agree with the supplied source inventory.

use super::{inventory_error, reference_resource, source_items};
use crate::platform::admission::{Meter, Observation};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{
    DeclarationPayload as Source, DeclarationVisibility, DependencyRecord, FunctionEffect,
    ImplementationOperand, ImplementationParameter, KernelSnapshot, OwnerRecord,
    PackageInterfaceDeclarationPayload as Interface, PackageInterfaceRecord, TypeForm, TypeObject,
    TypeObjectDigest,
};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

struct Budget<'a> {
    control: &'a ExecutionControl,
    work: &'a mut Meter,
    bytes: &'a mut Meter,
}

impl Budget<'_> {
    fn tick(&mut self) -> Result<(), ExecutionError> {
        self.control.check()?;
        self.work
            .charge(Observation::exact(1))
            .map_err(|_| capacity())?;
        Ok(())
    }

    fn entries<T>(&mut self, count: usize) -> Result<(), ExecutionError> {
        // Conservatively charge each borrowed map entry plus tree bookkeeping.
        // This is cumulative allocation, including entries subsequently discarded.
        self.bytes
            .charge(Observation::product(count, std::mem::size_of::<T>() + 128))
            .map_err(|_| capacity())?;
        Ok(())
    }

    fn items(&mut self, count: usize) -> Result<(), ExecutionError> {
        for _ in 0..count {
            self.tick()?;
        }
        Ok(())
    }

    fn same<T: PartialEq>(&mut self, left: &[T], right: &[T]) -> Result<bool, ExecutionError> {
        if left.len() != right.len() {
            return Ok(false);
        }
        for (left, right) in left.iter().zip(right) {
            self.tick()?;
            if left != right {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn capacity() -> ExecutionError {
    reference_resource(
        "normalized_reference_dependency_bound",
        "canonical dependency correspondence exhausted the selected work or metadata allowance",
    )
}

pub(super) fn validate(
    snapshots: &[&KernelSnapshot],
    control: &ExecutionControl,
    work: &mut Meter,
) -> Result<(), ExecutionError> {
    validate_with_meters(snapshots, control, work, &mut Meter::new(None))
}

// Associated preparation shares both selected allowances across its phases.
// The metadata metric is the existing cumulative borrowed-entry model, including
// correspondence indexes that are discarded before the next phase begins.
pub(super) fn validate_with_meters(
    snapshots: &[&KernelSnapshot],
    control: &ExecutionControl,
    work: &mut Meter,
    bytes: &mut Meter,
) -> Result<(), ExecutionError> {
    validate_with_budget(
        snapshots,
        &mut Budget {
            control,
            work,
            bytes,
        },
    )
}

fn validate_with_budget(
    snapshots: &[&KernelSnapshot],
    budget: &mut Budget<'_>,
) -> Result<(), ExecutionError> {
    budget.tick()?;
    let mut packages = BTreeMap::new();
    let mut types = BTreeMap::<TypeObjectDigest, &TypeObject>::new();
    for snapshot in snapshots {
        budget.tick()?;
        budget.entries::<(crate::platform::kernel::PackageId, &KernelSnapshot)>(1)?;
        if packages
            .insert(snapshot.root.package_id, *snapshot)
            .is_some()
        {
            return Err(inventory_error("duplicate canonical dependency body"));
        }
        for (digest, object) in &snapshot.types {
            budget.tick()?;
            match types.entry(*digest) {
                Entry::Occupied(previous) => {
                    if !same_type(previous.get(), object, budget)? {
                        return Err(inventory_error("canonical dependency type conflict"));
                    }
                }
                Entry::Vacant(entry) => {
                    budget.entries::<(TypeObjectDigest, &TypeObject)>(1)?;
                    entry.insert(object);
                }
            }
        }
    }
    let mut selections = BTreeMap::<_, &DependencyRecord>::new();
    let mut revisions = BTreeMap::new();
    for snapshot in snapshots {
        let mut selected_revisions = BTreeSet::new();
        if !snapshot.representation_dependencies.is_empty() {
            let reachable = authored_dependencies(snapshot, &packages, budget)?;
            for package in snapshot.representation_dependencies.keys() {
                budget.tick()?;
                if !reachable.contains(package) {
                    return Err(inventory_error(
                        "representation selection is outside the authored dependency closure",
                    ));
                }
            }
        }
        for (digest, object) in &snapshot.dependency_types {
            budget.tick()?;
            let source = types.get(digest).ok_or_else(|| {
                inventory_error("imported type has no complete canonical source body")
            })?;
            if !same_type(source, object, budget)? {
                return Err(inventory_error(
                    "imported type disagrees with canonical body",
                ));
            }
        }
        for (package, selection) in snapshot
            .dependencies
            .iter()
            .chain(&snapshot.representation_dependencies)
        {
            budget.tick()?;
            if *package != selection.package || *package == snapshot.root.package_id {
                return Err(inventory_error(
                    "dependency selection has a foreign package key",
                ));
            }
            let source = packages.get(package).ok_or_else(|| {
                inventory_error("complete canonical representation or dependency body is absent")
            })?;
            match selections.entry(*package) {
                Entry::Occupied(previous) => {
                    // The graph generation describes the binding's encoding, not
                    // the selected supplier. Representation loading may upgrade it.
                    if previous.get().semantic_revision != selection.semantic_revision
                        || previous.get().package_revision != selection.package_revision
                    {
                        return Err(inventory_error("conflicting exact dependency selections"));
                    }
                }
                Entry::Vacant(entry) => {
                    budget.entries::<(crate::platform::kernel::PackageId, &DependencyRecord)>(1)?;
                    entry.insert(selection);
                }
            }
            match revisions.entry(selection.package_revision) {
                Entry::Occupied(previous) if previous.get() != package => {
                    return Err(inventory_error(
                        "one package revision selects different packages",
                    ));
                }
                Entry::Occupied(_) => {}
                Entry::Vacant(entry) => {
                    budget.entries::<(
                        crate::platform::kernel::PackageRevisionDigest,
                        crate::platform::kernel::PackageId,
                    )>(1)?;
                    entry.insert(*package);
                }
            }
            let interface = snapshot
                .dependency_interfaces
                .get(&selection.package_revision)
                .ok_or_else(|| inventory_error("exact selected interface is absent"))?;
            if !selected_revisions.contains(&selection.package_revision) {
                budget.entries::<crate::platform::kernel::PackageRevisionDigest>(1)?;
                selected_revisions.insert(selection.package_revision);
            }
            for (key, imported) in interface.iter() {
                budget.tick()?;
                let canonical = source.owners.get(key).ok_or_else(|| {
                    inventory_error("imported owner is absent from the supplied canonical body")
                })?;
                if imported.header().owner != *key
                    || canonical.owner() != *key
                    || imported.header() != canonical.header()
                    || !same_owner(imported, canonical, budget)?
                {
                    return Err(inventory_error(
                        "imported owner disagrees with canonical body",
                    ));
                }
                check_roots(imported, snapshot, source, budget)?;
            }
        }
        for revision in snapshot.dependency_interfaces.keys() {
            budget.tick()?;
            if !selected_revisions.contains(revision) {
                return Err(inventory_error(
                    "interface has no exact dependency selection",
                ));
            }
        }
    }
    Ok(())
}

fn authored_dependencies<'a>(
    snapshot: &'a KernelSnapshot,
    packages: &BTreeMap<crate::platform::kernel::PackageId, &'a KernelSnapshot>,
    budget: &mut Budget<'_>,
) -> Result<BTreeSet<crate::platform::kernel::PackageId>, ExecutionError> {
    let mut reachable = BTreeSet::new();
    budget.entries::<&KernelSnapshot>(1)?;
    let mut pending = source_items(1)?;
    pending.push(snapshot);
    while let Some(source) = pending.pop() {
        budget.tick()?;
        for package in source.dependencies.keys() {
            budget.tick()?;
            if reachable.contains(package) {
                continue;
            }
            let child = packages.get(package).ok_or_else(|| {
                inventory_error("authored dependency body is unavailable for representation proof")
            })?;
            budget.entries::<crate::platform::kernel::PackageId>(1)?;
            budget.entries::<&KernelSnapshot>(1)?;
            reachable.insert(*package);
            pending
                .try_reserve(1)
                .map_err(|_| super::source_allocation())?;
            pending.push(*child);
        }
    }
    Ok(reachable)
}

fn same_type(
    left: &TypeObject,
    right: &TypeObject,
    budget: &mut Budget<'_>,
) -> Result<bool, ExecutionError> {
    budget.items(left.child_type_count())?;
    if let TypeForm::TaskFunction { effect, .. } = &left.form {
        budget.items(effect.requirements.len())?;
        budget.items(effect.parameters.len())?;
    }
    Ok(left == right)
}

fn same_effect(
    left: &FunctionEffect,
    right: &FunctionEffect,
    budget: &mut Budget<'_>,
) -> Result<bool, ExecutionError> {
    match (left, right) {
        (FunctionEffect::Pure, FunctionEffect::Pure) => Ok(true),
        (
            FunctionEffect::Task {
                effect_parameters: a,
                requirements: b,
            },
            FunctionEffect::Task {
                effect_parameters: c,
                requirements: d,
            },
        ) => Ok(budget.same(a, c)? && budget.same(b, d)?),
        _ => Ok(false),
    }
}

fn same_parameters(
    left: &[ImplementationParameter],
    right: &[ImplementationParameter],
    budget: &mut Budget<'_>,
) -> Result<bool, ExecutionError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (a, b) in left.iter().zip(right) {
        budget.tick()?;
        if a.id != b.id
            || a.name != b.name
            || a.contract != b.contract
            || a.self_type != b.self_type
            || !budget.same(&a.type_arguments, &b.type_arguments)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn same_operands(
    left: &[ImplementationOperand],
    right: &[ImplementationOperand],
    depth: usize,
    budget: &mut Budget<'_>,
) -> Result<bool, ExecutionError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    if left.is_empty() {
        return Ok(true);
    }
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(capacity());
    }
    for (a, b) in left.iter().zip(right) {
        budget.tick()?;
        let same = match (a, b) {
            (
                ImplementationOperand::Parameter {
                    scope: a,
                    parameter: b,
                },
                ImplementationOperand::Parameter {
                    scope: c,
                    parameter: d,
                },
            ) => a == c && b == d,
            (
                ImplementationOperand::Concrete {
                    implementation: a,
                    type_arguments: b,
                    implementations: c,
                },
                ImplementationOperand::Concrete {
                    implementation: d,
                    type_arguments: e,
                    implementations: f,
                },
            ) => a == d && budget.same(b, e)? && same_operands(c, f, depth + 1, budget)?,
            _ => false,
        };
        if !same {
            return Ok(false);
        }
    }
    Ok(true)
}

fn same_owner(
    imported: &PackageInterfaceRecord,
    canonical: &OwnerRecord,
    budget: &mut Budget<'_>,
) -> Result<bool, ExecutionError> {
    use PackageInterfaceRecord as I;
    Ok(match (imported, canonical) {
        (I::Declaration(a) | I::RepresentationDeclaration(a), OwnerRecord::Declaration(b)) => {
            let visibility_matches = match imported {
                I::Declaration(_) => b.visibility == DeclarationVisibility::Public,
                I::RepresentationDeclaration(_) => {
                    b.visibility != DeclarationVisibility::Public
                        && matches!(
                            b.payload,
                            Source::Record { .. }
                                | Source::OwnedRecord { .. }
                                | Source::Variant { .. }
                                | Source::ReadView(_)
                        )
                }
                _ => false,
            };
            visibility_matches
                && a.name == b.name
                && same_declaration(&a.payload, &b.payload, budget)?
        }
        (I::TypeParameter(a), OwnerRecord::TypeParameter(b)) => a == b,
        (I::EffectParameter(a), OwnerRecord::EffectParameter(b)) => a == b,
        (I::RequirementParameter(a), OwnerRecord::RequirementParameter(b)) => {
            budget.items(a.constraint.operations.len())?;
            a == b
        }
        (I::Field(a), OwnerRecord::Field(b)) => a == b,
        (I::Case(a), OwnerRecord::Case(b)) => a == b,
        (I::Operation(a), OwnerRecord::Operation(b)) => {
            budget.items(a.parameters.len())?;
            a == b
        }
        (I::Parameter(a), OwnerRecord::Parameter(b)) => a == b,
        (I::Requirement(a), OwnerRecord::Requirement(b)) => {
            budget.items(a.operations.len())?;
            budget.items(a.limits.len())?;
            a == b
        }
        (I::Port(a), OwnerRecord::Port(b)) => {
            a.declaration == b.declaration && a.name == b.name && a.function_type == b.function_type
        }
        _ => false,
    })
}

fn same_declaration(
    imported: &Interface,
    canonical: &Source,
    budget: &mut Budget<'_>,
) -> Result<bool, ExecutionError> {
    Ok(match (imported, canonical) {
        (
            Interface::Record {
                fields: a,
                type_parameters: b,
            },
            Source::Record {
                fields: c,
                type_parameters: d,
            },
        )
        | (
            Interface::OwnedRecord {
                fields: a,
                type_parameters: b,
            },
            Source::OwnedRecord {
                fields: c,
                type_parameters: d,
            },
        ) => budget.same(a, c)? && budget.same(b, d)?,
        (
            Interface::Variant {
                cases: a,
                type_parameters: b,
            },
            Source::Variant {
                cases: c,
                type_parameters: d,
            },
        ) => budget.same(a, c)? && budget.same(b, d)?,
        (Interface::Interface { operations: a }, Source::Interface { operations: b }) => {
            budget.same(a, b)?
        }
        (Interface::External(a), Source::External(b)) => {
            a.result == b.result
                && budget.same(&a.type_parameters, &b.type_parameters)?
                && budget.same(&a.parameters, &b.parameters)?
        }
        (Interface::Function(a), Source::Function(b)) => {
            a.result == b.result
                && budget.same(&a.read_scope_parameters, &b.read_scope_parameters)?
                && budget.same(&a.read_scope_sources, &b.read_scope_sources)?
                && budget.same(&a.type_parameters, &b.type_parameters)?
                && budget.same(&a.parameters, &b.parameters)?
                && budget.same(&a.requirement_parameters, &b.requirement_parameters)?
                && budget.same(&a.effect_parameters, &b.effect_parameters)?
                && same_parameters(
                    &a.implementation_parameters,
                    &b.implementation_parameters,
                    budget,
                )?
                && same_effect(&a.effect, &b.effect, budget)?
        }
        (Interface::ReadView(a), Source::ReadView(b)) => {
            budget.same(&a.type_parameters, &b.type_parameters)?
                && a.scope_parameter == b.scope_parameter
                && budget.same(&a.fields, &b.fields)?
        }
        (Interface::Constant { ty: a }, Source::Constant { ty: b, .. }) => a == b,
        (
            Interface::Component {
                requirements: a,
                ports: b,
            },
            Source::Component {
                requirements: c,
                ports: d,
            },
        ) => budget.same(a, c)? && budget.same(b, d)?,
        (Interface::OwnedContract(a), Source::OwnedContract(b)) => {
            if a.self_parameter != b.self_parameter
                || a.methods.len() != b.methods.len()
                || !budget.same(&a.type_parameters, &b.type_parameters)?
                || !budget.same(&a.associated_types, &b.associated_types)?
            {
                return Ok(false);
            }
            for (a, b) in a.methods.iter().zip(&b.methods) {
                budget.tick()?;
                if a.id != b.id
                    || a.name != b.name
                    || a.result != b.result
                    || !budget.same(&a.read_scope_parameters, &b.read_scope_parameters)?
                    || !budget.same(&a.read_scope_sources, &b.read_scope_sources)?
                    || !budget.same(&a.parameters, &b.parameters)?
                    || !same_effect(&a.effect, &b.effect, budget)?
                {
                    return Ok(false);
                }
            }
            true
        }
        (Interface::OwnedImplementation(a), Source::OwnedImplementation(b)) => {
            if a.contract != b.contract
                || a.self_type != b.self_type
                || a.methods.len() != b.methods.len()
                || !budget.same(&a.type_parameters, &b.type_parameters)?
                || !budget.same(&a.type_arguments, &b.type_arguments)?
                || !budget.same(&a.associated_types, &b.associated_types)?
                || !same_parameters(
                    &a.implementation_parameters,
                    &b.implementation_parameters,
                    budget,
                )?
            {
                return Ok(false);
            }
            for (a, b) in a.methods.iter().zip(&b.methods) {
                budget.tick()?;
                if a.method != b.method
                    || a.function != b.function
                    || !budget.same(&a.type_arguments, &b.type_arguments)?
                    || !budget.same(&a.read_scope_arguments, &b.read_scope_arguments)?
                    || !same_operands(&a.implementations, &b.implementations, 0, budget)?
                {
                    return Ok(false);
                }
            }
            true
        }
        _ => false,
    })
}

fn check_roots(
    record: &PackageInterfaceRecord,
    consumer: &KernelSnapshot,
    supplier: &KernelSnapshot,
    budget: &mut Budget<'_>,
) -> Result<(), ExecutionError> {
    let mut check = |ty: Option<TypeObjectDigest>| {
        budget.tick()?;
        let Some(ty) = ty else {
            return Ok(());
        };
        let a = consumer
            .types
            .get(&ty)
            .or_else(|| consumer.dependency_types.get(&ty));
        let b = supplier
            .types
            .get(&ty)
            .or_else(|| supplier.dependency_types.get(&ty));
        match (a, b) {
            (Some(a), Some(b)) if same_type(a, b, budget)? => Ok(()),
            _ => Err(inventory_error(
                "interface type root is absent from its exact supplier",
            )),
        }
    };
    use PackageInterfaceRecord as I;
    match record {
        I::Declaration(d) | I::RepresentationDeclaration(d) => match &d.payload {
            Interface::External(f) => check(Some(f.result))?,
            Interface::Function(f) => {
                check(Some(f.result.ty))?;
                for parameter in &f.implementation_parameters {
                    check(Some(parameter.self_type))?;
                    for ty in &parameter.type_arguments {
                        check(Some(*ty))?;
                    }
                }
            }
            Interface::Constant { ty } => check(Some(*ty))?,
            Interface::OwnedContract(c) => {
                for method in &c.methods {
                    for parameter in &method.parameters {
                        check(Some(parameter.ty))?;
                    }
                    check(Some(method.result.ty))?;
                }
            }
            Interface::OwnedImplementation(i) => {
                check(Some(i.self_type))?;
                for ty in &i.type_arguments {
                    check(Some(*ty))?;
                }
                for binding in &i.associated_types {
                    check(Some(binding.ty()))?;
                }
                for parameter in &i.implementation_parameters {
                    check(Some(parameter.self_type))?;
                    for ty in &parameter.type_arguments {
                        check(Some(*ty))?;
                    }
                }
                for method in &i.methods {
                    check(None)?;
                    for ty in &method.type_arguments {
                        check(Some(*ty))?;
                    }
                    check_operand_roots(&method.implementations, 0, &mut check)?;
                }
            }
            Interface::ReadView(view) => {
                for field in &view.fields {
                    check(Some(field.ty))?;
                }
            }
            Interface::Record { .. }
            | Interface::OwnedRecord { .. }
            | Interface::Variant { .. }
            | Interface::Interface { .. }
            | Interface::Component { .. } => {}
        },
        I::Field(f) => check(Some(f.ty))?,
        I::Case(c) => {
            if let Some(ty) = c.payload {
                check(Some(ty))?;
            }
        }
        I::Operation(o) => check(Some(o.result))?,
        I::Parameter(p) => check(Some(p.ty))?,
        I::Port(p) => check(Some(p.function_type))?,
        I::TypeParameter(_)
        | I::EffectParameter(_)
        | I::RequirementParameter(_)
        | I::Requirement(_) => {}
    }
    Ok(())
}

fn check_operand_roots(
    operands: &[ImplementationOperand],
    depth: usize,
    check: &mut impl FnMut(Option<TypeObjectDigest>) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    if operands.is_empty() {
        return Ok(());
    }
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(capacity());
    }
    for operand in operands {
        check(None)?;
        if let ImplementationOperand::Concrete {
            type_arguments,
            implementations,
            ..
        } = operand
        {
            for ty in type_arguments {
                check(Some(*ty))?;
            }
            check_operand_roots(implementations, depth + 1, check)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "reference_dependency_binding_tests.rs"]
mod tests;
