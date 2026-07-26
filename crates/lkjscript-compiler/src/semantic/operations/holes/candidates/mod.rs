mod build;
mod capabilities;
mod control;
mod ranking;

use crate::semantic::schema::*;
use build::{binding_expression, candidate, literal_expressions};
use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetLedger, ResourceCategory};
use ranking::{builtin_category, rank_key};

use super::site::HoleSite;

pub(super) fn enumerate_with_ledger(
    site: &HoleSite<'_>,
    scope: &[ScopeEntity],
    ledger: &mut BudgetLedger,
) -> Result<(Vec<HoleCandidate>, ExplorationRecord, Vec<ActionBlocker>), ProtocolError> {
    let expected = match &site.expected {
        Ok(ty) => ty,
        Err(reason) => return Ok(unsupported(*reason)),
    };
    let scope_count = match u64::try_from(scope.len()) {
        Ok(value) => value,
        Err(_) => return Ok(bounded_failure("hole candidate count overflow".into())),
    };
    let builtin_count = match u64::try_from(crate::hir::Operation::ALL.len()) {
        Ok(value) => value,
        Err(_) => return Ok(bounded_failure("built-in candidate count overflow".into())),
    };
    let Some(maximum) = scope_count
        .checked_add(builtin_count)
        .and_then(|value| value.checked_add(16))
    else {
        return Ok(bounded_failure("hole candidate count overflow".into()));
    };
    let node_count = match u64::try_from(site.tree.nodes().len()) {
        Ok(value) => value,
        Err(_) => return Ok(bounded_failure("hole search work overflow".into())),
    };
    let Some(work) = node_count
        .checked_mul(4)
        .and_then(|value| value.checked_add(4))
        .and_then(|per_candidate| per_candidate.checked_mul(maximum))
        .and_then(|value| value.checked_add(1))
    else {
        return Ok(bounded_failure("hole search work overflow".into()));
    };
    let mut request = ledger.scope(BudgetAuthority::SemanticRequest);
    let mut holes = request
        .child(BudgetAuthority::Holes)
        .map_err(crate::semantic::codec::budget_error)?;
    let mut work_reservation = holes
        .reserve(
            ResourceCategory::HoleSearchWork,
            work,
            BudgetCause::SemanticNode(u64::from(site.node)),
        )
        .map_err(crate::semantic::codec::budget_error)?;
    work_reservation
        .consume(work)
        .map_err(crate::semantic::codec::budget_error)?;
    work_reservation.return_unused();
    let mut reservation = holes
        .reserve(
            ResourceCategory::HoleCandidates,
            maximum,
            BudgetCause::SemanticNode(u64::from(site.node)),
        )
        .map_err(crate::semantic::codec::budget_error)?;
    let mut expressions = literal_expressions(site.tree, expected);
    expressions.extend(control::expressions(site, expected));
    for entity in scope {
        if let Some(expression) = binding_expression(entity, expected) {
            expressions.push((CandidateCategory::VisibleBinding, expression));
        }
    }
    let call_argument = |ty: &crate::hir::Type| {
        scope
            .iter()
            .find_map(|entity| binding_expression(entity, ty))
            .or_else(|| super::validate::witness(site.tree, ty, 0))
    };
    for operation in crate::hir::Operation::ALL {
        let crate::hir::Type::Fn { params, ret } = operation.signature() else {
            continue;
        };
        if ret.as_ref() != expected {
            continue;
        }
        let Some(arguments) = params
            .iter()
            .map(&call_argument)
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        expressions.push((
            builtin_category(*operation),
            Expression::BuiltinCall {
                operation: ClosedBuiltinOperation(*operation),
                arguments,
            },
        ));
    }
    for (name, params, result) in super::scope::function_signatures(site.tree) {
        if result != *expected {
            continue;
        }
        let Some(arguments) = params
            .iter()
            .map(&call_argument)
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        expressions.push((
            CandidateCategory::DirectFunction,
            Expression::UserCall { name, arguments },
        ));
    }
    let mut candidates = Vec::with_capacity(expressions.len().min(maximum as usize));
    let mut rejected = std::collections::BTreeMap::new();
    for (category, expression) in expressions {
        if !super::validate::checker_accepts(site, &expression) {
            let count = rejected.entry(category).or_insert(0_u64);
            *count = count.saturating_add(1);
            continue;
        }
        if reservation.consume(1).is_err() {
            break;
        }
        if let Some(candidate) = candidate(site, expected, category, expression) {
            candidates.push(candidate);
        }
    }
    reservation.return_unused();
    candidates.sort_by(|a, b| rank_key(a).cmp(&rank_key(b)));
    let omitted = omitted_categories(rejected);
    let exploration = ExplorationRecord {
        supported: true,
        truncated: false,
        charged_category: "hole_candidates".into(),
        charged_count: candidates.len() as u64,
        search_work: work,
        omitted,
        reason: None,
    };
    let blockers = unsupported_blockers();
    Ok((candidates, exploration, blockers))
}

fn unsupported(
    reason: TypeUnavailableReason,
) -> (Vec<HoleCandidate>, ExplorationRecord, Vec<ActionBlocker>) {
    super::candidate_support::unsupported(reason)
}

fn bounded_failure(message: String) -> (Vec<HoleCandidate>, ExplorationRecord, Vec<ActionBlocker>) {
    super::candidate_support::bounded_failure(message)
}

fn omitted_categories(
    rejected: std::collections::BTreeMap<CandidateCategory, u64>,
) -> Vec<OmittedCategory> {
    super::candidate_support::omitted_categories(rejected)
}

fn unsupported_blockers() -> Vec<ActionBlocker> {
    super::candidate_support::unsupported_blockers()
}
