use crate::operation::*;
use lkjscript_contracts::{CapabilityKind, OperationOwnership, RuntimeLowering};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedOperationRecord {
    pub operation: Operation,
    pub arity: usize,
    pub type_scheme: Type,
    pub effects: crate::hir::EffectSet,
    pub ownership: OperationOwnership,
    pub capability_requirements: Vec<CapabilityKind>,
    pub may_trap: bool,
    pub may_diverge: bool,
    pub runtime_lowering: RuntimeLowering,
}

impl Operation {
    pub fn record(self) -> TypedOperationRecord {
        let semantics = lkjscript_contracts::operation_semantics_by_id(self.identity());
        TypedOperationRecord {
            operation: self,
            arity: semantics.map_or(0, |record| usize::from(record.arity)),
            type_scheme: self.signature(),
            effects: self.effects(),
            ownership: semantics.map_or(OperationOwnership::Observes, |record| record.ownership),
            capability_requirements: semantics
                .map_or(&[][..], |record| record.capability_requirements)
                .to_vec(),
            may_trap: semantics.is_some_and(|record| record.may_trap),
            may_diverge: semantics.is_some_and(|record| record.may_diverge),
            runtime_lowering: semantics.map_or(RuntimeLowering::RuntimeCall, |record| {
                record.runtime_lowering
            }),
        }
    }
}
