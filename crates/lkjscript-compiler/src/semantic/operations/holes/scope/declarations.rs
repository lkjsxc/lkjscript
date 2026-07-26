use std::collections::BTreeMap;

use crate::hir::Type;
use crate::semantic::schema::{ScopeEntity, ScopeEntityKind};
use crate::source::ValidatedSourceTree;

pub(crate) fn function_signatures(tree: &ValidatedSourceTree) -> Vec<(String, Vec<Type>, Type)> {
    declared_functions(tree)
        .into_iter()
        .map(|(name, params, ret, _)| (name, params, ret))
        .collect()
}

pub(super) fn declared_functions(
    tree: &ValidatedSourceTree,
) -> Vec<(String, Vec<Type>, Type, String)> {
    let source = crate::semantic::tree::source_nodes(tree);
    let mut result = Vec::new();
    for declaration in tree.declarations() {
        if declaration.kind() != crate::source::DeclarationKind::Function {
            continue;
        }
        let Some(root) = source.get(declaration.node().index() as usize) else {
            continue;
        };
        let Some(function) = root
            .children
            .iter()
            .find(|child| super::super::types::call_is(child, "fn"))
        else {
            continue;
        };
        let Some((params, ret)) = function
            .children
            .iter()
            .find_map(super::super::types::signature)
        else {
            continue;
        };
        result.push((
            declaration.name().to_string(),
            params,
            ret,
            declaration.key().to_hex(),
        ));
    }
    result.sort_by(|a, b| a.0.cmp(&b.0).then(a.3.cmp(&b.3)));
    result
}

pub(super) fn insert(
    output: &mut BTreeMap<String, ScopeEntity>,
    program: Option<&crate::hir::Program>,
    revision: &str,
    identity: String,
    kind: ScopeEntityKind,
    name: String,
    ty: Type,
) {
    let Some(program) = program else { return };
    let exact = program.bindings.iter().any(|binding| {
        binding.name == name && binding.ty == ty && binding_kind_matches(&binding.kind, kind)
    }) || kind == ScopeEntityKind::Product
        && program.products.iter().any(|product| product.name == name);
    if !exact {
        return;
    }
    output.insert(
        identity.clone(),
        ScopeEntity {
            schema: "lkjscript.semantic-entity".into(),
            contract: crate::semantic::CONTRACT.to_hex(),
            source_revision: revision.into(),
            identity,
            kind,
            name,
            instantiated_type: super::super::types::canonical(&ty),
            ownership: super::super::types::ownership(&ty),
        },
    );
}

fn binding_kind_matches(kind: &crate::hir::BindingKind, expected: ScopeEntityKind) -> bool {
    matches!(
        (kind, expected),
        (
            crate::hir::BindingKind::Parameter,
            ScopeEntityKind::Parameter
        ) | (
            crate::hir::BindingKind::ImmutableLocal,
            ScopeEntityKind::ImmutableLocal
        ) | (
            crate::hir::BindingKind::MutableLocal,
            ScopeEntityKind::MutableLocal
        ) | (crate::hir::BindingKind::Function, ScopeEntityKind::Function)
    )
}

pub(crate) fn parse_canonical(value: &str) -> Option<Type> {
    let atoms: Vec<_> = value.split_whitespace().map(str::to_string).collect();
    crate::types::parse_one(&atoms, 0)
        .ok()
        .and_then(|(ty, used)| (used == atoms.len()).then_some(ty))
}
