use crate::hir::Type;
use crate::semantic::schema::SemanticEffect;

pub(super) fn contains_parameter(ty: &Type) -> bool {
    match ty {
        Type::Param(_) | Type::Forall { .. } => true,
        Type::List(inner) => contains_parameter(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(contains_parameter),
        Type::Fn { params, ret } => {
            params.iter().any(contains_parameter) || contains_parameter(ret)
        }
        _ => false,
    }
}

pub(super) fn all_effects() -> Vec<SemanticEffect> {
    vec![
        SemanticEffect::Allocates,
        SemanticEffect::ReadsMemory,
        SemanticEffect::WritesMemory,
        SemanticEffect::MutatesLocal,
        SemanticEffect::HostIo,
        SemanticEffect::MayTrap,
        SemanticEffect::MayExit,
        SemanticEffect::MayDiverge,
    ]
}
