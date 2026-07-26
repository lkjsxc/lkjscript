use crate::hir::{EffectSet, Type};
use crate::semantic::schema::*;
use crate::source::{SourceNode, SyntaxKind};

use super::super::site::HoleSite;

pub(super) fn identity(site: &HoleSite<'_>) -> TypedHoleIdentity {
    TypedHoleIdentity {
        schema: "lkjscript.typed-hole".into(),
        version: 1,
        source_revision: site.tree.revision().to_hex(),
        identity: format!("{}:{}", site.declaration_key, site.local_identity),
        declaration_key: site.declaration_key.clone(),
        local_identity: site.local_identity.clone(),
        node: site.node,
        node_fingerprint: crate::semantic::tree::fingerprint(site.source),
        source: site.tree.nodes()[site.node as usize]
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
            instantiated: !contains_parameter(ty),
        },
        Err(reason) => ExpectedTypeFact::Unavailable { reason: *reason },
    }
}

pub(super) fn constraints(
    site: &HoleSite<'_>,
    program: Option<&crate::hir::Program>,
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
    HoleConstraints {
        generic_variables,
        trait_obligations,
        allowed_effects: all_effects(),
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
            target: "expression".into(),
            required_result: site
                .expected
                .as_ref()
                .ok()
                .map(super::super::types::canonical),
            loop_depth: loop_depth(site.root, &site.path),
        },
        never_admissible: false,
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

fn loop_depth(root: &SourceNode, path: &[usize]) -> u32 {
    let mut node = root;
    let mut depth = 0_u32;
    for index in path {
        if matches!(&node.kind, SyntaxKind::Call { name } if name == "while") {
            depth = depth.saturating_add(1);
        }
        let Some(child) = node.children.get(*index) else {
            break;
        };
        node = child;
    }
    depth
}

fn contains_parameter(ty: &Type) -> bool {
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

fn all_effects() -> Vec<SemanticEffect> {
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
