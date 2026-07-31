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
    DeterministicUnique,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropGlueIdentity {
    ByteVector,
    Bytes,
    Resource(lkjscript_contracts::ResourceKind),
    Structural(StructuralDropGlueIdentity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropEventKind {
    ImplicitCleanup,
    ExplicitClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryDestruction {
    DropGlue(DropGlueIdentity),
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
    pub drop_glue: Option<DropGlueIdentity>,
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
                if let Some(mode) = owner_mode(place) {
                    obligations.push(SsaMemoryObligation {
                        function: function.id,
                        subject: MemoryObligationSubject::Owner {
                            place: place.id,
                            binding: place.binding,
                        },
                        ty: place.ty.clone(),
                        mode,
                        drop_glue: place.drop_glue,
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
                            drop_glue: None,
                        });
                    }
                }
            }
        }
        obligations.sort_unstable_by_key(sort_key);
        Self { obligations }
    }
}

include!("memory/modes.rs");

#[cfg(test)]
mod tests;
