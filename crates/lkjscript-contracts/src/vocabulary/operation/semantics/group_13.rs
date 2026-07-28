use super::*;

pub(super) const RECORDS: &[OperationSemanticsRecord] = &[
    record(
        123,
        1,
        "fn inputs bytes output i64",
        OperationEffects(2),
        OperationOwnership::Observes,
        false,
    ),
    record(
        124,
        2,
        "fn inputs bytes i64 output i64",
        OperationEffects(34),
        OperationOwnership::Observes,
        true,
    ),
    record(
        125,
        3,
        "fn inputs bytes i64 i64 output bytes",
        OperationEffects(35),
        OperationOwnership::Allocates,
        true,
    ),
    record(
        126,
        1,
        "fn inputs bytes output bytes",
        OperationEffects(35),
        OperationOwnership::Allocates,
        true,
    ),
    record(
        127,
        1,
        "fn inputs byte-vector output bytes",
        OperationEffects(38),
        OperationOwnership::ConsumesOwner,
        true,
    ),
    record(
        128,
        1,
        "fn inputs bytes output byte-vector",
        OperationEffects(39),
        OperationOwnership::ConsumesOwner,
        true,
    ),
    record(
        129,
        2,
        "fn inputs byte-slice i64 output i64",
        OperationEffects(34),
        OperationOwnership::Observes,
        true,
    ),
    record(
        130,
        3,
        "fn inputs byte-slice-mut i64 i64 output unit",
        OperationEffects(36),
        OperationOwnership::Mutates,
        true,
    ),
];

const fn record(
    identity: u16,
    arity: u8,
    type_scheme: &'static str,
    effects: OperationEffects,
    ownership: OperationOwnership,
    may_trap: bool,
) -> OperationSemanticsRecord {
    OperationSemanticsRecord {
        identity: OperationIdentity::new(identity),
        arity,
        type_scheme,
        generic_variables: &[],
        generic_constraints: &[],
        effects,
        capability_requirements: &[],
        ownership,
        may_trap,
        may_diverge: false,
        runtime_lowering: RuntimeLowering::RuntimeCall,
        semantic_source: SemanticSourceRelationship::BuiltinCall,
        legal_action_available: true,
    }
}
