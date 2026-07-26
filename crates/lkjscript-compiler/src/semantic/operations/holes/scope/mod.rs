mod declarations;
mod infer;
mod pattern;

use declarations::{declared_functions, insert};
pub(super) use declarations::{function_signatures, parse_canonical};

use std::collections::BTreeMap;

use crate::hir::Type;
use crate::semantic::schema::{ScopeEntity, ScopeEntityKind};
use crate::source::SourceNode;

use super::site::HoleSite;

pub(super) fn entities(
    site: &HoleSite<'_>,
    program: Option<&crate::hir::Program>,
) -> Vec<ScopeEntity> {
    let revision = site.tree.revision().to_hex();
    let mut entities = BTreeMap::<String, ScopeEntity>::new();
    for (name, params, result, key) in declared_functions(site.tree) {
        let ty = Type::Fn {
            params,
            ret: Box::new(result),
        };
        insert(
            &mut entities,
            program,
            &revision,
            key,
            ScopeEntityKind::Function,
            name,
            ty,
        );
    }
    for declaration in site.tree.declarations() {
        if declaration.kind() == crate::source::DeclarationKind::Product {
            let name = declaration.name().to_string();
            insert(
                &mut entities,
                program,
                &revision,
                declaration.key().to_hex(),
                ScopeEntityKind::Product,
                name.clone(),
                Type::Product(name),
            );
        }
    }
    add_lexical(site, program, &revision, &mut entities);
    entities.into_values().collect()
}

fn add_lexical(
    site: &HoleSite<'_>,
    program: Option<&crate::hir::Program>,
    revision: &str,
    output: &mut BTreeMap<String, ScopeEntity>,
) {
    let mut current = site.root;
    let mut prefix = Vec::new();
    for child_index in &site.path {
        if super::types::call_is(current, "fn") {
            if let Some(params) = current
                .children
                .iter()
                .find(|child| super::types::call_is(child, "params"))
            {
                add_parameters(params, site, program, revision, output);
            }
        }
        if super::types::call_is(current, "let") {
            for (binding_index, binding) in current.children.iter().take(*child_index).enumerate() {
                if !super::types::call_is(binding, "bind") {
                    continue;
                }
                let Some(name) = binding.children.first().and_then(super::types::source_name)
                else {
                    continue;
                };
                let Some(ty) = binding
                    .children
                    .get(1)
                    .and_then(|value| infer::expression(value, output))
                else {
                    continue;
                };
                insert(
                    output,
                    program,
                    revision,
                    local_identity(
                        &site.declaration_key,
                        "binding",
                        &prefix,
                        binding_index,
                        name,
                    ),
                    ScopeEntityKind::ImmutableLocal,
                    name.to_string(),
                    ty,
                );
            }
        }
        if super::types::call_is(current, "arm") && *child_index == 1 {
            if let Some(pattern) = current.children.first() {
                pattern::add(pattern, site, program, revision, &prefix, output);
            }
        }
        if super::types::call_is(current, "var") && *child_index == 3 {
            if let (Some(name), Some(ty)) = (
                current.children.first().and_then(super::types::source_name),
                current.children.get(1).and_then(super::types::type_form),
            ) {
                insert(
                    output,
                    program,
                    revision,
                    local_identity(&site.declaration_key, "mutable", &prefix, 0, name),
                    ScopeEntityKind::MutableLocal,
                    name.to_string(),
                    ty,
                );
            }
        }
        let Some(next) = current.children.get(*child_index) else {
            break;
        };
        current = next;
        prefix.push(*child_index);
    }
}

fn add_parameters(
    params: &SourceNode,
    site: &HoleSite<'_>,
    program: Option<&crate::hir::Program>,
    revision: &str,
    output: &mut BTreeMap<String, ScopeEntity>,
) {
    let mut index = 0;
    while index < params.children.len() {
        let Some(name) = params
            .children
            .get(index)
            .and_then(super::types::source_name)
        else {
            break;
        };
        let Some((ty, used)) = super::types::parse_type_nodes(&params.children[index + 1..]) else {
            break;
        };
        insert(
            output,
            program,
            revision,
            format!("{}:parameter:{index}:{name}", site.declaration_key),
            ScopeEntityKind::Parameter,
            name.to_string(),
            ty,
        );
        index += used + 1;
    }
}

pub(super) fn local_identity(
    declaration: &str,
    kind: &str,
    path: &[usize],
    index: usize,
    name: &str,
) -> String {
    let path = path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".");
    format!("{declaration}:{kind}:{path}:{index}:{name}")
}
