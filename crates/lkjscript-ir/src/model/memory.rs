use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMultiplicity {
    Copy,
    Affine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAliasing {
    Unique,
    BorrowedShared,
    BorrowedExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLocality {
    LocalOrEscaping,
    BorrowLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStorage {
    TransitionalTracedBuffer,
    BorrowedView,
    ExternalSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPortability {
    WorkerLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryContention {
    SingleOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryDestruction {
    CompilerFactOnly,
    EndBorrow,
    ExplicitExternalClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryIdentity {
    Value,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMode {
    pub multiplicity: MemoryMultiplicity,
    pub aliasing: MemoryAliasing,
    pub locality: MemoryLocality,
    pub storage: MemoryStorage,
    pub portability: MemoryPortability,
    pub contention: MemoryContention,
    pub destruction: MemoryDestruction,
    pub identity: MemoryIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryObligationSubject {
    Owner {
        place: PlaceId,
        binding: BindingId,
    },
    Loan {
        place: PlaceId,
        loan: LoanId,
        value: ValueId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsaMemoryObligation {
    pub function: FunctionId,
    pub subject: MemoryObligationSubject,
    pub ty: SsaType,
    pub mode: MemoryMode,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SsaMemoryInventory {
    pub obligations: Vec<SsaMemoryObligation>,
}

impl SsaMemoryInventory {
    pub(crate) fn from_program(program: &Program) -> Self {
        let mut obligations = Vec::new();
        for function in &program.functions {
            for place in &function.places {
                if let Some(mode) = owner_mode(&place.ty) {
                    obligations.push(SsaMemoryObligation {
                        function: function.id,
                        subject: MemoryObligationSubject::Owner {
                            place: place.id,
                            binding: place.binding,
                        },
                        ty: place.ty.clone(),
                        mode,
                    });
                }
            }
            for block in &function.blocks {
                for instruction in &block.instructions {
                    if let InstructionKind::Borrow {
                        place, loan, kind, ..
                    } = &instruction.kind
                    {
                        obligations.push(SsaMemoryObligation {
                            function: function.id,
                            subject: MemoryObligationSubject::Loan {
                                place: *place,
                                loan: *loan,
                                value: instruction.id,
                            },
                            ty: instruction.ty.clone(),
                            mode: borrow_mode(*kind),
                        });
                    }
                }
            }
        }
        obligations.sort_unstable_by_key(sort_key);
        Self { obligations }
    }
}

fn owner_mode(ty: &SsaType) -> Option<MemoryMode> {
    let (storage, destruction, identity) = match ty {
        SsaType::Owned(inner) if **inner == SsaType::Buf => (
            MemoryStorage::TransitionalTracedBuffer,
            MemoryDestruction::CompilerFactOnly,
            MemoryIdentity::Value,
        ),
        SsaType::Resource(_) => (
            MemoryStorage::ExternalSlot,
            MemoryDestruction::ExplicitExternalClose,
            MemoryIdentity::External,
        ),
        _ => return None,
    };
    Some(MemoryMode {
        multiplicity: MemoryMultiplicity::Affine,
        aliasing: MemoryAliasing::Unique,
        locality: MemoryLocality::LocalOrEscaping,
        storage,
        portability: MemoryPortability::WorkerLocal,
        contention: MemoryContention::SingleOwner,
        destruction,
        identity,
    })
}

fn borrow_mode(kind: BorrowKind) -> MemoryMode {
    MemoryMode {
        multiplicity: match kind {
            BorrowKind::Shared => MemoryMultiplicity::Copy,
            BorrowKind::Mutable => MemoryMultiplicity::Affine,
        },
        aliasing: match kind {
            BorrowKind::Shared => MemoryAliasing::BorrowedShared,
            BorrowKind::Mutable => MemoryAliasing::BorrowedExclusive,
        },
        locality: MemoryLocality::BorrowLocal,
        storage: MemoryStorage::BorrowedView,
        portability: MemoryPortability::WorkerLocal,
        contention: MemoryContention::SingleOwner,
        destruction: MemoryDestruction::EndBorrow,
        identity: MemoryIdentity::Value,
    }
}

fn sort_key(obligation: &SsaMemoryObligation) -> (u32, u8, u32, u32, u32) {
    match obligation.subject {
        MemoryObligationSubject::Owner { place, binding } => {
            (obligation.function.raw(), 0, place.raw(), binding.raw(), 0)
        }
        MemoryObligationSubject::Loan { place, loan, value } => (
            obligation.function.raw(),
            1,
            place.raw(),
            loan.raw(),
            value.raw(),
        ),
    }
}

#[cfg(test)]
mod tests;
