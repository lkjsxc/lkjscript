//! Categorical signature checks over current-admitted, immutable artifact inputs.
//!
//! LoadedArtifact's current seal and exact canonical/compiled callable/parameter
//! normalization precede this helper. Those proofs retain complete type grammar,
//! constraints and authored dependency/interface authority. This helper binds the
//! exact lexical occurrence and Owned member inventory; it neither synthesizes
//! import rights nor selects concrete outputs or grants runtime properties.
use super::{
    CompilationPayload, CompilationUnit, DeclarationPayload, DeclarationReference, FunctionEffect,
    Name, NormalizedParameter, OwnerKey, OwnerRecord, PackageId, ParameterId, ParameterParent,
    RuntimeOwnerMap, TypeForm, TypeObject, TypeObjectDigest, runtime_corrupt,
};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::{AssociatedTypeBound, OwnerKind};
use crate::platform::semantic_id::{AssociatedTypeMemberId, ImplementationParameterId};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Context<'a> {
    pub(super) declaration: DeclarationReference,
    pub(super) parameters: &'a [NormalizedParameter],
    pub(super) result: TypeObjectDigest,
    pub(super) units: &'a BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    pub(super) owners: &'a RuntimeOwnerMap,
    pub(super) types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
}

fn invalid() -> Diagnostic {
    runtime_corrupt(
        "normalized_associated_signature",
        "associated signature type lacks its exact admitted lexical Owned member",
    )
}

impl Context<'_> {
    pub(super) fn owned(
        &self,
        ty: TypeObjectDigest,
        parameter: Option<ParameterId>,
        work: &mut super::super::prepared_types::Budget<'_>,
    ) -> Result<bool, Diagnostic> {
        work.step()?;
        let TypeForm::AssociatedWitnessProjection {
            scope,
            parameter: witness,
            contract,
            member,
        } = &self.types.get(&ty).ok_or_else(invalid)?.form
        else {
            // Contract-local members never become function/external projections.
            return Err(invalid());
        };
        if *scope != self.declaration {
            return Err(invalid());
        }
        let key = OwnerKey::Declaration(self.declaration.declaration);
        let Some(OwnerRecord::Declaration(source)) =
            self.owners.get(&(self.declaration.package, key))
        else {
            return Err(invalid());
        };
        let DeclarationPayload::Function(function) = &source.payload else {
            return Err(invalid());
        };
        let kind = match function.effect {
            FunctionEffect::Pure => OwnerKind::PureFunction,
            FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
        };
        if source.header.owner != key
            || source.header.kind != kind
            || source.header.contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || function.parameters.len() != self.parameters.len()
        {
            return Err(invalid());
        }
        let mut occurrences = 0;
        for (id, normalized) in function.parameters.iter().zip(self.parameters) {
            work.step()?;
            if *id != normalized.parameter {
                return Err(invalid());
            }
            occurrences += usize::from(Some(*id) == parameter);
        }
        if let Some(parameter) = parameter {
            if occurrences != 1 {
                return Err(invalid());
            }
            work.step()?;
            let key = OwnerKey::Parameter(parameter);
            let Some(OwnerRecord::Parameter(source)) =
                self.owners.get(&(self.declaration.package, key))
            else {
                return Err(invalid());
            };
            if source.header.owner != key
                || source.header.kind != OwnerKind::Parameter
                || source.header.contract_version
                    != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                || source.parent != ParameterParent::Function(self.declaration.declaration)
                || source.ty != ty
            {
                return Err(invalid());
            }
        } else if self.result != ty || function.result != ty {
            return Err(invalid());
        }
        work.reserve::<(ImplementationParameterId, usize, usize, usize)>(
            function.implementation_parameters.len(),
        )?;
        work.reserve::<(&Name, usize, usize, usize)>(function.implementation_parameters.len())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut selected = None;
        for formal in &function.implementation_parameters {
            work.step()?;
            if !ids.insert(formal.id) || !names.insert(&formal.name) {
                return Err(invalid());
            }
            if formal.id == *witness {
                selected = Some(formal);
            }
        }
        let selected = selected.ok_or_else(invalid)?;
        if selected.contract != *contract || !self.types.contains_key(&selected.self_type) {
            return Err(invalid());
        }
        // The complete formal's legal Self and generic actuals were already
        // source-admitted and matched to the compiled signature. Do not impose
        // a new Self grammar here (e.g. requiring a bare type parameter).
        work.step()?;
        let key = OwnerKey::Declaration(contract.declaration);
        let Some(OwnerRecord::Declaration(source)) = self.owners.get(&(contract.package, key))
        else {
            return Err(invalid());
        };
        let DeclarationPayload::OwnedContract(definition) = &source.payload else {
            return Err(invalid());
        };
        if source.header.owner != key
            || source.header.kind != OwnerKind::OwnedContract
            || source.header.contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || selected.type_arguments.len() != definition.type_parameters.len()
        {
            return Err(invalid());
        }
        let unit = self
            .units
            .get(&(contract.package, key))
            .ok_or_else(invalid)?;
        let CompilationPayload::OwnedContract(compiled) = &unit.payload else {
            return Err(invalid());
        };
        if definition.associated_types.len() != compiled.associated_types.len() {
            return Err(invalid());
        }
        work.reserve::<(AssociatedTypeMemberId, usize, usize, usize)>(
            definition.associated_types.len(),
        )?;
        work.reserve::<(&Name, usize, usize, usize)>(definition.associated_types.len())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut owned = false;
        for (actual, compiled) in definition
            .associated_types
            .iter()
            .zip(&compiled.associated_types)
        {
            work.step()?;
            if actual != compiled || !ids.insert(actual.id) || !names.insert(&actual.name) {
                return Err(invalid());
            }
            if actual.id == *member {
                owned = match actual.bound {
                    AssociatedTypeBound::Owned => true,
                };
            }
        }
        if !owned {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
#[path = "prepared_signature_owned_tests.rs"]
mod tests;
