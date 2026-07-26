use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetLedger, ResourceCategory};

use crate::semantic::schema::*;
use crate::source::ValidatedSourceTree;

pub(crate) fn build(
    tree: &ValidatedSourceTree,
    node: u32,
    profile: ResourceProfile,
) -> Result<LegalActionsResult, ProtocolError> {
    let context = super::context::build(tree, node, profile)?;
    let site = super::site::find(tree, node)?;
    let maximum = u64::try_from(context.candidates.len())
        .unwrap_or(u64::MAX)
        .saturating_mul(2)
        .saturating_add(32);
    let mut ledger = BudgetLedger::new(profile.core());
    let mut request = ledger.scope(BudgetAuthority::SemanticRequest);
    let mut holes = request
        .child(BudgetAuthority::Holes)
        .map_err(budget_error)?;
    let mut reservation = holes
        .reserve(
            ResourceCategory::LegalActions,
            maximum,
            BudgetCause::SemanticNode(u64::from(node)),
        )
        .map_err(budget_error)?;
    let child_kinds = child_kinds(&context.candidates);
    let constructors = constructors(&context.candidates);
    let required_fields = required_fields(&site);
    let applicable_bindings = context
        .candidates
        .iter()
        .filter(|candidate| candidate.category == CandidateCategory::VisibleBinding)
        .map(|candidate| candidate.identity.clone())
        .collect::<Vec<_>>();
    let mut transactions = vec![
        HoleTransactionKind::FillHole,
        HoleTransactionKind::RefineHole,
    ];
    if super::site::deletion_legal(&site) {
        transactions.push(HoleTransactionKind::DeleteHole);
    }
    let charged = child_kinds
        .len()
        .saturating_add(constructors.len())
        .saturating_add(required_fields.len())
        .saturating_add(applicable_bindings.len())
        .saturating_add(transactions.len());
    let charged = u64::try_from(charged).unwrap_or(u64::MAX);
    reservation.consume(charged).map_err(budget_error)?;
    reservation.return_unused();
    let mut coverage = context.exploration;
    coverage.charged_category = "legal_actions".into();
    coverage.charged_count = charged;
    Ok(LegalActionsResult {
        hole: context.hole,
        legal_child_kinds: child_kinds,
        constructors,
        required_fields,
        expected_type: context.expected_type,
        applicable_bindings,
        transaction_kinds: transactions,
        coverage,
        blockers: context.blockers,
    })
}

fn child_kinds(candidates: &[HoleCandidate]) -> Vec<LegalChildKind> {
    let mut output = Vec::new();
    for candidate in candidates {
        let kind = match candidate.category {
            CandidateCategory::ExactLiteral => LegalChildKind::Literal,
            CandidateCategory::VisibleBinding => LegalChildKind::NameReference,
            CandidateCategory::ProductConstructor => LegalChildKind::ProductValue,
            CandidateCategory::OptionConstructor => LegalChildKind::OptionConstructor,
            CandidateCategory::ResultConstructor => LegalChildKind::ResultConstructor,
            CandidateCategory::DirectFunction => LegalChildKind::UserCall,
            CandidateCategory::DirectBuiltin | CandidateCategory::ExactConversion => {
                LegalChildKind::BuiltinCall
            }
            CandidateCategory::MatchSkeleton
            | CandidateCategory::ControlForm
            | CandidateCategory::NeverForm => continue,
        };
        if !output.contains(&kind) {
            output.push(kind);
        }
    }
    output.push(LegalChildKind::TypedHole);
    output
}

fn constructors(candidates: &[HoleCandidate]) -> Vec<ConstructorAction> {
    candidates
        .iter()
        .filter_map(|candidate| {
            let kind = match candidate.category {
                CandidateCategory::ProductConstructor => LegalChildKind::ProductValue,
                CandidateCategory::OptionConstructor => LegalChildKind::OptionConstructor,
                CandidateCategory::ResultConstructor => LegalChildKind::ResultConstructor,
                _ => return None,
            };
            Some(ConstructorAction {
                kind,
                name: candidate.identity.clone(),
                result_type: candidate.result_type.clone(),
            })
        })
        .collect()
}

fn required_fields(site: &super::site::HoleSite<'_>) -> Vec<RequiredField> {
    let Ok(crate::hir::Type::Product(expected)) = &site.expected else {
        return Vec::new();
    };
    let nodes = crate::semantic::tree::source_nodes(site.tree);
    let Some(declaration) = site.tree.declarations().iter().find(|item| {
        item.kind() == crate::source::DeclarationKind::Product && item.name() == expected
    }) else {
        return Vec::new();
    };
    let Some(root) = nodes.get(declaration.node().index() as usize) else {
        return Vec::new();
    };
    let Some(fields) = root
        .children
        .iter()
        .find(|child| super::types::call_is(child, "fields"))
    else {
        return Vec::new();
    };
    fields
        .children
        .iter()
        .filter_map(|field| {
            let name = field.children.first().and_then(super::types::source_name)?;
            let ty = field.children.get(1).and_then(super::types::type_form)?;
            Some(RequiredField {
                name: name.into(),
                expected_type: super::types::canonical(&ty),
            })
        })
        .collect()
}

fn budget_error(failure: lkjscript_core::BudgetError) -> ProtocolError {
    crate::semantic::codec::error(ProtocolErrorCode::ResourceLimit, failure.to_string())
}
