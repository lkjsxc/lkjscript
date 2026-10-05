//! Read-only composed callable admission over a candidate and its exact admitted suppliers.
//! Private bodies are analysis inputs; this reader grants no semantic lookup authority.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::callable_flow::{CallableClosureRead, validate_callable_closure};
use crate::platform::kernel::{
    DeclarationReference, EncodedOwnerKey, KernelSnapshot, OwnerKey, OwnerRecord, PackageId,
    SemanticRoot, TypeObject, TypeObjectDigest, decode_owner, decode_owner_binding,
    decode_type_object,
};
use crate::platform::package_transport::source::{
    CollectingStore, MAXIMUM_VALIDATION_VISITS, entries, required,
};
use crate::platform::storage::object::{ImmutableObjectStore, ObjectDomain, StoreWork};
use std::collections::{BTreeMap, BTreeSet};

struct Candidate<'a> {
    package: PackageId,
    owners: BTreeMap<OwnerKey, OwnerRecord>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    suppliers: &'a BTreeMap<PackageId, KernelSnapshot>,
    checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
}

impl CallableClosureRead for Candidate<'_> {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for (package, owners) in std::iter::once((self.package, &self.owners)).chain(
            self.suppliers
                .iter()
                .map(|(package, snapshot)| (*package, &snapshot.owners)),
        ) {
            for owner in owners.keys() {
                self.validation_checkpoint()?;
                if let OwnerKey::Declaration(declaration) = owner {
                    visitor(DeclarationReference {
                        package,
                        declaration: *declaration,
                    })?;
                }
            }
        }
        Ok(())
    }

    fn owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.validation_checkpoint()?;
        Ok(if package == self.package {
            self.owners.get(&owner)
        } else {
            self.suppliers
                .get(&package)
                .and_then(|s| s.owners.get(&owner))
        }
        .cloned())
    }

    fn type_object(
        &self,
        package: PackageId,
        ty: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        self.validation_checkpoint()?;
        Ok(if package == self.package {
            self.types.get(&ty)
        } else {
            self.suppliers
                .get(&package)
                .and_then(|s| s.types.get(&ty).or_else(|| s.dependency_types.get(&ty)))
        }
        .cloned())
    }

    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        (self.checkpoint)()
    }
}

fn candidate_object_error(mut diagnostic: Diagnostic, domain: ObjectDomain) -> Diagnostic {
    if diagnostic.class == DiagnosticClass::Corrupt && diagnostic.code == "package_source_missing" {
        diagnostic.code = "publication_repository_object_missing".into();
        diagnostic.message = format!(
            "candidate references a missing {} object; restore the complete prepared publication before retrying",
            domain.name()
        );
    }
    diagnostic
}

/// Returns complete read/semantic work, including earlier supplier admission. The caller owns
/// the store and publication lock; no repository is opened or locked from this analysis.
pub(super) fn validate<S: ImmutableObjectStore + ?Sized>(
    store: &CollectingStore<'_, S>,
    root: &SemanticRoot,
    suppliers: &BTreeMap<PackageId, KernelSnapshot>,
    prior_semantic_work: u64,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<u64, Diagnostic> {
    let exhausted = || {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "package_source_budget",
            "candidate composed callable admission exhausted aggregate validation visits",
        )
    };
    checkpoint()?;
    let mut work = StoreWork::default();
    let mut owners = BTreeMap::new();
    for (key, value) in entries(store, root.owners)? {
        checkpoint()?;
        let owner = EncodedOwnerKey::decode(&key)?;
        let binding = decode_owner_binding(&value, owner)?;
        let bytes = required(
            store,
            ObjectDomain::Owner,
            binding.object.bytes(),
            &mut work,
        )
        .map_err(|diagnostic| candidate_object_error(diagnostic, ObjectDomain::Owner))?;
        owners.insert(
            owner,
            decode_owner(&bytes, owner, binding.kind, binding.object)?,
        );
    }
    let mut pending = owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .collect::<BTreeSet<_>>();
    let mut types = BTreeMap::new();
    while let Some(ty) = pending.pop_first() {
        checkpoint()?;
        if types.contains_key(&ty) {
            continue;
        }
        let bytes = required(store, ObjectDomain::Type, ty.bytes(), &mut work)
            .map_err(|diagnostic| candidate_object_error(diagnostic, ObjectDomain::Type))?;
        let object = decode_type_object(&bytes, ty)?;
        pending.extend(object.child_types());
        types.insert(ty, object);
    }
    let observed = prior_semantic_work
        .checked_add(store.visits())
        .filter(|total| *total <= MAXIMUM_VALIDATION_VISITS)
        .ok_or_else(exhausted)?;
    let mut observed = usize::try_from(observed).map_err(|_| exhausted())?;
    validate_callable_closure(
        &Candidate {
            package: root.package_id,
            owners,
            types,
            suppliers,
            checkpoint,
        },
        &mut observed,
        MAXIMUM_VALIDATION_VISITS as usize,
    )?;
    Ok(observed as u64)
}
