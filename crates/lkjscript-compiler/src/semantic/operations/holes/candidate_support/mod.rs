mod bounds;

pub(super) use bounds::{bounded_failure, omitted_categories, unsupported, unsupported_blockers};

use crate::hir::EffectSet;
use crate::semantic::schema::*;

use super::site::HoleSite;

pub(super) fn effects(site: &HoleSite<'_>, expression: &Expression) -> Vec<SemanticEffect> {
    let set = expression_effects(site, expression);
    semantic_effects(set)
}

pub(super) fn semantic_effects(set: EffectSet) -> Vec<SemanticEffect> {
    let ordered = [
        (EffectSet::ALLOCATES, SemanticEffect::Allocates),
        (EffectSet::READS_MEMORY, SemanticEffect::ReadsMemory),
        (EffectSet::WRITES_MEMORY, SemanticEffect::WritesMemory),
        (EffectSet::MUTATES_LOCAL, SemanticEffect::MutatesLocal),
        (EffectSet::HOST_IO, SemanticEffect::HostIo),
        (EffectSet::MAY_TRAP, SemanticEffect::MayTrap),
        (EffectSet::MAY_EXIT, SemanticEffect::MayExit),
        (EffectSet::MAY_DIVERGE, SemanticEffect::MayDiverge),
    ];
    ordered
        .into_iter()
        .filter_map(|(flag, value)| set.contains(flag).then_some(value))
        .collect()
}

fn expression_effects(site: &HoleSite<'_>, expression: &Expression) -> EffectSet {
    match expression {
        Expression::BuiltinCall {
            operation,
            arguments,
        } => arguments
            .iter()
            .fold(operation.0.effects(), |effects, argument| {
                effects.union(expression_effects(site, argument))
            }),
        Expression::UserCall { name, arguments } => arguments
            .iter()
            .fold(function_effects(site, name), |effects, argument| {
                effects.union(expression_effects(site, argument))
            }),
        Expression::Let { bindings, body } => bindings
            .iter()
            .fold(expression_effects(site, body), |effects, binding| {
                effects.union(expression_effects(site, &binding.value))
            }),
        Expression::Var { initial, body, .. } => EffectSet::MUTATES_LOCAL
            .union(expression_effects(site, initial))
            .union(expression_effects(site, body)),
        Expression::Set { value, .. } => {
            EffectSet::MUTATES_LOCAL.union(expression_effects(site, value))
        }
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => expression_effects(site, condition)
            .union(expression_effects(site, then_branch))
            .union(expression_effects(site, else_branch)),
        Expression::While { condition, body } => body.iter().fold(
            EffectSet::MAY_DIVERGE.union(expression_effects(site, condition)),
            |effects, value| effects.union(expression_effects(site, value)),
        ),
        Expression::Loop { body, .. } => {
            body.iter().fold(EffectSet::MAY_DIVERGE, |effects, value| {
                effects.union(expression_effects(site, value))
            })
        }
        Expression::Return { value } | Expression::Break { value } => {
            EffectSet::MAY_DIVERGE.union(expression_effects(site, value))
        }
        Expression::Continue {} => EffectSet::MAY_DIVERGE,
        Expression::Trap { value } => EffectSet::MAY_TRAP.union(expression_effects(site, value)),
        Expression::Exit { code } => EffectSet::HOST_IO
            .union(EffectSet::MAY_EXIT)
            .union(expression_effects(site, code)),
        Expression::Do { expressions } => {
            expressions.iter().fold(EffectSet::PURE, |effects, value| {
                effects.union(expression_effects(site, value))
            })
        }
        Expression::ProductValue { fields, .. } => {
            fields.iter().fold(EffectSet::ALLOCATES, |effects, field| {
                effects.union(expression_effects(site, &field.value))
            })
        }
        Expression::Field { value, .. } => {
            EffectSet::READS_MEMORY.union(expression_effects(site, value))
        }
        Expression::WithField {
            value, replacement, ..
        } => EffectSet::ALLOCATES
            .union(expression_effects(site, value))
            .union(expression_effects(site, replacement)),
        _ => EffectSet::PURE,
    }
}

fn function_effects(site: &HoleSite<'_>, name: &str) -> EffectSet {
    let Some(witness) = site
        .expected
        .as_ref()
        .ok()
        .and_then(|ty| super::validate::witness(site.tree, ty, 0))
    else {
        return EffectSet::CONSERVATIVE_CALL;
    };
    let Ok(tree) = super::validate::completed_tree(site.tree, Some((site.node, witness))) else {
        return EffectSet::CONSERVATIVE_CALL;
    };
    let Ok(program) = crate::analyze::analyze_program(&tree) else {
        return EffectSet::CONSERVATIVE_CALL;
    };
    program
        .functions
        .iter()
        .find_map(|function| {
            program
                .binding(function.binding)
                .filter(|binding| binding.name == name)
                .map(|_| function.summary)
        })
        .unwrap_or(EffectSet::CONSERVATIVE_CALL)
}
