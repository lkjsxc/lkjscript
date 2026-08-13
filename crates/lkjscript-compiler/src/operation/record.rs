use crate::operation::*;
use lkjscript_contracts::{
    CapabilityKind, OperationOwnership, RuntimeLowering, SemanticConstructor,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedOperationRecord {
    pub(crate) operation: Operation,
    pub(crate) arity: usize,
    pub(crate) type_scheme: Type,
    pub(crate) effects: crate::hir::EffectSet,
    pub(crate) ownership: OperationOwnership,
    pub(crate) capability_requirements: Vec<CapabilityKind>,
    pub(crate) may_trap: bool,
    pub(crate) may_diverge: bool,
    pub(crate) runtime_lowering: RuntimeLowering,
}

impl Operation {
    /// Whether this operation can use the ordinary `ExprKind::Operation` -> runtime-op route.
    ///
    /// Control forms, numeric conversions, and enum constructors require dedicated lowering and
    /// therefore cannot be represented honestly by a flat direct-operation draft.
    pub(crate) fn supports_direct_operation_expression(self) -> bool {
        lkjscript_contracts::operation_semantics_by_id(self.identity()).is_some_and(|semantics| {
            semantics.runtime_lowering == RuntimeLowering::RuntimeCall
                && semantics.semantic_constructor == SemanticConstructor::BuiltinCall
                && semantics.legal_constructor_available
        })
    }

    pub(crate) fn record(self) -> TypedOperationRecord {
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
