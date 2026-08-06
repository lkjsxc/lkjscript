use crate::hir::{EffectSet, Type};
use crate::semantic::schema::*;
use crate::source::SourceNode;

use super::super::site::HoleSite;

// Hole discovery validates this revision-scoped node before constructing `HoleSite`.
#[allow(clippy::expect_used)]
pub(super) fn identity(site: &HoleSite<'_>) -> TypedHoleIdentity {
    TypedHoleIdentity {
        schema: "lkjscript.typed-hole".into(),
        contract: crate::semantic::CONTRACT.to_hex(),
        source_revision: site.tree.revision().to_hex(),
        identity: format!("{}:{}", site.declaration_key, site.local_identity),
        declaration_key: site.declaration_key.clone(),
        local_identity: site.local_identity.clone(),
        node: site.node,
        node_fingerprint: crate::semantic::tree::fingerprint(site.source),
        source: site
            .tree
            .nodes()
            .get(usize::try_from(site.node).expect("validated hole node is host-addressable"))
            .expect("validated hole node exists")
            .origin()
            .logical_path()
            .into(),
        span: crate::semantic::tree::span_record(site.source.span),
    }
}

pub(super) fn expected_fact(expected: &Result<Type, TypeUnavailableReason>) -> ExpectedTypeFact {
    match expected {
        Ok(ty) => ExpectedTypeFact::Available {
            canonical: super::super::types::canonical(ty),
            instantiated: !super::support::contains_parameter(ty),
        },
        Err(reason) => ExpectedTypeFact::Unavailable { reason: *reason },
    }
}

pub(super) fn constraints(
    site: &HoleSite<'_>,
    program: Option<&crate::hir::Program>,
    candidates: &[HoleCandidate],
) -> HoleConstraints {
    let (generic_variables, trait_obligations) = generics(site.root);
    let required = program.map_or_else(Vec::new, |program| {
        super::super::candidate_support::semantic_effects(owner_effects(site, program))
    });
    let expected_access = site
        .expected
        .as_ref()
        .map_or(OwnershipAccess::Unavailable, super::super::types::ownership);
    let exact = program.is_some() && site.expected.is_ok();
    let loop_result = super::control::nearest_loop_type(site)
        .as_ref()
        .map(super::super::types::canonical);
    let available_forms = ["loop", "return", "break", "continue", "trap", "exit"]
        .into_iter()
        .filter(|form| {
            candidates.iter().any(|candidate| {
                matches!(
                    (&candidate.expression, *form),
                    (Expression::Loop { .. }, "loop")
                        | (Expression::Return { .. }, "return")
                        | (Expression::Break { .. }, "break")
                        | (Expression::Continue {}, "continue")
                        | (Expression::Trap { .. }, "trap")
                        | (Expression::Exit { .. }, "exit")
                )
            })
        })
        .map(str::to_string)
        .collect();
    let never_admissible = candidates
        .iter()
        .any(|candidate| candidate.result_type == "never");
    HoleConstraints {
        generic_variables,
        trait_obligations,
        allowed_effects: super::support::all_effects(),
        already_required_effects: required,
        capabilities: ConstraintAvailability::Unavailable {
            reason: ConstraintUnavailableReason::NoCapabilityModel,
        },
        ownership: OwnershipConstraint {
            expected_access,
            checker_validated: exact,
            place_and_loan_facts: ConstraintAvailability::Unavailable {
                reason: if exact {
                    ConstraintUnavailableReason::NoExactSourceCorrelation
                } else {
                    ConstraintUnavailableReason::ExpectedTypeUnavailable
                },
            },
            region: "declaration_lexical_region".into(),
        },
        control: ControlConstraint {
            target: if loop_result.is_some() {
                "nearest_loop_or_function".into()
            } else {
                "function".into()
            },
            required_result: site
                .expected
                .as_ref()
                .ok()
                .map(super::super::types::canonical),
            function_return: super::super::types::canonical(&site.return_type),
            loop_result,
            available_forms,
            loop_depth: super::control::loop_depth(site),
        },
        never_admissible,
        material_incomplete: true,
    }
}

fn owner_effects(site: &HoleSite<'_>, program: &crate::hir::Program) -> EffectSet {
    let declaration = site
        .tree
        .declarations()
        .iter()
        .find(|item| item.key().to_hex() == site.declaration_key);
    let Some(declaration) = declaration else {
        return EffectSet::CONSERVATIVE_CALL;
    };
    if declaration.kind() == crate::source::DeclarationKind::Main {
        return program.main.body.effects;
    }
    program
        .functions
        .iter()
        .find_map(|function| {
            program
                .binding(function.binding)
                .filter(|binding| binding.name == declaration.name())
                .map(|_| function.body.effects)
        })
        .unwrap_or(EffectSet::CONSERVATIVE_CALL)
}

fn generics(root: &SourceNode) -> (Vec<String>, Vec<TraitObligation>) {
    let function = if super::super::types::call_is(root, "def") {
        root.children
            .iter()
            .find(|child| super::super::types::call_is(child, "fn"))
    } else {
        None
    };
    let Some(function) = function else {
        return (Vec::new(), Vec::new());
    };
    let variables = function
        .children
        .iter()
        .find(|child| super::super::types::call_is(child, "forall"))
        .map_or_else(Vec::new, |form| {
            form.children
                .iter()
                .filter_map(super::super::types::source_name)
                .map(str::to_string)
                .collect()
        });
    let obligations = function
        .children
        .iter()
        .find(|child| super::super::types::call_is(child, "bounds"))
        .map_or_else(Vec::new, |form| {
            form.children
                .iter()
                .filter_map(|bound| {
                    let variable = bound
                        .children
                        .first()
                        .and_then(super::super::types::source_name)?;
                    let trait_name = bound
                        .children
                        .get(1)
                        .and_then(super::super::types::source_name)?;
                    Some(TraitObligation {
                        variable: variable.into(),
                        trait_name: trait_name.into(),
                    })
                })
                .collect()
        });
    (variables, obligations)
}
