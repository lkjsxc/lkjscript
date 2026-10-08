//! Local projection fixtures, not stand-alone admitted programs.
use super::*;
use crate::platform::kernel::{
    DeclarationVisibility, FunctionDeclaration, FunctionEffect, ImplementationParameter, Name,
    OwnerHeader, OwnerKind,
};
use crate::platform::semantic_id::{DeclarationId, ModuleId};
use std::cell::Cell;

const SEED: &[u8] = b"demanded-lexical-scope-projection";

#[derive(Default)]
pub(super) struct Source {
    pub(super) owners: BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    pub(super) reads: Cell<usize>,
    pub(super) checkpoints: Cell<usize>,
    stop_at: Cell<Option<usize>>,
}

pub(super) fn owner(package: u64, declaration: u64) -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(SEED, package),
        declaration: DeclarationId::migrate(SEED, declaration),
    }
}

pub(super) fn ids(width: usize) -> Vec<ImplementationParameterId> {
    (0..width)
        .map(|i| ImplementationParameterId::migrate(SEED, i as u64 + 1))
        .collect()
}

impl Source {
    pub(super) fn reset(&self, stop_at: Option<usize>) {
        self.reads.set(0);
        self.checkpoints.set(0);
        self.stop_at.set(stop_at);
    }

    pub(super) fn add(&mut self, owner: DeclarationReference, ids: &[ImplementationParameterId]) {
        let record = DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(owner.declaration),
                OwnerKind::PureFunction,
            ),
            module: ModuleId::migrate(SEED, 1),
            name: Name::new("caller").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                implementation_parameters: ids
                    .iter()
                    .enumerate()
                    .map(|(index, id)| ImplementationParameter {
                        id: *id,
                        name: Name::new(format!("w{index}")).unwrap(),
                        contract: owner,
                        self_type: TypeObjectDigest::from_bytes([91; 32]),
                        type_arguments: Vec::new(),
                    })
                    .collect(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: TypeObjectDigest::from_bytes([92; 32]),
                result_borrow: None,
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(SEED, 1),
            }),
        };
        self.owners.insert(
            (owner.package, OwnerKey::Declaration(owner.declaration)),
            OwnerRecord::Declaration(record),
        );
    }
}

impl CallableClosureRead for Source {
    fn visit_callable_owners(
        &self,
        visitor: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        for (package, key) in self.owners.keys() {
            self.validation_checkpoint()?;
            if let OwnerKey::Declaration(declaration) = key {
                visitor(DeclarationReference {
                    package: *package,
                    declaration: *declaration,
                })?;
            }
        }
        Ok(())
    }
    fn owner(&self, package: PackageId, key: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.validation_checkpoint()?;
        self.reads.set(self.reads.get() + 1);
        Ok(self.owners.get(&(package, key)).cloned())
    }
    fn type_object(
        &self,
        _: PackageId,
        _: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        self.validation_checkpoint()?;
        Ok(None)
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        let next = self.checkpoints.get() + 1;
        self.checkpoints.set(next);
        if self.stop_at.get() == Some(next) {
            return Err(Diagnostic::new(
                DiagnosticClass::Cancelled,
                "scope_projection_cancelled",
                "controlled lexical projection cancellation",
            ));
        }
        Ok(())
    }
}

pub(super) fn analysis<'a>(read: &'a Source, work: &'a mut usize) -> Analysis<'a, Source> {
    Analysis {
        read,
        work,
        maximum_work: 10_000_000,
        observation: CallableFlowWork::default(),
        contexts: Vec::new(),
        indexes: BTreeMap::new(),
        selections: Vec::new(),
        selection_indexes: BTreeMap::new(),
        pending: Vec::new(),
        calls: Vec::new(),
        slot_owners: Vec::new(),
        edges: Vec::new(),
    }
}
