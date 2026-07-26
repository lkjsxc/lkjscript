use crate::semantic::schema::{CandidateCategory, HoleCandidate};

pub(super) fn builtin_category(operation: crate::hir::Operation) -> CandidateCategory {
    if matches!(
        operation,
        crate::hir::Operation::StrFromByte
            | crate::hir::Operation::StrFromI64
            | crate::hir::Operation::StrFromF64
            | crate::hir::Operation::BufFromStr
            | crate::hir::Operation::BufToStr
    ) {
        CandidateCategory::ExactConversion
    } else {
        CandidateCategory::DirectBuiltin
    }
}

pub(super) fn rank_key(candidate: &HoleCandidate) -> (u16, u16, u16, u32, &str, &str) {
    let rank = &candidate.rank;
    (
        rank.category,
        rank.effect_cost,
        rank.ownership_cost,
        rank.construction_cost,
        &rank.canonical_source,
        &rank.identity,
    )
}
