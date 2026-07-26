use crate::hir::Type;
use crate::semantic::schema::SemanticEffect;

pub(super) fn contains_parameter(ty: &Type) -> bool {
    match ty {
        Type::Param(_) | Type::Forall { .. } => true,
        Type::Owned(inner)
        | Type::Ref(inner)
        | Type::RefMut(inner)
        | Type::List(inner)
        | Type::Option(inner) => contains_parameter(inner),
        Type::Result(ok, error) => contains_parameter(ok) || contains_parameter(error),
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
