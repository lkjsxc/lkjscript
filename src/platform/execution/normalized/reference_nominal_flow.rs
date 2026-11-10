//! Finite nominal templates shared by raw canonical reference entry points.
//! This proof borrows source records and precedes concrete type substitution.

use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, DeclarationReference, KernelSnapshot, OwnerKey,
    OwnerRecord, PackageId, TypeForm, TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, ExecutionError>;
type Slot = (DeclarationReference, TypeParameterId);
type Flow = BTreeMap<Slot, BTreeMap<Slot, bool>>;
type Schemas = BTreeMap<DeclarationReference, (bool, BTreeSet<DeclarationReference>)>;

fn invalid(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_nominal_flow",
        message,
    )
}

struct Read<'a> {
    sources: BTreeMap<PackageId, &'a KernelSnapshot>,
    types: BTreeMap<TypeObjectDigest, &'a TypeObject>,
    control: &'a ExecutionControl,
}

impl Read<'_> {
    fn owner(&self, package: PackageId, key: OwnerKey) -> Option<&OwnerRecord> {
        self.sources.get(&package)?.owners.get(&key)
    }

    fn declaration(&self, reference: DeclarationReference) -> Option<&DeclarationRecord> {
        match self.owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )? {
            OwnerRecord::Declaration(record) => Some(record),
            _ => None,
        }
    }

    fn object(&self, ty: TypeObjectDigest) -> Option<&TypeObject> {
        self.types.get(&ty).copied()
    }

    fn arguments(
        &self,
        source: DeclarationReference,
        target: DeclarationReference,
        arguments: &[TypeObjectDigest],
        flow: &mut Flow,
    ) -> Result<()> {
        // Missing owners and malformed arity remain obligations of the caller's
        // existing inventory reader; neither can be expanded by that reader.
        let Some(record) = self.declaration(target) else {
            return Ok(());
        };
        let parameters = record.payload.type_parameters();
        if parameters.len() != arguments.len() {
            return Ok(());
        }
        for (parameter, argument) in parameters.iter().zip(arguments) {
            let mut pending = vec![(*argument, false)];
            let mut visited = BTreeSet::new();
            while let Some((ty, growing)) = pending.pop() {
                self.control.check()?;
                if !visited.insert((ty, growing)) {
                    continue;
                }
                let Some(object) = self.object(ty) else {
                    continue;
                };
                if let TypeForm::TypeParameter { parameter: formal } = object.form {
                    flow.entry((source, formal))
                        .or_default()
                        .entry((target, *parameter))
                        .and_modify(|previous| *previous |= growing)
                        .or_insert(growing);
                } else {
                    pending.extend(object.child_types().into_iter().map(|child| (child, true)));
                }
            }
        }
        Ok(())
    }
}

/// Field templates, rather than concrete root applications, determine nominal
/// cycles. Finite nested applications such as Box<Box<Cell>> add no schema edge.
pub(super) fn check(snapshots: &[&KernelSnapshot], control: &ExecutionControl) -> Result<()> {
    let mut read = Read {
        sources: BTreeMap::new(),
        types: BTreeMap::new(),
        control,
    };
    for source in snapshots {
        control.check()?;
        read.sources.insert(source.root.package_id, *source);
        for (ty, object) in source.types.iter().chain(&source.dependency_types) {
            control.check()?;
            if read
                .types
                .insert(*ty, object)
                .is_some_and(|previous| previous != object)
            {
                return Err(invalid("nominal type identity has conflicting objects"));
            }
        }
    }
    let mut schemas = Schemas::new();
    let mut flow = Flow::new();
    for source in snapshots {
        for (key, owner) in &source.owners {
            control.check()?;
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(record)) = (key, owner) else {
                continue;
            };
            let declaration = DeclarationReference {
                package: source.root.package_id,
                declaration: *id,
            };
            let mut roots = Vec::new();
            let owned = match &record.payload {
                DeclarationPayload::Record { fields, .. }
                | DeclarationPayload::OwnedRecord { fields, .. } => {
                    for field in fields {
                        if let Some(OwnerRecord::Field(field)) =
                            read.owner(declaration.package, OwnerKey::Field(*field))
                        {
                            roots.push(field.ty);
                        }
                    }
                    matches!(record.payload, DeclarationPayload::OwnedRecord { .. })
                }
                DeclarationPayload::Variant { cases, .. } => {
                    for case in cases {
                        if let Some(OwnerRecord::Case(case)) =
                            read.owner(declaration.package, OwnerKey::Case(*case))
                        {
                            roots.extend(case.payload);
                        }
                    }
                    false
                }
                DeclarationPayload::ReadView(view) => {
                    roots.extend(view.fields.iter().map(|field| field.ty));
                    false
                }
                _ => continue,
            };
            let mut edges = BTreeSet::new();
            let mut visited = BTreeSet::new();
            while let Some(ty) = roots.pop() {
                control.check()?;
                if !visited.insert(ty) {
                    continue;
                }
                let Some(object) = read.object(ty) else {
                    continue;
                };
                match &object.form {
                    TypeForm::Named {
                        declaration: target,
                    } => {
                        edges.insert(*target);
                    }
                    TypeForm::Applied {
                        declaration: target,
                        arguments,
                    }
                    | TypeForm::ReadViewApplied {
                        declaration: target,
                        arguments,
                        ..
                    } => {
                        edges.insert(*target);
                        read.arguments(declaration, *target, arguments, &mut flow)?;
                    }
                    _ => {}
                }
                roots.extend(object.child_types());
            }
            schemas.insert(declaration, (owned, edges));
        }
    }
    for (root, (owned, edges)) in &schemas {
        if !owned {
            continue;
        }
        let mut pending: Vec<_> = edges.iter().copied().collect();
        let mut visited = BTreeSet::new();
        while let Some(next) = pending.pop() {
            control.check()?;
            if next == *root {
                return Err(invalid("cyclic raw Owned declaration schema"));
            }
            if visited.insert(next)
                && let Some((_, children)) = schemas.get(&next)
            {
                pending.extend(children.iter().copied());
            }
        }
    }
    // A constructor-growing edge is unbounded exactly when its destination can
    // reach its source. Plain permutations and closed resets remain finite.
    for (source, outgoing) in &flow {
        for (target, growing) in outgoing {
            control.check()?;
            if !growing {
                continue;
            }
            let mut pending = vec![*target];
            let mut visited = BTreeSet::new();
            while let Some(slot) = pending.pop() {
                control.check()?;
                if slot == *source {
                    return Err(invalid(format!(
                        "expanding raw nominal parameter cycle: {}::{} {} -> {}::{} {}",
                        source.0.package,
                        source.0.declaration,
                        source.1,
                        target.0.package,
                        target.0.declaration,
                        target.1,
                    )));
                }
                if visited.insert(slot)
                    && let Some(children) = flow.get(&slot)
                {
                    pending.extend(children.keys().copied());
                }
            }
        }
    }
    Ok(())
}
